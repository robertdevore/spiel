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
use std::io::Read;
use std::path::Path;

/// GGML container magic ("ggml" little-endian) at the start of every whisper model.
const GGML_MAGIC: [u8; 4] = [0x67, 0x67, 0x6d, 0x6c];

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
    pub note: &'static str,
}

/// Models offered in the UI. `base.en` is the default: a good accuracy/speed balance
/// for English dictation on Apple Silicon.
pub const REGISTRY: &[ModelSpec] = &[
    ModelSpec {
        id: "tiny.en",
        label: "Tiny (English)",
        filename: "ggml-tiny.en.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
        approx_mb: 75,
        sha256: "",
        note: "Fastest, lowest accuracy. Good on older Macs.",
    },
    ModelSpec {
        id: "base.en",
        label: "Base (English)",
        filename: "ggml-base.en.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
        approx_mb: 142,
        sha256: "",
        note: "Recommended. Balanced speed and accuracy for English.",
    },
    ModelSpec {
        id: "small.en",
        label: "Small (English)",
        filename: "ggml-small.en.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
        approx_mb: 466,
        sha256: "",
        note: "Higher accuracy, slower. Best on Apple Silicon.",
    },
];

pub fn spec(id: &str) -> Option<&'static ModelSpec> {
    REGISTRY.iter().find(|m| m.id == id)
}

/// Is the model for `id` present and structurally valid on disk?
pub fn is_installed(model_dir: &Path, id: &str) -> bool {
    let Some(spec) = spec(id) else { return false };
    let path = model_dir.join(spec.filename);
    validate_file(&path, spec).is_ok()
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

    let final_path = model_dir.join(spec.filename);
    let part_path = model_dir.join(format!("{}.part", spec.filename));

    let client = reqwest::blocking::Client::builder()
        .timeout(None) // large file; rely on cancellation instead
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
    let mut file = std::fs::File::create(&part_path)
        .map_err(|e| SpielError::Download(format!("cannot create {part_path:?}: {e}")))?;
    let mut hasher = Sha256::new();
    let mut downloaded: u64 = 0;
    let mut buf = [0u8; 64 * 1024];

    on_progress(0, total);
    loop {
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
        file.write_all(&buf[..n])
            .map_err(|e| SpielError::Download(format!("write error: {e}")))?;
        hasher.update(&buf[..n]);
        downloaded += n as u64;
        on_progress(downloaded, total);
    }
    file.sync_all().ok();
    drop(file);

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
fn validate_file(path: &Path, spec: &ModelSpec) -> Result<()> {
    let mut f = std::fs::File::open(path).map_err(|_| SpielError::ModelMissing)?;
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic)
        .map_err(|_| SpielError::Model("model file is truncated".into()))?;
    if magic != GGML_MAGIC {
        return Err(SpielError::Model(
            "file is not a valid GGML whisper model".into(),
        ));
    }
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    // Guard against a header-only stub; require at least half the expected size.
    let min = (spec.approx_mb as u64) * 1024 * 1024 / 2;
    if len < min {
        return Err(SpielError::Model(
            "model file is smaller than expected".into(),
        ));
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_default() {
        assert!(spec("base.en").is_some());
    }

    #[test]
    fn unknown_model_is_none() {
        assert!(spec("nope").is_none());
    }

    #[test]
    fn missing_file_not_installed() {
        let dir = std::env::temp_dir().join("spiel_model_test_missing");
        assert!(!is_installed(&dir, "base.en"));
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
}
