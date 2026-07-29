# ANDROID CONTRACTS

> Контракты между Brain Backend (Rust) и Android-клиентом (Kotlin + Compose).
> Путь клиента: `android/` (от корня репозитория).

---

## 1. WebSocket: `/ws/agent`

Единственный эндпоинт для запуска агента и получения результатов в реальном времени.

### Подключение

```
ws://{host}:{port}/ws/agent
```

### Клиент → Сервер

При подключении клиент отправляет **одно** JSON-сообщение:

```json
{
  "task": "Создай hello.py и запусти",
  "mode": "auto"
}
```

| Поле | Тип | Описание |
|------|-----|----------|
| `task` | String | Текст задачи для агента |
| `mode` | String? | `"auto"` (default) — агент выполняет всё сам |

### Сервер → Клиент (события)

Каждое событие — одна JSON-строка (разделитель `\n`). Никакого pretty-print.

**1. `thought`** — модель размышляет между вызовами инструментов:
```json
{"type":"thought","text":"Сейчас создам файл...","ts":1712345678}
```

**2. `tool_call`** — агент вызывает инструмент:
```json
{"type":"tool_call","tool":"write_file","args":{"path":"hello.py","content":"print('hi')"},"call_id":"t0","ts":1712345678}
```

**3. `tool_result`** — результат выполнения:
```json
{"type":"tool_result","call_id":"t0","success":true,"summary":"ok (14 bytes)","ts":1712345678}
```

**4. `todo_update`** — состояние списка задач:
```json
{"type":"todo_update","todos":[{"id":"1","text":"Создать файл","status":"done"},{"id":"2","text":"Запустить","status":"pending"}],"ts":1712345678}
```

**5. `file_read`** — агент прочитал файл:
```json
{"type":"file_read","path":"hello.py","text":"print('hi')","ts":1712345678}
```

**6. `done`** — задача завершена:
```json
{"type":"done","summary":"Готово: создан hello.py и запущен","total_tokens":12340,"total_calls":8,"ts":1712345678}
```

**7. `error`** — критическая ошибка:
```json
{"type":"error","message":"Vault не разблокирован","ts":1712345678}
```

### Поля событий

| Поле | Тип | Описание |
|------|-----|----------|
| `type` | String | Тип события (один из 7 выше) |
| `ts` | Long | Unix timestamp в секундах |
| `call_id` | String | Короткий ID вызова: `t0`, `t1`, `t2`... (для `tool_call` и `tool_result`) |
| `success` | Boolean | Успешность (для `tool_result`) |
| `summary` | String | Краткое описание результата (для `tool_result`, `done`) |
| `total_tokens` | Int | Суммарное количество токенов (для `done`) |
| `total_calls` | Int | Количество вызовов инструментов (для `done`) |

### Жизненный цикл

1. Клиент подключается по WebSocket
2. Клиент отправляет `{"task": "...", "mode": "auto"}`
3. Сервер создаёт run и запускает агентский цикл
4. Сервер стримит события: `thought`, `tool_call`, `tool_result`, `todo_update`, `file_read`
5. При завершении: событие `done` (успех) или `error` (ошибка)
6. После `done`/`error` WebSocket закрывается сервером
7. Клиент может закрыть WebSocket принудительно (кнопка «Стоп») → агент останавливается

---

## 2. REST Endpoints (вспомогательные)

### Health
```
GET /health → {"status": "ok"}
```

### Projects
```
GET  /v1/projects → [{id, uuid, name, root_path}]
POST /v1/projects → {name} → {id}
GET  /v1/projects/:id → {id, uuid, name, root_path}
DELETE /v1/projects/:id → 204
```

### Runs
```
GET  /v1/runs → [{id, uuid, status, agent_name, goal}]
POST /v1/runs → {project_id?, agent_name, goal} → {id}
GET  /v1/runs/:id → {id, uuid, status, agent_name, goal}
POST /v1/runs/:id/transition → {to_status, reason?, summary?}
```

