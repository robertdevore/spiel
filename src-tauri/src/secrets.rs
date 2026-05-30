//! Secure secret storage for Spiel Phase 12.
//!
//! Stores API keys in memory only — not persisted to disk.
//! Keys are never returned in full to the frontend after initial set.
//! This is a session-only implementation. Keys are lost on app restart.
//! Future phases may integrate Tauri Stronghold for persistent encrypted storage.
//!
//! Architecture:
//! - `SecretStore` wraps a `HashMap<String, String>` behind a Mutex
//! - `set()` stores a key by name
//! - `get_status()` returns configured/not_configured with optional last-4 hint
//! - `get()` returns the key for internal use (never exposed to frontend)
//! - `delete()` removes a key
//! - `is_configured()` checks if a key exists
//!
//! Security:
//! - Keys never logged
//! - Keys never serialized in API responses
//! - Status only reveals configured/not_configured + optional suffix hint

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Status of a stored API key (safe to return to frontend).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyStatus {
    /// Name of the key (e.g., "openai")
    pub key_name: String,
    /// Whether a key is currently stored
    pub configured: bool,
    /// Last 4 characters of the key if configured (for user verification), null otherwise
    pub last_four: Option<String>,
}

/// Response for the validate_openai_provider_config command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfigValidation {
    /// Whether the provider is ready to use
    pub ready: bool,
    /// Human-readable status message
    pub message: String,
    /// Specific blockers (empty if ready)
    pub blockers: Vec<String>,
}

/// In-memory secret store. Not persisted. Cleared on app restart.
pub struct SecretStore {
    /// Map of key name → key value
    keys: HashMap<String, String>,
}

impl SecretStore {
    /// Create a new empty secret store.
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    /// Store a secret key. Returns the key name on success.
    /// The key value is stored but never returned in full to the frontend.
    pub fn set(&mut self, key_name: &str, key_value: &str) -> Result<(), String> {
        let trimmed_name = key_name.trim();
        let trimmed_value = key_value.trim();

        if trimmed_name.is_empty() {
            return Err("Key name cannot be empty.".into());
        }

        if trimmed_value.is_empty() {
            return Err("Key value cannot be empty. Provide a valid API key.".into());
        }

        // Basic format validation: OpenAI keys start with "sk-"
        if trimmed_name == "openai" && !trimmed_value.starts_with("sk-") {
            return Err("Invalid OpenAI API key format. Keys should start with 'sk-'.".into());
        }

        self.keys
            .insert(trimmed_name.to_string(), trimmed_value.to_string());
        Ok(())
    }

    /// Get the status of a key (safe to return to frontend).
    /// Never returns the full key value.
    pub fn get_status(&self, key_name: &str) -> ApiKeyStatus {
        let trimmed_name = key_name.trim();
        match self.keys.get(trimmed_name) {
            Some(key) => {
                let last_four = if key.len() >= 4 {
                    Some(key[key.len() - 4..].to_string())
                } else {
                    None
                };
                ApiKeyStatus {
                    key_name: trimmed_name.to_string(),
                    configured: true,
                    last_four,
                }
            }
            None => ApiKeyStatus {
                key_name: trimmed_name.to_string(),
                configured: false,
                last_four: None,
            },
        }
    }

    /// Get the actual key value for internal use (never expose to frontend).
    pub fn get(&self, key_name: &str) -> Option<String> {
        self.keys.get(key_name.trim()).cloned()
    }

    /// Delete a stored key. Returns Ok even if the key wasn't stored.
    pub fn delete(&mut self, key_name: &str) {
        self.keys.remove(key_name.trim());
    }

    /// Check whether a key is configured.
    pub fn is_configured(&self, key_name: &str) -> bool {
        self.keys.contains_key(key_name.trim())
    }

    /// Validate that a provider is ready to use.
    /// Checks: API key configured, local-only mode off, cloud providers enabled.
    pub fn validate_provider_config(
        &self,
        key_name: &str,
        local_only_mode: bool,
        cloud_providers_enabled: bool,
    ) -> ProviderConfigValidation {
        let mut blockers: Vec<String> = Vec::new();

        if local_only_mode {
            blockers.push(
                "Local-only mode is on. Disable local-only mode in Settings to use cloud providers."
                    .into(),
            );
        }

        if !cloud_providers_enabled {
            blockers
                .push("Cloud providers are disabled. Enable cloud providers in Settings.".into());
        }

        if !self.is_configured(key_name) {
            blockers.push(format!(
                "{} API key is not configured. Add your API key in Settings.",
                key_name
            ));
        }

        if blockers.is_empty() {
            ProviderConfigValidation {
                ready: true,
                message: format!("{} provider is ready to use.", key_name),
                blockers: vec![],
            }
        } else {
            ProviderConfigValidation {
                ready: false,
                message: blockers.join(" "),
                blockers,
            }
        }
    }
}

impl Default for SecretStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let mut store = SecretStore::new();
        store.set("openai", "sk-test1234abcd").unwrap();
        assert!(store.is_configured("openai"));
        assert_eq!(store.get("openai"), Some("sk-test1234abcd".into()));
    }

    #[test]
    fn test_status_never_returns_full_key() {
        let mut store = SecretStore::new();
        store.set("openai", "sk-test1234abcd").unwrap();
        let status = store.get_status("openai");
        assert!(status.configured);
        // Status should NOT contain the full key
        let status_json = serde_json::to_string(&status).unwrap();
        assert!(!status_json.contains("sk-test1234abcd"));
        // Should contain the hint
        assert!(status.last_four == Some("abcd".into()));
    }

    #[test]
    fn test_validate_all_blockers() {
        let store = SecretStore::new();
        let validation = store.validate_provider_config("openai", true, false);
        assert!(!validation.ready);
        assert_eq!(validation.blockers.len(), 3);
    }

    #[test]
    fn test_validate_ready() {
        let mut store = SecretStore::new();
        store.set("openai", "sk-test1234abcd").unwrap();
        let validation = store.validate_provider_config("openai", false, true);
        assert!(validation.ready);
        assert!(validation.blockers.is_empty());
    }

    #[test]
    fn test_delete() {
        let mut store = SecretStore::new();
        store.set("openai", "sk-test1234abcd").unwrap();
        assert!(store.is_configured("openai"));
        store.delete("openai");
        assert!(!store.is_configured("openai"));
    }

    #[test]
    fn test_invalid_openai_key_format() {
        let mut store = SecretStore::new();
        let result = store.set("openai", "not-a-valid-key");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_key_value() {
        let mut store = SecretStore::new();
        let result = store.set("openai", "");
        assert!(result.is_err());
    }
}
