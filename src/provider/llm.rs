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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
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

#[derive(Debug, Clone)]
pub struct LlmToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct LlmToolResult {
    pub content: String,
    pub tool_calls: Vec<LlmToolCall>,
    pub tokens_used: usize,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(
        &self,
        messages: &[LlmMessage],
        max_tokens: Option<usize>,
        temperature: Option<f32>,
    ) -> Result<LlmResponse, LlmError>;

    /// Tool-calling completion. Default: no tools, falls back to complete.
    async fn complete_with_tools(
        &self,
        messages: &[LlmMessage],
        max_tokens: Option<usize>,
        temperature: Option<f32>,
    ) -> Result<LlmToolResult, LlmError> {
        let resp = self.complete(messages, max_tokens, temperature).await?;
        Ok(LlmToolResult {
            content: resp.content,
            tool_calls: Vec::new(),
            tokens_used: resp.tokens_used,
        })
    }

    async fn complete_stream(
        &self,
        messages: &[LlmMessage],
        max_tokens: Option<usize>,
        temperature: Option<f32>,
        tx: tokio::sync::mpsc::Sender<StreamChunk>,
    ) -> Result<LlmResponse, LlmError> {
        let response = self.complete(messages, max_tokens, temperature).await?;
        let _ = tx
            .send(StreamChunk {
                delta: response.content.clone(),
                finished: true,
                tokens_used: Some(response.tokens_used),
            })
            .await;
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
