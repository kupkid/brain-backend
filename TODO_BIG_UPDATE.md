# TODO: Major UI/UX + Architecture Update

## Phase 0: Critical Bug Fixes (HOTFIX)

- [ ] **FIX: "Connection failed: unknown"** — ошибка появляется при отправке сообщения. Причина: `AgentRepository.kt` отправляет WS message, но somewhere ловит close/error и показывает "unknown". Исправить: `onFailure` callback с proper message
- [ ] **FIX: Token display** — показывать только токены ТЕКУЩЕГО ответа, не кумулятивные. Сейчас `tokens_used` из последнего LLM call. Добавить `per_message_tokens` в `DoneEvent`
- [ ] **FIX: Response text без bubble** — ответ агента должен растягиваться на весь экран, без скруглённого бэкграунда. Только plain text

## Phase 1: Chat UI Redesign (Skeleton Tool Calls)

- [ ] **Tool calls: skeleton loading animation** — вместо карточек с бордером, показывать текст с gradient shimmer (как в фото 1). Пока tool выполняется: `█████░░░░░░` shimmer текст
- [ ] **Tool calls: иконки** — 🔍 search, 📁 file, ⚙️ shell, 📝 todo, 🌐 browser. Иконка + краткое описание + call_id
- [ ] **Tool calls: expand/collapse** — свернуть/развернуть по тапу. Результат показывается в expanded state
- [ ] **Tool calls: время выполнения** — "Думал 1,8 секунд" под каждым tool call
- [ ] **Streaming text** — ответ агента стримится символ за символом (не чанками). `TextEvent` → append to current message
- [ ] **Copy/Regenerate/Voice/Translate/More** — кнопки под ответом (иконки как в фото 1-2)

## Phase 2: Token Stats (per-message)

- [ ] **Stats bar** — `↑12,3K tokens ↓2,6K tokens ⚡113,9 tok/s ⏱23,1s` под каждым ответом
- [ ] **Server-side**: `DoneEvent` добавить `tokens_input`, `tokens_output`, `tokens_per_sec`, `elapsed_ms`
- [ ] **Android**: `DoneEvent` расширенная, stats bar в UI
- [ ] **Форматирование**: 12300 → "12,3K", 1200 → "1,2K"

## Phase 3: Provider System (Multi-Provider)

- [ ] **Server: Provider model** — `providers` SQLite table: id, name, base_url, api_key (encrypted), enabled, type (openai/anthropic/google)
- [ ] **Server: Provider CRUD** — POST/GET/PUT/DELETE `/v1/providers`
- [ ] **Server: Provider models endpoint** — `GET /v1/providers/:id/models` → proxy to provider's /v1/models, parse capabilities
- [ ] **Android: ProvidersScreen** — список провайдеров (card-based, как фото 3). Каждая карточка: иконка, название, статус (вкл/выкл), кол-во моделей
- [ ] **Android: AddProviderDialog** — "Добавить поставщика" с пресетами (OpenAI, Google, Claude, Custom). Поля: имя, API key (с eye toggle), base URL, API path
- [ ] **Android: ProviderDetailScreen** — per-provider settings, model list, toggle enabled/disabled

## Phase 4: Multi-Model Selection

- [ ] **Android: Model selector в чате** — внизу чата (над input) горизонтальный список моделей. Каждая модель = chip с названием. Выбранная = highlighted
- [ ] **Android: Full-screen model picker** — по тапу на модель открывается full-screen список моделей со всех провайдеров
- [ ] **Android: Model capabilities** — при fetch определять: type (chat/embedding/image), input (text/image/audio/video), capabilities (tools, reasoning, vision)
- [ ] **Android: Model editor** — как фото 4: табы (Basic/Advanced/Tools), chips для type/input/output/capabilities

## Phase 5: Model Capabilities Detection

- [ ] **Server: Parse /v1/models response** — извлекать capabilities из ответа провайдера (engine_data, owned_by, etc.)
- [ ] **Server: Fallback capabilities** — если неизвестно: default to chat + text + tools (для большинства моделей)
- [ ] **Server: Manual capabilities** — allow user to set capabilities via PUT /v1/providers/:id/models/:model_id
- [ ] **Android: Display capabilities** — badge/chips: 🔧 Tools, 🧠 Reasoning, 👁 Vision, 🎵 Audio, 🎬 Video

## Phase 6: File/Photo Attachments

- [ ] **Server: File upload endpoint** — `POST /v1/files` → save to workspace, return file_id
- [ ] **Server: File serve** — `GET /v1/files/:id` → serve file
- [ ] **Android: Attach button** — "+" в input bar → gallery/camera/file picker
- [ ] **Android: Image preview** — thumbnail в чате перед отправкой
- [ ] **Android: Send as tool** — файл → base64 → в tool call (если модель поддерживает vision)

## Phase 7: Chat Persistence (SQLite)

- [ ] **Server: Chat table** — `chats` table: id, title, model, provider, created_at, updated_at
- [ ] **Server: Message table** — `messages` table: id, chat_id, role, content, tokens, created_at
- [ ] **Server: Event table** — `chat_events` table: id, message_id, event_type, payload_json
- [ ] **Android: ChatRepository SQLite** — заменить SharedPreferences+JSON на Room DB
- [ ] **Android: Chat export/import** — backup/restore для миграции

## Phase 8: Settings Redesign (MD Expressive)

- [ ] **Settings: Sections** — 🤖 Агент (модель по умолч, system prompt), 🌐 Провайдеры, 🔑 API ключи, 📁 Файлы, 💾 Бэкапы
- [ ] **Settings: Add Provider card** — красная карточка "Добавить поставщика" (как фото 3)
- [ ] **Settings: Provider cards** — свернутый список с toggle, иконкой, названием, моделью
- [ ] **Settings: Model editor** — full-screen как фото 4 с табами

## Phase 9: Build Info + Docs

- [ ] **GitHub Actions: server info** — в конце build лога: RAM, CPU, disk, OS
- [ ] **Code documentation** — rustdoc для всех pub модулей, KDoc для Android классов
- [ ] **README update** — обновить с новыми features

## Phase 10: Security Hardening

- [ ] **API key storage** — use Android Keystore (not SharedPreferences)
- [ ] **WS auth token** — JWT для WebSocket connections
- [ ] **Rate limiting** — per-IP on server
- [ ] **Input sanitization** — escape markdown in tool outputs

## Implementation Order

1. Commit current work ✓
2. Phase 0 (bug fixes — hotfix)
3. Phase 1 (skeleton tool calls)
4. Phase 2 (token stats)
5. Phase 7 (SQLite chats — replaces JSON)
6. Phase 3 (multi-provider server)
7. Phase 4 (multi-model UI)
8. Phase 5 (capabilities)
9. Phase 6 (file attachments)
10. Phase 8 (settings redesign)
11. Phase 9 (docs + build info)
12. Phase 10 (security)
