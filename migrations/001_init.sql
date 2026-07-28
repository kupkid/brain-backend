-- ============================================
-- BRAIN BACKEND DDL v2.1
-- SQLite WAL + sqlite-vec + FTS5
-- Applied corrections from architecture review
-- ============================================

PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;

-- ============================================
-- 1. ПРОЕКТЫ
-- ============================================
CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY,
    uuid BLOB(16) NOT NULL UNIQUE,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL UNIQUE,
    config_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_projects_uuid ON projects(uuid);

-- ============================================
-- 2. VAULT — Envelope Encryption
-- ============================================
CREATE TABLE IF NOT EXISTS vault_master_keys (
    id INTEGER PRIMARY KEY,
    algorithm TEXT NOT NULL DEFAULT 'aes-256-gcm',
    salt BLOB(16) NOT NULL,
    params_json TEXT NOT NULL DEFAULT '{"memory_cost":65536,"time_cost":3,"parallelism":4}',
    key_hash BLOB(32) NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    retired_at TEXT,
    CHECK (algorithm IN ('aes-256-gcm'))
);

-- Ровно один активный ключ (retired_at IS NULL)
CREATE UNIQUE INDEX IF NOT EXISTS idx_vault_keys_one_active
    ON vault_master_keys((1))
    WHERE retired_at IS NULL;

