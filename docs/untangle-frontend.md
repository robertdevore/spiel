# Frontend untangling decisions

The baseline `src/main.ts` combines backend contracts, mutable application state,
command workflows, feature rendering, input controls, and event lifecycle in one
1,074-line module. The intended UI and backend command/event contracts remain the
same.

- Put each feature's rendering in a module with explicit snapshot and action
  parameters. Setup/readiness, model management, settings, status/transcript, and
  profiling change for different reasons but currently share implicit globals.
- Keep backend contracts together in `contracts.ts`; keep reusable DOM fields and
  formatting separate from application orchestration.
- Give download start/error cleanup one implementation: setup and model cards
  duplicate it around baseline lines 480 and 706.
- Give Accessibility requests one implementation: both cards duplicate the request
  and a 500ms refresh timer around lines 520 and 625. Refresh immediately and when
  the window regains focus, because elapsed time does not establish permission.
- Share model warmup command/error handling, while keeping each caller's existing
  success behavior (setup displays the returned message; settings refreshes status).
- Let backend config validation normalize language. The frontend's normalization
  at baseline line 977 duplicates `model::normalize_language_hint`. Browser number
  bounds and clamps remain transport guards: Rust deserializes unsigned integers
  before config validation, so negative or oversized values must be bounded before
  invocation to preserve existing behavior. Backend validation remains authoritative.
- Encapsulate recording elapsed-time bookkeeping separately from the DOM renderer.
  Keep the existing 250ms label-only update so recording does not rebuild all fields.

No optional comment or test cleanup is included. Existing command names, payloads,
card order, model confirmation, profile choices, and event semantics are retained.

## Verification

- `npm run build` passed (TypeScript strict checking and production Vite build).
- A temporary VM/DOM comparison harness rendered the original HEAD frontend and
  refactored frontend with identical fixtures. All three DOM comparisons passed:
  idle/model list, recording/transcript, and setup/errors/active download. The
  harness was `/tmp/spiel-ui-regression.cjs`; no production dependencies were added.
- Feature renderers contain no backend invocations or mutable application globals;
  their actions are supplied by the entry module.
- The same harness verified immediate Accessibility status refresh, failed-download
  feedback/optimistic-state cleanup, and refresh after window focus returns.
- Recording status returns an explicit label-update handle; orchestration does not
  query or depend on the feature renderer's internal DOM structure.
