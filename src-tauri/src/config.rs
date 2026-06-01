//! User settings, persisted as plain JSON in the OS config directory.
//!
//! Deliberately small. Spiel has no account, no telemetry, and no cloud config —
//! everything here is a local preference. We use JSON (not SQLite) because there is
//! exactly one settings record and it benefits from being human-readable/editable.

use crate::error::{Result, SpielError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Global toggle hotkey, in `tauri-plugin-global-shortcut` syntax.
    pub hotkey: String,
    /// Model id from the registry (see `model.rs`), e.g. "base.en".
    pub model: String,
    /// Spoken language hint. "auto" lets Whisper detect; a code like "en" is faster.
    pub language: String,
    /// After transcribing, synthesize Cmd+V so text lands at the cursor.
    /// Requires macOS Accessibility permission. If false, text is left on the clipboard.
    pub auto_paste: bool,
    /// Restore whatever was on the clipboard before insertion (best-effort).
    pub restore_clipboard: bool,
    /// Hard cap on a single recording. Protects memory and bounds latency.
    pub max_seconds: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Cmd+Alt+D — "D" for dictate. Avoids the Spotlight/IME conflicts the
            // previous build kept hitting with Space-based combos.
            hotkey: "Cmd+Alt+D".to_string(),
            model: "base.en".to_string(),
            language: "en".to_string(),
            auto_paste: true,
            restore_clipboard: true,
            max_seconds: 120,
        }
    }
}

impl Config {
    /// Validate and normalize. Returns a corrected copy or an error for hard-invalid input.
    pub fn validated(mut self) -> Result<Self> {
        if self.hotkey.trim().is_empty() {
            return Err(SpielError::Config("Hotkey cannot be empty.".into()));
        }
        self.hotkey = self.hotkey.trim().to_string();

        let model = self.model.trim();
        if model.is_empty() {
            self.model = Config::default().model;
        } else if crate::model::spec(model).is_some() {
            self.model = model.to_string();
        } else {
            // Keep startup resilient to manual config edits or old values.
            self.model = Config::default().model;
        }

        let language = self.language.trim().to_ascii_lowercase();
        if language.is_empty() {
            self.language = "auto".into();
        } else if language == "en" || language == "auto" {
            self.language = language;
        } else {
            // Unknown hints should not break transcription; let Whisper auto-detect.
            self.language = "auto".into();
        }

        // Keep recordings sane: 5s..=600s.
        self.max_seconds = self.max_seconds.clamp(5, 600);
        Ok(self)
    }

    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(s) => {
                let cfg: Config = serde_json::from_str(&s)
                    .map_err(|e| SpielError::Config(format!("Corrupt settings file: {e}")))?;
                cfg.validated()
            }
            // First run (or deleted file): start from defaults and write them out.
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
                let cfg = Config::default();
                cfg.save(path)?;
                Ok(cfg)
            }
            Err(e) => Err(SpielError::Config(format!("Failed to read settings: {e}"))),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SpielError::Config(format!("Failed to create config dir: {e}")))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| SpielError::Config(format!("Failed to serialize settings: {e}")))?;
        std::fs::write(path, json)
            .map_err(|e| SpielError::Config(format!("Failed to write settings: {e}")))
    }
}

/// Resolved on-disk locations. Computed once at startup from Tauri's path resolver.
#[derive(Debug, Clone)]
pub struct Paths {
    pub config_file: PathBuf,
    pub model_dir: PathBuf,
}

impl Paths {
    pub fn model_path(&self, filename: &str) -> PathBuf {
        self.model_dir.join(filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_model_falls_back_to_default() {
        let cfg = Config {
            model: "unknown-model".into(),
            ..Config::default()
        }
        .validated()
        .unwrap();
        assert_eq!(cfg.model, Config::default().model);
    }

    #[test]
    fn language_normalizes_and_falls_back_to_auto() {
        let cfg = Config {
            language: " EN ".into(),
            ..Config::default()
        }
        .validated()
        .unwrap();
        assert_eq!(cfg.language, "en");

        let cfg2 = Config {
            language: "spanish".into(),
            ..Config::default()
        }
        .validated()
        .unwrap();
        assert_eq!(cfg2.language, "auto");
    }
}
