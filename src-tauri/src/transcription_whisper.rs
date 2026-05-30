//! Local Whisper transcription engine for Spiel Phase 6.
//!
//! Invokes a configured whisper.cpp binary directly (no shell)
//! to transcribe WAV audio files. Output is parsed conservatively.
//!
//! Architecture:
//! - Uses `std::process::Command` for safe process invocation
//! - Validates binary and model paths before running
//! - Captures stdout as transcript text
//! - Returns structured errors for all failure modes

use crate::transcription::{
    EngineKind, TranscriptionEngine, TranscriptionRequest, TranscriptionResult, WhisperConfig,
};
use std::process::Command;
use std::time::Instant;

/// Local Whisper transcription engine.
///
/// Requires a configured whisper.cpp binary and GGML model file.
/// Invokes the binary directly — no shell, no network.
pub struct LocalWhisperEngine {
    pub config: WhisperConfig,
}

impl LocalWhisperEngine {
    /// Create a new engine with the given configuration.
    pub fn new(config: WhisperConfig) -> Self {
        Self { config }
    }

    /// Build the command arguments for whisper.cpp.
    /// Safe: no shell, arguments passed as a Vec.
    fn build_args(audio_path: &str, config: &WhisperConfig) -> Vec<String> {
        let mut args = vec![
            "-m".to_string(),
            config.model_path.clone(),
            "-f".to_string(),
            audio_path.to_string(),
            "--output-txt".to_string(),
            "--no-timestamps".to_string(),
        ];

        if let Some(ref lang) = config.language {
            if !lang.trim().is_empty() {
                args.push("-l".to_string());
                args.push(lang.trim().to_string());
            }
        }

        args
    }
}

impl TranscriptionEngine for LocalWhisperEngine {
    fn transcribe(&self, request: &TranscriptionRequest) -> Result<TranscriptionResult, String> {
        // Validate config
        self.config.validate()?;

        // Validate audio file
        let audio_path = std::path::Path::new(&request.audio_file_path);
        if !audio_path.exists() {
            return Err(format!(
                "Audio file not found: {}. Record audio first.",
                request.audio_file_path
            ));
        }

        let started_at = Instant::now();
        let args = Self::build_args(&request.audio_file_path, &self.config);

        // Execute whisper.cpp directly — no shell
        let output = Command::new(&self.config.binary_path)
            .args(&args)
            .output()
            .map_err(|e| {
                format!(
                    "Failed to run whisper binary '{}': {}. Is whisper.cpp installed and in your PATH?",
                    self.config.binary_path, e
                )
            })?;

        let elapsed = started_at.elapsed();
        let duration_ms = elapsed.as_millis() as u64;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output.status.code().unwrap_or(-1);
            return Err(format!(
                "Whisper process exited with code {}. Error: {}",
                exit_code,
                stderr.trim()
            ));
        }

        // Parse stdout as transcript text
        let raw_text = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if raw_text.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "Whisper produced no output. The audio may be silent or too short. Details: {}",
                stderr.trim()
            ));
        }

        let created_at = crate::chrono_now_iso();
        let model_name = std::path::Path::new(&self.config.model_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(TranscriptionResult {
            raw_text,
            engine_name: format!("Local Whisper ({})", model_name),
            engine_kind: EngineKind::LocalWhisper,
            audio_file_path: request.audio_file_path.clone(),
            duration_ms,
            is_mock: false,
            created_at,
            error: None,
        })
    }

    fn name(&self) -> &str {
        "Local Whisper"
    }

    fn kind(&self) -> EngineKind {
        EngineKind::LocalWhisper
    }
}
