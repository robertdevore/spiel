//! Tauri commands — the entire surface the settings window can call. Each is narrow and
//! typed; there is no general-purpose file/shell/SQL access.

use crate::config::Config;
use crate::dictation;
use crate::error::{to_command_error, Result as SpielResult};
use crate::state::{AppState, PerfSnapshot, StatusSnapshot};
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

#[tauri::command]
pub fn get_perf_snapshot(state: State<AppState>) -> PerfSnapshot {
    state.perf_snapshot()
}

#[tauri::command]
pub fn clear_perf_samples(state: State<AppState>) {
    state.clear_perf_samples();
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

    let (old_hotkey, old_model, old_keep_model_loaded) = {
        let c = state.config.lock().unwrap();
        (c.hotkey.clone(), c.model.clone(), c.keep_model_loaded)
    };

    // Reject syntactically-invalid hotkeys before we touch persistence or runtime state.
    if validated.hotkey != old_hotkey {
        crate::validate_hotkey(&validated.hotkey).map_err(to_command_error)?;
    }

    // Keep persisted config and active hotkey consistent. If saving fails after a hotkey
    // change, roll the runtime registration back to the previous value.
    apply_hotkey_and_persist(
        &old_hotkey,
        &validated.hotkey,
        |hotkey| crate::register_hotkey(&app, hotkey),
        || validated.save(&state.paths.config_file),
    )
    .map_err(to_command_error)?;

    *state.config.lock().unwrap() = validated.clone();

    if validated.model != old_model || (old_keep_model_loaded && !validated.keep_model_loaded) {
        dictation::clear_model_cache(&state);
    }

    dictation::emit_status(&app);
    Ok(validated)
}

fn apply_hotkey_and_persist<FRegister, FSave>(
    old_hotkey: &str,
    new_hotkey: &str,
    mut register: FRegister,
    save: FSave,
) -> SpielResult<()>
where
    FRegister: FnMut(&str) -> SpielResult<()>,
    FSave: FnOnce() -> SpielResult<()>,
{
    let hotkey_changed = new_hotkey != old_hotkey;

    if hotkey_changed {
        if let Err(e) = register(new_hotkey) {
            let _ = register(old_hotkey);
            return Err(e);
        }
    }

    if let Err(e) = save() {
        if hotkey_changed {
            let _ = register(old_hotkey);
        }
        return Err(e);
    }

    Ok(())
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
    if model::is_installed(&state.paths.model_dir, &model_id) {
        return Err(format!("Model '{model_id}' is already installed."));
    }

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
        let mut last_emit = std::time::Instant::now();
        let result = model::download(
            &model_dir,
            spec,
            |downloaded, total| {
                if let Some(st) = app_for_progress.try_state::<AppState>() {
                    let mut dl = st.download.lock().unwrap();
                    dl.downloaded = downloaded;
                    dl.total = total;
                }
                let done = total.is_some_and(|t| downloaded >= t);
                let should_emit =
                    done || last_emit.elapsed() >= std::time::Duration::from_millis(120);
                if should_emit {
                    last_emit = std::time::Instant::now();
                    let _ = app_for_progress.emit(
                        "model-progress",
                        ModelProgress {
                            model_id: id_for_progress.clone(),
                            downloaded,
                            total,
                        },
                    );
                }
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
pub fn unload_model_from_memory(app: AppHandle, state: State<AppState>) {
    dictation::clear_model_cache(&state);
    dictation::emit_status(&app);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SpielError;
    use std::cell::RefCell;

    #[test]
    fn registers_new_hotkey_then_persists() {
        let calls = RefCell::new(Vec::<String>::new());

        let result = apply_hotkey_and_persist(
            "Cmd+Alt+D",
            "Cmd+Shift+K",
            |hk| {
                calls.borrow_mut().push(format!("register:{hk}"));
                Ok(())
            },
            || {
                calls.borrow_mut().push("save".into());
                Ok(())
            },
        );

        assert!(result.is_ok());
        assert_eq!(calls.into_inner(), vec!["register:Cmd+Shift+K", "save"]);
    }

    #[test]
    fn rolls_back_hotkey_if_save_fails() {
        let calls = RefCell::new(Vec::<String>::new());

        let result = apply_hotkey_and_persist(
            "Cmd+Alt+D",
            "Cmd+Shift+K",
            |hk| {
                calls.borrow_mut().push(format!("register:{hk}"));
                Ok(())
            },
            || {
                calls.borrow_mut().push("save".into());
                Err(SpielError::Config("disk full".into()))
            },
        );

        assert!(result.is_err());
        assert_eq!(
            calls.into_inner(),
            vec!["register:Cmd+Shift+K", "save", "register:Cmd+Alt+D"]
        );
    }

    #[test]
    fn unchanged_hotkey_only_persists() {
        let calls = RefCell::new(Vec::<String>::new());

        let result = apply_hotkey_and_persist(
            "Cmd+Alt+D",
            "Cmd+Alt+D",
            |hk| {
                calls.borrow_mut().push(format!("register:{hk}"));
                Ok(())
            },
            || {
                calls.borrow_mut().push("save".into());
                Ok(())
            },
        );

        assert!(result.is_ok());
        assert_eq!(calls.into_inner(), vec!["save"]);
    }

    #[test]
    fn restores_previous_hotkey_when_new_registration_fails() {
        let calls = RefCell::new(Vec::<String>::new());

        let result = apply_hotkey_and_persist(
            "Cmd+Alt+D",
            "Cmd+Shift+K",
            |hk| {
                calls.borrow_mut().push(format!("register:{hk}"));
                if hk == "Cmd+Shift+K" {
                    return Err(SpielError::Config("in use".into()));
                }
                Ok(())
            },
            || {
                calls.borrow_mut().push("save".into());
                Ok(())
            },
        );

        assert!(result.is_err());
        assert_eq!(
            calls.into_inner(),
            vec!["register:Cmd+Shift+K", "register:Cmd+Alt+D"]
        );
    }
}
