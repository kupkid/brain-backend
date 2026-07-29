use std::sync::{Arc, Mutex};
use rusqlite::Connection;
use tracing::{info, warn};

use crate::provider::llm::{LlmMessage, LlmProvider};
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
    const MAX_TOOL_HISTORY: usize = 6;

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
            tool_call_id: None,
            tool_calls: None,
        }];

        let pruned = self.prune_history(history);
        for msg in &pruned {
            messages.push(LlmMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
                tool_call_id: None,
                tool_calls: None,
            });
        }

        messages.push(LlmMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
            tool_call_id: None,
            tool_calls: None,
        });

        info!("User message ({} chars): {}", user_message.len(), if user_message.len() > 200 { format!("{}...", &user_message[..200]) } else { user_message.to_string() });

        let mut all_results = Vec::new();
        let mut total_tokens = 0;
        let mut tool_call_count = 0u32;

        loop {
            let result = match self.llm.complete_with_tools(&messages, Some(8192), Some(0.7)).await {
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
            total_tokens += result.tokens_used;

            if !result.tool_calls.is_empty() {
                let tool_calls_json: Vec<serde_json::Value> = result.tool_calls.iter().map(|tc| {
                    serde_json::json!({
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments.to_string(),
                        }
                    })
                }).collect();
                messages.push(LlmMessage {
                    role: "assistant".to_string(),
                    content: if result.content.is_empty() { String::new() } else { result.content.clone() },
                    tool_call_id: None,
                    tool_calls: Some(tool_calls_json),
                });

                for tc in &result.tool_calls {
                    info!("tool call: {}({})", tc.name, tc.arguments);
                    let output = match self.tools.call(&tc.name, &tc.arguments) {
                        Ok(o) => o,
                        Err(e) => ToolOutput::error(&e),
                    };

                    let content_for_llm = match &output.summary {
                        Some(s) => s.clone(),
                        None => match &output.result {
                            serde_json::Value::String(s) => s.clone(),
                            other => {
                                let s = other.to_string();
                                if s.len() > 4000 { s[..4000].to_string() } else { s }
                            }
                        }
                    };

                    messages.push(LlmMessage {
                        role: "tool".to_string(),
                        content: content_for_llm,
                        tool_call_id: Some(tc.id.clone()),
                        tool_calls: None,
                    });

                    all_results.push(ToolCallResult {
                        name: tc.name.clone(),
                        output,
                    });
                    tool_call_count += 1;
                }

                // Sliding window: collapse old tool calls to save context
                if tool_call_count as usize > Self::MAX_TOOL_HISTORY {
                    self.collapse_old_tool_calls(&mut messages);
                }

                // After first call, replace system prompt with short continuation
                if messages[0].role == "system" {
                    messages[0].content = "Continue. Use tools as needed.".to_string();
                }
            } else {
                self.store_memory(user_message, &result.content).await;
                return AgentResponse {
                    content: result.content,
                    tool_results: all_results,
                    tokens_used: total_tokens,
                };
            }
        }
    }

    fn collapse_old_tool_calls(&self, messages: &mut Vec<LlmMessage>) {
        // Find the system message (index 0), then collapse everything between
        // system and the last MAX_TOOL_HISTORY tool-call pairs into a summary.
        // Keep: system, user, last MAX_TOOL_HISTORY*2 messages (assistant+tool pairs), plus any new ones
        let system_len = 1; // system message at index 0
        let user_len = if messages.len() > 1 && messages[1].role == "user" { 1 } else { 0 };
        let keep_from = system_len + user_len; // start of prunable region

        // Count tool result messages from the end
        let mut tool_result_count = 0usize;
        for msg in messages.iter().rev() {
            if msg.role == "tool" || msg.role == "assistant" {
                tool_result_count += 1;
            } else {
                break;
            }
        }

        if tool_result_count <= Self::MAX_TOOL_HISTORY * 2 { return; }

        let collapse_end = messages.len() - Self::MAX_TOOL_HISTORY * 2;
        if collapse_end <= keep_from { return; }

        // Count how many tool calls we're collapsing
        let collapsed_count = messages[keep_from..collapse_end].iter()
            .filter(|m| m.role == "tool")
            .count();

        let summary = LlmMessage {
            role: "system".to_string(),
            content: format!("[{collapsed_count} tool results completed]"),
            tool_call_id: None,
            tool_calls: None,
        };

        messages.drain(keep_from..collapse_end);
        messages.insert(keep_from, summary);
    }

    fn build_system_prompt(&self) -> String {
        let now = chrono::Utc::now();
        let local = now + chrono::Duration::hours(3); // MSK
        let time_str = local.format("%H:%M").to_string();
        let weekday = match local.format("%A").to_string().as_str() {
            "Monday" => "понедельник",
            "Tuesday" => "вторник",
            "Wednesday" => "среда",
            "Thursday" => "четверг",
            "Friday" => "пятница",
            "Saturday" => "суббота",
            "Sunday" => "воскресенье",
            _ => "",
        };
        let month = match local.format("%B").to_string().as_str() {
            "January" => "января", "February" => "февраля", "March" => "марта",
            "April" => "апреля", "May" => "мая", "June" => "июня",
            "July" => "июля", "August" => "августа", "September" => "сентября",
            "October" => "октября", "November" => "ноября", "December" => "декабря",
            _ => "",
        };
        let day = local.format("%d").to_string().trim_start_matches('0').to_string();

        format!(
            "Ты — AI-агент Brain. Сегодня {weekday}, {day} {month} {year} года, {time} MSK.\n\n\
            Ты работаешь в терминале. У тебя есть инструменты для работы с файлами, shell, браузером и задачами.\n\n\
            Принципы:\n\
            - Действуй решительно. Выполняй задачу целиком, не фрагментами.\n\
            - Планируй через todo_create, отмечай прогресс через todo_update.\n\
            - Не спрашивай подтверждений — делай.\n\
            - Не создавай лишних файлов.\n\
            - Отвечай на языке пользователя. Будь краток.",
            weekday = weekday,
            day = day,
            month = month,
            year = now.format("%Y"),
            time = time_str,
        )
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
