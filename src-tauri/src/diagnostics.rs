//! Application health inspection shared by startup, maintenance, and settings.

use crate::{accessibility, config::Config, model, state::AppState};
use serde::Serialize;
use std::io::Write;

#[derive(Serialize)]
pub struct ReadinessSnapshot {
    pub model_dir: String,
    pub model_dir_writable: bool,
    pub config_file: String,
    pub config_writable: bool,
    pub config_path_safe: bool,
    pub model_dir_safe: bool,
    pub current_model: String,
    pub current_model_installed: bool,
    pub current_model_status: String,
    pub current_model_reason: String,
    pub model_store_bytes: u64,
    pub model_store_file_count: usize,
    pub hotkey_valid: bool,
    pub accessibility_supported: bool,
    pub accessibility_trusted: bool,
    pub active_download: bool,
    pub recommended_model: String,
    pub recommended_model_reason: String,
    pub setup_steps_remaining: usize,
}

#[derive(Clone, Serialize)]
pub struct StartupHealthSnapshot {
    pub checked_at_ms: u64,
    pub config_file: String,
    pub config_path_safe: bool,
    pub config_writable: bool,
    pub model_dir: String,
    pub model_dir_safe: bool,
    pub model_dir_writable: bool,
    pub current_model: String,
    pub current_model_status: String,
    pub current_model_reason: String,
    pub hotkey_valid: bool,
    pub accessibility_supported: bool,
    pub accessibility_trusted: bool,
    pub recommended_model: String,
    pub recommended_model_reason: String,
    pub removed_partial_files: usize,
    pub removed_sidecar_files: usize,
    pub startup_warnings: Vec<String>,
}

struct HealthFacts {
    config: Config,
    model_dir: String,
    config_file: String,
    config_path_safe: bool,
    config_writable: bool,
    model_dir_safe: bool,
    model_dir_writable: bool,
    install: model::InstallInfo,
    hotkey_valid: bool,
    accessibility_supported: bool,
    accessibility_trusted: bool,
    recommended_model: String,
    recommended_model_reason: String,
}

impl HealthFacts {
    fn collect(state: &AppState) -> Self {
        let config = state.config.lock().unwrap().clone();
        let (recommended_model, recommended_model_reason) =
            model::recommended_model_for_language(&config.language);
        let accessibility_supported = accessibility::is_supported();
        Self {
            model_dir: state.paths.model_dir.to_string_lossy().to_string(),
            config_file: state.paths.config_file.to_string_lossy().to_string(),
            config_path_safe: crate::config::validate_config_path(&state.paths.config_file).is_ok(),
            config_writable: probe_file_parent_writable(&state.paths.config_file),
            model_dir_safe: model::is_safe_model_path(&state.paths.model_dir, "model directory")
                .is_ok(),
            model_dir_writable: probe_model_dir_writable(&state.paths.model_dir),
            install: state.model_install_info(&config.model),
            hotkey_valid: crate::hotkey::validate(&config.hotkey).is_ok(),
            accessibility_supported,
            accessibility_trusted: accessibility_supported && accessibility::is_trusted(),
            recommended_model: recommended_model.into(),
            recommended_model_reason: recommended_model_reason.into(),
            config,
        }
    }

    fn needs_accessibility(&self) -> bool {
        self.accessibility_supported && !self.accessibility_trusted
    }

    fn setup_steps_remaining(&self) -> usize {
        usize::from(!self.install.is_installed())
            + usize::from(self.needs_accessibility())
            + usize::from(self.config.model != self.recommended_model)
    }

    fn warnings(&self) -> Vec<String> {
        [
            (
                !self.config_path_safe,
                "Config path failed safety validation.",
            ),
            (!self.config_writable, "Config directory is not writable."),
            (
                !self.model_dir_safe,
                "Model directory failed safety validation.",
            ),
            (!self.model_dir_writable, "Model directory is not writable."),
            (
                !self.install.is_installed(),
                "Current model is not installed or failed validation.",
            ),
            (
                self.needs_accessibility(),
                "Accessibility permission is not granted yet.",
            ),
            (
                !self.hotkey_valid,
                "Configured hotkey is invalid or unavailable.",
            ),
        ]
        .into_iter()
        .filter(|(failed, _)| *failed)
        .map(|(_, message)| message.into())
        .collect()
    }
}

