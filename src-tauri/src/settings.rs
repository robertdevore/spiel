//! Settings persistence for Spiel Phase 9/10.

use crate::database::Database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpielSettings {
    pub hotkey: String,
    pub history_enabled: bool,
    pub clipboard_restore_enabled: bool,
    pub default_transcription_engine: String,
    pub default_text_mode: String,
    pub default_cleanup_provider: String,
    pub local_whisper_binary_path: String,
    pub local_whisper_model_path: String,
    pub local_only_mode: bool,
    pub debug_logging_enabled: bool,
    pub auto_transcribe_after_recording: bool,
    pub auto_cleanup_after_transcription: bool,
    pub auto_save_history_after_cleanup: bool,
    pub auto_insert_after_cleanup: bool,
    /// Phase 12: whether cloud providers (OpenAI) are allowed
    pub cloud_providers_enabled: bool,
    /// Phase 12: OpenAI transcription model (default: whisper-1)
    pub openai_transcription_model: String,
    /// Phase 12: OpenAI cleanup model (default: gpt-4o-mini)
    pub openai_cleanup_model: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Default for SpielSettings {
    fn default() -> Self {
        Self {
            hotkey: "Alt+Shift+Space".into(),
            history_enabled: true,
            clipboard_restore_enabled: true,
            default_transcription_engine: "mock".into(),
            default_text_mode: "raw_dictation".into(),
            default_cleanup_provider: "basic".into(),
            local_whisper_binary_path: String::new(),
            local_whisper_model_path: String::new(),
            local_only_mode: true,
            debug_logging_enabled: false,
            auto_transcribe_after_recording: false,
            auto_cleanup_after_transcription: false,
            auto_save_history_after_cleanup: false,
            auto_insert_after_cleanup: false,
            cloud_providers_enabled: false,
            openai_transcription_model: "whisper-1".into(),
            openai_cleanup_model: "gpt-4o-mini".into(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSettingsRequest {
    pub hotkey: Option<String>,
    pub history_enabled: Option<bool>,
    pub clipboard_restore_enabled: Option<bool>,
    pub default_transcription_engine: Option<String>,
    pub default_text_mode: Option<String>,
    pub default_cleanup_provider: Option<String>,
    pub local_whisper_binary_path: Option<String>,
    pub local_whisper_model_path: Option<String>,
    pub local_only_mode: Option<bool>,
    pub debug_logging_enabled: Option<bool>,
    pub auto_transcribe_after_recording: Option<bool>,
    pub auto_cleanup_after_transcription: Option<bool>,
    pub auto_save_history_after_cleanup: Option<bool>,
    pub auto_insert_after_cleanup: Option<bool>,
    pub cloud_providers_enabled: Option<bool>,
    pub openai_transcription_model: Option<String>,
    pub openai_cleanup_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyStatus {
    pub local_only_mode: bool,
    pub history_enabled: bool,
    pub clipboard_restore_enabled: bool,
    pub debug_logging_enabled: bool,
    pub cloud_available: bool,
    pub openai_available: bool,
    pub local_llm_available: bool,
    pub history_encrypted: bool,
    pub clipboard_contents_stored: bool,
    pub api_keys_stored: bool,
    pub network_calls_possible: bool,
}

impl PrivacyStatus {
    pub fn from_settings(settings: &SpielSettings) -> Self {
        Self {
            local_only_mode: settings.local_only_mode,
            history_enabled: settings.history_enabled,
            clipboard_restore_enabled: settings.clipboard_restore_enabled,
            debug_logging_enabled: settings.debug_logging_enabled,
            cloud_available: settings.cloud_providers_enabled && !settings.local_only_mode,
            openai_available: settings.cloud_providers_enabled && !settings.local_only_mode,
            local_llm_available: false,
            history_encrypted: false,
            clipboard_contents_stored: false,
            api_keys_stored: false,
            network_calls_possible: settings.cloud_providers_enabled && !settings.local_only_mode,
        }
    }
}

pub fn load_settings(db: &Database) -> Result<SpielSettings, String> {
    let conn = db
        .conn
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;
    let result = conn.query_row(
        "SELECT hotkey, history_enabled, clipboard_restore_enabled, default_transcription_engine, default_text_mode, default_cleanup_provider, local_whisper_binary_path, local_whisper_model_path, local_only_mode, debug_logging_enabled, auto_transcribe_after_recording, auto_cleanup_after_transcription, auto_save_history_after_cleanup, auto_insert_after_cleanup, cloud_providers_enabled, openai_transcription_model, openai_cleanup_model, created_at, updated_at FROM app_settings WHERE id = 1",
        [],
        |row| Ok(SpielSettings {
            hotkey: row.get(0)?,
            history_enabled: row.get::<_,i32>(1)? != 0,
            clipboard_restore_enabled: row.get::<_,i32>(2)? != 0,
            default_transcription_engine: row.get(3)?,
            default_text_mode: row.get(4)?,
            default_cleanup_provider: row.get(5)?,
            local_whisper_binary_path: row.get(6)?,
            local_whisper_model_path: row.get(7)?,
            local_only_mode: row.get::<_,i32>(8)? != 0,
            debug_logging_enabled: row.get::<_,i32>(9)? != 0,
            auto_transcribe_after_recording: row.get::<_,i32>(10)? != 0,
            auto_cleanup_after_transcription: row.get::<_,i32>(11)? != 0,
            auto_save_history_after_cleanup: row.get::<_,i32>(12)? != 0,
            auto_insert_after_cleanup: row.get::<_,i32>(13)? != 0,
            cloud_providers_enabled: row.get::<_,i32>(14)? != 0,
            openai_transcription_model: row.get(15)?,
            openai_cleanup_model: row.get(16)?,
            created_at: row.get(17)?,
            updated_at: row.get(18)?,
        }),
    );
    match result {
        Ok(s) => Ok(s),
        Err(_) => {
            let d = SpielSettings::default();
            save_settings(db, &d)?;
            Ok(d)
        }
    }
}

pub fn save_settings(db: &Database, settings: &SpielSettings) -> Result<(), String> {
    let conn = db
        .conn
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;
    let now = crate::chrono_now_iso();
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (id, hotkey, history_enabled, clipboard_restore_enabled, default_transcription_engine, default_text_mode, default_cleanup_provider, local_whisper_binary_path, local_whisper_model_path, local_only_mode, debug_logging_enabled, auto_transcribe_after_recording, auto_cleanup_after_transcription, auto_save_history_after_cleanup, auto_insert_after_cleanup, cloud_providers_enabled, openai_transcription_model, openai_cleanup_model, created_at, updated_at) VALUES (1,?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",
        rusqlite::params![
            settings.hotkey, settings.history_enabled as i32, settings.clipboard_restore_enabled as i32,
            settings.default_transcription_engine, settings.default_text_mode, settings.default_cleanup_provider,
            settings.local_whisper_binary_path, settings.local_whisper_model_path,
            settings.local_only_mode as i32, settings.debug_logging_enabled as i32,
            settings.auto_transcribe_after_recording as i32, settings.auto_cleanup_after_transcription as i32,
            settings.auto_save_history_after_cleanup as i32, settings.auto_insert_after_cleanup as i32,
            settings.cloud_providers_enabled as i32, settings.openai_transcription_model, settings.openai_cleanup_model,
            settings.created_at, now,
        ],
    ).map_err(|e| format!("Failed to save settings: {}", e))?;
    Ok(())
}

pub fn reset_settings(db: &Database) -> Result<SpielSettings, String> {
    let d = SpielSettings::default();
    save_settings(db, &d)?;
    Ok(d)
}

pub fn validate_and_apply(
    current: &SpielSettings,
    update: &UpdateSettingsRequest,
) -> Result<SpielSettings, String> {
    let mut s = current.clone();
    if let Some(ref v) = update.hotkey {
        if v.trim().is_empty() {
            return Err("Hotkey cannot be empty.".into());
        }
        s.hotkey = v.clone();
    }
    if let Some(v) = update.history_enabled {
        s.history_enabled = v;
    }
    if let Some(v) = update.clipboard_restore_enabled {
        s.clipboard_restore_enabled = v;
    }
    if let Some(ref v) = update.default_transcription_engine {
        if !["mock", "local_whisper", "cloud"].contains(&v.as_str()) {
            return Err(format!("Invalid transcription engine '{}'", v));
        }
        s.default_transcription_engine = v.clone();
    }
    if let Some(ref v) = update.default_text_mode {
        if ![
            "raw_dictation",
            "clean_notes",
            "ai_prompt",
            "developer_review",
            "thought_piece",
        ]
        .contains(&v.as_str())
        {
            return Err(format!("Invalid text mode '{}'", v));
        }
        s.default_text_mode = v.clone();
    }
    if let Some(ref v) = update.default_cleanup_provider {
        if !["basic", "mock_ai", "openai", "local_llm_planned"].contains(&v.as_str()) {
            return Err(format!("Invalid cleanup provider '{}'", v));
        }
        s.default_cleanup_provider = v.clone();
    }
    if let Some(ref v) = update.local_whisper_binary_path {
        s.local_whisper_binary_path = v.clone();
    }
    if let Some(ref v) = update.local_whisper_model_path {
        s.local_whisper_model_path = v.clone();
    }
    if let Some(v) = update.local_only_mode {
        s.local_only_mode = v;
    }
    if let Some(v) = update.debug_logging_enabled {
        s.debug_logging_enabled = v;
    }
    if let Some(v) = update.auto_transcribe_after_recording {
        s.auto_transcribe_after_recording = v;
    }
    if let Some(v) = update.auto_cleanup_after_transcription {
        s.auto_cleanup_after_transcription = v;
    }
    if let Some(v) = update.auto_save_history_after_cleanup {
        s.auto_save_history_after_cleanup = v;
    }
    if let Some(v) = update.auto_insert_after_cleanup {
        s.auto_insert_after_cleanup = v;
    }
    if let Some(v) = update.cloud_providers_enabled {
        s.cloud_providers_enabled = v;
    }
    if let Some(ref v) = update.openai_transcription_model {
        if v.trim().is_empty() {
            return Err("OpenAI transcription model cannot be empty.".into());
        }
        s.openai_transcription_model = v.clone();
    }
    if let Some(ref v) = update.openai_cleanup_model {
        if v.trim().is_empty() {
            return Err("OpenAI cleanup model cannot be empty.".into());
        }
        s.openai_cleanup_model = v.clone();
    }
    Ok(s)
}
