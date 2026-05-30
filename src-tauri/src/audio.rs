//! Audio recording module for Spiel.
//! Uses CPAL for cross-platform microphone capture and hound for WAV output.
//!
//! Architecture:
//! - `start_recording()` spawns a background thread that collects samples
//!   into a shared buffer. The CPAL stream stays on that thread.
//! - `stop_recording()` signals the thread to stop, collects the buffer,
//!   and writes a WAV file.
//! - Communication via channels because CPAL streams are not `Send`.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Errors that can occur during audio recording.
#[derive(Debug, Clone)]
pub enum RecordingError {
    NoInputDevice,
    ConfigError(String),
    StreamError(String),
    AlreadyRecording,
    NotRecording,
    FileError(String),
    DeviceError(String),
}

impl std::fmt::Display for RecordingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoInputDevice => write!(f, "No microphone or input device found. Please connect a microphone and try again."),
            Self::ConfigError(e) => write!(f, "Audio configuration error: {}", e),
            Self::StreamError(e) => write!(f, "Audio stream error: {}", e),
            Self::AlreadyRecording => write!(f, "Recording is already in progress. Stop the current recording before starting a new one."),
            Self::NotRecording => write!(f, "No recording is in progress."),
            Self::FileError(e) => write!(f, "File error: {}", e),
            Self::DeviceError(e) => write!(f, "Microphone error: {}", e),
        }
    }
}

/// Metadata about a completed recording.
#[derive(Debug, Clone)]
pub struct RecordingMeta {
    pub file_path: String,
    pub filename: String,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub size_bytes: u64,
    pub created_at: String,
    pub device_name: Option<String>,
}

/// Handle for controlling an active recording session.
/// Uses channels to communicate with the recording thread.
pub struct RecordingHandle {
    /// Send a signal to stop the recording thread
    stop_tx: mpsc::Sender<()>,
    /// Receive the collected samples when the thread finishes
    samples_rx: mpsc::Receiver<Vec<i16>>,
    /// When recording started
    started_at: Instant,
    /// Audio configuration
    config: cpal::StreamConfig,
    /// Device name
    device_name: Option<String>,
    /// Shared elapsed time (updated by the recording thread)
    elapsed: Arc<Mutex<u64>>,
}

/// If the handle is dropped without calling stop(),
/// signal the recording thread to stop to prevent resource leaks.
impl Drop for RecordingHandle {
    fn drop(&mut self) {
        let _ = self.stop_tx.send(());
    }
}

impl RecordingHandle {
    /// Returns the elapsed duration since recording started.
    pub fn elapsed_ms(&self) -> u64 {
        *self.elapsed.lock().unwrap()
    }

    /// Stops recording and writes the WAV file.
    /// Consumes the handle.
    pub fn stop(self) -> Result<RecordingMeta, RecordingError> {
        // Signal the recording thread to stop
        let _ = self.stop_tx.send(());

        // Wait for the samples (with timeout)
        let samples = self
            .samples_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .map_err(|_| {
                RecordingError::StreamError(
                    "Timed out waiting for recording thread to finish".into(),
                )
            })?;

        let elapsed = self.started_at.elapsed();
        let duration_ms = elapsed.as_millis() as u64;

        // Write WAV file
        let file_path = create_temp_wav_path();
        let filename = std::path::Path::new(&file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("recording.wav")
            .to_string();

        let spec = hound::WavSpec {
            channels: self.config.channels,
            sample_rate: self.config.sample_rate.0,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(&file_path, spec)
            .map_err(|e| RecordingError::FileError(e.to_string()))?;

        for sample in &samples {
            writer
                .write_sample(*sample)
                .map_err(|e| RecordingError::FileError(e.to_string()))?;
        }

        writer
            .finalize()
            .map_err(|e| RecordingError::FileError(e.to_string()))?;

        let size_bytes = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);

        let created_at = crate::chrono_now_iso();

        Ok(RecordingMeta {
            file_path,
            filename,
            duration_ms,
            sample_rate: self.config.sample_rate.0,
            channels: self.config.channels,
            size_bytes,
            created_at,
            device_name: self.device_name.clone(),
        })
    }
}

/// Start recording from the default input device.
/// Spawns a background thread for audio capture.
pub fn start_recording() -> Result<RecordingHandle, RecordingError> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or(RecordingError::NoInputDevice)?;

    let device_name = device.name().ok();

    let config = device
        .default_input_config()
        .map_err(|e| RecordingError::ConfigError(e.to_string()))?;

    let stream_config: cpal::StreamConfig = config.into();

    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let (samples_tx, samples_rx) = mpsc::channel::<Vec<i16>>();

    let started_at = Instant::now();
    let elapsed = Arc::new(Mutex::new(0u64));
    let elapsed_clone = Arc::clone(&elapsed);

    // Spawn recording thread — this thread owns the CPAL stream
    let config_clone = stream_config.clone();
    std::thread::spawn(move || {
        let samples: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
        let samples_clone = Arc::clone(&samples);

        let stream = match device.build_input_stream(
            &config_clone,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut buf = samples_clone.lock().unwrap();
                for &sample in data {
                    let clamped = sample.clamp(-1.0, 1.0);
                    buf.push((clamped * 32767.0) as i16);
                }
            },
            |err| {
                eprintln!("Audio capture error: {}", err);
            },
            None,
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to build input stream: {}", e);
                let _ = samples_tx.send(Vec::new());
                return;
            }
        };

        if let Err(e) = stream.play() {
            eprintln!("Failed to start stream: {}", e);
            let _ = samples_tx.send(Vec::new());
            return;
        }

        // Update elapsed time periodically
        let start = Instant::now();
        loop {
            {
                let mut e = elapsed_clone.lock().unwrap();
                *e = start.elapsed().as_millis() as u64;
            }

            // Check for stop signal (non-blocking)
            if stop_rx.try_recv().is_ok() {
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // Collect and send samples
        let collected = {
            let mut buf = samples.lock().unwrap();
            std::mem::take(&mut *buf)
        };
        let _ = samples_tx.send(collected);
    });

    Ok(RecordingHandle {
        stop_tx,
        samples_rx,
        started_at,
        config: stream_config,
        device_name,
        elapsed,
    })
}

/// Create a path for a temporary WAV file in the system temp directory.
fn create_temp_wav_path() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let tmp = std::env::temp_dir();
    format!("{}/spiel_recording_{}.wav", tmp.display(), ts)
}
