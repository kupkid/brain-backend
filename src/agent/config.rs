use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_context_tokens: u32,
    pub context_threshold: f32,
    pub max_normal_observations: usize,
    pub max_low_observations: usize,
    pub tool_timeout_seconds: u64,
    pub workspace_dir: PathBuf,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 8000,
            context_threshold: 0.8,
            max_normal_observations: 20,
            max_low_observations: 5,
            tool_timeout_seconds: 30,
            workspace_dir: PathBuf::from("."),
        }
    }
}

impl AgentConfig {
    pub fn from_env() -> Self {
        let workspace_dir = std::env::var("BRAIN_WORKSPACE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| Self::default().workspace_dir);

        Self {
            workspace_dir,
            ..Self::default()
        }
    }
}
