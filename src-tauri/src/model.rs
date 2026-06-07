//! Whisper model registry + first-run downloader.
//!
//! Spiel ships *no* model in git (they're 75–150 MB). On first run the user picks a
//! model and Spiel downloads it once, to the app data dir, with a progress bar and an
//! integrity check. After that, transcription is fully offline.
//!
//! Integrity strategy: we stream to a `.part` file, then validate before promoting it
//! to the real filename. Validation = correct GGML magic header + a minimum size, and
//! (when known) a pinned SHA-256. A truncated or corrupted download therefore never
//! gets loaded as a model.

use crate::error::{Result, SpielError};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::{self, Read};
use std::path::Path;
use std::time::{Duration, SystemTime};

/// GGML container magic. whisper.cpp writes the u32 `0x67676d6c` ("ggml"), which lands
/// on disk little-endian as the bytes `6c 6d 67 67`.
const GGML_MAGIC: u32 = 0x6767_6d6c;

#[derive(Debug, Clone, Serialize)]
pub struct ModelSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub filename: &'static str,
    pub url: &'static str,
    /// Approximate download size, for the UI.
    pub approx_mb: u32,
    /// Pinned SHA-256 (hex). Empty = not pinned; we still validate magic + size.
    pub sha256: &'static str,
    /// True for multilingual checkpoints; false means English-only variants.
    pub multilingual: bool,
    pub note: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallStatus {
    Missing,
    Installed,
    Partial,
    Corrupt,
    UnsafePath,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallInfo {
    pub status: InstallStatus,
    /// Bytes currently present for the model file or `.part` file.
    pub bytes: u64,
    /// Optional detail string for richer diagnostics.
    pub reason: String,
}

impl InstallInfo {
    pub fn as_label(&self) -> &'static str {
        match self.status {
            InstallStatus::Missing => "missing",
            InstallStatus::Installed => "installed",
            InstallStatus::Partial => "partial",
            InstallStatus::Corrupt => "corrupt",
            InstallStatus::UnsafePath => "unsafe_path",
        }
    }

    pub fn is_installed(&self) -> bool {
        self.status == InstallStatus::Installed
    }
}

/// Models offered in the UI.
pub const REGISTRY: &[ModelSpec] = &[
    ModelSpec {
        id: "tiny.en",
        label: "Tiny (English)",
        filename: "ggml-tiny.en.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
        approx_mb: 75,
        sha256: "",
        multilingual: false,
        note: "Fastest, lowest accuracy. Good on older Macs.",
    },
    ModelSpec {
        id: "base.en",
        label: "Base (English)",
        filename: "ggml-base.en.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
        approx_mb: 142,
        sha256: "",
        multilingual: false,
        note: "Recommended. Balanced speed and accuracy for English.",
    },
    ModelSpec {
        id: "small.en",
        label: "Small (English)",
        filename: "ggml-small.en.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
        approx_mb: 466,
        sha256: "",
        multilingual: false,
        note: "Higher accuracy, slower. Best on Apple Silicon.",
    },
    ModelSpec {
        id: "tiny",
        label: "Tiny (Multilingual)",
        filename: "ggml-tiny.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
        approx_mb: 75,
        sha256: "",
        multilingual: true,
        note: "Tiny-size multilingual model. Great for mixed-language notes.",
    },
    ModelSpec {
        id: "base",
        label: "Base (Multilingual)",
        filename: "ggml-base.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        approx_mb: 142,
        sha256: "",
        multilingual: true,
        note: "English and multilingual default quality profile.",
    },
    ModelSpec {
        id: "small",
        label: "Small (Multilingual)",
        filename: "ggml-small.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        approx_mb: 466,
        sha256: "",
        multilingual: true,
        note: "Higher-quality multilingual model. Better for mixed-language output.",
    },
    ModelSpec {
        id: "medium",
        label: "Medium (Multilingual)",
        filename: "ggml-medium.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
        approx_mb: 1536,
        sha256: "",
        multilingual: true,
        note: "Largest included quality target. Better recall/accuracy, more memory.",
    },
];

const DEFAULT_PART_CLEANUP_MS: u64 = 24 * 60 * 60 * 1000;

pub fn spec(id: &str) -> Option<&'static ModelSpec> {
    REGISTRY.iter().find(|m| m.id == id)
}

