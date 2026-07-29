use brain_backend::agent::tools;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

fn setup_db() -> Arc<Mutex<Connection>> {
    #[allow(clippy::missing_transmute_annotations)]
    unsafe {
        rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    }
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    let ddl = include_str!("../migrations/001_init.sql");
    let todos = include_str!("../migrations/002_agent_todos.sql");
    conn.execute_batch(ddl).unwrap();
    conn.execute_batch(todos).unwrap();
    brain_backend::db::ensure_vec_table(&conn, 1024).unwrap();
    Arc::new(Mutex::new(conn))
}

#[test]
fn test_todo_create_update_list() {
    let conn = setup_db();

    // Create a test run
    {
        let c = conn.lock().unwrap();
        c.execute(
            "INSERT INTO runs (uuid, agent_name, goal, context_json) VALUES (?1, 'test', 'test goal', '{}')",
            [brain_backend::db::ids::new_uuid_blob()],
        ).unwrap();
    }

    let run_id = 1;
    let tb = tools::build_default_tools(&conn, run_id, std::path::PathBuf::from("."), 30);

    // Create todos
    let result = tb
        .call(
            "todo_create",
            &serde_json::json!({
                "tasks": [
                    {"id": "t1", "title": "Create file", "description": "Create hello.py"},
                    {"id": "t2", "title": "Execute", "description": "Run hello.py"},
                    {"id": "t3", "title": "Verify", "description": "Check output"}
                ]
            }),
        )
        .unwrap();
    let text = result.result.as_str().unwrap();
    assert!(text.contains("created 3"));
    assert!(text.contains("t1"));
    assert!(text.contains("t3"));
    assert_eq!(
        result.importance,
        brain_backend::agent::tool_trait::ToolImportance::High
    );

    // List todos
    let result = tb.call("todo_list", &serde_json::json!({})).unwrap();
    let list_text = result.result.as_str().unwrap();
    assert!(list_text.contains("pending"));
    assert_eq!(list_text.lines().count(), 3);

    // Update t1 to in_progress
    let result = tb
        .call(
            "todo_update",
            &serde_json::json!({
                "task_id": "t1", "status": "in_progress"
            }),
        )
        .unwrap();
    assert!(result.result.as_str().unwrap().contains("in_progress"));

    // Update t1 to done
    let result = tb
        .call(
            "todo_update",
            &serde_json::json!({
                "task_id": "t1", "status": "done"
            }),
        )
        .unwrap();
    assert!(result.result.as_str().unwrap().contains("done"));

    // Verify via API-style query
    let c = conn.lock().unwrap();
    let status: String = c
        .query_row(
            "SELECT status FROM agent_todos WHERE run_id = 1 AND task_id = 't1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "done");
}

#[test]
fn test_file_ops() {
    let tmp = std::env::temp_dir().join("brain_test_file_ops");
    std::fs::create_dir_all(&tmp).unwrap();

    let conn = setup_db();
    let tb = tools::build_default_tools(&conn, 1, tmp.clone(), 30);

    // Write file
    let result = tb
        .call(
            "write_file",
            &serde_json::json!({
                "path": "hello.py",
                "content": "print('hello world')"
            }),
        )
        .unwrap();
    let text = result.result.as_str().unwrap();
    assert!(text.contains("ok"));
    assert!(text.contains("bytes"));

    // Read file
    let result = tb
        .call(
            "read_file",
            &serde_json::json!({
                "path": "hello.py"
            }),
        )
        .unwrap();
    assert_eq!(result.result.as_str().unwrap(), "print('hello world')");

    // List dir
    let result = tb
        .call(
            "list_dir",
            &serde_json::json!({
                "path": "."
            }),
        )
        .unwrap();
    assert!(result.result.as_str().unwrap().contains("hello.py"));

    // Grep
    let result = tb
        .call(
            "grep",
            &serde_json::json!({
                "path": "hello.py",
                "pattern": "print"
            }),
        )
        .unwrap();
    assert!(result.result.as_str().unwrap().contains("L1:"));

    // Path traversal blocked
    let result = tb.call(
        "read_file",
        &serde_json::json!({
            "path": "../etc/passwd"
        }),
    );
    assert!(result.is_err());

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_shell_exec() {
    let tmp = std::env::temp_dir().join("brain_test_shell");
    std::fs::create_dir_all(&tmp).unwrap();

    let conn = setup_db();
    let tb = tools::build_default_tools(&conn, 1, tmp.clone(), 30);

    let result = tb
        .call(
            "shell_exec",
            &serde_json::json!({
                "command": "echo hello"
            }),
        )
        .unwrap();
    let stdout = result.result.as_str().unwrap();
    assert!(stdout.contains("hello"));

    // Deny list
    let result = tb.call(
        "shell_exec",
        &serde_json::json!({
            "command": "rm -rf /"
        }),
    );
    assert!(result.is_err());

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_browser_navigate() {
    let conn = setup_db();
    let tb = tools::build_default_tools(&conn, 1, std::path::PathBuf::from("."), 30);

    let result = tb.call(
        "browser_navigate",
        &serde_json::json!({
            "url": "https://httpbin.org/get"
        }),
    );
    // Internet may be unavailable — just check it returns a result
    if let Ok(output) = result {
        assert!(!output.result.as_str().unwrap().is_empty());
    }
}

#[test]
fn test_tool_schema() {
    let conn = setup_db();
    let tb = tools::build_default_tools(&conn, 1, std::path::PathBuf::from("."), 30);

    let schema = tb.schema();
    let defs = schema.as_array().unwrap();
    assert!(defs.len() >= 8); // shell, read, write, list, grep, browser, todo_create, todo_update, todo_list

    let names: Vec<&str> = defs
        .iter()
        .map(|d| d["function"]["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"shell_exec"));
    assert!(names.contains(&"todo_create"));
    assert!(names.contains(&"read_file"));
    assert!(names.contains(&"browser_navigate"));
}
