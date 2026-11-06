use serde::Serialize;
use std::{
    collections::VecDeque,
    env,
    time::{SystemTime, UNIX_EPOCH},
};

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
    pub fn new() -> Self {
        let enabled = crate::runtime_options::env_flag("SPIEL_PROFILE");
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

impl PerfState {
    pub fn snapshot(&self) -> PerfSnapshot {
        let perf = self;
        let mut totals: Vec<u64> = perf.samples.iter().map(|s| s.total_ms).collect();
        totals.sort_unstable();
        let p50 = percentile(&totals, 0.50);
        let p95 = percentile(&totals, 0.95);
        let count = perf.samples.len().max(1) as u64;
        let avg = totals.iter().sum::<u64>() / count;
        let max = totals.last().copied().unwrap_or(0);
        let avg_capture = perf.samples.iter().map(|s| s.capture_ms).sum::<u64>() / count;
        let avg_transcribe = perf.samples.iter().map(|s| s.transcribe_ms).sum::<u64>() / count;
        let avg_insert = perf.samples.iter().map(|s| s.insert_ms).sum::<u64>() / count;
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

    pub fn record(&mut self, mut sample: PerfSample) {
        let perf = self;
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

    pub fn record_download(&mut self, mut sample: DownloadPerfSample) {
        let perf = self;
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

    pub fn clear(&mut self) {
        let perf = self;
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
    percentile(&totals, 0.95)
}

fn aggregate_max_download_ms(samples: &VecDeque<DownloadPerfSample>) -> u64 {
    samples.iter().map(|s| s.total_ms).max().unwrap_or(0)
}

fn percentile(sorted: &[u64], fraction: f64) -> u64 {
    let index = (sorted.len() as f64 * fraction).ceil() as usize;
    sorted.get(index.saturating_sub(1)).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_dictations_preserve_download_metrics() {
        let mut perf = PerfState::new();
        perf.enabled = true;
        perf.record_download(DownloadPerfSample {
            wall_time_ms: 1,
            total_ms: 42,
            downloaded_bytes: 10,
            expected_bytes: Some(10),
            outcome: "downloaded".into(),
        });
        let snapshot = perf.snapshot();
        assert_eq!(snapshot.sample_count, 0);
        assert_eq!(snapshot.p95_total_ms, 0);
        assert_eq!(snapshot.average_download_ms, 42);
        assert_eq!(snapshot.p95_download_ms, 42);
        perf.clear();
        assert_eq!(perf.snapshot().download_sample_count, 0);
    }
    #[test]
    fn percentiles_keep_lower_median_and_nearest_rank() {
        assert_eq!(percentile(&[], 0.95), 0);
        assert_eq!(percentile(&[10, 20, 30, 40], 0.50), 20);
        assert_eq!(percentile(&[10, 20, 30, 40], 0.95), 40);
    }
}
