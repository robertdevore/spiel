//! Spiel — local-first push-to-talk dictation for macOS.
//!
//! Shape: a menu-bar (Accessory) app. A global hotkey toggles recording; on stop we
//! transcribe locally with whisper.cpp and paste the result at the cursor. The only
//! window is a settings/status panel, hidden by default and shown from the tray.

mod accessibility;
mod audio;
mod commands;
mod config;
mod dictation;
mod error;
mod insert;
mod model;
mod state;
mod whisper;

use config::{Config, Paths};
use state::AppState;
use std::str::FromStr;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();

            // Resolve on-disk locations and load (or create) settings.
            let config_dir = handle.path().app_config_dir()?;
            let data_dir = handle.path().app_data_dir()?;
            let paths = Paths {
                config_file: config_dir.join("config.json"),
                model_dir: data_dir.join("models"),
            };
            let config = Config::load(&paths.config_file).unwrap_or_default();
            let hotkey = config.hotkey.clone();

            app.manage(AppState::new(paths, config));

            // Menu-bar app: no Dock icon, lives in the status bar.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            build_tray(&handle)?;
            register_hotkey(&handle, &hotkey).map_err(|e| std::io::Error::other(e.to_string()))?;

            // Closing the settings window hides it; the app keeps running in the menu bar.
            if let Some(win) = app.get_webview_window("main") {
                let win_clone = win.clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                    }
                });
            }

            dictation::emit_status(&handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::get_config,
            commands::update_config,
            commands::list_models,
            commands::download_model,
            commands::cancel_download,
            commands::toggle_dictation,
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

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Spiel")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => dictation::toggle(app),
            "settings" => dictation::show_settings_window(app),
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

/// (Re)register the global toggle hotkey. Replaces any previously registered shortcut.
pub fn register_hotkey(app: &tauri::AppHandle, spec: &str) -> error::Result<()> {
    let shortcut = parse_shortcut(spec);
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    gs.on_shortcut(shortcut, move |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            dictation::toggle(app);
        }
    })
    .map_err(|e| {
        error::SpielError::Other(format!(
            "Could not register hotkey '{spec}': {e}. It may be in use by another app."
        ))
    })
}

/// Parse a hotkey string (e.g. "Cmd+Alt+D"). Falls back to the default if unparseable.
fn parse_shortcut(spec: &str) -> Shortcut {
    Shortcut::from_str(spec)
        .unwrap_or_else(|_| Shortcut::new(Some(Modifiers::SUPER | Modifiers::ALT), Code::KeyD))
}
