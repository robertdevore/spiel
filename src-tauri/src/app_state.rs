use serde::{Deserialize, Serialize};
use std::sync::Mutex;

pub use crate::cleanup::CleanupState;
pub use crate::history::HistoryStateData;
pub use crate::settings::SpielSettings;
pub use crate::transcription::{EngineInfo, TranscriptionState, WhisperConfig};
pub use crate::workflow::WorkflowState;

pub use crate::secrets::SecretStore;

/// Represents the current state of a capability in the app.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityStatus {
    pub name: String,
    pub status: String, // "implemented", "planned", "placeholder"
}

/// Hotkey registration and status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyState {
    pub shortcut: String,
    pub registered: bool,
    pub error: Option<String>,
    pub last_triggered: Option<String>,
    pub trigger_count: u32,
}

impl Default for HotkeyState {
    fn default() -> Self {
        Self {
            shortcut: "Cmd+Shift+S".into(),
            registered: false,
            error: None,
            last_triggered: None,
            trigger_count: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingState {
    Idle,
    Recording,
    Stopping,
    Complete,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingStatus {
    pub state: RecordingState,
    pub elapsed_ms: u64,
    pub started_at: Option<String>,
    pub last_recording: Option<LastRecording>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastRecording {
    pub file_path: String,
    pub filename: String,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub size_bytes: u64,
    pub created_at: String,
    pub device_name: Option<String>,
}

pub struct AppState {
    pub hotkey: Mutex<HotkeyState>,
    pub recording: Mutex<RecordingStateData>,
    pub transcription: Mutex<TranscriptionState>,
    pub whisper_config: Mutex<WhisperConfig>,
    pub cleanup: Mutex<CleanupState>,
    pub history_state: Mutex<HistoryStateData>,
    /// Settings (Phase 9) — cached in memory, persisted to SQLite
    pub settings: Mutex<SpielSettings>,
    /// Workflow state (Phase 10) — end-to-end orchestration
    pub workflow: Mutex<WorkflowState>,
    /// Secret store (Phase 12) — in-memory API key storage
    pub secrets: Mutex<SecretStore>,
}

pub struct RecordingStateData {
    pub state: RecordingState,
    pub started_at: Option<String>,
    pub elapsed_ms: u64,
    pub last_recording: Option<LastRecording>,
    pub error: Option<String>,
}

impl Default for RecordingStateData {
    fn default() -> Self {
        Self {
            state: RecordingState::Idle,
            started_at: None,
            elapsed_ms: 0,
            last_recording: None,
            error: None,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            hotkey: Mutex::new(HotkeyState::default()),
            recording: Mutex::new(RecordingStateData::default()),
            transcription: Mutex::new(TranscriptionState::default()),
            whisper_config: Mutex::new(WhisperConfig::default()),
            settings: Mutex::new(SpielSettings::default()),
            workflow: Mutex::new(WorkflowState::default()),
            cleanup: Mutex::new(CleanupState::default()),
            history_state: Mutex::new(HistoryStateData::default()),
            secrets: Mutex::new(SecretStore::new()),
        }
    }
}

impl AppState {
    pub fn capability_statuses() -> Vec<CapabilityStatus> {
        vec![
            CapabilityStatus {
                name: "ui_foundation".into(),
                status: "implemented".into(),
            },
            CapabilityStatus {
                name: "global_hotkey".into(),
                status: "implemented".into(),
            },
            CapabilityStatus {
                name: "audio_recording".into(),
                status: "implemented".into(),
            },
            CapabilityStatus {
                name: "transcription".into(),
                status: "implemented".into(),
            },
            CapabilityStatus {
                name: "clipboard_paste".into(),
                status: "implemented".into(),
            },
            CapabilityStatus {
                name: "local_history".into(),
                status: "implemented".into(),
            },
            CapabilityStatus {
                name: "text_modes".into(),
                status: "implemented".into(),
            },
            CapabilityStatus {
                name: "settings_persistence".into(),
                status: "implemented".into(),
            },
            CapabilityStatus {
                name: "cloud_providers".into(),
                status: "implemented".into(),
            },
        ]
    }
}
