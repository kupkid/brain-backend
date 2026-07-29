# Brain Backend — Agent Loop

## Overview

The Agent Loop is the core runtime that processes user messages and generates responses. It runs **entirely on the server** and operates independently of client connections.

## Agent Lifecycle

```
User sends message
    │
    ▼
┌─────────────────────────────────────┐
│  1. RECEIVE                         │
│  - Parse incoming event             │
│  - Validate payload                 │
│  - Store event with seq=N           │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│  2. ASSEMBLE CONTEXT                │
│  - Load run context slots           │
│  - Load project config              │
│  - Retrieve relevant memories       │
│  - Build prompt                     │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│  3. CALL LLM                        │
│  - Send prompt to LLM provider      │
│  - Receive response                 │
│  - Parse response (text or tool)    │
└──────────────┬──────────────────────┘
               │
               ▼
         ┌─────┴─────┐
         │ Response?  │
         └─────┬─────┘
               │
       ┌───────┴───────┐
       │               │
       ▼               ▼
   [Message]      [Tool Call]
       │               │
       ▼               ▼
┌──────────┐    ┌──────────┐
│ 4a. STORE│    │ 4b. EXEC │
│ response │    │  tool    │
│ seq=N+1  │    │          │
└──────────┘    └────┬─────┘
                     │
                     ▼
               ┌──────────┐
               │ 4c. STORE│
               │ result   │
               │ seq=N+2  │
               └────┬─────┘
                    │
                    ▼
              ┌─────┴─────┐
              │ More tools?│
              └─────┬─────┘
                    │
            ┌───────┴───────┐
            │               │
            ▼               ▼
        [Yes: loop]    [No: done]
            │               │
            │               ▼
            │       ┌──────────┐
            │       │ 5. FINAL │
            │       │ response │
            │       └──────────┘
            │
            └─── back to step 3
```

## State Machine

```
pending ──▶ running ──▶ completed
              │    │
              │    └──▶ failed
              │
              └──▶ cancelled
              
paused ──▶ running (resume)
```

## Component Details

### 1. Message Receiver

```rust
// Receives user message, stores as event
fn receive_message(run_id: i64, payload: &str) -> Result<i64> {
    let event = event_store.insert_event(run_id, "message", payload)?;
    Ok(event.seq)
}
```

### 2. Context Assembler

```rust
// Builds prompt from multiple sources
fn assemble_context(run_id: i64, project_id: Option<i64>) -> Result<Context> {
    let mut context = Context::new();
    
    // 1. Run context slots (system_prompt, tools_json)
    let slots = ctx_repo.slots_map(run_id)?;
    context.add_slots(slots);
    
    // 2. Project config
    if let Some(pid) = project_id {
        let project = proj_repo.get(pid)?;
        context.add_project(project);
    }
    
    // 3. Relevant memories (4 layers)
    let memories = mem_repo.retrieve(query, project_id, limit)?;
    context.add_memories(memories);
    
    // 4. Conversation history (recent events)
    let history = event_store.get_recent(run_id, limit)?;
    context.add_history(history);
    
    Ok(context)
}
```

### 3. LLM Caller

```rust
// Calls LLM with assembled context
async fn call_llm(context: &Context, tools: &[Tool]) -> Result<LlmResponse> {
    let messages = context.to_messages();
    let response = llm_provider.complete(
        &messages,
        Some(4096),  // max_tokens
        Some(0.7),   // temperature
    ).await?;
    Ok(response)
}
```

### 4. Tool Executor

```rust
// Executes tool calls
async fn execute_tool(tool_name: &str, args: &Value) -> Result<Value> {
    let tool_id = tool_repo.start(&NewToolInvocation {
        run_id,
        tool_name: tool_name.to_string(),
        arguments_json: args.to_string(),
    })?;
    
    let result = match tool_name {
        "search" => tools::search(args).await?,
        "read_file" => tools::read_file(args).await?,
        "write_file" => tools::write_file(args).await?,
        "bash" => tools::bash(args).await?,
        _ => return Err(anyhow!("unknown tool")),
    };
    
    tool_repo.complete(tool_id, &ToolResult {
        status: "success".to_string(),
        result_summary: Some(result.summary()),
        result_full: Some(result.to_string()),
        ..Default::default()
    })?;
    
    Ok(result)
}
```

