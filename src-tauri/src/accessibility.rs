//! macOS Accessibility permission helpers.
//!
//! Synthesizing Cmd+V (auto-paste) requires the app to be trusted for Accessibility.
//! macOS won't let us *grant* it programmatically — only the user can, in System
//! Settings — but we can check the current state and open the right pane to guide them.

/// Is the app currently trusted for Accessibility (i.e. can post key events)?
#[cfg(target_os = "macos")]
pub fn is_trusted() -> bool {
    macos_accessibility_client::accessibility::application_is_trusted()
}

/// Check trust and, if missing, trigger the system "grant Accessibility" prompt once.
#[cfg(target_os = "macos")]
pub fn prompt_if_needed() -> bool {
    macos_accessibility_client::accessibility::application_is_trusted_with_prompt()
}

/// Open System Settings at Privacy & Security → Accessibility.
#[cfg(target_os = "macos")]
pub fn open_settings_pane() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

#[cfg(target_os = "macos")]
pub fn is_supported() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub fn is_trusted() -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
pub fn prompt_if_needed() -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
pub fn open_settings_pane() {}

#[cfg(not(target_os = "macos"))]
pub fn is_supported() -> bool {
    false
}
