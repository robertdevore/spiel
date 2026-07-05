//! Shared application state and the snapshot the UI/tray render from.

use crate::audio::Recorder;
use crate::config::{Config, Paths};
use crate::focus::FocusTarget;
use crate::model;
use crate::whisper::Transcriber;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
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
    /// Last non-Spiel application that owned focus. Used to paste back into the app
    /// where the cursor was before the tray/settings UI stole focus.
    pub last_focus_target: Mutex<Option<FocusTarget>>,
    model_install_cache: Mutex<HashMap<String, CachedModelInstall>>,
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
            last_focus_target: Mutex::new(None),
            model_install_cache: Mutex::new(HashMap::new()),
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
pub struct DownloadPerfSample {
    pub wall_time_ms: u64,
    pub total_ms: u64,
    pub downloaded_bytes: u64,
    pub expected_bytes: Option<u64>,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerfSnapshot {
    pub enabled: bool,
    pub budget_ms: u64,
    pub sample_count: usize,
    pub average_total_ms: u64,
    pub p50_total_ms: u64,
    pub p95_total_ms: u64,
    pub max_total_ms: u64,
    pub over_budget_count: usize,
    pub average_capture_ms: u64,
    pub average_transcribe_ms: u64,
    pub average_insert_ms: u64,
    pub pasted_count: usize,
    pub clipboard_only_count: usize,
    pub insert_error_count: usize,
    pub download_sample_count: usize,
    pub average_download_ms: u64,
    pub p95_download_ms: u64,
    pub max_download_ms: u64,
    pub last: Option<PerfSample>,
    pub last_download: Option<DownloadPerfSample>,
}

pub struct PerfState {
    enabled: bool,
    budget_ms: u64,
    samples: VecDeque<PerfSample>,
    download_samples: VecDeque<DownloadPerfSample>,
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
            download_samples: VecDeque::with_capacity(64),
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
        let perf = self.perf.lock().unwrap();
        if perf.samples.is_empty() {
            return PerfSnapshot {
                enabled: perf.enabled,
                budget_ms: perf.budget_ms,
                sample_count: 0,
                average_total_ms: 0,
                p50_total_ms: 0,
                p95_total_ms: 0,
                max_total_ms: 0,
                over_budget_count: 0,
                average_capture_ms: 0,
                average_transcribe_ms: 0,
                average_insert_ms: 0,
                pasted_count: 0,
                clipboard_only_count: 0,
                insert_error_count: 0,
                download_sample_count: perf.download_samples.len(),
                average_download_ms: aggregate_avg_download_ms(&perf.download_samples),
                p95_download_ms: aggregate_p95_download_ms(&perf.download_samples),
                max_download_ms: aggregate_max_download_ms(&perf.download_samples),
                last: None,
                last_download: perf.download_samples.back().cloned(),
            };
        }
        let mut totals: Vec<u64> = perf.samples.iter().map(|s| s.total_ms).collect();
        totals.sort_unstable();
        let p50_idx = totals.len().saturating_sub(1) / 2;
        let p95_idx = ((totals.len() as f64 * 0.95).ceil() as usize)
            .saturating_sub(1)
            .min(totals.len() - 1);
        let p50 = totals[p50_idx];
        let p95 = totals[p95_idx];
        let sum: u64 = perf.samples.iter().map(|s| s.total_ms).sum();
        let avg = sum / perf.samples.len() as u64;
        let max = totals.last().copied().unwrap_or(0);
        let avg_capture =
            perf.samples.iter().map(|s| s.capture_ms).sum::<u64>() / perf.samples.len() as u64;
        let avg_transcribe =
            perf.samples.iter().map(|s| s.transcribe_ms).sum::<u64>() / perf.samples.len() as u64;
        let avg_insert =
            perf.samples.iter().map(|s| s.insert_ms).sum::<u64>() / perf.samples.len() as u64;
        let over_budget = perf
            .samples
            .iter()
            .filter(|s| s.total_ms > perf.budget_ms)
            .count();
        let pasted_count = perf
            .samples
            .iter()
            .filter(|s| s.outcome == "pasted")
            .count();
        let clipboard_only_count = perf
            .samples
            .iter()
            .filter(|s| s.outcome == "clipboard_only")
            .count();
        let insert_error_count = perf
            .samples
            .iter()
            .filter(|s| s.outcome == "insert_error")
            .count();
        PerfSnapshot {
            enabled: perf.enabled,
            budget_ms: perf.budget_ms,
            sample_count: perf.samples.len(),
            average_total_ms: avg,
            p50_total_ms: p50,
            p95_total_ms: p95,
            max_total_ms: max,
            over_budget_count: over_budget,
            average_capture_ms: avg_capture,
            average_transcribe_ms: avg_transcribe,
            average_insert_ms: avg_insert,
            pasted_count,
            clipboard_only_count,
            insert_error_count,
            download_sample_count: perf.download_samples.len(),
            average_download_ms: aggregate_avg_download_ms(&perf.download_samples),
            p95_download_ms: aggregate_p95_download_ms(&perf.download_samples),
            max_download_ms: aggregate_max_download_ms(&perf.download_samples),
            last: perf.samples.back().cloned(),
            last_download: perf.download_samples.back().cloned(),
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

    pub fn record_download_sample(&self, mut sample: DownloadPerfSample) {
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
        if perf.download_samples.len() >= 64 {
            perf.download_samples.pop_front();
        }
        perf.download_samples.push_back(sample);
    }

    pub fn clear_perf_samples(&self) {
        let mut perf = self.perf.lock().unwrap();
        perf.samples.clear();
        perf.download_samples.clear();
    }
}

fn aggregate_avg_download_ms(samples: &VecDeque<DownloadPerfSample>) -> u64 {
    if samples.is_empty() {
        return 0;
    }
    samples.iter().map(|s| s.total_ms).sum::<u64>() / samples.len() as u64
}

fn aggregate_p95_download_ms(samples: &VecDeque<DownloadPerfSample>) -> u64 {
    if samples.is_empty() {
        return 0;
    }
    let mut totals: Vec<u64> = samples.iter().map(|s| s.total_ms).collect();
    totals.sort_unstable();
    let idx = ((totals.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(totals.len() - 1);
    totals[idx]
}

fn aggregate_max_download_ms(samples: &VecDeque<DownloadPerfSample>) -> u64 {
    samples.iter().map(|s| s.total_ms).max().unwrap_or(0)
}
