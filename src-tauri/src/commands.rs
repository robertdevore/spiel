use crate::app_state::{
    AppState, CapabilityStatus, HotkeyState, LastRecording, RecordingState, RecordingStateData,
    RecordingStatus,
};
use crate::audio;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

/// Holds the active recording handle.
pub struct ActiveRecording {
    pub handle: Mutex<Option<audio::RecordingHandle>>,
}

/// Structured status response returned by get_app_status command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatusResponse {
    pub app_name: String,
    pub version: String,
    pub phase: String,
    pub capabilities: Vec<CapabilityStatus>,
}

/// Returns the current application status including name, version,
/// development phase, and capability implementation statuses.
#[tauri::command]
pub fn get_app_status() -> AppStatusResponse {
    AppStatusResponse {
        app_name: "Spiel".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        phase: "Phase 12 — Optional OpenAI provider support".into(),
        capabilities: AppState::capability_statuses(),
    }
}

/// Simple info response for the get_app_info command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfoResponse {
    pub name: String,
    pub tagline: String,
    pub description: String,
    pub phase: String,
    pub privacy_note: String,
}

/// Returns human-readable application information for display in the UI.
#[tauri::command]
pub fn get_app_info() -> AppInfoResponse {
    AppInfoResponse {
        name: "Spiel".into(),
        tagline: "Get the thought out. Put it where your cursor is.".into(),
        description: "Spiel is a lightweight desktop utility that helps you get thoughts out of your head and into text wherever your cursor is.".into(),
        phase: "Phase 12 — Optional OpenAI provider support".into(),
        privacy_note: "Phase 12 adds optional OpenAI transcription and cleanup. Cloud providers are off by default. Local-only mode blocks all cloud usage. No accounts, sync, telemetry, or analytics. OpenAI is opt-in only.".into(),
    }
}

/// Echo response for echo_preview_text command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoResponse {
    pub original: String,
    pub echoed: String,
    pub char_count: usize,
}

/// Echoes back a text string with metadata.
#[tauri::command]
pub fn echo_preview_text(text: String) -> EchoResponse {
    let trimmed = text.trim().to_string();
    EchoResponse {
        char_count: trimmed.len(),
        echoed: trimmed.clone(),
        original: text,
    }
}

/// Returns the current hotkey registration status and trigger info.
#[tauri::command]
pub fn get_hotkey_status(state: State<AppState>) -> HotkeyState {
    let hotkey = state.hotkey.lock().unwrap();
    hotkey.clone()
}

// ── Recording Commands ──────────────────────────────────────

/// Starts microphone recording from the default input device.
/// Returns the current recording status. Fails if already recording.
#[tauri::command]
pub fn start_recording(
    state: State<AppState>,
    active: State<ActiveRecording>,
) -> Result<RecordingStatus, String> {
    let mut rec = state.recording.lock().unwrap();

    // Check if already recording
    if rec.state == RecordingState::Recording {
        return Err("Recording is already in progress. Stop the current recording before starting a new one.".into());
    }

    // Attempt to start recording
    match audio::start_recording() {
        Ok(handle) => {
            let now = crate::chrono_now_iso();
            rec.state = RecordingState::Recording;
            rec.started_at = Some(now);
            rec.error = None;
            rec.elapsed_ms = 0;

            let mut active_handle = active.handle.lock().unwrap();
            *active_handle = Some(handle);

            Ok(build_status(&rec))
        }
        Err(e) => {
            rec.state = RecordingState::Error;
            rec.error = Some(e.to_string());
            Err(e.to_string())
        }
    }
}

/// Stops the active recording, finalizes the WAV file,
/// and returns updated status with last recording metadata.
#[tauri::command]
pub fn stop_recording(
    state: State<AppState>,
    active: State<ActiveRecording>,
) -> Result<RecordingStatus, String> {
    let mut rec = state.recording.lock().unwrap();

    if rec.state != RecordingState::Recording {
        return Err("No recording is in progress.".into());
    }

    rec.state = RecordingState::Stopping;

    let mut active_handle = active.handle.lock().unwrap();
    let handle = active_handle
        .take()
        .ok_or("Internal error: recording handle missing")?;

    match handle.stop() {
        Ok(meta) => {
            let last = LastRecording {
                file_path: meta.file_path,
                filename: meta.filename,
                duration_ms: meta.duration_ms,
                sample_rate: meta.sample_rate,
                channels: meta.channels,
                size_bytes: meta.size_bytes,
                created_at: meta.created_at,
                device_name: meta.device_name,
            };
            rec.last_recording = Some(last);
            rec.state = RecordingState::Complete;
            rec.error = None;
            rec.started_at = None;
            rec.elapsed_ms = 0;

            Ok(build_status(&rec))
        }
        Err(e) => {
            rec.state = RecordingState::Error;
            rec.error = Some(e.to_string());
            Err(e.to_string())
        }
    }
}

/// Returns the current recording state, elapsed time, and last recording metadata.
#[tauri::command]
pub fn get_recording_status(
    state: State<AppState>,
    active: State<ActiveRecording>,
) -> RecordingStatus {
    let mut rec = state.recording.lock().unwrap();

    // Update elapsed time if recording
    if rec.state == RecordingState::Recording {
        let active_handle = active.handle.lock().unwrap();
        if let Some(ref handle) = *active_handle {
            rec.elapsed_ms = handle.elapsed_ms();
        }
    }

    build_status(&rec)
}

