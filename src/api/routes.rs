use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::config::AppConfig;
use crate::vault::VaultRepository;
use crate::memory::{
    MemoryRepository, MemoryRetriever, MemoryIngestion, IngestParams,
    check_content, validate_layer_for_project,
};
use crate::run::{RunRepository, ToolRepository, RunContextRepository, EventStore};
use crate::workspace::{FsWorkspaceBackend, WorkspaceBackend};
use crate::project::ProjectRepository;

pub struct AppState {
    pub config: AppConfig,
    pub conn: Mutex<rusqlite::Connection>,
    pub master_key: Mutex<[u8; 32]>,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health
        .route("/health", get(health))
        // Projects
        .route("/v1/projects", get(list_projects).post(create_project))
        .route("/v1/projects/:id", get(get_project).delete(delete_project))
        // Runs
        .route("/v1/runs", get(list_runs).post(create_run))
        .route("/v1/runs/:id", get(get_run))
        .route("/v1/runs/:id/transition", post(transition_run))
        .route("/v1/runs/:id/events", get(list_run_events).post(append_run_event))
        .route("/v1/runs/:id/tools", get(list_run_tools))
        .route("/v1/runs/:id/tools/stats", get(run_tools_stats))
        .route("/v1/runs/:id/context", get(list_run_context).put(upsert_run_context))
        .route("/v1/runs/:id/context/:slot", get(get_context_slot).delete(delete_context_slot))
        // Memories
        .route("/v1/memories", get(list_memories).post(create_memory))
        .route("/v1/memories/search", post(search_memories))
        // Vault
        .route("/v1/credentials", get(list_credentials).post(store_credential))
        .route("/v1/credentials/:name", get(get_credential_metadata).delete(delete_credential))
        // Workspace
        .route("/v1/projects/:id/workspace", get(list_workspace))
        .route("/v1/projects/:id/workspace/*path", get(read_workspace_file).put(write_workspace_file))
        .with_state(state)
}

// === Health ===

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

// === Projects ===

#[derive(Deserialize)]
struct CreateProjectRequest {
    name: String,
    config_json: Option<String>,
}

#[derive(Serialize)]
struct ProjectResponse {
    id: i64,
    uuid: String,
    name: String,
    root_path: String,
}

