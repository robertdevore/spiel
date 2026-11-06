# Audio capture decision log

## Decision: one accumulator owns normalization, downsampling, and output limits

- `audio.rs` represented downsampling with independent optional ratio and state values even though they were always initialized together. The callback included a fallback for an impossible combination.
- Mono and interleaved-channel input repeated downmixing and capacity handling across downsampling branches.
- The mono downsampling branch limited input iteration by remaining output capacity; at high input rates this discarded frames near the duration cap and changed callback timing.
- Regression coverage must exercise the same accumulator used by the device callback, including frame limits, multiple callbacks, and incomplete channel frames.

Preserve sample conversion, nearest-frame downsampling, final low-rate interpolation, silence detection, and the configured duration cap. Keep platform capture and threading behavior unchanged.