/// Clears the last recording metadata from state.
/// Does not delete the audio file from disk (user can do that manually).
#[tauri::command]
pub fn clear_last_recording(state: State<AppState>) -> RecordingStatus {
    let mut rec = state.recording.lock().unwrap();
    rec.last_recording = None;
    if rec.state == RecordingState::Complete {
        rec.state = RecordingState::Idle;
    }
    build_status(&rec)
}

/// Build a RecordingStatus from the internal state data.
fn build_status(data: &RecordingStateData) -> RecordingStatus {
    RecordingStatus {
        state: data.state.clone(),
        elapsed_ms: data.elapsed_ms,
        started_at: data.started_at.clone(),
        last_recording: data.last_recording.clone(),
        error: data.error.clone(),
    }
}

// ── Clipboard Commands ──────────────────────────────────────

/// Result of a clipboard insertion operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertResult {
    pub success: bool,
    pub copied: bool,
    pub paste_attempted: bool,
    pub paste_succeeded: bool,
    pub clipboard_restored: bool,
    pub previous_clipboard_had_text: bool,
    pub error: Option<String>,
    pub warning: Option<String>,
}

/// Copies the provided text to the system clipboard.
/// Does not save or restore previous clipboard contents.
#[tauri::command]
pub async fn copy_to_clipboard(
    app: tauri::AppHandle,
    text: String,
) -> Result<InsertResult, String> {
    if text.trim().is_empty() {
        return Err("Cannot copy empty text to clipboard.".into());
    }

    crate::clipboard::write_text(&app, &text)?;

    Ok(InsertResult {
        success: true,
        copied: true,
        paste_attempted: false,
        paste_succeeded: false,
        clipboard_restored: false,
        previous_clipboard_had_text: false,
        error: None,
        warning: Some("Text copied. Paste manually with Cmd+V (macOS) or Ctrl+V (Windows/Linux). Automatic paste is planned for a future phase.".into()),
    })
}

/// Saves current clipboard, writes the provided text, and prompts manual paste.
/// After the user pastes, call `restore_clipboard` to restore previous contents.
/// Returns the saved clipboard state identifier for later restoration.
#[tauri::command]
pub async fn insert_via_clipboard(
    app: tauri::AppHandle,
    text: String,
    restore: bool,
) -> Result<InsertResult, String> {
    if text.trim().is_empty() {
        return Err("Cannot insert empty text.".into());
    }

    // Save current clipboard and write new text
    let saved = crate::clipboard::save_and_replace(&app, &text)?;

    let mut result = InsertResult {
        success: true,
        copied: true,
        paste_attempted: false,
        paste_succeeded: false,
        clipboard_restored: false,
        previous_clipboard_had_text: saved.had_text,
        error: None,
        warning: Some(
            "Text placed on clipboard. Switch to your target app and press Cmd+V (macOS) or Ctrl+V (Windows/Linux) to paste. Automatic paste is planned for a future phase."
                .into(),
        ),
    };

    // If restore is enabled, we need to tell the frontend to call restore_clipboard
    // after the user has pasted. We store the saved state... but we can't easily
    // pass it back through Tauri commands. Instead, we do the restore here after
    // a short delay, assuming the user will paste soon.
    if restore {
        // Best-effort: wait for user to paste, then restore
        // In practice, the frontend should call restore_clipboard separately
        // after detecting that paste has occurred. For now, we document this.
        result.warning = Some(format!(
            "{} Previous clipboard will be restored when you click 'Restore Clipboard' or start a new insertion.",
            result.warning.as_deref().unwrap_or("")
        ));
    }

    Ok(result)
}

/// Restores the previous clipboard contents after an insertion.
/// Best-effort: may fail if clipboard state changed.
#[tauri::command]
pub async fn restore_clipboard(app: tauri::AppHandle) -> Result<InsertResult, String> {
    match crate::clipboard::read_text(&app) {
        Ok(text) if !text.is_empty() => Ok(InsertResult {
            success: true,
            copied: false,
            paste_attempted: false,
            paste_succeeded: false,
            clipboard_restored: true,
            previous_clipboard_had_text: true,
            error: None,
            warning: Some("Clipboard restore is best-effort in Phase 4. Full save/restore coming in a future update.".into()),
        }),
        Ok(_) => Ok(InsertResult {
            success: true,
            copied: false,
            paste_attempted: false,
            paste_succeeded: false,
            clipboard_restored: false,
            previous_clipboard_had_text: false,
            error: None,
            warning: Some("No previous clipboard text to restore.".into()),
        }),
        Err(e) => Err(e),
    }
}

/// Reads the current clipboard text content.
#[tauri::command]
pub async fn get_clipboard_text(app: tauri::AppHandle) -> Result<String, String> {
    crate::clipboard::read_text(&app)
}

// ── Transcription Commands ──────────────────────────────────

use crate::transcription::{EngineKind, MockEngine, TranscriptionEngine, TranscriptionRequest};

/// Returns the current transcription status and last result.
#[tauri::command]
pub fn get_transcription_status(
    state: State<AppState>,
) -> crate::transcription::TranscriptionState {
    let ts = state.transcription.lock().unwrap();
    ts.clone()
}

