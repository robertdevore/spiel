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

Performed a second scan of high-risk modules and reran full verification commands. No additional reproducible runtime defects were found in the inspected scope.