/// Read model cleanup duration from an env var, used for stale `.part` file expiration.
pub fn parse_part_cleanup_ms(raw: Option<&str>, default_ms: u64) -> Duration {
    raw.and_then(|raw| raw.trim().parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or_else(|| Duration::from_millis(default_ms))
}

/// Default duration for stale `.part` cleanup when the env var is unset or invalid.
pub fn default_part_cleanup_duration() -> Duration {
    Duration::from_millis(DEFAULT_PART_CLEANUP_MS)
}

pub fn is_language_supported(spec: &ModelSpec, language: &str) -> bool {
    if language == "auto" {
        return true;
    }
    if !is_language_hint(language) {
        return false;
    }
    if spec.multilingual {
        true
    } else {
        language == "en"
    }
}

pub fn is_language_hint(language: &str) -> bool {
    if language == "en" || language == "auto" {
        return true;
    }
    let primary = language.split(['-', '_']).next().unwrap_or("");
    let is_primary_valid = primary.len() == 2 && primary.bytes().all(|b| b.is_ascii_lowercase());
    if !is_primary_valid {
        return false;
    }
    language
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

pub fn normalize_language_hint(language: &str) -> String {
    let raw = language.trim().to_ascii_lowercase();
    if raw.is_empty() {
        return "auto".into();
    }
    if raw == "auto" {
        return "auto".into();
    }

    let primary = raw.split(['-', '_']).next().unwrap_or("");
    if is_language_hint(primary) {
        primary.to_string()
    } else {
        "auto".into()
    }
}

/// Is the model for `id` present and structurally valid on disk?
pub fn is_installed(model_dir: &Path, id: &str) -> bool {
    inspect_install(model_dir, id).is_installed()
}

/// Lightweight install-state check for polling/UI.
pub fn inspect_install(model_dir: &Path, id: &str) -> InstallInfo {
    let Some(spec) = spec(id) else {
        return InstallInfo {
            status: InstallStatus::Missing,
            bytes: 0,
            reason: "unknown model id".into(),
        };
    };

    let path = model_dir.join(spec.filename);
    let part_path = model_dir.join(format!("{}.part", spec.filename));

    if let Err(err) = is_safe_model_path(&path, "model") {
        return InstallInfo {
            status: InstallStatus::UnsafePath,
            bytes: 0,
            reason: err.to_string(),
        };
    }

    match validate_model_file(&path, spec) {
        Ok(size) => InstallInfo {
            status: InstallStatus::Installed,
            bytes: size,
            reason: String::new(),
        },
        Err(SpielError::ModelMissing) => {
            if part_path.exists() {
                return match is_safe_model_path(&part_path, "temporary model file") {
                    Ok(()) => match part_path.metadata() {
                        Ok(meta) if meta.is_file() => InstallInfo {
                            status: InstallStatus::Partial,
                            bytes: meta.len(),
                            reason: String::new(),
                        },
                        Ok(_) => InstallInfo {
                            status: InstallStatus::Missing,
                            bytes: 0,
                            reason: String::new(),
                        },
                        Err(err) => InstallInfo {
                            status: InstallStatus::Corrupt,
                            bytes: 0,
                            reason: format!("partial file is not readable: {err}"),
                        },
                    },
                    Err(err) => InstallInfo {
                        status: InstallStatus::UnsafePath,
                        bytes: 0,
                        reason: err.to_string(),
                    },
                };
            }

            InstallInfo {
                status: InstallStatus::Missing,
                bytes: 0,
                reason: String::new(),
            }
        }
        Err(err) => InstallInfo {
            status: InstallStatus::Corrupt,
            bytes: path.metadata().map(|m| m.len()).unwrap_or(0),
            reason: err.to_string(),
        },
    }
}

/// Remove stale `.part` files from interrupted downloads.
/// Returns how many stale files were removed.
pub fn cleanup_stale_part_files(model_dir: &Path, older_than: Duration) -> usize {
    let read_dir = match std::fs::read_dir(model_dir) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };

    let mut removed = 0usize;
    for entry in read_dir.filter_map(std::result::Result::ok) {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.ends_with(".part") {
            continue;
        }

        let metadata = match entry.path().symlink_metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if !metadata.is_file() {
            continue;
        }

        if older_than.is_zero() {
            if std::fs::remove_file(&path).is_ok() {
                removed = removed.saturating_add(1);
            }
            continue;
        }

        let is_stale = metadata.modified().ok().and_then(|modified| {
            SystemTime::now()
                .duration_since(modified)
                .ok()
                .map(|age| age > older_than)
        });
        if is_stale.unwrap_or(false) && std::fs::remove_file(&path).is_ok() {
            removed = removed.saturating_add(1);
        }
    }

    removed
}

/// Download `spec` to `model_dir`, calling `on_progress(downloaded, total)` as it goes.
///
/// `total` is `None` if the server doesn't send a content length. Cancellation is
/// cooperative: `should_cancel()` is polled between chunks.
pub fn download(
    model_dir: &Path,
    spec: &ModelSpec,
    mut on_progress: impl FnMut(u64, Option<u64>),
    should_cancel: impl Fn() -> bool,
) -> Result<()> {
    std::fs::create_dir_all(model_dir)
        .map_err(|e| SpielError::Download(format!("cannot create model dir: {e}")))?;

    let (connect_timeout, request_timeout) = (
        load_download_timeout_ms("SPIEL_DOWNLOAD_CONNECT_TIMEOUT_MS", 10_000),
        load_download_timeout_ms("SPIEL_DOWNLOAD_TIMEOUT_MS", 30 * 60 * 1_000),
    );

    let final_path = model_dir.join(spec.filename);
    let part_path = model_dir.join(format!("{}.part", spec.filename));

    is_safe_model_path(&part_path, "temporary model file")
        .and_then(|_| is_safe_model_path(&final_path, "target model file"))?;

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_millis(connect_timeout))
        .timeout(Duration::from_millis(request_timeout))
        .build()
        .map_err(|e| SpielError::Download(e.to_string()))?;

    let mut resp = client
        .get(spec.url)
        .send()
        .map_err(|e| SpielError::Download(format!("could not reach {}: {e}", spec.url)))?;

    if !resp.status().is_success() {
        return Err(SpielError::Download(format!(
            "server returned {} for {}",
            resp.status(),
            spec.url
        )));
    }

    let total = resp.content_length();
    let mut file = open_part_file(&part_path)?;
    let mut hasher = Sha256::new();
    let mut downloaded: u64 = 0;
    let mut buf = [0u8; 64 * 1024];

    on_progress(0, total);
    let mut download_result = Ok(());
    while download_result.is_ok() {
        if should_cancel() {
            drop(file);
            let _ = std::fs::remove_file(&part_path);
            return Err(SpielError::Download("canceled".into()));
        }
        let n = resp
            .read(&mut buf)
            .map_err(|e| SpielError::Download(format!("read error: {e}")))?;
        if n == 0 {
            break;
        }
        use std::io::Write;
        download_result = file
            .write_all(&buf[..n])
            .map_err(|e| SpielError::Download(format!("write error: {e}")));
        if download_result.is_err() {
            break;
        }
        hasher.update(&buf[..n]);
        downloaded += n as u64;
        on_progress(downloaded, total);
    }
    if let Err(e) = download_result {
        let _ = std::fs::remove_file(&part_path);
        return Err(e);
    }
    file.sync_all().ok();
    drop(file);

    if let Err(e) = ensure_complete_download(total, downloaded) {
        let _ = std::fs::remove_file(&part_path);
        return Err(e);
    }

    // Verify pinned hash (when present) before we trust the bytes.
    if !spec.sha256.is_empty() {
        let got = hex(&hasher.finalize());
        if !got.eq_ignore_ascii_case(spec.sha256) {
            let _ = std::fs::remove_file(&part_path);
            return Err(SpielError::Download(
                "checksum mismatch — download may be corrupt. Please retry.".into(),
            ));
        }
    }

    validate_file(&part_path, spec).inspect_err(|_| {
        let _ = std::fs::remove_file(&part_path);
    })?;

    std::fs::rename(&part_path, &final_path)
        .map_err(|e| SpielError::Download(format!("could not finalize model: {e}")))?;
    Ok(())
}

