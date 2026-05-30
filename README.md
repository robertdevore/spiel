# Spiel

> **Get the thought out. Spiel puts it where your cursor is.**

Spiel is a lightweight desktop utility that helps you get thoughts out of your head and into text wherever your cursor is.

The core product loop: **hotkey → talk → transcript → cleaned text → inserted at cursor**

---

## Current Phase

**Phase 11 — Packaging, platform hardening, and release readiness**

Phase 11 prepares Spiel for reliable local builds and future distribution. Build documentation, platform notes, security audit, and QA checklist.

## What Currently Works

- ✅ Tauri v2 project compiles and builds cleanly (Rust + TypeScript + Vite)
- ✅ React + TypeScript frontend renders in browser dev mode
- ✅ **Global hotkey: Cmd+Shift+S** toggles real microphone recording
- ✅ **Real microphone recording** via CPAL — saves WAV files to temp directory
- ✅ **Clipboard copy and insert** — editable text, copy/paste with clipboard save/restore
- ✅ Hotkey registration status and trigger counter displayed in UI
- ✅ Recording elapsed timer and last recording metadata (file, duration, size, quality, device)
- ✅ **Text mode definitions**: 5 modes — Raw Dictation, Clean Notes, AI Prompt, Developer Review, Thought Piece
- ✅ **Cleanup pipeline**: Basic (deterministic, local) and Mock AI (testing) providers
- ✅ **Final text separation**: Raw transcript vs final text displayed separately
- ✅ **Copy/insert final text**: Manual copy and insert via existing Phase 4 clipboard tools
- ✅ **Local history**: SQLite-based persistence — save, view, delete, and clear past sessions
- ✅ **History privacy**: Local only, no sync, no accounts, clipboard never stored
- ✅ **Persisted settings**: All settings survive app restarts (SQLite, local only)
- ✅ **Privacy controls**: Local-only mode, history toggle, clipboard restore toggle
- ✅ **End-to-end workflow**: Guided flow from record → transcribe → cleanup → insert → save
- ✅ **Workflow safety**: Auto-insert off, never presses Enter, manual review default
- ✅ Rust backend with 44 commands
- ✅ Capability status display (8 implemented: ui_foundation, global_hotkey, audio_recording, transcription, clipboard_paste, local_history, text_modes, settings_persistence)
- ✅ Dark-themed, minimal, desktop-native UI
- ✅ No network calls — all processing is local

## What Does Not Work Yet

- ❌ Cloud sync (history is local only)
- ❌ Settings encryption (stored as plain SQLite settings row)
- ❌ History encryption (stored as plain SQLite)
- ❌ AI-powered cleanup (OpenAI, local LLM) — planned for future phases
- ❌ Hold-to-talk mode (release detection unreliable via current plugin)
- ❌ Automatic paste after cleanup (auto-insert defaults to off)

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop Framework | Tauri v2 |
| Backend | Rust |
| Frontend | React 18 + TypeScript |
| Build Tool | Vite 6 |
| Styling | Plain CSS (dark theme) |
| Hotkeys | Tauri global shortcut plugin (Cmd+Shift+S) |
| Audio | CPAL + hound (WAV output to temp dir) |
| Clipboard | Tauri clipboard manager (text read/write, manual paste) |
| Transcription | Mock engine + Local Whisper (whisper.cpp) + trait-based abstraction |
| History | SQLite (rusqlite, bundled) — local-only, no sync, no encryption |
| Future: Storage | SQLite (via rusqlite) |

## Development Setup

### Prerequisites

