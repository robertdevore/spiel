//! Shared application state and the snapshot the UI/tray render from.

use crate::audio::Recorder;
use crate::config::{Config, Paths};
use crate::whisper::Transcriber;
use serde::Serialize;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::VecDeque, env};

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
    pub perf: Mutex<PerfState>,
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
            perf: Mutex::new(PerfState::new()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PerfSample {
    pub wall_time_ms: u64,
    pub capture_ms: u64,
    pub transcribe_ms: u64,
    pub insert_ms: u64,
    pub total_ms: u64,
    pub audio_samples: usize,
    pub text_chars: usize,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerfSnapshot {
    pub enabled: bool,
    pub budget_ms: u64,
    pub sample_count: usize,
    pub average_total_ms: u64,
    pub p95_total_ms: u64,
    pub max_total_ms: u64,
    pub over_budget_count: usize,
    pub last: Option<PerfSample>,
}

pub struct PerfState {
    enabled: bool,
    budget_ms: u64,
    samples: VecDeque<PerfSample>,
}

impl PerfState {
    fn new() -> Self {
        let enabled = env::var("SPIEL_PROFILE")
            .ok()
            .map(|v| matches!(v.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false);
        let budget_ms = env::var("SPIEL_LATENCY_BUDGET_MS")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .map(|v| v.clamp(500, 120_000))
            .unwrap_or(8_000);
        Self {
            enabled,
            budget_ms,
            samples: VecDeque::with_capacity(128),
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

    pub fn perf_snapshot(&self) -> PerfSnapshot {
        let perf = self.perf.lock().unwrap();
        if perf.samples.is_empty() {
            return PerfSnapshot {
                enabled: perf.enabled,
                budget_ms: perf.budget_ms,
                sample_count: 0,
                average_total_ms: 0,
                p95_total_ms: 0,
                max_total_ms: 0,
                over_budget_count: 0,
                last: None,
            };
        }
        let mut totals: Vec<u64> = perf.samples.iter().map(|s| s.total_ms).collect();
        totals.sort_unstable();
        let p95_idx = ((totals.len() as f64 * 0.95).ceil() as usize)
            .saturating_sub(1)
            .min(totals.len() - 1);
        let p95 = totals[p95_idx];
        let sum: u64 = perf.samples.iter().map(|s| s.total_ms).sum();
        let avg = sum / perf.samples.len() as u64;
        let max = totals.last().copied().unwrap_or(0);
        let over_budget = perf
            .samples
            .iter()
            .filter(|s| s.total_ms > perf.budget_ms)
            .count();
        PerfSnapshot {
            enabled: perf.enabled,
            budget_ms: perf.budget_ms,
            sample_count: perf.samples.len(),
            average_total_ms: avg,
            p95_total_ms: p95,
            max_total_ms: max,
            over_budget_count: over_budget,
            last: perf.samples.back().cloned(),
        }
    }

    pub fn record_perf_sample(&self, mut sample: PerfSample) {
        let mut perf = self.perf.lock().unwrap();
        if !perf.enabled {
            return;
        }
        if sample.wall_time_ms == 0 {
            sample.wall_time_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
        }
        if perf.samples.len() >= 128 {
            perf.samples.pop_front();
        }
        if sample.total_ms > perf.budget_ms {
            eprintln!(
                "[spiel][perf] over budget: {}ms > {}ms (capture={} transcribe={} insert={} chars={} outcome={})",
                sample.total_ms,
                perf.budget_ms,
                sample.capture_ms,
                sample.transcribe_ms,
                sample.insert_ms,
                sample.text_chars,
                sample.outcome
            );
        }
        perf.samples.push_back(sample);
    }

    pub fn clear_perf_samples(&self) {
        self.perf.lock().unwrap().samples.clear();
    }
}
