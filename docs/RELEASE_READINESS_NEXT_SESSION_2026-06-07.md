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

Spiel now has substantially stronger release scaffolding:

- startup health can be fetched and emitted as an event
- model recommendations are language-aware
- warm-up can be triggered manually or on startup
- remote checksum provenance can come from a manifest or detached sidecar
- perf telemetry includes stage/actionability metrics and clipboard/download outcomes
- stale artifact cleanup now runs at startup and on a maintenance interval
- setup guidance is surfaced directly in the UI instead of being hidden in docs

## Checklist Completion

1. Startup integrity self-check event: completed
2. Configurable model warm-up flow: completed
3. Integrity provenance for remote downloads: completed
4. Performance telemetry actionability: completed
5. Model selection defaults per language family: completed
6. Periodic stale-artifact cleanup automation: completed
7. Stronger cancellation semantics in download pipeline: completed
8. Test coverage for transient download states: completed
9. Deterministic startup migration checks: completed
10. Cross-platform smoke matrix doc + script: completed
11. Explicit clipboard outcome metrics: completed
12. First-run setup wizard: completed

## Next Follow-Up Ideas

1. Add a persistent onboarding completion flag so enterprise deployments can suppress the wizard after policy-based provisioning.
2. Add signed manifest verification for checksum sources, not just detached hash transport.
3. Add an in-app release diagnostics export bundle for support teams.
4. Add integration tests around startup-health events and warm-up behavior.

## Files to Keep Watching

- `docs/RELEASE_READINESS_NEXT_SESSION.md`
- `docs/ENTERPRISE_READINESS_NEXT_SESSION.md`
- `docs/UNIVERSAL_REVIEW_NEXT_SESSION.md`
- `docs/PRODUCTION_ENHANCEMENT_NEXT_SESSION.md`
- `BUG_HUNT_REPORT.md`
