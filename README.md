# Spiel

> **Speak, and it lands where your cursor is.**

Spiel is a local-first push-to-talk dictation utility for macOS. Press a global hotkey,
talk, press it again — Spiel transcribes your speech **on-device** with Whisper and pastes
the text into whatever app you're using. No accounts, no telemetry, and (after a one-time
model download) no network.

This is the **v2 rebuild**. It was written from scratch after a senior review of the
original DeepSeek build found that its core loop was faked (see
[`docs/REVIEW-2026-05-30-opus48.md`](docs/REVIEW-2026-05-30-opus48.md)). The original is
preserved on the `main` branch; this branch (`rebuild/spiel-v2`) shares no code with it.

---

## How it works

```
hotkey ─▶ record (mic) ─▶ transcribe (whisper.cpp, on-device) ─▶ paste at cursor (Cmd+V)
```

- **Menu-bar app.** No Dock icon. Lives in the status bar; a small settings window opens
  from the tray.
- **One code path.** The hotkey, the tray "Start / Stop", and the UI button all call the
  same `dictation::toggle` — behavior can't drift between them.
- **In-memory audio.** Microphone samples are downmixed to mono, resampled to 16 kHz, and
  fed straight to Whisper. **No audio file is ever written to disk.**
- **Real insertion.** Text is placed on the clipboard and pasted with a synthesized Cmd+V.
  The previous clipboard is restored afterward. If Accessibility permission is missing, the
  text stays on the clipboard and Spiel tells you how to grant it — your words are never
  lost.

## What's real (and what isn't faked)

| Capability | Status |
| ---------- | ------ |
| Microphone capture | Real — CPAL, all sample formats, bounded by a max-seconds cap |
| Transcription | Real — embedded `whisper.cpp` via `whisper-rs`, fully offline |
| Model delivery | First-run download (checksum/structure-validated), then offline |
| Text insertion at cursor | Real — clipboard + synthesized Cmd+V, with clipboard restore |
| macOS permissions | Microphone usage string + Accessibility flow are wired |
| Networking | **Only** the one-time model download. Nothing else leaves the device |
| Telemetry / accounts | None |

There is no mock engine in the product path. If a model isn't installed, Spiel refuses to
"record into the void" and points you to the download instead of returning fake text.

## Privacy

Spiel records only while you're dictating. Audio is transcribed locally and never written
to disk. The only files Spiel creates are the Whisper model (in the app data directory) and
`config.json` (in the app config directory). No audio or transcript ever leaves your Mac.
The single network request in the entire app is the model download you trigger yourself.

## Requirements

- macOS 12+
- Rust 1.80+ and Node.js 18+
- **CMake** (`brew install cmake`) — `whisper-rs` compiles `whisper.cpp` from source
- Xcode Command Line Tools

## Build & run

```bash
npm install
npm run tauri dev      # run the app
npm run tauri build    # produce a .app / .dmg
```

### Verification

```bash
npm run build                                              # tsc (strict) + vite
cd src-tauri
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

All of the above are green.

### First run

1. Launch Spiel — it appears in the menu bar.
2. Open **Settings…** from the tray and **download a model** (Base · English recommended).
3. macOS will ask for **Microphone** access the first time you record. Grant it.
4. For auto-paste, grant **Accessibility** when prompted (Settings → Privacy & Security →
   Accessibility). Until then, transcripts go to the clipboard for a manual Cmd+V.
5. Press **Cmd+Alt+D** anywhere to start/stop dictation.

## Settings

`hotkey`, `model`, `language` (en/auto), `auto_paste`, `restore_clipboard`,
`max_seconds`. Stored as readable JSON in the app config directory.

## Performance Tuning

Spiel is optimized for low stop-to-text latency, but heavily loaded systems can still
benefit from tuning:

- `SPIEL_WHISPER_THREADS` — override transcription thread count (`1..16`).
- `SPIEL_PRE_PASTE_DELAY_MS` — clipboard settle delay before Cmd+V (default `60`).
- `SPIEL_RESTORE_DELAY_MS` — delay before restoring previous clipboard (default `220`).
- `SPIEL_LATENCY_BUDGET_MS` — budget threshold for profiling warnings (default `8000`).
- `SPIEL_PROFILE=1` — enable built-in per-stage profiling.

### Profiling Mode

With `SPIEL_PROFILE=1`, Spiel records rolling timing samples for:

- capture finalize latency
- transcription latency
- insertion latency
- total stop-to-result latency

The settings window shows a Performance Profile card with:

- sample count
- average / p95 / max total latency
- count over latency budget
- last-sample stage breakdown

## Architecture

| File | Responsibility |
| ---- | -------------- |
| `src-tauri/src/lib.rs` | Tauri builder, menu-bar tray, hotkey registration, window-hide |
| `src-tauri/src/dictation.rs` | The single record→transcribe→insert orchestrator |
| `src-tauri/src/audio.rs` | CPAL capture, downmix, resample to 16 kHz (in memory) |
| `src-tauri/src/whisper.rs` | Embedded whisper.cpp transcription + output cleanup |
| `src-tauri/src/model.rs` | Model registry + validated first-run downloader |
| `src-tauri/src/insert.rs` | Clipboard write + Cmd+V synthesis + clipboard restore |
| `src-tauri/src/accessibility.rs` | macOS Accessibility (TCC) checks and prompts |
| `src-tauri/src/config.rs` / `state.rs` | Settings persistence and shared state |
| `src-tauri/src/commands.rs` | The complete (narrow, typed) command surface |
| `src/main.ts` | Settings/status window |

## Troubleshooting

- **`fatal error: 'atomic' file not found` while building** — your Command Line Tools are
  missing toolchain C++ headers. Reinstall them (`sudo rm -rf
  /Library/Developer/CommandLineTools && xcode-select --install`), or build with
  `CXXFLAGS="-isystem $(xcrun --show-sdk-path)/usr/include/c++/v1"`.
- **Hotkey does nothing** — another app may own Cmd+Alt+D; change it in Settings.
- **Text isn't pasting** — grant Accessibility permission; until then text is on the
  clipboard.

---

Built with Tauri v2, Rust, whisper.cpp, and TypeScript.
