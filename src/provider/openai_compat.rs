use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::llm::{
    LlmError, LlmMessage, LlmProvider, LlmResponse, LlmToolCall, LlmToolResult, StructuredOutput,
};

pub struct OpenAiCompatLlm {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    tools_json: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    usage: Option<ChatUsage>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ResponseMessage,
}

#[derive(Deserialize, Clone)]
struct ResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<RawToolCall>>,
}

#[derive(Deserialize, Clone)]
struct RawToolCall {
    id: String,
    #[serde(rename = "type")]
    _call_type: String,
    function: RawFunction,
}

#[derive(Deserialize, Clone)]
struct RawFunction {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct ChatUsage {
    total_tokens: Option<usize>,
    prompt_tokens: Option<usize>,
    completion_tokens: Option<usize>,
}

impl OpenAiCompatLlm {
    pub fn new(api_key: String, model: String, base_url: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            base_url,
            tools_json: None,
        }
    }

    pub fn with_tools(mut self, tools: serde_json::Value) -> Self {
        self.tools_json = Some(tools);
        self
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

    async fn complete_with_tools_raw(
        &self,
        messages: &[LlmMessage],
        max_tokens: Option<usize>,
        temperature: Option<f32>,
    ) -> Result<LlmToolResult, LlmError> {
        let chat_messages: Vec<ChatMessage> = messages
            .iter()
            .map(|m| ChatMessage {
                role: m.role.clone(),
                content: Some(m.content.clone()),
                tool_calls: m.tool_calls.clone(),
                tool_call_id: m.tool_call_id.clone(),
            })
            .collect();

        let request = ChatRequest {
            model: self.model.clone(),
            messages: chat_messages,
            max_tokens,
            temperature,
            tools: self.tools_json.clone(),
        };

        let response = self.send_request(request).await?;
        let usage = response.usage.as_ref();
        let total_tokens = usage.and_then(|u| u.total_tokens).unwrap_or(0);
        let prompt_tokens = usage.and_then(|u| u.prompt_tokens).unwrap_or(total_tokens * 7 / 10);
        let completion_tokens = usage.and_then(|u| u.completion_tokens).unwrap_or(total_tokens - prompt_tokens);

        let choice = response
            .choices
            .first()
            .ok_or_else(|| LlmError::Provider("no choices".to_string()))?;
        let content = choice.message.content.clone().unwrap_or_default();

        let tool_calls: Vec<LlmToolCall> = choice
            .message
            .tool_calls
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|tc| {
                let args: serde_json::Value = serde_json::from_str(&tc.function.arguments).ok()?;
                Some(LlmToolCall {
                    id: tc.id,
                    name: tc.function.name,
                    arguments: args,
                })
            })
            .collect();

        info!(
            "LLM complete: model={}, tokens={} (in={prompt_tokens}, out={completion_tokens}), tool_calls={}",
            self.model,
            total_tokens,
            tool_calls.len()
        );

        Ok(LlmToolResult {
            content,
            tool_calls,
            tokens_used: total_tokens,
            tokens_input: prompt_tokens,
            tokens_output: completion_tokens,
        })
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatLlm {
    async fn complete(
        &self,
        messages: &[LlmMessage],
        max_tokens: Option<usize>,
        temperature: Option<f32>,
    ) -> Result<LlmResponse, LlmError> {
        let result = self
            .complete_with_tools_raw(messages, max_tokens, temperature)
            .await?;
        Ok(LlmResponse {
            content: result.content,
            tokens_used: result.tokens_used,
            model: self.model.clone(),
        })
    }

    async fn complete_with_tools(
        &self,
        messages: &[LlmMessage],
        max_tokens: Option<usize>,
        temperature: Option<f32>,
    ) -> Result<LlmToolResult, LlmError> {
        self.complete_with_tools_raw(messages, max_tokens, temperature)
            .await
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
                content: Some("say ok".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            max_tokens: Some(256),
            temperature: None,
            tools: None,
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
