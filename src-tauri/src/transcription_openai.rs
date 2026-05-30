//! OpenAI transcription engine for Spiel Phase 12.
//!
//! Sends audio files to OpenAI's speech-to-text API.
//! Requires: local-only mode disabled, cloud providers enabled, API key configured.
//! Uses reqwest with rustls-tls (no system OpenSSL dependency).
//!
//! Architecture:
//! - Implements `TranscriptionEngine` trait
//! - Sends multipart form data to POST https://api.openai.com/v1/audio/transcriptions
//! - Uses "whisper-1" model (configurable via settings)
//! - Validates prerequisites before any network call
//! - No automatic retry, no fallback to local engines

use crate::transcription::{
    EngineKind, TranscriptionEngine, TranscriptionRequest, TranscriptionResult,
};
use std::time::Instant;

/// Default OpenAI transcription model.
const DEFAULT_OPENAI_MODEL: &str = "whisper-1";
/// OpenAI transcription API endpoint.
const OPENAI_TRANSCRIPTION_URL: &str = "https://api.openai.com/v1/audio/transcriptions";

/// OpenAI-powered transcription engine.
/// Requires explicit opt-in: local-only mode off, API key configured.
pub struct OpenAiTranscriptionEngine {
    api_key: String,
    model: String,
}

impl OpenAiTranscriptionEngine {
    /// Create a new OpenAI transcription engine.
    /// The api_key must be validated before construction.
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            api_key,
            model: model.unwrap_or_else(|| DEFAULT_OPENAI_MODEL.to_string()),
        }
    }

    /// Build the HTTP client for OpenAI API calls.
    fn build_client() -> Result<reqwest::blocking::Client, String> {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))
    }
}

impl TranscriptionEngine for OpenAiTranscriptionEngine {
    fn transcribe(&self, request: &TranscriptionRequest) -> Result<TranscriptionResult, String> {
        // Validate audio file
        let audio_path = std::path::Path::new(&request.audio_file_path);
        if !audio_path.exists() {
            return Err(format!(
                "Audio file not found: {}. Record audio first.",
                request.audio_file_path
            ));
        }

        // Check file size (OpenAI limit: 25MB)
        let file_meta = std::fs::metadata(audio_path).map_err(|e| {
            format!(
                "Cannot read audio file metadata: {}. The file may have been moved or deleted.",
                e
            )
        })?;

        let max_bytes: u64 = 25 * 1024 * 1024;
        if file_meta.len() > max_bytes {
            return Err(format!(
                "Audio file is too large for OpenAI transcription ({} MB). Maximum is 25 MB. Try a shorter recording.",
                file_meta.len() / (1024 * 1024)
            ));
        }

        // Read audio file
        let audio_data =
            std::fs::read(audio_path).map_err(|e| format!("Failed to read audio file: {}", e))?;

        // Determine filename from path
        let filename = audio_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("recording.wav")
            .to_string();

        let started_at = Instant::now();

        // Build multipart form
        let part = reqwest::blocking::multipart::Part::bytes(audio_data)
            .file_name(filename)
            .mime_str("audio/wav")
            .map_err(|e| format!("Failed to build request: {}", e))?;

        let form = reqwest::blocking::multipart::Form::new()
            .text("model", self.model.clone())
            .text("response_format", "text")
            .part("file", part);

        // Send request
        let client = Self::build_client()?;
        let response = client
            .post(OPENAI_TRANSCRIPTION_URL)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    "OpenAI request timed out after 120 seconds. Check your network connection or try a shorter recording.".into()
                } else if e.is_connect() {
                    "Cannot connect to OpenAI API. Check your internet connection.".into()
                } else {
                    format!("OpenAI request failed: {}", e)
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().unwrap_or_else(|_| "Unknown error".into());
            if status.as_u16() == 401 {
                return Err(
                    "OpenAI API key is invalid or expired. Check your API key in Settings.".into(),
                );
            }
            if status.as_u16() == 429 {
                return Err("OpenAI rate limit reached. Wait a moment and try again.".into());
            }
            return Err(format!(
                "OpenAI returned error {}: {}",
                status.as_u16(),
                error_body
            ));
        }

        let raw_text = response
            .text()
            .map_err(|e| format!("Failed to read OpenAI response: {}", e))?;

        if raw_text.trim().is_empty() {
            return Err(
                "OpenAI returned an empty transcript. The audio may have no detectable speech."
                    .into(),
            );
        }

        let duration_ms = started_at.elapsed().as_millis() as u64;
        let created_at = crate::chrono_now_iso();

        Ok(TranscriptionResult {
            raw_text,
            engine_name: self.name().into(),
            engine_kind: self.kind(),
            audio_file_path: request.audio_file_path.clone(),
            duration_ms,
            is_mock: false,
            created_at,
            error: None,
        })
    }

    fn name(&self) -> &str {
        "OpenAI Transcription"
    }

    fn kind(&self) -> EngineKind {
        EngineKind::OpenAi
    }
}
