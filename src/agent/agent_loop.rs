use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{info, warn};

use super::config::AgentConfig;
use super::tool_trait::{ToolImportance, ToolOutput};
use super::tools::ToolBox;
use crate::memory::MemoryRepository;
use crate::memory::heuristic;
use crate::memory::repository::NewMemory;
use crate::provider::embedding::EmbeddingProvider;
use crate::provider::llm::{LlmMessage, LlmProvider};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsAgentEvent {
    Thought {
        text: String,
        ts: i64,
    },
    Text {
        text: String,
        ts: i64,
    },
    ToolCall {
        tool: String,
        args: serde_json::Value,
        call_id: String,
        ts: i64,
    },
    ToolResult {
        call_id: String,
        success: bool,
        summary: String,
        ts: i64,
    },
    TodoUpdate {
        todos: Vec<TodoItem>,
        ts: i64,
    },
    FileRead {
        path: String,
        text: String,
        ts: i64,
    },
    Done {
        summary: String,
        total_tokens: usize,
        total_calls: u32,
        tokens_input: usize,
        tokens_output: usize,
        elapsed_ms: u64,
        tokens_per_sec: f64,
        ts: i64,
    },
    Error {
        message: String,
        ts: i64,
    },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TodoItem {
    pub id: String,
    pub text: String,
    pub status: String,
}

pub struct AgentLoop {
    llm: Arc<dyn LlmProvider>,
    #[allow(dead_code)]
    embedding: Arc<dyn EmbeddingProvider>,
    conn: Arc<Mutex<Connection>>,
    tools: ToolBox,
    config: AgentConfig,
    run_id: i64,
    event_tx: Option<mpsc::Sender<WsAgentEvent>>,
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
        Self {
            llm,
            embedding,
            conn,
            tools,
            config,
            run_id,
            event_tx: None,
        }
    }

    pub fn with_event_sender(mut self, tx: mpsc::Sender<WsAgentEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    pub fn tools_ref(&self) -> &ToolBox {
        &self.tools
    }

    fn emit(&self, event: WsAgentEvent) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.try_send(event);
        }
    }

    fn ts() -> i64 {
        chrono::Utc::now().timestamp()
    }

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

        info!(
            "User message ({} chars): {}",
            user_message.len(),
            if user_message.len() > 200 {
                format!("{}...", &user_message[..200])
            } else {
                user_message.to_string()
            }
        );

        let mut all_results = Vec::new();
        let mut total_tokens = 0usize;
        let mut tool_call_count = 0u32;
        let mut tokens_input_total = 0usize;
        let mut tokens_output_total = 0usize;
        let start_time = std::time::Instant::now();

        loop {
            let llm_start = std::time::Instant::now();
            let result = match self
                .llm
                .complete_with_tools(&messages, Some(8192), Some(0.7))
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!("LLM error: {e}");
                    self.emit(WsAgentEvent::Error {
                        message: e.to_string(),
                        ts: Self::ts(),
                    });
                    return AgentResponse {
                        content: format!("Error: {e}"),
                        tool_results: all_results,
                        tokens_used: total_tokens,
                    };
                }
            };
            let _llm_elapsed = llm_start.elapsed();
            total_tokens += result.tokens_used;
            tokens_input_total += result.tokens_input;
            tokens_output_total += result.tokens_output;

            if !result.tool_calls.is_empty() {
                // Emit thought if LLM produced text content
                if !result.content.is_empty() {
                    self.emit(WsAgentEvent::Thought {
                        text: result.content.clone(),
                        ts: Self::ts(),
                    });
                }

                let tool_calls_json: Vec<serde_json::Value> = result
                    .tool_calls
                    .iter()
                    .map(|tc| {
                        serde_json::json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.name,
                                "arguments": tc.arguments.to_string(),
                            }
                        })
                    })
                    .collect();
                messages.push(LlmMessage {
                    role: "assistant".to_string(),
                    content: if result.content.is_empty() {
                        String::new()
                    } else {
                        result.content.clone()
                    },
                    tool_call_id: None,
                    tool_calls: Some(tool_calls_json),
                });

                for tc in &result.tool_calls {
                    let call_id = format!("t{tool_call_count}");
                    info!("tool call: {}({})", tc.name, tc.arguments);

                    // Emit tool_call
                    let args_val: serde_json::Value = tc.arguments.clone();
                    self.emit(WsAgentEvent::ToolCall {
                        tool: tc.name.clone(),
                        args: args_val.clone(),
                        call_id: call_id.clone(),
                        ts: Self::ts(),
                    });

                    // Execute tool
                    let output = match self.tools.call(&tc.name, &tc.arguments) {
                        Ok(o) => o,
                        Err(e) => ToolOutput::error(&e),
                    };

                    // Emit tool_result
                    let summary = output
                        .summary
                        .clone()
                        .unwrap_or_else(|| match &output.result {
                            serde_json::Value::String(s) => {
                                if s.len() > 200 {
                                    format!("{}...", &s[..200])
                                } else {
                                    s.clone()
                                }
                            }
                            other => {
                                let s = other.to_string();
                                if s.len() > 200 {
                                    format!("{}...", &s[..200])
                                } else {
                                    s
                                }
                            }
                        });
                    self.emit(WsAgentEvent::ToolResult {
                        call_id: call_id.clone(),
                        success: !output.result.is_null(),
                        summary,
                        ts: Self::ts(),
                    });

                    // Emit file_read if this was a read_file call
                    if tc.name == "read_file" {
                        let text = match &output.result {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        let path = args_val
                            .get("path")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        self.emit(WsAgentEvent::FileRead {
                            path,
                            text,
                            ts: Self::ts(),
                        });
                    }

                    // Emit todo_update if this was a todo tool
                    if tc.name.starts_with("todo_") {
                        self.emit_todo_state();
                    }

                    let content_for_llm = match &output.summary {
                        Some(s) => s.clone(),
                        None => match &output.result {
                            serde_json::Value::String(s) => s.clone(),
                            other => {
                                let s = other.to_string();
                                if s.len() > 4000 {
                                    s[..4000].to_string()
                                } else {
                                    s
                                }
                            }
                        },
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
                // Final response — no more tool calls
                self.store_memory(user_message, &result.content).await;

                // Emit TextEvent for streaming display
                self.emit(WsAgentEvent::Text {
                    text: result.content.clone(),
                    ts: Self::ts(),
                });

                let elapsed_ms = start_time.elapsed().as_millis() as u64;
                let tps = if elapsed_ms > 0 {
                    (total_tokens as f64 * 1000.0) / elapsed_ms as f64
                } else {
                    0.0
                };
                self.emit(WsAgentEvent::Done {
                    summary: result.content.clone(),
                    total_tokens,
                    total_calls: tool_call_count,
                    tokens_input: tokens_input_total,
                    tokens_output: tokens_output_total,
                    elapsed_ms,
                    tokens_per_sec: tps,
                    ts: Self::ts(),
                });
                return AgentResponse {
                    content: result.content,
                    tool_results: all_results,
                    tokens_used: total_tokens,
                };
            }
        }
    }

    fn emit_todo_state(&self) {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        use rusqlite::params;
        let mut stmt = match conn.prepare(
            "SELECT task_id, title, status FROM agent_todos WHERE run_id = ?1 ORDER BY created_at",
        ) {
            Ok(s) => s,
            Err(_) => return,
        };
        let todos: Vec<TodoItem> = stmt
            .query_map(params![self.run_id], |r| {
                Ok(TodoItem {
                    id: r.get(0)?,
                    text: r.get(1)?,
                    status: r.get(2)?,
                })
            })
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|r| r.ok())
            .collect();
        self.emit(WsAgentEvent::TodoUpdate {
            todos,
            ts: Self::ts(),
        });
    }

    fn collapse_old_tool_calls(&self, messages: &mut Vec<LlmMessage>) {
        let system_len = 1;
        let user_len = if messages.len() > 1 && messages[1].role == "user" {
            1
        } else {
            0
        };
        let keep_from = system_len + user_len;

        let mut tool_result_count = 0usize;
        for msg in messages.iter().rev() {
            if msg.role == "tool" || msg.role == "assistant" {
                tool_result_count += 1;
            } else {
                break;
            }
        }

        if tool_result_count <= Self::MAX_TOOL_HISTORY * 2 {
            return;
        }

        let collapse_end = messages.len() - Self::MAX_TOOL_HISTORY * 2;
        if collapse_end <= keep_from {
            return;
        }

        let collapsed_count = messages[keep_from..collapse_end]
            .iter()
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
            "January" => "января",
            "February" => "февраля",
            "March" => "марта",
            "April" => "апреля",
            "May" => "мая",
            "June" => "июня",
            "July" => "июля",
            "August" => "августа",
            "September" => "сентября",
            "October" => "октября",
            "November" => "ноября",
            "December" => "декабря",
            _ => "",
        };
        let day = local
            .format("%d")
            .to_string()
            .trim_start_matches('0')
            .to_string();

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
                    if normal_count < self.config.max_normal_observations
                        && used_tokens + tokens < threshold
                    {
                        used_tokens += tokens;
                        normal_count += 1;
                        pruned.push(msg.clone());
                    } else {
                        skipped += 1;
                    }
                }
                ToolImportance::Low => {
                    if low_count < self.config.max_low_observations
                        && used_tokens + tokens < threshold
                    {
                        used_tokens += tokens;
                        low_count += 1;
                        pruned.push(msg.clone());
                    } else {
                        skipped += 1;
                    }
                }
            }
        }
        pruned.reverse();

        if skipped > 0 {
            pruned.insert(
                0,
                AgentMessage {
                    role: "system".to_string(),
                    content: format!("[Earlier: {skipped} observations pruned from context]"),
                    importance: ToolImportance::High,
                },
            );
        }
        pruned
    }

    async fn store_memory(&self, user_msg: &str, agent_response: &str) {
        let conn = self.conn.lock().unwrap();
        let mem_repo = MemoryRepository::new(&conn);
        let hash = crate::memory::compute_content_hash(agent_response);
        let content = format!("User: {}\nAgent: {}", user_msg, agent_response);
        if !heuristic::check_content(&content).passed {
            return;
        }

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
