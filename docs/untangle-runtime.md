# Runtime untangling decisions

Source: requested caldoai/skills untangle-codebase skill, 2026-11-06.
Optional comment/test cleanup was not selected; existing behavior tests remain.

- Give transcription one cache owner and an explicit configuration snapshot.
  - dictation.rs rereads the selected model independently of language and loading policy.
  - commands.rs reaches into the cached transcriber; startup depends on dictation for warmup.
- Move settings-window presentation out of dictation.
  - Tray and commands import the recording workflow just to open settings.
- Give profiling its own owner and empty-safe aggregates.
  - state.rs duplicates the entire empty snapshot and percentile logic for downloads.
- Share environment flag and paste-delay parsing.
  - lib/state and focus/insert each duplicate their respective parsing policy.
- Keep required external accommodations: platform permissions, paste-consumption delays,
  focus exclusions, silence thresholds, model validation, retry/backoff and worker fallback.
