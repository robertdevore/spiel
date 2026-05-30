# Spiel — Phased Build Plan

## Phase 3: Audio Recording ✅ Current

**Goal**: Add real microphone recording, save audio to temporary WAV files.

**Status**: ✅ Complete. CPAL captures from default input device. Channel-based architecture (CPAL streams are not Send). WAV output via hound to system temp directory. Hotkey toggles real recording. UI shows elapsed time and last recording metadata.

**What was implemented**:
- CPAL audio capture from default microphone
- Channel-based threading (CPAL streams must stay on their creation thread)
- WAV output via hound (16-bit PCM)
- start/stop/get_status/clear commands
- Elapsed timer (200ms polling while recording)
- Last recording metadata card (file, duration, size, quality, device)
- Error handling: no mic, permission denied, duplicate start, stop while idle
- Hotkey integration: Ctrl+Shift+Space toggles real recording

**Known limitations**:
- Temp files not auto-cleaned
- No microphone selection UI (uses default device)
- macOS entitlement for production builds not configured

**Goal**: Create a working Tauri v2 desktop app with React + TypeScript frontend and Rust backend command layer.

**Status**: ✅ All acceptance criteria met. Builds pass (TypeScript, Vite, Rust).

---

## Phase 2: Global Hotkey ✅ Current

**Goal**: Register a global hotkey (Ctrl+Shift+Space) that toggles placeholder recording state.

**Status**: ✅ Complete. Default shortcut registered via `tauri-plugin-global-shortcut`. Toggle mode working. UI displays registration status, trigger count, and last-triggered timestamp.

**What was implemented**:
- Global shortcut registration (Rust side, `tauri-plugin-global-shortcut`)
- Tauri event emission on shortcut press (`hotkey-triggered`)
- Frontend listener that toggles `idle` ↔ `recording` placeholder state
- Hotkey status display (registered/error, shortcut label, trigger stats)
- Error handling for registration conflicts
- Phase 2 notice in UI explaining no audio/transcription yet

**Known limitations**:
- Settings not persisted (reset on app restart)
- Hold-to-talk not implemented (release detection unreliable)
- macOS may prompt for Accessibility permission

---

## Phase 3: Audio Recording

**Goal**: Capture audio from the default microphone and save as temporary WAV files.

**Scope**:
- Microphone permission handling
- Audio capture via CPAL or similar
- WAV encoding
- Temp file management
- Recording UI state (live)

**Non-Goals**:
- No transcription yet
- No microphone selection UI yet

**Acceptance Criteria**:
- User can start/stop recording
- Audio saved as valid WAV
- Temp files cleaned up after use
- Recording indicator works in UI

---

## Phase 4: Clipboard Copy/Paste Insertion ✅ Current

**Goal**: Insert text at the user's cursor via clipboard-based paste.

**Status**: ✅ Complete. Clipboard read/write via `tauri-plugin-clipboard-manager`. Editable insertion textarea. Copy and Insert buttons. Manual paste required (Cmd+V / Ctrl+V). Automatic paste deferred (requires enigo + macOS Accessibility).

**What was implemented**:
- `tauri-plugin-clipboard-manager` for clipboard read/write
- 4 clipboard commands: copy, insert via clipboard, restore, read
- Clipboard save/restore (best-effort, plain text only)
- Copy-only fallback
- Insertion text editor in UI

**Known limitations**:
- Automatic paste simulation not implemented (requires accessibility permissions)
- Non-text clipboard contents cannot be saved/restored
- Settings not persisted

---

## Phase 5: Transcription Abstraction

**Goal**: Define a transcription engine trait/interface and a mock engine for testing.

**Scope**:
- `TranscriptionEngine` trait in Rust
- Mock engine returning preset text
- Engine selection logic
- Error handling for transcription failures

**Non-Goals**:
- No real transcription yet
- No cloud engine yet

**Acceptance Criteria**:
- Mock engine returns transcripts
- Engine can be swapped without UI changes
- Error states handled gracefully

---

## Phase 6: First Transcription Engine

**Goal**: Integrate whisper.cpp for local speech-to-text.

**Scope**:
- whisper.cpp binding or CLI integration
- Model download/management
- WAV → text pipeline
- Transcription quality acceptable for dictation

**Non-Goals**:
- No cloud transcription yet
- No streaming transcription yet

**Acceptance Criteria**:
- Real speech → text works
- Latency acceptable (< 5s for 30s audio)
- Works offline
- Model management is user-friendly

---

## Phase 7: Text Modes and Cleanup ✅ Current

**Goal**: Add text mode definitions, cleanup provider abstraction, and basic deterministic cleanup.

**Status**: ✅ Complete. Five text modes defined in Rust (`modes.rs`). Cleanup provider trait with Basic and Mock AI implementations. Frontend selectors wired to real mode/provider behavior. Raw transcript and final text displayed separately. Final text copy/insert via existing Phase 4 clipboard tools.

