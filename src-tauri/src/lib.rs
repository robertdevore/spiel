pub mod app_state;
pub mod audio;
pub mod cleanup;
pub mod cleanup_basic;
pub mod cleanup_mock_ai;
pub mod cleanup_openai;
pub mod clipboard;
pub mod commands;
pub mod database;
pub mod history;
pub mod modes;
pub mod secrets;
pub mod settings;
pub mod transcription;
pub mod transcription_openai;
pub mod transcription_whisper;
pub mod workflow;

use app_state::AppState;
use commands::{ActiveRecording, DatabaseHandle};
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// Platform-appropriate default hotkey for Spiel.
fn default_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::SUPER.union(Modifiers::SHIFT)), Code::KeyS)
}

const DEFAULT_SHORTCUT_LABEL: &str = "Cmd+Shift+S";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .manage(ActiveRecording {
            handle: Mutex::new(None),
        })
        .manage(DatabaseHandle {
            db: Mutex::new(None),
        })
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            // Initialize database
            let db_path = app
                .path()
                .app_local_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join("spiel_history.db");

            match database::Database::open(&db_path) {
                Ok(database) => {
                    // Load settings after DB init
                    match settings::load_settings(&database) {
                        Ok(loaded) => {
                            let app_state = app.state::<AppState>();
                            let mut cache = app_state.settings.lock().unwrap();
                            *cache = loaded;
                        }
                        Err(e) => {
                            eprintln!("[Spiel] Failed to load settings: {}", e);
                        }
                    }

                    let db_handle = app.state::<DatabaseHandle>();
                    let mut db = db_handle.db.lock().unwrap();
                    *db = Some(database);
                }
                Err(e) => {
                    eprintln!("[Spiel] Failed to initialize database: {}", e);
                    let history_state = app.state::<AppState>();
                    let mut hs = history_state.history_state.lock().unwrap();
                    hs.error = Some(format!("Database unavailable: {}", e));
                }
            }
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                let _ = window.eval("console.log('Spiel backend initialized (debug)');");
            }

            // Register the global shortcut
            let state = app.state::<AppState>();
            let shortcut = default_shortcut();
            let shortcut_label = DEFAULT_SHORTCUT_LABEL.to_string();

            match app.global_shortcut().register(shortcut) {
                Ok(_) => {
                    let mut hotkey = state.hotkey.lock().unwrap();
                    hotkey.registered = true;
                    hotkey.error = None;
                    hotkey.shortcut = shortcut_label.clone();
                }
                Err(e) => {
                    let mut hotkey = state.hotkey.lock().unwrap();
                    hotkey.registered = false;
                    hotkey.error = Some(format!(
                        "Failed to register shortcut '{}': {}. The shortcut may be in use by another application.",
                        shortcut_label, e
                    ));
                    hotkey.shortcut = shortcut_label;
                }
            }

            // Listen for shortcut triggers — toggle recording
            let app_handle = app.handle().clone();
            let _ = app.global_shortcut().on_shortcut(shortcut, move |_app, _sc, event| {
                if event.state() == ShortcutState::Pressed {
                    let state = app_handle.state::<AppState>();
                    let mut hotkey = state.hotkey.lock().unwrap();
                    hotkey.last_triggered = Some(crate::chrono_now_iso());
                    hotkey.trigger_count += 1;
                    drop(hotkey);

                    // Toggle recording: if recording, stop; if idle/complete, start
                    // Also update workflow state
                    let should_start = {
                        let rec = state.recording.lock().unwrap();
                        rec.state != app_state::RecordingState::Recording
                    };

                    if should_start {
                        let mut wf = state.workflow.lock().unwrap();
                        wf.step = workflow::WorkflowStep::Recording;
                        drop(wf);

                        // Try to start recording via the command logic
                        let active = app_handle.state::<ActiveRecording>();
                        let mut rec = state.recording.lock().unwrap();
                        if rec.state == app_state::RecordingState::Recording {
                            return; // Already recording (race condition check)
                        }
                        match audio::start_recording() {
                            Ok(handle) => {
                                let now = crate::chrono_now_iso();
                                rec.state = app_state::RecordingState::Recording;
                                rec.started_at = Some(now);
                                rec.error = None;
                                rec.elapsed_ms = 0;
                                let mut active_handle = active.handle.lock().unwrap();
                                *active_handle = Some(handle);
                            }
                            Err(e) => {
                                rec.state = app_state::RecordingState::Error;
                                rec.error = Some(e.to_string());
                            }
                        }
                    } else {
                        // Stop recording
                        let active = app_handle.state::<ActiveRecording>();
                        let mut rec = state.recording.lock().unwrap();
                        if rec.state != app_state::RecordingState::Recording {
                            return;
                        }
                        rec.state = app_state::RecordingState::Stopping;
                        let mut active_handle = active.handle.lock().unwrap();
                        if let Some(handle) = active_handle.take() {
                            match handle.stop() {
                                Ok(meta) => {
                                    let filename = meta.filename.clone();
                                    rec.last_recording = Some(app_state::LastRecording {
                                        file_path: meta.file_path,
                                        filename: meta.filename,
                                        duration_ms: meta.duration_ms,
                                        sample_rate: meta.sample_rate,
                                        channels: meta.channels,
                                        size_bytes: meta.size_bytes,
                                        created_at: meta.created_at,
                                        device_name: meta.device_name,
                                    });
                                    rec.state = app_state::RecordingState::Complete;
                                    rec.error = None;
                                    rec.started_at = None;
                                    rec.elapsed_ms = 0;
                                    // Update workflow state
                                    let mut wf = state.workflow.lock().unwrap();
                                    wf.step = workflow::WorkflowStep::RecordingComplete;
                                    wf.last_recording_filename = Some(filename);
                                    wf.last_recording_duration_ms = Some(meta.duration_ms);
                                    drop(wf);
                                }
                                Err(e) => {
                                    rec.state = app_state::RecordingState::Error;
                                    rec.error = Some(e.to_string());
                                }
                            }
                        }
                    }

                    // Notify frontend
                    let _ = app_handle.emit("hotkey-triggered", ());
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_status,
            commands::get_app_info,
            commands::echo_preview_text,
            commands::get_hotkey_status,
            commands::start_recording,
            commands::stop_recording,
            commands::get_recording_status,
            commands::clear_last_recording,
            commands::copy_to_clipboard,
            commands::insert_via_clipboard,
            commands::restore_clipboard,
            commands::get_clipboard_text,
            commands::get_transcription_status,
            commands::transcribe_last_recording_mock,
            commands::get_available_transcription_engines,
            commands::clear_transcript,
            commands::validate_local_whisper_config,
            commands::get_whisper_config,
            commands::update_whisper_config,
            commands::transcribe_last_recording_local,
            commands::get_text_modes,
            commands::get_cleanup_providers,
            commands::run_cleanup,
            commands::clear_final_text,
            commands::get_cleanup_status,
            commands::save_history_entry,
            commands::list_history_entries,
            commands::get_history_entry,
            commands::delete_history_entry,
            commands::clear_history,
            commands::get_history_status,
            commands::set_history_enabled,
            commands::get_settings,
            commands::update_settings,
            commands::reset_settings,
            commands::start_workflow_recording,
            commands::stop_workflow_recording,
            commands::run_workflow_transcription,
            commands::run_workflow_cleanup,
            commands::insert_workflow_final_text,
            commands::save_workflow_to_history,
            commands::cancel_workflow,
            commands::get_workflow_status,
            commands::reset_workflow,
            commands::get_privacy_status,
            commands::set_openai_api_key,
            commands::get_openai_api_key_status,
            commands::delete_openai_api_key,
            commands::validate_openai_provider_config,
            commands::transcribe_with_openai,
            commands::cleanup_with_openai,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Spiel");
}

/// Returns the current UTC time as an ISO 8601 string.
#[doc(hidden)]
pub fn chrono_now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    let (year, month, day) = civil_from_days(days_since_epoch as i64);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
