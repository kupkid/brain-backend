use std::sync::{Arc, Mutex};
use rusqlite::Connection;
use tracing::{info, warn};

use crate::provider::llm::{LlmProvider, LlmMessage};
use crate::provider::embedding::EmbeddingProvider;
use crate::memory::MemoryRepository;
use crate::memory::repository::NewMemory;
use crate::memory::heuristic;
use super::config::AgentConfig;
use super::tools::ToolBox;
use super::tool_trait::{ToolOutput, ToolImportance};

pub struct AgentLoop {
    llm: Arc<dyn LlmProvider>,
    #[allow(dead_code)]
    embedding: Arc<dyn EmbeddingProvider>,
    conn: Arc<Mutex<Connection>>,
    tools: ToolBox,
    config: AgentConfig,
    run_id: i64,
}

#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
    pub importance: ToolImportance,
}

#[derive(Debug, Clone)]
pub struct AgentResponse {
    pub content: String,
    pub tool_results: Vec<ToolCallResult>,
    pub tokens_used: usize,
}

#[derive(Debug, Clone)]
pub struct ToolCallResult {
    pub name: String,
    pub output: ToolOutput,
}

impl AgentLoop {
    pub fn new(
        llm: Arc<dyn LlmProvider>,
        embedding: Arc<dyn EmbeddingProvider>,
        conn: Arc<Mutex<Connection>>,
        tools: ToolBox,
        config: AgentConfig,
        run_id: i64,
    ) -> Self {
        Self { llm, embedding, conn, tools, config, run_id }
    }

    pub fn tools_ref(&self) -> &ToolBox { &self.tools }

    pub async fn process_message(
        &self,
        user_message: &str,
        history: &[AgentMessage],
    ) -> AgentResponse {
        let mut messages = vec![LlmMessage {
            role: "system".to_string(),
            content: self.build_system_prompt(),
        }];

        let pruned = self.prune_history(history);
        for msg in &pruned {
            messages.push(LlmMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }

        messages.push(LlmMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        let mut all_results = Vec::new();
        let mut total_tokens = 0;

        loop {
            let response = match self.llm.complete(&messages, Some(4096), Some(0.7)).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("LLM error: {e}");
                    return AgentResponse {
                        content: format!("Error: {e}"),
                        tool_results: all_results,
                        tokens_used: total_tokens,
                    };
                }
            };
            total_tokens += response.tokens_used;

            if let Some(tool_calls) = self.parse_tool_calls(&response.content) {
                messages.push(LlmMessage {
                    role: "assistant".to_string(),
                    content: response.content.clone(),
                });

                for (name, args) in &tool_calls {
                    info!("tool call: {}({})", name, args);
                    let output = match self.tools.call(name, args) {
                        Ok(o) => o,
                        Err(e) => ToolOutput::error(&e),
                    };

                    let content_for_llm = match &output.summary {
                        Some(s) => s.clone(),
                        None => {
                            let s = output.result.to_string();
                            if s.len() > 4000 { s[..4000].to_string() } else { s }
                        }
                    };

                    messages.push(LlmMessage {
                        role: "tool".to_string(),
                        content: content_for_llm,
                    });

                    all_results.push(ToolCallResult {
                        name: name.clone(),
                        output,
                    });
                }
            } else {
                self.store_memory(user_message, &response.content).await;
                return AgentResponse {
                    content: response.content,
                    tool_results: all_results,
                    tokens_used: total_tokens,
                };
            }
        }
    }

    fn build_system_prompt(&self) -> String {
        let tool_names = self.tools.names().join(", ");
        format!(
            "You are a helpful AI agent with tools. Use them when needed.\n\
             Tools: {tool_names}.\n\
             To call a tool, write exactly:\n\
             ```tool\n{{\"name\":\"tool_name\",\"arguments\":{{...}}}}\n```\n\
             When done, respond normally without tool blocks."
        )
    }

    fn parse_tool_calls(&self, content: &str) -> Option<Vec<(String, serde_json::Value)>> {
        let mut calls = Vec::new();
        let mut remaining = content;
        while let Some(start) = remaining.find("```tool") {
            let after = &remaining[start + 7..];
            if let Some(end) = after.find("```") {
                let json_str = after[..end].trim();
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str)
                    && let (Some(name), Some(args)) = (value.get("name"), value.get("arguments")) {
                    calls.push((name.as_str().unwrap_or("").to_string(), args.clone()));
                }
                remaining = &after[end + 3..];
            } else { break; }
        }
        if calls.is_empty() { None } else { Some(calls) }
    }

    fn prune_history(&self, history: &[AgentMessage]) -> Vec<AgentMessage> {
        let mut pruned = Vec::new();
        let budget = self.config.max_context_tokens as usize;
        let threshold = (budget as f64 * self.config.context_threshold as f64) as usize;
        let mut used_tokens = 0usize;
        let mut normal_count = 0usize;
        let mut low_count = 0usize;
        let mut skipped = 0usize;

        for msg in history.iter().rev() {
            let tokens = msg.content.len() / 4;
            match msg.importance {
                ToolImportance::High => {
                    used_tokens += tokens;
                    pruned.push(msg.clone());
                }
                ToolImportance::Normal => {
                    if normal_count < self.config.max_normal_observations && used_tokens + tokens < threshold {
                        used_tokens += tokens;
                        normal_count += 1;
                        pruned.push(msg.clone());
                    } else { skipped += 1; }
                }
                ToolImportance::Low => {
                    if low_count < self.config.max_low_observations && used_tokens + tokens < threshold {
                        used_tokens += tokens;
                        low_count += 1;
                        pruned.push(msg.clone());
                    } else { skipped += 1; }
                }
            }
        }
        pruned.reverse();

        if skipped > 0 {
            pruned.insert(0, AgentMessage {
                role: "system".to_string(),
                content: format!("[Earlier: {skipped} observations pruned from context]"),
                importance: ToolImportance::High,
            });
        }
        pruned
    }

    async fn store_memory(&self, user_msg: &str, agent_response: &str) {
        let conn = self.conn.lock().unwrap();
        let mem_repo = MemoryRepository::new(&conn);
        let hash = crate::memory::compute_content_hash(agent_response);
        let content = format!("User: {}\nAgent: {}", user_msg, agent_response);
        if !heuristic::check_content(&content).passed { return; }

        let new_mem = NewMemory {
            collection_id: 1,
            project_id: None,
            run_id: Some(self.run_id),
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