- **Rust** (1.86+) — [rustup.rs](https://rustup.rs)
- **Node.js** (18+) — [nodejs.org](https://nodejs.org)
- **macOS**: Xcode Command Line Tools (`xcode-select --install`)
- **Windows**: Microsoft Visual Studio C++ Build Tools
- **Linux**: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, and other Tauri dependencies

### Install Dependencies

```bash
cd spiel
npm install
```

### Run in Development Mode

```bash
npm run tauri dev
```

This starts the Vite dev server and launches the Tauri window.

### Build for Production

```bash
npm run tauri build
```

### Frontend-Only Development (Browser)

You can develop the UI in a browser without Tauri:

```bash
npm run dev
```

Open http://localhost:1420 — note that Tauri commands will not work in the browser. The UI will display a notice that the backend is unavailable.

### TypeScript Check

```bash
npx tsc --noEmit
```

### Rust Check

```bash
cd src-tauri && cargo check
```

## Project Structure

```
spiel/
├── src/                          # React frontend
│   ├── main.tsx                  # Entry point
│   ├── App.tsx                   # Root component
│   ├── vite-env.d.ts             # Vite type declarations
│   ├── components/
│   │   ├── AppHeader.tsx         # App name + tagline
│   │   ├── CapturePanel.tsx      # Status, record button, flow simulator
│   │   ├── ModeSelector.tsx      # Text mode selection (planned)
│   │   ├── TranscriptPreview.tsx # Raw + cleaned transcript display
│   │   ├── HistoryPanel.tsx      # Past transcripts (placeholder)
│   │   ├── SettingsPanel.tsx     # Settings display (planned)
│   │   └── PrivacyNotice.tsx     # Privacy posture notice
│   ├── lib/
│   │   ├── types.ts              # TypeScript type definitions
│   │   └── api.ts                # Tauri invoke wrappers
│   └── styles/
│       └── app.css               # Application styles
├── src-tauri/                    # Rust backend
│   ├── Cargo.toml                # Rust dependencies
│   ├── build.rs                  # Tauri build script
│   ├── tauri.conf.json           # Tauri configuration
│   ├── capabilities/
│   │   └── default.json          # Tauri permissions
│   ├── icons/                    # App icons
│   └── src/
│       ├── main.rs               # Rust entry point
│       ├── lib.rs                # Tauri builder setup
│       ├── commands.rs           # Tauri command handlers
│       └── app_state.rs          # Application state
├── docs/
│   ├── architecture.md           # Architecture overview
│   └── phases.md                 # Phased build plan
├── index.html                    # HTML entry point
├── package.json                  # Node dependencies
├── tsconfig.json                 # TypeScript config
├── tsconfig.node.json            # TypeScript config for Vite
├── vite.config.ts                # Vite configuration
├── AGENT_PLAN.md                 # Agent execution plan
├── IMPLEMENTATION_LOG.md         # Implementation log
├── STATUS.md                     # Project status
└── README.md                     # This file
```

## Privacy Model

Spiel is designed to be **local-first and privacy-respecting**:

- **Phase 1**: No data collection of any kind. No audio, no keystrokes, no clipboard access, no network requests.
- **Future phases**: All recording, transcription, and text processing will happen locally by default. Cloud services (if added) will require explicit user opt-in.
- **No telemetry**: Spiel will never include hidden analytics or tracking.
- **Transparent permissions**: Every system permission (microphone, clipboard, accessibility) will be clearly explained when requested.

## Roadmap

| Phase | Feature | Status |
|-------|---------|--------|
| 1 | Desktop Foundation | ✅ Complete |
| 2 | Global Hotkey | ✅ Complete |
| 3 | Audio Recording | ✅ Complete |
| 4 | Clipboard Insertion | ✅ Complete |
| 5 | Transcription Abstraction | ✅ Current |
| 3 | Audio Recording | Planned |
| 4 | Clipboard Paste Insertion | Planned |
| 5 | Transcription Abstraction | Planned |
| 6 | Local Transcription (whisper.cpp) | Planned |
| 7 | Text Modes & Cleanup | Planned |
| 8 | Local History (SQLite) | Planned |
| 9 | Settings & Privacy Controls | Planned |
| 10 | Polish, Packaging & Platform Hardening | Planned |

## Manual Testing Checklist (Phase 1)

- [ ] App launches without errors (`npm run tauri dev`)
- [ ] Hotkey registration status shows green dot (registered)
- [ ] Pressing Cmd+Shift+S toggles state between Idle and Recording
- [ ] Trigger count increments on each press
- [ ] Hotkey works when another app is focused
- [ ] UI renders all sections (header, capture, modes, transcript, history, settings, privacy)
- [ ] Backend status dot shows green (when running in Tauri)
- [ ] Flow state simulator changes states correctly
- [ ] Mode selector shows all 5 modes (all planned)
- [ ] Transcript preview shows demo content
- [ ] History shows placeholder entries
- [ ] Settings shows all groups with planned badges
- [ ] Privacy notice is visible
- [ ] Capability grid shows implemented vs planned
- [ ] Window is resizable
- [ ] Dark theme is applied

---

## Build & Run

### Prerequisites
- Rust 1.86+ (`rustup update stable`)
- Node.js 18+ and npm 9+
- macOS 12+ (primary), Windows/Linux (configured, not fully tested)

### Quick Start
```bash
npm install
npm run tauri dev
```

### Build Commands
```bash
npm run build          # Frontend production build
cd src-tauri && cargo build   # Rust debug build
npm run tauri build    # Full Tauri production build (outputs .app/.dmg/.msi)
```

### Checks
```bash
npx tsc --noEmit           # TypeScript type check
cd src-tauri && cargo check     # Rust type check
cd src-tauri && cargo fmt --check  # Rust formatting
```

### Platform Notes
See [docs/platforms.md](docs/platforms.md) for macOS/Windows/Linux details.
See [docs/release.md](docs/release.md) for release status and QA checklist.
See [docs/security.md](docs/security.md) for security and privacy audit.

---

**Built with Tauri v2, React, TypeScript, and Rust.**
