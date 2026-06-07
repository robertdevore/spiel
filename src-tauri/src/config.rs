//! User settings, persisted as plain JSON in the OS config directory.
//!
//! Deliberately small. Spiel has no account, no telemetry, and no cloud config —
//! everything here is a local preference. We use JSON (not SQLite) because there is
//! exactly one settings record and it benefits from being human-readable/editable.

use crate::error::{Result, SpielError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{io, io::Write, path::Component, time};

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
        validate_config_path(path)?;

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
        validate_config_path(path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SpielError::Config(format!("Failed to create config dir: {e}")))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| SpielError::Config(format!("Failed to serialize settings: {e}")))?;
        let tmp = unique_temp_path(path, "json.tmp");
        let mut out = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&tmp)
            .map_err(|e| SpielError::Config(format!("Failed to create temp settings: {e}")))?;
        out.write_all(json.as_bytes())
            .map_err(|e| SpielError::Config(format!("Failed to write temp settings: {e}")))?;
        out.sync_all().map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            SpielError::Config(format!("Failed to flush temp settings: {e}"))
        })?;
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

pub fn validate_config_path(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| SpielError::Config("settings path must have a parent directory".into()))?;

    let mut cursor = PathBuf::new();
    for c in parent.components() {
        if let Component::RootDir | Component::CurDir = c {
            cursor.push(c.as_os_str());
            continue;
        }

        cursor.push(c.as_os_str());
        match std::fs::symlink_metadata(&cursor) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err(SpielError::Config(format!(
                        "refusing settings path with symlink component: {path:?}"
                    )));
                }
            }
            Err(e) => {
                if e.kind() != io::ErrorKind::NotFound {
                    return Err(SpielError::Config(format!(
                        "cannot check config path component {cursor:?}: {e}"
                    )));
                }
                // Stop at the first missing component. We validate only components that
                // already exist, then create the tail directories atomically.
                break;
            }
        }
    }

    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                return Err(SpielError::Config(format!(
                    "settings file path is a symlink: {path:?}"
                )));
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(SpielError::Config(format!(
                "cannot check settings path {path:?}: {e}"
            )));
        }
    }

    Ok(())
}

fn unique_temp_path(path: &Path, suffix: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let base_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "settings.json".to_string());
    path.with_file_name(format!(
        ".{}.{}.{suffix}",
        base_name,
        nanos.saturating_add(pid as u128)
    ))
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

    #[cfg(unix)]
    #[test]
    fn config_path_rejects_symlink_target() {
        use std::os::unix::fs::symlink;

        let base = std::env::temp_dir();
        let cfg_file = base.join(format!("spiel_config_{}.json", std::process::id()));
        let target = base.join(format!("spiel_config_target_{}.txt", std::process::id()));
        std::fs::write(&target, b"{}").ok();
        let _ = std::fs::remove_file(&cfg_file);
        symlink(&target, &cfg_file).unwrap();

        let err = validate_config_path(&cfg_file).expect_err("symlink should be rejected");
        assert!(err.to_string().contains("symlink"));

        let _ = std::fs::remove_file(&cfg_file);
        let _ = std::fs::remove_file(&target);
    }

    #[test]
    fn unique_temp_path_has_safe_file_name() {
        let base = std::env::temp_dir().join("spiel-config-test");
        let config_path = base.join("config.json");
        let temp = unique_temp_path(&config_path, "json.tmp");
        let name = temp.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with(".config.json"));
        assert!(!name.contains('"'));
    }
}
