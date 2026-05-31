//! Tauri commands — the entire surface the settings window can call. Each is narrow and
//! typed; there is no general-purpose file/shell/SQL access.

use crate::config::Config;
use crate::dictation;
use crate::error::to_command_error;
use crate::state::{AppState, StatusSnapshot};
use crate::{accessibility, model};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Serialize)]
pub struct ModelView {
    pub id: String,
    pub label: String,
    pub approx_mb: u32,
    pub note: String,
    pub installed: bool,
    pub is_current: bool,
}

#[derive(Clone, Serialize)]
struct ModelProgress {
    model_id: String,
    downloaded: u64,
    total: Option<u64>,
}

#[derive(Clone, Serialize)]
struct ModelDone {
    model_id: String,
    ok: bool,
    error: Option<String>,
}

#[tauri::command]
pub fn get_status(state: State<AppState>) -> StatusSnapshot {
    state.snapshot()
}

#[tauri::command]
pub fn get_config(state: State<AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

/// Validate, persist, and apply a new config. Re-registers the hotkey and invalidates the
/// model cache when those settings change.
#[tauri::command]
pub fn update_config(
    app: AppHandle,
    state: State<AppState>,
    config: Config,
) -> Result<Config, String> {
    let validated = config.validated().map_err(to_command_error)?;

    let (old_hotkey, old_model) = {
        let c = state.config.lock().unwrap();
        (c.hotkey.clone(), c.model.clone())
    };

    validated
        .save(&state.paths.config_file)
        .map_err(to_command_error)?;
    *state.config.lock().unwrap() = validated.clone();

    if validated.hotkey != old_hotkey {
        crate::register_hotkey(&app, &validated.hotkey).map_err(to_command_error)?;
    }
    if validated.model != old_model {
        dictation::clear_model_cache(&state);
    }

    dictation::emit_status(&app);
    Ok(validated)
}

#[tauri::command]
pub fn list_models(state: State<AppState>) -> Vec<ModelView> {
    let current = state.config.lock().unwrap().model.clone();
    model::REGISTRY
        .iter()
        .map(|m| ModelView {
            id: m.id.to_string(),
            label: m.label.to_string(),
            approx_mb: m.approx_mb,
            note: m.note.to_string(),
            installed: model::is_installed(&state.paths.model_dir, m.id),
            is_current: m.id == current,
        })
        .collect()
}

/// Kick off a model download in the background. Progress arrives via `model-progress`
/// events; completion via `model-done`. Returns immediately.
#[tauri::command]
pub fn download_model(
    app: AppHandle,
    state: State<AppState>,
    model_id: String,
) -> Result<(), String> {
    let Some(spec) = model::spec(&model_id) else {
        return Err(format!("Unknown model '{model_id}'."));
    };

    {
        let mut dl = state.download.lock().unwrap();
        if dl.active {
            return Err("A download is already in progress.".into());
        }
        dl.active = true;
        dl.model_id = Some(model_id.clone());
        dl.downloaded = 0;
        dl.total = None;
        dl.cancel = Arc::new(AtomicBool::new(false));
    }
    let cancel = state.download.lock().unwrap().cancel.clone();
    let model_dir = state.paths.model_dir.clone();

    std::thread::spawn(move || {
        let app_for_progress = app.clone();
        let id_for_progress = model_id.clone();
        let result = model::download(
            &model_dir,
            spec,
            |downloaded, total| {
                if let Some(st) = app_for_progress.try_state::<AppState>() {
                    let mut dl = st.download.lock().unwrap();
                    dl.downloaded = downloaded;
                    dl.total = total;
                }
                let _ = app_for_progress.emit(
                    "model-progress",
                    ModelProgress {
                        model_id: id_for_progress.clone(),
                        downloaded,
                        total,
                    },
                );
            },
            || cancel.load(Ordering::Relaxed),
        );

        if let Some(st) = app.try_state::<AppState>() {
            let mut dl = st.download.lock().unwrap();
            dl.active = false;
            dl.model_id = None;
        }

        let done = match &result {
            Ok(()) => ModelDone {
                model_id: model_id.clone(),
                ok: true,
                error: None,
            },
            Err(e) => ModelDone {
                model_id: model_id.clone(),
                ok: false,
                error: Some(e.to_string()),
            },
        };
        let _ = app.emit("model-done", done);
        dictation::emit_status(&app);
    });

    Ok(())
}

#[tauri::command]
pub fn cancel_download(state: State<AppState>) {
    let dl = state.download.lock().unwrap();
    dl.cancel.store(true, Ordering::Relaxed);
}

/// Start or stop dictation (same action the hotkey performs).
#[tauri::command]
pub fn toggle_dictation(app: AppHandle) {
    dictation::toggle(&app);
}

#[tauri::command]
pub fn accessibility_status() -> bool {
    accessibility::is_trusted()
}

/// Trigger the macOS prompt and open the settings pane to guide the user.
#[tauri::command]
pub fn request_accessibility() -> bool {
    let trusted = accessibility::prompt_if_needed();
    if !trusted {
        accessibility::open_settings_pane();
    }
    trusted
}

#[tauri::command]
pub fn show_settings(app: AppHandle) {
    dictation::show_settings_window(&app);
}
