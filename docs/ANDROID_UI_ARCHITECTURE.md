# Android UI Architecture — Brain App

## Overview

Jetpack Compose + Material3 dark-first app connecting to Brain Backend via WebSocket + REST.
Based on LastChat UI patterns (AMOLED dark, grouped bubbles, activity pills, section-based settings).

## File Map

```
android/app/src/main/java/com/brain/app/
├── AgentEvent.kt              — sealed class hierarchy (7 event types)
├── AgentRepository.kt         — WebSocket client (OkHttp → Flow<AgentEvent>)
├── AgentViewModel.kt          — AndroidViewModel, multi-chat, persistence
├── BrainSettings.kt           — SharedPreferences + mutableStateOf
├── ChatRepository.kt          — Chat persistence (SharedPreferences + JSON)
├── MainActivity.kt            — Entry point, navigation, drawer
├── ui/
│   ├── AgentChatScreen.kt     — Main chat: bubbles, tools, streaming, stats
│   ├── ProvidersScreen.kt     — Provider CRUD with presets
│   ├── ProviderDetailScreen.kt — Per-provider model list
│   ├── ModelEditorScreen.kt   — Model capabilities editor
│   ├── SettingsScreen.kt      — Section-based settings
│   └── theme/
│       └── Theme.kt           — AMOLED dark, dynamic color, shapes
```

## Navigation Flow

```
MainActivity
├── showSettings → SettingsScreen (overlay)
├── showProviders → ProvidersScreen (overlay)
├── providerDetailId → ProviderDetailScreen (overlay)
├── modelEditProviderId → ModelEditorScreen (overlay)
└── else → ModalNavigationDrawer
    ├── DrawerContent (chat history)
    └── AgentChatScreen (main chat)
```

## Data Flow

```
User types → AgentViewModel.sendTask()
  → AgentRepository.connect(task): Flow<AgentEvent>
    → OkHttp WebSocket → ws://host/ws/agent
    → Server sends TaskRequest JSON
    → Server spawns AgentLoop → WsAgentEvent stream
    → AgentRepository parses → AgentEvent sealed class
    → Flow collected in ViewModel → StateFlow<List<AgentEvent>>
    → UI recomposes
```

## Key Patterns

### 1. WebSocket Communication
- OkHttp with pingInterval(30s) for keepalive
- readTimeout(0) for long-lived connections
- Auth via `Authorization: Bearer <key>` header
- TaskRequest sent on open: `{"task": "...", "mode": null}`
- Events parsed via parseAgentEvent() with try-catch fallback

### 2. Event Streaming
- ThoughtEvent: user messages (displayed as right-aligned bubbles)
- TextEvent: streaming text chunks (accumulated in UI)
- ToolCallEvent: tool execution start (shimmer skeleton)
- ToolResultEvent: tool result (green/red badge)
- TodoUpdateEvent: todo list progress
- FileReadEvent: file content preview
- DoneEvent: final response + token stats
- ErrorEvent: connection/LLM errors

### 3. Chat Persistence
- ChatRepository stores ChatSession objects as JSON in SharedPreferences
- Each session has: id, title, model, messages, timestamps
- Messages stored as SerializedEvent for reconstruction
- Chats grouped by date in drawer (Today/Yesterday/date)

### 4. Settings Flow
- BrainSettings: SharedPreferences + mutableStateOf for live recompose
- Server config: host + API key → testConnection() → save
- Provider config: URL + API key → fetchModels() → save to server
- Provider CRUD: REST calls to /v1/providers/* with Bearer auth

## Security

- API keys: SharedPreferences (will migrate to Android Keystore)
- Server URL: SharedPreferences
- No secrets in logs or GitHub Actions
- Bearer auth on all REST endpoints
- WebSocket auth via Authorization header

## Theme (LastChat-inspired)

- AMOLED black (#000000) forced for background + surface
- Dynamic color on Android 12+ (primary/secondary/tertiary from wallpaper)
- Custom shapes: CardLarge(28dp), ButtonPill(50%), InputField(20dp)
- Message bubbles: User(20/20/20/6dp corners), Assistant(20/20/6/20dp)
- Status bar + nav bar forced to black

## Known Issues

1. "Ошибка загрузки: null" — ProvidersScreen loadProviders() swallows exception details
2. Chat persistence uses JSON not SQLite (will migrate to Room)
3. No Android Keystore for API keys yet
4. No WS reconnect logic (connection lost = error shown)
5. Model selector not persisted across app restarts
