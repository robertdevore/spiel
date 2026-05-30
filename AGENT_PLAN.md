# Spiel — Agent Plan

## Current Phase
Phase 12 — Optional OpenAI provider support

## Current Milestone
M12: 🔄 In Progress. Adding optional OpenAI transcription and cleanup providers behind strict local-only gates. Secure API key storage (session-only, documented). No cloud sync, accounts, telemetry, or auto-enable. Local Whisper and basic cleanup remain primary defaults.

## Existing Structure Summary
- 11 phases stabilized, 44 commands, 14 Rust modules
- Transcription: EngineKind enum (Mock, LocalWhisper, Cloud), trait-based engines
- Cleanup: CleanupProviderKind enum (Basic, MockAi, OpenAiPlanned, LocalLlmPlanned), trait-based providers
- Settings: SpielSettings with 18 fields, SQLite-backed, local_only_mode defaults to true
- PrivacyStatus struct already has openai_available field
- 3 Tauri capabilities: core, global-shortcut, clipboard-manager

## Files Expected to Change

**Rust Backend (new):**
- `src-tauri/src/secrets.rs` — SecretStore: in-memory API key storage, set/get_status/delete operations, no persistence
- `src-tauri/src/transcription_openai.rs` — OpenAiTranscriptionEngine: sends audio to OpenAI speech-to-text, validates local-only mode and key
- `src-tauri/src/cleanup_openai.rs` — OpenAiCleanupProvider: sends text to OpenAI chat completions, mode-specific prompts, validates local-only mode and key

**Rust Backend (modified):**
- `src-tauri/Cargo.toml` — Add `reqwest` with `rustls-tls` feature
- `src-tauri/src/transcription.rs` — Add `OpenAi` variant to EngineKind, update all_info()
- `src-tauri/src/cleanup.rs` — Change `OpenAiPlanned` → `OpenAi`, mark implemented when key configured
- `src-tauri/src/settings.rs` — Add 3 fields: cloud_providers_enabled, openai_transcription_model, openai_cleanup_model
- `src-tauri/src/database.rs` — Migration v4 for new settings columns
- `src-tauri/src/app_state.rs` — Add SecretStore to AppState, add cloud_providers capability
- `src-tauri/src/commands.rs` — Add secret commands (set/get_status/delete API key, validate config), OpenAI transcription/cleanup commands, update phase strings
- `src-tauri/src/lib.rs` — Register new modules, commands, manage SecretStore
- `src-tauri/capabilities/default.json` — Add network permission for api.openai.com

**Frontend (modified):**
- `src/lib/types.ts` — Add ApiKeyStatus, OpenAiProviderConfig types
- `src/lib/api.ts` — Add API wrappers for new commands
- `src/App.tsx` — Load API key status, pass to SettingsPanel
- `src/components/SettingsPanel.tsx` — Add OpenAI API key configuration UI
- `src/components/CapturePanel.tsx` — Add OpenAI provider options with cloud badges

**Documentation (modified):**
- `AGENT_PLAN.md` — this file
- `README.md` — Phase 12 features, OpenAI optional, API key handling
- `STATUS.md` — Phase 12 status, cloud provider behavior, security notes
- `IMPLEMENTATION_LOG.md` — Phase 12 entries
- `docs/architecture.md` — New modules, new permission
- `docs/phases.md` — Phase 12 complete
- `docs/security.md` — OpenAI data flow, key storage, network
- `docs/release.md` — New dependency, new permission

## Dependency Plan
- `reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }` — HTTPS to api.openai.com
- No Tauri Stronghold (session-only key storage for Phase 12 — documented limitation)
- No new frontend dependencies

## Tauri Permission Plan
Add 1 new permission:
- Network access to `https://api.openai.com` via Tauri's `connect-src` CSP update

## Secure Secret Storage Plan
- `SecretStore` struct held in AppState Mutex
- API key stored in memory only (not persisted to disk)
- Exposed commands: set_openai_api_key, get_openai_api_key_status, delete_openai_api_key, validate_openai_provider_config
- Key status returns: configured (with last 4 chars), not_configured
- Never returns full key to frontend after initial set
- Documented as session-only (survives app restart: ❌)

