//! One error type for the whole backend.
//!
//! Every fallible operation funnels into [`SpielError`]. Command handlers convert
//! it to a `String` (Tauri serializes `Result<T, String>` to the frontend), but
//! internally we keep typed variants so the orchestrator can react to *why*
//! something failed (e.g. "model missing" should prompt a download, not a generic
//! error toast).

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadFailureKind {
    Canceled,
    Checksum,
    HttpStatus,
    Network,
    Incomplete,
    UnsafePath,
}

impl DownloadFailureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Canceled => "canceled",
            Self::Checksum => "checksum",
            Self::HttpStatus => "http_status",
            Self::Network => "network",
            Self::Incomplete => "incomplete",
            Self::UnsafePath => "unsafe_path",
        }
    }
}

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

    #[error("Download failed: {message}")]
    DownloadFailure {
        kind: DownloadFailureKind,
        message: String,
    },

    #[error("Speech model error: {0}")]
    ModelChecksum(String),

    #[error("Settings error: {0}")]
    Config(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, SpielError>;

/// Convert an internal typed error to the string returned by Tauri commands.
pub fn to_command_error(e: SpielError) -> String {
    e.to_string()
}
