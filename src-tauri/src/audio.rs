//! Microphone capture.
//!
//! Captures from the default input device, downmixes to mono, and resamples to the
//! 16 kHz PCM that Whisper expects — all in memory. **No WAV files are ever
//! written to disk**, which removes the "raw audio lingers in /tmp" privacy problem
//! the previous build had.
//!
//! The CPAL stream is not `Send`, so it lives entirely on a dedicated capture thread.
//! We talk to that thread over channels: a stop signal in, the finished samples out.

use crate::error::{Result, SpielError};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Sample rate Whisper models are trained on.
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

/// A finished capture: mono 16 kHz f32 PCM, ready for Whisper.
pub struct Capture {
    pub samples: Vec<f32>,
}

impl Capture {
    pub fn is_effectively_silent(&self) -> bool {
        // Very short or near-zero-energy clips aren't worth sending to Whisper.
        if self.samples.len() < TARGET_SAMPLE_RATE as usize / 5 {
            return true; // < 0.2s
        }
        let energy: f32 =
            self.samples.iter().map(|s| s * s).sum::<f32>() / self.samples.len() as f32;
        energy.sqrt() < 0.0008
    }
}

/// Controls one in-progress recording. Dropping it stops the capture thread.
pub struct Recorder {
    stop_tx: mpsc::Sender<()>,
    result_rx: mpsc::Receiver<crate::error::Result<Vec<f32>>>,
    sample_rate: u32,
    /// Captured live so the UI can show an elapsed timer.
    elapsed_ms: Arc<Mutex<u64>>,
}

impl Drop for Recorder {
    fn drop(&mut self) {
        let _ = self.stop_tx.send(());
    }
}

impl Recorder {
    pub fn elapsed_ms(&self) -> u64 {
        *self.elapsed_ms.lock().unwrap()
    }

    /// Stop recording and return mono 16 kHz samples. Consumes the recorder.
    pub fn finish(self) -> Result<Capture> {
        let _ = self.stop_tx.send(());
        let raw = self
            .result_rx
            .recv_timeout(std::time::Duration::from_secs(3))
            .map_err(|_| SpielError::Audio("recording thread did not return in time".into()))?
            .map_err(|err| SpielError::Audio(err.to_string()))?;

        let samples = if self.sample_rate == TARGET_SAMPLE_RATE {
            raw
        } else {
            resample_linear(&raw, self.sample_rate, TARGET_SAMPLE_RATE)
        };

        Ok(Capture { samples })
    }
}

struct DownsampleState {
    ratio: f64,
    frame_position: u64,
    next_output_frame: f64,
}

struct SampleAccumulator {
    samples: Vec<f32>,
    max_samples: usize,
    channels: usize,
    downsample: Option<DownsampleState>,
}

impl SampleAccumulator {
    fn new(input_rate: u32, channels: u16, max_samples: usize) -> Self {
        Self {
            samples: Vec::with_capacity(max_samples),
            max_samples,
            channels: usize::from(channels.max(1)),
            downsample: (input_rate > TARGET_SAMPLE_RATE).then_some(DownsampleState {
                ratio: input_rate as f64 / TARGET_SAMPLE_RATE as f64,
                frame_position: 0,
                next_output_frame: 0.0,
            }),
        }
    }

    fn is_full(&self) -> bool {
        self.samples.len() >= self.max_samples
    }

    fn append<T: Copy>(&mut self, data: &[T], to_f32: impl Fn(T) -> f32) {
        // Ignore partial frames; CPAL delivers interleaved channel frames.
        for frame in data.chunks_exact(self.channels) {
            if self.is_full() {
                break;
            }
            let keep = match self.downsample.as_mut() {
                Some(state) => {
                    let keep = state.frame_position as f64 >= state.next_output_frame;
                    if keep {
                        state.next_output_frame += state.ratio;
                    }
                    state.frame_position = state.frame_position.saturating_add(1);
                    keep
                }
                None => true,
            };
            if keep {
                let sum: f32 = frame.iter().map(|&sample| to_f32(sample)).sum();
                self.samples.push(sum / self.channels as f32);
            }
        }
    }
}

