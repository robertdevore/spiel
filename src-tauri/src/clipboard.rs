//! Clipboard operations for Spiel Phase 4.
//! Wraps `tauri-plugin-clipboard-manager` for text clipboard read/write.
//!
//! Architecture:
//! - `read_text()` reads current plain text from clipboard
//! - `write_text()` writes text to clipboard
//! - `SavedClipboard` holds previous clipboard state for restore
//!
//! Paste simulation (Cmd+V / Ctrl+V) is NOT implemented in Phase 4.
//! Users manually paste after copying. Automatic paste requires
//! accessibility permissions (planned for a future phase).

use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

/// Holds saved clipboard state for later restoration.
#[derive(Debug, Clone)]
pub struct SavedClipboard {
    /// Whether the previous clipboard contained plain text
    pub had_text: bool,
    /// The saved text, if available
    pub text: Option<String>,
}

impl SavedClipboard {
    /// Create an empty saved state (no previous text).
    pub fn empty() -> Self {
        Self {
            had_text: false,
            text: None,
        }
    }
}

/// Read plain text from the system clipboard.
/// Returns empty string if no text is available.
pub fn read_text(app: &AppHandle) -> Result<String, String> {
    app.clipboard()
        .read_text()
        .map_err(|e| format!("Failed to read clipboard: {}", e))
}

/// Write plain text to the system clipboard.
pub fn write_text(app: &AppHandle, text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Err("Cannot write empty text to clipboard.".into());
    }
    app.clipboard()
        .write_text(text.to_string())
        .map_err(|e| format!("Failed to write clipboard: {}", e))
}

/// Save current clipboard contents and write new text.
/// Returns the saved state for later restoration.
pub fn save_and_replace(app: &AppHandle, new_text: &str) -> Result<SavedClipboard, String> {
    // Try to read current clipboard for later restore
    let saved = match read_text(app) {
        Ok(prev) if !prev.is_empty() => SavedClipboard {
            had_text: true,
            text: Some(prev),
        },
        Ok(_) => SavedClipboard {
            had_text: false,
            text: None,
        },
        Err(_) => SavedClipboard::empty(),
    };

    write_text(app, new_text)?;

    Ok(saved)
}

/// Restore previously saved clipboard contents.
/// Returns Ok(true) if restore was attempted, Ok(false) if there was nothing to restore.
pub fn restore_saved(app: &AppHandle, saved: &SavedClipboard) -> Result<bool, String> {
    if let Some(ref text) = saved.text {
        write_text(app, text)?;
        Ok(true)
    } else {
        Ok(false)
    }
}
