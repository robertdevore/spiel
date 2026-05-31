//! Delivering transcribed text to wherever the cursor is.
//!
//! The flow: save the current clipboard → put our text on it → synthesize Cmd+V so it
//! lands in the focused app → restore the previous clipboard. If Accessibility isn't
//! granted (so we can't paste), we leave the text on the clipboard and tell the caller,
//! which surfaces a one-time "grant permission" prompt. We never lose the user's text:
//! worst case it's sitting on the clipboard ready for a manual Cmd+V.

use crate::accessibility;
use crate::error::{Result, SpielError};
use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct InsertOutcome {
    /// We synthesized Cmd+V and the text should be at the cursor.
    pub pasted: bool,
    /// Text is on the clipboard but was not auto-pasted (manual paste needed).
    pub clipboard_only: bool,
    /// The previous clipboard contents were put back after pasting.
    pub restored_previous: bool,
    /// Auto-paste was requested but Accessibility permission is missing.
    pub needs_accessibility: bool,
}

/// Place `text` into the focused app. `auto_paste`/`restore_clipboard` come from settings.
pub fn insert(text: &str, auto_paste: bool, restore_clipboard: bool) -> Result<InsertOutcome> {
    if text.is_empty() {
        return Err(SpielError::Insertion("nothing to insert".into()));
    }

    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| SpielError::Insertion(format!("clipboard unavailable: {e}")))?;

    // Remember what was there (text only; we can't faithfully restore images/files).
    let previous = clipboard.get_text().ok().filter(|s| !s.is_empty());

    clipboard
        .set_text(text.to_string())
        .map_err(|e| SpielError::Insertion(format!("could not set clipboard: {e}")))?;

    let mut outcome = InsertOutcome::default();

    if !auto_paste {
        outcome.clipboard_only = true;
        return Ok(outcome);
    }

    if !accessibility::is_trusted() {
        // Don't paste blindly (the keystrokes would no-op); keep text on the clipboard.
        outcome.clipboard_only = true;
        outcome.needs_accessibility = true;
        return Ok(outcome);
    }

    paste_via_cmd_v()?;
    outcome.pasted = true;

    if restore_clipboard {
        if let Some(prev) = previous {
            // Give the focused app a beat to read the pasteboard before we overwrite it.
            std::thread::sleep(std::time::Duration::from_millis(180));
            if clipboard.set_text(prev).is_ok() {
                outcome.restored_previous = true;
            }
        }
    }

    Ok(outcome)
}

/// Synthesize a Cmd+V keystroke. macOS maps `Key::Meta` to the Command key.
fn paste_via_cmd_v() -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| SpielError::Insertion(format!("input synthesis unavailable: {e}")))?;
    enigo
        .key(Key::Meta, Press)
        .map_err(|e| SpielError::Insertion(e.to_string()))?;
    let press_result = enigo.key(Key::Unicode('v'), Click);
    // Always release Meta, even if the 'v' press failed, so we don't leave Cmd "stuck".
    let release_result = enigo.key(Key::Meta, Release);
    press_result.map_err(|e| SpielError::Insertion(e.to_string()))?;
    release_result.map_err(|e| SpielError::Insertion(e.to_string()))?;
    Ok(())
}
