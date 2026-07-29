use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::llm::{LlmError, LlmMessage, LlmProvider, LlmResponse, StreamChunk, StructuredOutput};

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

#[derive(Deserialize)]
struct StreamResponse {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: Option<StreamDelta>,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct StreamDelta {
    content: Option<String>,
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

    async fn complete_stream(
        &self,
        messages: &[LlmMessage],
        max_tokens: Option<usize>,
        temperature: Option<f32>,
        tx: tokio::sync::mpsc::Sender<StreamChunk>,
    ) -> Result<LlmResponse, LlmError> {
        let chat_messages: Vec<ChatMessage> = messages
            .iter()
            .map(|m| ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let url = format!("{}/chat/completions", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "messages": chat_messages,
                "max_tokens": max_tokens,
                "temperature": temperature,
                "stream": true,
            }))
            .send()
            .await
            .map_err(|e| LlmError::Provider(format!("stream request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LlmError::Provider(format!("HTTP {status}: {body}")));
        }

        let mut stream = response.bytes_stream();
        let mut full_content = String::new();
        let tokens_used = 0usize;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| LlmError::Provider(format!("stream read error: {e}")))?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || !line.starts_with("data: ") { continue; }
                let data = &line[6..];
                if data == "[DONE]" { continue; }

                if let Ok(sr) = serde_json::from_str::<StreamResponse>(data)
                    && let Some(choice) = sr.choices.first()
                    && let Some(delta) = &choice.delta
                    && let Some(content) = &delta.content
                {
                    full_content.push_str(content);
                    let _ = tx.send(StreamChunk {
                        delta: content.clone(),
                        finished: false,
                        tokens_used: None,
                    }).await;
                }
            }
        }

        let _ = tx.send(StreamChunk {
            delta: String::new(),
            finished: true,
            tokens_used: Some(tokens_used),
        }).await;

        info!("LLM stream complete: model={}, chars={}", self.model, full_content.len());

        Ok(LlmResponse {
            content: full_content,
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
