# Spiel

Spiel is a local-first macOS dictation stack built with **Tauri + Rust** and backed by Whisper (`whisper-rs`) for offline transcription.

It is designed to feel “quiet” in operation:

- no background servers
- no telemetry
- no model traffic after setup
- no raw audio persisted to disk

## What’s in the repo now

- `src/main.ts` — settings/status UI + model list/actions.
- `src/styles.css` — minimal styling.
- `src-tauri/src/` — Rust backend runtime: capture, transcription, downloads, insertion, and IPC.
- `dist/` — generated frontend bundle from the latest build (not required to keep in source control).
- `docs/` — review notes and release-readiness tracking.
- `BUG_HUNT_REPORT.md` and `STATUS.md` — prior findings and execution notes.

This keeps source-of-truth in `src/` and `src-tauri/src/`, with only build/reference artifacts at root.

## Core Experience

- Global hotkey toggle (`Cmd+Alt+D` by default).
- Menu bar app with tray status and settings.
- Start recording → transcribe → insert path in one worker thread.
- Model-driven fallback messages for permission and startup issues.
- Model list with install state (`installed`, `partial`, `corrupt`, `missing`, `unsafe_path`).

## Universal/Enterprise Characteristics

- **Offline-first**: transcription runs locally.
- **Permission-safe**: microphone + Accessibility prompts are explicit and visible.
- **Config hygiene**: configuration is normalized, validated, and safely saved through atomic temp-file writes.
- **Model integrity checks**:
  - GGML magic header
  - size sanity checks
  - optional SHA pin (when provided)
  - safe path checks that reject symlink targets
- **Operational controls** for resource policy:
  - `keep_model_loaded` toggle
  - thread count clamp
  - recording duration clamp
  - install cache + path checks
  - install health visibility (`installed`, `partial`, `corrupt`, `missing`, `unsafe_path`) with reason and age metadata

## Supported Models

Registry entries include English and multilingual families for pragmatic tradeoffs:

- `tiny.en`, `base.en`, `small.en` (English-only)
- `tiny`, `base`, `small`, `medium` (multilingual)

Language hints are validated against the selected model family so unsupported combinations automatically degrade to safe defaults.

## Quick Start

```bash
npm install
npm run tauri dev
```

## Configuration Surface

- `hotkey`
- `model`
- `language`
- `auto_paste`
- `restore_clipboard`
- `keep_model_loaded`
- `transcription_threads`
- `max_seconds`

### Language handling

- `auto` is always accepted.
- BCP-47/legacy regional tags like `en-US` normalize to `en`.
- Invalid values fall back to `auto` or model-safe defaults.

## Backend Commands Exposed to the UI

- `get_status`
- `get_config`
- `get_perf_snapshot`
- `clear_perf_samples`
- `update_config`
- `list_models`
- `get_readiness`
- `download_model`
- `delete_model`
- `cancel_download`
- `toggle_dictation`
- `unload_model_from_memory`
- `accessibility_status`
- `request_accessibility`
- `show_settings`

## Runtime Environment Knobs

- `SPIEL_WHISPER_THREADS` (`1..16`) to override model thread count.
- `SPIEL_PRE_PASTE_DELAY_MS` and `SPIEL_RESTORE_DELAY_MS` for paste timing.
- `SPIEL_MODEL_DIR` for custom local model storage.
- `SPIEL_ACCESSIBILITY_POLL_MS` to tune permission polling (default 1000ms, range 250–30000).
- `SPIEL_PART_CLEANUP_MS` for stale `.part` file cleanup during startup/download (milliseconds, 0 keeps all, defaults to 24h).
- `SPIEL_PROFILE` and `SPIEL_LATENCY_BUDGET_MS` for profiling behavior.
- `SPIEL_DOWNLOAD_CONNECT_TIMEOUT_MS` and `SPIEL_DOWNLOAD_TIMEOUT_MS` for download robustness.
- `SPIEL_DOWNLOAD_RETRIES` (`0..8`) for transient download retries (default `2` retries).
- `SPIEL_DOWNLOAD_RETRY_BACKOFF_MS` (`100..30000`, default `250`) for initial retry delay before each attempt.

### Integrity behavior

- Registry SHA-256 pins are honored when set.
- Optional sidecar checksums (`<model>.sha256`) are auto-used for local files when present.

## Non-macOS behavior

- `playback/copy` auto-paste is macOS-only; non-macOS builds still provide clipboard fallback and clearly surface when explicit permission-based paste is unavailable.
- Accessibility trust prompts and status are no-ops on non-macOS platforms.

## Architecture

- `src/main.ts` and `src/styles.css`: front-end panel and UX state orchestration.
- `src-tauri/src/`: backend engine for capture, transcription, insertion, settings, model downloads, and status.
- `src-tauri/src/commands.rs`: IPC command boundary used by the UI.

## Performance and Memory

- Capture buffer is bounded by target sample rate (`16 kHz`) and recording window.
- In-callback downmix + downsample reduces worker copy volume.
- Optional “keep model in RAM” controls let you trade startup latency for memory footprint.
- Dictation duration is clamped to sane values (`5..600` seconds).
- Readiness diagnostics report current model-store footprint to support disk/memory planning.

## Security & Privacy

- No audio files are written by normal operation.
- Only model files are written during explicit model download.
- Clipboard insertion never assumes Accessibility trust; it falls back to manual paste if needed.
- Model paths reject symlinks and parent directory traversal.
- Download and settings writes are bounded with validation and cleanup on error.

## Verification

```bash
npm run build
cd src-tauri
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Troubleshooting

- **I can’t start with hotkey**: likely key conflict; set another sequence in settings.
- **Model install says partial/corrupt**: delete and repair from model list, then re-download.
- **Auto-paste does nothing**: Accessibility must be trusted.
- **Transcription is slow**: switch to multilingual `small`/`base` trade-off, or lower threads/disable `keep_model_loaded` for memory.
- **Install is repeatedly failing with checksum errors**: remove corrupted model + sidecar and retry. Sidecar validation is intentionally strict to prevent silent corruption.

## Roadmap

The next engineering pass is tracked in `docs/RELEASE_READINESS_NEXT_SESSION.md`.
