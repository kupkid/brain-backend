use rusqlite::{Connection, OptionalExtension, params};
use tracing::info;

#[derive(Debug, Clone)]
pub struct RunContext {
    pub id: i64,
    pub run_id: i64,
    pub slot: String,
    pub content: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ContextSlot {
    pub slot: String,
    pub content: String,
}

pub struct RunContextRepository<'a> {
    conn: &'a Connection,
}

impl<'a> RunContextRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Upsert a context slot (insert or replace)
    pub fn upsert(&self, run_id: i64, slot: &str, content: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO run_contexts (run_id, slot, content)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(run_id, slot) DO UPDATE SET
                content = excluded.content,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            params![run_id, slot, content],
        )?;
        info!("context upserted: run={}, slot={}", run_id, slot);
        Ok(())
    }

    /// Get a single context slot
    pub fn get(&self, run_id: i64, slot: &str) -> anyhow::Result<Option<RunContext>> {
        self.conn
            .query_row(
                "SELECT id, run_id, slot, content, updated_at
                 FROM run_contexts WHERE run_id = ?1 AND slot = ?2",
                params![run_id, slot],
                |r| {
                    Ok(RunContext {
                        id: r.get(0)?,
                        run_id: r.get(1)?,
                        slot: r.get(2)?,
                        content: r.get(3)?,
                        updated_at: r.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Get all context slots for a run
    pub fn list_by_run(&self, run_id: i64) -> anyhow::Result<Vec<RunContext>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, slot, content, updated_at
             FROM run_contexts WHERE run_id = ?1 ORDER BY slot",
        )?;
        let contexts = stmt
            .query_map(params![run_id], |r| {
                Ok(RunContext {
                    id: r.get(0)?,
                    run_id: r.get(1)?,
                    slot: r.get(2)?,
                    content: r.get(3)?,
                    updated_at: r.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(contexts)
    }

    /// Get all context slots as key-value pairs (for prompt assembly)
    pub fn slots_map(
        &self,
        run_id: i64,
    ) -> anyhow::Result<std::collections::HashMap<String, String>> {
        let contexts = self.list_by_run(run_id)?;
        let map = contexts.into_iter().map(|c| (c.slot, c.content)).collect();
        Ok(map)
    }

    /// Delete a specific context slot
    pub fn delete(&self, run_id: i64, slot: &str) -> anyhow::Result<bool> {
        let affected = self.conn.execute(
            "DELETE FROM run_contexts WHERE run_id = ?1 AND slot = ?2",
            params![run_id, slot],
        )?;
        Ok(affected > 0)
    }

    /// Delete all context slots for a run
    pub fn delete_all(&self, run_id: i64) -> anyhow::Result<usize> {
        let affected = self.conn.execute(
            "DELETE FROM run_contexts WHERE run_id = ?1",
            params![run_id],
        )?;
        Ok(affected)
    }

    /// Bulk upsert multiple slots in a transaction
    pub fn upsert_batch(&self, run_id: i64, slots: &[ContextSlot]) -> anyhow::Result<()> {
        self.conn.execute_batch("BEGIN IMMEDIATE;")?;
        for slot in slots {
            self.conn.execute(
                "INSERT INTO run_contexts (run_id, slot, content)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(run_id, slot) DO UPDATE SET
                    content = excluded.content,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
                params![run_id, slot.slot, slot.content],
            )?;
        }
        self.conn.execute_batch("COMMIT;")?;
        Ok(())
    }
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
        let uuid = crate::db::ids::new_uuid_blob();
        conn.execute(
            "INSERT INTO runs (uuid, agent_name, goal) VALUES (?1, 'test', 'goal')",
            params![uuid],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn upsert_and_get() {
        let conn = setup_db();
        let run_id = create_test_run(&conn);
        let repo = RunContextRepository::new(&conn);

        repo.upsert(run_id, "system_prompt", "You are helpful.")
            .unwrap();
        let ctx = repo.get(run_id, "system_prompt").unwrap().unwrap();
        assert_eq!(ctx.content, "You are helpful.");

        // Upsert overwrites
        repo.upsert(run_id, "system_prompt", "Updated prompt.")
            .unwrap();
        let ctx = repo.get(run_id, "system_prompt").unwrap().unwrap();
        assert_eq!(ctx.content, "Updated prompt.");
    }

    #[test]
    fn list_and_delete() {
        let conn = setup_db();
        let run_id = create_test_run(&conn);
        let repo = RunContextRepository::new(&conn);

        repo.upsert(run_id, "system_prompt", "prompt").unwrap();
        repo.upsert(run_id, "tools_json", "[]").unwrap();

        let all = repo.list_by_run(run_id).unwrap();
        assert_eq!(all.len(), 2);

        assert!(repo.delete(run_id, "tools_json").unwrap());
        let all = repo.list_by_run(run_id).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].slot, "system_prompt");

        let deleted = repo.delete_all(run_id).unwrap();
        assert_eq!(deleted, 1);
    }

    #[test]
    fn bulk_upsert() {
        let conn = setup_db();
        let run_id = create_test_run(&conn);
        let repo = RunContextRepository::new(&conn);

        let slots = vec![
            ContextSlot {
                slot: "a".to_string(),
                content: "1".to_string(),
            },
            ContextSlot {
                slot: "b".to_string(),
                content: "2".to_string(),
            },
        ];
        repo.upsert_batch(run_id, &slots).unwrap();

        let map = repo.slots_map(run_id).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map["a"], "1");
        assert_eq!(map["b"], "2");
    }
}
