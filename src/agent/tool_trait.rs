use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolImportance {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub result: serde_json::Value,
    pub summary: Option<String>,
    pub token_estimate: u32,
    pub importance: ToolImportance,
}

impl ToolOutput {
    pub fn new(result: serde_json::Value, importance: ToolImportance) -> Self {
        let token_estimate = estimate_tokens(&result.to_string());
        Self {
            result,
            summary: None,
            token_estimate,
            importance,
        }
    }

    pub fn with_summary(mut self, summary: &str) -> Self {
        self.summary = Some(summary.to_string());
        self
    }

    pub fn text(text: &str, importance: ToolImportance) -> Self {
        let token_estimate = estimate_tokens(text);
        Self {
            result: serde_json::Value::String(text.to_string()),
            summary: None,
            token_estimate,
            importance,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self::text(msg, ToolImportance::Normal)
    }
}

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value;
    fn execute(&self, args: &serde_json::Value) -> Result<ToolOutput, String>;
}

fn estimate_tokens(text: &str) -> u32 {
    (text.len() / 4) as u32
}