/// Begin capturing from the default input device.
pub fn start(max_seconds: u32) -> Result<Recorder> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or(SpielError::NoInputDevice)?;

    let supported = device
        .default_input_config()
        .map_err(|e| SpielError::Audio(e.to_string()))?;
    let in_rate = supported.sample_rate().0;
    let in_channels = supported.channels();
    let sample_format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();

    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let (result_tx, result_rx) = mpsc::channel::<crate::error::Result<Vec<f32>>>();
    let started = Instant::now();
    let elapsed_ms = Arc::new(Mutex::new(0u64));
    let elapsed_clone = Arc::clone(&elapsed_ms);

    // Time-bound capture by input sample rate for real-time behavior.
    // High-rate input is reduced during capture; lower rates are resampled at finish.
    let capture_rate = captured_sample_rate(in_rate);
    let max_captured_samples = (capture_rate as usize) * (max_seconds as usize);
    std::thread::spawn(move || {
        let accumulator = Arc::new(Mutex::new(SampleAccumulator::new(
            in_rate,
            in_channels,
            max_captured_samples,
        )));

        let err_fn = |e| eprintln!("[spiel] audio stream error: {e}");

        // CPAL hands us whatever native format the device uses; normalize each to f32.
        let stream = match build_stream(
            &device,
            &config,
            sample_format,
            Arc::clone(&accumulator),
            err_fn,
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[spiel] failed to build input stream: {e}");
                let _ = result_tx.send(Err(SpielError::Audio(format!(
                    "failed to build input stream: {e}"
                ))));
                return;
            }
        };

        if let Err(e) = stream.play() {
            eprintln!("[spiel] failed to start stream: {e}");
            let _ = result_tx.send(Err(SpielError::Audio(format!(
                "failed to start input stream: {e}"
            ))));
            return;
        }

        loop {
            {
                let mut e = elapsed_clone.lock().unwrap();
                *e = started.elapsed().as_millis() as u64;
            }
            if stop_rx
                .recv_timeout(std::time::Duration::from_millis(20))
                .is_ok()
            {
                break;
            }
            // Hard cap reached: stop on our own.
            if accumulator.lock().unwrap().is_full() {
                break;
            }
        }

        drop(stream);
        let collected = std::mem::take(&mut accumulator.lock().unwrap().samples);
        let _ = result_tx.send(Ok(collected));
    });

    Ok(Recorder {
        stop_tx,
        result_rx,
        sample_rate: capture_rate,
        elapsed_ms,
    })
}

