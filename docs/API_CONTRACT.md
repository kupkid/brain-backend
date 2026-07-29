# Brain Backend — API Contract

## Base URL
```
http://localhost:8642
```

## Authentication
All endpoints require `Authorization: Bearer <token>` header.
Token is obtained during client registration (not yet implemented).

## Common Response Codes

| Code | Meaning |
|------|---------|
| 200 | Success |
| 201 | Created |
| 204 | No Content (delete success) |
| 400 | Bad Request (validation error) |
| 404 | Not Found |
| 409 | Conflict (idempotency violation) |
| 500 | Internal Server Error |

## Common Headers
```
Content-Type: application/json
X-Request-ID: <uuid> (optional, for tracing)
```

---

## Health

### GET /v1/health
Health check endpoint.

**Response:**
```json
{
  "status": "ok",
  "version": "0.1.0",
  "vault_initialized": true,
  "uptime_seconds": 3600
}
```

---

## Projects

### POST /v1/projects
Create a new project with workspace.

**Request:**
```json
{
  "name": "my-project",
  "config_json": "{\"model\":\"claude-3\"}"
}
```

**Response (201):**
```json
{
  "id": 1,
  "uuid": "0190e0c1-2b3a-7xxx-xxxx-xxxxxxxxxxxx",
  "name": "my-project",
  "root_path": "~/.brain/projects/0190e0c1.../workspace",
  "config_json": "{\"model\":\"claude-3\"}",
  "created_at": "2026-07-28T22:00:00.000Z",
  "updated_at": "2026-07-28T22:00:00.000Z"
}
```

### GET /v1/projects
List all projects.

**Query Parameters:**
- `limit` (optional, default 50): Max results

**Response:**
```json
[
  {
    "id": 1,
    "uuid": "...",
    "name": "my-project",
    "config_json": "...",
    "created_at": "...",
    "updated_at": "..."
  }
]
```

### GET /v1/projects/:id
Get project by ID.

**Response:** Single project object (same as above).

### DELETE /v1/projects/:id
Delete project and its workspace.

**Response:** 204 No Content

---

## Runs

### POST /v1/runs
Create a new run (agent execution cycle).

**Request:**
```json
{
  "project_id": 1,
  "agent_name": "my-agent",
  "goal": "Research and summarize topic X",
  "context_json": "{}"
}
```

**Response (201):**
```json
{
  "id": 1,
  "uuid": "...",
  "project_id": 1,
  "agent_name": "my-agent",
  "goal": "Research and summarize topic X",
  "status": "pending",
  "created_at": "..."
}
```

### GET /v1/runs
List runs.

**Query Parameters:**
- `project_id` (optional): Filter by project
- `status` (optional): Filter by status (pending/running/completed/failed/cancelled)
- `limit` (optional, default 50)

**Response:** Array of run objects.

### GET /v1/runs/:id
Get run by ID.

**Response:** Single run object with full details:
```json
{
  "id": 1,
  "uuid": "...",
  "project_id": 1,
  "agent_name": "my-agent",
  "goal": "...",
  "context_json": "{}",
  "status": "running",
  "summary": null,
  "tokens_used": 1500,
  "cost_cents": 3,
  "created_at": "...",
  "updated_at": "...",
  "completed_at": null
}
```

### POST /v1/runs/:id/transition
Transition run status.

**Request:**
```json
{
  "status": "running",
  "reason": "Agent started processing"
}
```

**Valid transitions:**
- pending → running, cancelled
- running → paused, completed, failed, cancelled
- paused → running, cancelled
- (any) → cancelled (with reason)

---

## Run Events

### GET /v1/runs/:id/events
List events for a run (ordered by seq).

**Query Parameters:**
- `after_seq` (optional): Only return events with seq > N (for sync)
- `limit` (optional, default 100)

**Response:**
```json
[
  {
    "id": 1,
    "run_id": 1,
    "event_uuid": "...",
    "seq": 1,
    "event_type": "message",
    "payload": "{\"text\":\"hello\"}",
    "created_at": "..."
  },
  {
    "id": 2,
    "run_id": 1,
    "event_uuid": "...",
    "seq": 2,
    "event_type": "tool_call",
    "payload": "{\"tool\":\"search\",\"args\":{\"query\":\"hello\"}}",
    "created_at": "..."
  }
]
```

**Event types:**
- `message`: User/agent message
- `tool_call`: Agent invoked a tool
- `tool_result`: Tool returned result
- `state_change`: Run status changed
- `error`: Error occurred
- `system`: System event (heartbeat, etc.)

### POST /v1/runs/:id/events
Append event to a run.

**Request:**
```json
{
  "event_type": "message",
  "payload": "{\"text\":\"hello\"}",
  "idempotency_key": "0190e0c1-..."
}
```

**Response (201):**
```json
{
  "id": 3,
  "seq": 3,
  "event_uuid": "...",
  "created_at": "..."
}
```

