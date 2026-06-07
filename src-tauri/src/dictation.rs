//! The dictation loop: the one place recording, transcription, and insertion are wired
//! together. The hotkey, the tray menu, and the UI all call [`toggle`] — there is a
//! single code path, so behavior can't drift between entry points (a core flaw of the
//! previous build, where the hotkey and the commands did different things).

use crate::error::SpielError;
use crate::state::{AppState, PerfSample, Phase};
use crate::whisper::Transcriber;
use crate::{audio, insert, model};
use serde::Serialize;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Serialize)]
struct TranscriptEvent {
    text: String,
    outcome: insert::InsertOutcome,
}

#[derive(Clone, Serialize)]
struct PerfEvent {
    capture_ms: u64,
    transcribe_ms: u64,
    insert_ms: u64,
    total_ms: u64,
    text_chars: usize,
    outcome: String,
}

/// Push the current status to the UI and reflect it in the menu-bar title.
///
/// Safe to call from any thread: `emit` is thread-safe, and the tray title update (an
/// NSStatusItem mutation) is marshaled onto the main thread, which macOS requires.
pub fn emit_status(app: &AppHandle) {
    let state = app.state::<AppState>();
    let snap = state.snapshot();
    let _ = app.emit("status", snap.clone());

    let phase = snap.phase;
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        if let Some(tray) = app.tray_by_id("main") {
            let title = match phase {
                Phase::Recording => "● Rec",
                Phase::Transcribing => "… ",
                Phase::Inserting => "↧ ",
                _ => "",
            };
            let _ = tray.set_title(Some(title));
        }
    });
}

/// Start recording if idle, or stop-and-process if already recording.
/// Busy phases (transcribing/inserting) are ignored so a stray keypress can't corrupt
/// an in-flight job.
pub fn toggle(app: &AppHandle) {
    let state = app.state::<AppState>();
    let phase = state.status.lock().unwrap().phase;

    match phase {
        Phase::Recording => stop_and_process(app),
        Phase::Idle | Phase::Error => start(app),
        Phase::Transcribing | Phase::Inserting => { /* busy — ignore */ }
    }
}

fn start(app: &AppHandle) {
    let state = app.state::<AppState>();

    let (model_id, max_seconds) = {
        let c = state.config.lock().unwrap();
        (c.model.clone(), c.max_seconds)
    };

    // Refuse to "record into the void": without a model there's nothing to transcribe.
    if !state.model_install_info(&model_id).is_installed() {
        state.set_phase(
            Phase::Error,
            Some("Speech model not installed. Open Settings to download it.".into()),
        );
        emit_status(app);
        show_settings_window(app);
        return;
    }

    match audio::start(max_seconds) {
        Ok(recorder) => {
            *state.recorder.lock().unwrap() = Some(recorder);
            state.set_phase(Phase::Recording, None);
        }
        Err(e) => {
            state.set_phase(Phase::Error, Some(e.to_string()));
        }
    }
    emit_status(app);
}

fn stop_and_process(app: &AppHandle) {
    let state = app.state::<AppState>();
    let stop_pressed_at = Instant::now();

    let recorder = state.recorder.lock().unwrap().take();
    let Some(recorder) = recorder else {
        state.set_phase(Phase::Idle, None);
        emit_status(app);
        return;
    };

    let capture = match recorder.finish() {
        Ok(c) => c,
        Err(e) => {
            state.set_phase(Phase::Error, Some(e.to_string()));
            emit_status(app);
            return;
        }
    };
    let capture_ms = stop_pressed_at.elapsed().as_millis() as u64;

    state.set_phase(Phase::Transcribing, None);
    emit_status(app);

    // Transcription is CPU-heavy; never block the main thread with it.
    let app = app.clone();
    std::thread::spawn(move || {
        process_capture(&app, capture, capture_ms, stop_pressed_at);
    });
}

