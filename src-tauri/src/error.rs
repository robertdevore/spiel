//! One error type for the whole backend.
//!
//! Every fallible operation funnels into [`SpielError`]. Command handlers convert
//! it to a `String` (Tauri serializes `Result<T, String>` to the frontend), but
//! internally we keep typed variants so the orchestrator can react to *why*
//! something failed (e.g. "model missing" should prompt a download, not a generic
//! error toast).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpielError {
    #[error("No microphone found. Connect an input device and try again.")]
    NoInputDevice,

    #[error("Microphone access failed: {0}. Grant microphone permission in System Settings → Privacy & Security → Microphone.")]
    Audio(String),

    #[error("The speech model isn't ready yet. Download it from Spiel's settings window.")]
    ModelMissing,

    #[error("Speech model error: {0}")]
    Model(String),

    #[error("Transcription failed: {0}")]
    Transcription(String),

    #[error("Couldn't insert text: {0}")]
    Insertion(String),

    #[error("Download failed: {0}")]
    Download(String),

    #[error("Settings error: {0}")]
    Config(String),

    #[error("{0}")]
    Other(String),
}

impl SpielError {
    /// A stable machine code the frontend can branch on without parsing prose.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoInputDevice => "no_input_device",
            Self::Audio(_) => "audio",
            Self::ModelMissing => "model_missing",
            Self::Model(_) => "model",
            Self::Transcription(_) => "transcription",
            Self::Insertion(_) => "insertion",
            Self::Download(_) => "download",
            Self::Config(_) => "config",
            Self::Other(_) => "other",
        }
    }
}

pub type Result<T> = std::result::Result<T, SpielError>;

/// Command-layer helper: collapse a typed error into the `(code, message)` the UI shows.
pub fn to_command_error(e: SpielError) -> String {
    // We keep the human message; the code travels in a separate status field where needed.
    e.to_string()
}
