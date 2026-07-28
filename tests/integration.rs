use rusqlite::Connection;
use std::sync::Once;
use brain_backend::db::ids;
use brain_backend::workspace::WorkspaceBackend;

static INIT: Once = Once::new();

fn setup_db() -> Connection {
    INIT.call_once(|| {
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }
    });

    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

    let ddl = include_str!("../migrations/001_init.sql");
    conn.execute_batch(ddl).unwrap();

    conn
}

fn setup_db_with_collection() -> Connection {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO embedding_collections (uuid, model_name, dimensions, distance_metric)
         VALUES (?1, 'test-model', 1024, 'cosine')",
        [ids::new_uuid_blob()],
    ).unwrap();
    brain_backend::db::ensure_vec_table(&conn, 1024).unwrap();
    conn
}

// === Project CRUD Tests ===

#[test]
fn test_project_create_get_list_delete() {
    let conn = setup_db();
    let data_dir = std::env::temp_dir().join("brain_test_projects");
    std::fs::create_dir_all(&data_dir).ok();

    let repo = brain_backend::project::ProjectRepository::new(&conn, data_dir.clone());

    let id = repo.create(&brain_backend::project::NewProject {
        name: "test-project".to_string(),
        config_json: r#"{"model":"test"}"#.to_string(),
    }).unwrap();
    assert!(id > 0);

    let project = repo.get(id).unwrap().unwrap();
    assert_eq!(project.name, "test-project");
    assert!(project.root_path.contains("workspace"));

    let projects = repo.list(10).unwrap();
    assert_eq!(projects.len(), 1);

    let deleted = repo.delete(id).unwrap();
    assert!(deleted);
    assert!(repo.get(id).unwrap().is_none());

    std::fs::remove_dir_all(&data_dir).ok();
}

#[test]
fn test_project_uuid_lookup() {
    let conn = setup_db();
    let data_dir = std::env::temp_dir().join("brain_test_project_uuid");
    std::fs::create_dir_all(&data_dir).ok();

    let repo = brain_backend::project::ProjectRepository::new(&conn, data_dir.clone());
    let id = repo.create(&brain_backend::project::NewProject {
        name: "uuid-test".to_string(),
        config_json: "{}".to_string(),
    }).unwrap();

    let project = repo.get(id).unwrap().unwrap();
    let found = repo.get_by_uuid(&project.uuid).unwrap().unwrap();
    assert_eq!(found.name, "uuid-test");

    std::fs::remove_dir_all(&data_dir).ok();
}

// === Vault Integration Tests ===

#[test]
fn test_vault_init_unlock_store_get() {
    let conn = setup_db();
    let vault = brain_backend::vault::VaultRepository::new(&conn);

    let passphrase = b"test-passphrase-123";
    let material = vault.init(passphrase).unwrap();
    assert!(!material.key.is_empty());

    let id = vault.store_credential(
        &material.key,
        "api-key",
        "global",
        None,
        b"sk-test-12345",
        &["test".to_string()],
    ).unwrap();
    assert!(id > 0);

    let meta = vault.get_credential_metadata("api-key", "global", None).unwrap().unwrap();
    assert_eq!(meta.name, "api-key");
    assert_eq!(meta.key_version, 1);

    let creds = vault.list_credentials("global", None).unwrap();
    assert_eq!(creds.len(), 1);

    let deleted = vault.delete_credential("api-key", "global", None).unwrap();
    assert!(deleted);
    assert!(vault.get_credential_metadata("api-key", "global", None).unwrap().is_none());
}

#[test]
fn test_vault_unlock_wrong_passphrase() {
    let conn = setup_db();
    let vault = brain_backend::vault::VaultRepository::new(&conn);
    vault.init(b"correct-passphrase").unwrap();
    assert!(vault.unlock(b"wrong-passphrase").is_err());
}

// === Run + Events + Tools Integration ===

