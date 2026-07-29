use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub listen_addr: String,
    pub listen_port: u16,
    pub master_key_hex: Option<String>,
    pub embedding_provider: EmbeddingProviderConfig,
    pub llm_provider: LlmProviderConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingProviderConfig {
    pub provider_type: String, // "openai", "ollama", "local"
    pub endpoint: String,
    pub api_key: Option<String>,
    pub model: String,
    pub dimensions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub provider_type: String, // "openai", "anthropic", "ollama"
    pub endpoint: String,
    pub api_key: Option<String>,
    pub model: String,
    pub max_tokens: usize,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let data_dir = std::env::var("BRAIN_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("~/.brain"));

        let listen_addr =
            std::env::var("BRAIN_LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0".to_string());

        let listen_port = std::env::var("BRAIN_LISTEN_PORT")
            .unwrap_or_else(|_| "8642".to_string())
            .parse()?;

        let master_key_hex = std::env::var("BRAIN_MASTER_KEY").ok();

        let embedding_provider = EmbeddingProviderConfig {
            provider_type: std::env::var("EMBEDDING_PROVIDER_TYPE")
                .unwrap_or_else(|_| "ollama".to_string()),
            endpoint: std::env::var("EMBEDDING_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            api_key: std::env::var("EMBEDDING_API_KEY").ok(),
            model: std::env::var("EMBEDDING_MODEL").unwrap_or_else(|_| "bge-m3".to_string()),
            dimensions: std::env::var("EMBEDDING_DIMENSIONS")
                .unwrap_or_else(|_| "1024".to_string())
                .parse()?,
        };

        let llm_provider = LlmProviderConfig {
            provider_type: std::env::var("LLM_PROVIDER_TYPE")
                .unwrap_or_else(|_| "openai".to_string()),
            endpoint: std::env::var("LLM_ENDPOINT")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            api_key: std::env::var("LLM_API_KEY").ok(),
            model: std::env::var("LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string()),
            max_tokens: std::env::var("LLM_MAX_TOKENS")
                .unwrap_or_else(|_| "4096".to_string())
                .parse()?,
        };

        Ok(Self {
            data_dir,
            listen_addr,
            listen_port,
            master_key_hex,
            embedding_provider,
            llm_provider,
        })
    }
}
