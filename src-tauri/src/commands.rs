//! Tauri commands — the entire surface the settings window can call. Each is narrow and
//! typed; there is no general-purpose file/shell/SQL access.

use crate::config::Config;
use crate::diagnostics::{self, ReadinessSnapshot, StartupHealthSnapshot};
use crate::dictation;
use crate::error::to_command_error;
use crate::memory::MemorySnapshot;
use crate::state::{AppState, DownloadPerfSample, PerfSnapshot, StatusSnapshot};
use crate::{accessibility, memory, model};
use serde::Serialize;
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
    outcome: String,
    downloaded_bytes: u64,
    expected_bytes: Option<u64>,
    checksum_source: String,
}

#[tauri::command]
pub fn get_readiness(state: State<AppState>) -> ReadinessSnapshot {
    diagnostics::readiness(&state)
}

#[tauri::command]
pub fn get_startup_health(state: State<AppState>) -> StartupHealthSnapshot {
    diagnostics::build_startup_health(&state, model::CleanupReport::default())
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
pub fn get_memory_snapshot() -> MemorySnapshot {
    memory::snapshot()
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
        crate::hotkey::validate(&validated.hotkey).map_err(to_command_error)?;
    }

    // Keep persisted config and active hotkey consistent. If saving fails after a hotkey
    // change, roll the runtime registration back to the previous value.
    crate::hotkey::apply_and_persist(
        &old_hotkey,
        &validated.hotkey,
        |hotkey| crate::hotkey::register(&app, hotkey),
        || validated.save(&state.paths.config_file),
    )
    .map_err(to_command_error)?;

    *state.config.lock().unwrap() = validated.clone();

    if validated.model != old_model || (old_keep_model_loaded && !validated.keep_model_loaded) {
        state.transcription.clear();
    }
    state.clear_model_install_cache();

    dictation::emit_status(&app);
    Ok(validated)
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
    let reservation = state
        .download
        .reserve(&model_id)
        .ok_or_else(|| "A download is already in progress.".to_string())?;

    let partial_ttl =
        model::parse_part_cleanup_ms(std::env::var("SPIEL_PART_CLEANUP_MS").ok().as_deref(), 0);
    model::cleanup_stale_model_artifacts(&state.paths.model_dir, partial_ttl);
    if model::is_installed(&state.paths.model_dir, &model_id) {
        return Err(format!("Model '{model_id}' is already installed."));
    }
    state.clear_model_install_cache_entry(&model_id);

    let model_dir = state.paths.model_dir.clone();

    std::thread::spawn(move || {
        let app_for_progress = app.clone();
        let id_for_progress = model_id.clone();
        let mut last_emit = std::time::Instant::now();
        let started_at = std::time::Instant::now();
        let result = model::download(
            &model_dir,
            spec,
            |downloaded, total| {
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
            || reservation.is_canceled(),
        );

        if let Some(st) = app.try_state::<AppState>() {
            st.clear_model_install_cache_entry(&model_id);
        }

        drop(reservation);
        let done = match &result {
            Ok(summary) => ModelDone {
                model_id: model_id.clone(),
                ok: true,
                error: None,
                outcome: "downloaded".into(),
                downloaded_bytes: summary.downloaded_bytes,
                expected_bytes: summary.expected_bytes,
                checksum_source: summary.checksum_source.clone(),
            },
            Err(e) => ModelDone {
                model_id: model_id.clone(),
                ok: false,
                error: Some(e.to_string()),
                outcome: model::classify_download_error(e).into(),
                downloaded_bytes: 0,
                expected_bytes: None,
                checksum_source: "none".into(),
            },
        };
        if let Some(st) = app.try_state::<AppState>() {
            st.record_download_sample(DownloadPerfSample {
                wall_time_ms: 0,
                total_ms: started_at.elapsed().as_millis() as u64,
                downloaded_bytes: done.downloaded_bytes,
                expected_bytes: done.expected_bytes,
                outcome: done.outcome.clone(),
            });
        }
        let _ = app.emit("model-done", done);
        if let Some(st) = app.try_state::<AppState>() {
            st.clear_model_install_cache_entry(&model_id);
            let current_model = st.config.lock().unwrap().model.clone();
            if current_model == model_id && result.is_ok() {
                st.transcription.clear();
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

    if state.download.is_downloading(&model_id) {
        return Err("Cannot delete a model while it is downloading.".into());
    }

    let model_path = state.paths.model_path(spec.filename);
    if !model_path.exists() {
        return Err(format!("Model '{model_id}' is not installed."));
    }

    let current_model = state.config.lock().unwrap().model.clone();
    if current_model == model_id {
        return Err("Cannot delete the active model directly. Switch models first.".into());
    }

    state.transcription.clear();

    model::delete_files(&state.paths.model_dir, spec).map_err(to_command_error)?;

    state.clear_model_install_cache_entry(&model_id);
    Ok(())
}

#[tauri::command]
pub fn cancel_download(state: State<AppState>) {
    state.download.cancel();
}

/// Start or stop dictation (same action the hotkey performs).
#[tauri::command]
pub fn toggle_dictation(app: AppHandle) {
    dictation::toggle(&app);
}

#[tauri::command]
pub fn unload_model_from_memory(app: AppHandle, state: State<AppState>) {
    state.transcription.clear();
    dictation::emit_status(&app);
}

#[tauri::command]
pub fn warm_up_model(state: State<AppState>) -> Result<String, String> {
    let config = state.config.lock().unwrap().clone();
    state
        .transcription
        .warm_up(&state.paths, &config)
        .map_err(to_command_error)?;
    Ok(if config.keep_model_loaded {
        "Model warmed and kept in memory.".into()
    } else {
        "Model load path validated. Enable 'Keep model loaded in memory' to keep it hot.".into()
    })
}

#[tauri::command]
pub fn accessibility_status() -> bool {
    accessibility::is_trusted()
}

/// Trigger the macOS prompt and open the settings pane to guide the user.
#[tauri::command]
pub fn request_accessibility(app: AppHandle) -> bool {
    let trusted = accessibility::prompt_if_needed();
    if !trusted {
        accessibility::open_settings_pane();
    }
    dictation::emit_status(&app);
    trusted
}

#[tauri::command]
pub fn show_settings(app: AppHandle) {
    crate::settings_window::show(&app);
}
