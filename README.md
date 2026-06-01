# Spiel

Speak, and it lands where your cursor is.

Spiel is a local-first push-to-talk dictation app for macOS. Press a global hotkey, speak, press again, and Spiel transcribes on-device with Whisper and pastes into the focused app.

## What It Does

- Menu-bar app (no Dock icon).
- One dictation path for hotkey, tray action, and UI button.
- In-memory audio only (no temp audio files).
- On-device transcription (`whisper-rs` / whisper.cpp).
- Cursor insertion via clipboard + synthesized Cmd+V.
- Fallback behavior when Accessibility is missing: transcript stays on clipboard.

## Privacy Model

- No accounts, no telemetry, no cloud sync.
- No network traffic during dictation/transcription.
- One network path exists: model download you explicitly trigger.
- Persisted local files:
  - `config.json` in app config directory
  - Whisper model files in app data directory

## Requirements

- macOS 12+
- Node.js 18+
- Rust 1.80+
- CMake (`brew install cmake`)
- Xcode Command Line Tools

## Quick Start

```bash
npm install
npm run tauri dev
```

## First Run

1. Open **Settings** from the tray icon.
2. Download a model (`base.en` recommended).
3. Grant **Microphone** permission when prompted.
4. Grant **Accessibility** for auto-paste when prompted.
5. Use `Cmd+Alt+D` to start/stop dictation.

## Build And Verification

```bash
npm run build
cd src-tauri
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Settings

- `hotkey`
- `model` (`tiny.en`, `base.en`, `small.en`)
- `language` (`en`, `auto`)
- `auto_paste`
- `restore_clipboard`
- `max_seconds`

## Performance And Profiling

Spiel includes low-latency defaults plus runtime tuning:

- `SPIEL_WHISPER_THREADS` (`1..16`)
- `SPIEL_PRE_PASTE_DELAY_MS` (default `60`)
- `SPIEL_RESTORE_DELAY_MS` (default `220`)
- `SPIEL_LATENCY_BUDGET_MS` (default `8000`)

Enable profiling mode:

```bash
SPIEL_PROFILE=1 npm run tauri dev
```

When enabled, the settings UI shows a **Performance Profile** card with rolling latency stats:

- sample count
- average / p95 / max total latency
- over-budget count
- most recent stage breakdown (capture, transcribe, insert, total)

## Troubleshooting

- Hotkey does nothing:
  - another app may own the shortcut; change `hotkey` in Settings.
- Text does not auto-paste:
  - grant Accessibility; transcript is still on clipboard.
- Build fails with missing C++ headers:
  - reinstall Command Line Tools (`xcode-select --install`).

## Repository Notes

This branch is the v2 rebuild (`rebuild/spiel-v2`) after a review of the earlier implementation. Historical review doc: `docs/REVIEW-2026-05-30-opus48.md`.
