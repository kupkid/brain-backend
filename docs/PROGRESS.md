# Brain App — UI/UX Progress Tracker

## Current State (Session: UI Overhaul)
- **37+ commits** on `kupkid/brain-backend`
- Server: `148.253.209.232:3000`, systemd, 49 tests pass
- Android: Kotlin + Compose + OkHttp WS, APK builds via GH Actions
- **LastChat cloned** at `/root/projects/LastChat/` for UI reference

## What's Done
- [x] Server: DB-driven providers, vault, memories, runs, workspace, agent loop
- [x] WebSocket: /ws/agent endpoint, keepalive ping/pong, EventBus
- [x] Android: Basic chat, settings, providers CRUD, model editor
- [x] APK: builds via GitHub Actions (15MB)
- [x] CI: cargo check + fmt + clippy + test

## This Session: Full UI Overhaul
### Phase 1: Foundation ✅
- [x] Theme.kt — AMOLED dark + AppShapes + dynamic color
- [x] Shape.kt — LastChat shape tokens (MessageOutgoing/Incoming, CardLarge, etc.)
- [x] PROGRESS.md — this file

### Phase 2: Chat UI (LastChat-inspired) ✅
- [x] GroupedMessageBubble — iMessage-style smart corners (SINGLE/FIRST/MIDDLE/LAST)
- [x] ActivityPill — tool call indicator with shimmer + color per tool type
- [x] AgentChatScreen — grouped bubbles, activity pills, streaming text, stats
- [x] ChatInput — pill shape + model picker + send button
- [x] Empty state — centered "Чем могу помочь?"

### Phase 3: Settings + Providers ✅
- [x] SettingsScreen — section-based, haptic items, server config dialog
- [x] ProvidersScreen — fix "Ошибка загрузки: null" + error colors + safe response checks

### Phase 4: Connection + Security (partial)
- [x] WebSocket: OkHttp keepalive ping/pong (30s)
- [x] Error messages: descriptive connection errors (timeout, refused, reset, 401, 404)
- [ ] WebSocket reconnect logic (exponential backoff) — TODO
- [ ] Android Keystore for API keys — TODO
- [ ] Room DB for chat persistence — TODO

### Phase 5: Documentation ✅
- [x] ANDROID_UI_ARCHITECTURE.md — file map, data flow, security
- [x] PROGRESS.md — this file
- [x] UI_GUIDE.md — visual language, components, navigation

## Key Decisions
1. **AMOLED black** (#000000) forced for background + surface (LastChat pattern)
2. **Smart corners**: User=20/20/20/6dp, Assistant=20/20/6/20dp (iMessage style)
3. **Activity pills** instead of card-based tool calls (LastChat pattern)
4. **Plus button** in chat input for model picker + attachments
5. **Section-based settings** with icon + title + chevron items
6. **WebSocket reconnect** with exponential backoff
7. **Android Keystore** for API keys (EncryptedSharedPreferences)

## File Structure (Target)
```
android/app/src/main/java/com/brain/app/
├── AgentEvent.kt              — sealed class (7 event types) ✓
├── AgentRepository.kt         — WS client + reconnect
├── AgentViewModel.kt          — AndroidViewModel, multi-chat ✓
├── BrainSettings.kt           — SharedPreferences ✓
├── ChatRepository.kt          — Chat persistence ✓
├── MainActivity.kt            — Navigation + drawer
├── ui/
│   ├── AgentChatScreen.kt     — Chat: grouped bubbles + activity pills
│   ├── ProvidersScreen.kt     — Provider CRUD (fixed)
│   ├── ProviderDetailScreen.kt — Model list ✓
│   ├── ModelEditorScreen.kt   — Model editor ✓
│   ├── SettingsScreen.kt      — Section-based settings
│   └── theme/
│       ├── Theme.kt           — AMOLED dark + shapes
│       └── Shape.kt           — AppShapes tokens
```
