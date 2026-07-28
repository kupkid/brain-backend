use rusqlite::{params, Connection};
use tracing::info;

use crate::provider::embedding::EmbeddingProvider;
use super::repository::{MemoryRepository, StoredMemory};

pub struct MemoryRetriever<'a> {
    conn: &'a Connection,
}

#[derive(Debug)]
pub struct RetrievalResult {
    pub memories: Vec<StoredMemory>,
    pub scores: Vec<(i64, f64)>, // (memory_id, fused_score)
}

impl<'a> MemoryRetriever<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Hybrid retrieval: FTS5 + vec0 KNN + recency boost
    pub async fn retrieve(
        &self,
        query: &str,
        project_id: Option<i64>,
        collection_id: i64,
        embedding_provider: &dyn EmbeddingProvider,
        limit: usize,
    ) -> anyhow::Result<RetrievalResult> {
        // 1. FTS5 search
        let fts_results = self.fts_search(query, project_id, limit * 2)?;

        // 2. Vector search (if embedding available)
        let vec_results = match embedding_provider.embed(query).await {
            Ok(query_embedding) => {
                let dimensions: i32 = self.conn.query_row(
                    "SELECT dimensions FROM embedding_collections WHERE id = ?1",
                    params![collection_id],
                    |r| r.get(0),
                )?;
                self.vec_search(&query_embedding, dimensions, project_id, collection_id, limit * 2)?
            }
            Err(e) => {
                info!("embedding failed for retrieval, using FTS only: {}", e);
                vec![]
            }
        };

        // 3. Merge with RRF (Reciprocal Rank Fusion)
        let merged = self.reciprocal_rank_fusion(&fts_results, &vec_results, limit);

        // 4. Load full memories
        let repo = MemoryRepository::new(self.conn);
        let mut memories = Vec::new();
        let mut scores = Vec::new();

        for (memory_id, score) in &merged {
            if let Some(mem) = repo.get_active(*memory_id)? {
                memories.push(mem);
                scores.push((*memory_id, *score));
            }
        }

        // 5. Touch accessed memories (fire and forget)
        let ids: Vec<i64> = scores.iter().map(|(id, _)| *id).collect();
        let _ = repo.touch_batch(&ids);

        Ok(RetrievalResult { memories, scores })
    }

    fn fts_search(
        &self,
        query: &str,
        project_id: Option<i64>,
        limit: usize,
    ) -> anyhow::Result<Vec<(i64, f64)>> {
        let (sql, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if project_id.is_some() {
            (
                "SELECT m.id, fts.rank
                 FROM memories_fts fts
                 JOIN memories m ON m.id = fts.rowid
                 WHERE memories_fts MATCH ?1
                   AND m.lifecycle_status = 'active'
                   AND m.project_id = ?2
                 ORDER BY fts.rank
                 LIMIT ?3".to_string(),
                vec![
                    Box::new(query.to_string()) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(project_id.unwrap()),
                    Box::new(limit as i64),
                ],
            )
        } else {
            (
                "SELECT m.id, fts.rank
                 FROM memories_fts fts
                 JOIN memories m ON m.id = fts.rowid
                 WHERE memories_fts MATCH ?1
                   AND m.lifecycle_status = 'active'
                 ORDER BY fts.rank
                 LIMIT ?2".to_string(),
                vec![
                    Box::new(query.to_string()) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(limit as i64),
                ],
            )
        };

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
        let results = self.conn
            .prepare(&sql)?
            .query_map(param_refs.as_slice(), |r| {
                let id: i64 = r.get(0)?;
                let rank: f64 = r.get(1)?;
                // Convert rank to score (FTS5 rank is negative, lower = better)
                Ok((id, -rank))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    fn vec_search(
        &self,
        query_embedding: &[f32],
        dimensions: i32,
        project_id: Option<i64>,
        collection_id: i64,
        limit: usize,
    ) -> anyhow::Result<Vec<(i64, f64)>> {
        let table_name = format!("vec_mem_{}", dimensions);
        let query_bytes: Vec<u8> = query_embedding.iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        let results = self.conn
            .prepare(&format!(
                "SELECT v.vector_id, v.distance
                 FROM {table_name} v
                 JOIN memories m ON m.id = v.vector_id
                 WHERE v.embedding MATCH ?1
                   AND m.lifecycle_status = 'active'
                   AND m.project_id IS ?2
                   AND m.collection_id = ?3
                 ORDER BY v.distance
                 LIMIT ?4"
            ))?
            .query_map(rusqlite::params![query_bytes, project_id, collection_id, limit as i64], |r| {
                let id: i64 = r.get(0)?;
                let distance: f64 = r.get(1)?;
                // Convert distance to similarity score (lower distance = higher score)
                Ok((id, 1.0 / (1.0 + distance)))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    fn reciprocal_rank_fusion(
        &self,
        fts_results: &[(i64, f64)],
        vec_results: &[(i64, f64)],
        limit: usize,
    ) -> Vec<(i64, f64)> {
        let k = 60.0; // RRF constant
        let mut scores: std::collections::HashMap<i64, f64> = std::collections::HashMap::new();

        // FTS scores
        for (rank, (id, _)) in fts_results.iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as f64 + 1.0);
            *scores.entry(*id).or_insert(0.0) += rrf_score;
        }

        // Vector scores
        for (rank, (id, _)) in vec_results.iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as f64 + 1.0);
            *scores.entry(*id).or_insert(0.0) += rrf_score;
        }

        // Boost by importance and recency
        let mut scored: Vec<(i64, f64)> = scores.into_iter().collect();
        for (id, score) in &mut scored {
            if let Ok(mem) = self.conn.query_row(
                "SELECT importance, layer, access_count, created_at
                 FROM memories WHERE id = ?1 AND lifecycle_status = 'active'",
                params![*id],
                |r| {
                    Ok((
                        r.get::<_, f64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                },
            ) {
                let (importance, layer, access_count, _created_at) = mem;
                let mut boost = 1.0 + (importance * 0.5);
                if layer == "long_term" {
                    boost *= 1.5;
                }
                boost *= 1.0 + (*score).min(access_count as f64).ln();
                *score *= boost;
            }
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }
}
