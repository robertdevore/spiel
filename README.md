# Spiel

Spiel is a local-first desktop dictation app. Press a hotkey, speak, press it again, and Spiel transcribes your speech locally and inserts the text where your cursor was.

It is built with Tauri, Rust, and Whisper through `whisper-rs`.

## Why Spiel Exists

Most dictation tools either send audio to a cloud service, interrupt your workflow, or require a subscription. Spiel is designed to be quiet, fast, and private:

- Transcription runs on your machine.
- No account is required.
- No telemetry is built in.
- Normal dictation does not write raw audio files to disk.
- Model downloads happen only when you choose to install a model.

## Download

Installers are attached to GitHub releases:

https://github.com/robertdevore/spiel/releases

Choose the file for your computer:

- Intel Mac: `Spiel_1.0.1_macOS_x64.dmg`
- Apple Silicon Mac: `Spiel_1.0.1_macOS_arm64.dmg`
- Windows: `Spiel_1.0.1_x64-setup.exe` or `Spiel_1.0.1_x64_en-US.msi`
- Linux: `Spiel_1.0.1_amd64.AppImage` or `Spiel_1.0.1_amd64.deb`

### macOS Security Note

Current release builds are unsigned and not notarized. macOS may show a Gatekeeper warning such as:

> Apple could not verify "Spiel.app" is free of malware.

For now, open it from `System Settings -> Privacy & Security -> Open Anyway`, or remove the quarantine flag after copying the app to Applications:

```bash
xattr -dr com.apple.quarantine /Applications/Spiel.app
open /Applications/Spiel.app
```

Signed and notarized macOS builds require Apple Developer ID credentials in the release workflow.

## Quick Start

1. Install Spiel for your platform.
2. Open the app.
3. Install a speech model when prompted. `base.en` is a good first choice for English.
4. Grant microphone permission.
5. On macOS, grant Accessibility permission if you want Spiel to paste text into the active app automatically.
6. Put your cursor in a text field.
7. Press the global hotkey, speak, then press it again to stop.

The default hotkey is `Cmd+Alt+D` on macOS.

## How Dictation Works

Spiel is a toggle:

1. Start dictation from the hotkey, menu bar, or app window.
2. Speak normally.
3. Stop dictation with the same control.
4. Spiel transcribes locally.
5. Spiel inserts the text at the cursor when supported.

If automatic paste is unavailable, Spiel leaves the transcript on the clipboard so you can paste manually.

## Models

Spiel supports English-only and multilingual Whisper model families:

- English-only: `tiny.en`, `base.en`, `small.en`
- Multilingual: `tiny`, `base`, `small`, `medium`

Smaller models are faster and use less memory. Larger models can be more accurate but need more disk space, RAM, and CPU time.

## Privacy

Spiel is designed for local-first dictation:

- Audio capture is processed locally.
- Transcription runs through local Whisper models.
- No transcript or audio telemetry is sent by the app.
- Network access is used for model downloads and optional checksum manifest checks.
- Model paths are validated to reject unsafe symlink or traversal targets.

## Platform Notes

macOS is the primary supported platform. It supports microphone capture, global hotkey dictation, and Accessibility-assisted paste.

Windows and Linux builds are available from the release workflow, but cursor insertion behavior may be clipboard-first depending on platform permissions and desktop environment support.

## Build From Source

Requirements:

- Node.js 22+
- Rust stable
- Platform build tools required by Tauri

Install dependencies:

```bash
npm install
```

Run in development:

```bash
npm run tauri dev
```

Build the frontend:

```bash
npm run build
```

Run Rust tests:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Build a native package for the current operating system:

```bash
npm run package
```

Platform-specific package commands:

```bash
npm run package:mac
npm run package:windows
npm run package:linux
```

Local macOS artifacts are written to `release/artifacts/`.

## Release Builds

The `Package apps` GitHub Actions workflow builds and uploads:

- `Spiel-macOS-arm64`: Apple Silicon `.app.zip` and `.dmg`
- `Spiel-macOS-x64`: Intel `.app.zip` and `.dmg`
- `Spiel-Windows`: `.msi` and NSIS `.exe`
- `Spiel-Linux`: `.AppImage` and `.deb`

Publishing a GitHub release runs the workflow and attaches those files to the release after each platform build finishes.

## Useful Environment Variables

- `SPIEL_MODEL_DIR`: custom local model storage directory.
- `SPIEL_WHISPER_THREADS`: override transcription thread count.
- `SPIEL_WARMUP_ON_START`: preload the current model on launch.
- `SPIEL_PRE_PASTE_DELAY_MS`: delay before paste.
- `SPIEL_RESTORE_DELAY_MS`: delay before restoring the clipboard.
- `SPIEL_DOWNLOAD_RETRIES`: retry count for transient model download failures.
- `SPIEL_DOWNLOAD_RETRY_BACKOFF_MS`: initial retry backoff.
- `SPIEL_DOWNLOAD_CONNECT_TIMEOUT_MS`: model download connection timeout.
- `SPIEL_DOWNLOAD_TIMEOUT_MS`: model download total timeout.
- `SPIEL_MODEL_MANIFEST_URL`: optional checksum manifest URL.

## Troubleshooting

### macOS says the app cannot be verified

The current builds are not notarized. Use `System Settings -> Privacy & Security -> Open Anyway`, or remove the quarantine flag with `xattr` after copying the app to Applications.

### The hotkey does nothing

Another app may already own the shortcut. Open Spiel settings and choose a different hotkey.

### Dictation records but does not paste

On macOS, automatic paste requires Accessibility permission. If Accessibility is not granted, Spiel should leave the transcript on the clipboard for manual paste.

### Transcription is slow

Use a smaller model, reduce transcription threads, or enable model warm-up/keep-loaded behavior if you have enough memory.

### Model install says partial or corrupt

Delete the model from Spiel and download it again. If you use sidecar checksums, make sure the `.sha256` file matches the model exactly.

## Repository Layout

- `src/`: Tauri frontend.
- `src-tauri/src/`: Rust backend for capture, transcription, insertion, model management, settings, and IPC commands.
- `scripts/`: packaging helpers.
- `.github/workflows/package.yml`: release packaging workflow.

See [CHANGELOG.md](CHANGELOG.md) for release notes.

## License

MIT. See `LICENSE`.
