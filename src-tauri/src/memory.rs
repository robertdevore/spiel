use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct MemorySnapshot {
    pub rss_bytes: u64,
    pub source: &'static str,
}

pub fn snapshot() -> MemorySnapshot {
    MemorySnapshot {
        rss_bytes: resident_set_bytes().unwrap_or(0),
        source: source_name(),
    }
}

#[cfg(target_os = "linux")]
fn resident_set_bytes() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let pages = statm.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    Some(pages.saturating_mul(page_size_bytes()))
}

#[cfg(target_os = "macos")]
fn resident_set_bytes() -> Option<u64> {
    let output = std::process::Command::new("/bin/ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_ps_rss_kib(std::str::from_utf8(&output.stdout).ok()?)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn resident_set_bytes() -> Option<u64> {
    None
}

#[cfg(target_os = "linux")]
fn page_size_bytes() -> u64 {
    4096
}

#[cfg(target_os = "macos")]
fn parse_ps_rss_kib(raw: &str) -> Option<u64> {
    raw.trim()
        .parse::<u64>()
        .ok()
        .map(|kib| kib.saturating_mul(1024))
}

fn source_name() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "procfs"
    }
    #[cfg(target_os = "macos")]
    {
        "ps"
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        "unsupported"
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "macos")]
    #[test]
    fn parses_ps_rss_kib() {
        assert_eq!(super::parse_ps_rss_kib(" 1234\n"), Some(1_263_616));
        assert_eq!(super::parse_ps_rss_kib(""), None);
    }
}
