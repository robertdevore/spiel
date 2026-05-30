# Spiel — Architecture

## High-Level App Loop

```
User speaks → Spiel captures audio → Audio saved to temp WAV
  → Transcription engine processes → Raw transcript produced
  → Cleanup mode applied (deterministic, local) → Final text ready
  → Text inserted at cursor position (clipboard or accessibility API)
```

**Phase 7** implements the full record→transcribe→cleanup→insert loop. Cleanup is deterministic and local. AI-powered cleanup (OpenAI, local LLM) is planned for future phases.

## Frontend Responsibilities (React + TypeScript)

- Render the main UI: capture panel, mode selector, transcript preview, history, settings
- Manage local UI state (flow states: idle → recording → processing → complete → error)
- Invoke Tauri commands via `@tauri-apps/api/core`
- Display capability status from the backend
- Coordinate recording lifecycle, display real-time transcript and cleanup results

## Rust Backend Responsibilities (Tauri v2)

- Serve as the trusted system boundary between the UI and OS
- Expose narrow, explicit Tauri commands for each operation
- In future phases:
  - Manage global hotkey registration
  - Capture audio from the default microphone
  - Invoke transcription engines (local whisper.cpp, optional cloud)
  - Perform text mode cleanup
  - Manage clipboard read/write for paste insertion
  - Store transcripts in SQLite for local history
  - Enforce privacy controls

## Module Plan (All Implemented)

| Module | Purpose | Phase |
|--------|---------|-------|
| `hotkey.rs` | Global hotkey registration | ✅ Phase 2 (implemented in lib.rs setup) |
| `audio.rs` | Microphone capture, WAV encoding | ✅ Phase 3 (channel-based CPAL architecture) |
| `clipboard.rs` | Clipboard read/write/save/restore | ✅ Phase 4 (manual paste, auto-paste deferred) |
| `transcription.rs` | Transcription engine abstraction | ✅ Phase 5 |
| `transcription_whisper.rs` | Local Whisper engine | ✅ Phase 6 |
| `modes.rs` | Text processing mode definitions | ✅ Phase 7 |
| `cleanup.rs` | Cleanup provider abstraction | ✅ Phase 7 |
| `cleanup_basic.rs` | Basic deterministic cleanup provider | ✅ Phase 7 |
| `cleanup_mock_ai.rs` | Mock AI cleanup provider | ✅ Phase 7 |
| `database.rs` | SQLite connection, migrations, schema versioning | ✅ Phase 8 |
| `history.rs` | SQLite transcript storage, CRUD operations | ✅ Phase 8 |
| `settings.rs` | Persistent user settings in SQLite | ✅ Phase 9 |
| `workflow.rs` | End-to-end workflow state machine | ✅ Phase 10 |
| `commands.rs` | All 44 Tauri command handlers | ✅ Phase 1–10 |
| `app_state.rs` | Application state with 8 Mutex fields | ✅ Phase 1–10 |

## Security Boundaries

Tauri acts as the security boundary between:

1. **The webview frontend** (sandboxed, no direct system access)
2. **The Rust backend** (controlled system access via explicit commands)
3. **The operating system** (microphone, clipboard, global hotkeys, filesystem)

### Permission Model

Each system capability requires explicit Tauri permissions declared in `capabilities/default.json`.

Current permissions (3 total, all justified):

- `core:default` — Phase 1: Tauri runtime
- `global-shortcut:default` — Phase 2: Ctrl+Shift+Space hotkey
- `clipboard-manager:default` — Phase 4: Text copy/paste for insertion

No network, shell, filesystem, updater, or accessibility permissions.

## Why Tauri

- **Native performance**: Rust backend, minimal overhead
- **Small binary size**: ~5-10MB vs Electron's ~100MB+
- **Security-first**: Webview sandboxed, explicit permission model
- **Cross-platform**: macOS, Windows, Linux from one codebase
- **No Chromium dependency**: Uses the OS native webview

## Why Clipboard Paste for MVP Text Insertion

For MVP text insertion, Spiel will use clipboard-based paste:

1. Save current clipboard content
2. Write cleaned transcript to clipboard
3. Simulate Cmd+V (or use accessibility API)
4. Restore original clipboard content

This avoids requiring accessibility permissions initially, though those may be added later for a smoother experience.

## Why Local-First

- **Privacy**: Transcripts never leave the device by default
- **Speed**: No network latency for transcription (local whisper.cpp)
- **Reliability**: Works offline
- **User trust**: Clear data boundaries — nothing is uploaded unless explicitly configured
- **GDPR/Compliance**: No data processing agreements needed for local-only operation
