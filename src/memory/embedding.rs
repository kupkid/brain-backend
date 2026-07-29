#![allow(dead_code)]

use rusqlite::{Connection, params};

pub struct MemoryEmbeddingStore<'a> {
    conn: &'a Connection,
}

#[derive(Debug)]
pub struct KnnResult {
    pub memory_id: i64,
    pub distance: f64,
}

impl<'a> MemoryEmbeddingStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn search_knn(
        &self,
        query_embedding: &[f32],
        dimensions: i32,
        k: usize,
        exclude_memory_ids: &[i64],
    ) -> anyhow::Result<Vec<KnnResult>> {
        let valid = matches!(dimensions, 384 | 768 | 1024 | 1536 | 3072);
        anyhow::ensure!(valid, "dimensions {} not in whitelist", dimensions);

        let table_name = format!("vec_mem_{}", dimensions);
        let query_bytes: Vec<u8> = query_embedding
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        let (sql, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
            if exclude_memory_ids.is_empty() {
                (
                    format!(
                        "SELECT vector_id, distance FROM {table_name}
                     WHERE embedding MATCH ?1 AND k = ?2
                     ORDER BY distance"
                    ),
                    vec![
                        Box::new(query_bytes) as Box<dyn rusqlite::types::ToSql>,
                        Box::new(k as i64),
                    ],
                )
            } else {
                let placeholders: Vec<String> =
                    exclude_memory_ids.iter().map(|_| "?".to_string()).collect();
                (
                    format!(
                        "SELECT vector_id, distance FROM {table_name}
                     WHERE embedding MATCH ?1 AND k = ?2
                       AND vector_id NOT IN ({})
                     ORDER BY distance",
                        placeholders.join(",")
                    ),
                    {
                        let mut p: Vec<Box<dyn rusqlite::types::ToSql>> =
                            vec![Box::new(query_bytes), Box::new(k as i64)];
                        for id in exclude_memory_ids {
                            p.push(Box::new(*id));
                        }
                        p
                    },
                )
            };

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let results = self
            .conn
            .prepare(&sql)?
            .query_map(param_refs.as_slice(), |r| {
                Ok(KnnResult {
                    memory_id: r.get(0)?,
                    distance: r.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    pub fn remove(&self, memory_id: i64, dimensions: i32) -> anyhow::Result<()> {
        let table_name = format!("vec_mem_{}", dimensions);
        self.conn.execute(
            &format!("DELETE FROM {table_name} WHERE vector_id = ?1"),
            params![memory_id],
        )?;
        Ok(())
    }

    pub fn count(&self, dimensions: i32) -> anyhow::Result<i64> {
        let table_name = format!("vec_mem_{}", dimensions);
        let count: i64 =
            self.conn
                .query_row(&format!("SELECT COUNT(*) FROM {table_name}"), [], |r| {
                    r.get(0)
                })?;
        Ok(count)
    }
}
