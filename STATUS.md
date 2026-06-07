# Spiel — Release Readiness Status

Last updated: 2026-06-07
Version: 0.2.0
Branch: `main`

## Release Summary

Spiel is feature-complete for local macOS dictation (record -> transcribe -> insert) with a functioning tray UX, global hotkey, offline transcription path, model download flow, and built-in latency profiling.

## Verified In This Environment

- TypeScript + Vite build passes.
- Rust format check passes.
- Rust tests pass.
- Rust clippy with `-D warnings` passes.

Commands:

```bash
npm run build
cd src-tauri
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Current Command Surface (Tauri)

The app currently exposes 14 Tauri commands:

- `get_status`
- `get_config`
- `get_readiness`
- `get_perf_snapshot`
- `clear_perf_samples`
- `update_config`
- `list_models`
- `download_model`
- `cancel_download`
- `delete_model`
- `unload_model_from_memory`
- `toggle_dictation`
- `accessibility_status`
- `request_accessibility`
- `show_settings`

## Security/Privacy Snapshot

- Local-first transcription path (whisper.cpp via `whisper-rs`).
- No account or telemetry systems present.
- Network use is limited to user-initiated model downloads.
- CSP is restricted to local app surfaces.
- Clipboard fallback prevents transcript loss when Accessibility is unavailable.

## Platform And Packaging Readiness

Ready:

- macOS app/tray architecture configured.
- App and DMG bundle targets configured in `tauri.conf.json`.
- Microphone usage description present in `Info.plist` merge file.
- macOS audio-input entitlement present for hardened runtime builds.

Not fully verified in this headless environment:

- Full GUI runtime smoke test.
- End-to-end permission prompt UX (Microphone + Accessibility).
- Signed/notarized distribution workflow.

## Known Release Blockers

- Code signing and notarization are not configured in this repository.
- Windows and Linux runtime validation is not part of this release scope.

## Current Next-Session Backlog

See:

- `docs/RELEASE_READINESS_NEXT_SESSION.md`
- `docs/ENTERPRISE_READINESS_NEXT_SESSION.md`.

## Recommended Final Pre-Release QA

1. Run `npm run tauri dev` on a macOS desktop session.
2. Validate first-run flow (model download, microphone prompt, accessibility flow).
3. Validate hotkey conflict handling and fallback behavior.
4. Validate paste behavior in at least 3 target apps (e.g., Notes, Slack, browser text area).
5. Build distributable with `npm run tauri build` and test launch/install path on a clean machine.
