#![allow(dead_code)]

use rusqlite::{Connection, OptionalExtension, params};
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

        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_atomic(&self, mem: &NewMemory, embedding: Option<&[f32]>) -> anyhow::Result<i64> {
        self.conn.execute_batch("BEGIN IMMEDIATE;")?;

        let id = match self.insert(mem) {
            Ok(id) => id,
            Err(e) => {
                self.conn.execute_batch("ROLLBACK;")?;
                return Err(e);
            }
        };

        if let Some(vec) = embedding
            && let Err(e) = self.insert_embedding(id, mem.collection_id, vec)
        {
            self.conn.execute_batch("ROLLBACK;")?;
            return Err(e);
        }

        self.conn.execute_batch("COMMIT;")?;
        Ok(id)
    }

    fn insert_embedding(
        &self,
        memory_id: i64,
        collection_id: i64,
        embedding: &[f32],
    ) -> anyhow::Result<()> {
        let dimensions: i32 = self.conn.query_row(
            "SELECT dimensions FROM embedding_collections WHERE id = ?1",
            params![collection_id],
            |r| r.get(0),
        )?;

        let table_name = format!("vec_mem_{}", dimensions);
        let embedding_bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();

        self.conn.execute(
            &format!("INSERT INTO {table_name} (vector_id, embedding) VALUES (?1, ?2)"),
            params![memory_id, embedding_bytes],
        )?;

        Ok(())
    }

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

    pub fn list_by_project(
        &self,
        project_id: i64,
        layer: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<StoredMemory>> {
        let (sql, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match layer {
            Some(l) => (
                "SELECT id, uuid, collection_id, project_id, content, content_hash,
                        memory_type, layer, importance, access_count, lifecycle_status,
                        source, created_at
                 FROM memories
                 WHERE project_id = ?1 AND layer = ?2 AND lifecycle_status = 'active'
                 ORDER BY importance DESC, created_at DESC
                 LIMIT ?3"
                    .to_string(),
                vec![
                    Box::new(project_id) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(l.to_string()),
                    Box::new(limit as i64),
                ],
            ),
            None => (
                "SELECT id, uuid, collection_id, project_id, content, content_hash,
                        memory_type, layer, importance, access_count, lifecycle_status,
                        source, created_at
                 FROM memories
                 WHERE project_id = ?1 AND lifecycle_status = 'active'
                 ORDER BY importance DESC, created_at DESC
                 LIMIT ?2"
                    .to_string(),
                vec![
                    Box::new(project_id) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(limit as i64),
                ],
            ),
        };

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let rows = self
            .conn
            .prepare(&sql)?
            .query_map(param_refs.as_slice(), |r| {
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
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub fn list_global_profile(&self, limit: usize) -> anyhow::Result<Vec<StoredMemory>> {
        let rows = self
            .conn
            .prepare(
                "SELECT id, uuid, collection_id, project_id, content, content_hash,
                    memory_type, layer, importance, access_count, lifecycle_status,
                    source, created_at
             FROM memories
             WHERE layer = 'global_profile' AND project_id IS NULL AND lifecycle_status = 'active'
             ORDER BY importance DESC, created_at DESC
             LIMIT ?1",
            )?
            .query_map(params![limit as i64], |r| {
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
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub fn list_by_layer(
        &self,
        layer: &str,
        project_id: Option<i64>,
        limit: usize,
    ) -> anyhow::Result<Vec<StoredMemory>> {
        let rows = self
            .conn
            .prepare(
                "SELECT id, uuid, collection_id, project_id, content, content_hash,
                    memory_type, layer, importance, access_count, lifecycle_status,
                    source, created_at
             FROM memories
             WHERE layer = ?1 AND project_id IS ?2 AND lifecycle_status = 'active'
             ORDER BY importance DESC, created_at DESC
             LIMIT ?3",
            )?
            .query_map(params![layer, project_id, limit as i64], |r| {
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
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

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

    pub fn archive_stale(
        &self,
        project_id: Option<i64>,
        layer: &str,
        max_age_days: i64,
    ) -> anyhow::Result<usize> {
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
        let param_values: Vec<Box<dyn rusqlite::types::ToSql>> = ids
            .iter()
            .map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        self.conn.execute(&sql, param_refs.as_slice())?;
        Ok(())
    }

    pub fn hard_delete(&self, id: i64) -> anyhow::Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn count_active(&self, project_id: Option<i64>) -> anyhow::Result<i64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE project_id IS ?1 AND lifecycle_status = 'active'",
            params![project_id],
            |r| r.get(0),
        ).map_err(Into::into)
    }
}