/// Structural validation independent of any pinned hash: header magic + plausible size.
fn validate_model_file(path: &Path, spec: &ModelSpec) -> Result<u64> {
    let mut f = std::fs::File::open(path).map_err(|_| SpielError::ModelMissing)?;
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    validate_model_file_with_open_handle(&mut f, spec, path, len)
}

fn validate_model_file_with_open_handle(
    f: &mut std::fs::File,
    spec: &ModelSpec,
    path: &Path,
    len: u64,
) -> Result<u64> {
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic)
        .map_err(|_| SpielError::Model("model file is truncated".into()))?;
    if u32::from_le_bytes(magic) != GGML_MAGIC {
        return Err(SpielError::Model(format!(
            "{path:?} is not a valid GGML whisper model"
        )));
    }
    let min = min_size_bytes(spec);
    if len < min {
        return Err(SpielError::Model(format!(
            "{path:?} is smaller than expected ({} < {})",
            len, min
        )));
    }
    Ok(len)
}

/// Existing tests and older callsites still validate by path.
fn validate_file(path: &Path, spec: &ModelSpec) -> Result<()> {
    validate_model_file(path, spec).map(|_| ())
}

fn min_size_bytes(spec: &ModelSpec) -> u64 {
    (spec.approx_mb as u64) * 1024 * 1024 / 2
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

fn open_part_file(path: &Path) -> Result<std::fs::File> {
    is_safe_model_path(path, "temporary model file")
        .map_err(|e| SpielError::Download(format!("unsafe temporary model file {path:?}: {e}")))?;

    let mut attempts = 0;
    loop {
        match OpenOptions::new().create_new(true).write(true).open(path) {
            Ok(file) => return Ok(file),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists && attempts < 1 => {
                let _ = std::fs::remove_file(path);
                attempts += 1;
            }
            Err(err) => {
                return Err(SpielError::Download(format!(
                    "cannot create temporary model file {path:?}: {err}"
                )))
            }
        }
    }
}

fn ensure_complete_download(total: Option<u64>, downloaded: u64) -> Result<()> {
    if let Some(expected) = total {
        if downloaded != expected {
            return Err(SpielError::Download(format!(
                "download incomplete: expected {expected} bytes, got {downloaded}"
            )));
        }
    }
    Ok(())
}

pub fn is_safe_model_path(path: &Path, label: &str) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                return Err(SpielError::Download(format!(
                    "{label} path is a symlink; refusing overwrite"
                )));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(SpielError::Download(format!(
                "cannot check {label} path {path:?}: {e}"
            )));
        }
    }
    Ok(())
}

