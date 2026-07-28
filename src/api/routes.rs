use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::config::AppConfig;
use crate::vault::VaultRepository;
use crate::memory::MemoryRepository;
use crate::run::RunRepository;
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
        // Memories
        .route("/v1/memories", post(create_memory))
        .route("/v1/memories/search", post(search_memories))
        // Vault
        .route("/v1/credentials", get(list_credentials).post(store_credential))
        .route("/v1/credentials/:name", get(get_credential).delete(delete_credential))
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
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
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
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
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
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
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
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
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
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
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
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
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
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
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
    let status = match crate::run::state::RunStatus::from_str(&req.to_status) {
        Some(s) => s,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid status"}))).into_response(),
    };
    match repo.transition(id, status, req.reason.as_deref(), req.summary.as_deref(), req.error_message.as_deref()) {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
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
}

async fn create_memory(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateMemoryRequest>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let repo = MemoryRepository::new(&conn);
    let new_mem = crate::memory::repository::NewMemory {
        collection_id: req.collection_id.unwrap_or(1),
        project_id: req.project_id,
        run_id: None,
        content: req.content,
        content_hash: Vec::new(), // Will be computed by ingestion
        memory_type: req.memory_type,
        layer: req.layer.unwrap_or_else(|| "short_term".to_string()),
        importance: req.importance.unwrap_or(0.5),
        source: "user".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };
    match repo.insert(&new_mem) {
        Ok(id) => (StatusCode::CREATED, Json(serde_json::json!({"id": id}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

#[derive(Deserialize)]
struct SearchMemoryRequest {
    query: String,
    project_id: Option<i64>,
    limit: Option<usize>,
}

async fn search_memories(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<SearchMemoryRequest>,
) -> impl IntoResponse {
    // TODO: implement full retrieval pipeline
    Json(serde_json::json!({
        "query": _req.query,
        "results": [],
        "message": "retrieval pipeline not yet implemented"
    }))
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
            let responses: Vec<serde_json::Value> = creds.into_iter().map(|(id, name, tags, created)| {
                serde_json::json!({
                    "id": id,
                    "name": name,
                    "tags": serde_json::from_str::<serde_json::Value>(&tags).unwrap_or(serde_json::json!([])),
                    "created_at": created,
                })
            }).collect();
            Json(serde_json::json!(responses)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
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
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn get_credential(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();
    let vault = VaultRepository::new(&conn);
    let master_key = state.master_key.lock().unwrap();

    match vault.get_credential(&master_key, &name, "global", None) {
        Ok(Some(plaintext)) => {
            let value = String::from_utf8_lossy(&plaintext).to_string();
            Json(serde_json::json!({"name": name, "value": value})).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
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
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}
