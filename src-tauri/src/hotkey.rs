//! Hotkey validation, registration, and the settings persistence transaction.

use crate::error;
use std::str::FromStr;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// (Re)register the global toggle hotkey. Replaces any previously registered shortcut.
pub fn register(app: &tauri::AppHandle, spec: &str) -> error::Result<()> {
    let shortcut = parse_shortcut(spec)?;
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    gs.on_shortcut(shortcut, move |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            crate::dictation::toggle(app);
        }
    })
    .map_err(|e| {
        error::SpielError::Config(format!(
            "Could not register hotkey '{spec}': {e}. It may already be in use by another app."
        ))
    })
}

/// Validate a hotkey string without registering it. Used to reject bad input early.
pub fn validate(spec: &str) -> error::Result<()> {
    parse_shortcut(spec).map(|_| ())
}

/// Parse a hotkey string (e.g. "Cmd+Alt+D"). Returns a clear error for invalid input.
fn parse_shortcut(spec: &str) -> error::Result<Shortcut> {
    Shortcut::from_str(spec.trim()).map_err(|_| {
        error::SpielError::Config(format!(
            "'{spec}' isn't a valid hotkey. Use one or more modifiers plus a key — for \
             example Cmd+Alt+D, Cmd+Shift+Space, or CmdOrCtrl+Shift+K. Letters and named \
             keys work; punctuation like '?' does not."
        ))
    })
}

pub fn apply_and_persist<FRegister, FSave>(
    old_hotkey: &str,
    new_hotkey: &str,
    mut register: FRegister,
    save: FSave,
) -> error::Result<()>
where
    FRegister: FnMut(&str) -> error::Result<()>,
    FSave: FnOnce() -> error::Result<()>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SpielError;
    use std::cell::RefCell;

    #[test]
    fn registers_new_hotkey_then_persists() {
        let calls = RefCell::new(Vec::<String>::new());

        let result = apply_and_persist(
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

        let result = apply_and_persist(
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

        let result = apply_and_persist(
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

        let result = apply_and_persist(
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
