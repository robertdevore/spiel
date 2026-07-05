//! Remember and restore the app that had focus before Spiel was clicked.
//!
//! The global hotkey usually leaves focus alone, but menu-bar and settings clicks move
//! focus away from the text field. Keeping a small "last frontmost app" cache lets
//! dictation started from the tray still paste back where the user was typing.

use crate::state::AppState;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusTarget {
    pub pid: i32,
}

const DEFAULT_FOCUS_RESTORE_DELAY_MS: u64 = 140;

pub fn start_tracker(app: AppHandle) {
    let poll_interval = std::env::var("SPIEL_FOCUS_POLL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(|value| value.clamp(100, 5_000))
        .unwrap_or(250);
    let own_pid = std::process::id() as i32;

    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(poll_interval));
        let Some(state) = app.try_state::<AppState>() else {
            continue;
        };
        let Some(target) = frontmost_target() else {
            continue;
        };
        if should_remember(target, own_pid) {
            *state.last_focus_target.lock().unwrap() = Some(target);
        }
    });
}

pub fn remember_current_frontmost(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let Some(target) = frontmost_target() else {
        return;
    };
    if should_remember(target, std::process::id() as i32) {
        *state.last_focus_target.lock().unwrap() = Some(target);
    }
}

pub fn restore_before_paste(target: Option<FocusTarget>) {
    let Some(target) = target else {
        return;
    };
    if activate(target) {
        std::thread::sleep(std::time::Duration::from_millis(env_delay_ms(
            "SPIEL_FOCUS_RESTORE_DELAY_MS",
            DEFAULT_FOCUS_RESTORE_DELAY_MS,
        )));
    }
}

fn should_remember(target: FocusTarget, own_pid: i32) -> bool {
    target.pid > 0 && target.pid != own_pid && !is_system_ui(target)
}

fn env_delay_ms(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(|v| v.clamp(0, 2_000))
        .unwrap_or(default)
}

#[cfg(target_os = "macos")]
fn frontmost_target() -> Option<FocusTarget> {
    use objc2_app_kit::NSWorkspace;

    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication()?;
    let pid = app.processIdentifier();
    if pid <= 0 {
        return None;
    }
    Some(FocusTarget { pid })
}

#[cfg(not(target_os = "macos"))]
fn frontmost_target() -> Option<FocusTarget> {
    None
}

#[cfg(target_os = "macos")]
fn activate(target: FocusTarget) -> bool {
    use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};

    let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(target.pid)
    else {
        return false;
    };
    if app.isTerminated() {
        return false;
    }
    let _ = app.unhide();
    #[allow(deprecated)]
    app.activateWithOptions(
        NSApplicationActivationOptions::ActivateAllWindows
            | NSApplicationActivationOptions::ActivateIgnoringOtherApps,
    )
}

#[cfg(not(target_os = "macos"))]
fn activate(_target: FocusTarget) -> bool {
    false
}

#[cfg(target_os = "macos")]
fn is_system_ui(target: FocusTarget) -> bool {
    use objc2_app_kit::NSRunningApplication;

    let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(target.pid)
    else {
        return false;
    };
    let Some(bundle_id) = app.bundleIdentifier() else {
        return false;
    };
    matches!(
        bundle_id.to_string().as_str(),
        "com.apple.systemuiserver" | "com.apple.dock" | "com.apple.controlcenter"
    )
}

#[cfg(not(target_os = "macos"))]
fn is_system_ui(_target: FocusTarget) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_invalid_or_own_processes() {
        assert!(!should_remember(FocusTarget { pid: -1 }, 100));
        assert!(!should_remember(FocusTarget { pid: 100 }, 100));
    }
}
