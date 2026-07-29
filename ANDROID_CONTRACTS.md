# Android Contracts — Brain Backend API ↔ UI

> Каждая строка = функция бэкенда → компонент Android Material 3

## API Endpoints

| Функция | Эндпоинт | Модель данных | MD3 Компонент | Обновление | Приоритет |
|---------|----------|---------------|---------------|------------|-----------|
| Todo list | `GET /v1/runs/{id}/todos` | `{tasks: [{id, title, status, ...}]}` | `LazyColumn` + `TaskCard` (Checkbox + Title + StatusChip) | Poll / WebSocket | P0 |
| Todo live update | WebSocket `/v1/runs/{id}/ws` | `TodoEvent {task_id, status}` | Snackbar / анимация чипа | Real-time | P0 |
| Agent status | `GET /v1/runs/{id}` | `StoredRun {status, summary, tokens_used}` | `LinearProgressIndicator` + `Text` (status) | Poll 1s / WS | P0 |
| Run events | `GET /v1/runs/{id}/events` | `{events: [{seq, event_type, payload}]}` | `LazyColumn` + `EventCard` | Poll / WS | P1 |
| Tool stats | `GET /v1/runs/{id}/tools/stats` | `ToolStats {total, success, errors, total_tokens}` | `Row` + `Badge` counters | On demand | P1 |
| Context slots | `GET /v1/runs/{id}/context` | `{slots: [{slot, content}]}` | `ExpandableCard` per slot | On demand | P2 |
| Memories | `GET /v1/memories?project_id=N` | `{memories: [{content, layer, importance}]}` | `LazyColumn` + `MemoryCard` | On demand | P2 |
| Memory search | `POST /v1/memories/search` | `{query, results: [...]}` | `SearchBar` + `LazyColumn` | On type (debounce) | P2 |
| Projects | `GET /v1/projects` | `{projects: [{id, name, root_path}]}` | `LazyColumn` + `ProjectCard` | On open | P0 |
| Create project | `POST /v1/projects` | `{name}` → `Project` | `Dialog` → `TextField` + `Button` | On action | P0 |
| Vault creds | `GET /v1/credentials` | `{credentials: [{name, key_version}]}` | `LazyColumn` + `CredentialCard` (lock icon) | On demand | P1 |
| Workspace files | `GET /v1/projects/{id}/workspace` | `{entries: [{name, type, size}]}` | `LazyColumn` + `FileRow` (icon + name + size) | On demand | P1 |
| Read file | `GET /v1/projects/{id/workspace/*path}` | `{content, encoding}` | `CodeBlock` / `Text` | On tap | P1 |
| Health | `GET /health` | `{status: "ok"}` | Splash screen indicator | App start | P0 |

## WebSocket Events

| Event Type | Payload | UI Action |
|------------|---------|-----------|
| `todo_update` | `{task_id, status}` | Animate chip color change |
| `tool_call` | `{name, arguments}` | Show tool chip with spinner |
| `tool_result` | `{name, output, success}` | Update chip to ✓/✗ |
| `state_change` | `{from, to}` | Update `LinearProgressIndicator` |
| `message` | `{content}` | Append to chat |
| `error` | `{message}` | `Snackbar` (red) |

## Polling Strategy

| Экран | Интервал | Причина |
|-------|----------|---------|
| Run progress (todos) | 1s / WebSocket | Критично — пользователь ждёт |
| Agent status | 1s / WebSocket | Прогресс-бар |
| Todo live | WebSocket push | Real-time |
| Memories | On demand | Не критично |
| Vault | On demand | Редко меняется |
| Projects | On open | Статичные данные |

## Material 3 Компоненты

| Компонент | Где используется | Пример |
|-----------|-----------------|--------|
| `LazyColumn` | Списки (todos, memories, events) | `LazyColumn { items(todos) { TaskCard(it) } }` |
| `TaskCard` | Todo item | `Card` + `Checkbox` + `Text` + `AssistChip` (status) |
| `LinearProgressIndicator` | Run progress | `LinearProgressIndicator(progress = done/total)` |
| `Snackbar` | Ошибки, тосты | `snackbarHostState.showSnackbar("Error")` |
| `SearchBar` | Memory search | `SearchBar(query = ..., onQueryChange = ...)` |
| `AssistChip` | Статус таска | `AssistChip(label = { Text("done") })` |
| `Badge` | Счётчики | `Badge { Text("$count") }` |
| `CircularProgressIndicator` | Loading | `CircularProgressIndicator()` |
| `NavigationRail` | Навигация | Agent / Projects / Settings |
| `BottomSheet` | Детали таска | `ModalBottomSheet` с описанием |

## Data Flow

```
Android App
  │
  ├── GET /v1/runs/{id}/todos  ──→  LazyColumn<TaskCard>
  ├── WS /v1/runs/{id}/ws      ──→  Real-time updates
  ├── GET /v1/runs/{id}        ──→  Progress bar
  ├── POST /v1/memories/search  ──→  SearchBar results
  └── GET /v1/projects         ──→  Project list
```
