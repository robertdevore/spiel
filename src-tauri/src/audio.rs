//! Microphone capture.
//!
//! Captures from the default input device, downmixes to mono, and resamples to the
//! 16 kHz f32 PCM that Whisper expects — all in memory. **No WAV files are ever
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

/// A finished capture: mono 16 kHz f32 samples, ready for Whisper.
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
    result_rx: mpsc::Receiver<Vec<f32>>,
    in_rate: u32,
    in_channels: u16,
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

    /// Stop recording and return resampled mono 16 kHz samples. Consumes the recorder.
    pub fn finish(self) -> Result<Capture> {
        let _ = self.stop_tx.send(());
        let raw = self
            .result_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .map_err(|_| SpielError::Audio("recording thread did not return in time".into()))?;

        let mono = downmix_to_mono(&raw, self.in_channels);
        let samples = resample_linear(&mono, self.in_rate, TARGET_SAMPLE_RATE);

        Ok(Capture { samples })
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
    let (result_tx, result_rx) = mpsc::channel::<Vec<f32>>();
    let started = Instant::now();
    let elapsed_ms = Arc::new(Mutex::new(0u64));
    let elapsed_clone = Arc::clone(&elapsed_ms);

    // Cap interleaved samples so a forgotten recording can't grow without bound.
    let max_samples = (in_rate as usize) * (in_channels as usize) * (max_seconds as usize);

    std::thread::spawn(move || {
        let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let cb_buffer = Arc::clone(&buffer);

        let err_fn = |e| eprintln!("[spiel] audio stream error: {e}");

        // CPAL hands us whatever native format the device uses; normalize each to f32.
        let stream = match build_stream(
            &device,
            &config,
            sample_format,
            cb_buffer,
            max_samples,
            err_fn,
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[spiel] failed to build input stream: {e}");
                let _ = result_tx.send(Vec::new());
                return;
            }
        };

        if let Err(e) = stream.play() {
            eprintln!("[spiel] failed to start stream: {e}");
            let _ = result_tx.send(Vec::new());
            return;
        }

        loop {
            {
                let mut e = elapsed_clone.lock().unwrap();
                *e = started.elapsed().as_millis() as u64;
            }
            if stop_rx
                .recv_timeout(std::time::Duration::from_millis(50))
                .is_ok()
            {
                break;
            }
            // Hard cap reached: stop on our own.
            if buffer.lock().unwrap().len() >= max_samples {
                break;
            }
        }

        let collected = std::mem::take(&mut *buffer.lock().unwrap());
        let _ = result_tx.send(collected);
    });

    Ok(Recorder {
        stop_tx,
        result_rx,
        in_rate,
        in_channels,
        elapsed_ms,
    })
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    format: SampleFormat,
    buffer: Arc<Mutex<Vec<f32>>>,
    max_samples: usize,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> std::result::Result<cpal::Stream, cpal::BuildStreamError> {
    macro_rules! input {
        ($t:ty, $to_f32:expr) => {{
            let buffer = Arc::clone(&buffer);
            device.build_input_stream(
                config,
                move |data: &[$t], _: &cpal::InputCallbackInfo| {
                    let mut buf = buffer.lock().unwrap();
                    if buf.len() >= max_samples {
                        return;
                    }
                    buf.extend(data.iter().map(|&s| ($to_f32)(s)));
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

/// Average interleaved channels down to mono.
fn downmix_to_mono(interleaved: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    let ch = channels as usize;
    interleaved
        .chunks(ch)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

/// Resample mono audio to `out_rate`. Downsampling averages each source window, which
/// doubles as a crude anti-alias low-pass — adequate for speech going into Whisper.
fn resample_linear(input: &[f32], in_rate: u32, out_rate: u32) -> Vec<f32> {
    if in_rate == out_rate || input.is_empty() {
        return input.to_vec();
    }
    let ratio = in_rate as f64 / out_rate as f64;
    let out_len = (input.len() as f64 / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let start = (i as f64 * ratio) as usize;
        let end = (((i + 1) as f64 * ratio) as usize)
            .min(input.len())
            .max(start + 1);
        let window = &input[start..end];
        out.push(window.iter().sum::<f32>() / window.len() as f32);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_averages_stereo() {
        let stereo = [1.0, 0.0, 0.5, 0.5];
        assert_eq!(downmix_to_mono(&stereo, 2), vec![0.5, 0.5]);
    }

    #[test]
    fn downmix_passthrough_mono() {
        let mono = [0.1, 0.2, 0.3];
        assert_eq!(downmix_to_mono(&mono, 1), vec![0.1, 0.2, 0.3]);
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
