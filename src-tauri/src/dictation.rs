//! The dictation loop: the one place recording, transcription, and insertion are wired
//! together. The hotkey, the tray menu, and the UI all call [`toggle`] — there is a
//! single code path, so behavior can't drift between entry points (a core flaw of the
//! previous build, where the hotkey and the commands did different things).

use crate::error::SpielError;
use crate::state::{AppState, Phase};
use crate::whisper::Transcriber;
use crate::{audio, insert, model};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Serialize)]
struct TranscriptEvent {
    text: String,
    outcome: insert::InsertOutcome,
}

/// Push the current status to the UI and reflect it in the menu-bar title.
pub fn emit_status(app: &AppHandle) {
    let state = app.state::<AppState>();
    let snap = state.snapshot();

    if let Some(tray) = app.tray_by_id("main") {
        let title = match snap.phase {
            Phase::Recording => "● Rec",
            Phase::Transcribing => "… ",
            Phase::Inserting => "↧ ",
            _ => "",
        };
        let _ = tray.set_title(Some(title));
    }

    let _ = app.emit("status", snap);
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
    if !model::is_installed(&state.paths.model_dir, &model_id) {
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

    state.set_phase(Phase::Transcribing, None);
    emit_status(app);

    // Transcription is CPU-heavy; never block the main thread with it.
    let app = app.clone();
    std::thread::spawn(move || {
        process_capture(&app, capture);
    });
}

fn process_capture(app: &AppHandle, capture: audio::Capture) {
    let state = app.state::<AppState>();

    if capture.is_effectively_silent() {
        state.set_phase(Phase::Idle, Some("No speech detected.".into()));
        emit_status(app);
        return;
    }

    let language = state.config.lock().unwrap().language.clone();

    let transcriber = match ensure_transcriber(&state) {
        Ok(t) => t,
        Err(e) => {
            state.set_phase(Phase::Error, Some(e.to_string()));
            emit_status(app);
            return;
        }
    };

    let text = match transcriber.transcribe(&capture.samples, &language) {
        Ok(t) => t,
        Err(e) => {
            state.set_phase(Phase::Error, Some(e.to_string()));
            emit_status(app);
            return;
        }
    };

    if text.trim().is_empty() {
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

    match insert::insert(&text, auto_paste, restore) {
        Ok(outcome) => {
            {
                let mut s = state.status.lock().unwrap();
                s.needs_accessibility = outcome.needs_accessibility;
            }
            let message = if outcome.needs_accessibility {
                Some("Text copied. Grant Accessibility in Settings to auto-paste.".into())
            } else if outcome.clipboard_only {
                Some("Text copied to clipboard. Press Cmd+V to paste.".into())
            } else {
                None
            };
            state.set_phase(Phase::Idle, message);
            let _ = app.emit("transcript", TranscriptEvent { text, outcome });
        }
        Err(e) => {
            state.set_phase(Phase::Error, Some(e.to_string()));
        }
    }
    emit_status(app);
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

pub fn show_settings_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}
