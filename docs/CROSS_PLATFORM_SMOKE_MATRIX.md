# Spiel Cross-Platform Smoke Matrix

## Goal

This is the quick release gate for validating that Spiel still behaves predictably across macOS-first flows and non-macOS fallback behavior.

## macOS smoke

1. Launch app from source with `npm run tauri dev`.
2. Confirm tray icon appears and settings window can be shown/hidden.
3. Verify first-run setup wizard appears when the current model is missing.
4. Download the recommended model and confirm `model-done` completes without UI lockup.
5. Trigger dictation with the configured hotkey and verify:
   - recording starts
   - stop transitions to transcribing
   - transcript appears
   - auto-paste succeeds when Accessibility is granted
6. Disable Accessibility and verify clipboard-only fallback message appears.
7. Click `Warm Current Model` and verify no error is surfaced.
8. Toggle `keep_model_loaded` and confirm repeated dictations reduce first-response latency.
9. Inspect readiness and perf panels for:
   - writable config/model paths
   - recommended model
   - p50/p95 totals
   - download sample metrics

## Linux / Windows fallback smoke

1. Launch app.
2. Confirm settings window loads and model/readiness panels render.
3. Confirm Accessibility guidance states unsupported/no-op rather than failing.
4. Download a model and verify install diagnostics update.
5. Run dictation and confirm clipboard-only flow is surfaced clearly if auto-paste is unavailable.

## Integrity checks

1. If `SPIEL_MODEL_MANIFEST_URL` is set, confirm downloads still complete and checksum provenance is accepted.
2. Add a bad `<model>.sha256` sidecar and confirm the model is reported as corrupt.
3. Leave an orphan `.sha256` file in the model directory and confirm periodic cleanup removes it.

## Release commands

```bash
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
```

