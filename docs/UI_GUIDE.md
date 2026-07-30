# Android UI Guide — Brain App

## Visual Language

**Theme**: Pure AMOLED dark (#000000 background). Dynamic color on Android 12+ (primary from wallpaper).

**Shapes** (from LastChat's AppShapes):
- `MessageOutgoing`: 20/20/20/6dp (user, right-aligned)
- `MessageIncoming`: 20/20/6/20dp (assistant, left-aligned)
- `InputField`: 20dp rounded
- `ButtonPill`: 50% fully rounded
- `CardLarge`: 28dp, `CardSmall`: 16dp

**Colors**:
- Primary: Blue (#90CAF9) for user bubbles, accents
- Green (#4CAF50) for success, file operations
- Blue (#2196F3) for file read operations
- Orange (#FF9800) for search
- Purple (#9C27B0) for todo
- Cyan (#00BCD4) for browser
- Red (#EF5350) for errors

## Component Architecture

### 1. GroupedMessageBubble
iMessage-style stacked bubbles with smart corner radii:
- `BubblePosition`: SINGLE, FIRST, MIDDLE, LAST
- `BubbleRole`: USER (right), ASSISTANT (left), ACTIVITY
- Small corners (6dp) where bubbles connect
- Large corners (20dp) on exposed edges

### 2. ActivityPill
Tool call indicator with morphing animation:
- Each tool type has its own color and icon
- Shimmer animation while running
- Inline result badge (green ✓ / red ✗)

### 3. ChatInputArea
Full-width rounded input with model picker:
- Pill-shaped text field (20dp corners)
- Model selector chip (tap to expand dropdown)
- Send button (arrow up, primary color)
- Max 6 lines expandable

### 4. SettingsItem
Section-based settings with:
- 40dp circular icon container
- Title (16sp, white) + subtitle (13sp, gray)
- Chevron right (20dp, dimmed)
- Press-scale animation (0.97f)
- Haptic feedback on tap

### 5. ProviderCard
Provider management cards:
- Letter avatar in colored circle
- Name + URL + model count badge
- Toggle, fetch models, delete actions
- Expand/collapse on tap

## Navigation Flow

```
MainActivity
├── showSettings → SettingsScreen (full-screen overlay)
├── showProviders → ProvidersScreen (full-screen overlay)
├── providerDetailId → ProviderDetailScreen (full-screen overlay)
├── modelEditProviderId → ModelEditorScreen (full-screen overlay)
└── else → ModalNavigationDrawer
    ├── DrawerContent (chat history, grouped by date)
    │   ├── "Brain" header + New Chat + Settings buttons
    │   ├── Divider
    │   └── LazyColumn: "Сегодня"/"Вчера"/date → chat items
    └── AgentChatScreen (main chat area)
        ├── TopAppBar (menu + title + new chat + stop)
        ├── LazyColumn (events)
        │   ├── EmptyState (icon + "Чем могу помочь?")
        │   ├── UserBubble (right-aligned, primary color)
        │   ├── ActivityPill (tool call indicator)
        │   ├── ToolResultInline (green/red badge)
        │   ├── TodoProgress (progress bar + items)
        │   ├── FileReadBlock (monospace header + content)
        │   ├── StreamingText (full-width, copy button)
        │   ├── ResponseBlock (text + action buttons + stats)
        │   └── ErrorBlock (red card + warning icon)
        └── ChatInputArea (model picker + text field + send)
```

## WebSocket Communication

```
User types task → AgentViewModel.sendTask()
  → AgentRepository.connect(task): Flow<AgentEvent>
    → OkHttp WebSocket → ws://host/ws/agent
    → Authorization: Bearer <key>
    → Server sends TaskRequest JSON
    → Server spawns AgentLoop → WsAgentEvent stream
    → AgentRepository parses → AgentEvent sealed class
    → Flow collected in ViewModel → StateFlow<List<AgentEvent>>
    → UI recomposes via LazyColumn
```

## Event Types

| Event | Display | Source |
|-------|---------|--------|
| `ThoughtEvent` | User message (right bubble) | User input |
| `TextEvent` | Streaming response text | LLM output |
| `ToolCallEvent` | ActivityPill (shimmer while running) | Agent tool call |
| `ToolResultEvent` | Inline badge (green/red) | Tool result |
| `TodoUpdateEvent` | Progress bar + checkbox list | Todo tool |
| `FileReadEvent` | Monospace path + content preview | File read tool |
| `DoneEvent` | Response text + stats bar | LLM completion |
| `ErrorEvent` | Red card with warning icon | Connection/LLM error |

## Stats Bar Format

```
↑12,3K ↓2,6K ⚡113,9 tok/s ⏱23,1s
```

- `↑` = tokens_input, `↓` = tokens_output
- `⚡` = tokens_per_sec (from API usage)
- `⏱` = elapsed_ms / 1000

## Settings Structure

### Общие настройки (General)
- 🎨 Тема (Theme) — Dark mode
- ⚙️ Настройки (Settings) — Server, connection
- 🤖 Ассистент (Assistant) — Model, behavior

### Модели и службы (Models & Services)
- ✨ Модель по умолчанию — Default model
- ☁️ Провайдеры — AI providers CRUD
- 🔍 Служба поиска — Search service
- 🎙 Голос — TTS/STT