### 5. Memory Ingestor

```rust
// Extracts and stores memories from conversations
fn ingest_memories(run_id: i64, messages: &[Message]) -> Result<()> {
    for msg in messages {
        // 1. Heuristic filter (reject junk)
        if !check_content(&msg.content) {
            continue;
        }
        
        // 2. Compute content hash
        let hash = compute_content_hash(&msg.content);
        
        // 3. Check for duplicates
        if mem_repo.find_active_by_hash(project_id, collection_id, &hash)?.is_some() {
            continue;
        }
        
        // 4. Generate embedding
        let embedding = embedding_provider.embed(&msg.content).await?;
        
        // 5. Store atomically (memory + FTS + vec0)
        mem_repo.insert_atomic(&NewMemory {
            content: msg.content.clone(),
            content_hash: hash,
            embedding: Some(embedding),
            ..Default::default()
        })?;
    }
    Ok(())
}
```

## Tool Definitions

### Built-in Tools (Alpha)

| Tool | Description | Arguments |
|------|-------------|-----------|
| `search` | Search memories | `{query: string, limit?: number}` |
| `read_file` | Read workspace file | `{path: string}` |
| `write_file` | Write workspace file | `{path: string, content: string}` |
| `list_dir` | List workspace directory | `{path?: string}` |
| `bash` | Execute shell command | `{command: string, timeout?: number}` |

### Future Tools

| Tool | Description |
|------|-------------|
| `web_search` | Search the internet |
| `web_fetch` | Fetch URL content |
| `send_email` | Send email via SMTP |
| `calendar` | Manage calendar events |
| `notes` | Create/manage notes |

## Agent Loop Implementation

```rust
async fn agent_loop(run_id: i64) -> Result<()> {
    loop {
        // 1. Check run status
        let run = run_repo.get(run_id)?;
        if run.status != "running" {
            break;
        }
        
        // 2. Assemble context
        let context = assemble_context(run_id, run.project_id)?;
        
        // 3. Call LLM
        let response = call_llm(&context, &tools).await?;
        
        // 4. Handle response
        match response.type {
            ResponseType::Message(text) => {
                // Store final message
                event_store.insert_event(run_id, "message", &text)?;
                break;  // Done
            }
            ResponseType::ToolCall { name, args } => {
                // Store tool call
                event_store.insert_event(run_id, "tool_call", &json!({
                    "tool": name,
                    "args": args
                }))?;
                
                // Execute tool
                let result = execute_tool(&name, &args).await?;
                
                // Store tool result
                event_store.insert_event(run_id, "tool_result", &result)?;
                
                // Continue loop (LLM will see result)
            }
        }
    }
    
    // Transition to completed
    run_repo.transition(run_id, RunStatus::Completed, None, None, None)?;
    
    Ok(())
}
```

## Concurrency

- Each run has its own agent loop (async task)
- Runs are independent (no shared state between runs)
- SQLite WAL allows concurrent reads
- Mutex for write serialization (single writer)

## Error Handling

- **LLM failure**: Retry 3x with exponential backoff, then fail run
- **Tool failure**: Store error in event, continue loop (LLM sees error)
- **Server crash**: Run stays in "running" status, restart picks up pending runs
- **Timeout**: Kill run after configurable timeout (default: 5 minutes)

## Implementation TODO

- [ ] Agent loop async task manager
- [ ] LLM provider integration (Ollama, Claude, GPT)
- [ ] Tool registry and execution sandbox
- [ ] Memory ingestion pipeline
- [ ] Retry logic with backoff
- [ ] Run timeout and cleanup
- [ ] WebSocket push for live events
