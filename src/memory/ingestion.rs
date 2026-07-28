use rusqlite::Connection;
use sha2::{Sha256, Digest};
use tracing::{info, warn};

use crate::provider::embedding::EmbeddingProvider;
use crate::provider::llm::LlmProvider;
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

impl<'a> MemoryIngestion<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Ingest a new memory: dedup → extract → embed → store atomically
    pub async fn ingest(
        &self,
        content: &str,
        memory_type: &str,
        layer: &str,
        importance: f64,
        source: &str,
        source_ref: Option<&str>,
        project_id: Option<i64>,
        run_id: Option<i64>,
        collection_id: i64,
        embedding_provider: &dyn EmbeddingProvider,
    ) -> anyhow::Result<IngestResult> {
        // 1. Compute content hash
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let content_hash = hasher.finalize().to_vec();

        let repo = MemoryRepository::new(self.conn);

        // 2. Check for duplicate
        if let Some(existing_id) = repo.find_active_by_hash(project_id, collection_id, &content_hash)? {
            info!("duplicate memory detected (hash match), skipping: existing_id={}", existing_id);
            return Ok(IngestResult {
                memory_id: existing_id,
                is_duplicate: true,
                superseded_id: None,
            });
        }

        // 3. Generate embedding
        let embedding = match embedding_provider.embed(content).await {
            Ok(e) => Some(e),
            Err(e) => {
                warn!("embedding generation failed, storing without vector: {}", e);
                None
            }
        };

        // 4. Store atomically (memory + FTS + vec0)
        let new_mem = NewMemory {
            collection_id,
            project_id,
            run_id,
            content: content.to_string(),
            content_hash,
            memory_type: memory_type.to_string(),
            layer: layer.to_string(),
            importance,
            source: source.to_string(),
            source_ref: source_ref.map(|s| s.to_string()),
            metadata_json: "{}".to_string(),
        };

        let memory_id = repo.insert_atomic(&new_mem, embedding.as_deref())?;
        info!("ingested memory: id={}, type={}, layer={}", memory_id, memory_type, layer);

        Ok(IngestResult {
            memory_id,
            is_duplicate: false,
            superseded_id: None,
        })
    }

    /// Ingest with supersede: if similar memory exists, supersede it
    pub async fn ingest_with_supersede(
        &self,
        content: &str,
        memory_type: &str,
        layer: &str,
        importance: f64,
        source: &str,
        source_ref: Option<&str>,
        project_id: Option<i64>,
        run_id: Option<i64>,
        collection_id: i64,
        embedding_provider: &dyn EmbeddingProvider,
        similarity_threshold: f64,
    ) -> anyhow::Result<IngestResult> {
        // 1. Compute hash and check dedup
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let content_hash = hasher.finalize().to_vec();

        let repo = MemoryRepository::new(self.conn);
        if let Some(existing_id) = repo.find_active_by_hash(project_id, collection_id, &content_hash)? {
            info!("exact duplicate, skipping: existing_id={}", existing_id);
            return Ok(IngestResult {
                memory_id: existing_id,
                is_duplicate: true,
                superseded_id: None,
            });
        }

        // 2. Generate embedding
        let embedding = match embedding_provider.embed(content).await {
            Ok(e) => e,
            Err(e) => {
                warn!("embedding failed for supersede check: {}", e);
                // Fall back to simple ingest without supersede
                return self.ingest(
                    content, memory_type, layer, importance, source, source_ref,
                    project_id, run_id, collection_id, embedding_provider,
                ).await;
            }
        };

        // 3. Find similar active memories for potential supersede
        let similar = self.find_similar_for_supersede(
            &embedding, collection_id, project_id, similarity_threshold,
        )?;

        // 4. Store new memory
        let new_mem = NewMemory {
            collection_id,
            project_id,
            run_id,
            content: content.to_string(),
            content_hash,
            memory_type: memory_type.to_string(),
            layer: layer.to_string(),
            importance,
            source: source.to_string(),
            source_ref: source_ref.map(|s| s.to_string()),
            metadata_json: "{}".to_string(),
        };

        let memory_id = repo.insert_atomic(&new_mem, Some(&embedding))?;

        // 5. Supersede similar memories
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
        // Get dimensions from collection
        let dimensions: i32 = self.conn.query_row(
            "SELECT dimensions FROM embedding_collections WHERE id = ?1",
            params![collection_id],
            |r| r.get(0),
        )?;

        let table_name = format!("vec_mem_{}", dimensions);
        let query_bytes: Vec<u8> = embedding.iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        // Search for very similar memories (low distance = high similarity)
        let results: Vec<(i64, f64)> = self.conn
            .prepare(&format!(
                "SELECT v.vector_id, v.distance
                 FROM {table_name} v
                 JOIN memories m ON m.id = v.vector_id
                 WHERE v.embedding MATCH ?1
                   AND m.lifecycle_status = 'active'
                   AND m.project_id IS ?2
                   AND m.collection_id = ?3
                 ORDER BY v.distance
                 LIMIT 5"
            ))?
            .query_map(rusqlite::params![query_bytes, project_id, collection_id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // Filter by threshold (for cosine distance: low distance = high similarity)
        let similar: Vec<(i64, f64)> = results
            .into_iter()
            .filter(|(_, dist)| *dist < threshold)
            .collect();

        Ok(similar)
    }
}

use rusqlite::params;
