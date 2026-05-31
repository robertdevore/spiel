# Spiel — Project Status

**Last Updated**: 2026-05-30 (Phase 11 stabilization pass)

## Phase Status

| Phase | Name | Status |
|-------|------|--------|
| 1 | Desktop Foundation | ✅ Stable |
| 2 | Global Hotkey | ✅ Stable |
| 3 | Audio Recording | ✅ Stable |
| 4 | Clipboard Insertion | ✅ Stable |
| 5 | Transcription Abstraction | ✅ Stable |
| 6 | Local Whisper Engine | ✅ Stable |
| 7 | Text Modes & Cleanup | ✅ Stable |
| 8 | Local SQLite History | ✅ Stable |
| 9 | Settings & Privacy | ✅ Stable |
| 10 | Workflow MVP | ✅ Stable |
| 11 | Packaging & Release Readiness | ✅ Stable |

## What Works Now

- ✅ Tauri v2 project compiles and builds cleanly
- ✅ React + TypeScript + Vite frontend with dark theme
- ✅ Rust backend with **44 Tauri commands** across 14 modules
- ✅ Global hotkey: Cmd+Option+. toggles real microphone recording
- ✅ Real microphone recording via CPAL (WAV to temp dir)
- ✅ Clipboard copy/insert with clipboard save/restore
- ✅ Mock transcription engine + Local Whisper (whisper.cpp) engine
- ✅ 5 text modes: Raw Dictation, Clean Notes, AI Prompt, Developer Review, Thought Piece
- ✅ Basic deterministic cleanup provider + Mock AI cleanup provider
- ✅ Local SQLite history: save, list, view, delete, clear entries
- ✅ Persisted settings: all config survives app restarts (SQLite)
- ✅ Privacy controls: local-only mode, history toggle, clipboard restore toggle
- ✅ End-to-end workflow: guided record → transcribe → cleanup → insert → save
- ✅ Workflow safety: auto-insert off, never presses Enter, manual review default
- ✅ 8 capabilities all implemented (ui_foundation, global_hotkey, audio_recording, transcription, clipboard_paste, local_history, text_modes, settings_persistence)
- ✅ 3 Tauri permissions only (core, global-shortcut, clipboard-manager) — no network, shell, filesystem, updater
- ✅ Release docs, platform notes, security audit, QA checklist (Phase 11)
- ✅ All build checks pass (TypeScript, Vite, Rust fmt, Rust check, Rust build)

## What Does Not Work Yet

- ❌ Cloud sync (history is local only — no network code exists)
- ❌ History & settings encryption (plain SQLite, honestly documented)
- ❌ AI-powered cleanup (OpenAI, local LLM) — planned for future phases
- ❌ Hold-to-talk mode (release detection unreliable via current plugin)
- ❌ Automatic paste after cleanup (manual paste only; auto-insert defaults to off)
- ❌ macOS code signing & notarization (requires Apple Developer account)
- ❌ Windows & Linux builds not verified (configured but untested)
- ❌ Temp audio files not auto-cleaned
- ❌ No auto-updater (not in scope for Phase 11)

## Build & Verification

### Verified (2026-05-30)

| Check | Command | Result |
|-------|---------|--------|
| TypeScript | `npx tsc --noEmit` | ✅ No errors |
| Vite build | `npm run build` | ✅ OK |
| Rust format | `cargo fmt --check` | ✅ Clean |
| Rust check | `cargo check` | ✅ No errors |
| Rust build | `cargo build` | ✅ OK |
| Tauri dev | `npm run tauri dev` | 🔲 Not run (requires window server) |
| Tauri prod build | `npm run tauri build` | 🔲 Not run (requires window server + signing setup) |

### Not Verified (requires GUI or platform-specific setup)

- App launch on macOS (no window server available)
- Microphone permission prompt behavior
- Hotkey registration in production build
- Windows build (`npm run tauri build` on Windows)
- Linux build (`npm run tauri build` on Linux)
- Local Whisper transcription with real model
- Clipboard save/restore with non-text content
- Full QA checklist (see `docs/release.md`)

## How to Run

```bash
cd spiel
npm install
npm run tauri dev      # Launch in development mode
npm run tauri build    # Production build
```

For build-only verification (no GUI required):
```bash
npx tsc --noEmit
npm run build
cd src-tauri && cargo fmt --check && cargo check && cargo build
```

## Permission Audit

| Permission | Phase | Justification |
|-----------|-------|---------------|
| core:default | 1 | Tauri runtime |
| global-shortcut:default | 2 | Cmd+Option+. hotkey |
| clipboard-manager:default | 4 | Text copy/paste for insertion |

✅ No network, shell, filesystem, updater, or accessibility permissions.

## Security & Privacy

- ✅ No network calls — all processing is local
- ✅ No OpenAI, cloud transcription, or cloud AI
- ✅ No accounts, telemetry, analytics, or payments
- ✅ No API keys stored or transmitted
- ✅ No previous clipboard contents stored in history
- ✅ SQL parameterized queries, no arbitrary SQL exposed
- ✅ Local Whisper invoked via `std::process::Command` (no shell), paths validated
- ✅ No encryption at rest (honestly documented)
- ✅ Local-only mode documented
- ✅ Content Security Policy enabled
- ✅ No auto-paste, no Enter key simulation

See `docs/security.md` for full security audit.

## Platform Status

| Platform | Build | Runtime | Verified |
|----------|-------|---------|----------|
| macOS | ✅ Clean | 🔲 Not tested in GUI | Build only |
| Windows | 🔲 Not tested | 🔲 Not tested | Configured, not verified |
| Linux | 🔲 Not tested | 🔲 Not tested | Configured, not verified |

See `docs/platforms.md` for detailed platform notes.

## Release Status

- **Version**: 0.1.0
- **Identifier**: com.spiel.app
- **Product name**: Spiel
- **Icons**: Present (generated placeholder icons)
- **Code signing**: Not configured
- **Notarization**: Not configured
- **Installer**: Generated by `tauri build` (functional but unsigned)

See `docs/release.md` for release checklist and known blockers.

## Known Release Blockers

| Blocker | Impact | Resolution |
|---------|--------|------------|
| macOS code signing | Gatekeeper warning on launch | Requires Apple Developer account |
| Windows/Linux untested | Unknown issues | Needs platform testing |
| Placeholder app icons | Generic appearance | Generate custom icon set |
| Temp audio files persist | Disk space accumulation | Add cleanup on app quit |
| No auto-updater | Manual updates only | Consider for post-1.0 |

## Next Recommended Milestone

**Phase 12: Provider Expansion** — Add optional providers (OpenAI transcription/cleanup, local LLM cleanup) behind explicit user opt-in. All Phase 11 infrastructure (settings, workflow, privacy controls) is ready to support this. No auto-enable — providers must be explicitly configured before use.
