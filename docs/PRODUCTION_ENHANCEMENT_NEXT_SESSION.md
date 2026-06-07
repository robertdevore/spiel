# Spiel Production Enhancement — Next Session

_Date: 2026-06-07_

## This Cycle (Summary)

- Hardened readiness diagnostics with model-store footprint reporting and writable-path validation.
- Added safe model download temp-file handling and path safety checks to reduce partial-write risk.
- Improved model artifact cleanup behavior and cleanup-window configuration.
- Added model-store telemetry tests and symlink-aware directory scanning.
- Increased frontend visibility of operational state (`get_readiness`, model directory health, storage usage).
- Refined audio resampling and buffering edge cases so duration caps stay consistent across sample rates.

## Why this improves enterprise readiness

- Faster support diagnostics when a model install is unavailable or slow to download.
- Reduced ambiguity around write-permission and storage constraints.
- Better confidence in operational safety against symlink-based path attacks.
- Clearer memory and disk observability for fleet rollout guidance.

## Next Session Focus (Prioritized)

1. Add model manifest-driven integrity policy (sha manifest URL + per-model pinned checksums) and fail-fast on drift.
2. Add startup integrity self-check event for installed models and cached transcriber status.
3. Add a first-run onboarding wizard (`model + permissions + permissions retry`) with explicit zero-friction flow.
4. Add memory profile presets that can be applied by one click (`low`, `balanced`, `high-accuracy`).
5. Add transient-download retry with exponential backoff and jitter.
6. Add cross-platform smoke matrix and a scripted verification checklist (macOS, Linux, Windows).
7. Move storage diagnostics into `list_models` (bytes + last-modified) to avoid extra backend round-trips.
8. Add transcript outcome telemetry to distinguish pasted vs clipboard-only outcomes in `PerfEvent`.
9. Add model warmup policy environment setting for first-transcribe latency with explicit memory budget controls.
10. Add structured logging sink in Rust for support triage without leaking transcript/model content.
11. Add explicit test for readiness command under concurrent reads and ephemeral model-dir failures.
