//! Local transcription via embedded whisper.cpp (the `whisper-rs` crate).
//!
//! By default dictation runs in a short-lived worker process so Whisper's model/state
//! memory exits with the worker instead of growing the menu-bar app's resident set.
//! Users can still opt into an in-process cached model for lower first-token latency.
//! Everything runs on-device; there is no network path.

use crate::error::{Result, SpielError};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// A loaded model, ready to transcribe. Cheap to clone (it's an `Arc` inside).
#[derive(Clone)]
pub struct Transcriber {
    ctx: Arc<WhisperContext>,
    /// Which model id produced this context, so the cache knows when to reload.
    pub model_id: String,
}

impl Transcriber {
    /// Load a GGML model from disk. Doubles as the final integrity gate: if whisper.cpp
    /// can't parse it, we surface a clear error rather than crashing later.
    pub fn load(model_path: &Path, model_id: &str) -> Result<Self> {
        if !model_path.exists() {
            return Err(SpielError::ModelMissing);
        }
        let path_str = model_path
            .to_str()
            .ok_or_else(|| SpielError::Model("model path is not valid UTF-8".into()))?;

        let ctx_params = low_memory_context_params();
        let ctx = WhisperContext::new_with_params(path_str, ctx_params)
            .map_err(|e| SpielError::Model(format!("failed to load model: {e}")))?;

        Ok(Self {
            ctx: Arc::new(ctx),
            model_id: model_id.to_string(),
        })
    }

    /// Transcribe mono 16 kHz f32 samples. `language` is a code like "en", or "auto".
    pub fn transcribe(
        &self,
        samples: &[f32],
        language: &str,
        configured_threads: u8,
    ) -> Result<String> {
        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| SpielError::Transcription(e.to_string()))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(choose_thread_count(configured_threads));
        params.set_no_timestamps(true);
        params.set_single_segment(true);
        params.set_audio_ctx(audio_context_for_samples(samples.len()));
        params.set_n_max_text_ctx(0);
        params.set_translate(false);
        params.set_no_context(true);
        params.set_suppress_blank(true);
        params.set_suppress_non_speech_tokens(true);
        params.set_temperature(0.0);
        params.set_temperature_inc(0.0);
        if language != "auto" && !language.is_empty() {
            params.set_language(Some(language));
        }
        // We want clean text out, not progress noise on stdout.
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        state
            .full(params, samples)
            .map_err(|e| SpielError::Transcription(e.to_string()))?;

        let num_segments = state
            .full_n_segments()
            .map_err(|e| SpielError::Transcription(e.to_string()))?;

        let mut text = String::new();
        for i in 0..num_segments {
            let seg = state
                .full_get_segment_text(i)
                .map_err(|e| SpielError::Transcription(e.to_string()))?;
            text.push_str(&seg);
        }

        Ok(clean_output(&text))
    }
}

pub fn transcribe_in_worker(
    model_path: &Path,
    model_id: &str,
    samples: &[f32],
    language: &str,
    configured_threads: u8,
) -> Result<String> {
    let exe = std::env::current_exe()
        .map_err(|e| SpielError::Transcription(format!("cannot locate Spiel executable: {e}")))?;
    let model_path = model_path
        .to_str()
        .ok_or_else(|| SpielError::Model("model path is not valid UTF-8".into()))?;

    let mut child = Command::new(exe)
        .arg("--spiel-transcribe-worker")
        .arg(model_path)
        .arg(model_id)
        .arg(language)
        .arg(configured_threads.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            SpielError::Transcription(format!("failed to start transcription worker: {e}"))
        })?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| SpielError::Transcription("worker stdin unavailable".into()))?;
        for sample in samples {
            stdin.write_all(&sample.to_le_bytes()).map_err(|e| {
                SpielError::Transcription(format!("failed to send audio to worker: {e}"))
            })?;
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|e| SpielError::Transcription(format!("transcription worker failed: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(SpielError::Transcription(if stderr.is_empty() {
            format!("transcription worker exited with {}", output.status)
        } else {
            stderr
        }));
    }

    String::from_utf8(output.stdout)
        .map(|text| text.trim_end_matches('\n').to_string())
        .map_err(|e| SpielError::Transcription(format!("worker returned invalid UTF-8: {e}")))
}