pub fn readiness(state: &AppState) -> ReadinessSnapshot {
    let facts = HealthFacts::collect(state);
    let setup_steps_remaining = facts.setup_steps_remaining();
    let (model_store_bytes, model_store_file_count) = summarize_model_dir(&state.paths.model_dir);
    ReadinessSnapshot {
        model_dir: facts.model_dir,
        model_dir_writable: facts.model_dir_writable,
        config_file: facts.config_file,
        config_writable: facts.config_writable,
        config_path_safe: facts.config_path_safe,
        model_dir_safe: facts.model_dir_safe,
        current_model: facts.config.model,
        current_model_installed: facts.install.is_installed(),
        current_model_status: facts.install.as_label().to_string(),
        current_model_reason: facts.install.reason,
        model_store_bytes,
        model_store_file_count,
        hotkey_valid: facts.hotkey_valid,
        accessibility_supported: facts.accessibility_supported,
        accessibility_trusted: facts.accessibility_trusted,
        active_download: state.download.is_active(),
        recommended_model: facts.recommended_model,
        recommended_model_reason: facts.recommended_model_reason,
        setup_steps_remaining,
    }
}

fn probe_model_dir_writable(model_dir: &std::path::Path) -> bool {
    if model::is_safe_model_path(model_dir, "model directory").is_err() {
        return false;
    }
    probe_directory_writable(model_dir, "spiel-write-test")
}

fn probe_file_parent_writable(path: &std::path::Path) -> bool {
    path.parent()
        .is_some_and(|parent| probe_directory_writable(parent, "spiel-config-write-test"))
}

fn probe_directory_writable(directory: &std::path::Path, prefix: &str) -> bool {
    let probe_path = directory.join(unique_probe_name(prefix));
    let write_result = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe_path)
        .and_then(|mut f| f.write_all(b"1"));
    let was_writable = write_result.is_ok();
    let _ = std::fs::remove_file(&probe_path);
    was_writable
}

fn unique_probe_name(prefix: &str) -> String {
    format!(
        ".{}.{}.{}",
        prefix,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    )
}

pub fn build_startup_health(
    state: &AppState,
    cleanup_report: model::CleanupReport,
) -> StartupHealthSnapshot {
    let facts = HealthFacts::collect(state);
    let startup_warnings = facts.warnings();
    StartupHealthSnapshot {
        checked_at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        config_file: facts.config_file,
        config_path_safe: facts.config_path_safe,
        config_writable: facts.config_writable,
        model_dir: facts.model_dir,
        model_dir_safe: facts.model_dir_safe,
        model_dir_writable: facts.model_dir_writable,
        current_model: facts.config.model,
        current_model_status: facts.install.as_label().to_string(),
        current_model_reason: facts.install.reason,
        hotkey_valid: facts.hotkey_valid,
        accessibility_supported: facts.accessibility_supported,
        accessibility_trusted: facts.accessibility_trusted,
        recommended_model: facts.recommended_model,
        recommended_model_reason: facts.recommended_model_reason,
        removed_partial_files: cleanup_report.removed_partial_files,
        removed_sidecar_files: cleanup_report.removed_sidecar_files,
        startup_warnings,
    }
}

