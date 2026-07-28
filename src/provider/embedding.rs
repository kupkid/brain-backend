#![allow(dead_code, unused_imports)] // SCAFFOLD — temporary until embedding provider integration

use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("provider error: {0}")]
    Provider(String),

    #[error("dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("rate limited, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("model not available: {0}")]
    ModelUnavailable(String),
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embedding for a single text
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;

    /// Generate embeddings for multiple texts (batch)
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError>;

    /// Get model dimensions
    fn dimensions(&self) -> usize;

    /// Get model name
    fn model_name(&self) -> &str;

    /// Health check
    async fn health_check(&self) -> bool;
}
