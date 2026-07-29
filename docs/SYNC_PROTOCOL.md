# Brain Backend — Sync Protocol

## Overview

The sync protocol ensures that the client and server stay consistent even when connections drop. The server is the **source of truth**. The client tracks which events it has seen and catches up on reconnection.

## Core Concept: Sequence Numbers

Every event in a run has a **monotonic `seq` number** (1, 2, 3, ...). This is allocated atomically by the server. The client tracks `last_synced_seq` per run.

```
Server events for run #42:
  seq=1  user_message    "hello"
  seq=2  tool_call       {"tool":"search","args":...}
  seq=3  tool_result     {"results":[...]}
  seq=4  message         "Here's what I found..."
  seq=5  state_change    {"status":"completed"}
```

## Sync Flow

### Normal Operation (Online)

```
Client                          Server
  │                               │
  │──── POST /v1/runs/:id/events ────▶  User sends message
  │                               │  Server creates event seq=N
  │                               │
  │◀─── WebSocket: event seq=N ─────  Server pushes event
  │                               │
  │──── POST /v1/runs/:id/events ────  (if client sends more)
  │                               │
  │◀─── WebSocket: event seq=N+1 ────  Agent responds
  │                               │
  │  Client updates last_synced   │
  │  = N+1                        │
```

### Reconnection After Offline

```
Client                          Server
  │                               │
  │  [offline for 5 minutes]      │  Agent kept running:
  │                               │  seq=3,4,5,6,7,8 created
  │                               │
  │──── GET /v1/runs/:id ──────────▶  "What's the run status?"
  │◀─── {status:"completed",       │
  │      last_seq: 8}             │
  │                               │
  │──── GET /v1/runs/:id/events   │
  │      ?after_seq=2 ────────────▶  "Give me events after seq=2"
  │◀─── [{seq=3,...},{seq=4,...}, │
  │      {seq=5,...},...{seq=8}]  │
  │                               │
  │  Client applies missed events│
  │  Updates last_synced = 8      │
  │  UI shows all responses       │
```

### Client Comes Online With Queued Messages

```
Client                          Server
  │                               │
  │  [was offline, has queued     │
  │   message: "search for X"]    │
  │                               │
  │──── POST /v1/runs/:id/events ────▶  Send queued message
  │      (idempotency_key)        │  Server creates event seq=N
  │                               │
  │◀─── WebSocket: event seq=N ─────  Acknowledged
  │                               │
  │  Client removes from local    │
  │  queue, updates last_synced   │
```

## Client-Side Storage

The client maintains a local SQLite/IndexedDB with:

```sql
-- Local message queue (for offline)
CREATE TABLE outbox (
    id INTEGER PRIMARY KEY,
    run_id INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    idempotency_key BLOB(16) NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    synced INTEGER NOT NULL DEFAULT 0
);

-- Last synced seq per run
CREATE TABLE sync_state (
    run_id INTEGER PRIMARY KEY,
    last_synced_seq INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);

-- Local event cache (optional, for offline reading)
CREATE TABLE events_cache (
    id INTEGER PRIMARY KEY,
    run_id INTEGER NOT NULL,
    seq INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(run_id, seq)
);
```

## Sync Protocol Details

### 1. Initial Sync (First Connect)

```
Client → Server: GET /v1/runs?status=running
Server → Client: [{run_id: 42, last_seq: 8, status: "running"}, ...]

For each run:
  Client → Server: GET /v1/runs/42/events?after_seq=0
  Server → Client: [{seq:1,...}, {seq:2,...}, ..., {seq:8,...}]
  Client: last_synced_seq[42] = 8
```

### 2. Incremental Sync (Reconnect)

```
Client reads local: last_synced_seq[42] = 5
Client → Server: GET /v1/runs/42/events?after_seq=5
Server → Client: [{seq:6,...}, {seq:7,...}, {seq:8,...}]
Client: last_synced_seq[42] = 8
```

### 3. Send Message (Online)

```
Client → Server: POST /v1/runs/42/events
                 {event_type: "message", payload: {text: "hello"}}
Server: creates event seq=9
Server → Client: WebSocket push {seq:9, event_type:"message", ...}
Client: last_synced_seq[42] = 9
```

### 4. Send Message (Offline → Online)

```
Client: stores in outbox (synced=0)
Client comes online:
  Client → Server: POST /v1/runs/42/events
                   {event_type: "message", payload: {text: "hello"},
                    idempotency_key: <uuid>}
Server: creates event seq=9
Server → Client: {id: 9, seq: 9}
Client: marks outbox entry as synced=1
Client: last_synced_seq[42] = 9
```

### 5. Conflict Resolution

**Rule: Server is always right.**

- Client sends event with `idempotency_key`
- Server checks if `event_uuid` already exists → returns existing event (idempotent)
- Client never overwrites server state
- If client has stale data, it discards and re-fetches from server

### 6. Run Status Sync

```
Client polls or receives via WebSocket:
  GET /v1/runs/42 → {status: "running", last_seq: 12}
  
If status changed since last sync:
  Client → Server: GET /v1/runs/42/events?after_seq=12
  Server → Client: [{seq:13,...}, {seq:14,...}]
```

## WebSocket Protocol

### Connection
```
ws://server:8642/v1/runs/:id/ws?token=AUTH_TOKEN
```

### Server → Client Messages
```json
{"seq": 15, "event_type": "tool_call", "payload": {...}}
{"seq": 16, "event_type": "tool_result", "payload": {...}}
{"seq": 17, "event_type": "message", "payload": {"text": "..."}}
{"seq": 18, "event_type": "state_change", "payload": {"status": "completed"}}
```

### Client → Server Messages
```json
{"type": "ping"}
{"type": "message", "payload": {"text": "hello"}, "idempotency_key": "..."}
```

### Heartbeat
- Server sends ping every 30s
- Client responds with pong
- If no pong in 10s, server closes connection
- Client reconnects automatically

## Edge Cases

### 1. Server Restart
- All events are in SQLite (durable)
- Client reconnects, fetches missed events
- No data loss

### 2. Client Restart
- Client reads `sync_state` from local DB
- Fetches events after `last_synced_seq`
- Resumes normally

### 3. Network Partition
- Client queues messages locally
- Server continues processing
- On reconnect: client syncs server state, sends queued messages
- Idempotency ensures no duplicates

### 4. Run Completes While Client Offline
- Client reconnects, sees `status: "completed"`
- Fetches all missed events
- Displays final state

### 5. Multiple Clients
- Each client tracks its own `last_synced_seq`
- All clients see the same server state
- No client-to-client sync needed

## Implementation TODO

- [ ] WebSocket endpoint in Axum
- [ ] `after_seq` parameter on GET /v1/runs/:id/events
- [ ] Client-side outbox + sync_state tables
- [ ] Auto-reconnect with exponential backoff
- [ ] Heartbeat mechanism
- [ ] Offline message queue in client
