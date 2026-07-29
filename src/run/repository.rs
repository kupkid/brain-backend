use rusqlite::{Connection, OptionalExtension, params};
use tracing::info;

use super::state::RunStatus;
use crate::db::ids;

#[derive(Debug, Clone)]
pub struct NewRun {
    pub project_id: Option<i64>,
    pub agent_name: String,
    pub goal: String,
    pub context_json: String,
}

#[derive(Debug, Clone)]
pub struct StoredRun {
    pub id: i64,
    pub uuid: Vec<u8>,
    pub project_id: Option<i64>,
    pub agent_name: String,
    pub goal: String,
    pub context_json: String,
    pub status: String,
    pub summary: Option<String>,
    pub tokens_used: i64,
    pub cost_cents: i64,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

pub struct RunRepository<'a> {
    conn: &'a Connection,
}

impl<'a> RunRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Create run + seq counter atomically
    pub fn create(&self, run: &NewRun) -> anyhow::Result<i64> {
        self.conn.execute_batch("BEGIN IMMEDIATE;")?;

        let uuid = ids::new_uuid_blob();

        let result = self.conn.execute(
            "INSERT INTO runs (uuid, project_id, agent_name, goal, context_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                uuid,
                run.project_id,
                run.agent_name,
                run.goal,
                run.context_json
            ],
        );

        if let Err(e) = result {
            self.conn.execute_batch("ROLLBACK;")?;
            return Err(e.into());
        }

        let run_id = self.conn.last_insert_rowid();

        // Create seq counter in same transaction
        if let Err(e) = self.conn.execute(
            "INSERT INTO run_seq_counters (run_id, next_seq) VALUES (?1, 1)",
            params![run_id],
        ) {
            self.conn.execute_batch("ROLLBACK;")?;
            return Err(e.into());
        }

        self.conn.execute_batch("COMMIT;")?;

        info!("created run: id={}, agent={}", run_id, run.agent_name);
        Ok(run_id)
    }

    /// Get run by id
    pub fn get(&self, id: i64) -> anyhow::Result<Option<StoredRun>> {
        self.conn
            .query_row(
                "SELECT id, uuid, project_id, agent_name, goal, context_json,
                        status, summary, tokens_used, cost_cents,
                        created_at, updated_at, completed_at
                 FROM runs WHERE id = ?1",
                params![id],
                |r| {
                    Ok(StoredRun {
                        id: r.get(0)?,
                        uuid: r.get(1)?,
                        project_id: r.get(2)?,
                        agent_name: r.get(3)?,
                        goal: r.get(4)?,
                        context_json: r.get(5)?,
                        status: r.get(6)?,
                        summary: r.get(7)?,
                        tokens_used: r.get(8)?,
                        cost_cents: r.get(9)?,
                        created_at: r.get(10)?,
                        updated_at: r.get(11)?,
                        completed_at: r.get(12)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Transition run status
    pub fn transition(
        &self,
        id: i64,
        new_status: RunStatus,
        reason: Option<&str>,
        summary: Option<&str>,
        error_message: Option<&str>,
    ) -> anyhow::Result<()> {
        self.conn.execute_batch("BEGIN IMMEDIATE;")?;

        // Get current status
        let current_status: String =
            self.conn
                .query_row("SELECT status FROM runs WHERE id = ?1", params![id], |r| {
                    r.get(0)
                })?;

        let current = RunStatus::parse_status(&current_status)
            .ok_or_else(|| anyhow::anyhow!("invalid current status: {}", current_status))?;

        // Validate transition
        let _transition = super::state::RunStateMachine::validate_transition(
            &current,
            &new_status,
            summary.is_some(),
            error_message.is_some(),
            reason.is_some(),
            0, // TODO: get actual pending tool count
        )
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        // Update run
        let _completed_at = matches!(
            new_status,
            RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled
        )
        .then(|| {
            // Will be set by DB default
            String::new()
        });

        self.conn.execute(
            "UPDATE runs
             SET status = ?1, summary = COALESCE(?2, summary),
                 completed_at = CASE WHEN ?1 IN ('completed', 'failed', 'cancelled')
                                     THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                                     ELSE completed_at END
             WHERE id = ?3",
            params![new_status.as_str(), summary, id],
        )?;

        // Record transition
        self.conn.execute(
            "INSERT INTO run_transitions (run_id, from_status, to_status, reason)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, current.as_str(), new_status.as_str(), reason],
        )?;

        self.conn.execute_batch("COMMIT;")?;

        info!(
            "run {} transitioned: {} → {}",
            id,
            current.as_str(),
            new_status.as_str()
        );
        Ok(())
    }

    /// List runs with filters
    pub fn list(
        &self,
        project_id: Option<i64>,
        status: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<StoredRun>> {
        let (sql, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
            match (project_id, status) {
                (Some(pid), Some(s)) => (
                    "SELECT id, uuid, project_id, agent_name, goal, context_json,
                        status, summary, tokens_used, cost_cents,
                        created_at, updated_at, completed_at
                 FROM runs WHERE project_id = ?1 AND status = ?2
                 ORDER BY created_at DESC LIMIT ?3"
                        .to_string(),
                    vec![
                        Box::new(pid) as Box<dyn rusqlite::types::ToSql>,
                        Box::new(s.to_string()),
                        Box::new(limit as i64),
                    ],
                ),
                (Some(pid), None) => (
                    "SELECT id, uuid, project_id, agent_name, goal, context_json,
                        status, summary, tokens_used, cost_cents,
                        created_at, updated_at, completed_at
                 FROM runs WHERE project_id = ?1
                 ORDER BY created_at DESC LIMIT ?2"
                        .to_string(),
                    vec![
                        Box::new(pid) as Box<dyn rusqlite::types::ToSql>,
                        Box::new(limit as i64),
                    ],
                ),
                (None, Some(s)) => (
                    "SELECT id, uuid, project_id, agent_name, goal, context_json,
                        status, summary, tokens_used, cost_cents,
                        created_at, updated_at, completed_at
                 FROM runs WHERE status = ?1
                 ORDER BY created_at DESC LIMIT ?2"
                        .to_string(),
                    vec![Box::new(s.to_string()), Box::new(limit as i64)],
                ),
                (None, None) => (
                    "SELECT id, uuid, project_id, agent_name, goal, context_json,
                        status, summary, tokens_used, cost_cents,
                        created_at, updated_at, completed_at
                 FROM runs
                 ORDER BY created_at DESC LIMIT ?1"
                        .to_string(),
                    vec![Box::new(limit as i64)],
                ),
            };

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let runs = self
            .conn
            .prepare(&sql)?
            .query_map(param_refs.as_slice(), |r| {
                Ok(StoredRun {
                    id: r.get(0)?,
                    uuid: r.get(1)?,
                    project_id: r.get(2)?,
                    agent_name: r.get(3)?,
                    goal: r.get(4)?,
                    context_json: r.get(5)?,
                    status: r.get(6)?,
                    summary: r.get(7)?,
                    tokens_used: r.get(8)?,
                    cost_cents: r.get(9)?,
                    created_at: r.get(10)?,
                    updated_at: r.get(11)?,
                    completed_at: r.get(12)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(runs)
    }
}
