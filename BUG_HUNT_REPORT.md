# BUG_HUNT_REPORT

## Review Metadata

- Date/time: 2026-06-01 08:03:50 EDT
- Scope: Full recon + targeted bug-hunt across frontend (`src/`) and backend (`src-tauri/src/`)
- Limitation: GUI-only runtime behavior (microphone permission dialogs, tray interactions) was not executed in this headless environment.

## Project Structure Summary

- Frontend entrypoint: `src/main.ts`
- Frontend styles: `src/styles.css`
- Backend entrypoints: `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- Core backend modules:
  - `dictation.rs` (record -> transcribe -> insert orchestration)
  - `audio.rs` (CPAL capture + resampling)
  - `whisper.rs` (local transcription)
  - `insert.rs` (clipboard + Cmd+V insertion)
  - `model.rs` (model registry + downloader)
  - `commands.rs` (Tauri command surface)
  - `config.rs` / `state.rs` (settings + shared state)

## Commands Discovered

- Frontend:
  - `npm run build`
  - `npm run dev`
  - `npm run tauri dev`
  - `npm run tauri build`
- Backend (`src-tauri`):
  - `cargo test`
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`

## Initial Risk Areas

- Hotkey update path in `commands::update_config` (state/persistence/runtime consistency)
- Background model download lifecycle and cancellation state transitions
- Multi-threaded dictation transitions (`Recording -> Transcribing -> Inserting -> Idle/Error`)
- Clipboard restore semantics and Accessibility fallback behavior

## Verification Commands Available

