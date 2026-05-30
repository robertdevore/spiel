# Spiel — Release Notes

## Current Release Status

**Version**: 0.1.0
**Phase**: 11 — Packaging and Release Readiness
**Status**: Pre-release (internal development)

Spiel is functional but not yet distributed publicly. All core features work locally in development mode.

## Build Prerequisites

- **Rust**: 1.86+ (1.96.0 tested)
- **Node.js**: 18+ (23.11.0 tested)
- **npm**: 9+
- **macOS**: 12+ (primary target)
- **Windows/Linux**: Configured in Tauri but not verified

## Local Development Commands

```bash
# Install dependencies
npm install

# Run in development mode (hot reload)
npm run tauri dev

# Frontend-only dev (browser, no Rust backend)
npm run dev

# TypeScript type check
npx tsc --noEmit

# Frontend production build
npm run build

# Rust format check
cd src-tauri && cargo fmt --check

# Rust type check
cd src-tauri && cargo check

# Full Rust build
cd src-tauri && cargo build
```

## Production Build

```bash
# Full Tauri production build
npm run tauri build
```

This produces platform-specific binaries in `src-tauri/target/release/bundle/`.

### macOS Output
- `.app` bundle in `target/release/bundle/macos/`
- `.dmg` disk image (if configured)
- **Not code signed** — users must right-click → Open on first launch
- **Not notarized** — Gatekeeper will show a warning

### Windows Output
- `.msi` installer and/or `.exe` in `target/release/bundle/msi/`
- **Not verified** — Windows builds not tested in current environment

### Linux Output
- `.deb` and/or `.AppImage` in `target/release/bundle/deb/`
- **Not verified** — Linux builds not tested in current environment

## Signing and Notarization

- **Status**: Not configured
- **macOS code signing**: Requires Apple Developer account + signing certificates
- **macOS notarization**: Requires Apple Developer account + notary tool setup
- **Windows signing**: Requires code signing certificate
- These are deployment tasks, not development tasks.

## Installer Status

- Tauri generates platform installers automatically via `tauri build`
- Installers are functional but unsigned
- Custom installer branding not configured

## Manual QA Checklist

### Core Features
- [ ] App launches and shows "Spiel" title
- [ ] Global hotkey (Cmd+Shift+S) registers
- [ ] Start recording via button
- [ ] Stop recording via button
- [ ] Recording metadata displays (file, duration, size)
- [ ] Mock transcription works
- [ ] Local Whisper transcription works (if configured)
- [ ] Cleanup modes all produce output
- [ ] Copy final text to clipboard
- [ ] Insert final text at cursor (manual paste)
- [ ] Save to history
- [ ] View history entry
- [ ] Delete history entry
- [ ] Clear all history
- [ ] Settings panel loads
- [ ] Save settings
- [ ] Reset settings to defaults
- [ ] Settings persist after restart
- [ ] Local-only mode visible
- [ ] Privacy status accurate

### Safety Checks
- [ ] Auto-insert defaults to off
- [ ] No Enter key sent on insert
- [ ] No form/message submission
- [ ] No network calls in Developer Tools
- [ ] No OpenAI/cloud calls
- [ ] No API keys visible in UI or storage
- [ ] Clipboard restore works (best-effort)

### Platform Checks (macOS)
- [ ] Microphone permission prompt appears
- [ ] Global hotkey works
- [ ] App appears in Dock with correct name
- [ ] Quit and reopen preserves data

## Smoke Test Checklist

Quick verification for each build:
- [ ] `npm run tauri dev` launches without errors
- [ ] `npm run build` succeeds
- [ ] `cargo build` succeeds
- [ ] `cargo check` reports no errors
- [ ] `npx tsc --noEmit` reports no errors
- [ ] `cargo fmt --check` passes

## Known Release Blockers

| Blocker | Impact | Resolution |
|---------|--------|------------|
| macOS code signing not configured | Users see Gatekeeper warning | Requires Apple Developer account |
| Windows/Linux not tested | Unknown issues may exist | Needs platform testing |
| No app icon branding | Uses default Tauri icon | Generate custom icons |
| Temp audio files not auto-cleaned | Disk space accumulation | Add cleanup on quit |
| No auto-updater | Manual updates only | Consider for post-1.0 |
