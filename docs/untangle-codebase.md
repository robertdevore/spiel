# Spiel structural refactor — 2026-11-06

Applied [untangle-codebase](https://github.com/caldoai/skills/blob/main/skills/untangle-codebase/SKILL.md)
from baseline `3d9a0eb`. Optional blanket comment/test cleanup was not selected.

## Decisions and evidence

- [Frontend features and actions](untangle-frontend.md)
- [Audio accumulation](untangle-audio.md)
- [Diagnostics, hotkeys and download ownership](untangle-diagnostics.md)
- [Model artifacts and typed download failures](untangle-models.md)
- [Transcription, presentation and profiling](untangle-runtime.md)

Independent review covered frontend, backend, and build scripts before edits and
again after integration. The accepted structural findings are addressed. Required
OS focus/clipboard timing, polling, network retries, platform branches and model
validation remain. No native microphone or foreground-application paste session
was performed; those integrations retain their existing APIs.

Build scripts share `build-env.sh`, preserving the existing macOS SDK C++ header
workaround. On this machine plain Cargo failed to locate C++ standard headers;
source that script before native checks. Debug artifacts exhausted disk during
Clippy; final checks disable debug symbols and incremental compilation to bound
build storage. Only generated Spiel debug output was cleaned.

The dependency audit found existing high-severity development dependency reports
for locked Vite 6.4.3 and source-map-js. Dependency upgrades are separate from this
structural change; the review item is recorded in SignalBox. This audit does not
establish exploitability of the packaged app.

## Final verification

- Production frontend build and strict TypeScript check passed.
- Three original/refactored DOM fixture comparisons passed; shared permission,
  download-error cleanup, and focus-refresh interaction checks passed.
- Rust suite passed: 72 library tests and 5 transcription-helper tests.
- `cargo fmt --check`, shell syntax checks, and `git diff --check` passed.
- Clippy passed for all targets/features with warnings denied.
- Independent agent follow-up reviews found no further worthwhile structural issue.

Reproduce native verification with:

```sh
source scripts/build-env.sh
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
```
