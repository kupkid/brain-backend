use std::sync::{Arc, Mutex};
use rusqlite::Connection;
use crate::agent::tool_trait::{Tool, ToolOutput, ToolImportance};
use crate::agent::todo::{TodoRepository, NewTodo};

pub struct TodoCreate {
    conn: Arc<Mutex<Connection>>,
    run_id: i64,
}

impl TodoCreate {
    pub fn new(conn: Arc<Mutex<Connection>>, run_id: i64) -> Self {
        Self { conn, run_id }
    }
}

impl Tool for TodoCreate {
    fn name(&self) -> &str { "todo_create" }
    fn description(&self) -> &str { "Create todo tasks for tracking agent progress." }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "tasks": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "title": { "type": "string" },
                            "description": { "type": "string" }
                        },
                        "required": ["id", "title"]
                    }
                }
            },
            "required": ["tasks"]
        })
    }
    fn execute(&self, args: &serde_json::Value) -> Result<ToolOutput, String> {
        let tasks_raw = args["tasks"].as_array().ok_or("missing 'tasks'")?;
        let tasks: Vec<NewTodo> = tasks_raw.iter().map(|t| NewTodo {
            task_id: t["id"].as_str().unwrap_or("").to_string(),
            title: t["title"].as_str().unwrap_or("").to_string(),
            description: t["description"].as_str().unwrap_or("").to_string(),
        }).collect();
        let repo = TodoRepository::new(Arc::clone(&self.conn));
        let items = repo.create_batch(self.run_id, &tasks).map_err(|e| e.to_string())?;
        Ok(ToolOutput::new(serde_json::json!({
            "created": items.len(),
            "tasks": items.iter().map(|i| serde_json::json!({
                "id": i.task_id, "title": i.title, "status": i.status
            })).collect::<Vec<_>>(),
        }), ToolImportance::High))
    }
}

pub struct TodoUpdate {
    conn: Arc<Mutex<Connection>>,
    run_id: i64,
}

impl TodoUpdate {
    pub fn new(conn: Arc<Mutex<Connection>>, run_id: i64) -> Self {
        Self { conn, run_id }
    }
}

impl Tool for TodoUpdate {
    fn name(&self) -> &str { "todo_update" }
    fn description(&self) -> &str { "Update a todo task status." }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string" },
                "status": { "type": "string", "enum": ["pending", "in_progress", "done", "failed"] }
            },
            "required": ["task_id", "status"]
        })
    }
    fn execute(&self, args: &serde_json::Value) -> Result<ToolOutput, String> {
        let task_id = args["task_id"].as_str().ok_or("missing 'task_id'")?;
        let status = args["status"].as_str().ok_or("missing 'status'")?;
        let repo = TodoRepository::new(Arc::clone(&self.conn));
        let item = repo.update_status(self.run_id, task_id, status).map_err(|e| e.to_string())?;
        match item {
            Some(i) => Ok(ToolOutput::new(serde_json::json!({
                "id": i.task_id, "title": i.title, "status": i.status
            }), ToolImportance::High)),
            None => Err(format!("task '{task_id}' not found")),
        }
    }
}

pub struct TodoList {
    conn: Arc<Mutex<Connection>>,
    run_id: i64,
}

impl TodoList {
    pub fn new(conn: Arc<Mutex<Connection>>, run_id: i64) -> Self {
        Self { conn, run_id }
    }
}

impl Tool for TodoList {
    fn name(&self) -> &str { "todo_list" }
    fn description(&self) -> &str { "List all todos for the current run." }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }
    fn execute(&self, _args: &serde_json::Value) -> Result<ToolOutput, String> {
        let repo = TodoRepository::new(Arc::clone(&self.conn));
        let items = repo.list_by_run(self.run_id).map_err(|e| e.to_string())?;
        Ok(ToolOutput::new(serde_json::json!({
            "tasks": items.iter().map(|i| serde_json::json!({
                "id": i.task_id, "title": i.title, "description": i.description, "status": i.status
            })).collect::<Vec<_>>(),
        }), ToolImportance::High))
    }
}
