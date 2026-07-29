-- Multi-provider system
-- providers: multiple LLM/embedding providers with encrypted API keys
-- provider_models: discovered models per provider with capabilities

CREATE TABLE IF NOT EXISTS providers (
    id INTEGER PRIMARY KEY,
    uuid BLOB(16) NOT NULL UNIQUE,
    name TEXT NOT NULL,
    type TEXT NOT NULL DEFAULT 'openai',
    base_url TEXT NOT NULL,
    api_key_encrypted BLOB,
    api_key_dek_nonce BLOB(12),
    api_key_ciphertext BLOB,
    api_key_ciphertext_nonce BLOB(12),
    api_key_key_version INTEGER,
    enabled INTEGER NOT NULL DEFAULT 1,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK (type IN ('openai', 'anthropic', 'google', 'cohere', 'custom'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_providers_one_default
    ON providers(is_default)
    WHERE is_default = 1;

CREATE TABLE IF NOT EXISTS provider_models (
    id INTEGER PRIMARY KEY,
    provider_id INTEGER NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    model_id TEXT NOT NULL,
    model_type TEXT NOT NULL DEFAULT 'chat',
    display_name TEXT,
    context_window INTEGER,
    max_output INTEGER,
    supports_tools INTEGER NOT NULL DEFAULT 0,
    supports_vision INTEGER NOT NULL DEFAULT 0,
    supports_reasoning INTEGER NOT NULL DEFAULT 0,
    supports_audio INTEGER NOT NULL DEFAULT 0,
    supports_video INTEGER NOT NULL DEFAULT 0,
    input_modalities TEXT NOT NULL DEFAULT '["text"]',
    output_modalities TEXT NOT NULL DEFAULT '["text"]',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(provider_id, model_id),
    CHECK (model_type IN ('chat', 'embedding', 'image', 'audio', 'video'))
);
