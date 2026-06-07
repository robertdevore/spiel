# Spiel

Spiel is a local-first macOS push-to-talk dictation app built with Tauri + Rust.
It is designed to be quiet, private, and immediately usable:

- Press one global hotkey to start/stop recording.
- Transcribe offline with whisper.cpp (`whisper-rs`).
- Insert text where your cursor is via a trusted accessibility paste flow.
- Fall back to clipboard paste if auto-paste is unavailable.

The codebase is intentionally compact and optimized for reliability in long-running, real-world usage.

## Why Spiel is Universal + Enterprise-Focused

- **Offline first by default**: no transcription traffic leaves your device.
- **Multilingual models** are available directly in the model registry.
- **Predictable settings path** and clear runtime state model reduce support burden.
- **Memory controls** (thread count + keep-model-in-RAM toggles) allow trade-offs by deployment profile.
- **Failure-safe startup and config writes** reduce startup-brick/recovery risks.
- **Deterministic model downloads** with checksum/length checks and `.part` cleanup on failure.
- **Permission-safe UX** for microphone and Accessibility.

## Architecture (2-minute view)

- **Frontend (`src/main.ts`)**: settings/status window and tray-oriented controls.
- **Backend (`src-tauri/src/`)**: recording, model download, transcription, insertion, and state.
- **No network in the transcription hot path**: model download is user-initiated, one-time, cached locally.

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

1. Start the app and open **Settings** from the tray icon.
2. Choose a model (tiny/base/small) and download it.
3. Grant **Microphone** permission when prompted.
4. If using auto-paste, grant **Accessibility** when prompted.
5. Press `Cmd+Alt+D` (or your custom hotkey).

## Core Settings

- **hotkey**
- **model**
  - `tiny.en`, `base.en`, `small.en` (English-only)
  - `tiny`, `base`, `small`, `medium` (multilingual)
- **language**
  - `auto` (detect automatically)
  - locale shorthand (`en`, `es`, `fr`, `de`, `pt`, `ru`, `ja`, `zh`, ...)
- **auto_paste**
- **restore_clipboard**
- **keep_model_loaded**
- **transcription_threads**
- **max_seconds**

`language` is validated against selected model semantics:

- English-only model + non-English hint -> automatically falls back to `en`.
- Multilingual model + invalid/unsupported hint -> falls back to `auto`.

## Performance and Memory Profiles

In the UI **Settings** panel you also get quick profiles:

- **Low Memory**: `tiny.en`, 1 thread, unload model after each dictation
- **Balanced**: `base.en`, 2 threads, unload model after each dictation
- **Quality**: `small` (multilingual), 2 threads, unload model after each dictation
- **Global**: `medium` (multilingual), 2 threads, unload model after each dictation

You can also add/remove `keep_model_loaded` manually per session.

### Useful memory tuning

- Set `keep_model_loaded = false` to free model RAM after each run.
- Reduce `transcription_threads` for lower peak memory/CPU pressure.
- Prefer English-only models for memory-critical machines.

## Performance Observability

When `SPIEL_PROFILE=1`, additional runtime metrics are tracked and shown in the Settings panel.

```bash
SPIEL_PROFILE=1 npm run tauri dev
```

## Runtime Environment Controls

- `SPIEL_WHISPER_THREADS` (`1..16`) overrides `transcription_threads` at runtime.
- `SPIEL_PRE_PASTE_DELAY_MS` (default `60`) tuning delay before Cmd+V.
- `SPIEL_RESTORE_DELAY_MS` (default `220`) tuning delay before clipboard restore.
- `SPIEL_MODEL_DIR` to keep Whisper models in a custom directory.
- `SPIEL_LATENCY_BUDGET_MS` (default `8000`) used by profile statistics.

## Security & Privacy Notes

- No accounts, no telemetry.
- No audio is written to disk.
- Raw audio exists only in memory during recording.
- Model artifacts download into the app data directory and are validated before use.
- Insertion failures and permission state are surfaced explicitly in the UI.

## Build and verification

```bash
npm run build
cd src-tauri
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Command Surface

Backend commands exposed to the UI:

- `get_status`
- `get_config`
- `get_perf_snapshot`
- `clear_perf_samples`
- `update_config`
- `list_models`
- `download_model`
- `delete_model`
- `cancel_download`
- `toggle_dictation`
- `unload_model_from_memory`
- `accessibility_status`
- `request_accessibility`
- `show_settings`

## Troubleshooting

- **Hotkey does nothing**
  - Another app may already use the shortcut; change it in Settings.
- **Text does not auto-paste**
  - Accessibility must be trusted. Grant permission and retry.
- **Model download stalls/aborts**
  - Check network connectivity and storage space.
- **Unexpected model load failures**
  - Remove the failed `.part` file and retry download from Settings.

## Release Notes / Reviews

- `docs/REVIEW-2026-05-30-opus48.md` contains the historical review.
- `BUG_HUNT_REPORT.md` contains prior bug-fix evidence and prior check-ins.
- `docs/RELEASE_READINESS_NEXT_SESSION.md` tracks the current deep-dive action list and remaining work.
