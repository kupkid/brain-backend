use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::embedding::{EmbeddingError, EmbeddingProvider};

pub struct OpenAiCompatEmbedding {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    dimensions: usize,
}

#[derive(Serialize)]
struct EmbedRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedData>,
}

#[derive(Deserialize)]
struct EmbedData {
    embedding: Vec<f32>,
}

impl OpenAiCompatEmbedding {
    pub fn new(api_key: String, model: String, base_url: String, dimensions: usize) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            base_url,
            dimensions,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiCompatEmbedding {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let results = self.embed_batch(&[text]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingError::Provider("empty response".to_string()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let request = EmbedRequest {
            input: texts.iter().map(|s| s.to_string()).collect(),
            model: self.model.clone(),
        };

        let url = format!("{}/embeddings", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| EmbeddingError::Provider(format!("request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(EmbeddingError::Provider(format!("HTTP {status}: {body}")));
        }

        let embed_response: EmbedResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::Provider(format!("parse error: {e}")))?;

        let embeddings: Vec<Vec<f32>> = embed_response.data.into_iter().map(|d| d.embedding).collect();

        for emb in &embeddings {
            if emb.len() != self.dimensions {
                return Err(EmbeddingError::DimensionMismatch {
                    expected: self.dimensions,
                    actual: emb.len(),
                });
            }
        }

        info!(
            "embedded {} texts (model={}, dims={})",
            texts.len(),
            self.model,
            self.dimensions
        );
        Ok(embeddings)
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    async fn health_check(&self) -> bool {
        match self.embed("test").await {
            Ok(v) => v.len() == self.dimensions,
            Err(e) => {
                tracing::warn!("embedding health check failed: {e}");
                false
            }
        }
    }
}