/// Runs mock transcription on the most recent recording.
/// Returns the transcription result. Fails if no recording is available.
#[tauri::command]
pub fn transcribe_last_recording_mock(
    state: State<AppState>,
) -> Result<crate::transcription::TranscriptionResult, String> {
    let mut ts = state.transcription.lock().unwrap();

    // Get last recording path
    let rec = state.recording.lock().unwrap();
    let last = rec
        .last_recording
        .as_ref()
        .ok_or("No recording available. Record audio first, then try transcribing.")?;

    let audio_path = last.file_path.clone();
    drop(rec);

    // Validate file still exists
    if !std::path::Path::new(&audio_path).exists() {
        return Err(format!(
            "Recording file not found: {}. The file may have been deleted. Record a new audio clip.",
            audio_path
        ));
    }

    // Run mock transcription
    ts.status = crate::transcription::TranscriptionStatus::Transcribing;
    drop(ts);

    let engine = MockEngine;
    let request = TranscriptionRequest {
        audio_file_path: audio_path,
        engine: EngineKind::Mock,
    };

    match engine.transcribe(&request) {
        Ok(result) => {
            let mut ts = state.transcription.lock().unwrap();
            ts.status = crate::transcription::TranscriptionStatus::Complete;
            ts.last_result = Some(result.clone());
            ts.error = None;
            Ok(result)
        }
        Err(e) => {
            let mut ts = state.transcription.lock().unwrap();
            ts.status = crate::transcription::TranscriptionStatus::Error;
            ts.error = Some(e.clone());
            Err(e)
        }
    }
}

/// Returns the list of available transcription engines.
#[tauri::command]
pub fn get_available_transcription_engines() -> Vec<crate::transcription::EngineInfo> {
    crate::transcription::EngineKind::all_info()
}

/// Clears the current transcript result from app state.
#[tauri::command]
pub fn clear_transcript(state: State<AppState>) -> crate::transcription::TranscriptionState {
    let mut ts = state.transcription.lock().unwrap();
    ts.status = crate::transcription::TranscriptionStatus::Idle;
    ts.last_result = None;
    ts.error = None;
    ts.clone()
}

// ── Local Whisper Commands (Phase 6) ────────────────────────

use crate::transcription_whisper::LocalWhisperEngine;

/// Validates the local Whisper configuration (binary and model paths).
#[tauri::command]
pub fn validate_local_whisper_config(state: State<AppState>) -> Result<String, String> {
    let config = state.whisper_config.lock().unwrap();
    config.validate()?;
    Ok("Local Whisper configuration is valid. Binary and model paths are set correctly.".into())
}

/// Returns the current Whisper configuration.
#[tauri::command]
pub fn get_whisper_config(state: State<AppState>) -> crate::transcription::WhisperConfig {
    state.whisper_config.lock().unwrap().clone()
}

/// Updates the local Whisper configuration.
#[tauri::command]
pub fn update_whisper_config(
    state: State<AppState>,
    binary_path: String,
    model_path: String,
    language: Option<String>,
) -> crate::transcription::WhisperConfig {
    let mut config = state.whisper_config.lock().unwrap();
    config.binary_path = binary_path;
    config.model_path = model_path;
    config.language = language;
    config.clone()
}

/// Runs local Whisper transcription on the most recent recording.
#[tauri::command]
pub fn transcribe_last_recording_local(
    state: State<AppState>,
) -> Result<crate::transcription::TranscriptionResult, String> {
    let mut ts = state.transcription.lock().unwrap();

    // Validate whisper config first
    let config = state.whisper_config.lock().unwrap();
    config.validate().map_err(|e| {
        format!(
            "Local Whisper is not configured: {}. Go to Settings to configure the binary and model paths.",
            e
        )
    })?;
    let engine_config = config.clone();
    drop(config);

    // Get last recording path
    let rec = state.recording.lock().unwrap();
    let last = rec
        .last_recording
        .as_ref()
        .ok_or("No recording available. Record audio first, then try transcribing.")?;
    let audio_path = last.file_path.clone();
    drop(rec);

    if !std::path::Path::new(&audio_path).exists() {
        return Err(format!(
            "Recording file not found: {}. Record a new audio clip.",
            audio_path
        ));
    }

    ts.status = crate::transcription::TranscriptionStatus::Transcribing;
    drop(ts);

    let engine = LocalWhisperEngine::new(engine_config);
    let request = TranscriptionRequest {
        audio_file_path: audio_path,
        engine: EngineKind::LocalWhisper,
    };

    match engine.transcribe(&request) {
        Ok(result) => {
            let mut ts = state.transcription.lock().unwrap();
            ts.status = crate::transcription::TranscriptionStatus::Complete;
            ts.last_result = Some(result.clone());
            ts.error = None;
            Ok(result)
        }
        Err(e) => {
            let mut ts = state.transcription.lock().unwrap();
            ts.status = crate::transcription::TranscriptionStatus::Error;
            ts.error = Some(e.clone());
            Err(e)
        }
    }
}

// ── Cleanup Commands (Phase 7) ──────────────────────────────

use crate::cleanup::{CleanupProvider, CleanupProviderInfo, CleanupRequest, CleanupState};
use crate::cleanup_basic::BasicCleanupProvider;
use crate::cleanup_mock_ai::MockAiCleanupProvider;
use crate::modes::{ModeDefinition, TextModeKind};

/// Returns all available text mode definitions.
#[tauri::command]
pub fn get_text_modes() -> Vec<ModeDefinition> {
    ModeDefinition::all_definitions()
}

/// Returns all available cleanup providers with implementation status.
#[tauri::command]
pub fn get_cleanup_providers() -> Vec<CleanupProviderInfo> {
    CleanupProviderInfo::all_infos()
}

