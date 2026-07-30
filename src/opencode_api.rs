use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::sse::Event;
use axum::response::sse::Sse;
use axum::routing::delete;
use axum::routing::get;
use axum::routing::patch;
use axum::routing::post;
use axum::{Json, Router};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::broadcast;
use tokio::time::{Duration, interval};

use crate::api::AppState;

// ============================================================
// SSE BROADCASTER
// ============================================================

#[derive(Clone)]
pub struct SseBroadcaster {
    tx: broadcast::Sender<String>,
}

impl SseBroadcaster {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn broadcast(&self, event_type: &str, payload: serde_json::Value) {
        let global_event = serde_json::json!({
            "directory": "/",
            "payload": {
                "type": event_type,
                "properties": payload
            }
        });
        let data = format!("data: {}\n\n", global_event);
        let _ = self.tx.send(data);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }
}

// ============================================================
// OPENCODE TYPES
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcSession {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub directory: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<OcSessionTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revert: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcSessionTime {
    pub created: i64,
    pub updated: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compacting: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcMessage {
    pub info: OcMessageInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parts: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcMessageInfo {
    pub id: String,
    pub session_id: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcPromptBody {
    pub parts: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_reply: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcProvider {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<OcModel>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcModel {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcAgent {
    pub name: String,
    pub description: String,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcProject {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcSkill {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcCommand {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SessionQuery {
    #[serde(default)]
    pub directory: Option<String>,
    #[serde(default)]
    pub roots: Option<bool>,
    #[serde(default)]
    pub start: Option<i64>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct FileQuery {
    pub directory: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FindQuery {
    pub query: Option<String>,
    #[serde(rename = "type")]
    pub file_type: Option<String>,
    pub limit: Option<usize>,
    pub pattern: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TextSearchQuery {
    pub pattern: Option<String>,
    pub directory: Option<String>,
}

// ============================================================
// ROUTER
// ============================================================

pub fn create_opencode_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health
        .route("/healthy", get(healthy))
        .route("/global/health", get(oc_health))
        .route("/global/event", get(oc_global_event))
        .route("/global/dispose", post(oc_global_dispose))
        .route("/global/config", get(oc_get_config).patch(oc_update_config))
        // Path
        .route("/path", get(oc_path))
        // Sessions
        .route("/session", get(oc_list_sessions).post(oc_create_session))
        .route("/session/status", get(oc_session_status))
        .route(
            "/session/:id",
            get(oc_get_session)
                .delete(oc_delete_session)
                .patch(oc_update_session),
        )
        .route("/session/:id/abort", post(oc_abort_session))
        .route("/session/:id/children", get(oc_session_children))
        .route("/session/:id/todo", get(oc_session_todo))
        .route("/session/:id/fork", post(oc_fork_session))
        // Messages & Prompt
        .route(
            "/session/:id/message",
            get(oc_list_messages).post(oc_prompt),
        )
        .route("/session/:id/message/:mid", get(oc_get_message))
        .route("/session/:id/prompt_async", post(oc_prompt_async))
        .route("/session/:id/command", post(oc_command))
        .route("/session/:id/shell", post(oc_shell))
        .route(
            "/session/:id/message/:mid/part/:pid",
            delete(oc_delete_part).patch(oc_update_part),
        )
        // Config
        .route("/config", get(oc_get_config).patch(oc_update_config))
        .route("/config/providers", get(oc_config_providers))
        // Providers
        .route("/provider", get(oc_list_providers))
        .route("/provider/auth", get(oc_provider_auth))
        // Projects
        .route("/project", get(oc_list_projects))
        .route("/project/current", get(oc_current_project))
        .route("/project/:id", patch(oc_update_project))
        // Agents
        .route("/agent", get(oc_list_agents))
        // Skills
        .route("/skill", get(oc_list_skills))
        // Commands
        .route("/command", get(oc_list_commands))
        // File
        .route("/file", get(oc_list_files))
        .route("/file/content", get(oc_read_file))
        // Find
        .route("/find/file", get(oc_find_files))
        .route("/find", get(oc_find_text))
        // VCS
        .route("/vcs", get(oc_vcs_info))
        // Permission/Question (stubs)
        .route("/permission", get(oc_list_permissions))
        .route("/permission/:id/reply", post(oc_reply_permission))
        .route("/question", get(oc_list_questions))
        .route("/question/:id/reply", post(oc_reply_question))
        .route("/question/:id/reject", post(oc_reject_question))
        // MCP (stubs)
        .route("/mcp", get(oc_mcp_status))
        // PTY (stubs)
        .route("/pty", get(oc_list_pty))
        // LSP / Formatter (stubs)
        .route("/lsp", get(oc_lsp_status))
        .route("/formatter", get(oc_formatter_status))
        // Log
        .route("/log", post(oc_log))
        // Instance
        .route("/instance/dispose", post(oc_instance_dispose))
        .with_state(state)
}

// ============================================================
// HANDLERS
// ============================================================

// --- Health ---

async fn healthy() -> impl IntoResponse {
    "true"
}

async fn oc_health() -> impl IntoResponse {
    Json(serde_json::json!({
        "healthy": true,
        "version": "0.1.0-brain"
    }))
}

async fn oc_path() -> impl IntoResponse {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    Json(serde_json::json!({
        "home": home,
        "state": format!("{home}/.brain"),
        "config": format!("{home}/.config/brain"),
        "worktree": format!("{home}/brain-workspace"),
        "directory": std::env::current_dir().unwrap_or_default().to_string_lossy(),
    }))
}

// --- Global SSE Event ---

async fn oc_global_event(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.sse_broadcaster.subscribe();

    let stream = async_stream::stream! {
        // Send initial connection event
        let init = serde_json::json!({
            "directory": "/",
            "payload": { "type": "server.connected", "properties": {} }
        });
        yield Ok(Event::default().data(init.to_string()));

        let mut rx = rx;
        let mut heartbeat = interval(Duration::from_secs(30));
        loop {
            tokio::select! {
                _ = heartbeat.tick() => {
                    yield Ok(Event::default().comment("heartbeat"));
                }
                msg = rx.recv() => {
                    match msg {
                        Ok(data) => {
                            // Strip "data: " prefix and "\n\n" suffix
                            let trimmed = data.strip_prefix("data: ").unwrap_or(&data);
                            let trimmed = trimmed.strip_suffix("\n\n").unwrap_or(trimmed);
                            yield Ok(Event::default().data(trimmed));
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {}
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("heartbeat"),
    )
}

async fn oc_global_dispose() -> impl IntoResponse {
    Json(serde_json::json!(true))
}

async fn oc_instance_dispose() -> impl IntoResponse {
    Json(serde_json::json!(true))
}

// --- Sessions (map to runs) ---

async fn oc_list_sessions(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SessionQuery>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = crate::run::RunRepository::new(&conn);

    let limit = q.limit.unwrap_or(50) as i64;
    let result = repo.list(None, None, limit as usize);

    match result {
        Ok(runs) => {
            let sessions: Vec<OcSession> = runs.into_iter().map(|r| run_to_session(&r)).collect();
            Json(serde_json::json!(sessions)).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "internal"})),
        )
            .into_response(),
    }
}

async fn oc_create_session(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let title = body
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("New Session");
    let directory = std::env::current_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let conn = state.conn.lock().unwrap();
    let repo = crate::run::RunRepository::new(&conn);

    let new_run = crate::run::repository::NewRun {
        project_id: None,
        agent_name: "opencode".to_string(),
        goal: title.to_string(),
        context_json: serde_json::json!({"directory": directory}).to_string(),
    };

    match repo.create(&new_run) {
        Ok(id) => {
            let uuid = {
                let mut stmt = conn.prepare("SELECT uuid FROM runs WHERE id = ?1").unwrap();
                stmt.query_row(rusqlite::params![id], |r| {
                    let b: Vec<u8> = r.get(0)?;
                    Ok(hex::encode(&b))
                })
                .unwrap_or_default()
            };

            let session = OcSession {
                id: format!("ses_{uuid}"),
                slug: None,
                project_id: None,
                directory,
                parent_id: None,
                title: Some(title.to_string()),
                version: Some("1.0".to_string()),
                time: Some(OcSessionTime {
                    created: chrono::Utc::now().timestamp_millis(),
                    updated: chrono::Utc::now().timestamp_millis(),
                    compacting: Some(0),
                    archived: Some(0),
                }),
                permission: Some(vec![]),
                summary: None,
                share: None,
                revert: None,
            };

            // Broadcast session.created
            state
                .sse_broadcaster
                .broadcast("session.created", serde_json::json!({ "info": session }));

            (StatusCode::CREATED, Json(serde_json::json!(session))).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "internal"})),
        )
            .into_response(),
    }
}

async fn oc_get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let run = find_run_by_session_id(&conn, &id);
    match run {
        Some(r) => Json(serde_json::json!(run_to_session(&r))).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn oc_delete_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let run_id = find_run_id_by_session_id(&conn, &id);
    if let Some(rid) = run_id {
        let _ = conn.execute("DELETE FROM runs WHERE id = ?1", rusqlite::params![rid]);
        state.sse_broadcaster.broadcast(
            "session.deleted",
            serde_json::json!({ "info": { "id": id } }),
        );
        Json(serde_json::json!(true)).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn oc_update_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    if let Some(rid) = find_run_id_by_session_id(&conn, &id) {
        if let Some(title) = body.get("title").and_then(|v| v.as_str()) {
            let _ = conn.execute(
                "UPDATE runs SET goal = ?1 WHERE id = ?2",
                rusqlite::params![title, rid],
            );
        }
        let run = crate::run::RunRepository::new(&conn).get(rid);
        match run {
            Ok(Some(r)) => Json(serde_json::json!(run_to_session(&r))).into_response(),
            _ => StatusCode::NOT_FOUND.into_response(),
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn oc_abort_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    if let Some(rid) = find_run_id_by_session_id(&conn, &id) {
        let _ = conn.execute(
            "UPDATE runs SET status = 'cancelled', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?1",
            rusqlite::params![rid],
        );
        state
            .sse_broadcaster
            .broadcast("session.idle", serde_json::json!({ "sessionID": id }));
        Json(serde_json::json!(true)).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn oc_session_children(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let _ = (state, id);
    Json(serde_json::json!([])).into_response()
}

async fn oc_session_todo(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    if let Some(rid) = find_run_id_by_session_id(&conn, &id) {
        let mut stmt = conn
            .prepare(
                "SELECT task_id, title, status, created_at FROM agent_todos WHERE run_id = ?1 ORDER BY created_at",
            )
            .unwrap();
        let todos: Vec<serde_json::Value> = stmt
            .query_map(rusqlite::params![rid], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, String>(0)?,
                    "title": r.get::<_, String>(1)?,
                    "status": r.get::<_, String>(2)?,
                    "time": { "created": r.get::<_, String>(3)? }
                }))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        Json(serde_json::json!(todos)).into_response()
    } else {
        Json(serde_json::json!([])).into_response()
    }
}

async fn oc_fork_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let _ = (state, id, body);
    StatusCode::NOT_IMPLEMENTED.into_response()
}

// --- Messages ---

async fn oc_list_messages(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    if let Some(rid) = find_run_id_by_session_id(&conn, &id) {
        let store = crate::run::EventStore::new(&conn);
        match store.get_events(rid) {
            Ok(events) => {
                let messages = events_to_messages(&id, &events);
                Json(serde_json::json!(messages)).into_response()
            }
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal"})),
            )
                .into_response(),
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn oc_get_message(
    State(state): State<Arc<AppState>>,
    Path((id, mid)): Path<(String, String)>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    if let Some(rid) = find_run_id_by_session_id(&conn, &id) {
        let store = crate::run::EventStore::new(&conn);
        match store.get_events(rid) {
            Ok(events) => {
                let messages = events_to_messages(&id, &events);
                if let Some(msg) = messages.into_iter().find(|m| m.info.id == mid) {
                    Json(serde_json::json!(msg)).into_response()
                } else {
                    StatusCode::NOT_FOUND.into_response()
                }
            }
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

// --- Prompt (the main one — triggers agent) ---

async fn oc_prompt(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<OcPromptBody>,
) -> impl IntoResponse {
    // Extract user text from parts
    let user_text = body
        .parts
        .iter()
        .filter_map(|p| {
            if p.get("type")?.as_str()? == "text" {
                p.get("text")?.as_str().map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    if user_text.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "no text in parts"})),
        )
            .into_response();
    }

    let conn = state.conn.lock().unwrap();
    let run_id = match find_run_id_by_session_id(&conn, &id) {
        Some(rid) => rid,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    // Create user message event
    let user_msg_id = format!("msg_{}", crate::db::ids::new_uuid());
    let store = crate::run::EventStore::new(&conn);
    let user_event = serde_json::json!({
        "info": {
            "id": user_msg_id,
            "sessionID": id,
            "role": "user",
            "time": { "created": chrono::Utc::now().timestamp_millis() },
            "agent": body.agent.as_deref().unwrap_or("coder")
        },
        "parts": [{
            "type": "text",
            "text": user_text
        }]
    });
    let _ = store.insert_event(run_id, "message", &user_event.to_string());

    // Broadcast user message
    state
        .sse_broadcaster
        .broadcast("message.updated", serde_json::json!({ "info": user_event }));

    // Set session to busy
    state.sse_broadcaster.broadcast(
        "session.status",
        serde_json::json!({ "sessionID": id, "status": { "type": "busy" } }),
    );

    // Spawn agent loop in background
    let broadcaster = state.sse_broadcaster.clone();
    let llm_factory = state.llm_factory.clone();
    let embedding = state.embedding.clone();
    let data_dir = state.config.data_dir.clone();
    let master_key = *state.master_key.lock().unwrap();
    let session_id = id.clone();
    let task_clone = user_text.clone();

    tokio::spawn(async move {
        run_opencode_agent(
            &session_id,
            run_id,
            &task_clone,
            broadcaster,
            llm_factory,
            embedding,
            data_dir,
            master_key,
        )
        .await;
    });

    // Return immediate response (like prompt_async)
    let assistant_msg_id = format!("msg_{}", crate::db::ids::new_uuid());
    Json(serde_json::json!({
        "info": {
            "id": assistant_msg_id,
            "sessionID": id,
            "role": "assistant",
            "time": { "created": chrono::Utc::now().timestamp_millis() },
            "agent": body.agent.as_deref().unwrap_or("coder"),
            "model": body.model
        },
        "parts": []
    }))
    .into_response()
}

async fn oc_prompt_async(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<OcPromptBody>,
) -> impl IntoResponse {
    // Same as oc_prompt but returns 204
    let _ = oc_prompt(State(state), Path(id), Json(body)).await;
    StatusCode::NO_CONTENT.into_response()
}

async fn oc_command(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let _ = (state, id, body);
    StatusCode::NOT_IMPLEMENTED.into_response()
}

async fn oc_shell(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let _ = (state, id, body);
    StatusCode::NOT_IMPLEMENTED.into_response()
}

async fn oc_delete_part(
    State(state): State<Arc<AppState>>,
    Path((_id, _mid, _pid)): Path<(String, String, String)>,
) -> impl IntoResponse {
    let _ = state;
    Json(serde_json::json!(true)).into_response()
}

async fn oc_update_part(
    State(state): State<Arc<AppState>>,
    Path((_id, _mid, _pid)): Path<(String, String, String)>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let _ = (state, body);
    StatusCode::NOT_IMPLEMENTED.into_response()
}

// --- Session status ---

async fn oc_session_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = crate::run::RunRepository::new(&conn);
    match repo.list(None, None, 100) {
        Ok(runs) => {
            let mut statuses = HashMap::new();
            for r in &runs {
                let sid = format!("ses_{}", hex::encode(&r.uuid));
                let status = match r.status.as_str() {
                    "running" => serde_json::json!({ "type": "busy" }),
                    _ => serde_json::json!({ "type": "idle" }),
                };
                statuses.insert(sid, status);
            }
            Json(serde_json::json!(statuses)).into_response()
        }
        Err(_) => Json(serde_json::json!({})).into_response(),
    }
}

// --- Config ---

async fn oc_get_config() -> impl IntoResponse {
    Json(serde_json::json!({
        "theme": "eucalyptus",
        "locale": "en",
        "model": { "providerID": "", "modelID": "" },
        "agent": {},
        "mcp": {}
    }))
}

async fn oc_update_config(Json(body): Json<serde_json::Value>) -> impl IntoResponse {
    // Stub — accept but don't persist
    Json(body).into_response()
}

async fn oc_config_providers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let providers_repo = crate::settings::providers::ProvidersRepository::new(&conn);
    match providers_repo.list() {
        Ok(providers) => {
            let oc_providers: Vec<OcProvider> = providers
                .into_iter()
                .map(|p| OcProvider {
                    id: p.provider_type.clone(),
                    name: p.name.clone(),
                    source: Some(p.provider_type.clone()),
                    models: None,
                    auth: None,
                })
                .collect();
            Json(serde_json::json!({
                "providers": oc_providers,
                "default": {}
            }))
            .into_response()
        }
        Err(_) => Json(serde_json::json!({ "providers": [], "default": {} })).into_response(),
    }
}

// --- Providers ---

async fn oc_list_providers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let providers_repo = crate::settings::providers::ProvidersRepository::new(&conn);
    match providers_repo.list() {
        Ok(providers) => {
            let oc_providers: Vec<OcProvider> = providers
                .iter()
                .map(|p| OcProvider {
                    id: p.provider_type.clone(),
                    name: p.name.clone(),
                    source: Some(p.provider_type.clone()),
                    models: None,
                    auth: None,
                })
                .collect();
            let connected: Vec<String> = providers
                .iter()
                .filter(|p| p.enabled)
                .map(|p| p.provider_type.clone())
                .collect();
            Json(serde_json::json!({
                "all": oc_providers,
                "connected": connected
            }))
            .into_response()
        }
        Err(_) => Json(serde_json::json!({ "all": [], "connected": [] })).into_response(),
    }
}

async fn oc_provider_auth() -> impl IntoResponse {
    Json(serde_json::json!({}))
}

// --- Projects ---

async fn oc_list_projects(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = crate::project::ProjectRepository::new(&conn, state.config.data_dir.clone());
    match repo.list(100) {
        Ok(projects) => {
            let oc_projects: Vec<OcProject> = projects
                .into_iter()
                .map(|p| OcProject {
                    id: format!("proj_{}", hex::encode(&p.uuid)),
                    name: p.name,
                    directory: Some(p.root_path),
                    icon: None,
                })
                .collect();
            Json(serde_json::json!(oc_projects)).into_response()
        }
        Err(_) => Json(serde_json::json!([])).into_response(),
    }
}

async fn oc_current_project(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = crate::project::ProjectRepository::new(&conn, state.config.data_dir.clone());
    match repo.list(1) {
        Ok(projects) if !projects.is_empty() => {
            let p = &projects[0];
            Json(serde_json::json!(OcProject {
                id: format!("proj_{}", hex::encode(&p.uuid)),
                name: p.name.clone(),
                directory: Some(p.root_path.clone()),
                icon: None,
            }))
            .into_response()
        }
        _ => {
            // Create a default project from CWD
            let cwd = std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            Json(serde_json::json!(OcProject {
                id: "proj_default".to_string(),
                name: "brain-backend".to_string(),
                directory: Some(cwd),
                icon: None,
            }))
            .into_response()
        }
    }
}

async fn oc_update_project(
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let _ = (id, body);
    StatusCode::NOT_IMPLEMENTED.into_response()
}

// --- Agents ---

async fn oc_list_agents() -> impl IntoResponse {
    Json(serde_json::json!(vec![
        OcAgent {
            name: "coder".to_string(),
            description: "Main coding agent — writes code, runs commands, manages files"
                .to_string(),
            mode: "primary".to_string(),
            native: Some(true),
            hidden: Some(false),
            permission: Some(vec![]),
            model: None,
            prompt: Some("You are a helpful coding assistant.".to_string()),
            options: None,
            steps: Some(50),
        },
        OcAgent {
            name: "task".to_string(),
            description: "Task decomposition and project management agent".to_string(),
            mode: "subagent".to_string(),
            native: Some(true),
            hidden: Some(false),
            permission: Some(vec![]),
            model: None,
            prompt: Some("You help break down complex tasks into manageable steps.".to_string()),
            options: None,
            steps: Some(20),
        },
    ]))
}

// --- Skills ---

async fn oc_list_skills() -> impl IntoResponse {
    Json(serde_json::json!([]))
}

// --- Commands ---

async fn oc_list_commands() -> impl IntoResponse {
    Json(serde_json::json!(vec![OcCommand {
        name: "compact".to_string(),
        description: "Compact the conversation context".to_string(),
        agent: Some("coder".to_string()),
        model: None,
    },]))
}

// --- File ---

async fn oc_list_files(Query(q): Query<FileQuery>) -> impl IntoResponse {
    let dir = q.path.unwrap_or_else(|| ".".to_string());
    match std::fs::read_dir(&dir) {
        Ok(entries) => {
            let files: Vec<serde_json::Value> = entries
                .filter_map(|e| e.ok())
                .map(|e| {
                    let file_type = e.file_type().ok();
                    let is_dir = file_type.map(|ft| ft.is_dir()).unwrap_or(false);
                    serde_json::json!({
                        "name": e.file_name().to_string_lossy(),
                        "path": format!("./{}", e.path().strip_prefix(&dir).unwrap_or(&e.path()).to_string_lossy()),
                        "absolute": e.path().to_string_lossy(),
                        "type": if is_dir { "directory" } else { "file" },
                        "ignored": false
                    })
                })
                .collect();
            Json(serde_json::json!(files)).into_response()
        }
        Err(_) => Json(serde_json::json!([])).into_response(),
    }
}

async fn oc_read_file(Query(q): Query<HashMap<String, String>>) -> impl IntoResponse {
    if let Some(path) = q.get("path") {
        match std::fs::read_to_string(path) {
            Ok(content) => Json(serde_json::json!({
                "type": "text",
                "content": content,
                "diff": null,
                "patch": null,
                "encoding": "utf-8"
            }))
            .into_response(),
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        }
    } else {
        StatusCode::BAD_REQUEST.into_response()
    }
}

// --- Find ---

async fn oc_find_files(Query(q): Query<FindQuery>) -> impl IntoResponse {
    let _ = q;
    Json(serde_json::json!([])).into_response()
}

async fn oc_find_text(Query(q): Query<TextSearchQuery>) -> impl IntoResponse {
    let _ = q;
    Json(serde_json::json!([])).into_response()
}

// --- VCS ---

async fn oc_vcs_info() -> impl IntoResponse {
    // Try to detect git branch
    let branch = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "main".to_string());

    Json(serde_json::json!({ "branch": branch }))
}

// --- Permission / Question (stubs) ---

async fn oc_list_permissions() -> impl IntoResponse {
    Json(serde_json::json!([]))
}

async fn oc_reply_permission(
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let _ = (id, body);
    Json(serde_json::json!(true)).into_response()
}

async fn oc_list_questions() -> impl IntoResponse {
    Json(serde_json::json!([]))
}

async fn oc_reply_question(
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let _ = (id, body);
    Json(serde_json::json!(true)).into_response()
}

async fn oc_reject_question(Path(id): Path<String>) -> impl IntoResponse {
    let _ = id;
    Json(serde_json::json!(true)).into_response()
}

// --- MCP (stub) ---

async fn oc_mcp_status() -> impl IntoResponse {
    Json(serde_json::json!({ "servers": [] }))
}

// --- PTY (stub) ---

async fn oc_list_pty() -> impl IntoResponse {
    Json(serde_json::json!([]))
}

// --- LSP / Formatter (stubs) ---

async fn oc_lsp_status() -> impl IntoResponse {
    Json(serde_json::json!([]))
}

async fn oc_formatter_status() -> impl IntoResponse {
    Json(serde_json::json!([]))
}

// --- Log ---

async fn oc_log(Json(body): Json<serde_json::Value>) -> impl IntoResponse {
    let level = body.get("level").and_then(|v| v.as_str()).unwrap_or("info");
    let msg = body.get("message").and_then(|v| v.as_str()).unwrap_or("");
    match level {
        "error" => tracing::error!("opencode log: {}", msg),
        "warn" => tracing::warn!("opencode log: {}", msg),
        _ => tracing::info!("opencode log: {}", msg),
    }
    StatusCode::OK.into_response()
}

// ============================================================
// HELPERS
// ============================================================

fn run_to_session(run: &crate::run::repository::StoredRun) -> OcSession {
    let created_ms = chrono::NaiveDateTime::parse_from_str(
        &run.created_at.replace('Z', ""),
        "%Y-%m-%dT%H:%M:%S%.f",
    )
    .ok()
    .map(|dt| {
        chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)
            .timestamp_millis()
    })
    .unwrap_or(0);

    OcSession {
        id: format!("ses_{}", hex::encode(&run.uuid)),
        slug: None,
        project_id: None,
        directory: std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        parent_id: None,
        title: Some(run.goal.clone()),
        version: Some("1.0".to_string()),
        time: Some(OcSessionTime {
            created: created_ms,
            updated: created_ms,
            compacting: Some(0),
            archived: Some(0),
        }),
        permission: Some(vec![]),
        summary: None,
        share: None,
        revert: None,
    }
}

fn find_run_by_session_id(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Option<crate::run::repository::StoredRun> {
    let hex_str = session_id.strip_prefix("ses_").unwrap_or(session_id);
    let uuid = hex::decode(hex_str).ok()?;
    let repo = crate::run::RunRepository::new(conn);
    repo.list(None, None, 1000)
        .ok()
        .and_then(|runs| runs.into_iter().find(|r| r.uuid == uuid))
}

fn find_run_id_by_session_id(conn: &rusqlite::Connection, session_id: &str) -> Option<i64> {
    let hex_str = session_id.strip_prefix("ses_").unwrap_or(session_id);
    let uuid = hex::decode(hex_str).ok()?;
    conn.query_row(
        "SELECT id FROM runs WHERE uuid = ?1",
        rusqlite::params![uuid],
        |r| r.get(0),
    )
    .ok()
}

fn events_to_messages(session_id: &str, events: &[crate::run::events::RunEvent]) -> Vec<OcMessage> {
    let mut messages: Vec<OcMessage> = Vec::new();
    let mut current_role = String::new();
    let mut current_parts: Vec<serde_json::Value> = Vec::new();
    let mut current_msg_id = String::new();
    let mut current_time = 0i64;

    for event in events {
        if event.event_type == "message" {
            // Parse the payload as message
            if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&event.payload) {
                let info = payload.get("info");
                let role = info
                    .and_then(|i| i.get("role"))
                    .and_then(|r| r.as_str())
                    .unwrap_or("user");

                // If role changed, flush previous message
                if !current_role.is_empty() && current_role != role {
                    messages.push(OcMessage {
                        info: OcMessageInfo {
                            id: current_msg_id.clone(),
                            session_id: session_id.to_string(),
                            role: current_role.clone(),
                            time: Some(serde_json::json!({ "created": current_time })),
                            agent: None,
                            model: None,
                            summary: None,
                            error: None,
                        },
                        parts: Some(current_parts.clone()),
                    });
                    current_parts.clear();
                }

                current_role = role.to_string();
                current_msg_id = info
                    .and_then(|i| i.get("id"))
                    .and_then(|id| id.as_str())
                    .unwrap_or(&format!("msg_{}", event.id))
                    .to_string();
                current_time = event
                    .created_at
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .ok()
                    .map(|dt| dt.timestamp_millis())
                    .unwrap_or(0);

                if let Some(parts) = payload.get("parts") {
                    if let Some(arr) = parts.as_array() {
                        current_parts.extend(arr.iter().cloned());
                    }
                }
            }
        } else if event.event_type == "tool_call" || event.event_type == "tool_result" {
            // Convert tool events to parts
            if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&event.payload) {
                let tool_name = payload
                    .get("tool")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown");
                let state = if event.event_type == "tool_call" {
                    serde_json::json!({ "type": "running", "input": payload.get("args").map(|a| a.to_string()).unwrap_or_default() })
                } else {
                    let success = payload
                        .get("success")
                        .and_then(|s| s.as_bool())
                        .unwrap_or(true);
                    if success {
                        serde_json::json!({ "type": "completed", "title": tool_name, "output": payload.get("summary").and_then(|s| s.as_str()).unwrap_or("") })
                    } else {
                        serde_json::json!({ "type": "error", "error": { "message": payload.get("error").and_then(|e| e.as_str()).unwrap_or("tool error") } })
                    }
                };
                current_parts.push(serde_json::json!({
                    "type": "tool",
                    "callID": format!("call_{}", event.id),
                    "tool": tool_name,
                    "state": state
                }));
            }
        }
    }

    // Flush last message
    if !current_role.is_empty() {
        messages.push(OcMessage {
            info: OcMessageInfo {
                id: current_msg_id,
                session_id: session_id.to_string(),
                role: current_role,
                time: Some(serde_json::json!({ "created": current_time })),
                agent: None,
                model: None,
                summary: None,
                error: None,
            },
            parts: Some(current_parts),
        });
    }

    messages
}

// ============================================================
// AGENT RUNNER (OpenCode-compatible)
// ============================================================

async fn run_opencode_agent(
    session_id: &str,
    run_id: i64,
    user_text: &str,
    broadcaster: SseBroadcaster,
    llm_factory: Arc<crate::ws_agent::LlmFactory>,
    embedding: Arc<dyn crate::provider::embedding::EmbeddingProvider>,
    data_dir: std::path::PathBuf,
    master_key: [u8; 32],
) {
    use crate::agent::{
        AgentConfig, AgentLoop,
        agent_loop::{AgentMessage, WsAgentEvent},
        tools,
    };

    let db_path = data_dir.join("brain.db");
    let conn = match crate::db::init_db(&db_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("DB init failed for agent: {}", e);
            broadcaster.broadcast(
                "session.error",
                serde_json::json!({ "sessionID": session_id, "error": { "message": format!("DB init failed: {e}") } }),
            );
            return;
        }
    };

    let conn = Arc::new(Mutex::new(conn));
    crate::db::ensure_vec_table(&conn.lock().unwrap(), embedding.dimensions() as i32).ok();

    let config = AgentConfig::from_env();
    let workspace = config.workspace_dir.clone();
    std::fs::create_dir_all(&workspace).ok();

    let toolbox = tools::build_default_tools(&conn, run_id, workspace, config.tool_timeout_seconds);
    let tools_schema = toolbox.schema();

    let llm = llm_factory(&conn.lock().unwrap(), &master_key, tools_schema);
    let (tx, mut rx) = tokio::sync::mpsc::channel::<WsAgentEvent>(64);

    let agent =
        AgentLoop::new(llm, embedding, conn.clone(), toolbox, config, run_id).with_event_sender(tx);

    let task = user_text.to_string();
    let sid = session_id.to_string();
    let bcast = broadcaster.clone();

    let agent_handle = tokio::spawn(async move {
        let history: Vec<AgentMessage> = Vec::new();
        agent.process_message(&task, &history).await
    });

    // Forward agent events to SSE broadcaster
    let bcast2 = bcast.clone();
    let sid2 = sid.clone();
    let forwarder = tokio::spawn(async move {
        let mut _part_id_counter = 0u32;
        while let Some(event) = rx.recv().await {
            match &event {
                WsAgentEvent::Thought { text, ts: _ } => {
                    bcast2.broadcast(
                        "message.part.delta",
                        serde_json::json!({
                            "sessionID": sid2,
                            "messageID": format!("msg_assistant_{sid2}"),
                            "partID": "prt_text_0",
                            "field": "text",
                            "delta": text
                        }),
                    );
                }
                WsAgentEvent::Text { text, ts: _ } => {
                    bcast2.broadcast(
                        "message.part.delta",
                        serde_json::json!({
                            "sessionID": sid2,
                            "messageID": format!("msg_assistant_{sid2}"),
                            "partID": "prt_text_0",
                            "field": "text",
                            "delta": text
                        }),
                    );
                }
                WsAgentEvent::ToolCall {
                    tool,
                    call_id,
                    args,
                    ts: _,
                } => {
                    _part_id_counter += 1;
                    bcast2.broadcast(
                        "message.part.updated",
                        serde_json::json!({
                            "sessionID": sid2,
                            "part": {
                                "type": "tool",
                                "callID": call_id,
                                "tool": tool,
                                "state": {
                                    "type": "running",
                                    "input": args.to_string()
                                }
                            }
                        }),
                    );
                }
                WsAgentEvent::ToolResult {
                    call_id,
                    success,
                    summary,
                    ts: _,
                } => {
                    let state = if *success {
                        serde_json::json!({ "type": "completed", "title": "", "output": summary })
                    } else {
                        serde_json::json!({ "type": "error", "error": { "message": summary } })
                    };
                    bcast2.broadcast(
                        "message.part.updated",
                        serde_json::json!({
                            "sessionID": sid2,
                            "part": {
                                "type": "tool",
                                "callID": call_id,
                                "state": state
                            }
                        }),
                    );
                }
                WsAgentEvent::Done {
                    summary,
                    total_tokens,
                    ..
                } => {
                    // Emit final assistant message
                    bcast2.broadcast(
                        "message.updated",
                        serde_json::json!({
                            "info": {
                                "id": format!("msg_assistant_{sid2}"),
                                "sessionID": sid2,
                                "role": "assistant",
                                "time": { "created": chrono::Utc::now().timestamp_millis() },
                                "agent": "coder"
                            },
                            "parts": [
                                { "type": "text", "text": summary },
                                { "type": "step-finish", "reason": "stop", "cost": 0, "tokens": { "input": total_tokens, "output": 0 } }
                            ]
                        }),
                    );
                    // Session idle
                    bcast2.broadcast("session.idle", serde_json::json!({ "sessionID": sid2 }));
                    bcast2.broadcast(
                        "session.status",
                        serde_json::json!({ "sessionID": sid2, "status": { "type": "idle" } }),
                    );
                }
                WsAgentEvent::Error { message, ts: _ } => {
                    bcast2.broadcast(
                        "session.error",
                        serde_json::json!({ "sessionID": sid2, "error": { "message": message } }),
                    );
                    bcast2.broadcast(
                        "session.status",
                        serde_json::json!({ "sessionID": sid2, "status": { "type": "idle" } }),
                    );
                }
                _ => {}
            }
        }
    });

    // Wait for agent to complete
    let _ = agent_handle.await;
    forwarder.abort();
}