## OpenAI Transcription Provider Plan
- EngineKind::OpenAi variant in transcription.rs
- OpenAiTranscriptionEngine struct implementing TranscriptionEngine trait
- Accepts audio file path, reads file, sends multipart form to POST https://api.openai.com/v1/audio/transcriptions
- Model: whisper-1 (configurable via settings)
- Validates: local_only_mode disabled, API key configured, audio file exists
- Returns TranscriptionResult with engine_kind=OpenAi, is_mock=false

## OpenAI Cleanup Provider Plan
- CleanupProviderKind::OpenAi replaces OpenAiPlanned
- OpenAiCleanupProvider struct implementing CleanupProvider trait
- Sends text to POST https://api.openai.com/v1/chat/completions
- Model: gpt-4o-mini (configurable via settings)
- Mode-specific system prompts per the 5 text modes
- Validates: local_only_mode disabled, API key configured
- Returns CleanupResult with provider=OpenAi, is_mock=false

## Local-Only Enforcement Plan
- All OpenAI commands check settings.local_only_mode before proceeding
- If local_only_mode is true, return clear error: "Local-only mode is on. Cloud providers are disabled. Disable local-only mode in Settings to use OpenAI."
- Same check for cloud_providers_enabled setting
- UI shows "Local-only mode is on. Cloud providers are disabled." banner when applicable

## UI/Privacy Warning Plan
- SettingsPanel: OpenAI API key section with configure/delete, shows "configured" or "not configured"
- CapturePanel: provider options marked with ☁️ cloud badge
- Warning shown before first cloud use: "Using OpenAI sends data to api.openai.com"
- Provider errors displayed inline

## Acceptance Criteria
- [ ] OpenAI support is optional, off by default
- [ ] local_only_mode blocks OpenAI providers
- [ ] API key stored in memory only (session-only)
- [ ] API key never returned in plaintext to frontend
- [ ] API key never logged
- [ ] OpenAI transcription engine exists and uses transcription trait
- [ ] OpenAI cleanup provider exists and uses cleanup trait
- [ ] Local Whisper still works
- [ ] Basic cleanup still works
- [ ] Mock providers still work
- [ ] No cloud fallback exists
- [ ] No telemetry, analytics, accounts, or cloud sync added
- [ ] All build checks pass
- [ ] Docs updated

## Verification Plan
- `cargo fmt --check`
- `cargo check`
- `npx tsc --noEmit`
- `npm run build`
- `cargo build`
- Manual test checklist

**Modified (completed):**
- `AGENT_PLAN.md` — this file
- `README.md` — Build & Run section, platform notes links
- `STATUS.md` — Full rewrite with Phase 11 data, permission audit, platform table
- `IMPLEMENTATION_LOG.md` — Phase 9, 10, 11 entries
- `docs/phases.md` — Phase 10/11 marked complete
- `docs/architecture.md` — Updated module plan, permissions section
- `src-tauri/capabilities/default.json` — Phase 11 description
- `src-tauri/src/commands.rs` — Phase 11 strings in get_app_status/get_app_info

## Permission Audit
3 permissions total, all justified:
| Permission | Phase | Purpose |
|-----------|-------|---------|
| core:default | 1 | Tauri runtime |
| global-shortcut:default | 2 | Cmd+Option+. hotkey |
| clipboard-manager:default | 4 | Clipboard read/write |

No network, shell, updater, filesystem, or accessibility permissions. ✅
- `IMPLEMENTATION_LOG.md` — Phase 9 entries
- `STATUS.md` — updated status
- `README.md` — Phase 9 features
- `docs/architecture.md` — settings module
- `docs/phases.md` — Phase 9

## Dependency Plan
- No new Rust or npm dependencies. Reuse rusqlite + serde/serde_json from Phase 8.
- Settings persisted in SQLite `app_settings` table (singleton row).

## Tauri Permission Plan
No new permissions. Same as Phase 8.

## Settings Storage Plan
- SQLite table `app_settings` with a singleton row (id=1)
- Migration v2 creates the table if missing
- Load on startup, save on change
- Reset restores safe defaults without deleting history