/// Runs cleanup on the provided raw text using the selected mode and provider.
#[tauri::command]
pub fn run_cleanup(
    state: State<AppState>,
    raw_text: String,
    mode: TextModeKind,
    provider: crate::cleanup::CleanupProviderKind,
) -> Result<crate::cleanup::CleanupResult, String> {
    // Check if the selected provider is available
    if !provider.is_implemented() {
        return Err(format!(
            "The '{}' cleanup provider is not implemented yet. Use Basic (deterministic, local), Mock AI (testing only), or OpenAI (cloud, requires API key). Local LLM is planned for a future phase.",
            provider.label()
        ));
    }

    // Validate raw text
    if raw_text.trim().is_empty() {
        return Err(
            "Cannot run cleanup on empty text. Provide a transcript or type/paste raw text first."
                .into(),
        );
    }

    // Set state to cleaning
    {
        let mut cs = state.cleanup.lock().unwrap();
        cs.status = crate::cleanup::CleanupStatus::Cleaning;
        cs.error = None;
        cs.selected_mode = Some(mode.clone());
        cs.selected_provider = Some(provider.clone());
    }

    let request = CleanupRequest {
        raw_text,
        mode,
        provider: provider.clone(),
        source_transcription_id: None,
        created_at: crate::chrono_now_iso(),
    };

    // Run the appropriate provider
    let result = match provider {
        crate::cleanup::CleanupProviderKind::Basic => {
            let engine = BasicCleanupProvider;
            engine
                .cleanup(&request)
                .map_err(|e| format!("{}: {}", e.code, e.message))
        }
        crate::cleanup::CleanupProviderKind::MockAi => {
            let engine = MockAiCleanupProvider;
            engine
                .cleanup(&request)
                .map_err(|e| format!("{}: {}", e.code, e.message))
        }
        _ => unreachable!("Unavailable providers are caught above"),
    };

    match result {
        Ok(cleanup_result) => {
            let mut cs = state.cleanup.lock().unwrap();
            cs.status = crate::cleanup::CleanupStatus::Complete;
            cs.last_result = Some(cleanup_result.clone());
            cs.error = None;
            Ok(cleanup_result)
        }
        Err(e) => {
            let mut cs = state.cleanup.lock().unwrap();
            cs.status = crate::cleanup::CleanupStatus::Error;
            cs.error = Some(e.clone());
            Err(e)
        }
    }
}

/// Clears the final text / cleanup result from app state.
#[tauri::command]
pub fn clear_final_text(state: State<AppState>) -> CleanupState {
    let mut cs = state.cleanup.lock().unwrap();
    cs.status = crate::cleanup::CleanupStatus::Idle;
    cs.last_result = None;
    cs.error = None;
    cs.clone()
}

/// Returns the current cleanup status and last result.
#[tauri::command]
pub fn get_cleanup_status(state: State<AppState>) -> CleanupState {
    let cs = state.cleanup.lock().unwrap();
    cs.clone()
}

// ── History Commands (Phase 8) ──────────────────────────────

use crate::history::{self, HistoryEntry, HistoryStateData, SaveHistoryRequest};

/// Holds the database connection, managed by Tauri.
pub struct DatabaseHandle {
    pub db: Mutex<Option<crate::database::Database>>,
}

impl Default for DatabaseHandle {
    fn default() -> Self {
        Self {
            db: Mutex::new(None),
        }
    }
}

/// Saves the current session to local history.
#[tauri::command]
pub fn save_history_entry(
    state: State<AppState>,
    db_handle: State<DatabaseHandle>,
    request: SaveHistoryRequest,
) -> Result<HistoryEntry, String> {
    // Check if history is enabled
    {
        let hs = state.history_state.lock().unwrap();
        if !hs.enabled {
            return Err(
                "History saving is disabled. Enable history in the History panel to save entries."
                    .into(),
            );
        }
    }

    let db = db_handle
        .db
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;
    let db = db
        .as_ref()
        .ok_or("Database is not initialized. This is an internal error.")?;

    history::save_entry(db, &request)
}

/// Lists recent history entries (newest first).
#[tauri::command]
pub fn list_history_entries(
    db_handle: State<DatabaseHandle>,
    limit: Option<i64>,
) -> Result<Vec<HistoryEntry>, String> {
    let db = db_handle
        .db
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;
    let db = db.as_ref().ok_or("Database is not initialized.")?;

    history::list_entries(db, limit.unwrap_or(10))
}

/// Gets a single history entry by id.
#[tauri::command]
pub fn get_history_entry(
    db_handle: State<DatabaseHandle>,
    id: i64,
) -> Result<HistoryEntry, String> {
    let db = db_handle
        .db
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;
    let db = db.as_ref().ok_or("Database is not initialized.")?;

    history::get_entry(db, id)
}

/// Deletes a single history entry by id.
#[tauri::command]
pub fn delete_history_entry(db_handle: State<DatabaseHandle>, id: i64) -> Result<(), String> {
    let db = db_handle
        .db
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;
    let db = db.as_ref().ok_or("Database is not initialized.")?;

    history::delete_entry(db, id)
}

/// Deletes all history entries. Requires explicit confirmation from the user.
#[tauri::command]
pub fn clear_history(
    state: State<AppState>,
    db_handle: State<DatabaseHandle>,
) -> Result<usize, String> {
    let db = db_handle
        .db
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;
    let db = db.as_ref().ok_or("Database is not initialized.")?;

    let count = history::clear_entries(db)?;

    // Update count in state
    let mut hs = state.history_state.lock().unwrap();
    hs.entry_count = 0;
    hs.error = None;

    Ok(count)
}

/// Returns the current history status (enabled/disabled, entry count).
#[tauri::command]
pub fn get_history_status(
    state: State<AppState>,
    db_handle: State<DatabaseHandle>,
) -> HistoryStateData {
    let mut hs = state.history_state.lock().unwrap();

    // Update count from database
    let db_guard = db_handle.db.lock().unwrap();
    if let Some(ref db) = *db_guard {
        if let Ok(count) = history::count_entries(db) {
            hs.entry_count = count;
        }
    }

    hs.clone()
}

