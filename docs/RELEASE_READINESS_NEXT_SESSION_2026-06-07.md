# Spiel — Release Readiness Next Session (2026-06-07)

## This Cycle (Completed)

1. Added install metadata visibility in model status: status reason + file mtime (`install_reason`, `install_modified_ms`).
2. Threaded install metadata through `list_models` and surfaced it in the speech-model UI card.
3. Added local checksum-sidecar support (`<model>.sha256`) with strict token parsing and hash validation.
4. Refactored download flow to validate hashes via shared validation path and removed duplicate inline checksum logic.
5. Added bounded download retry behavior with exponential backoff and jitter.
6. Added runtime env knobs:
   - `SPIEL_DOWNLOAD_RETRIES`
   - `SPIEL_DOWNLOAD_RETRY_BACKOFF_MS`
7. Added regression tests for download knobs and checksum-sidecar validation.
8. Kept existing safety checks (`is_safe_model_path`) in download/install flows.
9. Refreshed root README with updated env controls and integrity behavior.
10. Verified all current automated checks (`cargo test`, `cargo clippy -D warnings`, `cargo fmt --check`, `npm run build`).

## Release Readiness Status

Spiel is materially stronger for enterprise deployment, but we still have room before “shining star” status:

- We now have better integrity posture, retry resilience, and observability.
- We still need explicit multilingual model quality guidance, performance guardrails, and stronger lifecycle UX around permission state and failures.

## Next Session Checklist

1. Add startup integrity self-check event
   - Expose a one-command health probe for model artifacts + runtime readiness and return a machine-consumable summary payload.

2. Add configurable model warm-up profiles
   - `low_memory`, `balanced`, `quality` presets affecting `keep_model_loaded`, thread cap, and first-run preloads.

3. Add integrity provenance for remote downloads
   - Add checksum manifest download support (registry JSON or detached manifest file) so drift is detected before install finalize.

4. Add performance telemetry actionability
   - Emit structured performance samples (download, transcribe, insert p50/p95) and wire them into a lightweight dashboard in the UI.

5. Add model selection defaults per language family
   - Provide explicit user-facing recommendation for English-only vs multilingual path based on configured language.

6. Add periodic stale-artifact cleanup automation
   - Add a background cleanup task for stale `.part` and non-`*.bin` model artifacts and stale sidecar files.

7. Add stronger cancellation semantics in download pipeline
   - Distinguish cancellation reason, network failure reason, checksum failure reason in progress/status updates.

8. Add test coverage for transient download states
   - Unit/integration-style tests for retry, backoff delay calculations, cancellation, and partial-byte recovery.

9. Add deterministic startup migration checks
   - Validate `model_dir`, `config` path safety, and model permissions in a startup health report before user interacts.

10. Add cross-platform smoke matrix doc + scripts
   - Non-macOS flow checks, macOS accessibility/microphone permission matrix, and model download/install smoke steps.

11. Add explicit clipboard outcome metrics
   - Track whether transcription was inserted automatically, manually copied, or failed to paste, and expose counts in status/perf panel.

12. Add first-run setup wizard
   - Permission checks + recommended model + quick validation pass for a frictionless enterprise onboarding path.

## Files to Keep Watching

- `docs/RELEASE_READINESS_NEXT_SESSION.md`
- `docs/ENTERPRISE_READINESS_NEXT_SESSION.md`
- `docs/UNIVERSAL_REVIEW_NEXT_SESSION.md`
- `docs/PRODUCTION_ENHANCEMENT_NEXT_SESSION.md`
- `BUG_HUNT_REPORT.md`

