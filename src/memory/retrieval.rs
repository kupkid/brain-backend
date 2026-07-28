#![allow(dead_code)]

use rusqlite::{params, Connection};

use super::repository::{MemoryRepository, StoredMemory};

pub struct MemoryRetriever<'a> {
    conn: &'a Connection,
}

#[derive(Debug)]
pub struct RetrievalResult {
    pub memories: Vec<StoredMemory>,
    pub scores: Vec<(i64, f64)>,
}

impl<'a> MemoryRetriever<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn retrieve(
        &self,
        query: &str,
        project_id: Option<i64>,
        collection_id: i64,
        embedding: Option<&[f32]>,
        limit: usize,
    ) -> anyhow::Result<RetrievalResult> {
        let fts_results = self.fts_search(query, project_id, limit * 2)?;

        let vec_results = if let Some(query_embedding) = embedding {
            let dimensions: i32 = self.conn.query_row(
                "SELECT dimensions FROM embedding_collections WHERE id = ?1",
                params![collection_id],
                |r| r.get(0),
            )?;
            self.vec_search(query_embedding, dimensions, project_id, collection_id, limit * 2)?
        } else {
            vec![]
        };

        let merged = self.reciprocal_rank_fusion(&fts_results, &vec_results, limit);

        let repo = MemoryRepository::new(self.conn);
        let mut memories = Vec::new();
        let mut scores = Vec::new();

        for (memory_id, score) in &merged {
            if let Some(mem) = repo.get_active(*memory_id)? {
                memories.push(mem);
                scores.push((*memory_id, *score));
            }
        }

        let ids: Vec<i64> = scores.iter().map(|(id, _)| *id).collect();
        let _ = repo.touch_batch(&ids);

        Ok(RetrievalResult { memories, scores })
    }

    pub fn fts_search(
        &self,
        query: &str,
        project_id: Option<i64>,
        limit: usize,
    ) -> anyhow::Result<Vec<(i64, f64)>> {
        let (sql, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(pid) = project_id {
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
                    Box::new(pid),
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
                   AND k = ?4
                   AND m.lifecycle_status = 'active'
                   AND m.project_id IS ?2
                   AND m.collection_id = ?3
                 ORDER BY v.distance"
            ))?
            .query_map(rusqlite::params![query_bytes, project_id, collection_id, limit as i64], |r| {
                let id: i64 = r.get(0)?;
                let distance: f64 = r.get(1)?;
                Ok((id, 1.0 / (1.0 + distance)))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    pub fn reciprocal_rank_fusion(
        &self,
        fts_results: &[(i64, f64)],
        vec_results: &[(i64, f64)],
        limit: usize,
    ) -> Vec<(i64, f64)> {
        let k = 60.0;
        let mut scores: std::collections::HashMap<i64, f64> = std::collections::HashMap::new();

        for (rank, (id, _)) in fts_results.iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as f64 + 1.0);
            *scores.entry(*id).or_insert(0.0) += rrf_score;
        }

        for (rank, (id, _)) in vec_results.iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as f64 + 1.0);
            *scores.entry(*id).or_insert(0.0) += rrf_score;
        }

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
                if layer == "project" || layer == "global_profile" {
                    boost *= 1.5;
                }
                if access_count > 0 {
                    boost *= 1.0 + (access_count as f64).ln();
                }
                *score *= boost;
            }
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }
}
