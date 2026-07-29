use std::path::PathBuf;
use std::sync::Arc;

use brain_backend::agent::{AgentLoop, ToolRegistry};
use brain_backend::agent::agent_loop::AgentMessage;
use brain_backend::provider::cohere_llm::CohereLlm;
use brain_backend::provider::cohere_embedding::CohereEmbedding;
use brain_backend::provider::llm::LlmProvider;
use brain_backend::provider::embedding::EmbeddingProvider;
use brain_backend::db;
use colored::*;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "brain_backend=info".into()),
        )
        .init();

    let api_key = std::env::var("COHERE_API_KEY")
        .expect("COHERE_API_KEY env var is required");

    let data_dir = std::env::var("BRAIN_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs().unwrap_or_else(|| PathBuf::from(".")));

    std::fs::create_dir_all(&data_dir).ok();
    let db_path = data_dir.join("brain.db");

    println!("{}", "=== Brain Agent ===".cyan().bold());
    println!("Data dir: {}", data_dir.display());
    println!("LLM: command-a-plus-05-2026");
    println!("Embedding: embed-multilingual-v3.0 (1024d)");
    println!("Type {} to exit\n", "/quit".yellow());

    let conn = db::init_db(&db_path).expect("failed to init db");
    db::ensure_vec_table(&conn, 1024).ok();

    let llm = Arc::new(CohereLlm::new(api_key.clone(), None, None));
    let embedding = Arc::new(CohereEmbedding::new(api_key, None));

    // Health check
    print!("Health check... ");
    if llm.health_check().await && embedding.health_check().await {
        println!("{}", "OK".green());
    } else {
        println!("{}", "FAILED (check API key)".red());
        std::process::exit(1);
    }

    let tools = build_tools();
    println!("Tools: {}\n", tools.names().join(", ").dimmed());

    let agent = AgentLoop::new(
        llm,
        embedding,
        Arc::new(conn),
        tools,
        data_dir,
    );

    let mut history: Vec<AgentMessage> = Vec::new();
    let stdin = std::io::stdin();

    loop {
        print!("{} ", ">".green().bold());
        use std::io::Write;
        std::io::stdout().flush().ok();

        let mut input = String::new();
        if stdin.read_line(&mut input).unwrap_or(0) == 0 {
            break;
        }
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if input == "/quit" || input == "/exit" || input == "/q" {
            println!("Bye!");
            break;
        }
        if input == "/history" {
            for msg in &history {
                let role = if msg.role == "user" { "You".blue() } else { "Agent".green() };
                println!("  {}: {}", role, msg.content);
            }
            continue;
        }
        if input == "/clear" {
            history.clear();
            println!("History cleared.");
            continue;
        }
        if input == "/tools" {
            println!("Available tools:");
            for name in agent.tools_ref().names() {
                println!("  - {}", name);
            }
            continue;
        }

        let response = agent.process_message(input, &history).await;

        println!("\n{}", response.content);
        if !response.tool_calls.is_empty() {
            println!("\n{}", "Tool calls:".dimmed());
            for tc in &response.tool_calls {
                let status = if tc.success { "ok".green() } else { "err".red() };
                println!("  {} {} — {}", status, tc.name, tc.output.dimmed());
            }
        }
        println!("{}\n", format!("({} tokens)", response.tokens_used).dimmed());

        history.push(AgentMessage {
            role: "user".to_string(),
            content: input.to_string(),
        });
        history.push(AgentMessage {
            role: "assistant".to_string(),
            content: response.content,
        });
    }
}

fn dirs() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".brain"))
}

fn build_tools() -> ToolRegistry {
    ToolRegistry::new()
        .register(
            "read_file",
            "Read a file's contents",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path to read" }
                },
                "required": ["path"]
            }),
            |args| {
                let v: serde_json::Value = serde_json::from_str(args)
                    .map_err(|e| format!("invalid args: {e}"))?;
                let path = v["path"].as_str().ok_or("missing path")?;
                std::fs::read_to_string(path)
                    .map_err(|e| format!("read error: {e}"))
            },
        )
        .register(
            "write_file",
            "Write content to a file",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "content": { "type": "string", "description": "Content to write" }
                },
                "required": ["path", "content"]
            }),
            |args| {
                let v: serde_json::Value = serde_json::from_str(args)
                    .map_err(|e| format!("invalid args: {e}"))?;
                let path = v["path"].as_str().ok_or("missing path")?;
                let content = v["content"].as_str().ok_or("missing content")?;
                if let Some(parent) = std::path::Path::new(path).parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                std::fs::write(path, content)
                    .map_err(|e| format!("write error: {e}"))?;
                Ok(format!("wrote {} bytes", content.len()))
            },
        )
        .register(
            "list_dir",
            "List directory contents",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path" }
                },
                "required": ["path"]
            }),
            |args| {
                let v: serde_json::Value = serde_json::from_str(args)
                    .map_err(|e| format!("invalid args: {e}"))?;
                let path = v["path"].as_str().ok_or("missing path")?;
                let mut entries = Vec::new();
                for entry in std::fs::read_dir(path)
                    .map_err(|e| format!("read_dir error: {e}"))? {
                    let entry = entry.map_err(|e| format!("entry error: {e}"))?;
                    let meta = entry.metadata().map_err(|e| format!("meta error: {e}"))?;
                    let kind = if meta.is_dir() { "dir" } else { "file" };
                    entries.push(format!("[{}] {} ({} bytes)", kind, entry.file_name().to_string_lossy(), meta.len()));
                }
                Ok(entries.join("\n"))
            },
        )
        .register(
            "shell_exec",
            "Execute a shell command",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Shell command" }
                },
                "required": ["command"]
            }),
            |args| {
                let v: serde_json::Value = serde_json::from_str(args)
                    .map_err(|e| format!("invalid args: {e}"))?;
                let cmd = v["command"].as_str().ok_or("missing command")?;
                let output = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(cmd)
                    .output()
                    .map_err(|e| format!("exec error: {e}"))?;
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let mut result = String::new();
                if !stdout.is_empty() { result.push_str(&stdout); }
                if !stderr.is_empty() {
                    if !result.is_empty() { result.push('\n'); }
                    result.push_str(&stderr);
                }
                if result.is_empty() {
                    result = format!("exit code: {}", output.status.code().unwrap_or(-1));
                }
                Ok(result)
            },
        )
}