fn summarize_model_dir(model_dir: &std::path::Path) -> (u64, usize) {
    let mut total_bytes: u64 = 0;
    let mut file_count: usize = 0;

    let entries = match std::fs::read_dir(model_dir) {
        Ok(entries) => entries,
        Err(_) => return (0, 0),
    };

    for entry in entries.filter_map(std::result::Result::ok) {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if !(name.ends_with(".bin") || name.ends_with(".part") || name.ends_with(".sha256")) {
            continue;
        }

        let metadata = match entry.file_type() {
            Ok(ft) if ft.is_symlink() => continue,
            Ok(ft) if !ft.is_file() => continue,
            _ => match entry.path().metadata() {
                Ok(meta) => meta,
                Err(_) => continue,
            },
        };

        if !metadata.is_file() {
            continue;
        }

        file_count = file_count.saturating_add(1);
        total_bytes = total_bytes.saturating_add(metadata.len());
    }

    (total_bytes, file_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn healthy_facts() -> HealthFacts {
        HealthFacts {
            config: Config::default(),
            model_dir: "models".into(),
            config_file: "config.json".into(),
            config_path_safe: true,
            config_writable: true,
            model_dir_safe: true,
            model_dir_writable: true,
            install: model::InstallInfo {
                status: model::InstallStatus::Installed,
                bytes: 1,
                modified_ms: None,
                reason: String::new(),
            },
            hotkey_valid: true,
            accessibility_supported: true,
            accessibility_trusted: true,
            recommended_model: Config::default().model,
            recommended_model_reason: String::new(),
        }
    }

    #[test]
    fn setup_counts_only_actionable_requirements() {
        let mut facts = healthy_facts();
        assert_eq!(facts.setup_steps_remaining(), 0);
        assert!(facts.warnings().is_empty());
        facts.install.status = model::InstallStatus::Missing;
        facts.accessibility_trusted = false;
        facts.recommended_model = "base".into();
        assert_eq!(facts.setup_steps_remaining(), 3);
        assert_eq!(
            facts.warnings(),
            vec![
                "Current model is not installed or failed validation.",
                "Accessibility permission is not granted yet.",
            ]
        );
        facts.accessibility_supported = false;
        assert_eq!(facts.setup_steps_remaining(), 2);
        assert_eq!(
            facts.warnings(),
            vec!["Current model is not installed or failed validation."]
        );
    }

    #[test]
    fn path_and_hotkey_failures_keep_warning_order_without_inflating_setup_steps() {
        let mut facts = healthy_facts();
        facts.config_path_safe = false;
        facts.config_writable = false;
        facts.model_dir_safe = false;
        facts.model_dir_writable = false;
        facts.hotkey_valid = false;
        assert_eq!(facts.setup_steps_remaining(), 0);
        assert_eq!(
            facts.warnings(),
            vec![
                "Config path failed safety validation.",
                "Config directory is not writable.",
                "Model directory failed safety validation.",
                "Model directory is not writable.",
                "Configured hotkey is invalid or unavailable.",
            ]
        );
    }

    #[test]
    fn summarize_model_dir_counts_model_files_only() {
        let dir = std::env::temp_dir().join(format!(
            "spiel-readiness-summary-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));

        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("ggml-tiny.bin"), vec![0_u8; 1024]).unwrap();
        fs::write(dir.join("ggml-tiny.bin.part"), vec![0_u8; 2048]).unwrap();

        let (bytes, files) = summarize_model_dir(&dir);
        assert_eq!(files, 2);
        assert_eq!(bytes, 3072);

        fs::remove_file(dir.join("ggml-tiny.bin")).unwrap();
        fs::remove_file(dir.join("ggml-tiny.bin.part")).unwrap();
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn summarize_model_dir_ignores_symlink_targets() {
        use std::os::unix::fs::symlink;

        let dir = std::env::temp_dir().join(format!(
            "spiel-readiness-summary-symlink-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let target = std::env::temp_dir().join(format!(
            "spiel-readiness-target-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::write(&target, vec![0_u8; 128]).unwrap();
        let link = dir.join("ggml-small.bin");
        symlink(&target, &link).unwrap();

        let (bytes, files) = summarize_model_dir(&dir);
        assert_eq!(bytes, 0);
        assert_eq!(files, 0);

        fs::remove_file(&target).unwrap();
        fs::remove_file(link).unwrap();
        fs::remove_dir_all(&dir).unwrap();
    }
}