**Idempotency:** If `idempotency_key` matches an existing event, returns the existing event (200 instead of 201).

---

## Tool Invocations

### GET /v1/runs/:id/tools
List tool invocations for a run.

**Response:**
```json
[
  {
    "id": 1,
    "tool_name": "search",
    "arguments_json": "{\"query\":\"hello\"}",
    "status": "success",
    "result_summary": "Found 5 results",
    "duration_ms": 234,
    "tokens_used": 0,
    "cost_cents": 0,
    "started_at": "...",
    "completed_at": "..."
  }
]
```

### GET /v1/runs/:id/tools/stats
Get aggregated tool statistics.

**Response:**
```json
{
  "total": 5,
  "success": 4,
  "errors": 1,
  "total_duration_ms": 1234,
  "total_tokens": 0,
  "total_cost": 0
}
```

---

## Run Context

### GET /v1/runs/:id/context
List context slots for a run.

**Response:**
```json
[
  {
    "slot": "system_prompt",
    "content": "You are a helpful assistant...",
    "updated_at": "..."
  },
  {
    "slot": "tools_json",
    "content": "[{\"name\":\"search\"}]",
    "updated_at": "..."
  }
]
```

### PUT /v1/runs/:id/context
Create or update a context slot.

**Request:**
```json
{
  "slot": "system_prompt",
  "content": "You are a helpful assistant..."
}
```

### GET /v1/runs/:id/context/:slot
Get a specific context slot.

### DELETE /v1/runs/:id/context/:slot
Delete a context slot.

---

## Memories

### POST /v1/memories
Create a memory.

**Request:**
```json
{
  "project_id": 1,
  "collection_id": 1,
  "layer": "episodic",
  "content": "User prefers dark mode in all applications",
  "memory_type": "fact",
  "source": "user",
  "importance": 0.8,
  "embedding": [0.1, 0.2, ...]
}
```

**Layers:**
- `global_profile`: User-level (project_id=NULL)
- `project`: Project-specific
- `episodic`: Event-based
- `working`: Short-term, volatile

**Memory types:**
- `fact`: Factual information
- `procedure`: How to do something
- `episode`: What happened
- `relationship`: Connection between entities

**Heuristic validation:**
- Content must be ≥10 characters
- Must have ≥3 words
- Rejects junk patterns (lorem ipsum, repeated chars, etc.)

### GET /v1/memories
List memories.

**Query Parameters:**
- `project_id` (optional): Filter by project
- `layer` (optional): Filter by layer
- `limit` (optional, default 50)

### POST /v1/memories/search
Hybrid search (FTS5 + vector KNN + RRF fusion).

**Request:**
```json
{
  "query": "dark mode preferences",
  "project_id": 1,
  "collection_id": 1,
  "limit": 10,
  "embedding": [0.1, 0.2, ...]
}
```

**Response:**
```json
{
  "results": [
    {
      "id": 1,
      "content": "User prefers dark mode",
      "score": 0.95,
      "memory_type": "fact",
      "layer": "episodic",
      "importance": 0.8,
      "access_count": 5,
      "created_at": "..."
    }
  ]
}
```

---

## Vault (Credentials)

### POST /v1/credentials
Store an encrypted credential.

**Request:**
```json
{
  "name": "openai-api-key",
  "scope": "global",
  "value": "sk-xxxxxxxxxxxx",
  "tags": ["api-key", "openai"]
}
```

**Response (201):**
```json
{
  "id": 1,
  "name": "openai-api-key",
  "key_version": 1
}
```

### GET /v1/credentials
List credentials (metadata only, never decrypted).

**Response:**
```json
[
  {
    "id": 1,
    "name": "openai-api-key",
    "key_version": 1,
    "tags": ["api-key", "openai"],
    "created_at": "..."
  }
]
```

### GET /v1/credentials/:name
Get credential metadata.

### DELETE /v1/credentials/:name
Delete a credential.

---

## Workspace

### GET /v1/projects/:id/workspace
List files in project workspace.

**Response:**
```json
[
  {
    "path": "README.md",
    "is_dir": false,
    "size": 1024,
    "modified": "2026-07-28T22:00:00Z"
  },
  {
    "path": "src",
    "is_dir": true,
    "size": 0,
    "modified": "..."
  }
]
```

### GET /v1/projects/:id/workspace/*path
Read a file from workspace.

**Response:**
```json
{
  "path": "README.md",
  "content": "# My Project\n...",
  "encoding": "utf-8"
}
```

Binary files return base64:
```json
{
  "path": "image.png",
  "content": "iVBORw0KGgo...",
  "encoding": "base64"
}
```

### PUT /v1/projects/:id/workspace/*path
Write a file to workspace.

**Request:**
```json
{
  "content": "# My Project\nHello world"
}
```

**Security:** Path traversal (`../`), absolute paths, and symlink escape are rejected.
