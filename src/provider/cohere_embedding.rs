use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::embedding::{EmbeddingError, EmbeddingProvider};

pub struct CohereEmbedding {
    client: Client,
    api_key: String,
    model: String,
    dimensions: usize,
}

#[derive(Serialize)]
struct EmbedRequest {
    texts: Vec<String>,
    model: String,
    input_type: String,
    #[serde(rename = "embeddingTypes")]
    embedding_types: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum EmbedResponse {
    Typed(TypedData),
    Direct(DirectData),
}

#[derive(Deserialize)]
struct TypedData {
    embeddings: TypedEmbeddings,
}

#[derive(Deserialize)]
struct TypedEmbeddings {
    #[serde(rename = "float")]
    float_vec: Option<Vec<Vec<f32>>>,
}

#[derive(Deserialize)]
struct DirectData {
    embeddings: DirectEmbeddings,
}

#[derive(Deserialize)]
struct DirectEmbeddings {
    float: Vec<Vec<f32>>,
}

impl CohereEmbedding {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        let model = model.unwrap_or_else(|| "embed-multilingual-v3.0".to_string());
        let dimensions = match model.as_str() {
            "embed-english-v3.0" => 1024,
            "embed-multilingual-v3.0" => 1024,
            "embed-english-light-v3.0" => 384,
            "embed-multilingual-light-v3.0" => 384,
            _ => 1024,
        };

        Self {
            client: Client::new(),
            api_key,
            model,
            dimensions,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for CohereEmbedding {
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
            texts: texts.iter().map(|s| s.to_string()).collect(),
            model: self.model.clone(),
            input_type: "search_document".to_string(),
            embedding_types: vec!["float".to_string()],
        };

        let response = self
            .client
            .post("https://api.cohere.ai/v1/embed")
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

        let embeddings = match embed_response {
            EmbedResponse::Typed(t) => {
                t.embeddings
                    .float_vec
                    .ok_or_else(|| EmbeddingError::Provider("no float embeddings".to_string()))?
            }
            EmbedResponse::Direct(d) => d.embeddings.float,
        };

        for emb in &embeddings {
            if emb.len() != self.dimensions {
                return Err(EmbeddingError::DimensionMismatch {
                    expected: self.dimensions,
                    actual: emb.len(),
                });
            }
        }

        info!("embedded {} texts (model={}, dims={})", texts.len(), self.model, self.dimensions);
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
            Err(_) => false,
        }
    }
}
