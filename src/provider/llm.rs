#![allow(dead_code, unused_imports)]

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

/// Streaming chunk from LLM
#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub delta: String,
    pub finished: bool,
    pub tokens_used: Option<usize>,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(
        &self,
        messages: &[LlmMessage],
        max_tokens: Option<usize>,
        temperature: Option<f32>,
    ) -> Result<LlmResponse, LlmError>;

    /// Streaming completion — yields chunks via channel
    async fn complete_stream(
        &self,
        messages: &[LlmMessage],
        max_tokens: Option<usize>,
        temperature: Option<f32>,
        tx: tokio::sync::mpsc::Sender<StreamChunk>,
    ) -> Result<LlmResponse, LlmError> {
        // Default: fall back to non-streaming
        let response = self.complete(messages, max_tokens, temperature).await?;
        let _ = tx.send(StreamChunk {
            delta: response.content.clone(),
            finished: true,
            tokens_used: Some(response.tokens_used),
        }).await;
        Ok(response)
    }

    async fn structured_complete(
        &self,
        messages: &[LlmMessage],
        schema: &serde_json::Value,
        max_tokens: Option<usize>,
    ) -> Result<StructuredOutput, LlmError>;

    fn model_name(&self) -> &str;
    async fn health_check(&self) -> bool;
}
