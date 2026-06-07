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
    /// Keep Whisper model loaded in RAM between dictations for lower latency.
    /// If false, model is unloaded after each dictation to minimize idle memory.
    pub keep_model_loaded: bool,
    /// Transcription thread count. Lower values reduce memory/thread pressure.
    pub transcription_threads: u8,
    /// Hard cap on a single recording. Protects memory and bounds latency.
    pub max_seconds: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Cmd+Alt+D — "D" for dictate. Avoids the Spotlight/IME conflicts the
            // previous build kept hitting with Space-based combos.
            hotkey: "Cmd+Alt+D".to_string(),
            // Memory-first default: tiny model keeps footprint much lower.
            model: "tiny.en".to_string(),
            language: "en".to_string(),
            auto_paste: true,
            restore_clipboard: true,
            keep_model_loaded: false,
            transcription_threads: 2,
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

        let language = crate::model::normalize_language_hint(&self.language);
        let model_spec = crate::model::spec(&self.model)
            .or_else(|| crate::model::spec("tiny.en"))
            .unwrap();
        if !crate::model::is_language_supported(model_spec, &language) {
            self.language = "en".into();
        } else {
            self.language = language;
        }

        // Keep recordings sane: 5s..=600s.
        self.max_seconds = self.max_seconds.clamp(5, 600);
        self.transcription_threads = self.transcription_threads.clamp(1, 8);
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
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json)
            .map_err(|e| SpielError::Config(format!("Failed to write temp settings: {e}")))?;
        #[cfg(windows)]
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
        std::fs::rename(&tmp, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            SpielError::Config(format!("Failed to finalize settings: {e}"))
        })?;
        set_config_permissions(path)?;
        Ok(())
    }
}

fn set_config_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, mode)
            .map_err(|e| SpielError::Config(format!("Failed to set config permissions: {e}")))?;
    }
    Ok(())
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
            language: " en-US ".into(),
            ..Config::default()
        }
        .validated()
        .unwrap();
        assert_eq!(cfg.language, "en");

        let cfg2 = Config {
            language: "zz-99".into(),
            ..Config::default()
        }
        .validated()
        .unwrap();
        assert_eq!(cfg2.language, "en");
    }

    #[test]
    fn english_only_models_force_english_language() {
        let cfg = Config {
            model: "tiny.en".into(),
            language: "es".into(),
            ..Config::default()
        }
        .validated()
        .unwrap();
        assert_eq!(cfg.language, "en");
    }

    #[test]
    fn multilingual_models_allow_supported_language_hints() {
        let cfg = Config {
            model: "base".into(),
            language: "es-MX".into(),
            ..Config::default()
        }
        .validated()
        .unwrap();
        assert_eq!(cfg.language, "es");
    }

    #[test]
    fn transcription_threads_is_clamped() {
        let cfg = Config {
            transcription_threads: 0,
            ..Config::default()
        }
        .validated()
        .unwrap();
        assert_eq!(cfg.transcription_threads, 1);

        let cfg2 = Config {
            transcription_threads: 50,
            ..Config::default()
        }
        .validated()
        .unwrap();
        assert_eq!(cfg2.transcription_threads, 8);
    }
}