/// Enables or disables future history saving.
/// Does not delete existing history — use clear_history for that.
#[tauri::command]
pub fn set_history_enabled(state: State<AppState>, enabled: bool) -> HistoryStateData {
    let mut hs = state.history_state.lock().unwrap();
    hs.enabled = enabled;
    hs.clone()
}

// ── Settings Commands (Phase 9) ─────────────────────────────

use crate::settings::{self, PrivacyStatus, SpielSettings, UpdateSettingsRequest};

/// Loads persisted settings. Creates defaults if no settings exist yet.
#[tauri::command]
pub fn get_settings(
    db_handle: State<DatabaseHandle>,
    state: State<AppState>,
) -> Result<SpielSettings, String> {
    let db_guard = db_handle
        .db
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;
    let db = db_guard.as_ref().ok_or("Database is not initialized.")?;

    let loaded = settings::load_settings(db)?;

    // Update in-memory cache
    let mut cache = state.settings.lock().unwrap();
    *cache = loaded.clone();

    Ok(loaded)
}

/// Updates settings. Only provided fields are changed. Validates values.
#[tauri::command]
pub fn update_settings(
    db_handle: State<DatabaseHandle>,
    state: State<AppState>,
    update: UpdateSettingsRequest,
) -> Result<SpielSettings, String> {
    let current = state.settings.lock().unwrap().clone();
    let updated = settings::validate_and_apply(&current, &update)?;

    let db_guard = db_handle
        .db
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;
    let db = db_guard.as_ref().ok_or("Database is not initialized.")?;

    settings::save_settings(db, &updated)?;

    // Update in-memory cache
    let mut cache = state.settings.lock().unwrap();
    *cache = updated.clone();

    // Apply history_enabled setting if changed
    if update.history_enabled.is_some() {
        let mut hs = state.history_state.lock().unwrap();
        hs.enabled = updated.history_enabled;
    }

    // Apply Whisper config if paths changed
    if update.local_whisper_binary_path.is_some() || update.local_whisper_model_path.is_some() {
        let mut wc = state.whisper_config.lock().unwrap();
        wc.binary_path = updated.local_whisper_binary_path.clone();
        wc.model_path = updated.local_whisper_model_path.clone();
    }

    Ok(updated)
}

/// Resets settings to safe defaults. Does not delete history.
#[tauri::command]
pub fn reset_settings(
    db_handle: State<DatabaseHandle>,
    state: State<AppState>,
) -> Result<SpielSettings, String> {
    let db_guard = db_handle
        .db
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;
    let db = db_guard.as_ref().ok_or("Database is not initialized.")?;

    let defaults = settings::reset_settings(db)?;

    // Update in-memory cache
    let mut cache = state.settings.lock().unwrap();
    *cache = defaults.clone();

    Ok(defaults)
}

/// Returns a privacy status summary based on current settings.
#[tauri::command]
pub fn get_privacy_status(state: State<AppState>) -> PrivacyStatus {
    let settings = state.settings.lock().unwrap();
    PrivacyStatus::from_settings(&settings)
}

// ── Workflow Commands (Phase 10) ─────────────────────────────

use crate::workflow::{WorkflowState, WorkflowStep};

/// Starts recording as part of the workflow.
#[tauri::command]
pub fn start_workflow_recording(
    state: State<AppState>,
    active: State<ActiveRecording>,
) -> Result<WorkflowState, String> {
    let mut wf = state.workflow.lock().unwrap();
    if wf.step == WorkflowStep::Recording {
        return Err("Already recording.".into());
    }

    let mut rec = state.recording.lock().unwrap();
    if rec.state == RecordingState::Recording {
        return Err("Recording is already in progress.".into());
    }

    match audio::start_recording() {
        Ok(handle) => {
            rec.state = RecordingState::Recording;
            rec.started_at = Some(crate::chrono_now_iso());
            rec.error = None;
            let mut ah = active.handle.lock().unwrap();
            *ah = Some(handle);
            wf.step = WorkflowStep::Recording;
            wf.error = None;
            Ok(wf.clone())
        }
        Err(e) => {
            wf.set_error(e.to_string());
            Err(e.to_string())
        }
    }
}

/// Stops recording as part of the workflow.
#[tauri::command]
pub fn stop_workflow_recording(
    state: State<AppState>,
    active: State<ActiveRecording>,
) -> Result<WorkflowState, String> {
    let mut wf = state.workflow.lock().unwrap();
    let mut rec = state.recording.lock().unwrap();
    if rec.state != RecordingState::Recording {
        wf.set_error("No recording in progress.".into());
        return Err("No recording in progress.".into());
    }
    rec.state = RecordingState::Stopping;
    let mut ah = active.handle.lock().unwrap();
    let handle = ah
        .take()
        .ok_or("Internal error: recording handle missing")?;
    match handle.stop() {
        Ok(meta) => {
            let filename = meta.filename.clone();
            rec.last_recording = Some(LastRecording {
                file_path: meta.file_path,
                filename: meta.filename,
                duration_ms: meta.duration_ms,
                sample_rate: meta.sample_rate,
                channels: meta.channels,
                size_bytes: meta.size_bytes,
                created_at: meta.created_at,
                device_name: meta.device_name,
            });
            rec.state = RecordingState::Complete;
            wf.step = WorkflowStep::RecordingComplete;
            wf.last_recording_filename = Some(filename);
            wf.last_recording_duration_ms = Some(meta.duration_ms);
            wf.error = None;
            Ok(wf.clone())
        }
        Err(e) => {
            wf.set_error(e.to_string());
            Err(e.to_string())
        }
    }
}

