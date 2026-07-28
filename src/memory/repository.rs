use rusqlite::{params, Connection, OptionalExtension};
use tracing::info;

use crate::db::ids;

#[derive(Debug, Clone)]
pub struct NewMemory {
    pub collection_id: i64,
    pub project_id: Option<i64>,
    pub run_id: Option<i64>,
    pub content: String,
    pub content_hash: Vec<u8>,
    pub memory_type: String,
    pub layer: String,
    pub importance: f64,
    pub source: String,
    pub source_ref: Option<String>,
    pub metadata_json: String,
}

#[derive(Debug, Clone)]
pub struct StoredMemory {
    pub id: i64,
    pub uuid: Vec<u8>,
    pub collection_id: i64,
    pub project_id: Option<i64>,
    pub content: String,
    pub content_hash: Vec<u8>,
    pub memory_type: String,
    pub layer: String,
    pub importance: f64,
    pub access_count: i64,
    pub lifecycle_status: String,
    pub source: String,
    pub created_at: String,
}

pub struct MemoryRepository<'a> {
    conn: &'a Connection,
}

impl<'a> MemoryRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Insert memory atomically (memory + FTS + vec0 handled by caller)
    pub fn insert(&self, mem: &NewMemory) -> anyhow::Result<i64> {
        let uuid = ids::new_uuid_blob();

        self.conn.execute(
            "INSERT INTO memories
                (uuid, collection_id, project_id, run_id, content, content_hash,
                 memory_type, layer, importance, source, source_ref, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                uuid,
                mem.collection_id,
                mem.project_id,
                mem.run_id,
                mem.content,
                mem.content_hash,
                mem.memory_type,
                mem.layer,
                mem.importance,
                mem.source,
                mem.source_ref,
                mem.metadata_json,
            ],
        )?;

        let id = self.conn.last_insert_rowid();
        Ok(id)
    }

    /// Insert memory + FTS + vec0 atomically
    pub fn insert_atomic(
        &self,
        mem: &NewMemory,
        embedding: Option<&[f32]>,
    ) -> anyhow::Result<i64> {
        self.conn.execute_batch("BEGIN IMMEDIATE;")?;

        let id = match self.insert(mem) {
            Ok(id) => id,
            Err(e) => {
                self.conn.execute_batch("ROLLBACK;")?;
                return Err(e);
            }
        };

        // Insert embedding if provided
        if let Some(vec) = embedding {
            if let Err(e) = self.insert_embedding(id, mem.collection_id, vec) {
                self.conn.execute_batch("ROLLBACK;")?;
                return Err(e);
            }
        }

        self.conn.execute_batch("COMMIT;")?;
        Ok(id)
    }

    fn insert_embedding(&self, memory_id: i64, collection_id: i64, embedding: &[f32]) -> anyhow::Result<()> {
        // Get dimensions from collection
        let dimensions: i32 = self.conn.query_row(
            "SELECT dimensions FROM embedding_collections WHERE id = ?1",
            params![collection_id],
            |r| r.get(0),
        )?;

        let table_name = format!("vec_mem_{}", dimensions);

        // Use zerocopy to convert f32 slice to bytes
        let embedding_bytes: Vec<u8> = embedding.iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        self.conn.execute(
            &format!("INSERT INTO {table_name} (vector_id, embedding) VALUES (?1, ?2)"),
            params![memory_id, embedding_bytes],
        )?;

        Ok(())
    }

    /// Get active memory by id
    pub fn get_active(&self, id: i64) -> anyhow::Result<Option<StoredMemory>> {
        self.conn
            .query_row(
                "SELECT id, uuid, collection_id, project_id, content, content_hash,
                        memory_type, layer, importance, access_count, lifecycle_status,
                        source, created_at
                 FROM memories WHERE id = ?1 AND lifecycle_status = 'active'",
                params![id],
                |r| {
                    Ok(StoredMemory {
                        id: r.get(0)?,
                        uuid: r.get(1)?,
                        collection_id: r.get(2)?,
                        project_id: r.get(3)?,
                        content: r.get(4)?,
                        content_hash: r.get(5)?,
                        memory_type: r.get(6)?,
                        layer: r.get(7)?,
                        importance: r.get(8)?,
                        access_count: r.get(9)?,
                        lifecycle_status: r.get(10)?,
                        source: r.get(11)?,
                        created_at: r.get(12)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Check for duplicate by content_hash in active memories
    pub fn find_active_by_hash(
        &self,
        project_id: Option<i64>,
        collection_id: i64,
        content_hash: &[u8],
    ) -> anyhow::Result<Option<i64>> {
        self.conn
            .query_row(
                "SELECT id FROM memories
                 WHERE project_id IS ?1 AND collection_id = ?2
                   AND content_hash = ?3 AND lifecycle_status = 'active'",
                params![project_id, collection_id, content_hash],
                |r| r.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Supersede old memory: set superseded_by and lifecycle_status
    pub fn supersede(&self, old_id: i64, new_id: i64) -> anyhow::Result<()> {
        self.conn.execute(
            "UPDATE memories
             SET superseded_by = ?1, lifecycle_status = 'superseded',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?2 AND lifecycle_status = 'active'",
            params![new_id, old_id],
        )?;
        Ok(())
    }

    /// Archive memories older than threshold
    pub fn archive_stale(&self, project_id: Option<i64>, layer: &str, max_age_days: i64) -> anyhow::Result<usize> {
        let affected = self.conn.execute(
            "UPDATE memories
             SET lifecycle_status = 'archived',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE lifecycle_status = 'active'
               AND project_id IS ?1
               AND layer = ?2
               AND access_count = 0
               AND created_at < datetime('now', '-' || ?3 || ' days')",
            params![project_id, layer, max_age_days],
        )?;
        if affected > 0 {
            info!("archived {} stale {} memories", affected, layer);
        }
        Ok(affected)
    }

    /// Increment access count
    pub fn touch(&self, id: i64) -> anyhow::Result<()> {
        self.conn.execute(
            "UPDATE memories
             SET access_count = access_count + 1,
                 last_accessed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Batch touch (update access counts for multiple memories)
    pub fn touch_batch(&self, ids: &[i64]) -> anyhow::Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "UPDATE memories
             SET access_count = access_count + 1,
                 last_accessed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id IN ({})",
            placeholders.join(",")
        );
        let params: Vec<Box<dyn rusqlite::types::ToSql>> = ids
            .iter()
            .map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        self.conn.execute(&sql, param_refs.as_slice())?;
        Ok(())
    }

    /// Delete memory (hard delete, only for alpha retention)
    pub fn hard_delete(&self, id: i64) -> anyhow::Result<bool> {
        let affected = self.conn.execute(
            "DELETE FROM memories WHERE id = ?1",
            params![id],
        )?;
        Ok(affected > 0)
    }
}
