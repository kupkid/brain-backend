use crate::db::ids;
use rusqlite::{Connection, OptionalExtension};
use std::sync::{Arc, Mutex};
use tracing::info;

#[derive(Debug, Clone)]
pub struct TodoItem {
    pub id: i64,
    pub run_id: i64,
    pub task_id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct TodoRepository {
    conn: Arc<Mutex<Connection>>,
}

impl TodoRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn create_batch(&self, run_id: i64, tasks: &[NewTodo]) -> anyhow::Result<Vec<TodoItem>> {
        let conn = self.conn.lock().unwrap();
        let mut items = Vec::new();
        for task in tasks {
            let _uuid = ids::new_uuid_blob();
            conn.execute(
                "INSERT INTO agent_todos (run_id, task_id, title, description, status)
                 VALUES (?1, ?2, ?3, ?4, 'pending')",
                rusqlite::params![run_id, task.task_id, task.title, task.description],
            )?;
            let id = conn.last_insert_rowid();
            let item = conn.query_row(
                "SELECT id, run_id, task_id, title, description, status, created_at, updated_at
                 FROM agent_todos WHERE id = ?1",
                rusqlite::params![id],
                |r| {
                    Ok(TodoItem {
                        id: r.get(0)?,
                        run_id: r.get(1)?,
                        task_id: r.get(2)?,
                        title: r.get(3)?,
                        description: r.get(4)?,
                        status: r.get(5)?,
                        created_at: r.get(6)?,
                        updated_at: r.get(7)?,
                    })
                },
            )?;
            items.push(item);
        }
        info!("created {} todos for run {}", items.len(), run_id);
        Ok(items)
    }

    pub fn update_status(
        &self,
        run_id: i64,
        task_id: &str,
        status: &str,
    ) -> anyhow::Result<Option<TodoItem>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE agent_todos SET status = ?1 WHERE run_id = ?2 AND task_id = ?3",
            rusqlite::params![status, run_id, task_id],
        )?;
        let item = conn.query_row(
            "SELECT id, run_id, task_id, title, description, status, created_at, updated_at
             FROM agent_todos WHERE run_id = ?1 AND task_id = ?2",
            rusqlite::params![run_id, task_id],
            |r| {
                Ok(TodoItem {
                    id: r.get(0)?,
                    run_id: r.get(1)?,
                    task_id: r.get(2)?,
                    title: r.get(3)?,
                    description: r.get(4)?,
                    status: r.get(5)?,
                    created_at: r.get(6)?,
                    updated_at: r.get(7)?,
                })
            },
        );
        Ok(item.optional()?)
    }

    pub fn list_by_run(&self, run_id: i64) -> anyhow::Result<Vec<TodoItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, task_id, title, description, status, created_at, updated_at
             FROM agent_todos WHERE run_id = ?1 ORDER BY created_at",
        )?;
        let items = stmt
            .query_map(rusqlite::params![run_id], |r| {
                Ok(TodoItem {
                    id: r.get(0)?,
                    run_id: r.get(1)?,
                    task_id: r.get(2)?,
                    title: r.get(3)?,
                    description: r.get(4)?,
                    status: r.get(5)?,
                    created_at: r.get(6)?,
                    updated_at: r.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(items)
    }
}

#[derive(Debug, Clone)]
pub struct NewTodo {
    pub task_id: String,
    pub title: String,
    pub description: String,
}
