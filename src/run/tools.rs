use rusqlite::{Connection, OptionalExtension, params};
use tracing::info;

use crate::db::ids;

#[derive(Debug, Clone)]
pub struct ToolInvocation {
    pub id: i64,
    pub uuid: Vec<u8>,
    pub run_id: i64,
    pub event_id: Option<i64>,
    pub tool_name: String,
    pub arguments_json: String,
    pub result_summary: Option<String>,
    pub result_full: Option<String>,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub tokens_used: i64,
    pub cost_cents: i64,
    pub retry_count: i64,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewToolInvocation {
    pub run_id: i64,
    pub event_id: Option<i64>,
    pub tool_name: String,
    pub arguments_json: String,
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub result_summary: Option<String>,
    pub result_full: Option<String>,
    pub status: String,
    pub duration_ms: Option<i64>,
    pub tokens_used: i64,
    pub cost_cents: i64,
    pub error_message: Option<String>,
}

pub struct ToolRepository<'a> {
    conn: &'a Connection,
}

impl<'a> ToolRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Start a new tool invocation
    pub fn start(&self, new: &NewToolInvocation) -> anyhow::Result<i64> {
        let uuid = ids::new_uuid_blob();
        self.conn.execute(
            "INSERT INTO tool_invocations (uuid, run_id, event_id, tool_name, arguments_json, status)
             VALUES (?1, ?2, ?3, ?4, ?5, 'running')",
            params![uuid, new.run_id, new.event_id, new.tool_name, new.arguments_json],
        )?;
        let id = self.conn.last_insert_rowid();
        info!(
            "tool started: id={}, tool={}, run={}",
            id, new.tool_name, new.run_id
        );
        Ok(id)
    }

    /// Complete a tool invocation with result
    pub fn complete(&self, id: i64, result: &ToolResult) -> anyhow::Result<()> {
        self.conn.execute(
            "UPDATE tool_invocations
             SET status = ?1, result_summary = ?2, result_full = ?3,
                 duration_ms = ?4, tokens_used = ?5, cost_cents = ?6,
                 error_message = ?7,
                 completed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?8",
            params![
                result.status,
                result.result_summary,
                result.result_full,
                result.duration_ms,
                result.tokens_used,
                result.cost_cents,
                result.error_message,
                id,
            ],
        )?;
        info!("tool completed: id={}, status={}", id, result.status);
        Ok(())
    }

    /// Get tool invocation by id
    pub fn get(&self, id: i64) -> anyhow::Result<Option<ToolInvocation>> {
        self.conn
            .query_row(
                "SELECT id, uuid, run_id, event_id, tool_name, arguments_json,
                        result_summary, result_full, status, started_at,
                        completed_at, duration_ms, tokens_used, cost_cents,
                        retry_count, error_message
                 FROM tool_invocations WHERE id = ?1",
                params![id],
                |r| {
                    Ok(ToolInvocation {
                        id: r.get(0)?,
                        uuid: r.get(1)?,
                        run_id: r.get(2)?,
                        event_id: r.get(3)?,
                        tool_name: r.get(4)?,
                        arguments_json: r.get(5)?,
                        result_summary: r.get(6)?,
                        result_full: r.get(7)?,
                        status: r.get(8)?,
                        started_at: r.get(9)?,
                        completed_at: r.get(10)?,
                        duration_ms: r.get(11)?,
                        tokens_used: r.get(12)?,
                        cost_cents: r.get(13)?,
                        retry_count: r.get(14)?,
                        error_message: r.get(15)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// List tool invocations for a run
    pub fn list_by_run(&self, run_id: i64) -> anyhow::Result<Vec<ToolInvocation>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, uuid, run_id, event_id, tool_name, arguments_json,
                    result_summary, result_full, status, started_at,
                    completed_at, duration_ms, tokens_used, cost_cents,
                    retry_count, error_message
             FROM tool_invocations WHERE run_id = ?1 ORDER BY started_at",
        )?;
        let tools = stmt
            .query_map(params![run_id], |r| {
                Ok(ToolInvocation {
                    id: r.get(0)?,
                    uuid: r.get(1)?,
                    run_id: r.get(2)?,
                    event_id: r.get(3)?,
                    tool_name: r.get(4)?,
                    arguments_json: r.get(5)?,
                    result_summary: r.get(6)?,
                    result_full: r.get(7)?,
                    status: r.get(8)?,
                    started_at: r.get(9)?,
                    completed_at: r.get(10)?,
                    duration_ms: r.get(11)?,
                    tokens_used: r.get(12)?,
                    cost_cents: r.get(13)?,
                    retry_count: r.get(14)?,
                    error_message: r.get(15)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tools)
    }

    /// Count pending (running) tools for a run
    pub fn count_pending(&self, run_id: i64) -> anyhow::Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM tool_invocations WHERE run_id = ?1 AND status = 'running'",
                params![run_id],
                |r| r.get(0),
            )
            .map_err(Into::into)
    }

    /// Get aggregated stats for a run
    pub fn stats(&self, run_id: i64) -> anyhow::Result<ToolStats> {
        let row = self.conn.query_row(
            "SELECT
                COUNT(*) as total,
                SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) as success,
                SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) as errors,
                SUM(COALESCE(duration_ms, 0)) as total_duration_ms,
                SUM(tokens_used) as total_tokens,
                SUM(cost_cents) as total_cost
             FROM tool_invocations WHERE run_id = ?1",
            params![run_id],
            |r| {
                Ok(ToolStats {
                    total: r.get(0)?,
                    success: r.get(1)?,
                    errors: r.get(2)?,
                    total_duration_ms: r.get(3)?,
                    total_tokens: r.get(4)?,
                    total_cost: r.get(5)?,
                })
            },
        )?;
        Ok(row)
    }
}

#[derive(Debug, Clone)]
pub struct ToolStats {
    pub total: i64,
    pub success: i64,
    pub errors: i64,
    pub total_duration_ms: i64,
    pub total_tokens: i64,
    pub total_cost: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn.execute_batch(include_str!("../../migrations/001_init.sql"))
            .unwrap();
        conn
    }

    fn create_test_run(conn: &Connection) -> i64 {
        let uuid = ids::new_uuid_blob();
        conn.execute(
            "INSERT INTO runs (uuid, agent_name, goal) VALUES (?1, 'test', 'test goal')",
            params![uuid],
        )
        .unwrap();
        let run_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO run_seq_counters (run_id, next_seq) VALUES (?1, 1)",
            params![run_id],
        )
        .unwrap();
        run_id
    }

    #[test]
    fn start_and_complete_tool() {
        let conn = setup_db();
        let run_id = create_test_run(&conn);
        let repo = ToolRepository::new(&conn);

        let new = NewToolInvocation {
            run_id,
            event_id: None,
            tool_name: "bash".to_string(),
            arguments_json: r#"{"command":"ls"}"#.to_string(),
        };

        let tool_id = repo.start(&new).unwrap();
        assert!(tool_id > 0);

        let result = ToolResult {
            result_summary: Some("file1.txt\nfile2.txt".to_string()),
            result_full: None,
            status: "success".to_string(),
            duration_ms: Some(42),
            tokens_used: 0,
            cost_cents: 0,
            error_message: None,
        };
        repo.complete(tool_id, &result).unwrap();

        let tool = repo.get(tool_id).unwrap().unwrap();
        assert_eq!(tool.tool_name, "bash");
        assert_eq!(tool.status, "success");
        assert_eq!(tool.duration_ms, Some(42));
        assert!(tool.completed_at.is_some());
    }

    #[test]
    fn count_pending_tools() {
        let conn = setup_db();
        let run_id = create_test_run(&conn);
        let repo = ToolRepository::new(&conn);

        let new = NewToolInvocation {
            run_id,
            event_id: None,
            tool_name: "bash".to_string(),
            arguments_json: "{}".to_string(),
        };

        let tool_id = repo.start(&new).unwrap();
        assert_eq!(repo.count_pending(run_id).unwrap(), 1);

        repo.complete(
            tool_id,
            &ToolResult {
                result_summary: None,
                result_full: None,
                status: "success".to_string(),
                duration_ms: None,
                tokens_used: 0,
                cost_cents: 0,
                error_message: None,
            },
        )
        .unwrap();
        assert_eq!(repo.count_pending(run_id).unwrap(), 0);
    }

    #[test]
    fn stats_aggregation() {
        let conn = setup_db();
        let run_id = create_test_run(&conn);
        let repo = ToolRepository::new(&conn);

        for i in 0..3 {
            let new = NewToolInvocation {
                run_id,
                event_id: None,
                tool_name: format!("tool_{}", i),
                arguments_json: "{}".to_string(),
            };
            let tool_id = repo.start(&new).unwrap();
            repo.complete(
                tool_id,
                &ToolResult {
                    result_summary: None,
                    result_full: None,
                    status: if i == 2 {
                        "error".to_string()
                    } else {
                        "success".to_string()
                    },
                    duration_ms: Some(100 * (i + 1)),
                    tokens_used: 10 * i,
                    cost_cents: i,
                    error_message: if i == 2 {
                        Some("timeout".to_string())
                    } else {
                        None
                    },
                },
            )
            .unwrap();
        }

        let stats = repo.stats(run_id).unwrap();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.success, 2);
        assert_eq!(stats.errors, 1);
        assert_eq!(stats.total_duration_ms, 600); // 100+200+300
        assert_eq!(stats.total_tokens, 30); // 0+10+20
        assert_eq!(stats.total_cost, 3); // 0+1+2
    }
}