fn process_capture(
    app: &AppHandle,
    capture: audio::Capture,
    capture_ms: u64,
    stop_pressed_at: Instant,
) {
    let state = app.state::<AppState>();
    let sample_count = capture.samples.len();

    if capture.is_effectively_silent() {
        state.record_perf_sample(PerfSample {
            wall_time_ms: 0,
            capture_ms,
            transcribe_ms: 0,
            insert_ms: 0,
            total_ms: stop_pressed_at.elapsed().as_millis() as u64,
            audio_samples: sample_count,
            text_chars: 0,
            outcome: "silent".into(),
        });
        state.set_phase(Phase::Idle, Some("No speech detected.".into()));
        emit_status(app);
        return;
    }

    let (language, threads, keep_model_loaded) = {
        let c = state.config.lock().unwrap();
        (
            c.language.clone(),
            c.transcription_threads,
            c.keep_model_loaded,
        )
    };

    let transcribe_started_at = Instant::now();
    let transcriber = match ensure_transcriber(&state) {
        Ok(t) => t,
        Err(e) => {
            state.record_perf_sample(PerfSample {
                wall_time_ms: 0,
                capture_ms,
                transcribe_ms: transcribe_started_at.elapsed().as_millis() as u64,
                insert_ms: 0,
                total_ms: stop_pressed_at.elapsed().as_millis() as u64,
                audio_samples: sample_count,
                text_chars: 0,
                outcome: "model_load_error".into(),
            });
            state.set_phase(Phase::Error, Some(e.to_string()));
            emit_status(app);
            return;
        }
    };

    let text = match transcriber.transcribe(&capture.samples, &language, threads) {
        Ok(t) => t,
        Err(e) => {
            state.record_perf_sample(PerfSample {
                wall_time_ms: 0,
                capture_ms,
                transcribe_ms: transcribe_started_at.elapsed().as_millis() as u64,
                insert_ms: 0,
                total_ms: stop_pressed_at.elapsed().as_millis() as u64,
                audio_samples: sample_count,
                text_chars: 0,
                outcome: "transcription_error".into(),
            });
            state.set_phase(Phase::Error, Some(e.to_string()));
            emit_status(app);
            return;
        }
    };
    let transcribe_ms = transcribe_started_at.elapsed().as_millis() as u64;
    drop(capture);
    if !keep_model_loaded {
        clear_model_cache(&state);
    }

    if text.trim().is_empty() {
        state.record_perf_sample(PerfSample {
            wall_time_ms: 0,
            capture_ms,
            transcribe_ms,
            insert_ms: 0,
            total_ms: stop_pressed_at.elapsed().as_millis() as u64,
            audio_samples: sample_count,
            text_chars: 0,
            outcome: "empty".into(),
        });
        state.set_phase(Phase::Idle, Some("No speech detected.".into()));
        emit_status(app);
        return;
    }

    state.set_phase(Phase::Inserting, None);
    emit_status(app);

    let (auto_paste, restore) = {
        let c = state.config.lock().unwrap();
        (c.auto_paste, c.restore_clipboard)
    };

    // Clipboard access and Cmd+V synthesis go through AppKit/CoreGraphics, which must run
    // on the main thread — doing this on the worker thread crashes the app. Marshal it.
    let app = app.clone();
    let text_chars = text.chars().count();
    let insert_started_at = Instant::now();
    let _ = app.clone().run_on_main_thread(move || {
        let state = app.state::<AppState>();
        let mut sample_outcome = "insert_error".to_string();
        match insert::insert(&text, auto_paste, restore) {
            Ok(outcome) => {
                state.status.lock().unwrap().needs_accessibility = outcome.needs_accessibility;
                let message = if outcome.needs_accessibility {
                    Some("Text copied. Grant Accessibility in Settings to auto-paste.".into())
                } else if outcome.clipboard_only {
                    Some("Text copied to clipboard. Press Cmd+V to paste.".into())
                } else {
                    None
                };
                sample_outcome = if outcome.pasted {
                    "pasted".into()
                } else if outcome.clipboard_only {
                    "clipboard_only".into()
                } else {
                    "insert_ok".into()
                };
                state.set_phase(Phase::Idle, message);
                let _ = app.emit("transcript", TranscriptEvent { text, outcome });
            }
            Err(e) => {
                state.set_phase(Phase::Error, Some(e.to_string()));
            }
        }
        let sample = PerfSample {
            wall_time_ms: 0,
            capture_ms,
            transcribe_ms,
            insert_ms: insert_started_at.elapsed().as_millis() as u64,
            total_ms: stop_pressed_at.elapsed().as_millis() as u64,
            audio_samples: sample_count,
            text_chars,
            outcome: sample_outcome.clone(),
        };
        state.record_perf_sample(sample.clone());
        let _ = app.emit(
            "perf",
            PerfEvent {
                capture_ms: sample.capture_ms,
                transcribe_ms: sample.transcribe_ms,
                insert_ms: sample.insert_ms,
                total_ms: sample.total_ms,
                text_chars: sample.text_chars,
                outcome: sample_outcome,
            },
        );
        emit_status(&app);
    });
}

/// Return the cached model context, loading (and caching) it if absent or stale.
fn ensure_transcriber(state: &AppState) -> crate::error::Result<Transcriber> {
    let model_id = state.config.lock().unwrap().model.clone();
    let spec = model::spec(&model_id).ok_or(SpielError::ModelMissing)?;
    let path = state.paths.model_path(spec.filename);

    let mut guard = state.transcriber.lock().unwrap();
    if let Some(t) = guard.as_ref() {
        if t.model_id == model_id {
            return Ok(t.clone());
        }
    }
    let loaded = Transcriber::load(&path, &model_id)?;
    *guard = Some(loaded.clone());
    Ok(loaded)
}

/// Invalidate the cached model (call after the model setting changes).
pub fn clear_model_cache(state: &AppState) {
    *state.transcriber.lock().unwrap() = None;
}

pub fn warm_up_current_model(state: &AppState, keep_loaded: bool) -> crate::error::Result<()> {
    let _ = ensure_transcriber(state)?;
    if !keep_loaded {
        clear_model_cache(state);
    }
    Ok(())
}

pub fn show_settings_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}
