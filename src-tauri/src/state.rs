//! Shared application state and the snapshot the UI/tray render from.

use crate::audio::Recorder;
use crate::config::{Config, Paths};
use crate::downloads::Downloads;
use crate::focus::FocusTarget;
use crate::model;
use crate::perf::PerfState;
pub use crate::perf::{DownloadPerfSample, PerfSample, PerfSnapshot};
use crate::transcription::Transcription;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

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

#[derive(Clone)]
struct CachedModelInstall {
    info: model::InstallInfo,
    checked_at: Instant,
}

const MODEL_INSTALL_TTL: Duration = Duration::from_millis(500);

impl Default for StatusState {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            message: None,
            needs_accessibility: false,
        }
    }
}

pub struct AppState {
    pub paths: Paths,
    pub config: Mutex<Config>,
    pub status: Mutex<StatusState>,
    pub recorder: Mutex<Option<Recorder>>,
    /// Lazily-loaded, cached model context. Reloaded when the model setting changes.
    pub transcription: Transcription,
    pub download: Downloads,
    /// Last non-Spiel application that owned focus. Used to paste back into the app
    /// where the cursor was before the tray/settings UI stole focus.
    pub last_focus_target: Mutex<Option<FocusTarget>>,
    model_install_cache: Mutex<HashMap<String, CachedModelInstall>>,
    perf: Mutex<PerfState>,
}

impl AppState {
    pub fn new(paths: Paths, config: Config) -> Self {
        Self {
            paths,
            config: Mutex::new(config),
            status: Mutex::new(StatusState::default()),
            recorder: Mutex::new(None),
            transcription: Transcription::default(),
            download: Downloads::default(),
            last_focus_target: Mutex::new(None),
            model_install_cache: Mutex::new(HashMap::new()),
            perf: Mutex::new(PerfState::new()),
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
    pub accessibility_supported: bool,
}

impl AppState {
    pub fn model_install_info(&self, model_id: &str) -> model::InstallInfo {
        let now = Instant::now();
        if let Some(cached) = self
            .model_install_cache
            .lock()
            .unwrap()
            .get(model_id)
            .filter(|entry| now.duration_since(entry.checked_at) < MODEL_INSTALL_TTL)
            .cloned()
        {
            return cached.info;
        }

        let fresh = model::inspect_install(&self.paths.model_dir, model_id);
        self.model_install_cache.lock().unwrap().insert(
            model_id.to_string(),
            CachedModelInstall {
                info: fresh.clone(),
                checked_at: now,
            },
        );
        fresh
    }

    pub fn clear_model_install_cache(&self) {
        self.model_install_cache.lock().unwrap().clear();
    }

    pub fn clear_model_install_cache_entry(&self, model_id: &str) {
        self.model_install_cache.lock().unwrap().remove(model_id);
    }

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
            needs_accessibility: status.needs_accessibility && !crate::accessibility::is_trusted(),
            recording_elapsed_ms: elapsed,
            model_id: config.model.clone(),
            model_installed: self.model_install_info(&config.model).is_installed(),
            accessibility_trusted: crate::accessibility::is_supported()
                && crate::accessibility::is_trusted(),
            accessibility_supported: crate::accessibility::is_supported(),
        }
    }

    pub fn set_phase(&self, phase: Phase, message: Option<String>) {
        let mut s = self.status.lock().unwrap();
        s.phase = phase;
        s.message = message;
    }

    pub fn perf_snapshot(&self) -> PerfSnapshot {
        self.perf.lock().unwrap().snapshot()
    }
    pub fn record_perf_sample(&self, sample: PerfSample) {
        self.perf.lock().unwrap().record(sample);
    }
    pub fn record_download_sample(&self, sample: DownloadPerfSample) {
        self.perf.lock().unwrap().record_download(sample);
    }
    pub fn clear_perf_samples(&self) {
        self.perf.lock().unwrap().clear();
    }
}