- `npm run build`
- `cargo test`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`

## Bugs

## Bug: Hotkey update can leave persisted config and active registration out of sync

Status: Fixed  
Severity: High  
Area: `src-tauri/src/commands.rs` (`update_config`)  
Type: Logic

### Evidence

`update_config` persisted the new config and updated in-memory config before attempting `register_hotkey`. If runtime registration failed (e.g., shortcut conflict), the command returned an error but the new value was already saved, violating the intended invariant in the comment.

### Impact

Users could see a saved hotkey that was not actually active. This causes confusing behavior and can make dictation appear broken after settings changes.

### Root Cause

Incorrect transaction ordering: persistence/state mutation occurred before runtime side-effect success was guaranteed.

### Fix Plan

Introduce a small transactional helper that:

1. Registers the new hotkey first when changed.
2. Persists config.
3. Rolls runtime registration back to the previous hotkey if persistence fails.

Then call that helper from `update_config` before mutating shared state.

### Verification

- Added regression tests:
  - `commands::tests::registers_new_hotkey_then_persists`
  - `commands::tests::rolls_back_hotkey_if_save_fails`
  - `commands::tests::unchanged_hotkey_only_persists`
- Ran:
  - `cargo test`
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `npm run build`

### Result

Fixed. Runtime registration and persisted config now remain consistent across success/failure paths.

## Bug: Failed hotkey re-registration can silently clear the previous shortcut

Status: Fixed  
Severity: High  
Area: `src-tauri/src/lib.rs` + `src-tauri/src/commands.rs` hotkey update path  
Type: Logic

### Evidence

`register_hotkey` begins by calling `unregister_all()` and then attempts the new registration. If the new registration fails (e.g., key already taken), the old hotkey is already removed.

### Impact

A failed hotkey change could leave the app with no active global shortcut until restart or manual reconfiguration, while the user only sees a generic validation error.

### Root Cause

Non-transactional replace behavior in shortcut registration: old binding removed before new binding success is guaranteed.

### Fix Plan

Extend hotkey apply logic to restore the previous hotkey when registering the new hotkey fails.

### Verification

- Added regression test:
  - `commands::tests::restores_previous_hotkey_when_new_registration_fails`
- Ran:
  - `cargo test`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `npm run build`

### Result

Fixed. Failed hotkey changes now attempt to restore the previous registration immediately.

## Bug: Status message rendering used `innerHTML` for backend-sourced text

Status: Fixed  
Severity: Medium  
Area: `src/main.ts` (`statusCard`)  
Type: Security

### Evidence

`statusCard` interpolated `s.message` into a template string assigned to `innerHTML`.

### Impact

If any backend error/status string ever includes HTML-like content, the renderer could interpret markup instead of treating it as plain text.

### Root Cause

Unsafe DOM insertion API (`innerHTML`) used for dynamic text fields.

### Fix Plan

Render status and hotkey hint using explicit DOM nodes and `textContent`/`append`, removing HTML interpretation from dynamic text.

### Verification

- Ran:
  - `npm run build`
  - `cargo test`
  - `cargo clippy --all-targets --all-features -- -D warnings`

### Result

Fixed. Dynamic status/hotkey text now renders as plain text only.

## Bug: Malformed/legacy config values could leave runtime in degraded state

Status: Fixed  
Severity: Medium  
Area: `src-tauri/src/config.rs`  
Type: Logic

### Evidence

`Config::validated` previously accepted any non-empty `model` and `language` values. Manual edits or stale config values could persist unknown model IDs (no active model selected) or unsupported language hints (possible transcription failures).

### Impact

App could boot with confusing state: dictation blocked by a non-existent model, or repeated transcription errors due to invalid language hints.

### Root Cause

Insufficient normalization and allow-list validation in config validation.

### Fix Plan

Normalize and constrain config values during validation:

1. Unknown/invalid model IDs fall back to default model (`base.en`).
2. Language is trimmed/lowercased and restricted to current supported values (`en`, `auto`), otherwise falls back to `auto`.

### Verification

- Added regression tests:
  - `config::tests::unknown_model_falls_back_to_default`
  - `config::tests::language_normalizes_and_falls_back_to_auto`
- Ran:
  - `cargo test`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `npm run build`

### Result

Fixed. Config loading is now resilient to malformed or legacy values without trapping users in degraded states.

## Bug: Overlapping refresh polling could trigger invoke pileups and stale UI races

Status: Fixed  
Severity: Medium  
Area: `src/main.ts` (`refreshAll` + recording timer)  
Type: Performance

### Evidence

`refreshAll` could be called repeatedly (events + 250ms recording poll) without an in-flight guard, allowing overlapping backend invokes and out-of-order UI updates.

### Impact

Under load or slow IPC, this can create unnecessary command pressure and transient stale renders, especially during long recordings.

### Root Cause

No concurrency guard around async status refresh pipeline.

### Fix Plan

Add a lightweight `refreshInFlight` guard to skip starting a new refresh while one is already running.

### Verification

- Ran:
  - `npm run build`
  - `cargo test`
  - `cargo clippy --all-targets --all-features -- -D warnings`

### Result

Fixed. Poll-driven and event-driven refreshes no longer overlap.

## Bug: Model download accepted mismatched byte length when server advertised content-length

Status: Fixed  
Severity: Medium  
Area: `src-tauri/src/model.rs` (`download`)  
Type: Reliability

### Evidence

When `sha256` is not pinned (current registry values), integrity depended on GGML header + minimum size. If a transfer ended with a mismatched `content-length`, there was no explicit length consistency check before model promotion.

### Impact

Potential acceptance of incomplete/corrupt model payloads in some network failure modes.

### Root Cause

Missing explicit `downloaded == content_length` validation when `content-length` is provided.

### Fix Plan

Add strict completion check before validation/promotion; delete `.part` and fail when expected and received byte counts differ.

### Verification

- Added regression tests:
  - `model::tests::detects_incomplete_download_when_length_known`
  - `model::tests::allows_download_when_length_unknown`
- Ran:
  - `cargo test`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `npm run build`

### Result

Fixed. Downloads with known length must now complete exactly before model finalization.

## Bug: Stale migration-era status document can mislead maintenance work

Status: Confirmed  
Severity: Low  
Area: `STATUS.md`  
Type: Documentation

### Evidence

`STATUS.md` describes old architecture details (e.g., command counts/modules/features) that do not match the rebuilt codebase.

### Impact

Potential confusion during onboarding and bug triage. Low runtime risk, but raises maintenance and DX risk.

### Root Cause

Documentation drift after major rebuild.

### Fix Plan

Defer to a dedicated documentation refresh pass so current behavior can be documented comprehensively and consistently with README + code.

### Verification

Manual comparison between `STATUS.md` and active source tree.

### Result

Confirmed, deferred.

## Re-review Pass (Post-fix)

Performed a deeper third scan focused on edge cases (config corruption/mismatch, polling overlap races, download integrity boundaries) and fixed additional confirmed issues with regression coverage.

## Latency Deep-Dive Checklist (Enterprise Responsiveness)

All items below were implemented in this pass.

1. Replace strict file-open model checks in status hot paths with metadata-only checks (`model::is_installed`) to cut repeated disk I/O during UI refreshes.
2. Tighten recorder shutdown latency (`audio::Recorder::finish` timeout from 10s to 3s) to fail fast instead of hanging stop->transcribe transitions.
3. Reduce stop-signal polling interval in capture loop (50ms -> 20ms) for faster stop responsiveness.
4. Cap audio callback buffer growth exactly at remaining capacity (avoid overshoot allocations/writes once max duration is reached).
5. Make Whisper thread count contention-aware (reserve one core; cap max threads) with env override (`SPIEL_WHISPER_THREADS`) for workload tuning.
6. Reduce pre-paste settle delay (120ms -> 60ms default) and expose runtime tuning (`SPIEL_PRE_PASTE_DELAY_MS`).
7. Reduce clipboard-restore delay (500ms -> 220ms default) and expose runtime tuning (`SPIEL_RESTORE_DELAY_MS`).
8. Remove backend polling for recording elapsed UI updates; use local client-side elapsed clock + lightweight render tick.
9. Narrow config-save refreshes to status/model targeted calls instead of full `get_status + get_config + list_models` every save.
10. Coalesce high-frequency frontend renders behind `requestAnimationFrame` (`queueRender`) to reduce reflow/repaint churn.
11. Throttle backend `model-progress` IPC emits to ~120ms cadence instead of per-chunk spam.

### Verification (Post-checklist)

- `cargo test` (23 passed)
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo fmt --check`
- `npm run build`

