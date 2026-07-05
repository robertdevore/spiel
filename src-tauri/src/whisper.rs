//! Local transcription via embedded whisper.cpp (the `whisper-rs` crate).
//!
//! The model is loaded once and cached (it's the expensive part — ~150 MB into RAM).
//! Each transcription creates a cheap per-call state, so concurrent calls are safe and
//! the loaded context is reused. Everything runs on-device; there is no network path.

use crate::error::{Result, SpielError};
use std::path::Path;
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

        let ctx = WhisperContext::new_with_params(path_str, WhisperContextParameters::default())
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
}