**What was implemented**:
- `modes.rs`: TextModeKind enum (5 variants), ModeDefinition struct, all_definitions()
- `cleanup.rs`: CleanupProvider trait, CleanupProviderKind enum (4 variants), CleanupRequest, CleanupResult, CleanupError, CleanupState
- `cleanup_basic.rs`: BasicCleanupProvider implementing CleanupProvider trait — deterministic, local-only text transformations for all 5 modes
- `cleanup_mock_ai.rs`: MockAiCleanupProvider implementing CleanupProvider trait — simulated AI output for testing, clearly labeled as mock
- 5 new Tauri commands: get_text_modes, get_cleanup_providers, run_cleanup, clear_final_text, get_cleanup_status
- CleanupState added to AppState
- Frontend: CapturePanel cleanup section with mode/provider selectors, run cleanup button, final text display, copy/insert/clear buttons
- Frontend: ModeSelector updated with real backend data and implemented status
- Frontend: TranscriptPreview updated with cleanup metadata display
- Frontend: CSS styles for cleanup badges, warnings, actions
- Phase strings updated throughout to "Phase 7"
- Total commands: 25

**Known limitations**:
- All cleanup is deterministic/template-based, not AI-powered
- OpenAI and local LLM providers are planned but not implemented
- No automatic end-to-end workflow
- Filler words preserved in all modes
- Developer Review and Thought Piece templates use placeholders where content isn't found

---

## Phase 8: Local History

**Goal**: Store transcripts locally in SQLite.

**Scope**:
- SQLite database for transcripts
- CRUD operations (list, get, delete)
- Search functionality
- History UI (real data)
- Privacy toggle (disable history)

**Non-Goals**:
- No cloud sync
- No export yet

**Acceptance Criteria**:
- Transcripts persist across app restarts
- Search works
- Delete works
- History can be disabled

---

## Phase 9: Settings and Privacy Controls

**Goal**: Make all settings configurable and persistent.

**Scope**:
- Settings persistence (JSON file or SQLite)
- Hotkey configuration
- Transcription engine selection
- Paste behavior settings
- History settings
- Privacy mode toggles
- Clear data functionality

**Non-Goals**:
- No account system
- No cloud sync

**Acceptance Criteria**:
- Settings persist across restarts
- All toggles functional
- Privacy controls enforceable

---

## Phase 10: Workflow MVP ✅ Complete

**Goal**: Connect all existing features into one coherent end-to-end workflow.

**Status**: ✅ Complete. WorkflowState struct with 13-step WorkflowStep enum. 9 workflow Tauri commands. Hotkey integration for start/stop. Migration v3 for workflow settings. Manual review default. Auto-insert off.

**What was implemented**:
- `workflow.rs`: WorkflowStep enum (13 steps: Idle → Recording → ... → SavedToHistory → Error), WorkflowState struct, reset() and set_error() methods
- 4 workflow automation settings (all default false): auto_transcribe_after_recording, auto_cleanup_after_transcription, auto_save_history_after_cleanup, auto_insert_after_cleanup
- 9 workflow commands: start/stop_workflow_recording, run_workflow_transcription, run_workflow_cleanup, insert_workflow_final_text, save_workflow_to_history, cancel_workflow, get_workflow_status, reset_workflow
- Database migration v3 for workflow settings columns
- WorkflowPanel.tsx: guided step-by-step UI with active/complete/error states
- Hotkey integration: Ctrl+Shift+Space updates WorkflowState on start/stop

**Known limitations**:
- Auto-insert defaults to off (safety feature)
- Workflow state is in-memory only (resets on app restart)
- No skip-ahead — all steps must be completed in order

---

## Phase 11: Packaging & Release Readiness ✅ Complete

**Goal**: Prepare for reliable local builds and future distribution.

**Status**: ✅ Complete. Release docs created. Platform notes documented. Security/privacy audit performed. Build commands documented and verified. Permission audit passed.

**What was implemented**:
- `docs/release.md`: Build commands (dev + production), manual QA checklist (25 items), smoke tests, known release blockers, signing/notarization status
- `docs/platforms.md`: macOS (verified areas, permissions, limitations), Windows (expected behavior, unverified areas, config notes), Linux (expected behavior, Wayland/X11 caveats, clipboard notes, config notes)
- `docs/security.md`: Privacy model (what Spiel stores/never stores/never does), dependency security table, command surface audit (44 commands), Local Whisper safety, permissions audit (3 capabilities, all justified), known limitations
- `README.md` updated: Build & Run section, Rust version corrected, Phase 11 header
- `STATUS.md` completely rewritten: Phase status table, 44 commands, permission audit table, platform status table, release status, known blockers
- `capabilities/default.json`: Description updated to Phase 11
- `src-tauri/src/commands.rs`: Phase strings updated to Phase 11
- `docs/architecture.md`: Stale references removed, module plan table updated (all 15 modules implemented)
- `IMPLEMENTATION_LOG.md`: Phase 9/10/11 entries added
- `AGENT_PLAN.md`: M11 milestone marked complete

**Known limitations**:
- macOS code signing and notarization not configured (requires Apple Developer account)
- Windows and Linux builds not verified (configured but untested)
- Placeholder app icons (functional but unbranded)
- No auto-updater (not in scope for Phase 11)
- No CI/CD pipeline configured
