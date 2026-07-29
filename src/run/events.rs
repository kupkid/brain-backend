use rusqlite::{Connection, OptionalExtension, params};

use crate::db::ids;

#[derive(Debug, Clone)]
pub struct RunEvent {
    pub id: i64,
    pub run_id: i64,
    pub event_uuid: Vec<u8>,
    pub seq: i64,
    pub event_type: String,
    pub payload: String,
    pub created_at: String,
}

pub struct EventStore<'a> {
    conn: &'a Connection,
}

impl<'a> EventStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Allocate next seq atomically for a run
    pub fn allocate_seq(&self, run_id: i64) -> anyhow::Result<i64> {
        // Ensure counter exists (created with run in same transaction)
        let seq: i64 = self.conn.query_row(
            "UPDATE run_seq_counters SET next_seq = next_seq + 1
             WHERE run_id = ?1
             RETURNING next_seq - 1",
            params![run_id],
            |r| r.get(0),
        )?;
        Ok(seq)
    }

    /// Insert event with idempotency (event_uuid)
    pub fn insert_event(
        &self,
        run_id: i64,
        event_type: &str,
        payload: &str,
    ) -> anyhow::Result<RunEvent> {
        let event_uuid = ids::new_uuid_blob();
        let seq = self.allocate_seq(run_id)?;

        self.conn.execute(
            "INSERT INTO run_events (run_id, event_uuid, seq, event_type, payload)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![run_id, event_uuid, seq, event_type, payload],
        )?;

        let id = self.conn.last_insert_rowid();

        Ok(RunEvent {
            id,
            run_id,
            event_uuid,
            seq,
            event_type: event_type.to_string(),
            payload: payload.to_string(),
            created_at: String::new(), // Will be filled by DB default
        })
    }

    /// Check if event already exists (idempotency check)
    pub fn event_exists(&self, run_id: i64, event_uuid: &[u8]) -> anyhow::Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM run_events WHERE run_id = ?1 AND event_uuid = ?2",
            params![run_id, event_uuid],
            |r| r.get(0),
        )?;
        Ok(count > 0)
    }

    /// Get events for a run (ordered by seq)
    pub fn get_events(&self, run_id: i64) -> anyhow::Result<Vec<RunEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, event_uuid, seq, event_type, payload, created_at
             FROM run_events WHERE run_id = ?1 ORDER BY seq",
        )?;
        let events = stmt
            .query_map(params![run_id], |r| {
                Ok(RunEvent {
                    id: r.get(0)?,
                    run_id: r.get(1)?,
                    event_uuid: r.get(2)?,
                    seq: r.get(3)?,
                    event_type: r.get(4)?,
                    payload: r.get(5)?,
                    created_at: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }

    /// Get events after a specific seq (for WebSocket replay)
    pub fn get_events_after(&self, run_id: i64, after_seq: i64) -> anyhow::Result<Vec<RunEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, event_uuid, seq, event_type, payload, created_at
             FROM run_events WHERE run_id = ?1 AND seq > ?2 ORDER BY seq",
        )?;
        let events = stmt
            .query_map(params![run_id, after_seq], |r| {
                Ok(RunEvent {
                    id: r.get(0)?,
                    run_id: r.get(1)?,
                    event_uuid: r.get(2)?,
                    seq: r.get(3)?,
                    event_type: r.get(4)?,
                    payload: r.get(5)?,
                    created_at: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }

    /// Get last seq for a run (for WebSocket handshake)
    pub fn last_seq(&self, run_id: i64) -> anyhow::Result<Option<i64>> {
        self.conn
            .query_row(
                "SELECT MAX(seq) FROM run_events WHERE run_id = ?1",
                params![run_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(Into::into)
    }
}