## Production Observability Upgrade

Added built-in profiling mode for real-world latency validation and tuning:

- Backend stage timing capture in dictation loop (capture/transcribe/insert/total).
- Rolling perf snapshot (`avg`, `p95`, `max`, over-budget count).
- New commands:
  - `get_perf_snapshot`
  - `clear_perf_samples`
- Frontend Performance Profile card in settings window (visible when `SPIEL_PROFILE=1`).
- Runtime tuning env vars documented in README.

## Memory Reduction Checklist (<100MB Idle Goal)

Implemented in this round:

1. Switch default model to `tiny.en` (lower model footprint than `base.en`).
2. Add `keep_model_loaded` setting and default it to `false`.
3. Unload cached model automatically after each dictation when `keep_model_loaded=false`.
4. Add `transcription_threads` setting and clamp to `1..8`.
5. Use configured thread count in Whisper decode path to cap thread-stack overhead.
6. Keep environment override (`SPIEL_WHISPER_THREADS`) for force-limiting thread usage in production.
7. Add explicit UI guidance for memory-first operation (Tiny + unload + low threads).
8. Add manual `unload_model_from_memory` command to force immediate memory release.
9. Add “Unload Model From Memory Now” button in settings for no-restart memory recovery.
10. Drop large capture buffer as soon as transcription completes (`drop(capture)`).
11. Keep status hot-path model checks metadata-only (`is_installed` no full file parse).

Verification for this round:

- `npm run build`
- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
