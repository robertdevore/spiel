# Spiel — Enterprise Readiness Next Session

_Date: 2026-06-07_

## Status Snapshot

Spiel is now significantly beyond the minimum runnable path: model install health is explicit, multilingual support is stable, download lifecycle is safer, audio memory is bounded, and permissions/status UX now stays synchronized in real time.

## This Cycle — Completed (High-Value)

- ✅ Hardened model path handling (`SPIEL_MODEL_DIR`) against `..` traversal and symlink abuse.
- ✅ Introduced install health states (`installed`, `partial`, `corrupt`, `unsafe_path`) for every registry item.
- ✅ Added model download status caching + TTL to reduce repeated disk IO in status and model-list refreshes.
- ✅ Added install size reporting to the UI model cards.
- ✅ Added safe model deletion flow with guardrails for active downloads and active-model protection.
- ✅ Added in-callback capture downmixing and bounded target-rate buffering.
- ✅ Added accessibility trust polling to keep status/UI trust state current.
- ✅ Fixed audio callback hot-path allocations and added focused regression tests.
- ✅ Improved model delete robustness by evicting cached model context before file removal.
- ✅ Consolidated and updated root documentation for enterprise expectations and operational knobs.

## Next Session Checklist

Use this as the next production-hardening batch. Each item is intentionally concrete and testable.

1. **Add property-based tests for `normalize_language_hint` and status transitions**
   - Why: protects against regressions in multilingual behavior and config normalization.
   - Deliverable: at least 20 quick property checks with edge and fuzz inputs.

2. **Add a startup self-check command / event**
   - Why: enterprise environments need deterministic readiness evidence on launch.
   - Deliverable: optional event payload with model cache status + model cache health summary.

3. **Add explicit startup model warm-up policy**
   - Why: reduce “first transcription jitter” while preserving memory rules.
   - Deliverable: controlled preload for selected profiles (`global`, `quality`, `low_memory`).

4. **Move install cache TTL and memory guardrails behind config/UI profile**
   - Why: one-size-fits-all TTL/behavior can overfit small RAM and large RAM machines.
   - Deliverable: profile-aware polling + cap settings in config.

5. **Add structured logging backend for events**
   - Why: replace `eprintln!` with a lightweight sink so logs are searchable in enterprise environments.
   - Deliverable: redact model IDs/paths where appropriate.

6. **Add cancellation + resume telemetry for interrupted downloads**
   - Why: teams need to audit partial downloads and retry behavior.
   - Deliverable: expose last-failed reason and `bytes_downloaded/expected` in UI.

7. **Add non-invasive stress test for repeated hotkey toggles**
   - Why: ensures dictation state machine stays coherent under rapid edge traffic.
   - Deliverable: unit/integration-style coverage around `toggle` transitions.

8. **Add clipboard safety test hooks**
   - Why: insertion edge-cases vary across editors and permissions; catching regressions is hard manually.
   - Deliverable: mock-bridgeable insert module with explicit outcome assertions.

9. **Add model registry metadata expansion**
   - Why: make multilingual quality settings clearer and easier to reason about.
   - Deliverable: documented fields for approximate RAM and expected WER class by model profile.

10. **Add `docs/` end-to-end runbook and CI matrix plan**
   - Why: enterprise adoption needs reproducible verification steps.
   - Deliverable: one-page runbook + matrix for macOS smoke + permissions + install flows.

11. **Reduce UI re-render churn by patching partial DOM updates for model progress**
   - Why: large model lists or high-framerate recording can churn allocations.
   - Deliverable: small DOM reconciliation path for `model-progress` events.

12. **Add explicit compatibility note for non-macOS targets**
   - Why: current desktop UX is macOS-first and accessibility behavior differs by platform.
   - Deliverable: README + STATUS update stating support scope and limitations.

13. **Add model-store cleanup policy for stale `.part` files**
   - Why: disk hygiene for unattended environments.
   - Deliverable: periodic cleanup and age-based cleanup command.

14. **Add perf-baseline script**
   - Why: quantify memory and latency claims with reproducible numbers.
   - Deliverable: a local script that captures `RSS`, `transcribe_ms`, and `insert_ms` medians.

## Open Risk to Watch

- Whisper load and insert behavior are tightly coupled to machine entropy/performance characteristics.
- Permission prompts remain OS-driven and user-scoped, so release tests must validate in a clean-profile macOS session.
