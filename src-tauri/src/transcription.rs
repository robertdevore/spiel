//! Transcription abstraction for Spiel Phase 5.
//!
//! Defines types, an engine trait, and engine implementations.
//!
//! Architecture:
//! - `TranscriptionEngine` trait: common interface for all engines
//! - `MockEngine`: returns mock text for testing (Phase 5)
//! - `LocalWhisperEngine`: local whisper.cpp via CLI (Phase 6)
//! - `OpenAiTranscriptionEngine`: cloud transcription via OpenAI API (Phase 12)

use serde::{Deserialize, Serialize};

/// Kinds of transcription engines available or planned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    /// Mock engine — returns labeled placeholder text (Phase 5)
    Mock,
    /// Local whisper.cpp engine (Phase 6)
    LocalWhisper,
    /// OpenAI cloud transcription (Phase 12) — optional, requires API key
    OpenAi,
}

impl EngineKind {
    /// Whether this engine is currently implemented.
    pub fn is_implemented(&self) -> bool {
        matches!(self, Self::Mock | Self::LocalWhisper | Self::OpenAi)
    }

    /// Human-readable label.
    pub fn label(&self) -> &str {
        match self {
            Self::Mock => "Mock Engine",
            Self::LocalWhisper => "Local Whisper",
            Self::OpenAi => "OpenAI (cloud)",
        }
    }
}

/// Available engine descriptors for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineInfo {
    pub kind: EngineKind,
    pub label: String,
    pub implemented: bool,
}

/// Current transcription status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionStatus {
    Idle,
    Pending,
    Transcribing,
    Complete,
    Error,
}

/// Request to transcribe an audio file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionRequest {
    pub audio_file_path: String,
    pub engine: EngineKind,
}

/// Result of a transcription operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub raw_text: String,
    pub engine_name: String,
    pub engine_kind: EngineKind,
    pub audio_file_path: String,
    pub duration_ms: u64,
    pub is_mock: bool,
    pub created_at: String,
    pub error: Option<String>,
}

/// Status snapshot for the get_transcription_status command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionState {
    pub status: TranscriptionStatus,
    pub last_result: Option<TranscriptionResult>,
    pub error: Option<String>,
    pub available_engines: Vec<EngineInfo>,
}

impl Default for TranscriptionState {
    fn default() -> Self {
        Self {
            status: TranscriptionStatus::Idle,
            last_result: None,
            error: None,
            available_engines: EngineKind::all_info(),
        }
    }
}

impl EngineKind {
    /// Returns info for all known engine kinds.
    pub fn all_info() -> Vec<EngineInfo> {
        vec![
            EngineInfo {
                kind: EngineKind::Mock,
                label: "Mock Engine".into(),
                implemented: true,
            },
            EngineInfo {
                kind: EngineKind::LocalWhisper,
                label: "Local Whisper".into(),
                implemented: true,
            },
            EngineInfo {
                kind: EngineKind::OpenAi,
                label: "OpenAI (cloud)".into(),
                implemented: true,
            },
        ]
    }
}

/// Trait for transcription engines.
/// Future engines (whisper.cpp, cloud) implement this trait.
pub trait TranscriptionEngine: Send + Sync {
    /// Transcribe the given audio file.
    fn transcribe(&self, request: &TranscriptionRequest) -> Result<TranscriptionResult, String>;

    /// Human-readable name of this engine.
    fn name(&self) -> &str;

    /// The kind of this engine.
    fn kind(&self) -> EngineKind;
}

/// Mock transcription engine for Phase 5.
///
/// Validates that the audio file path exists, then returns a clearly
/// labeled mock transcript. Never reads audio data or calls external APIs.
pub struct MockEngine;

impl TranscriptionEngine for MockEngine {
    fn transcribe(&self, request: &TranscriptionRequest) -> Result<TranscriptionResult, String> {
        // Validate the file exists
        let path = std::path::Path::new(&request.audio_file_path);
        if !path.exists() {
            return Err(format!(
                "Audio file not found: {}. Record audio first, then try transcribing.",
                request.audio_file_path
            ));
        }

        // Get file metadata for duration estimate (crude: size-based)
        let size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        // Rough estimate: 16-bit mono at 44100 Hz = 88200 bytes/sec
        let estimated_ms = if size_bytes > 0 {
            (size_bytes as f64 / 88.2) as u64
        } else {
            0
        };

        let created_at = crate::chrono_now_iso();

        Ok(TranscriptionResult {
            raw_text: "This is a mock transcript generated by Spiel's Phase 5 transcription abstraction. Real speech-to-text is not implemented yet.\n\nWhen real transcription engines are added (whisper.cpp, cloud), your actual spoken words will appear here.".into(),
            engine_name: self.name().into(),
            engine_kind: self.kind(),
            audio_file_path: request.audio_file_path.clone(),
            duration_ms: estimated_ms,
            is_mock: true,
            created_at,
            error: None,
        })
    }

    fn name(&self) -> &str {
        "Mock Engine"
    }

    fn kind(&self) -> EngineKind {
        EngineKind::Mock
    }
}

/// Configuration for the local Whisper engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhisperConfig {
    /// Path to the whisper.cpp binary
    pub binary_path: String,
    /// Path to the GGML model file
    pub model_path: String,
    /// Optional language code (e.g., "en")
    pub language: Option<String>,
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self {
            binary_path: String::new(),
            model_path: String::new(),
            language: None,
        }
    }
}

impl WhisperConfig {
    /// Validate that the binary and model paths exist.
    pub fn validate(&self) -> Result<(), String> {
        if self.binary_path.trim().is_empty() {
            return Err("Whisper binary path is not configured. Set the path to your local whisper.cpp binary.".into());
        }
        let bp = std::path::Path::new(&self.binary_path);
        if !bp.exists() {
            return Err(format!(
                "Whisper binary not found at: {}. Please verify the path.",
                self.binary_path
            ));
        }
        if !bp.is_file() {
            return Err(format!(
                "Whisper binary path is not a file: {}",
                self.binary_path
            ));
        }

        if self.model_path.trim().is_empty() {
            return Err("Whisper model path is not configured. Set the path to your GGML model file (e.g., ggml-base.en.bin).".into());
        }
        let mp = std::path::Path::new(&self.model_path);
        if !mp.exists() {
            return Err(format!(
                "Whisper model not found at: {}. Please verify the path.",
                self.model_path
            ));
        }

        Ok(())
    }
}
pub fn run_transcription(
    engine: &dyn TranscriptionEngine,
    request: &TranscriptionRequest,
) -> Result<TranscriptionResult, String> {
    // Validate engine is supported
    if !request.engine.is_implemented() {
        return Err(format!(
            "The '{}' engine is not implemented yet. Mock and Local Whisper engines are available in Phase 6.",
            request.engine.label()
        ));
    }

    engine.transcribe(request)
}
