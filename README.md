# Brain Backend

Personal Agent OS backend — a lightweight, self-contained AI agent runtime.

## Architecture

- **Stack**: Rust, Tokio, Axum, SQLite (WAL mode)
- **Storage**: rusqlite + sqlite-vec for vector search, FTS5 for full-text search
- **Security**: AES-256-GCM envelope encryption, Argon2id key derivation
- **Runtime**: Single binary, <5MB idle RAM

## Features

- **Project Management**: Create/manage projects with workspace isolation
- **Vault**: AES-256-GCM encrypted credential storage with key rotation
- **Run Engine**: State machine for agent runs with event sourcing
- **Memory Engine**: 4-layer memory system (global_profile, project, episodic, working)
- **Vector Search**: sqlite-vec for KNN similarity search
- **Full-Text Search**: FTS5 with Porter stemming
- **Hybrid Retrieval**: Reciprocal Rank Fusion (RRF) merging FTS + vector results
- **Context Builder**: Assemble prompts from memories, project config, conversation history
- **Workspace**: File system backend with path traversal protection

## Quick Start

```bash
# Build
cargo build --release

# Run (first time initializes vault)
BRAIN_VAULT_PASSPHRASE="your-secret" ./target/release/brain-backend

# API available at http://localhost:8642
```

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `BRAIN_VAULT_PASSPHRASE` | Yes | - | Master passphrase for vault encryption |
| `BRAIN_DATA_DIR` | No | `~/.brain` | Data directory for SQLite DB |
| `BRAIN_LISTEN_ADDR` | No | `0.0.0.0` | HTTP listen address |
| `BRAIN_LISTEN_PORT` | No | `8642` | HTTP listen port |
| `BRAIN_LOG_LEVEL` | No | `info` | Log level (trace/debug/info/warn/error) |

## API Endpoints

### Health
- `GET /v1/health` — Health check

### Projects
- `POST /v1/projects` — Create project
- `GET /v1/projects` — List projects
- `GET /v1/projects/:id` — Get project
- `DELETE /v1/projects/:id` — Delete project
- `GET /v1/projects/:id/workspace` — List workspace files
- `GET /v1/projects/:id/workspace/*path` — Read workspace file
- `PUT /v1/projects/:id/workspace/*path` — Write workspace file

### Runs
- `POST /v1/runs` — Create run
- `GET /v1/runs` — List runs
- `GET /v1/runs/:id` — Get run
- `POST /v1/runs/:id/transition` — Transition run status
- `GET /v1/runs/:id/events` — List run events
- `POST /v1/runs/:id/events` — Append run event
- `GET /v1/runs/:id/tools` — List tool invocations
- `GET /v1/runs/:id/tools/stats` — Tool statistics
- `GET /v1/runs/:id/context` — List context slots
- `PUT /v1/runs/:id/context` — Upsert context slot
- `GET /v1/runs/:id/context/:slot` — Get context slot
- `DELETE /v1/runs/:id/context/:slot` — Delete context slot

### Memories
- `POST /v1/memories` — Create memory (with heuristic validation)
- `GET /v1/memories` — List memories (by project/layer/global)
- `POST /v1/memories/search` — Hybrid search (FTS5 + vector)

### Vault
- `POST /v1/credentials` — Store credential (encrypted)
- `GET /v1/credentials` — List credentials (metadata only)
- `GET /v1/credentials/:name` — Get credential metadata
- `DELETE /v1/credentials/:name` — Delete credential

## Test

```bash
# Unit tests
cargo test

# With logging
RUST_LOG=debug cargo test

# Release build
cargo build --release
```

## License

MIT
