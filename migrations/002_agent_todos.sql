-- ============================================
-- 12. AGENT TODOS — Task Tracking for Agent Runs
-- ============================================
CREATE TABLE IF NOT EXISTS agent_todos (
    id INTEGER PRIMARY KEY,
    run_id INTEGER NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    task_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK (status IN ('pending', 'in_progress', 'done', 'failed')),
    UNIQUE(run_id, task_id)
);

CREATE INDEX IF NOT EXISTS idx_todos_run ON agent_todos(run_id);
CREATE INDEX IF NOT EXISTS idx_todos_status ON agent_todos(status);

CREATE TRIGGER IF NOT EXISTS trg_todos_updated
    AFTER UPDATE ON agent_todos
    WHEN new.updated_at = old.updated_at
BEGIN
    UPDATE agent_todos SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = new.id;
END;
