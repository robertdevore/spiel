//! Shared application state and the snapshot the UI/tray render from.

use crate::audio::Recorder;
use crate::config::{Config, Paths};
use crate::whisper::Transcriber;
use serde::Serialize;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// Where the dictation loop currently is. Drives the tray icon and the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Idle,
    Recording,
    Transcribing,
    Inserting,
    Error,
}

pub struct StatusState {
    pub phase: Phase,
    pub message: Option<String>,
    /// Last auto-paste needed Accessibility permission that wasn't granted.
    pub needs_accessibility: bool,
}

impl Default for StatusState {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            message: None,
            needs_accessibility: false,
        }
    }
}

#[derive(Default)]
pub struct DownloadState {
    pub active: bool,
    pub model_id: Option<String>,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub cancel: Arc<AtomicBool>,
}

pub struct AppState {
    pub paths: Paths,
    pub config: Mutex<Config>,
    pub status: Mutex<StatusState>,
    pub recorder: Mutex<Option<Recorder>>,
    /// Lazily-loaded, cached model context. Reloaded when the model setting changes.
    pub transcriber: Mutex<Option<Transcriber>>,
    pub download: Mutex<DownloadState>,
}

impl AppState {
    pub fn new(paths: Paths, config: Config) -> Self {
        Self {
            paths,
            config: Mutex::new(config),
            status: Mutex::new(StatusState::default()),
            recorder: Mutex::new(None),
            transcriber: Mutex::new(None),
            download: Mutex::new(DownloadState::default()),
        }
    }
}

/// Serializable view of everything the frontend/tray needs in one shot.
#[derive(Debug, Clone, Serialize)]
pub struct StatusSnapshot {
    pub phase: Phase,
    pub message: Option<String>,
    pub needs_accessibility: bool,
    pub recording_elapsed_ms: u64,
    pub model_id: String,
    pub model_installed: bool,
    pub accessibility_trusted: bool,
}

impl AppState {
    pub fn snapshot(&self) -> StatusSnapshot {
        let status = self.status.lock().unwrap();
        let config = self.config.lock().unwrap();
        let elapsed = self
            .recorder
            .lock()
            .unwrap()
            .as_ref()
            .map(|r| r.elapsed_ms())
            .unwrap_or(0);
        StatusSnapshot {
            phase: status.phase,
            message: status.message.clone(),
            needs_accessibility: status.needs_accessibility,
            recording_elapsed_ms: elapsed,
            model_id: config.model.clone(),
            model_installed: crate::model::is_installed(&self.paths.model_dir, &config.model),
            accessibility_trusted: crate::accessibility::is_trusted(),
        }
    }

    pub fn set_phase(&self, phase: Phase, message: Option<String>) {
        let mut s = self.status.lock().unwrap();
        s.phase = phase;
        s.message = message;
    }
}
