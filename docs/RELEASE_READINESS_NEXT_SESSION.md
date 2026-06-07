# Spiel Release Readiness — Next Session Checklist

_Date: 2026-06-06_

## Summary

Spiel is now production-oriented for local-first dictation with practical multilingual support, stronger config validation, and safer runtime behavior. The goal of this pass was to remove remaining user-facing rough edges and make the codebase easier to operate as a reference implementation.

## What’s completed in this cycle

1. Multilingual model registry
- Added English and multilingual checkpoints for small/base/medium families.
- Added model metadata (`multilingual`) to enforce language compatibility.

2. Language validation and normalization
- Added language hint normalization and validation with region-safe normalization (e.g. `en-US` -> `en`).
- Prevented invalid language + English-only model combinations from breaking startup.

3. Safe settings persistence
- Switched config write path to atomic temp-file write + rename.
- Added best-practice config file mode on Unix (`0600`) for local privacy hardening.

4. Download robustness
- Added cleanup guarantee for partial `.part` files on stream/write failures.
- Kept explicit content-length completion checks and integrity path intact.

5. Audio memory profile improvements
- In-callback downmixing to mono and lower per-session sample target, reducing peak recording memory.
- Kept stop timeout and capture cap safeguards in place.

6. Accessibility status realism
- Snapshot now reflects live trust state so stale “needs Accessibility” notices clear automatically after grant.

7. UI language experience
- Replaced fixed language select with datalist-backed language field + normalization before save.
- Added multilingual-aware profiles and a global multilingual profile option.

8. Docs modernization
- Updated README for multilingual model guidance and security/performance posture.
- Added this next-session checklist document.

9. Legacy dead code cleanup
- Removed stale legacy module that was no longer in the active architecture.

## Remaining follow-up (recommended next session)

1. Add model caching telemetry and startup cache warm-up diagnostics (cache hit/miss counts).
2. Add UI display of active model and current thread/language validation state.
3. Add property-based tests for config normalization edge cases and `normalize_language` parity between frontend and backend.
4. Add integration-style test around full dictation path mock (capture + transcribe stub + insert stub).
5. Replace broad `eprintln!` calls with a structured logger sink in production.
6. Add onboarding modal for first-run model+permissions flow.
7. Add runtime warning when multilingual+English-only models are mixed in saved config.
8. Add optional model prefetch/background checksum verification job.
9. Add an explicit accessibility trust change event to refresh UI instantly when permission flips externally.
10. Expand `docs/REVIEW` process to include startup/memory baseline scripts + reproducible test matrix.