## Settings Schema Plan
Fields: hotkey, history_enabled, clipboard_restore_enabled, default_transcription_engine, default_text_mode, default_cleanup_provider, local_whisper_binary_path, local_whisper_model_path, local_only_mode, debug_logging_enabled, created_at, updated_at

## Integration Plan
- `history_enabled` → wired to Phase 8 `set_history_enabled` command
- `clipboard_restore_enabled` → wired to Phase 4 insert button restore flag
- `local_whisper_binary_path/model_path` → wired to Phase 6 WhisperConfig
- `default_text_mode` → wired to Phase 7 mode selector
- `default_cleanup_provider` → wired to Phase 7 provider selector
- Settings not wired yet are clearly marked "stored, not yet active"
- `AGENT_PLAN.md` — this file
- `IMPLEMENTATION_LOG.md` — Phase 8 entries
- `STATUS.md` — updated status
- `README.md` — Phase 8 features
- `docs/architecture.md` — history/database modules
- `docs/phases.md` — Phase 8 status

## Files That Should Not Be Touched
- `src-tauri/src/audio.rs` — unchanged
- `src-tauri/src/clipboard.rs` — unchanged
- `src-tauri/src/transcription.rs` — unchanged
- `src-tauri/src/transcription_whisper.rs` — unchanged
- `src-tauri/src/modes.rs` — unchanged
- `src-tauri/src/cleanup.rs` — unchanged
- `src-tauri/src/cleanup_basic.rs` — unchanged
- `src-tauri/src/cleanup_mock_ai.rs` — unchanged
- `src-tauri/tauri.conf.json` — no config changes needed
- `src-tauri/capabilities/default.json` — no new permissions needed
- `src/components/AppHeader.tsx` — unchanged
- `src/components/ModeSelector.tsx` — unchanged
- `src/components/TranscriptPreview.tsx` — unchanged
- `src/components/SettingsPanel.tsx` — unchanged
- `src/components/PrivacyNotice.tsx` — unchanged
- `src-tauri/src/main.rs` — unchanged
- `vite.config.ts`, `tsconfig.json`, `package.json`, `index.html` — unchanged

## Dependency Plan
- **Rust**: Add `rusqlite = { version = "0.31", features = ["bundled"] }` for SQLite with bundled libsqlite3
- **npm**: No new dependencies

## Tauri Permission Plan
No new permissions. Phase 8 needs only `app_local_data_dir` which is available via `tauri::Manager::path()` without additional capabilities.
Existing permissions (core:default, global-shortcut:default, clipboard-manager:default) remain unchanged.

## SQLite/Database Plan
- Use `rusqlite` with `bundled` feature (statically links SQLite, no system dependency)
- Database file: `{app_local_data_dir}/spiel_history.db`
- Thread-safe via `Mutex<Connection>` in `Database` struct
- Schema version tracked in `schema_version` table

## Migration Plan
- `initialize_database()` runs on app setup
- Creates tables if not exists (idempotent)
- Schema version 1: `history_entries` table + `schema_version` table
- Safe to run multiple times (IF NOT EXISTS)

## History Schema Plan
Table `history_entries`:
| Column | Type | Notes |
|--------|------|-------|
| id | INTEGER PRIMARY KEY AUTOINCREMENT | |
| created_at | TEXT NOT NULL | ISO 8601 |
| updated_at | TEXT NOT NULL | ISO 8601 |
| title | TEXT NOT NULL DEFAULT '' | Auto-generated from first line |
| raw_text | TEXT NOT NULL | |
| final_text | TEXT NOT NULL DEFAULT '' | |
| mode | TEXT NOT NULL DEFAULT '' | TextModeKind serialized |
| cleanup_provider | TEXT NOT NULL DEFAULT '' | CleanupProviderKind serialized |
| transcription_engine | TEXT NOT NULL DEFAULT '' | Engine name |
| transcription_engine_kind | TEXT NOT NULL DEFAULT '' | EngineKind serialized |
| audio_file_path | TEXT | Nullable |
| audio_duration_ms | INTEGER | Nullable |
| transcript_duration_ms | INTEGER | Nullable |
| cleanup_duration_ms | INTEGER | Nullable |
| is_mock_transcript | INTEGER NOT NULL DEFAULT 0 | Boolean |
| is_mock_cleanup | INTEGER NOT NULL DEFAULT 0 | Boolean |
| error | TEXT | Nullable |