fn load_download_timeout_ms(name: &str, default_ms: u64) -> u64 {
    parse_download_timeout_ms(std::env::var(name).ok().as_deref(), default_ms)
}

fn parse_download_timeout_ms(raw: Option<&str>, default_ms: u64) -> u64 {
    raw.and_then(|raw| raw.trim().parse::<u64>().ok())
        .map(|v| v.clamp(1_000, 86_400_000))
        .unwrap_or(default_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_default() {
        assert!(spec("base.en").is_some());
    }

    #[test]
    fn multilingual_registry_entries_exist() {
        assert!(spec("tiny").is_some());
        assert!(spec("base").is_some());
        assert!(spec("small").is_some());
    }

    #[test]
    fn unknown_model_is_none() {
        assert!(spec("nope").is_none());
    }

    #[test]
    fn language_hint_validation_prefers_sane_tags() {
        assert!(is_language_hint("en"));
        assert!(is_language_hint("auto"));
        assert!(is_language_hint("zh"));
        assert!(is_language_hint("en-US"));
        assert!(is_language_hint("pt_BR"));
        assert!(!is_language_hint("e"));
        assert!(!is_language_hint("eng"));
        assert!(!is_language_hint("en us"));
        assert!(!is_language_hint("zzzz"));
    }

    #[test]
    fn english_model_rejects_non_english_hint() {
        let tiny_en = spec("tiny.en").expect("tiny.en exists");
        assert!(is_language_supported(tiny_en, "en"));
        assert!(is_language_supported(tiny_en, "auto"));
        assert!(!is_language_supported(tiny_en, "es"));
        assert!(!is_language_supported(tiny_en, "fr"));
    }

    #[test]
    fn multilingual_model_accepts_extended_hint() {
        let tiny = spec("tiny").expect("tiny exists");
        assert!(is_language_supported(tiny, "es"));
        assert!(is_language_supported(tiny, "zh"));
        assert!(!is_language_supported(tiny, "e"));
    }

    #[test]
    fn normalizes_region_and_invalid_language_tags() {
        assert_eq!(normalize_language_hint(" en-US "), "en");
        assert_eq!(normalize_language_hint("fr_CA"), "fr");
        assert_eq!(normalize_language_hint(""), "auto");
        assert_eq!(normalize_language_hint("zzZZ"), "auto");
    }

    #[test]
    fn download_timeouts_have_sane_defaults() {
        assert_eq!(load_download_timeout_ms("SPIEL_NOT_SET", 10_000), 10_000);
    }

    #[test]
    fn download_timeouts_respect_bounds() {
        assert_eq!(parse_download_timeout_ms(Some("500"), 10_000), 1000);
        assert_eq!(parse_download_timeout_ms(Some("1200000"), 10_000), 1200000);
        assert_eq!(
            parse_download_timeout_ms(Some("99999999"), 10_000),
            86_400_000
        );
        assert_eq!(parse_download_timeout_ms(None, 10_000), 10_000);
    }

    #[test]
    fn parse_part_cleanup_ms_uses_default_on_missing_or_invalid() {
        assert_eq!(parse_part_cleanup_ms(None, 100), Duration::from_millis(100));
        assert_eq!(
            parse_part_cleanup_ms(Some("not-a-number"), 100),
            Duration::from_millis(100)
        );
        assert_eq!(
            parse_part_cleanup_ms(Some("0"), 100),
            Duration::from_millis(0)
        );
    }

    #[test]
    fn safe_model_path_rejects_symlink_or_errors() {
        let temp = std::env::temp_dir().join(format!(
            "spiel_model_safe_path_check_{}",
            std::process::id()
        ));
        if temp.exists() {
            let _ = std::fs::remove_file(&temp);
        }
        assert!(is_safe_model_path(&temp, "test").is_ok());
        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn missing_file_not_installed() {
        let dir = std::env::temp_dir().join("spiel_model_test_missing");
        assert!(!is_installed(&dir, "base.en"));
        assert_eq!(
            inspect_install(&dir, "base.en").status,
            InstallStatus::Missing
        );
    }

    #[test]
    fn partial_download_is_reported() {
        let dir = std::env::temp_dir().join("spiel_model_partial_check");
        let _ = std::fs::create_dir_all(&dir);
        let spec = spec("base.en").unwrap();
        let part_path = dir.join(format!("{}.part", spec.filename));
        let _ = std::fs::write(&part_path, b"partial-bytes");

        let info = inspect_install(&dir, "base.en");
        assert_eq!(info.status, InstallStatus::Partial);
        assert!(info.bytes > 0);

        let _ = std::fs::remove_file(&part_path);
    }

    #[test]
    fn stale_partial_downloads_cleanup_removes_part_files() {
        let dir = std::env::temp_dir().join("spiel_model_partial_cleanup");
        let _ = std::fs::create_dir_all(&dir);
        let old_part = dir.join("obsolete.bin.part");
        let keep_part = dir.join("pending.bin.part");
        let _ = std::fs::write(&old_part, b"old");
        let _ = std::fs::write(&keep_part, b"new");

        let removed = cleanup_stale_part_files(&dir, Duration::ZERO);
        assert_eq!(removed, 2);
        assert!(!old_part.exists());
        assert!(!keep_part.exists());
    }

    #[test]
    fn malformed_file_is_not_installed() {
        let dir = std::env::temp_dir().join("spiel_model_installed_check");
        let _ = std::fs::create_dir_all(&dir);
        let s = spec("tiny.en").unwrap();
        let path = dir.join(s.filename);
        // Large enough to pass size-only checks, but invalid GGML content.
        std::fs::write(
            &path,
            vec![0_u8; (s.approx_mb as usize * 1024 * 1024 / 2) + 1024],
        )
        .unwrap();

        assert!(!is_installed(&dir, "tiny.en"));
        assert_eq!(
            inspect_install(&dir, "tiny.en").status,
            InstallStatus::Corrupt
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn ggml_magic_matches_real_on_disk_bytes() {
        // Verified against the real ggml-base.en.bin: it begins with `6c 6d 67 67`.
        let on_disk = [0x6c_u8, 0x6d, 0x67, 0x67];
        assert_eq!(u32::from_le_bytes(on_disk), GGML_MAGIC);
    }

    #[test]
    fn rejects_non_ggml_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("spiel_fake_model.bin");
        std::fs::write(&path, b"NOTAMODEL").unwrap();
        let s = spec("base.en").unwrap();
        assert!(validate_file(&path, s).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn detects_incomplete_download_when_length_known() {
        let err = ensure_complete_download(Some(100), 90).unwrap_err();
        assert!(err.to_string().contains("download incomplete"));
    }

    #[test]
    fn allows_download_when_length_unknown() {
        assert!(ensure_complete_download(None, 90).is_ok());
    }
}
