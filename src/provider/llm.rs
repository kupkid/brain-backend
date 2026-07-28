#![allow(dead_code, unused_imports)] // SCAFFOLD — temporary until LLM provider integration

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("provider error: {0}")]
    Provider(String),

    #[error("invalid JSON output: {0}")]
    InvalidJson(String),

    #[error("rate limited")]
    RateLimited,

    #[error("context length exceeded: {tokens} tokens")]
    ContextLengthExceeded { tokens: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub tokens_used: usize,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredOutput {
    pub json: serde_json::Value,
    pub tokens_used: usize,
    pub model: String,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Generate a completion
    async fn complete(
        &self,
        messages: &[LlmMessage],
        max_tokens: Option<usize>,
        temperature: Option<f32>,
    ) -> Result<LlmResponse, LlmError>;

    /// Generate structured JSON output with schema validation
    async fn structured_complete(
        &self,
        messages: &[LlmMessage],
        schema: &serde_json::Value,
        max_tokens: Option<usize>,
    ) -> Result<StructuredOutput, LlmError>;

    /// Get model name
    fn model_name(&self) -> &str;

    /// Health check
    async fn health_check(&self) -> bool;
}
