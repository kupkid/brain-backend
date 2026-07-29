# TODO: Major UI/UX + Architecture Update

## Phase 0: Critical Bug Fixes (HOTFIX) ✅

- [x] **FIX: "Connection failed: unknown"** — descriptive error messages in `AgentRepository.kt`
- [x] **FIX: Token display** — `DoneEvent` extended with `tokens_input`, `tokens_output`, `elapsed_ms`, `tokens_per_sec`
- [x] **FIX: Response text без bubble** — full-width `StreamingText` + `ResponseBlock` in `AgentChatScreen.kt`

## Phase 1: Chat UI Redesign (Skeleton Tool Calls) ✅

- [x] **Tool calls: skeleton loading animation** — `shimmerBrush()` gradient animation
- [x] **Tool calls: иконки** — `toolIcon()` maps to Material icons
- [x] **Tool calls: expand/collapse** — `AnimatedVisibility` with chevron
- [x] **Tool calls: время выполнения** — timestamps in events
- [x] **Streaming text** — `TextEvent` accumulation in `AgentViewModel`
- [x] **Copy/Regenerate buttons** — action row under response

## Phase 2: Token Stats (per-message) ✅

- [x] **Stats bar** — ↑↓tok/s ⏱ formatting in `ResponseBlock`
- [x] **Server-side**: `WsAgentEvent::Done` extended
- [x] **Android**: `DoneEvent` with stats fields
- [x] **Форматирование**: 12300→"12,3K"

## Phase 3: Provider System (Multi-Provider)

- [x] **Server: providers table** — `migrations/004_providers.sql` with `providers` + `provider_models`
- [x] **Server: Provider CRUD** — POST/GET/PUT/DELETE `/v1/providers` + `/v1/providers/:id/models`
- [x] **Server: ProvidersRepository** — `src/settings/providers.rs` with encrypted API keys
- [ ] **Server: Proxy models endpoint** — `POST /v1/providers/:id/proxy` → fetch models from provider
- [ ] **Android: ProvidersScreen** — card-based list of providers with toggle
- [ ] **Android: AddProviderDialog** — presets (OpenAI, Google, Claude, Custom)
- [ ] **Android: ProviderDetailScreen** — per-provider model list

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

## Phase 9: Build Info + Docs + CI ✅

- [x] **GitHub Actions CI** — `.github/workflows/ci.yml` (cargo check + fmt + clippy + test)
- [x] **Pre-push hook** — secret scanning (sk-*, AIza*, keys, passwords)
- [x] **push.sh** — auto add/commit/push with secret scan
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