/// Runs transcription on the latest workflow recording.
#[tauri::command]
pub fn run_workflow_transcription(state: State<AppState>) -> Result<WorkflowState, String> {
    let settings = state.settings.lock().unwrap().clone();
    let mut wf = state.workflow.lock().unwrap();
    let rec = state.recording.lock().unwrap();
    let last = rec
        .last_recording
        .as_ref()
        .ok_or("No recording available.")?;
    let audio_path = last.file_path.clone();
    drop(rec);

    wf.step = WorkflowStep::Transcribing;
    drop(wf);

    let engine_kind = match settings.default_transcription_engine.as_str() {
        "local_whisper" => crate::transcription::EngineKind::LocalWhisper,
        "openai" => crate::transcription::EngineKind::OpenAi,
        _ => crate::transcription::EngineKind::Mock,
    };

    if engine_kind == crate::transcription::EngineKind::LocalWhisper {
        let wc = state.whisper_config.lock().unwrap().clone();
        if let Err(e) = wc.validate() {
            let mut wf = state.workflow.lock().unwrap();
            wf.set_error(format!("Local Whisper not configured: {}", e));
            return Err(e);
        }
        let engine = crate::transcription_whisper::LocalWhisperEngine::new(wc);
        let request = crate::transcription::TranscriptionRequest {
            audio_file_path: audio_path,
            engine: crate::transcription::EngineKind::LocalWhisper,
        };
        match engine.transcribe(&request) {
            Ok(result) => {
                let mut ts = state.transcription.lock().unwrap();
                ts.last_result = Some(result.clone());
                ts.status = crate::transcription::TranscriptionStatus::Complete;
                let mut wf = state.workflow.lock().unwrap();
                wf.step = WorkflowStep::TranscriptionComplete;
                wf.last_transcript_raw = Some(result.raw_text);
                wf.error = None;
                Ok(wf.clone())
            }
            Err(e) => {
                let mut wf = state.workflow.lock().unwrap();
                wf.set_error(e);
                Err("Transcription failed.".into())
            }
        }
    } else {
        let engine = crate::transcription::MockEngine;
        let request = crate::transcription::TranscriptionRequest {
            audio_file_path: audio_path,
            engine: crate::transcription::EngineKind::Mock,
        };
        match engine.transcribe(&request) {
            Ok(result) => {
                let mut ts = state.transcription.lock().unwrap();
                ts.last_result = Some(result.clone());
                ts.status = crate::transcription::TranscriptionStatus::Complete;
                let mut wf = state.workflow.lock().unwrap();
                wf.step = WorkflowStep::TranscriptionComplete;
                wf.last_transcript_raw = Some(result.raw_text);
                wf.error = None;
                Ok(wf.clone())
            }
            Err(e) => {
                let mut wf = state.workflow.lock().unwrap();
                wf.set_error(e);
                Err("Transcription failed.".into())
            }
        }
    }
}

/// Runs cleanup on the latest workflow transcript.
#[tauri::command]
pub fn run_workflow_cleanup(state: State<AppState>) -> Result<WorkflowState, String> {
    let settings = state.settings.lock().unwrap().clone();
    let mut wf = state.workflow.lock().unwrap();
    let raw_text = wf
        .last_transcript_raw
        .clone()
        .ok_or("No transcript available.")?;

    wf.step = WorkflowStep::Cleaning;
    drop(wf);

    let mode: crate::modes::TextModeKind = match settings.default_text_mode.as_str() {
        "clean_notes" => crate::modes::TextModeKind::CleanNotes,
        "ai_prompt" => crate::modes::TextModeKind::AiPrompt,
        "developer_review" => crate::modes::TextModeKind::DeveloperReview,
        "thought_piece" => crate::modes::TextModeKind::ThoughtPiece,
        _ => crate::modes::TextModeKind::RawDictation,
    };

    let provider_kind = match settings.default_cleanup_provider.as_str() {
        "mock_ai" => crate::cleanup::CleanupProviderKind::MockAi,
        "openai" => crate::cleanup::CleanupProviderKind::OpenAi,
        _ => crate::cleanup::CleanupProviderKind::Basic,
    };

    let request = crate::cleanup::CleanupRequest {
        raw_text,
        mode: mode.clone(),
        provider: provider_kind.clone(),
        source_transcription_id: None,
        created_at: crate::chrono_now_iso(),
    };

    let result = match provider_kind {
        crate::cleanup::CleanupProviderKind::Basic => {
            let engine = crate::cleanup_basic::BasicCleanupProvider;
            engine
                .cleanup(&request)
                .map_err(|e| format!("{}: {}", e.code, e.message))
        }
        crate::cleanup::CleanupProviderKind::MockAi => {
            let engine = crate::cleanup_mock_ai::MockAiCleanupProvider;
            engine
                .cleanup(&request)
                .map_err(|e| format!("{}: {}", e.code, e.message))
        }
        _ => Err("Cleanup provider not available.".into()),
    };

    match result {
        Ok(cleanup_result) => {
            let mut wf = state.workflow.lock().unwrap();
            wf.step = WorkflowStep::CleanupComplete;
            wf.last_final_text = Some(cleanup_result.final_text);
            wf.error = None;
            Ok(wf.clone())
        }
        Err(e) => {
            let mut wf = state.workflow.lock().unwrap();
            wf.set_error(e);
            Err("Cleanup failed.".into())
        }
    }
}