#[test]
fn test_run_lifecycle_with_events() {
    let conn = setup_db();
    let data_dir = std::env::temp_dir().join("brain_test_run_lifecycle");
    std::fs::create_dir_all(&data_dir).ok();

    let proj_repo = brain_backend::project::ProjectRepository::new(&conn, data_dir.clone());
    let project_id = proj_repo.create(&brain_backend::project::NewProject {
        name: "run-test".to_string(),
        config_json: "{}".to_string(),
    }).unwrap();

    let run_repo = brain_backend::run::RunRepository::new(&conn);
    let event_store = brain_backend::run::EventStore::new(&conn);

    let run_id = run_repo.create(&brain_backend::run::NewRun {
        project_id: Some(project_id),
        agent_name: "test-agent".to_string(),
        goal: "test goal".to_string(),
        context_json: "{}".to_string(),
    }).unwrap();

    let run = run_repo.get(run_id).unwrap().unwrap();
    assert_eq!(run.status, "pending");

    let event = event_store.insert_event(run_id, "message", r#"{"text":"hello"}"#).unwrap();
    assert!(event.id > 0);

    let events = event_store.get_events(run_id).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "message");

    let exists = event_store.event_exists(run_id, &event.event_uuid).unwrap();
    assert!(exists);

    run_repo.transition(run_id, brain_backend::run::state::RunStatus::Running, Some("started"), None, None).unwrap();
    let run = run_repo.get(run_id).unwrap().unwrap();
    assert_eq!(run.status, "running");

    run_repo.transition(run_id, brain_backend::run::state::RunStatus::Completed, None, Some("done"), None).unwrap();
    let run = run_repo.get(run_id).unwrap().unwrap();
    assert_eq!(run.status, "completed");

    let runs = run_repo.list(Some(project_id), None, 10).unwrap();
    assert_eq!(runs.len(), 1);

    std::fs::remove_dir_all(&data_dir).ok();
}

#[test]
fn test_tool_invocations() {
    let conn = setup_db();
    let data_dir = std::env::temp_dir().join("brain_test_tools");
    std::fs::create_dir_all(&data_dir).ok();

    let proj_repo = brain_backend::project::ProjectRepository::new(&conn, data_dir.clone());
    let project_id = proj_repo.create(&brain_backend::project::NewProject {
        name: "tool-test".to_string(),
        config_json: "{}".to_string(),
    }).unwrap();

    let run_repo = brain_backend::run::RunRepository::new(&conn);
    let tool_repo = brain_backend::run::ToolRepository::new(&conn);

    let run_id = run_repo.create(&brain_backend::run::NewRun {
        project_id: Some(project_id),
        agent_name: "test-agent".to_string(),
        goal: "test tools".to_string(),
        context_json: "{}".to_string(),
    }).unwrap();

    let tool_id = tool_repo.start(&brain_backend::run::tools::NewToolInvocation {
        run_id,
        event_id: None,
        tool_name: "search".to_string(),
        arguments_json: r#"{"query":"hello"}"#.to_string(),
    }).unwrap();
    assert!(tool_id > 0);

    let pending = tool_repo.count_pending(run_id).unwrap();
    assert_eq!(pending, 1);

    tool_repo.complete(tool_id, &brain_backend::run::tools::ToolResult {
        result_summary: Some(r#"{"results":[]}"#.to_string()),
        result_full: None,
        status: "success".to_string(),
        duration_ms: Some(42),
        tokens_used: 0,
        cost_cents: 0,
        error_message: None,
    }).unwrap();

    let pending = tool_repo.count_pending(run_id).unwrap();
    assert_eq!(pending, 0);

    let tools = tool_repo.list_by_run(run_id).unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].tool_name, "search");

    let stats = tool_repo.stats(run_id).unwrap();
    assert_eq!(stats.total, 1);
    assert_eq!(stats.success, 1);

    std::fs::remove_dir_all(&data_dir).ok();
}

// === Context Builder Tests ===