No clipboard data, no API keys, no secrets stored.

## UI State Plan
- History panel: list of recent entries (5 most recent), empty state, entry detail view, delete/clear buttons, enabled/disabled toggle
- CapturePanel: "Save to History" button after cleanup completes
- Manual save only (Option A) — no autosave

## Privacy Plan
- History is local only — no sync, no network
- No encryption (clearly documented)
- History can be disabled via toggle
- Clear history permanently deletes all entries
- Previous clipboard contents never stored
- Audio file contents never stored in database

## Error Handling Plan
- Database open failure → human-readable error, app continues with history disabled
- Migration failure → clear error, app continues
- Save while disabled → clear "history is disabled" error
- Empty entry save → validation error
- Entry not found → clear "not found" error
- Database locked → retry suggestion
- Corrupted database → clear error with recovery suggestion

## Acceptance Criteria
1. SQLite database initializes safely
2. History schema/migration exists and is idempotent
3. User can save a history entry (manual button)
4. User can list recent history entries
5. User can view a history entry's raw/final text and metadata
6. User can delete a single entry
7. User can clear all history (with confirmation)
8. User can disable/enable future history saving
9. UI clearly states history is local only
10. Previous clipboard contents are not stored
11. No network calls, no cloud sync
12. Existing recording/transcription/cleanup/clipboard still works
13. All build checks pass

**Frontend (modified):**
- `src/lib/types.ts` — add TextModeKind, CleanupProviderKind, ModeDefinition (update), CleanupRequest, CleanupResult, CleanupStateData, CleanupStatusType
- `src/lib/api.ts` — add getTextModes, getCleanupProviders, runCleanup, clearFinalText, getCleanupStatus
- `src/App.tsx` — add cleanup state, pass to CapturePanel and ModeSelector/TranscriptPreview
- `src/components/ModeSelector.tsx` — update modes to be implemented, wire to cleanup behavior
- `src/components/CapturePanel.tsx` — add cleanup section: provider selector, run cleanup button, final text display, copy/insert final text, clear final text, error display
- `src/components/TranscriptPreview.tsx` — remove (functionality merged into CapturePanel)
- `src/styles/app.css` — cleanup section styles, final text styles, provider badges

**Documentation:**
- `AGENT_PLAN.md` — this file
- `IMPLEMENTATION_LOG.md` — Phase 7 entries
- `STATUS.md` — updated status
- `README.md` — Phase 7 features
- `docs/architecture.md` — modes and cleanup modules
- `docs/phases.md` — Phase 7 status

## Files That Should Not Be Touched
- `src-tauri/src/audio.rs` — unchanged
- `src-tauri/src/clipboard.rs` — unchanged
- `src-tauri/src/transcription.rs` — unchanged (except maybe stale error message in run_transcription)
- `src-tauri/src/transcription_whisper.rs` — unchanged
- `src-tauri/Cargo.toml` — no new dependencies needed
- `src-tauri/tauri.conf.json` — no config changes
- `src-tauri/capabilities/default.json` — no new permissions
- `src/components/AppHeader.tsx` — unchanged
- `src/components/HistoryPanel.tsx` — unchanged
- `src/components/SettingsPanel.tsx` — unchanged
- `src/components/PrivacyNotice.tsx` — unchanged
- `src-tauri/src/main.rs` — unchanged
- `vite.config.ts`, `tsconfig.json`, `package.json`, `index.html` — unchanged

## Dependency Plan
- **Rust**: No new dependencies
- **npm**: No new dependencies
- All cleanup is deterministic string manipulation in Rust

## Tauri Permission Plan
No new permissions. Phase 7 adds no network, shell, filesystem, or accessibility access.
Existing permissions (core:default, global-shortcut:default, clipboard-manager:default) remain unchanged.

