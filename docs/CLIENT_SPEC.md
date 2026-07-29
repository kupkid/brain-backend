# Brain Backend — Client Specification

## Overview

The client is a **thin app** (mobile/desktop) that communicates with the Brain Server. The agent runs on the server; the client handles UI, local queuing, and sync.

## Client Responsibilities

1. **UI Rendering**: Display conversations, memories, run status
2. **Message Sending**: User inputs → server events
3. **Event Receiving**: Real-time updates via WebSocket
4. **Offline Queue**: Store messages when disconnected
5. **Sync Management**: Catch up on missed events after reconnection
6. **Auth Management**: Store and refresh auth tokens

## Architecture

```
┌─────────────────────────────────────┐
│           Client App                 │
├─────────────────────────────────────┤
│  UI Layer                           │
│  ├─ Chat view                       │
│  ├─ Memory browser                  │
│  ├─ Run status                      │
│  └─ Settings                        │
├─────────────────────────────────────┤
│  State Manager                      │
│  ├─ Current run state               │
│  ├─ Event cache                     │
│  └─ Sync state (last_synced_seq)    │
├─────────────────────────────────────┤
│  Network Layer                      │
│  ├─ HTTP client                     │
│  ├─ WebSocket client                │
│  └─ Auth interceptor                │
├─────────────────────────────────────┤
│  Local Storage (SQLite/IndexedDB)   │
│  ├─ outbox (offline message queue)  │
│  ├─ sync_state (seq tracking)       │
│  ├─ events_cache (recent events)    │
│  └─ settings (auth token, etc.)     │
└─────────────────────────────────────┘
```

## Local Storage Schema

### outbox (Offline Message Queue)
```sql
CREATE TABLE outbox (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id INTEGER NOT NULL,
    event_type TEXT NOT NULL DEFAULT 'message',
    payload TEXT NOT NULL,
    idempotency_key BLOB(16) NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    synced INTEGER NOT NULL DEFAULT 0
);
```

### sync_state (Seq Tracking)
```sql
CREATE TABLE sync_state (
    run_id INTEGER PRIMARY KEY,
    last_synced_seq INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### events_cache (Recent Events)
```sql
CREATE TABLE events_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id INTEGER NOT NULL,
    seq INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(run_id, seq)
);
```

### settings
```sql
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
-- Keys: auth_token, server_url, last_sync_at
```

## API Usage

### Initial Sync (App Start)

```javascript
// 1. Load settings
const serverUrl = getSetting('server_url');
const authToken = getSetting('auth_token');

// 2. Fetch active runs
const runs = await fetch(`${serverUrl}/v1/runs?status=running`, {
  headers: { 'Authorization': `Bearer ${authToken}` }
});

// 3. For each run, sync missed events
for (const run of runs) {
  const lastSynced = getSyncState(run.id) || 0;
  const events = await fetch(
    `${serverUrl}/v1/runs/${run.id}/events?after_seq=${lastSynced}`,
    { headers: { 'Authorization': `Bearer ${authToken}` } }
  );
  
  // Apply events to local cache
  for (const event of events) {
    applyEvent(event);
    updateSyncState(run.id, event.seq);
  }
}
```

### Send Message (Online)

```javascript
async function sendMessage(runId, text) {
  const idempotencyKey = crypto.randomUUID();
  
  // 1. Send to server
  const response = await fetch(`${serverUrl}/v1/runs/${runId}/events`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${authToken}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({
      event_type: 'message',
      payload: JSON.stringify({ text }),
      idempotency_key: idempotencyKey
    })
  });
  
  // 2. Update sync state
  const { seq } = await response.json();
  updateSyncState(runId, seq);
}
```

### Send Message (Offline → Online)

```javascript
async function sendMessageOffline(runId, text) {
  const idempotencyKey = crypto.randomUUID();
  
  // 1. Store in outbox
  db.run(
    'INSERT INTO outbox (run_id, payload, idempotency_key) VALUES (?, ?, ?)',
    [runId, JSON.stringify({ text }), idempotencyKey]
  );
  
  // 2. Show in UI immediately (optimistic)
  addMessageToUI(runId, { text, pending: true });
}

// When coming online:
async function syncOutbox() {
  const pending = db.all('SELECT * FROM outbox WHERE synced = 0');
  
  for (const msg of pending) {
    try {
      const response = await fetch(`${serverUrl}/v1/runs/${msg.run_id}/events`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${authToken}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          event_type: msg.event_type,
          payload: msg.payload,
          idempotency_key: msg.idempotency_key
        })
      });
      
      const { seq } = await response.json();
      updateSyncState(msg.run_id, seq);
      
      // Mark as synced
      db.run('UPDATE outbox SET synced = 1 WHERE id = ?', [msg.id]);
      
      // Update UI (remove pending indicator)
      confirmMessage(msg.run_id, seq);
    } catch (e) {
      // Will retry on next sync
      console.error('Sync failed:', e);
    }
  }
}
```

### WebSocket Connection

```javascript
function connectWebSocket(runId) {
  const ws = new WebSocket(
    `ws://${serverUrl}/v1/runs/${runId}/ws?token=${authToken}`
  );
  
  ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    
    switch (data.type) {
      case 'event':
        applyEvent(data.event);
        updateSyncState(runId, data.event.seq);
        break;
      case 'ping':
        ws.send(JSON.stringify({ type: 'pong' }));
        break;
    }
  };
  
  ws.onclose = () => {
    // Reconnect with exponential backoff
    setTimeout(() => connectWebSocket(runId), 3000);
  };
}
```

## Offline Behavior

### When Offline
1. User sends message → stored in outbox
2. Message shown in UI with "pending" indicator
3. WebSocket disconnected (no events received)

### When Coming Online
1. Sync outbox (send queued messages)
2. Connect WebSocket
3. Fetch missed events (after_seq)
4. Update UI with server state
5. Remove "pending" indicators

### Conflict Resolution
- Server is always source of truth
- Client discards local state if server has newer seq
- Idempotency keys prevent duplicate events

## UI Components

### Chat View
- Message list (user + agent messages)
- Input field
- Send button
- Connection status indicator (online/offline)
- Pending message indicators

### Memory Browser
- List of memories (filterable by layer, type)
- Search bar (hybrid search)
- Memory detail view
- Create/edit/delete actions

### Run Status
- Current run status (pending/running/completed/failed)
- Event log (scrollable)
- Tool invocations with timing
- Token usage and cost

### Settings
- Server URL configuration
- Auth token management
- Offline mode toggle
- Sync status

## Technology Choices

### Recommended Stack
- **Flutter**: Cross-platform (iOS/Android/Desktop), good SQLite support
- **React Native**: Cross-platform, larger ecosystem
- **Swift/Kotlin**: Native performance, platform-specific

### Local Storage
- **SQLite**: For outbox, sync_state, events_cache
- **Hive/Realm**: Alternative for simpler key-value storage

### Networking
- **HTTP**: reqwest (Rust), dio (Flutter), axios (JS)
- **WebSocket**: tungstenite (Rust), web_socket_channel (Flutter)

## Implementation Priority

1. **MVP**: HTTP API + basic chat UI
2. **Phase 2**: WebSocket for live updates
3. **Phase 3**: Offline queue + sync
4. **Phase 4**: Memory browser
5. **Phase 5**: Run status + tool visualization