async fn list_projects(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = ProjectRepository::new(&conn, state.config.data_dir.clone());
    match repo.list(100) {
        Ok(projects) => {
            let responses: Vec<ProjectResponse> = projects.into_iter().map(|p| ProjectResponse {
                id: p.id,
                uuid: hex::encode(&p.uuid),
                name: p.name,
                root_path: p.root_path,
            }).collect();
            Json(serde_json::json!(responses)).into_response()
        }
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

async fn create_project(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProjectRequest>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = ProjectRepository::new(&conn, state.config.data_dir.clone());
    let new_project = crate::project::repository::NewProject {
        name: req.name,
        config_json: req.config_json.unwrap_or_else(|| "{}".to_string()),
    };
    match repo.create(&new_project) {
        Ok(id) => (StatusCode::CREATED, Json(serde_json::json!({"id": id}))).into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

async fn get_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = ProjectRepository::new(&conn, state.config.data_dir.clone());
    match repo.get(id) {
        Ok(Some(p)) => Json(serde_json::json!(ProjectResponse {
            id: p.id,
            uuid: hex::encode(&p.uuid),
            name: p.name,
            root_path: p.root_path,
        })).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

async fn delete_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = ProjectRepository::new(&conn, state.config.data_dir.clone());
    match repo.delete(id) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

// === Runs ===

#[derive(Deserialize)]
struct CreateRunRequest {
    project_id: Option<i64>,
    agent_name: String,
    goal: String,
    context_json: Option<String>,
}

#[derive(Serialize)]
struct RunResponse {
    id: i64,
    uuid: String,
    status: String,
    agent_name: String,
    goal: String,
}

async fn list_runs(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = RunRepository::new(&conn);
    match repo.list(None, None, 50) {
        Ok(runs) => {
            let responses: Vec<RunResponse> = runs.into_iter().map(|r| RunResponse {
                id: r.id,
                uuid: hex::encode(&r.uuid),
                status: r.status,
                agent_name: r.agent_name,
                goal: r.goal,
            }).collect();
            Json(serde_json::json!(responses)).into_response()
        }
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

async fn create_run(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateRunRequest>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = RunRepository::new(&conn);
    let new_run = crate::run::repository::NewRun {
        project_id: req.project_id,
        agent_name: req.agent_name,
        goal: req.goal,
        context_json: req.context_json.unwrap_or_else(|| "{}".to_string()),
    };
    match repo.create(&new_run) {
        Ok(id) => (StatusCode::CREATED, Json(serde_json::json!({"id": id}))).into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

async fn get_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = RunRepository::new(&conn);
    match repo.get(id) {
        Ok(Some(r)) => Json(serde_json::json!(RunResponse {
            id: r.id,
            uuid: hex::encode(&r.uuid),
            status: r.status,
            agent_name: r.agent_name,
            goal: r.goal,
        })).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

#[derive(Deserialize)]
struct TransitionRequest {
    to_status: String,
    reason: Option<String>,
    summary: Option<String>,
    error_message: Option<String>,
}

async fn transition_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<TransitionRequest>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = RunRepository::new(&conn);
    let status = match crate::run::state::RunStatus::parse_status(&req.to_status) {
        Some(s) => s,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid status"}))).into_response(),
    };
    match repo.transition(id, status, req.reason.as_deref(), req.summary.as_deref(), req.error_message.as_deref()) {
        Ok(()) => StatusCode::OK.into_response(),
        Err(_e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "transition failed"}))).into_response(),
    }
}

// === Run Events ===

#[derive(Deserialize)]
struct AppendEventRequest {
    event_type: String,
    payload: Option<String>,
}

#[derive(Serialize)]
struct EventResponse {
    id: i64,
    seq: i64,
    event_type: String,
    payload: String,
    created_at: String,
}

async fn list_run_events(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let store = EventStore::new(&conn);
    match store.get_events(id) {
        Ok(events) => {
            let responses: Vec<EventResponse> = events.into_iter().map(|e| EventResponse {
                id: e.id,
                seq: e.seq,
                event_type: e.event_type,
                payload: e.payload,
                created_at: e.created_at,
            }).collect();
            Json(serde_json::json!(responses)).into_response()
        }
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

async fn append_run_event(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<AppendEventRequest>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let store = EventStore::new(&conn);
    match store.insert_event(id, &req.event_type, req.payload.as_deref().unwrap_or("{}")) {
        Ok(event) => (StatusCode::CREATED, Json(serde_json::json!({
            "id": event.id,
            "seq": event.seq,
        }))).into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

// === Run Tools ===

#[derive(Serialize)]
struct ToolResponse {
    id: i64,
    tool_name: String,
    status: String,
    duration_ms: Option<i64>,
    tokens_used: i64,
    error_message: Option<String>,
    started_at: String,
    completed_at: Option<String>,
}

async fn list_run_tools(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = ToolRepository::new(&conn);
    match repo.list_by_run(id) {
        Ok(tools) => {
            let responses: Vec<ToolResponse> = tools.into_iter().map(|t| ToolResponse {
                id: t.id,
                tool_name: t.tool_name,
                status: t.status,
                duration_ms: t.duration_ms,
                tokens_used: t.tokens_used,
                error_message: t.error_message,
                started_at: t.started_at,
                completed_at: t.completed_at,
            }).collect();
            Json(serde_json::json!(responses)).into_response()
        }
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

async fn run_tools_stats(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = ToolRepository::new(&conn);
    match repo.stats(id) {
        Ok(stats) => Json(serde_json::json!({
            "total": stats.total,
            "success": stats.success,
            "errors": stats.errors,
            "total_duration_ms": stats.total_duration_ms,
            "total_tokens": stats.total_tokens,
            "total_cost": stats.total_cost,
        })).into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

// === Run Context ===

#[derive(Deserialize)]
struct UpsertContextRequest {
    slot: String,
    content: String,
}

async fn list_run_context(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = RunContextRepository::new(&conn);
    match repo.list_by_run(id) {
        Ok(contexts) => {
            let responses: Vec<serde_json::Value> = contexts.into_iter().map(|c| {
                serde_json::json!({
                    "slot": c.slot,
                    "content": c.content,
                    "updated_at": c.updated_at,
                })
            }).collect();
            Json(serde_json::json!(responses)).into_response()
        }
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

async fn upsert_run_context(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpsertContextRequest>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = RunContextRepository::new(&conn);
    match repo.upsert(id, &req.slot, &req.content) {
        Ok(()) => StatusCode::OK.into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

async fn get_context_slot(
    State(state): State<Arc<AppState>>,
    Path((id, slot)): Path<(i64, String)>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = RunContextRepository::new(&conn);
    match repo.get(id, &slot) {
        Ok(Some(ctx)) => Json(serde_json::json!({
            "slot": ctx.slot,
            "content": ctx.content,
            "updated_at": ctx.updated_at,
        })).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

async fn delete_context_slot(
    State(state): State<Arc<AppState>>,
    Path((id, slot)): Path<(i64, String)>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = RunContextRepository::new(&conn);
    match repo.delete(id, &slot) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

// === Memories ===

#[derive(Deserialize)]
struct CreateMemoryRequest {
    content: String,
    memory_type: String,
    layer: Option<String>,
    importance: Option<f64>,
    project_id: Option<i64>,
    collection_id: Option<i64>,
    run_id: Option<i64>,
    source: Option<String>,
}

async fn create_memory(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateMemoryRequest>,
) -> impl IntoResponse {
    let layer = req.layer.unwrap_or_else(|| "working".to_string());
    let project_id = req.project_id;

    if let Err(e) = validate_layer_for_project(&layer, project_id) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e}))).into_response();
    }

    let heuristic = check_content(&req.content);
    if !heuristic.passed {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "content rejected by heuristic filter",
            "reason": heuristic.reason
        }))).into_response();
    }

    let conn = state.conn.lock().unwrap();
    let ingestion = MemoryIngestion::new(&conn);
    let collection_id = req.collection_id.unwrap_or(1);

    let ingest_params = IngestParams {
        content: req.content,
        memory_type: req.memory_type,
        layer,
        importance: req.importance.unwrap_or(0.5),
        source: req.source.unwrap_or_else(|| "user".to_string()),
        source_ref: None,
        project_id,
        run_id: req.run_id,
        collection_id,
        embedding: None,
    };

    match ingestion.ingest(&ingest_params) {
        Ok(result) => (StatusCode::CREATED, Json(serde_json::json!({
            "id": result.memory_id,
            "is_duplicate": result.is_duplicate,
        }))).into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

#[derive(Deserialize)]
struct SearchMemoryRequest {
    query: String,
    project_id: Option<i64>,
    collection_id: Option<i64>,
    limit: Option<usize>,
    #[allow(dead_code)] // reserved for future layer-scoped search
    layer: Option<String>,
}

#[derive(Serialize)]
struct MemorySearchResult {
    id: i64,
    content: String,
    memory_type: String,
    layer: String,
    importance: f64,
    score: f64,
    source: String,
    created_at: String,
}

async fn search_memories(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchMemoryRequest>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let retriever = MemoryRetriever::new(&conn);
    let limit = req.limit.unwrap_or(20);
    let collection_id = req.collection_id.unwrap_or(1);

    match retriever.retrieve(
        &req.query,
        req.project_id,
        collection_id,
        None,
        limit,
    ) {
        Ok(result) => {
            let responses: Vec<MemorySearchResult> = result.memories.into_iter()
                .zip(result.scores.iter())
                .map(|(mem, (id, score))| {
                    let _ = id;
                    MemorySearchResult {
                        id: mem.id,
                        content: mem.content,
                        memory_type: mem.memory_type,
                        layer: mem.layer,
                        importance: mem.importance,
                        score: *score,
                        source: mem.source,
                        created_at: mem.created_at,
                    }
                })
                .collect();
            Json(serde_json::json!(responses)).into_response()
        }
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "search failed"}))).into_response(),
    }
}

#[derive(Deserialize)]
struct ListMemoriesRequest {
    project_id: Option<i64>,
    layer: Option<String>,
    limit: Option<usize>,
}

async fn list_memories(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ListMemoriesRequest>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = MemoryRepository::new(&conn);
    let limit = req.limit.unwrap_or(50);

    let result = match (req.project_id, req.layer) {
        (Some(pid), Some(layer)) => repo.list_by_project(pid, Some(&layer), limit),
        (Some(pid), None) => repo.list_by_project(pid, None, limit),
        (None, Some(layer)) => repo.list_by_layer(&layer, None, limit),
        (None, None) => repo.list_global_profile(limit),
    };

    match result {
        Ok(memories) => {
            let responses: Vec<serde_json::Value> = memories.into_iter().map(|m| {
                serde_json::json!({
                    "id": m.id,
                    "content": m.content,
                    "memory_type": m.memory_type,
                    "layer": m.layer,
                    "importance": m.importance,
                    "access_count": m.access_count,
                    "source": m.source,
                    "created_at": m.created_at,
                })
            }).collect();
            Json(serde_json::json!(responses)).into_response()
        }
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

// === Vault ===

#[derive(Deserialize)]
struct StoreCredentialRequest {
    name: String,
    scope: Option<String>,
    project_id: Option<i64>,
    value: String,
    tags: Option<Vec<String>>,
}

async fn list_credentials(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let vault = VaultRepository::new(&conn);
    match vault.list_credentials("global", None) {
        Ok(creds) => {
            let responses: Vec<serde_json::Value> = creds.into_iter().map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "name": c.name,
                    "key_version": c.key_version,
                    "tags": serde_json::from_str::<serde_json::Value>(&c.tags_json).unwrap_or(serde_json::json!([])),
                    "created_at": c.created_at,
                })
            }).collect();
            Json(serde_json::json!(responses)).into_response()
        }
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

async fn store_credential(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StoreCredentialRequest>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let vault = VaultRepository::new(&conn);
    let master_key = state.master_key.lock().unwrap();
    let scope = req.scope.unwrap_or_else(|| "global".to_string());
    let tags = req.tags.unwrap_or_default();

    match vault.store_credential(
        &master_key,
        &req.name,
        &scope,
        req.project_id,
        req.value.as_bytes(),
        &tags,
    ) {
        Ok(id) => (StatusCode::CREATED, Json(serde_json::json!({"id": id}))).into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

/// GET /v1/credentials/:name — returns metadata ONLY, never the decrypted secret.
async fn get_credential_metadata(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let vault = VaultRepository::new(&conn);

    match vault.get_credential_metadata(&name, "global", None) {
        Ok(Some(meta)) => Json(serde_json::json!({
            "id": meta.id,
            "name": meta.name,
            "scope": meta.scope,
            "key_version": meta.key_version,
            "tags": serde_json::from_str::<serde_json::Value>(&meta.tags_json).unwrap_or(serde_json::json!([])),
            "created_at": meta.created_at,
        })).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

async fn delete_credential(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let vault = VaultRepository::new(&conn);

    match vault.delete_credential(&name, "global", None) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

// === Workspace ===

async fn list_workspace(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let proj_repo = ProjectRepository::new(&conn, state.config.data_dir.clone());
    let workspace = FsWorkspaceBackend::new(state.config.data_dir.clone());

    match proj_repo.get(id) {
        Ok(Some(project)) => {
            let uuid = project.uuid;
            match workspace.list_dir(&uuid, "") {
                Ok(entries) => {
                    let responses: Vec<serde_json::Value> = entries.into_iter().map(|e| {
                        serde_json::json!({
                            "path": e.path.to_str().unwrap_or(""),
                            "is_dir": e.is_dir,
                            "size": e.size,
                            "modified": e.modified,
                        })
                    }).collect();
                    Json(serde_json::json!(responses)).into_response()
                }
                Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "workspace error"}))).into_response(),
            }
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

async fn read_workspace_file(
    State(state): State<Arc<AppState>>,
    Path((id, file_path)): Path<(i64, String)>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let proj_repo = ProjectRepository::new(&conn, state.config.data_dir.clone());
    let workspace = FsWorkspaceBackend::new(state.config.data_dir.clone());

    match proj_repo.get(id) {
        Ok(Some(project)) => {
            let uuid = project.uuid;
            match workspace.read_file(&uuid, &file_path) {
                Ok(content) => {
                    // Try to decode as UTF-8, fall back to base64
                    match String::from_utf8(content.clone()) {
                        Ok(text) => Json(serde_json::json!({
                            "path": file_path,
                            "content": text,
                            "encoding": "utf-8",
                        })).into_response(),
                        Err(_) => {
                            use base64::Engine;
                            let encoded = base64::engine::general_purpose::STANDARD.encode(&content);
                            Json(serde_json::json!({
                                "path": file_path,
                                "content": encoded,
                                "encoding": "base64",
                            })).into_response()
                        }
                    }
                }
                Err(_e) => StatusCode::NOT_FOUND.into_response(),
            }
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}

#[derive(Deserialize)]
struct WriteFileRequest {
    content: String,
}

async fn write_workspace_file(
    State(state): State<Arc<AppState>>,
    Path((id, file_path)): Path<(i64, String)>,
    Json(req): Json<WriteFileRequest>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let proj_repo = ProjectRepository::new(&conn, state.config.data_dir.clone());
    let workspace = FsWorkspaceBackend::new(state.config.data_dir.clone());

    match proj_repo.get(id) {
        Ok(Some(project)) => {
            let uuid = project.uuid;
            match workspace.write_file(&uuid, &file_path, req.content.as_bytes()) {
                Ok(()) => StatusCode::OK.into_response(),
                Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "write failed"}))).into_response(),
            }
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response(),
    }
}
