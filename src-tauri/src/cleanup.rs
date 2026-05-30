//! Cleanup provider abstraction for Spiel Phase 7.
//!
//! Defines the cleanup pipeline: take a raw transcript + selected mode,
//! run it through a cleanup provider, and produce final text.
//!
//! Architecture:
//! - `CleanupProvider` trait: common interface for all cleanup engines
//! - `CleanupRequest`: input specification (raw text, mode, provider)
//! - `CleanupResult`: output (raw text, final text, metadata)
//! - `CleanupError`: structured error with recovery info
//! - `CleanupState`: app-level state tracking
//!
//! Providers implemented:
//! - BasicCleanupProvider (deterministic, local) — Phase 7
//! - MockAiCleanupProvider (mock, clearly labeled) — Phase 7
//! - OpenAiCleanupProvider (cloud AI via OpenAI API) — Phase 12
//!
//! Planned for future phases:
//! - LocalLlmCleanupProvider

use crate::modes::TextModeKind;
use serde::{Deserialize, Serialize};

/// Kinds of cleanup providers available or planned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupProviderKind {
    /// Basic deterministic cleanup — always available, local only
    Basic,
    /// Mock AI cleanup — simulates future AI behavior, clearly labeled
    MockAi,
    /// OpenAI-powered cleanup (Phase 12) — optional, requires API key
    OpenAi,
    /// Local LLM cleanup — planned, not implemented
    LocalLlmPlanned,
}

impl CleanupProviderKind {
    /// Whether this provider is currently implemented.
    pub fn is_implemented(&self) -> bool {
        matches!(self, Self::Basic | Self::MockAi | Self::OpenAi)
    }

    /// Human-readable label.
    pub fn label(&self) -> &str {
        match self {
            Self::Basic => "Basic (deterministic, local)",
            Self::MockAi => "Mock AI (testing only)",
            Self::OpenAi => "OpenAI (cloud)",
            Self::LocalLlmPlanned => "Local LLM (planned — not available)",
        }
    }

    /// Short description.
    pub fn description(&self) -> &str {
        match self {
            Self::Basic => "Deterministic text cleanup. No AI. Runs entirely on your device.",
            Self::MockAi => {
                "Simulates future AI cleanup for testing. Clearly labeled as mock. No real AI."
            }
            Self::OpenAi => {
                "Cloud AI cleanup via OpenAI API. Sends text to api.openai.com. Requires API key."
            }
            Self::LocalLlmPlanned => "Local AI cleanup via on-device LLM. Not implemented yet.",
        }
    }

    /// All available providers.
    pub fn all_providers() -> Vec<CleanupProviderKind> {
        vec![
            Self::Basic,
            Self::MockAi,
            Self::OpenAi,
            Self::LocalLlmPlanned,
        ]
    }
}

/// Available provider descriptors for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupProviderInfo {
    pub kind: CleanupProviderKind,
    pub label: String,
    pub description: String,
    pub implemented: bool,
}

impl CleanupProviderInfo {
    /// Create info from a provider kind.
    pub fn from_kind(kind: &CleanupProviderKind) -> Self {
        Self {
            kind: kind.clone(),
            label: kind.label().into(),
            description: kind.description().into(),
            implemented: kind.is_implemented(),
        }
    }

    /// All provider infos.
    pub fn all_infos() -> Vec<CleanupProviderInfo> {
        CleanupProviderKind::all_providers()
            .iter()
            .map(Self::from_kind)
            .collect()
    }
}

/// Request to clean up a raw transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupRequest {
    /// The raw transcript text to process
    pub raw_text: String,
    /// The text mode to apply
    pub mode: TextModeKind,
    /// The cleanup provider to use
    pub provider: CleanupProviderKind,
    /// Optional transcription ID for traceability
    pub source_transcription_id: Option<String>,
    /// ISO 8601 timestamp when request was created
    pub created_at: String,
}

/// Result of a cleanup operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    /// The original raw text (preserved)
    pub raw_text: String,
    /// The cleaned/prepared final text
    pub final_text: String,
    /// The mode that was applied
    pub mode: TextModeKind,
    /// The provider that processed it
    pub provider: CleanupProviderKind,
    /// Whether this result came from a mock provider
    pub is_mock: bool,
    /// Whether the text was actually changed
    pub changed: bool,
    /// ISO 8601 timestamp when cleanup started
    pub created_at: String,
    /// ISO 8601 timestamp when cleanup completed
    pub completed_at: String,
    /// Time spent cleaning in milliseconds
    pub duration_ms: u64,
    /// Human-readable warnings (e.g., "AI providers are not available yet")
    pub warnings: Vec<String>,
    /// Error if cleanup failed (None on success)
    pub error: Option<CleanupError>,
}

/// Structured error from a cleanup operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupError {
    /// Machine-readable error code
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Whether this error is recoverable (user can fix input and retry)
    pub recoverable: bool,
    /// Additional details if safe to expose
    pub details: Option<String>,
}

/// Current cleanup status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupStatus {
    /// No cleanup has been run
    Idle,
    /// Waiting for a transcript to be available
    WaitingForTranscript,
    /// Cleanup is in progress
    Cleaning,
    /// Cleanup completed successfully
    Complete,
    /// Cleanup failed with an error
    Error,
    /// Cleanup was cancelled
    Canceled,
}

/// Application-level cleanup state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupState {
    /// Current cleanup status
    pub status: CleanupStatus,
    /// The last cleanup result (if any)
    pub last_result: Option<CleanupResult>,
    /// Error message if status is Error
    pub error: Option<String>,
    /// Currently selected mode (persisted in state)
    pub selected_mode: Option<TextModeKind>,
    /// Currently selected provider (persisted in state)
    pub selected_provider: Option<CleanupProviderKind>,
}

impl Default for CleanupState {
    fn default() -> Self {
        Self {
            status: CleanupStatus::Idle,
            last_result: None,
            error: None,
            selected_mode: None,
            selected_provider: Some(CleanupProviderKind::Basic),
        }
    }
}

/// Trait for cleanup providers.
/// Future providers (OpenAI, local LLM) implement this trait.
pub trait CleanupProvider: Send + Sync {
    /// Clean up raw text using the specified mode.
    fn cleanup(&self, request: &CleanupRequest) -> Result<CleanupResult, CleanupError>;

    /// Human-readable name of this provider.
    fn name(&self) -> &str;

    /// The kind of this provider.
    fn kind(&self) -> CleanupProviderKind;

    /// Whether this provider is a mock.
    fn is_mock(&self) -> bool;
}
