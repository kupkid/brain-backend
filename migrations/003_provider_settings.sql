-- Provider settings — stores LLM/embedding provider config
-- API key encrypted via vault (not stored as plaintext)
CREATE TABLE IF NOT EXISTS provider_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    base_url TEXT NOT NULL,
    api_key_encrypted BLOB,
    api_key_dek_nonce BLOB(12),
    api_key_ciphertext BLOB,
    api_key_ciphertext_nonce BLOB(12),
    api_key_key_version INTEGER,
    llm_model TEXT NOT NULL,
    llm_max_tokens INTEGER NOT NULL DEFAULT 8192,
    embedding_model TEXT NOT NULL,
    embedding_dimensions INTEGER NOT NULL DEFAULT 1024,
    embedding_endpoint TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
