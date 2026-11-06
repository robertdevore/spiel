# Diagnostics decision log

## Collect health facts once and project the existing IPC views

- `commands.rs::get_readiness` and `build_startup_health` independently inspect config/model paths, model installation, accessibility, hotkeys and model recommendations. Policy changes require coordinated edits; repeated permission checks can disagree within one snapshot.
- `lib.rs` calls `commands::build_startup_health`, making application startup and maintenance depend on the settings IPC adapter.
- Write probes duplicate their file creation/write/removal lifecycle.

Move diagnostic data, inspection and tests into `diagnostics.rs`. Keep both public snapshot shapes and warning order unchanged, with a shared facts collector and pure setup/warning decisions. Tauri command functions delegate to the diagnostics domain; startup and maintenance call it directly. Retain filesystem probing and platform-specific Accessibility semantics.

## Give hotkeys their own owner

Hotkey parsing and registration live in the application composition root while settings persistence/rollback lives in the IPC layer. Move parsing, registration and the tested persistence transaction into `hotkey.rs`, so diagnostic and command callers depend on the domain rather than application startup.

## Reserve downloads before touching model storage

`download_model` checks active state, releases its mutex, cleans partial files, then checks active state again. Concurrent requests can clean the first request's active partial file. Reserve the slot under one lock before cleanup/preflight; reset on an already-installed rejection. Preserve the existing single-download policy and response messages.

## Start initialized tasks directly

Startup health and optional model warmup sleep for fixed 350/500 ms despite starting after state registration. Remove these timing guesses; state readiness follows setup order. The UI's explicit startup-health query still supports windows opened later.

Periodic maintenance uses the same download mutex and skips cleanup while a reservation is active, so maintenance cannot remove partial artifacts owned by a running download.

## Make download ownership release itself

Replace independently mutable active/id/progress/cancel fields with a `Downloads` owner and a non-cloneable reservation. A reservation claims the single download slot before preflight and releases it on every return/unwind or explicit completion. Progress travels directly in events; the old stored progress fields had no readers. Maintenance runs through `when_idle`, keeping exclusion and cleanup in one critical section. Regression tests exercise competing thread reservations, failed-preflight release, cancellation isolation, and maintenance exclusion.
