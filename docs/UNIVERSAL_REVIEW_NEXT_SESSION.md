# Spiel — Enterprise-Readiness Round

_Date: 2026-06-06_

## This Round: Completed

1. Language input parity
- Frontend language normalization now aligns to backend policy (`auto` or 2-letter lowercase codes).
- Added validation path updates so `es-US` normalizes cleanly while `eng` is rejected.

2. Audio hot path efficiency
- In-callback downmixing already writes mono, so finish path no longer re-downmixes.
- Capture buffer now pre-allocates target capacity to avoid repeated growth allocations.
- Callback ignores partial frames rather than coercing uneven-frame samples.

3. Safer, bounded downloads
- Added download timeout config via environment:
  - `SPIEL_DOWNLOAD_CONNECT_TIMEOUT_MS`
  - `SPIEL_DOWNLOAD_TIMEOUT_MS`
- Added path-safety checks to refuse symlink model targets.
- Added cleanup of stale `.part` files before a new download begins.

4. Operational control: model deletion
- Added backend `delete_model` command.
- Added UI model delete action for installed non-active models.
- Deletion clears warm model cache to avoid stale in-memory state.

5. Flexible model storage location
- Added `SPIEL_MODEL_DIR` override for enterprise/local-share/custom model layouts.
- Updated docs to reflect new runtime knob.

6. Documentation + surface updates
- Updated README command surface and runtime controls.
- Updated STATUS command count and list.

## Verification

- `cargo fmt`
- `cargo test` (35 tests passed)
- `cargo clippy --all-targets --all-features -- -D warnings`
- `npm run build`

## Enterprise Enhancements to Schedule Next

1. Add offline installer cache lifecycle management (`list_models` should expose disk usage and last-used ages).
2. Add end-to-end recovery tests for model delete/download/permission state in CI.
3. Expose model health checks (missing files, stale checksum cache, disk fullness) in the Settings panel.
4. Add first-run onboarding wizard (permissions + minimum model recommendation).
5. Add optional model auto-download policy (`wifi_only`, `allow_unmetered_only`) for policy-driven environments.
6. Add structured logging instead of `eprintln!` and redact sensitive values for enterprise tracing.
7. Add accessibility trust-change event polling so UI updates when trust is changed outside the app.
8. Add optional “memory pressure” mode in which transcription threads auto-scale from config when RAM is low.
9. Add cross-platform smoke test command in CI for macOS/Windows/Linux parity.
10. Add property-based tests for language/model validation and command boundary states.
11. Add explicit hardening for hostile `SPIEL_MODEL_DIR` paths and path normalization.
