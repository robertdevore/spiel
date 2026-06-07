# Spiel Release Readiness — Next Session Checklist

_Date: 2026-06-07_

## What moved this cycle

1. Centralized stale `.part` cleanup duration handling with a shared parser and startup/default behavior.
2. Added config persistence hardening for symlink-safe settings path handling.
3. Added collision-safe temp config writes and fsync before finalizing settings.
4. Added model-ready diagnostics command (`get_readiness`) and UI diagnostics card.
5. Added model directory writability probe to readiness payloads for supportability.
6. Improved startup and download cleanup handling to consistently use shared cleanup policy.
7. Added regression coverage for part cleanup parsing and symlink rejection in config path handling.
8. Added model-store diagnostics (`get_readiness` now reports model artifact bytes and file counts).

## Next Session Checklist

1. Add checksum manifest support for model registry entries to verify trusted releases without manual `sha256` updates.
2. Add explicit startup model integrity self-check command/event for automated post-install checks.
3. Add configurable model warmup profile (e.g., `SPIEL_WARMUP_MODEL`) with first-run recommendation.
4. Persist and surface clipboard paste outcome categories (`pasted` vs `clipboard_only`) in transcript telemetry.
5. Add backoff-aware model download retry policy for transient network errors.
6. Add optional runtime log sink with privacy-safe redaction.
7. Add non-macOS first-class UX treatment for permission flows in onboarding text.
8. Add integration-style test for command-state transitions under rapid hotkey toggles.
9. Add configurable model warmup policy for preloading and first-run optimization.
10. Move disk usage telemetry into `list_models` with artifact metadata + last modified timestamps.
11. Add performance guardrail preset that auto-reduces transcription threads when memory pressure is detected.
12. Add unit test for readiness probe path to ensure temp-file cleanup on partial failures.
13. Add cross-platform smoke checklist for Linux/Windows packaging and startup behavior.
