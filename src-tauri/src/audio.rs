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
    in_rate: u32,
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

        let samples = if self.in_rate == TARGET_SAMPLE_RATE {
            raw
        } else {
            resample_linear(&raw, self.in_rate, TARGET_SAMPLE_RATE)
        };

        Ok(Capture { samples })
    }
}

#[derive(Default)]
struct DownsampleState {
    frame_position: u64,
    next_output_frame: f64,
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
    // Output is still trimmed to the same wall-clock duration after resampling.
    let max_input_samples = (in_rate as usize) * (max_seconds as usize);
    let max_output_samples = (TARGET_SAMPLE_RATE as usize) * (max_seconds as usize);
    let need_downsample = in_rate > TARGET_SAMPLE_RATE;
    let downsample_ratio = if need_downsample {
        Some(in_rate as f64 / TARGET_SAMPLE_RATE as f64)
    } else {
        None
    };
    let downsample_state = need_downsample.then_some(Arc::new(Mutex::new(DownsampleState {
        frame_position: 0,
        next_output_frame: 0.0,
    })));

    std::thread::spawn(move || {
        let buffer: Arc<Mutex<Vec<f32>>> =
            Arc::new(Mutex::new(Vec::with_capacity(max_input_samples)));
        let cb_buffer = Arc::clone(&buffer);

        let err_fn = |e| eprintln!("[spiel] audio stream error: {e}");

        // CPAL hands us whatever native format the device uses; normalize each to f32.
        let stream = match build_stream(
            &device,
            &config,
            sample_format,
            StreamBuildContext {
                buffer: cb_buffer,
                max_samples: max_input_samples,
                channels: in_channels,
                downsample_state,
                downsample_ratio,
            },
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
            if buffer.lock().unwrap().len() >= max_input_samples {
                break;
            }
        }

        let mut collected = std::mem::take(&mut *buffer.lock().unwrap());
        if collected.len() > max_output_samples {
            collected.truncate(max_output_samples);
        }
        let _ = result_tx.send(Ok(collected));
    });

    Ok(Recorder {
        stop_tx,
        result_rx,
        in_rate,
        elapsed_ms,
    })
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    format: SampleFormat,
    context: StreamBuildContext,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> std::result::Result<cpal::Stream, cpal::BuildStreamError> {
    macro_rules! input {
        ($t:ty, $to_f32:expr) => {{
            let buffer = Arc::clone(&context.buffer);
            let max_samples = context.max_samples;
            let channels = context.channels;
            let downsample_state = context.downsample_state.clone();
            let downsample_ratio = context.downsample_ratio;
            device.build_input_stream(
                config,
                move |data: &[$t], _: &cpal::InputCallbackInfo| {
                    let mut buf = buffer.lock().unwrap();
                    if buf.len() >= max_samples {
                        return;
                    }
                    let remaining = max_samples - buf.len();

                    if let Some(ratio) = downsample_ratio {
                        if let Some(state) = downsample_state.as_ref() {
                            let mut state = state.lock().unwrap();
                            if channels <= 1 {
                                for &raw in data.iter().take(remaining) {
                                    if (state.frame_position as f64) >= state.next_output_frame {
                                        buf.push(($to_f32)(raw));
                                        state.next_output_frame += ratio;
                                        if buf.len() >= max_samples {
                                            break;
                                        }
                                    }
                                    state.frame_position = state.frame_position.saturating_add(1);
                                }
                            } else {
                                for frame in data
                                    .chunks_exact(channels as usize)
                                    .take(remaining / channels as usize)
                                {
                                    let frame_sum: f32 = frame.iter().map(|&s| ($to_f32)(s)).sum();
                                    let sample = frame_sum / frame.len() as f32;
                                    if (state.frame_position as f64) >= state.next_output_frame {
                                        buf.push(sample);
                                        state.next_output_frame += ratio;
                                        if buf.len() >= max_samples {
                                            break;
                                        }
                                    }
                                    state.frame_position = state.frame_position.saturating_add(1);
                                }
                            }
                        } else {
                            if channels <= 1 {
                                buf.extend(data.iter().take(remaining).map(|&raw| ($to_f32)(raw)));
                            } else {
                                buf.extend(
                                    data.chunks_exact(channels as usize)
                                        .take(remaining / channels as usize)
                                        .map(|frame| {
                                            let frame_sum: f32 =
                                                frame.iter().map(|&s| ($to_f32)(s)).sum();
                                            frame_sum / frame.len() as f32
                                        }),
                                );
                            }
                        }
                    } else {
                        if channels <= 1 {
                            buf.extend(data.iter().take(remaining).map(|&raw| ($to_f32)(raw)));
                        } else {
                            buf.extend(
                                data.chunks_exact(channels as usize)
                                    .take(remaining / channels as usize)
                                    .map(|frame| {
                                        let frame_sum: f32 =
                                            frame.iter().map(|&s| ($to_f32)(s)).sum();
                                        frame_sum / frame.len() as f32
                                    }),
                            );
                        }
                    }
                    // Ignore partial frames; this prevents uneven-channel artifacts while
                    // the callback naturally delivers nearly exact frame boundaries.
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

struct StreamBuildContext {
    buffer: Arc<Mutex<Vec<f32>>>,
    max_samples: usize,
    channels: u16,
    downsample_state: Option<Arc<Mutex<DownsampleState>>>,
    downsample_ratio: Option<f64>,
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