### Memories
```
GET  /v1/memories → [{id, content, memory_type, layer, importance, source, created_at}]
POST /v1/memories → {content, memory_type, layer?, project_id?} → {id, is_duplicate}
POST /v1/memories/search → {query, project_id?, limit?} → [{id, content, score, ...}]
```

### Vault
```
GET    /v1/credentials → [{id, name, key_version, tags, created_at}]
POST   /v1/credentials → {name, value, scope?, tags?} → {id}
GET    /v1/credentials/:name → {id, name, scope, key_version, tags}
DELETE /v1/credentials/:name → 204
```

### Workspace
```
GET /v1/projects/:id/workspace → [{path, is_dir, size, modified}]
GET /v1/projects/:id/workspace/*path → {path, content, encoding}
PUT /v1/projects/:id/workspace/*path → {content} → 200
```

---

## 3. Kotlin Data Classes

```kotlin
// === WebSocket Events ===

@Serializable
sealed class AgentEvent {
    abstract val type: String
    abstract val ts: Long
}

@Serializable
data class ThoughtEvent(
    override val type: String = "thought",
    override val ts: Long,
    val text: String
) : AgentEvent()

@Serializable
data class ToolCallEvent(
    override val type: String = "tool_call",
    override val ts: Long,
    val tool: String,
    val args: JsonObject,
    val call_id: String
) : AgentEvent()

@Serializable
data class ToolResultEvent(
    override val type: String = "tool_result",
    override val ts: Long,
    val call_id: String,
    val success: Boolean,
    val summary: String
) : AgentEvent()

@Serializable
data class TodoUpdateEvent(
    override val type: String = "todo_update",
    override val ts: Long,
    val todos: List<TodoItem>
) : AgentEvent()

@Serializable
data class TodoItem(
    val id: String,
    val text: String,
    val status: String
)

@Serializable
data class FileReadEvent(
    override val type: String = "file_read",
    override val ts: Long,
    val path: String,
    val text: String
) : AgentEvent()

@Serializable
data class DoneEvent(
    override val type: String = "done",
    override val ts: Long,
    val summary: String,
    val total_tokens: Int,
    val total_calls: Int
) : AgentEvent()

@Serializable
data class ErrorEvent(
    override val type: String = "error",
    override val ts: Long,
    val message: String
) : AgentEvent()

// === WebSocket Request ===

@Serializable
data class TaskRequest(
    val task: String,
    val mode: String = "auto"
)

// === REST Models ===

@Serializable
data class Project(
    val id: Long,
    val uuid: String,
    val name: String,
    val root_path: String
)

@Serializable
data class Run(
    val id: Long,
    val uuid: String,
    val status: String,
    val agent_name: String,
    val goal: String
)

@Serializable
data class Memory(
    val id: Long,
    val content: String,
    val memory_type: String,
    val layer: String,
    val importance: Double,
    val source: String,
    val created_at: String
)

@Serializable
data class Credential(
    val id: Long,
    val name: String,
    val key_version: Int,
    val tags: List<String>,
    val created_at: String
)
```

---

## 4. Парсинг событий

```kotlin
fun parseAgentEvent(json: String): AgentEvent {
    val obj = Json.parseToJsonElement(json).jsonObject
    return when (obj["type"]?.jsonPrimitive?.content) {
        "thought" -> Json.decodeFromJsonElement<ThoughtEvent>(obj)
        "tool_call" -> Json.decodeFromJsonElement<ToolCallEvent>(obj)
        "tool_result" -> Json.decodeFromJsonElement<ToolResultEvent>(obj)
        "todo_update" -> Json.decodeFromJsonElement<TodoUpdateEvent>(obj)
        "file_read" -> Json.decodeFromJsonElement<FileReadEvent>(obj)
        "done" -> Json.decodeFromJsonElement<DoneEvent>(obj)
        "error" -> Json.decodeFromJsonElement<ErrorEvent>(obj)
        else -> throw IllegalArgumentException("Unknown event type: $obj")
    }
}
```
