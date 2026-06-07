//! Tauri commands — the entire surface the settings window can call. Each is narrow and
//! typed; there is no general-purpose file/shell/SQL access.

use crate::config::Config;
use crate::dictation;
use crate::error::{to_command_error, Result as SpielResult};
use crate::state::{AppState, PerfSnapshot, StatusSnapshot};
use crate::{accessibility, model};
use serde::Serialize;
use std::io::Write;
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
    pub install_status: String,
    pub install_bytes: u64,
    pub install_modified_ms: Option<u64>,
    pub install_reason: String,
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

#[derive(Serialize)]
pub struct ReadinessSnapshot {
    pub model_dir: String,
    pub model_dir_writable: bool,
    pub current_model: String,
    pub current_model_installed: bool,
    pub model_store_bytes: u64,
    pub model_store_file_count: usize,
    pub hotkey_valid: bool,
    pub accessibility_supported: bool,
    pub accessibility_trusted: bool,
    pub active_download: bool,
}

#[tauri::command]
pub fn get_readiness(state: State<AppState>) -> ReadinessSnapshot {
    let config = state.config.lock().unwrap().clone();
    let current_model = config.model.clone();
    let model_dir = state.paths.model_dir.clone();
    let model_dir_writable = probe_model_dir_writable(&model_dir);
    let (model_store_bytes, model_store_file_count) = summarize_model_dir(&model_dir);

    ReadinessSnapshot {
        model_dir: model_dir.to_string_lossy().to_string(),
        model_dir_writable,
        current_model,
        current_model_installed: state.model_install_info(&config.model).is_installed(),
        model_store_bytes,
        model_store_file_count,
        hotkey_valid: crate::validate_hotkey(&config.hotkey).is_ok(),
        accessibility_supported: crate::accessibility::is_supported(),
        accessibility_trusted: crate::accessibility::is_supported()
            && crate::accessibility::is_trusted(),
        active_download: state.download.lock().unwrap().active,
    }
}

fn probe_model_dir_writable(model_dir: &std::path::Path) -> bool {
    if model::is_safe_model_path(model_dir, "model directory").is_err() {
        return false;
    }
    let probe_path = model_dir.join(format!(
        ".spiel-write-test.{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    let write_result = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe_path)
        .and_then(|mut f| f.write_all(b"1"));

    let was_writable = write_result.is_ok();
    let _ = std::fs::remove_file(&probe_path);
    was_writable
}

fn summarize_model_dir(model_dir: &std::path::Path) -> (u64, usize) {
    let mut total_bytes: u64 = 0;
    let mut file_count: usize = 0;

    let entries = match std::fs::read_dir(model_dir) {
        Ok(entries) => entries,
        Err(_) => return (0, 0),
    };

    for entry in entries.filter_map(std::result::Result::ok) {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if !(name.ends_with(".bin") || name.ends_with(".part")) {
            continue;
        }

        let metadata = match entry.file_type() {
            Ok(ft) if ft.is_symlink() => continue,
            Ok(ft) if !ft.is_file() => continue,
            _ => match entry.path().metadata() {
                Ok(meta) => meta,
                Err(_) => continue,
            },
        };

        if !metadata.is_file() {
            continue;
        }

        file_count = file_count.saturating_add(1);
        total_bytes = total_bytes.saturating_add(metadata.len());
    }

    (total_bytes, file_count)
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
    state.clear_model_install_cache();

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
        .map(|m| {
            let install = state.model_install_info(m.id);
            ModelView {
                id: m.id.to_string(),
                label: m.label.to_string(),
                approx_mb: m.approx_mb,
                note: m.note.to_string(),
                installed: install.is_installed(),
                install_status: install.as_label().to_string(),
                install_bytes: install.bytes,
                install_modified_ms: install.modified_ms,
                install_reason: install.reason.clone(),
                is_current: m.id == current,
            }
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
    let partial_ttl =
        model::parse_part_cleanup_ms(std::env::var("SPIEL_PART_CLEANUP_MS").ok().as_deref(), 0);
    model::cleanup_stale_part_files(&state.paths.model_dir, partial_ttl);
    if model::is_installed(&state.paths.model_dir, &model_id) {
        return Err(format!("Model '{model_id}' is already installed."));
    }
    state.clear_model_install_cache_entry(&model_id);

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
            st.clear_model_install_cache_entry(&model_id);
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
        if let Some(st) = app.try_state::<AppState>() {
            st.clear_model_install_cache_entry(&model_id);
            let current_model = st.config.lock().unwrap().model.clone();
            if current_model == model_id && result.is_ok() {
                dictation::clear_model_cache(&st);
            }
        }
        dictation::emit_status(&app);
    });

    Ok(())
}

#[tauri::command]
pub fn delete_model(state: State<AppState>, model_id: String) -> Result<(), String> {
    let Some(spec) = model::spec(&model_id) else {
        return Err(format!("Unknown model '{model_id}'."));
    };

    {
        let dl = state.download.lock().unwrap();
        if dl.active && dl.model_id.as_deref() == Some(model_id.as_str()) {
            return Err("Cannot delete a model while it is downloading.".into());
        }
    }

    let model_path = state.paths.model_path(spec.filename);
    if !model_path.exists() {
        return Err(format!("Model '{model_id}' is not installed."));
    }

    let current_model = state.config.lock().unwrap().model.clone();
    if current_model == model_id {
        return Err("Cannot delete the active model directly. Switch models first.".into());
    }

    {
        let mut transcriber = state.transcriber.lock().unwrap();
        let _previous = transcriber.take();
    }

    std::fs::remove_file(&model_path).map_err(|e| format!("Failed to remove '{model_id}': {e}"))?;

    let part_path = state
        .paths
        .model_dir
        .join(format!("{}.part", spec.filename));
    let _ = std::fs::remove_file(part_path);

    state.clear_model_install_cache_entry(&model_id);
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
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn summarize_model_dir_counts_model_files_only() {
        let dir = std::env::temp_dir().join(format!(
            "spiel-readiness-summary-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));

        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("ggml-tiny.bin"), vec![0_u8; 1024]).unwrap();
        fs::write(dir.join("ggml-tiny.bin.part"), vec![0_u8; 2048]).unwrap();

        let (bytes, files) = summarize_model_dir(&dir);
        assert_eq!(files, 2);
        assert_eq!(bytes, 3072);

        fs::remove_file(dir.join("ggml-tiny.bin")).unwrap();
        fs::remove_file(dir.join("ggml-tiny.bin.part")).unwrap();
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn summarize_model_dir_ignores_symlink_targets() {
        use std::os::unix::fs::symlink;

        let dir = std::env::temp_dir().join(format!(
            "spiel-readiness-summary-symlink-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let target = std::env::temp_dir().join(format!(
            "spiel-readiness-target-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::write(&target, vec![0_u8; 128]).unwrap();
        let link = dir.join("ggml-small.bin");
        symlink(&target, &link).unwrap();

        let (bytes, files) = summarize_model_dir(&dir);
        assert_eq!(bytes, 0);
        assert_eq!(files, 0);

        fs::remove_file(&target).unwrap();
        fs::remove_file(link).unwrap();
        fs::remove_dir_all(&dir).unwrap();
    }
}
