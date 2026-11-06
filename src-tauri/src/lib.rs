//! Spiel — local-first push-to-talk dictation for macOS.
//!
//! Shape: a menu-bar (Accessory) app. A global hotkey toggles recording; on stop we
//! transcribe locally with whisper.cpp and paste the result at the cursor. The only
//! settings/status panel is created only when opened so idle RAM stays low.

mod accessibility;
mod audio;
mod commands;
mod config;
mod diagnostics;
mod dictation;
mod downloads;
mod error;
mod focus;
mod hotkey;
mod insert;
mod memory;
mod model;
mod perf;
mod runtime_options;
mod settings_window;
mod state;
mod transcription;
mod whisper;

pub fn run_transcription_worker_from_args(args: &[String]) -> bool {
    whisper::run_worker_from_args(args)
}

use crate::runtime_options::env_flag;
use config::{Config, Paths};
use state::AppState;
use std::path::Component;
use std::path::{Path, PathBuf};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();

            // Resolve on-disk locations and load (or create) settings.
            let config_dir = handle.path().app_config_dir()?;
            let data_dir = handle.path().app_data_dir()?;
            let model_dir = std::env::var_os("SPIEL_MODEL_DIR")
                .filter(|v| !v.is_empty())
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| data_dir.join("models"));
            let model_dir = match resolve_model_dir(model_dir, &data_dir) {
                Ok(path) => path,
                Err(error) => {
                    eprintln!("[spiel] invalid SPIEL_MODEL_DIR, using default: {error}");
                    data_dir.join("models")
                }
            };
            let ttl = model::parse_part_cleanup_ms(
                std::env::var("SPIEL_PART_CLEANUP_MS").ok().as_deref(),
                model::default_part_cleanup_duration().as_millis() as u64,
            );
            let cleanup_report = model::cleanup_stale_model_artifacts(&model_dir, ttl);
            let paths = Paths {
                config_file: config_dir.join("config.json"),
                model_dir,
            };
            let config = match Config::load(&paths.config_file) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("[spiel] settings load failed, using defaults: {e}");
                    let cfg = Config::default();
                    let _ = cfg.save(&paths.config_file);
                    cfg
                }
            };
            let hotkey = config.hotkey.clone();

            app.manage(AppState::new(paths, config));

            // Menu-bar app: no Dock icon, lives in the status bar.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            build_tray(&handle)?;
            // If a saved hotkey is invalid/taken, fall back to the default rather than
            // failing startup — the app must still come up so the user can fix it.
            if hotkey::register(&handle, &hotkey).is_err() {
                let _ = hotkey::register(&handle, &Config::default().hotkey);
            }

            dictation::emit_status(&handle);
            emit_startup_health(handle.clone(), cleanup_report.clone());
            start_model_store_maintenance(handle.clone(), ttl);
            start_optional_model_warmup(handle.clone());

            focus::start_tracker(handle.clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::get_config,
            commands::get_readiness,
            commands::get_startup_health,
            commands::get_perf_snapshot,
            commands::get_memory_snapshot,
            commands::clear_perf_samples,
            commands::update_config,
            commands::list_models,
            commands::download_model,
            commands::delete_model,
            commands::cancel_download,
            commands::toggle_dictation,
            commands::unload_model_from_memory,
            commands::warm_up_model,
            commands::accessibility_status,
            commands::request_accessibility,
            commands::show_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Spiel");
}

/// Build the menu-bar tray icon and its menu.
fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let toggle = MenuItemBuilder::with_id("toggle", "Start / Stop Dictation").build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "Settings…").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit Spiel").build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&toggle)
        .item(&settings)
        .item(&quit)
        .build()?;

    TrayIconBuilder::with_id("main")
        // A monochrome mic, rendered as a macOS template image so it adapts to the
        // light/dark menu bar automatically.
        .icon(tauri::include_image!("icons/tray-mic.png"))
        .icon_as_template(true)
        .tooltip("Spiel")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => dictation::toggle(app),
            "settings" => settings_window::show(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn resolve_model_dir(model_dir: PathBuf, fallback_root: &Path) -> Result<PathBuf, String> {
    let resolved = if model_dir.is_relative() {
        fallback_root.join(model_dir)
    } else {
        model_dir
    };

    validate_path_components(&resolved).map_err(|e| e.to_string())?;

    if resolved.exists() {
        let md = std::fs::metadata(&resolved)
            .map_err(|e| format!("cannot access model directory {resolved:?}: {e}"))?;
        if !md.is_dir() {
            return Err(format!("model directory is not a directory: {resolved:?}"));
        }
    }

    std::fs::create_dir_all(&resolved)
        .map_err(|e| format!("cannot create model directory {resolved:?}: {e}"))?;
    Ok(resolved)
}

fn validate_path_components(path: &Path) -> std::result::Result<(), String> {
    let mut cursor = PathBuf::new();
    for c in path.components() {
        cursor.push(c.as_os_str());
        if let Component::CurDir = c {
            continue;
        }
        if let Component::ParentDir = c {
            return Err("parent directory reference is not allowed in model path".to_string());
        }
        if let Ok(meta) = std::fs::symlink_metadata(&cursor) {
            if meta.file_type().is_symlink() {
                return Err(format!("model path component {cursor:?} is a symlink"));
            }
        }
    }
    Ok(())
}

fn emit_startup_health(app: tauri::AppHandle, cleanup_report: model::CleanupReport) {
    let _ = std::thread::Builder::new()
        .name("spiel-startup-health".into())
        .stack_size(64 * 1024)
        .spawn(move || {
            if let Some(state) = app.try_state::<AppState>() {
                let snapshot = diagnostics::build_startup_health(&state, cleanup_report);
                let _ = app.emit("startup-health", snapshot);
            }
        });
}

fn start_model_store_maintenance(app: tauri::AppHandle, older_than: std::time::Duration) {
    if !env_flag("SPIEL_MAINTENANCE_POLL") {
        return;
    }
    let interval_ms = std::env::var("SPIEL_MAINTENANCE_POLL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(|value| value.clamp(60_000, 86_400_000))
        .unwrap_or(15 * 60 * 1_000);
    let _ = std::thread::Builder::new()
        .name("spiel-model-maintenance".into())
        .stack_size(64 * 1024)
        .spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(interval_ms));
            if let Some(state) = app.try_state::<AppState>() {
                let Some(report) = state.download.when_idle(|| {
                    model::cleanup_stale_model_artifacts(&state.paths.model_dir, older_than)
                }) else {
                    continue;
                };
                if report.removed_partial_files > 0 || report.removed_sidecar_files > 0 {
                    state.clear_model_install_cache();
                    let _ = app.emit(
                        "startup-health",
                        diagnostics::build_startup_health(&state, report),
                    );
                }
            }
        });
}

fn start_optional_model_warmup(app: tauri::AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let config = state.config.lock().unwrap().clone();
    let should_warm = config.keep_model_loaded || env_flag("SPIEL_WARMUP_ON_START");
    if !should_warm {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("spiel-model-warmup".into())
        .stack_size(64 * 1024)
        .spawn(move || {
            let Some(state) = app.try_state::<AppState>() else {
                return;
            };
            let _ = state.transcription.warm_up(&state.paths, &config);
        });
}