#[test]
fn test_context_builder_assembly() {
    let conn = setup_db();
    let data_dir = std::env::temp_dir().join("brain_test_context");
    std::fs::create_dir_all(&data_dir).ok();

    let proj_repo = brain_backend::project::ProjectRepository::new(&conn, data_dir.clone());
    let project_id = proj_repo.create(&brain_backend::project::NewProject {
        name: "ctx-test".to_string(),
        config_json: r#"{"system_prompt":"You are helpful"}"#.to_string(),
    }).unwrap();

    let run_repo = brain_backend::run::RunRepository::new(&conn);
    let ctx_repo = brain_backend::run::RunContextRepository::new(&conn);

    let run_id = run_repo.create(&brain_backend::run::NewRun {
        project_id: Some(project_id),
        agent_name: "test-agent".to_string(),
        goal: "test context".to_string(),
        context_json: "{}".to_string(),
    }).unwrap();

    ctx_repo.upsert(run_id, "system_prompt", "You are helpful").unwrap();
    ctx_repo.upsert(run_id, "tools_json", r#"[{"name":"search"}]"#).unwrap();

    let builder = brain_backend::context::ContextBuilder::new(&conn, data_dir.clone());
    let assembled = builder.assemble(run_id, Some(project_id), 10000).unwrap();

    assert!(assembled.slots.iter().any(|s| s.slot == "system_prompt"));
    assert!(assembled.slots.iter().any(|s| s.slot == "tools_json"));

    let prompt = brain_backend::context::ContextBuilder::format_prompt(&assembled);
    assert!(prompt.contains("=== system_prompt ==="));
    assert!(prompt.contains("You are helpful"));

    std::fs::remove_dir_all(&data_dir).ok();
}

// === Workspace Backend Tests ===

#[test]
fn test_workspace_read_write_list() {
    let data_dir = std::env::temp_dir().join("brain_test_workspace");
    std::fs::create_dir_all(&data_dir).ok();

    let workspace = brain_backend::workspace::FsWorkspaceBackend::new(data_dir.clone());
    let uuid = ids::new_uuid().as_bytes().to_vec();

    workspace.write_file(&uuid, "test.txt", b"hello world").unwrap();

    let content = workspace.read_file(&uuid, "test.txt").unwrap();
    assert_eq!(content, b"hello world");

    let entries = workspace.list_dir(&uuid, "").unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path.to_str().unwrap(), "test.txt");

    assert!(workspace.exists(&uuid, "test.txt").unwrap());

    assert!(workspace.read_file(&uuid, "../etc/passwd").is_err());
    assert!(workspace.read_file(&uuid, "/etc/passwd").is_err());

    std::fs::remove_dir_all(&data_dir).ok();
}

// === Memory + Run + Project Full Flow ===

#[test]
fn test_full_flow_project_run_memory() {
    let conn = setup_db_with_collection();
    let data_dir = std::env::temp_dir().join("brain_test_full_flow");
    std::fs::create_dir_all(&data_dir).ok();

    let proj_repo = brain_backend::project::ProjectRepository::new(&conn, data_dir.clone());
    let project_id = proj_repo.create(&brain_backend::project::NewProject {
        name: "full-flow".to_string(),
        config_json: "{}".to_string(),
    }).unwrap();

    let run_repo = brain_backend::run::RunRepository::new(&conn);
    let mem_repo = brain_backend::memory::MemoryRepository::new(&conn);

    let run_id = run_repo.create(&brain_backend::run::NewRun {
        project_id: Some(project_id),
        agent_name: "full-flow-agent".to_string(),
        goal: "test full flow".to_string(),
        context_json: "{}".to_string(),
    }).unwrap();
    assert!(run_id > 0);

    let content_hash = brain_backend::memory::compute_content_hash("User prefers dark mode");
    let mem_id = mem_repo.insert(&brain_backend::memory::repository::NewMemory {
        project_id: Some(project_id),
        collection_id: 1,
        run_id: None,
        layer: "episodic".to_string(),
        content: "User prefers dark mode in all applications".to_string(),
        content_hash,
        memory_type: "fact".to_string(),
        source: "user".to_string(),
        importance: 0.8,
        source_ref: None,
        metadata_json: "{}".to_string(),
    }).unwrap();
    assert!(mem_id > 0);

    let memories = mem_repo.list_by_project(project_id, Some("episodic"), 10).unwrap();
    assert_eq!(memories.len(), 1);

    let count = mem_repo.count_active(Some(project_id)).unwrap();
    assert_eq!(count, 1);

    std::fs::remove_dir_all(&data_dir).ok();
}