pub fn run_worker_from_args(args: &[String]) -> bool {
    if args.get(1).map(String::as_str) != Some("--spiel-transcribe-worker") {
        return false;
    }

    let result = run_worker(args);
    match result {
        Ok(text) => {
            println!("{text}");
            true
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    }
}

fn run_worker(args: &[String]) -> Result<String> {
    let model_path = args
        .get(2)
        .ok_or_else(|| SpielError::Transcription("worker missing model path".into()))?;
    let model_id = args
        .get(3)
        .ok_or_else(|| SpielError::Transcription("worker missing model id".into()))?;
    let language = args
        .get(4)
        .ok_or_else(|| SpielError::Transcription("worker missing language".into()))?;
    let threads = args
        .get(5)
        .and_then(|raw| raw.parse::<u8>().ok())
        .unwrap_or(1);

    let mut bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|e| SpielError::Transcription(format!("failed to read worker audio: {e}")))?;
    if bytes.len() % std::mem::size_of::<f32>() != 0 {
        return Err(SpielError::Transcription(
            "worker received partial f32 audio frame".into(),
        ));
    }
    let samples = bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect::<Vec<_>>();

    let transcriber = Transcriber::load(Path::new(model_path), model_id)?;
    transcriber.transcribe(&samples, language, threads)
}

fn low_memory_context_params() -> WhisperContextParameters<'static> {
    let mut params = WhisperContextParameters::default();
    params.use_gpu(false);
    params.flash_attn(true);
    params
}

fn audio_context_for_samples(sample_count: usize) -> i32 {
    let seconds = (sample_count as f32 / 16_000.0).ceil() as i32;
    (seconds * 50).clamp(150, 1500)
}

/// Tidy Whisper output: drop leading/trailing whitespace and the bracketed non-speech
/// tags it emits for silence/music, which should never be inserted as text.
fn clean_output(raw: &str) -> String {
    let mut out = String::new();
    for line in raw.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let lower = t.to_ascii_lowercase();
        if is_non_speech_tag(&lower) || is_non_speech_caption(&lower) {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(t);
    }
    out.trim().to_string()
}

fn is_non_speech_tag(lower: &str) -> bool {
    const TAGS: &[&str] = &[
        "[blank_audio]",
        "[silence]",
        "(silence)",
        "[ silence ]",
        "[music]",
        "(music)",
        "[inaudible]",
        "[noise]",
        "[ pause ]",
    ];
    TAGS.contains(&lower)
}

fn is_non_speech_caption(lower: &str) -> bool {
    let bracketed = (lower.starts_with('[') && lower.ends_with(']'))
        || (lower.starts_with('(') && lower.ends_with(')'));
    bracketed
        && [
            "inaudible",
            "foreign language",
            "speaking",
            "silence",
            "music",
            "noise",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
}

fn choose_thread_count(configured_threads: u8) -> i32 {
    if let Ok(raw) = std::env::var("SPIEL_WHISPER_THREADS") {
        if let Ok(v) = raw.trim().parse::<i32>() {
            return v.clamp(1, 16);
        }
    }
    let configured = (configured_threads as i32).clamp(1, 8);
    let cores = std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4);
    // Leave one core for UI/system work so dictation remains responsive under load.
    configured.min((cores - 1).clamp(1, 8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_whitespace_and_joins() {
        assert_eq!(clean_output("  Hello\n  world  "), "Hello world");
    }

    #[test]
    fn strips_non_speech_tags() {
        assert_eq!(clean_output("[BLANK_AUDIO]"), "");
        assert_eq!(clean_output("Hi\n[silence]\nthere"), "Hi there");
        assert_eq!(clean_output("[inaudible foreign language]"), "");
        assert_eq!(clean_output("(speaking foreign language)"), "");
    }

    #[test]
    fn empty_stays_empty() {
        assert_eq!(clean_output("   \n  "), "");
    }

    #[test]
    fn thread_count_is_bounded() {
        let n = choose_thread_count(2);
        assert!((1..=8).contains(&n));
    }

    #[test]
    fn audio_context_is_bounded_for_dictation_length() {
        assert_eq!(audio_context_for_samples(16_000), 150);
        assert_eq!(audio_context_for_samples(16_000 * 120), 1500);
        assert_eq!(audio_context_for_samples(16_000 * 600), 1500);
    }
}