## Mode Definition Plan
Five text modes defined in `src-tauri/src/modes.rs`:
1. **raw_dictation**: Trim whitespace, preserve wording, no rewriting
2. **clean_notes**: Basic punctuation/spacing cleanup, normalize whitespace, split paragraphs
3. **ai_prompt**: Deterministic template wrapper (not AI-generated)
4. **developer_review**: Deterministic template with headings (not AI-generated)
5. **thought_piece**: Deterministic template with structure (not AI-generated)

## Cleanup Provider Plan
Four provider kinds defined in `src-tauri/src/cleanup.rs`:
| Kind | Status | Behavior |
|------|--------|----------|
| basic | implemented | Deterministic text cleanup per mode (trims, normalizes, wraps in templates) |
| mock_ai | implemented | Slightly smarter mock that simulates what future AI would do — clearly labeled as mock |
| openai_planned | planned/unavailable | Returns clear error if selected |
| local_llm_planned | planned/unavailable | Returns clear error if selected |

CleanupProvider trait: `fn cleanup(&self, request: &CleanupRequest) -> Result<CleanupResult, CleanupError>`

## UI State Plan
New cleanup section in CapturePanel:
- Provider selector (basic / mock_ai / openai_planned / local_llm_planned)
- "Run Cleanup" button (disabled if no transcript / empty raw text)
- Final text display (separate from raw transcript)
- Cleanup metadata (mode, provider, is_mock, warnings, duration_ms)
- Copy final text button (uses existing Phase 4 copy_to_clipboard)
- Insert final text button (uses existing Phase 4 insert_via_clipboard)
- Clear final text button
- Error display for cleanup failures

## Clipboard Integration Plan
Final text uses existing Phase 4 clipboard commands:
- `copy_to_clipboard` with final_text
- `insert_via_clipboard` with final_text
No new clipboard behavior. No automatic paste.

## Error Handling Plan
- No transcript available → clear error, disable cleanup button
- Empty raw text → validation error
- Unsupported mode → error returned from backend
- Unavailable provider selected → clear error (openai_planned, local_llm_planned)
- Cleanup already active → state check
- Cleanup result empty → warning
- Backend lock/state errors → human-readable errors
- Clipboard integration unavailable → falls through to existing error handling

## Acceptance Criteria
1. Text mode definitions exist in Rust (5 modes)
2. Cleanup provider abstraction exists (trait-based)
3. Basic deterministic cleanup provider works for all 5 modes
4. Mock AI cleanup provider works (labeled as mock)
5. UI shows mode selector (all 5 modes selectable, marked implemented)
6. UI shows cleanup provider selector
7. UI can run cleanup against current transcript
8. UI shows raw transcript and final text separately
9. UI shows cleanup metadata (mode, provider, is_mock, warnings)
10. No-transcript/empty-text errors handled clearly
11. OpenAI/local LLM providers marked planned/unavailable, not implemented
12. No network calls, no SQLite, no automatic paste
13. Existing transcription still works
14. All build checks pass (cargo check, cargo build, tsc --noEmit, npm run build, cargo fmt --check)

## Verification Plan
1. `cargo fmt --check`
2. `cargo check`
3. `npx tsc --noEmit`
4. `npm run build`
5. `cargo build`
6. Manual review of all modes' output

## Risks
- ModeSelector and TranscriptPreview currently display placeholder/demo content — need careful refactoring to use real backend data
- App.tsx has duplicate `<TranscriptPreview>` and `<HistoryPanel>` components — will clean up
- The existing `run_transcription` function in transcription.rs has a stale error message referencing Phase 5 — fix if encountered

## Verification Plan
1. `cargo check` — Rust type check
2. `cargo build` — Rust compilation
3. `npx tsc --noEmit` — TypeScript check
4. `npm run build` — Vite build
5. `cargo fmt --check` — Rust format

## Risks
- `tauri-plugin-clipboard-manager` version compatibility with Tauri v2.11
- Clipboard restore may fail on some Linux configurations
- Paste simulation deferred (requires accessibility permissions)