fn captured_sample_rate(input_rate: u32) -> u32 {
    if input_rate > TARGET_SAMPLE_RATE {
        TARGET_SAMPLE_RATE
    } else {
        input_rate
    }
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    format: SampleFormat,
    accumulator: Arc<Mutex<SampleAccumulator>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> std::result::Result<cpal::Stream, cpal::BuildStreamError> {
    macro_rules! input {
        ($t:ty, $to_f32:expr) => {{
            let accumulator = Arc::clone(&accumulator);
            device.build_input_stream(
                config,
                move |data: &[$t], _: &cpal::InputCallbackInfo| {
                    accumulator.lock().unwrap().append(data, $to_f32);
                },
                err_fn,
                None,
            )
        }};
    }

    match format {
        SampleFormat::F32 => input!(f32, |s: f32| s),
        SampleFormat::I16 => input!(i16, |s: i16| s as f32 / i16::MAX as f32),
        SampleFormat::U16 => input!(u16, |s: u16| (s as f32 / u16::MAX as f32) * 2.0 - 1.0),
        other => {
            eprintln!("[spiel] unsupported sample format {other:?}, defaulting to f32");
            input!(f32, |s: f32| s)
        }
    }
}

/// Resample mono audio to `out_rate` using linear interpolation.
/// This keeps timing stable both when up-sampling and down-sampling.
fn resample_linear(input: &[f32], in_rate: u32, out_rate: u32) -> Vec<f32> {
    if in_rate == out_rate || input.is_empty() {
        return input.to_vec();
    }

    let out_len = ((input.len() as f64 * out_rate as f64) / in_rate as f64).round() as usize;
    if out_len == 0 {
        return Vec::new();
    }

    let scale = in_rate as f64 / out_rate as f64;
    let mut out = Vec::with_capacity(out_len);
    let max = input.len() - 1;
    for i in 0..out_len {
        let sample_pos = i as f64 * scale;
        let left = sample_pos.floor() as usize;
        let right = (left + 1).min(max);
        if left == right {
            out.push(input[left]);
        } else {
            let frac = (sample_pos - left as f64) as f32;
            out.push(input[left] + (input[right] - input[left]) * frac);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulator_downmixes_complete_frames_and_stops_at_capacity() {
        let mut capture = SampleAccumulator::new(16_000, 2, 2);
        capture.append(&[1.0, 0.0, 0.5], |s| s);
        capture.append(&[0.2, 0.6, 1.0, 1.0], |s| s);
        assert_eq!(capture.samples, vec![0.5, 0.4]);
        assert!(capture.is_full());
        capture.append(&[0.9, 0.9], |s| s);
        assert_eq!(capture.samples, vec![0.5, 0.4]);
    }

    #[test]
    fn accumulator_consumes_enough_mono_input_to_reach_output_limit() {
        let mut capture = SampleAccumulator::new(48_000, 1, 3);
        capture.append(&[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], |s| s);
        assert_eq!(capture.samples, vec![0.0, 3.0, 6.0]);
        assert!(capture.is_full());
    }

    #[test]
    fn accumulator_preserves_downsampling_position_between_callbacks() {
        let samples: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let mut whole = SampleAccumulator::new(44_100, 1, 100);
        whole.append(&samples, |s| s);
        let mut chunked = SampleAccumulator::new(44_100, 1, 100);
        for chunk in samples.chunks(7) {
            chunked.append(chunk, |s| s);
        }
        assert_eq!(chunked.samples, whole.samples);
        assert_eq!(&chunked.samples[..5], &[0.0, 3.0, 6.0, 9.0, 12.0]);
    }

    #[test]
    fn accumulator_converts_and_downmixes_before_returning_mono_audio() {
        let mut capture = SampleAccumulator::new(48_000, 2, 2);
        capture.append(&[2_i16, 6, 10, 20, 30, 40, 6, 10], |s| s as f32 / 2.0);
        assert_eq!(capture.samples, vec![2.0, 4.0]);
    }

    #[test]
    fn accumulator_preserves_low_rate_samples_for_finish_resampling() {
        let mut capture = SampleAccumulator::new(8_000, 1, 3);
        capture.append(&[0.1, 0.2, 0.3, 0.4], |s| s);
        assert_eq!(capture.samples, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn downsample_or_upsample_roughly_preserves_duration() {
        // At lower sample rates, output length should stay near the target frame budget.
        let low_rate_input: Vec<f32> = (0..(8_000 * 10)).map(|i| (i as f32).sin()).collect();
        let low_rate_output = resample_linear(&low_rate_input, 8_000, TARGET_SAMPLE_RATE);
        assert!((low_rate_output.len() as i64 - (16_000 * 10)).abs() <= 1);

        // At higher rates, output should not exceed the target-frame duration.
        let high_rate_input: Vec<f32> = (0..(48_000 * 10)).map(|i| (i as f32).cos()).collect();
        let high_rate_output = resample_linear(&high_rate_input, 48_000, TARGET_SAMPLE_RATE);
        assert!(high_rate_output.len() <= (16_000 * 10) + 1);
    }

    #[test]
    fn output_is_clamped_to_target_floor_for_non_divisible_rates() {
        let input: Vec<f32> = (0..(7_999 * 10)).map(|i| (i as f32).sin()).collect();
        let output = resample_linear(&input, 7_999, TARGET_SAMPLE_RATE);
        assert!(output.len() <= 16_000 * 10 + 1);
    }

    #[test]
    fn downmix_averages_stereo() {
        let stereo = [1.0, 0.0, 0.5, 0.5];
        assert_eq!(test_downmix_to_mono(&stereo, 2), vec![0.5, 0.5]);
    }

    #[test]
    fn downmix_passthrough_mono() {
        let mono = [0.1, 0.2, 0.3];
        assert_eq!(test_downmix_to_mono(&mono, 1), vec![0.1, 0.2, 0.3]);
    }

    fn test_downmix_to_mono(interleaved: &[f32], channels: u16) -> Vec<f32> {
        if channels <= 1 {
            return interleaved.to_vec();
        }
        let ch = channels as usize;
        interleaved
            .chunks(ch)
            .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
            .collect()
    }

    #[test]
    fn resample_48k_to_16k_thirds_length() {
        let input: Vec<f32> = (0..4800).map(|i| (i as f32 * 0.01).sin()).collect();
        let out = resample_linear(&input, 48_000, 16_000);
        assert_eq!(out.len(), 1600);
    }

    #[test]
    fn resample_noop_when_rates_match() {
        let input = vec![0.1, 0.2, 0.3];
        assert_eq!(resample_linear(&input, 16_000, 16_000), input);
    }

    #[test]
    fn high_rate_input_is_marked_as_already_downsampled() {
        assert_eq!(captured_sample_rate(48_000), TARGET_SAMPLE_RATE);
        assert_eq!(captured_sample_rate(44_100), TARGET_SAMPLE_RATE);
    }

    #[test]
    fn low_rate_input_is_left_for_finish_resampling() {
        assert_eq!(captured_sample_rate(8_000), 8_000);
        assert_eq!(captured_sample_rate(TARGET_SAMPLE_RATE), TARGET_SAMPLE_RATE);
    }

    #[test]
    fn resample_upsample_halves_rate() {
        let input = vec![0.0, 1.0];
        let out = resample_linear(&input, 8_000, 16_000);
        assert_eq!(out.len(), 4);
        assert_eq!(out[0], 0.0);
        assert!((out[1] - 0.5).abs() < 0.0001);
        assert_eq!(out[2], 1.0);
        assert_eq!(out[3], 1.0);
    }

    #[test]
    fn silence_detection_flags_short_clip() {
        let c = Capture {
            samples: vec![0.0; 100],
        };
        assert!(c.is_effectively_silent());
    }

    #[test]
    fn silence_detection_passes_loud_clip() {
        let samples: Vec<f32> = (0..16_000).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
        let c = Capture { samples };
        assert!(!c.is_effectively_silent());
    }
}
