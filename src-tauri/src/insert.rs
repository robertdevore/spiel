//! Delivering transcribed text to wherever the cursor is.
//!
//! The flow: save the current clipboard → put our text on it → synthesize Cmd+V so it
//! lands in the focused app → restore the previous clipboard. If Accessibility isn't
//! granted (so we can't paste), we leave the text on the clipboard and tell the caller,
//! which surfaces a one-time "grant permission" prompt. We never lose the user's text:
//! worst case it's sitting on the clipboard ready for a manual Cmd+V.

use crate::accessibility;
use crate::error::{Result, SpielError};
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

    // Let the pasteboard write settle before we trigger the paste, so the focused app
    // can't read a stale changeCount.
    std::thread::sleep(std::time::Duration::from_millis(120));
    paste_via_cmd_v()?;
    outcome.pasted = true;

    if restore_clipboard {
        if let Some(prev) = previous {
            // Critical: wait for the focused app to actually consume the paste before we
            // put the old clipboard back. Too short a wait and the app reads the restored
            // (previous) contents instead of the transcript — which looks like Spiel
            // "pasting your clipboard".
            std::thread::sleep(std::time::Duration::from_millis(500));
            if clipboard.set_text(prev).is_ok() {
                outcome.restored_previous = true;
            }
        }
    }

    Ok(outcome)
}

/// Synthesize Cmd+V by posting the V keycode with the Command modifier flag set.
///
/// We deliberately do NOT use a Unicode-character event: on macOS that injects the
/// literal character 'v' and ignores held modifiers, which is why a naive approach types
/// "v" instead of pasting. Posting keycode 9 (kVK_ANSI_V) with the Command flag is a real
/// Cmd+V the focused app interprets as paste.
#[cfg(target_os = "macos")]
fn paste_via_cmd_v() -> Result<()> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    const KEY_V: u16 = 9; // kVK_ANSI_V

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .map_err(|_| SpielError::Insertion("could not create input event source".into()))?;

    let key_down = CGEvent::new_keyboard_event(source.clone(), KEY_V, true)
        .map_err(|_| SpielError::Insertion("could not create key-down event".into()))?;
    key_down.set_flags(CGEventFlags::CGEventFlagCommand);
    key_down.post(CGEventTapLocation::HID);

    let key_up = CGEvent::new_keyboard_event(source, KEY_V, false)
        .map_err(|_| SpielError::Insertion("could not create key-up event".into()))?;
    key_up.set_flags(CGEventFlags::CGEventFlagCommand);
    key_up.post(CGEventTapLocation::HID);

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn paste_via_cmd_v() -> Result<()> {
    Err(SpielError::Insertion(
        "auto-paste is only implemented on macOS".into(),
    ))
}
