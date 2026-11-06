# Model lifecycle decision log

## Decision: model storage owns artifact names and deletion

Install inspection, downloads, and command-layer deletion independently construct final, partial, and checksum paths. A model-owned artifact value keeps these names together; `delete_files(model_dir, spec)` removes the model and its companions while preserving best-effort companion cleanup. Existing primary-file failure wording remains unchanged.

## Decision: classify download failures by explicit categories

Cancellation controls retry through the literal human message `canceled`; metrics inspect error substrings for checksum, HTTP, network, incomplete-download and unsafe-path failures. Typed categories decouple behavior from prose. Keep displayed error prefixes and existing category strings stable, including generic validation and unknown outcomes.

## Decision: remove unused command error-code mapper

`SpielError::code` had no production callers: command handlers return display strings, and download events use their separate category classifier. Remove the unused mapper and correct its command-conversion documentation; retain typed download categories and prose-independent tests.