CREATE TABLE IF NOT EXISTS credentials_vault (
    id INTEGER PRIMARY KEY,
    uuid BLOB(16) NOT NULL UNIQUE,
    project_id INTEGER REFERENCES projects(id) ON DELETE CASCADE,
    scope TEXT NOT NULL DEFAULT 'global',
    name TEXT NOT NULL,
    encrypted_dek BLOB NOT NULL,
    dek_nonce BLOB(12) NOT NULL,
    ciphertext BLOB NOT NULL,
    ciphertext_nonce BLOB(12) NOT NULL,
    key_version INTEGER NOT NULL REFERENCES vault_master_keys(id),
    tags_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK (
        (scope = 'global' AND project_id IS NULL) OR
        (scope = 'project' AND project_id IS NOT NULL)
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_vault_name_scope
    ON credentials_vault(name, scope, project_id);
CREATE INDEX IF NOT EXISTS idx_vault_project ON credentials_vault(project_id);

-- ============================================
-- 3. EMBEDDING COLLECTIONS
-- ============================================
CREATE TABLE IF NOT EXISTS embedding_collections (
    id INTEGER PRIMARY KEY,
    uuid BLOB(16) NOT NULL UNIQUE,
    model_name TEXT NOT NULL,
    dimensions INTEGER NOT NULL,
    distance_metric TEXT NOT NULL DEFAULT 'cosine',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    active INTEGER NOT NULL DEFAULT 1,
    CHECK (distance_metric IN ('l2', 'cosine', 'l1')),
    CHECK (dimensions IN (384, 768, 1024, 1536, 3072))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_emb_active
    ON embedding_collections(active)
    WHERE active = 1;

-- ============================================
-- 4. RUNS — State Machine
-- ============================================
CREATE TABLE IF NOT EXISTS runs (
    id INTEGER PRIMARY KEY,
    uuid BLOB(16) NOT NULL UNIQUE,
    project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
    agent_name TEXT NOT NULL,
    goal TEXT NOT NULL,
    context_json TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'pending',
    summary TEXT,
    tokens_used INTEGER NOT NULL DEFAULT 0,
    cost_cents INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    completed_at TEXT,
    CHECK (status IN ('pending', 'running', 'paused', 'completed', 'failed', 'cancelled'))
);

CREATE INDEX IF NOT EXISTS idx_runs_project ON runs(project_id);
CREATE INDEX IF NOT EXISTS idx_runs_status ON runs(status);
CREATE INDEX IF NOT EXISTS idx_runs_created ON runs(created_at);
CREATE INDEX IF NOT EXISTS idx_runs_agent ON runs(agent_name);

-- ============================================
-- 4b. RUN TRANSITIONS — State Machine Audit Log
-- ============================================
CREATE TABLE IF NOT EXISTS run_transitions (
    id INTEGER PRIMARY KEY,
    run_id INTEGER NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    from_status TEXT NOT NULL,
    to_status TEXT NOT NULL,
    reason TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_trans_run ON run_transitions(run_id);

-- ============================================
-- 5. RUN EVENTS — с atomical seq allocation
-- ============================================
CREATE TABLE IF NOT EXISTS run_seq_counters (
    run_id INTEGER PRIMARY KEY REFERENCES runs(id) ON DELETE CASCADE,
    next_seq INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS run_events (
    id INTEGER PRIMARY KEY,
    run_id INTEGER NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    event_uuid BLOB(16) NOT NULL,
    seq INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK (event_type IN (
        'message', 'tool_call', 'tool_result',
        'state_change', 'error', 'system'
    )),
    UNIQUE(run_id, seq),
    UNIQUE(run_id, event_uuid)
);

CREATE INDEX IF NOT EXISTS idx_events_run ON run_events(run_id);
CREATE INDEX IF NOT EXISTS idx_events_type ON run_events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_created ON run_events(created_at);

-- ============================================
-- 6. TOOL INVOCATIONS
-- ============================================
CREATE TABLE IF NOT EXISTS tool_invocations (
    id INTEGER PRIMARY KEY,
    uuid BLOB(16) NOT NULL UNIQUE,
    run_id INTEGER NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    event_id INTEGER REFERENCES run_events(id) ON DELETE SET NULL,
    tool_name TEXT NOT NULL,
    arguments_json TEXT NOT NULL DEFAULT '{}',
    result_summary TEXT,
    result_full TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    started_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    completed_at TEXT,
    duration_ms INTEGER,
    tokens_used INTEGER NOT NULL DEFAULT 0,
    cost_cents INTEGER NOT NULL DEFAULT 0,
    retry_count INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    CHECK (status IN ('pending', 'running', 'success', 'error', 'timeout'))
);

CREATE INDEX IF NOT EXISTS idx_tool_run ON tool_invocations(run_id);
CREATE INDEX IF NOT EXISTS idx_tool_name ON tool_invocations(tool_name);
CREATE INDEX IF NOT EXISTS idx_tool_status ON tool_invocations(status);
CREATE INDEX IF NOT EXISTS idx_tool_started ON tool_invocations(started_at);

-- ============================================
-- 7. RUN CONTEXT — Working Memory (Transient)
-- ============================================
CREATE TABLE IF NOT EXISTS run_contexts (
    id INTEGER PRIMARY KEY,
    run_id INTEGER NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    slot TEXT NOT NULL,
    content TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(run_id, slot)
);

CREATE INDEX IF NOT EXISTS idx_ctx_run ON run_contexts(run_id);

-- ============================================
-- 8. MEMORIES — 4-Layer Architecture
-- ============================================
CREATE TABLE IF NOT EXISTS memories (
    id INTEGER PRIMARY KEY,
    uuid BLOB(16) NOT NULL UNIQUE,
    collection_id INTEGER NOT NULL REFERENCES embedding_collections(id),
    project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
    run_id INTEGER REFERENCES runs(id) ON DELETE SET NULL,
    content TEXT NOT NULL,
    content_hash BLOB(32) NOT NULL,
    memory_type TEXT NOT NULL,
    layer TEXT NOT NULL,
    importance REAL NOT NULL DEFAULT 0.5,
    access_count INTEGER NOT NULL DEFAULT 0,
    last_accessed_at TEXT,
    superseded_by INTEGER REFERENCES memories(id),
    lifecycle_status TEXT NOT NULL DEFAULT 'active',
    source TEXT NOT NULL DEFAULT 'agent',
    source_ref TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK (memory_type IN ('fact', 'procedure', 'episode', 'relationship')),
    CHECK (layer IN ('global_profile', 'project', 'episodic', 'working')),
    CHECK (lifecycle_status IN ('active', 'archived', 'superseded', 'deleted')),
    CHECK (source IN ('agent', 'user', 'system', 'extraction')),
    CHECK (importance >= 0.0 AND importance <= 1.0)
);

CREATE INDEX IF NOT EXISTS idx_mem_uuid ON memories(uuid);
CREATE INDEX IF NOT EXISTS idx_mem_project ON memories(project_id);
CREATE INDEX IF NOT EXISTS idx_mem_collection ON memories(collection_id);
CREATE INDEX IF NOT EXISTS idx_mem_type ON memories(memory_type);
CREATE INDEX IF NOT EXISTS idx_mem_layer ON memories(layer);
CREATE INDEX IF NOT EXISTS idx_mem_lifecycle ON memories(lifecycle_status);
CREATE INDEX IF NOT EXISTS idx_mem_content_hash ON memories(content_hash);
CREATE INDEX IF NOT EXISTS idx_mem_created ON memories(created_at);
CREATE INDEX IF NOT EXISTS idx_mem_superseded ON memories(superseded_by);

-- Dedup: один active memory с одним content_hash в рамках project+collection
CREATE UNIQUE INDEX IF NOT EXISTS idx_memories_dedup_active
    ON memories(project_id, collection_id, content_hash)
    WHERE lifecycle_status = 'active';

-- ============================================
-- 9. MEMORY FTS5 — Hybrid Search
-- ============================================
CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
    content,
    memory_type,
    layer,
    source,
    content=memories,
    content_rowid=id,
    tokenize='porter unicode61 remove_diacritics 2'
);

-- Триггеры FTS (только content/малоизменяемые колонки)
CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, content, memory_type, layer, source)
    VALUES (new.id, new.content, new.memory_type, new.layer, new.source);
END;

CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, content, memory_type, layer, source)
    VALUES ('delete', old.id, old.content, old.memory_type, old.layer, old.source);
END;

CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories
WHEN old.content IS NOT new.content
   OR old.memory_type IS NOT new.memory_type
   OR old.layer IS NOT new.layer
   OR old.source IS NOT new.source
BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, content, memory_type, layer, source)
    VALUES ('delete', old.id, old.content, old.memory_type, old.layer, old.source);
    INSERT INTO memories_fts(rowid, content, memory_type, layer, source)
    VALUES (new.id, new.content, new.memory_type, new.layer, new.source);
END;

-- ============================================
-- 10. MEMORY VECTORS — vec0 (alpha: memories.id как vector_id)
-- ============================================
-- vec0 таблицы создаются динамически при старте:
--   vec_mem_{dimensions} USING vec0(vector_id INTEGER PRIMARY KEY, embedding float[{dimensions}])
--
-- Dimensions whitelist: 384, 768, 1024, 1536, 3072
-- memories.id используется как integer vector_id напрямую.
-- Collection_id хранится в memories.collection_id для определения модели.

-- ============================================
-- 11. UPDATED_AT GUARDED TRIGGERS
-- ============================================
CREATE TRIGGER IF NOT EXISTS trg_projects_updated
    AFTER UPDATE ON projects
    WHEN new.updated_at = old.updated_at
BEGIN
    UPDATE projects SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = new.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_runs_updated
    AFTER UPDATE ON runs
    WHEN new.updated_at = old.updated_at
BEGIN
    UPDATE runs SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = new.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_memories_updated
    AFTER UPDATE ON memories
    WHEN new.updated_at = old.updated_at
BEGIN
    UPDATE memories SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = new.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_vault_updated
    AFTER UPDATE ON credentials_vault
    WHEN new.updated_at = old.updated_at
BEGIN
    UPDATE credentials_vault SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = new.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_ctx_updated
    AFTER UPDATE ON run_contexts
    WHEN new.updated_at = old.updated_at
BEGIN
    UPDATE run_contexts SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = new.id;
END;
