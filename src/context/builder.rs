use rusqlite::Connection;

use crate::memory::MemoryRepository;
use crate::project::ProjectRepository;
use crate::run::context::RunContextRepository;

#[derive(Debug, Clone)]
pub struct ContextSlot {
    pub slot: String,
    pub content: String,
    pub token_estimate: usize,
}

#[derive(Debug, Clone)]
pub struct AssembledContext {
    pub run_id: i64,
    pub project_id: Option<i64>,
    pub slots: Vec<ContextSlot>,
    pub total_tokens: usize,
    pub memory_count: usize,
}

pub struct ContextBuilder<'a> {
    conn: &'a Connection,
    data_dir: std::path::PathBuf,
}

impl<'a> ContextBuilder<'a> {
    pub fn new(conn: &'a Connection, data_dir: std::path::PathBuf) -> Self {
        Self { conn, data_dir }
    }

    /// Assemble full context for a run
    pub fn assemble(
        &self,
        run_id: i64,
        project_id: Option<i64>,
        max_memory_tokens: usize,
    ) -> anyhow::Result<AssembledContext> {
        let mut slots = Vec::new();
        let mut total_tokens = 0;
        let mut memory_count = 0;

        // 1. Load existing run context slots (system prompt, tools, etc.)
        let ctx_repo = RunContextRepository::new(self.conn);
        let existing = ctx_repo.slots_map(run_id)?;

        // System prompt
        if let Some(prompt) = existing.get("system_prompt") {
            let tokens = estimate_tokens(prompt);
            slots.push(ContextSlot {
                slot: "system_prompt".to_string(),
                content: prompt.clone(),
                token_estimate: tokens,
            });
            total_tokens += tokens;
        }

        // Tools definition
        if let Some(tools) = existing.get("tools_json") {
            let tokens = estimate_tokens(tools);
            slots.push(ContextSlot {
                slot: "tools_json".to_string(),
                content: tools.clone(),
                token_estimate: tokens,
            });
            total_tokens += tokens;
        }

        // 2. Project context
        if let Some(pid) = project_id {
            let proj_repo = ProjectRepository::new(self.conn, self.data_dir.clone());
            if let Ok(Some(project)) = proj_repo.get(pid) {
                let project_context =
                    format!("Project: {}\nConfig: {}", project.name, project.config_json);
                let tokens = estimate_tokens(&project_context);
                slots.push(ContextSlot {
                    slot: "project".to_string(),
                    content: project_context,
                    token_estimate: tokens,
                });
                total_tokens += tokens;
            }
        }

        // 3. Relevant memories (FTS5 search)
        let mem_repo = MemoryRepository::new(self.conn);

        // Get memories from different layers
        let layers = ["global_profile", "project", "episodic", "working"];
        let mut memories_text = String::new();
        let _budget_per_layer = max_memory_tokens / layers.len();

        for layer in &layers {
            let layer_memories = match *layer {
                "global_profile" => mem_repo.list_global_profile(10)?,
                "project" => {
                    if let Some(pid) = project_id {
                        mem_repo.list_by_project(pid, Some("project"), 10)?
                    } else {
                        Vec::new()
                    }
                }
                "episodic" => {
                    if let Some(pid) = project_id {
                        mem_repo.list_by_project(pid, Some("episodic"), 10)?
                    } else {
                        Vec::new()
                    }
                }
                "working" => {
                    if let Some(pid) = project_id {
                        mem_repo.list_by_project(pid, Some("working"), 10)?
                    } else {
                        Vec::new()
                    }
                }
                _ => Vec::new(),
            };

            for mem in &layer_memories {
                let entry = format!("[{}] {}\n", mem.memory_type, mem.content);
                let entry_tokens = estimate_tokens(&entry);
                if total_tokens + entry_tokens <= max_memory_tokens {
                    memories_text.push_str(&entry);
                    total_tokens += entry_tokens;
                    memory_count += 1;
                }
            }
        }

        if !memories_text.is_empty() {
            slots.push(ContextSlot {
                slot: "memories".to_string(),
                content: memories_text,
                token_estimate: total_tokens
                    - slots.iter().map(|s| s.token_estimate).sum::<usize>(),
            });
        }

        // 4. Conversation history placeholder
        if let Some(history) = existing.get("conversation_history") {
            let tokens = estimate_tokens(history);
            if total_tokens + tokens <= max_memory_tokens {
                slots.push(ContextSlot {
                    slot: "conversation_history".to_string(),
                    content: history.clone(),
                    token_estimate: tokens,
                });
                total_tokens += tokens;
            }
        }

        Ok(AssembledContext {
            run_id,
            project_id,
            slots,
            total_tokens,
            memory_count,
        })
    }

    /// Format assembled context as a single prompt string
    pub fn format_prompt(ctx: &AssembledContext) -> String {
        let mut parts = Vec::new();

        for slot in &ctx.slots {
            parts.push(format!("=== {} ===\n{}", slot.slot, slot.content));
        }

        parts.join("\n\n")
    }
}

/// Rough token estimate (1 token ≈ 4 chars for English, ~2 chars for CJK)
fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("hello"), 1);
        assert_eq!(estimate_tokens("hello world test"), 4);
    }
}
