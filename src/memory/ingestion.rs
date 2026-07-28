#![allow(dead_code)]

use rusqlite::{params, Connection};
use sha2::{Sha256, Digest};
use tracing::info;

use super::repository::{MemoryRepository, NewMemory};

pub struct MemoryIngestion<'a> {
    conn: &'a Connection,
}

#[derive(Debug)]
pub struct IngestResult {
    pub memory_id: i64,
    pub is_duplicate: bool,
    pub superseded_id: Option<i64>,
}

pub fn compute_content_hash(content: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hasher.finalize().to_vec()
}

#[derive(Debug, Clone)]
pub struct IngestParams {
    pub content: String,
    pub memory_type: String,
    pub layer: String,
    pub importance: f64,
    pub source: String,
    pub source_ref: Option<String>,
    pub project_id: Option<i64>,
    pub run_id: Option<i64>,
    pub collection_id: i64,
    pub embedding: Option<Vec<f32>>,
}

impl<'a> MemoryIngestion<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn ingest(&self, params: &IngestParams) -> anyhow::Result<IngestResult> {
        let content_hash = compute_content_hash(&params.content);
        let repo = MemoryRepository::new(self.conn);

        if let Some(existing_id) = repo.find_active_by_hash(params.project_id, params.collection_id, &content_hash)? {
            info!("duplicate memory detected (hash match): existing_id={}", existing_id);
            return Ok(IngestResult {
                memory_id: existing_id,
                is_duplicate: true,
                superseded_id: None,
            });
        }

        let new_mem = NewMemory {
            collection_id: params.collection_id,
            project_id: params.project_id,
            run_id: params.run_id,
            content: params.content.clone(),
            content_hash,
            memory_type: params.memory_type.clone(),
            layer: params.layer.clone(),
            importance: params.importance,
            source: params.source.clone(),
            source_ref: params.source_ref.clone(),
            metadata_json: "{}".to_string(),
        };

        let embedding = params.embedding.as_deref();
        let memory_id = repo.insert_atomic(&new_mem, embedding)?;
        info!("ingested memory: id={}, type={}, layer={}", memory_id, params.memory_type, params.layer);

        Ok(IngestResult {
            memory_id,
            is_duplicate: false,
            superseded_id: None,
        })
    }

    pub fn ingest_with_supersede(
        &self,
        params: &IngestParams,
        similarity_threshold: f64,
    ) -> anyhow::Result<IngestResult> {
        let content_hash = compute_content_hash(&params.content);
        let repo = MemoryRepository::new(self.conn);

        if let Some(existing_id) = repo.find_active_by_hash(params.project_id, params.collection_id, &content_hash)? {
            info!("exact duplicate, skipping: existing_id={}", existing_id);
            return Ok(IngestResult {
                memory_id: existing_id,
                is_duplicate: true,
                superseded_id: None,
            });
        }

        let similar = if let Some(ref embedding) = params.embedding {
            self.find_similar_for_supersede(embedding, params.collection_id, params.project_id, similarity_threshold)?
        } else {
            vec![]
        };

        let new_mem = NewMemory {
            collection_id: params.collection_id,
            project_id: params.project_id,
            run_id: params.run_id,
            content: params.content.clone(),
            content_hash,
            memory_type: params.memory_type.clone(),
            layer: params.layer.clone(),
            importance: params.importance,
            source: params.source.clone(),
            source_ref: params.source_ref.clone(),
            metadata_json: "{}".to_string(),
        };

        let embedding = params.embedding.as_deref();
        let memory_id = repo.insert_atomic(&new_mem, embedding)?;

        let mut superseded_id = None;
        for (old_id, _distance) in &similar {
            repo.supersede(*old_id, memory_id)?;
            superseded_id = Some(*old_id);
            info!("superseded memory: old_id={}, new_id={}", old_id, memory_id);
        }

        Ok(IngestResult {
            memory_id,
            is_duplicate: false,
            superseded_id,
        })
    }

    fn find_similar_for_supersede(
        &self,
        embedding: &[f32],
        collection_id: i64,
        project_id: Option<i64>,
        threshold: f64,
    ) -> anyhow::Result<Vec<(i64, f64)>> {
        let dimensions: i32 = self.conn.query_row(
            "SELECT dimensions FROM embedding_collections WHERE id = ?1",
            params![collection_id],
            |r| r.get(0),
        )?;

        let table_name = format!("vec_mem_{}", dimensions);
        let query_bytes: Vec<u8> = embedding.iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        let results: Vec<(i64, f64)> = self.conn
            .prepare(&format!(
                "SELECT v.vector_id, v.distance
                 FROM {table_name} v
                 JOIN memories m ON m.id = v.vector_id
                 WHERE v.embedding MATCH ?1
                   AND k = 5
                   AND m.lifecycle_status = 'active'
                   AND m.project_id IS ?2
                   AND m.collection_id = ?3
                 ORDER BY v.distance"
            ))?
            .query_map(rusqlite::params![query_bytes, project_id, collection_id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let similar: Vec<(i64, f64)> = results
            .into_iter()
            .filter(|(_, dist)| *dist < threshold)
            .collect();

        Ok(similar)
    }
}
