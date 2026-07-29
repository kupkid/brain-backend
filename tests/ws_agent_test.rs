use brain_backend::agent::agent_loop::{AgentMessage, WsAgentEvent};
use brain_backend::agent::tools;
use brain_backend::agent::{AgentConfig, AgentLoop};
use brain_backend::provider::embedding::{EmbeddingError, EmbeddingProvider};
use brain_backend::provider::llm::{
    LlmError, LlmMessage, LlmProvider, LlmResponse, LlmToolCall, LlmToolResult, StructuredOutput,
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Mock LLM that returns one tool call (list_dir), then a final text response.
struct MockLlm {
    call_count: std::sync::atomic::AtomicU32,
}

impl MockLlm {
    fn new() -> Self {
        Self {
            call_count: std::sync::atomic::AtomicU32::new(0),
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for MockLlm {
    async fn complete(
        &self,
        _messages: &[LlmMessage],
        _max_tokens: Option<usize>,
        _temperature: Option<f32>,
    ) -> Result<LlmResponse, LlmError> {
        let n = self
            .call_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if n == 0 {
            // First call: return tool call
            Ok(LlmResponse {
                content: String::new(),
                tokens_used: 100,
                model: "mock".to_string(),
            })
        } else {
            // Subsequent calls: return final text
            Ok(LlmResponse {
                content: "Задача выполнена".to_string(),
                tokens_used: 50,
                model: "mock".to_string(),
            })
        }
    }

    async fn complete_with_tools(
        &self,
        _messages: &[LlmMessage],
        _max_tokens: Option<usize>,
        _temperature: Option<f32>,
    ) -> Result<LlmToolResult, LlmError> {
        let n = self
            .call_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if n == 0 {
            // First call: return one tool call
            Ok(LlmToolResult {
                content: "Проверяю рабочую директорию".to_string(),
                tool_calls: vec![LlmToolCall {
                    id: "call_mock_0".to_string(),
                    name: "list_dir".to_string(),
                    arguments: serde_json::json!({"path": "."}),
                }],
                tokens_used: 150,
                tokens_input: 100,
                tokens_output: 50,
            })
        } else {
            // Second call: final response (no tools)
            Ok(LlmToolResult {
                content: "Задача выполнена: рабочая директория проверена".to_string(),
                tool_calls: vec![],
                tokens_used: 80,
                tokens_input: 50,
                tokens_output: 30,
            })
        }
    }

    async fn structured_complete(
        &self,
        _messages: &[LlmMessage],
        _schema: &serde_json::Value,
        _max_tokens: Option<usize>,
    ) -> Result<StructuredOutput, LlmError> {
        unimplemented!("not used in this test")
    }

    fn model_name(&self) -> &str {
        "mock"
    }
    async fn health_check(&self) -> bool {
        true
    }
}

/// Mock embedding provider
struct MockEmbedding;

#[async_trait::async_trait]
impl EmbeddingProvider for MockEmbedding {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, EmbeddingError> {
        Ok(vec![0.0; 1024])
    }
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        Ok(texts.iter().map(|_| vec![0.0; 1024]).collect())
    }
    fn dimensions(&self) -> usize {
        1024
    }
    fn model_name(&self) -> &str {
        "mock-embed"
    }
    async fn health_check(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn test_agent_events_flow() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("brain.db");
    let conn = Arc::new(Mutex::new(brain_backend::db::init_db(&db_path).unwrap()));
    brain_backend::db::ensure_vec_table(&conn.lock().unwrap(), 1024).ok();

    // Ensure embedding collection
    {
        let c = conn.lock().unwrap();
        let count: i64 = c
            .query_row("SELECT COUNT(*) FROM embedding_collections", [], |r| {
                r.get(0)
            })
            .unwrap_or(0);
        if count == 0 {
            let uuid = brain_backend::db::ids::new_uuid_blob();
            c.execute(
                "INSERT INTO embedding_collections (uuid, model_name, dimensions, distance_metric, active)
                 VALUES (?1, 'mock', 1024, 'cosine', 1)",
                [uuid],
            ).ok();
        }
    }

    // Create run
    let run_id = {
        let c = conn.lock().unwrap();
        let uuid = brain_backend::db::ids::new_uuid_blob();
        c.execute(
            "INSERT INTO runs (uuid, agent_name, goal, context_json, status)
             VALUES (?1, 'test', 'test task', '{}', 'running')",
            [uuid],
        )
        .unwrap();
        c.last_insert_rowid()
    };

    let config = AgentConfig {
        workspace_dir: tmp.path().to_path_buf(),
        ..AgentConfig::default()
    };

    let workspace = config.workspace_dir.clone();
    let toolbox = tools::build_default_tools(&conn, run_id, workspace, 30);

    let llm = Arc::new(MockLlm::new());
    let embedding = Arc::new(MockEmbedding);

    let (tx, mut rx) = mpsc::channel::<WsAgentEvent>(64);

    let agent = AgentLoop::new(llm, embedding, conn, toolbox, config, run_id).with_event_sender(tx);

    // Run agent
    let agent = Arc::new(agent);
    let agent_clone = Arc::clone(&agent);
    tokio::spawn(async move {
        let history: Vec<AgentMessage> = Vec::new();
        agent_clone
            .process_message("Создай файл test.txt", &history)
            .await
    });

    // Collect events
    let mut events = Vec::new();
    while let Some(ev) = rx.recv().await {
        let is_terminal = matches!(&ev, WsAgentEvent::Done { .. } | WsAgentEvent::Error { .. });
        events.push(ev);
        if is_terminal {
            break;
        }
    }

    // Verify events
    assert!(!events.is_empty(), "should receive at least one event");

    let has_tool_call = events
        .iter()
        .any(|e| matches!(e, WsAgentEvent::ToolCall { .. }));
    let has_tool_result = events
        .iter()
        .any(|e| matches!(e, WsAgentEvent::ToolResult { .. }));
    let has_done = events
        .iter()
        .any(|e| matches!(e, WsAgentEvent::Done { .. }));

    assert!(has_tool_call, "should have tool_call event");
    assert!(has_tool_result, "should have tool_result event");
    assert!(has_done, "should have done event");

    // Verify done event has valid stats
    if let Some(WsAgentEvent::Done {
        total_tokens,
        total_calls,
        ..
    }) = events
        .iter()
        .find(|e| matches!(e, WsAgentEvent::Done { .. }))
    {
        assert!(*total_tokens > 0, "total_tokens should be > 0");
        assert!(*total_calls > 0, "total_calls should be > 0");
    }

    println!("Events received: {}", events.len());
    for ev in &events {
        match ev {
            WsAgentEvent::Thought { text, .. } => println!("  thought: {text}"),
            WsAgentEvent::ToolCall { tool, call_id, .. } => {
                println!("  tool_call: {tool} ({call_id})")
            }
            WsAgentEvent::ToolResult {
                call_id,
                success,
                summary,
                ..
            } => println!("  tool_result: {call_id} success={success} {summary}"),
            WsAgentEvent::Done {
                total_tokens,
                total_calls,
                ..
            } => println!("  done: {total_tokens} tokens, {total_calls} calls"),
            _ => println!("  other event"),
        }
    }
}

#[tokio::test]
async fn test_agent_error_emits_error_event() {
    use brain_backend::provider::llm::LlmError;

    struct FailingLlm;

    #[async_trait::async_trait]
    impl LlmProvider for FailingLlm {
        async fn complete(
            &self,
            _: &[LlmMessage],
            _: Option<usize>,
            _: Option<f32>,
        ) -> Result<LlmResponse, LlmError> {
            Err(LlmError::Provider("mock failure".into()))
        }
        async fn complete_with_tools(
            &self,
            _: &[LlmMessage],
            _: Option<usize>,
            _: Option<f32>,
        ) -> Result<LlmToolResult, LlmError> {
            Err(LlmError::Provider("mock failure".into()))
        }
        async fn structured_complete(
            &self,
            _: &[LlmMessage],
            _: &serde_json::Value,
            _: Option<usize>,
        ) -> Result<StructuredOutput, LlmError> {
            unimplemented!()
        }
        fn model_name(&self) -> &str {
            "failing"
        }
        async fn health_check(&self) -> bool {
            true
        }
    }

    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("brain.db");
    let conn = Arc::new(Mutex::new(brain_backend::db::init_db(&db_path).unwrap()));
    brain_backend::db::ensure_vec_table(&conn.lock().unwrap(), 1024).ok();

    let run_id = {
        let c = conn.lock().unwrap();
        let uuid = brain_backend::db::ids::new_uuid_blob();
        c.execute(
            "INSERT INTO runs (uuid, agent_name, goal, context_json, status)
             VALUES (?1, 'test', 'fail', '{}', 'running')",
            [uuid],
        )
        .unwrap();
        c.last_insert_rowid()
    };

    let config = AgentConfig {
        workspace_dir: tmp.path().to_path_buf(),
        ..AgentConfig::default()
    };

    let toolbox = tools::build_default_tools(&conn, run_id, tmp.path().to_path_buf(), 30);

    let (tx, mut rx) = mpsc::channel::<WsAgentEvent>(64);

    let agent = AgentLoop::new(
        Arc::new(FailingLlm),
        Arc::new(MockEmbedding),
        conn,
        toolbox,
        config,
        run_id,
    )
    .with_event_sender(tx);

    let agent = Arc::new(agent);
    let agent_clone = Arc::clone(&agent);
    tokio::spawn(async move {
        let history: Vec<AgentMessage> = Vec::new();
        agent_clone.process_message("test", &history).await
    });

    let mut events = Vec::new();
    while let Some(ev) = rx.recv().await {
        let is_terminal = matches!(&ev, WsAgentEvent::Done { .. } | WsAgentEvent::Error { .. });
        events.push(ev);
        if is_terminal {
            break;
        }
    }

    assert!(!events.is_empty());
    assert!(
        events
            .iter()
            .any(|e| matches!(e, WsAgentEvent::Error { .. }))
    );
    println!("Error test: {} events", events.len());
}
