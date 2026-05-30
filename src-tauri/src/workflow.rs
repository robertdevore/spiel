//! End-to-end workflow state machine for Spiel Phase 10.
//!
//! Orchestrates the full Spiel core loop:
//! record → transcribe → cleanup → insert → save
//!
//! Safety principles:
//! - Auto-insert defaults to off
//! - Never presses Enter or submits forms
//! - Manual review mode is the default workflow
//! - All steps are explicit and auditable

use serde::{Deserialize, Serialize};

/// The current step in the Spiel workflow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStep {
    /// No workflow active
    Idle,
    /// Recording audio from microphone
    Recording,
    /// Recording stopped, finalizing WAV
    RecordingStopping,
    /// Recording complete, ready for transcription
    RecordingComplete,
    /// Transcription in progress
    Transcribing,
    /// Transcription complete, ready for cleanup
    TranscriptionComplete,
    /// Cleanup/mode preparation in progress
    Cleaning,
    /// Cleanup complete, final text ready
    CleanupComplete,
    /// Semi-auto: insert attempted
    Inserting,
    /// Insertion was attempted (result stored)
    InsertAttempted,
    /// Saved to history
    SavedToHistory,
    /// Workflow cancelled by user
    Canceled,
    /// Workflow hit an error
    Error,
}

impl WorkflowStep {
    pub fn label(&self) -> &str {
        match self {
            Self::Idle => "Idle",
            Self::Recording => "Recording",
            Self::RecordingStopping => "Stopping",
            Self::RecordingComplete => "Recording Complete",
            Self::Transcribing => "Transcribing",
            Self::TranscriptionComplete => "Transcription Complete",
            Self::Cleaning => "Cleaning",
            Self::CleanupComplete => "Cleanup Complete",
            Self::Inserting => "Inserting",
            Self::InsertAttempted => "Insertion Attempted",
            Self::SavedToHistory => "Saved to History",
            Self::Canceled => "Canceled",
            Self::Error => "Error",
        }
    }
}

/// Full workflow state, managed by Tauri.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub step: WorkflowStep,
    pub error: Option<String>,
    pub last_recording_filename: Option<String>,
    pub last_recording_duration_ms: Option<u64>,
    pub last_transcript_raw: Option<String>,
    pub last_final_text: Option<String>,
    pub last_history_entry_id: Option<i64>,
    pub insertion_attempted: bool,
    pub insertion_result_message: Option<String>,
}

impl Default for WorkflowState {
    fn default() -> Self {
        Self {
            step: WorkflowStep::Idle,
            error: None,
            last_recording_filename: None,
            last_recording_duration_ms: None,
            last_transcript_raw: None,
            last_final_text: None,
            last_history_entry_id: None,
            insertion_attempted: false,
            insertion_result_message: None,
        }
    }
}

impl WorkflowState {
    /// Reset to idle, clearing transient workflow data.
    /// Does NOT delete history or settings.
    pub fn reset(&mut self) {
        self.step = WorkflowStep::Idle;
        self.error = None;
        self.last_recording_filename = None;
        self.last_recording_duration_ms = None;
        self.last_transcript_raw = None;
        self.last_final_text = None;
        self.last_history_entry_id = None;
        self.insertion_attempted = false;
        self.insertion_result_message = None;
    }

    /// Move to error state with a message.
    pub fn set_error(&mut self, msg: String) {
        self.step = WorkflowStep::Error;
        self.error = Some(msg);
    }
}
