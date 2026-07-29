use axum::extract::ws::{Message, WebSocket};
use futures::StreamExt;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tracing::info;

use crate::agent::{
    AgentConfig, AgentLoop,
    agent_loop::{AgentMessage, WsAgentEvent},
    tools,
};
use crate::db;
use crate::provider::embedding::EmbeddingProvider;
use crate::provider::llm::LlmProvider;

#[derive(serde::Deserialize)]
pub struct WsTaskRequest {
    pub task: String,
    pub mode: Option<String>,
}

pub type LlmFactory = Box<
    dyn Fn(&rusqlite::Connection, &[u8; 32], serde_json::Value) -> Arc<dyn LlmProvider>
        + Send
        + Sync,
>;

pub async fn run_agent_ws(
    mut socket: WebSocket,
    llm_factory: Arc<LlmFactory>,
    embedding: Arc<dyn EmbeddingProvider>,
    data_dir: std::path::PathBuf,
    master_key: [u8; 32],
) {
    // 1. Wait for task message from client
    let task_msg = match socket.next().await {
        Some(Ok(Message::Text(text))) => text,
        _ => {
            let _ = socket.send(Message::Text(
                serde_json::json!({"type":"error","message":"ожидается JSON с task","ts":chrono::Utc::now().timestamp()}).to_string()
            )).await;
            return;
        }
    };

    let request: WsTaskRequest = match serde_json::from_str(&task_msg) {
        Ok(r) => r,
        Err(e) => {
            let _ = socket.send(Message::Text(
                serde_json::json!({"type":"error","message":format!("невалидный JSON: {e}"),"ts":chrono::Utc::now().timestamp()}).to_string()
            )).await;
            return;
        }
    };

    info!(
        "WS agent task: {}",
        &request.task[..request.task.len().min(200)]
    );

    // 2. Init DB
    let db_path = data_dir.join("brain.db");
    let conn = match db::init_db(&db_path) {
        Ok(c) => c,
        Err(e) => {
            let _ = socket.send(Message::Text(
                serde_json::json!({"type":"error","message":format!("DB init failed: {e}"),"ts":chrono::Utc::now().timestamp()}).to_string()
            )).await;
            return;
        }
    };

    let conn = Arc::new(Mutex::new(conn));
    db::ensure_vec_table(&conn.lock().unwrap(), embedding.dimensions() as i32).ok();

    // Ensure default embedding collection
    {
        let c = conn.lock().unwrap();
        let count: i64 = c
            .query_row("SELECT COUNT(*) FROM embedding_collections", [], |r| {
                r.get(0)
            })
            .unwrap_or(0);
        if count == 0 {
            use crate::db::ids;
            let uuid = ids::new_uuid_blob();
            c.execute(
                "INSERT INTO embedding_collections (uuid, model_name, dimensions, distance_metric, active)
                 VALUES (?1, 'embed-multilingual-v3.0', 1024, 'cosine', 1)",
                [uuid],
            ).ok();
        }
    }

    // 3. Create run
    let run_id = {
        let c = conn.lock().unwrap();
        let uuid = crate::db::ids::new_uuid_blob();
        c.execute(
            "INSERT INTO runs (uuid, agent_name, goal, context_json, status)
             VALUES (?1, 'ws-agent', ?2, '{}', 'running')",
            rusqlite::params![uuid, request.task],
        )
        .expect("failed to create run");
        c.last_insert_rowid()
    };

    info!("WS agent run_id={run_id}");

    // 4. Create agent with event sender
    let config = AgentConfig::from_env();
    let workspace = config.workspace_dir.clone();
    std::fs::create_dir_all(&workspace).ok();

    let toolbox = tools::build_default_tools(&conn, run_id, workspace, config.tool_timeout_seconds);
    let tools_schema = toolbox.schema();

    let llm = llm_factory(&conn.lock().unwrap(), &master_key, tools_schema);
    let (tx, mut rx) = mpsc::channel::<WsAgentEvent>(64);

    let agent =
        AgentLoop::new(llm, embedding, conn.clone(), toolbox, config, run_id).with_event_sender(tx);

    // 5. Spawn agent task
    let task = request.task.clone();
    let agent_handle = tokio::spawn(async move {
        let history: Vec<AgentMessage> = Vec::new();
        agent.process_message(&task, &history).await
    });

    // 6. Forward events to WebSocket with keepalive ping
    let mut ping_interval = interval(Duration::from_secs(30));
    ping_interval.tick().await; // skip first immediate tick

    loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                if socket.send(Message::Ping(vec![0x42])).await.is_err() {
                    info!("WS ping failed, client disconnected");
                    agent_handle.abort();
                    break;
                }
            }
            event = rx.recv() => {
                match event {
                    Some(ev) => {
                        let is_terminal = matches!(&ev, WsAgentEvent::Done { .. } | WsAgentEvent::Error { .. });
                        let json = serde_json::to_string(&ev).unwrap_or_default();
                        if socket.send(Message::Text(json)).await.is_err() {
                            info!("WS client disconnected");
                            agent_handle.abort();
                            break;
                        }
                        if is_terminal {
                            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                            break;
                        }
                    }
                    None => break,
                }
            }
            msg = socket.next() => {
                match msg {
                    Some(Ok(Message::Pong(_))) => {} // keepalive pong, ignore
                    Some(Ok(Message::Close(_))) | None => {
                        info!("WS client closed connection");
                        agent_handle.abort();
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // 7. Cleanup
    {
        let c = conn.lock().unwrap();
        let _ = c.execute(
            "UPDATE runs SET status = 'completed', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?1 AND status = 'running'",
            rusqlite::params![run_id],
        );
    }
}
