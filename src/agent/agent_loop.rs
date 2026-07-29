use std::path::PathBuf;
use std::sync::Arc;
use rusqlite::Connection;
use tracing::{info, warn};

use crate::provider::llm::{LlmProvider, LlmMessage};
use crate::provider::embedding::EmbeddingProvider;
use crate::memory::{MemoryRepository};
use crate::memory::repository::NewMemory;
use crate::memory::heuristic;
use super::tools::{ToolRegistry, ToolCall, ToolResult as AgentToolResult};

pub struct AgentLoop {
    llm: Arc<dyn LlmProvider>,
    #[allow(dead_code)]
    embedding: Arc<dyn EmbeddingProvider>,
    conn: Arc<Connection>,
    tools: ToolRegistry,
    #[allow(dead_code)]
    data_dir: PathBuf,
    system_prompt: String,
}

#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct AgentResponse {
    pub content: String,
    pub tool_calls: Vec<AgentToolResult>,
    pub tokens_used: usize,
}

impl AgentLoop {
    pub fn new(
        llm: Arc<dyn LlmProvider>,
        embedding: Arc<dyn EmbeddingProvider>,
        conn: Arc<Connection>,
        tools: ToolRegistry,
        data_dir: PathBuf,
    ) -> Self {
        let system_prompt = r#"You are a helpful AI agent. You have access to tools for file operations and shell commands. Think step by step. Use tools when needed to accomplish tasks. Always explain what you're doing."#.to_string();

        Self {
            llm,
            embedding,
            conn,
            tools,
            data_dir,
            system_prompt,
        }
    }

    pub fn tools_ref(&self) -> &ToolRegistry {
        &self.tools
    }

    pub async fn process_message(
        &self,
        user_message: &str,
        history: &[AgentMessage],
    ) -> AgentResponse {
        let mut messages = vec![
            LlmMessage {
                role: "system".to_string(),
                content: self.build_system_prompt(),
            },
        ];

        for msg in history {
            messages.push(LlmMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }

        messages.push(LlmMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        let mut all_tool_calls = Vec::new();
        let mut total_tokens = 0;

        // Agent loop: keep calling LLM until no more tool calls
        loop {
            let response = match self.llm.complete(&messages, Some(4096), Some(0.7)).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("LLM error: {e}");
                    return AgentResponse {
                        content: format!("Error: {e}"),
                        tool_calls: all_tool_calls,
                        tokens_used: total_tokens,
                    };
                }
            };

            total_tokens += response.tokens_used;

            // Check for tool calls in the response
            if let Some(calls) = self.parse_tool_calls(&response.content) {
                messages.push(LlmMessage {
                    role: "assistant".to_string(),
                    content: response.content.clone(),
                });

                for call in &calls {
                    info!("tool call: {}({})", call.name, call.arguments);

                    let result = match self.tools.call(&call.name, &call.arguments.to_string()) {
                        Ok(output) => AgentToolResult {
                            name: call.name.clone(),
                            output: output.clone(),
                            success: true,
                        },
                        Err(e) => AgentToolResult {
                            name: call.name.clone(),
                            output: format!("Error: {e}"),
                            success: false,
                        },
                    };

                    messages.push(LlmMessage {
                        role: "tool".to_string(),
                        content: format!("[{} result]: {}", result.name, result.output),
                    });

                    all_tool_calls.push(result);
                }
            } else {
                // No tool calls — final response
                self.store_memory(user_message, &response.content).await;

                return AgentResponse {
                    content: response.content,
                    tool_calls: all_tool_calls,
                    tokens_used: total_tokens,
                };
            }
        }
    }

    fn build_system_prompt(&self) -> String {
        let tool_names = self.tools.names().join(", ");
        format!(
            "{}\n\nAvailable tools: {}.\n\nTo use a tool, respond with a JSON block:\n```tool\n{{\"name\": \"tool_name\", \"arguments\": {{...}}}}\n```\nWhen done, just respond normally without a tool block.",
            self.system_prompt, tool_names
        )
    }

    fn parse_tool_calls(&self, content: &str) -> Option<Vec<ToolCall>> {
        // Look for ```tool ... ``` blocks
        let mut calls = Vec::new();
        let mut remaining = content;

        while let Some(start) = remaining.find("```tool") {
            let after_start = &remaining[start + 7..];
            if let Some(end) = after_start.find("```") {
                let json_str = after_start[..end].trim();
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let (Some(name), Some(args)) = (value.get("name"), value.get("arguments")) {
                        calls.push(ToolCall {
                            name: name.as_str().unwrap_or("").to_string(),
                            arguments: args.clone(),
                        });
                    }
                }
                remaining = &after_start[end + 3..];
            } else {
                break;
            }
        }

        if calls.is_empty() {
            None
        } else {
            Some(calls)
        }
    }

    async fn store_memory(&self, user_msg: &str, agent_response: &str) {
        let mem_repo = MemoryRepository::new(&self.conn);
        let hash = crate::memory::compute_content_hash(agent_response);

        let content = format!("User: {}\nAgent: {}", user_msg, agent_response);
        if !heuristic::check_content(&content).passed {
            return;
        }

        let new_mem = NewMemory {
            collection_id: 1,
            project_id: None,
            run_id: None,
            layer: "episodic".to_string(),
            content,
            content_hash: hash,
            memory_type: "episode".to_string(),
            source: "agent".to_string(),
            importance: 0.5,
            source_ref: None,
            metadata_json: "{}".to_string(),
        };

        if let Err(e) = mem_repo.insert(&new_mem) {
            warn!("failed to store memory: {e}");
        }
    }
}