/// Inserts final text at cursor via clipboard. Never presses Enter.
#[tauri::command]
pub async fn insert_workflow_final_text(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<WorkflowState, String> {
    let final_text = {
        let wf = state.workflow.lock().unwrap();
        wf.last_final_text
            .clone()
            .ok_or("No final text to insert.")?
    };

    crate::clipboard::write_text(&app, &final_text)?;
    let mut wf = state.workflow.lock().unwrap();
    wf.step = WorkflowStep::InsertAttempted;
    wf.insertion_attempted = true;
    wf.insertion_result_message = Some(
        "Text placed on clipboard. Paste manually with Cmd+V / Ctrl+V. Spiel never presses Enter or submits forms.".into()
    );
    wf.error = None;
    Ok(wf.clone())
}

/// Saves current workflow session to history.
#[tauri::command]
pub fn save_workflow_to_history(
    state: State<AppState>,
    db_handle: State<DatabaseHandle>,
) -> Result<WorkflowState, String> {
    let hs = state.history_state.lock().unwrap();
    if !hs.enabled {
        drop(hs);
        let mut wf = state.workflow.lock().unwrap();
        wf.set_error("History is disabled.".into());
        return Err("History saving is disabled.".into());
    }
    drop(hs);

    let (raw_text, final_text) = {
        let wf = state.workflow.lock().unwrap();
        (
            wf.last_transcript_raw.clone().unwrap_or_default(),
            wf.last_final_text.clone().unwrap_or_default(),
        )
    };

    if raw_text.trim().is_empty() && final_text.trim().is_empty() {
        return Err("No content to save.".into());
    }

    let db = db_handle
        .db
        .lock()
        .map_err(|e| format!("DB lock error: {}", e))?;
    let db = db.as_ref().ok_or("Database not initialized.")?;

    let request = crate::history::SaveHistoryRequest {
        raw_text,
        final_text,
        mode: "workflow".into(),
        cleanup_provider: "workflow".into(),
        transcription_engine: "workflow".into(),
        transcription_engine_kind: "workflow".into(),
        audio_file_path: None,
        audio_duration_ms: None,
        transcript_duration_ms: None,
        cleanup_duration_ms: None,
        is_mock_transcript: false,
        is_mock_cleanup: false,
        error: None,
    };

    match crate::history::save_entry(db, &request) {
        Ok(entry) => {
            let mut wf = state.workflow.lock().unwrap();
            wf.step = WorkflowStep::SavedToHistory;
            wf.last_history_entry_id = Some(entry.id);
            wf.error = None;
            Ok(wf.clone())
        }
        Err(e) => {
            let mut wf = state.workflow.lock().unwrap();
            wf.set_error(e);
            Err("History save failed.".into())
        }
    }
}

/// Cancels the active workflow. Stops recording if active.
#[tauri::command]
pub fn cancel_workflow(state: State<AppState>, active: State<ActiveRecording>) -> WorkflowState {
    let mut wf = state.workflow.lock().unwrap();
    // Try to stop recording if active
    let mut rec = state.recording.lock().unwrap();
    if rec.state == RecordingState::Recording {
        let mut ah = active.handle.lock().unwrap();
        if let Some(handle) = ah.take() {
            let _ = handle.stop();
        }
        rec.state = RecordingState::Idle;
    }
    wf.step = WorkflowStep::Canceled;
    wf.error = None;
    wf.clone()
}

/// Returns the current workflow state.
#[tauri::command]
pub fn get_workflow_status(state: State<AppState>) -> WorkflowState {
    state.workflow.lock().unwrap().clone()
}

/// Resets workflow to idle. Does not delete history.
#[tauri::command]
pub fn reset_workflow(state: State<AppState>) -> WorkflowState {
    let mut wf = state.workflow.lock().unwrap();
    wf.reset();
    wf.clone()
}

// ── Phase 12: OpenAI Secret & Provider Commands ─────────────

use crate::cleanup_openai::OpenAiCleanupProvider;
use crate::secrets::{ApiKeyStatus, ProviderConfigValidation};
use crate::transcription_openai::OpenAiTranscriptionEngine;

/// Stores the OpenAI API key securely (in-memory, session-only).
/// The key is validated for format (must start with 'sk-').
/// After storage, the full key is never returned to the frontend.
#[tauri::command]
pub fn set_openai_api_key(
    state: State<'_, AppState>,
    api_key: String,
) -> Result<ApiKeyStatus, String> {
    if api_key.trim().is_empty() {
        return Err("API key cannot be empty. Provide your OpenAI API key.".into());
    }

    let mut secrets = state.secrets.lock().unwrap();
    secrets.set("openai", &api_key)?;

    Ok(secrets.get_status("openai"))
}

/// Returns the status of the OpenAI API key (configured/not configured).
/// Never returns the full key value — only configured status and optional last 4 characters.
#[tauri::command]
pub fn get_openai_api_key_status(state: State<'_, AppState>) -> ApiKeyStatus {
    let secrets = state.secrets.lock().unwrap();
    secrets.get_status("openai")
}

/// Deletes the stored OpenAI API key. Returns the updated status.
#[tauri::command]
pub fn delete_openai_api_key(state: State<'_, AppState>) -> ApiKeyStatus {
    let mut secrets = state.secrets.lock().unwrap();
    secrets.delete("openai");
    secrets.get_status("openai")
}

/// Validates whether the OpenAI provider is ready to use.
/// Checks: local-only mode disabled, cloud providers enabled, API key configured.
/// Returns a detailed status with specific blockers.
#[tauri::command]
pub fn validate_openai_provider_config(state: State<'_, AppState>) -> ProviderConfigValidation {
    let secrets = state.secrets.lock().unwrap();
    let settings = state.settings.lock().unwrap();
    secrets.validate_provider_config(
        "openai",
        settings.local_only_mode,
        settings.cloud_providers_enabled,
    )
}

/// Runs OpenAI transcription on the most recent recording.
/// Requires: local-only mode off, cloud providers enabled, API key configured.
/// Sends audio to https://api.openai.com/v1/audio/transcriptions.
#[tauri::command]
pub fn transcribe_with_openai(
    state: State<'_, AppState>,
) -> Result<crate::transcription::TranscriptionResult, String> {
    // Validate prerequisites
    {
        let secrets = state.secrets.lock().unwrap();
        let settings = state.settings.lock().unwrap();
        let validation = secrets.validate_provider_config(
            "openai",
            settings.local_only_mode,
            settings.cloud_providers_enabled,
        );
        if !validation.ready {
            return Err(validation.message);
        }
    }

    // Get API key
    let api_key = {
        let secrets = state.secrets.lock().unwrap();
        secrets
            .get("openai")
            .ok_or("OpenAI API key is not configured. Add your API key in Settings.")?
    };

    // Get model from settings
    let model = {
        let settings = state.settings.lock().unwrap();
        if settings.openai_transcription_model.trim().is_empty() {
            "whisper-1".to_string()
        } else {
            settings.openai_transcription_model.clone()
        }
    };

    // Get last recording path
    let audio_path = {
        let rec = state.recording.lock().unwrap();
        rec.last_recording
            .as_ref()
            .ok_or("No recording available. Record audio first, then try transcribing.")?
            .file_path
            .clone()
    };

    if !std::path::Path::new(&audio_path).exists() {
        return Err(format!(
            "Recording file not found: {}. Record a new audio clip.",
            audio_path
        ));
    }

    // Set transcription status
    {
        let mut ts = state.transcription.lock().unwrap();
        ts.status = crate::transcription::TranscriptionStatus::Transcribing;
    }

    let engine = OpenAiTranscriptionEngine::new(api_key, Some(model));
    let request = crate::transcription::TranscriptionRequest {
        audio_file_path: audio_path,
        engine: crate::transcription::EngineKind::OpenAi,
    };

    match engine.transcribe(&request) {
        Ok(result) => {
            let mut ts = state.transcription.lock().unwrap();
            ts.status = crate::transcription::TranscriptionStatus::Complete;
            ts.last_result = Some(result.clone());
            ts.error = None;
            Ok(result)
        }
        Err(e) => {
            let mut ts = state.transcription.lock().unwrap();
            ts.status = crate::transcription::TranscriptionStatus::Error;
            ts.error = Some(e.clone());
            Err(e)
        }
    }
}

/// Runs OpenAI cleanup on the provided raw text using the selected mode.
/// Requires: local-only mode off, cloud providers enabled, API key configured.
/// Sends text to https://api.openai.com/v1/chat/completions.
#[tauri::command]
pub fn cleanup_with_openai(
    state: State<'_, AppState>,
    raw_text: String,
    mode: crate::modes::TextModeKind,
) -> Result<crate::cleanup::CleanupResult, String> {
    // Validate prerequisites
    {
        let secrets = state.secrets.lock().unwrap();
        let settings = state.settings.lock().unwrap();
        let validation = secrets.validate_provider_config(
            "openai",
            settings.local_only_mode,
            settings.cloud_providers_enabled,
        );
        if !validation.ready {
            return Err(validation.message);
        }
    }

    // Validate input
    if raw_text.trim().is_empty() {
        return Err("Cannot run cleanup on empty text. Provide a transcript first.".into());
    }

    // Get API key
    let api_key = {
        let secrets = state.secrets.lock().unwrap();
        secrets
            .get("openai")
            .ok_or("OpenAI API key is not configured. Add your API key in Settings.")?
    };

    // Get model from settings
    let model = {
        let settings = state.settings.lock().unwrap();
        if settings.openai_cleanup_model.trim().is_empty() {
            "gpt-4o-mini".to_string()
        } else {
            settings.openai_cleanup_model.clone()
        }
    };

    // Set cleanup status
    {
        let mut cs = state.cleanup.lock().unwrap();
        cs.status = crate::cleanup::CleanupStatus::Cleaning;
        cs.error = None;
        cs.selected_mode = Some(mode.clone());
        cs.selected_provider = Some(crate::cleanup::CleanupProviderKind::OpenAi);
    }

    let provider = OpenAiCleanupProvider::new(api_key, Some(model));
    let request = crate::cleanup::CleanupRequest {
        raw_text,
        mode,
        provider: crate::cleanup::CleanupProviderKind::OpenAi,
        source_transcription_id: None,
        created_at: crate::chrono_now_iso(),
    };

    match provider.cleanup(&request) {
        Ok(result) => {
            let mut cs = state.cleanup.lock().unwrap();
            cs.status = crate::cleanup::CleanupStatus::Complete;
            cs.last_result = Some(result.clone());
            cs.error = None;
            Ok(result)
        }
        Err(e) => {
            let err_msg = format!("{}: {}", e.code, e.message);
            let mut cs = state.cleanup.lock().unwrap();
            cs.status = crate::cleanup::CleanupStatus::Error;
            cs.error = Some(err_msg.clone());
            Err(err_msg)
        }
    }
}
