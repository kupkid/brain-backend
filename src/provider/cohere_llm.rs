use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::llm::{LlmError, LlmMessage, LlmProvider, LlmResponse, StructuredOutput};

pub struct CohereLlm {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    usage: Option<ChatUsage>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatUsage {
    total_tokens: Option<usize>,
}

impl CohereLlm {
    pub fn new(api_key: String, model: Option<String>, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "command-a-plus-05-2026".to_string()),
            base_url: base_url.unwrap_or_else(|| "https://api.cohere.ai/compatibility/v1".to_string()),
        }
    }

    async fn send_request(&self, request: ChatRequest) -> Result<ChatResponse, LlmError> {
        let url = format!("{}/chat/completions", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::Provider(format!("request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LlmError::Provider(format!("HTTP {status}: {body}")));
        }

        response
            .json()
            .await
            .map_err(|e| LlmError::Provider(format!("parse error: {e}")))
    }
}

#[async_trait]
impl LlmProvider for CohereLlm {
    async fn complete(
        &self,
        messages: &[LlmMessage],
        max_tokens: Option<usize>,
        temperature: Option<f32>,
    ) -> Result<LlmResponse, LlmError> {
        let chat_messages: Vec<ChatMessage> = messages
            .iter()
            .map(|m| ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let request = ChatRequest {
            model: self.model.clone(),
            messages: chat_messages,
            max_tokens,
            temperature,
        };

        let response = self.send_request(request).await?;
        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();
        let tokens_used = response.usage.and_then(|u| u.total_tokens).unwrap_or(0);

        info!("LLM complete: model={}, tokens={}", self.model, tokens_used);

        Ok(LlmResponse {
            content,
            tokens_used,
            model: self.model.clone(),
        })
    }

    async fn structured_complete(
        &self,
        messages: &[LlmMessage],
        _schema: &serde_json::Value,
        max_tokens: Option<usize>,
    ) -> Result<StructuredOutput, LlmError> {
        let response = self.complete(messages, max_tokens, None).await?;

        let json: serde_json::Value = serde_json::from_str(&response.content)
            .map_err(|e| LlmError::InvalidJson(format!("{e}: {}", response.content)))?;

        Ok(StructuredOutput {
            json,
            tokens_used: response.tokens_used,
            model: response.model,
        })
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    async fn health_check(&self) -> bool {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "hi".to_string(),
            }],
            max_tokens: Some(5),
            temperature: None,
        };

        match self.send_request(request).await {
            Ok(_) => true,
            Err(e) => {
                warn!("LLM health check failed: {e}");
                false
            }
        }
    }
}
