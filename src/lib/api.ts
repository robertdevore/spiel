import { invoke } from "@tauri-apps/api/core";
import type { AppStatusResponse, AppInfoResponse, EchoResponse, HotkeyStatus, RecordingStatus, InsertResult, TranscriptionStateData, TranscriptionResult, EngineInfo, WhisperConfig, ModeDefinition, CleanupProviderInfo, CleanupResult, CleanupStateData, TextModeKind, CleanupProviderKind, HistoryEntry, SaveHistoryRequest, HistoryStateData, SpielSettings, UpdateSettingsRequest, PrivacyStatus } from "./types";

export async function getAppStatus(): Promise<AppStatusResponse> {
  return invoke<AppStatusResponse>("get_app_status");
}

export async function getAppInfo(): Promise<AppInfoResponse> {
  return invoke<AppInfoResponse>("get_app_info");
}

export async function echoPreviewText(text: string): Promise<EchoResponse> {
  return invoke<EchoResponse>("echo_preview_text", { text });
}

export async function getHotkeyStatus(): Promise<HotkeyStatus> {
  return invoke<HotkeyStatus>("get_hotkey_status");
}

/** Start microphone recording. Returns updated status. */
export async function startRecording(): Promise<RecordingStatus> {
  return invoke<RecordingStatus>("start_recording");
}

/** Stop microphone recording and finalize WAV file. */
export async function stopRecording(): Promise<RecordingStatus> {
  return invoke<RecordingStatus>("stop_recording");
}

/** Get current recording state and metadata. */
export async function getRecordingStatus(): Promise<RecordingStatus> {
  return invoke<RecordingStatus>("get_recording_status");
}

/** Clear last recording metadata from state. */
export async function clearLastRecording(): Promise<RecordingStatus> {
  return invoke<RecordingStatus>("clear_last_recording");
}

/** Copy text to clipboard. */
export async function copyToClipboard(text: string): Promise<InsertResult> {
  return invoke<InsertResult>("copy_to_clipboard", { text });
}

/** Save clipboard, write text, prompt manual paste. */
export async function insertViaClipboard(text: string, restore: boolean): Promise<InsertResult> {
  return invoke<InsertResult>("insert_via_clipboard", { text, restore });
}

/** Attempt to restore previous clipboard contents. */
export async function restoreClipboard(): Promise<InsertResult> {
  return invoke<InsertResult>("restore_clipboard");
}

/** Read current clipboard text. */
export async function getClipboardText(): Promise<string> {
  return invoke<string>("get_clipboard_text");
}

/** Get current transcription state and last result. */
export async function getTranscriptionStatus(): Promise<TranscriptionStateData> {
  return invoke<TranscriptionStateData>("get_transcription_status");
}

/** Run mock transcription on the latest recording. */
export async function transcribeLastRecordingMock(): Promise<TranscriptionResult> {
  return invoke<TranscriptionResult>("transcribe_last_recording_mock");
}

/** Get list of available transcription engines. */
export async function getAvailableEngines(): Promise<EngineInfo[]> {
  return invoke<EngineInfo[]>("get_available_transcription_engines");
}

/** Clear current transcript from state. */
export async function clearTranscript(): Promise<TranscriptionStateData> {
  return invoke<TranscriptionStateData>("clear_transcript");
}

/** Validate local Whisper binary and model paths. */
export async function validateWhisperConfig(): Promise<string> {
  return invoke<string>("validate_local_whisper_config");
}

/** Get current Whisper configuration. */
export async function getWhisperConfig(): Promise<WhisperConfig> {
  return invoke<WhisperConfig>("get_whisper_config");
}

/** Update Whisper configuration paths. */
export async function updateWhisperConfig(
  binaryPath: string, modelPath: string, language: string | null
): Promise<WhisperConfig> {
  return invoke<WhisperConfig>("update_whisper_config", {
    binaryPath, modelPath, language,
  });
}

/** Run local Whisper transcription on the latest recording. */
export async function transcribeLastRecordingLocal(): Promise<TranscriptionResult> {
  return invoke<TranscriptionResult>("transcribe_last_recording_local");
}

// ── Phase 7: Cleanup Commands ──────────────────────────────

/** Get available text mode definitions. */
export async function getTextModes(): Promise<ModeDefinition[]> {
  return invoke<ModeDefinition[]>("get_text_modes");
}

/** Get available cleanup providers with implementation status. */
export async function getCleanupProviders(): Promise<CleanupProviderInfo[]> {
  return invoke<CleanupProviderInfo[]>("get_cleanup_providers");
}

/** Run cleanup on raw text with the specified mode and provider. */
export async function runCleanup(
  rawText: string, mode: TextModeKind, provider: CleanupProviderKind
): Promise<CleanupResult> {
  return invoke<CleanupResult>("run_cleanup", { rawText, mode, provider });
}

/** Clear final text / cleanup result from state. */
export async function clearFinalText(): Promise<CleanupStateData> {
  return invoke<CleanupStateData>("clear_final_text");
}

/** Get current cleanup status and last result. */
export async function getCleanupStatus(): Promise<CleanupStateData> {
  return invoke<CleanupStateData>("get_cleanup_status");
}

// ── Phase 8: History Commands ──────────────────────────────

/** Save a session to local history. */
export async function saveHistoryEntry(request: SaveHistoryRequest): Promise<HistoryEntry> {
  return invoke<HistoryEntry>("save_history_entry", { request });
}

/** List recent history entries. */
export async function listHistoryEntries(limit?: number): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>("list_history_entries", { limit: limit ?? null });
}

/** Get a single history entry by id. */
export async function getHistoryEntry(id: number): Promise<HistoryEntry> {
  return invoke<HistoryEntry>("get_history_entry", { id });
}

/** Delete a history entry by id. */
export async function deleteHistoryEntry(id: number): Promise<void> {
  return invoke<void>("delete_history_entry", { id });
}

/** Delete all history entries. */
export async function clearHistory(): Promise<number> {
  return invoke<number>("clear_history");
}

/** Get history status (enabled, count). */
export async function getHistoryStatus(): Promise<HistoryStateData> {
  return invoke<HistoryStateData>("get_history_status");
}

/** Enable or disable history saving. */
export async function setHistoryEnabled(enabled: boolean): Promise<HistoryStateData> {
  return invoke<HistoryStateData>("set_history_enabled", { enabled });
}

// ── Phase 9: Settings Commands ──────────────────────────────

/** Load persisted settings. */
export async function getSettings(): Promise<SpielSettings> {
  return invoke<SpielSettings>("get_settings");
}

/** Update settings (only provided fields are changed). */
export async function updateSettings(update: UpdateSettingsRequest): Promise<SpielSettings> {
  return invoke<SpielSettings>("update_settings", { update });
}

/** Reset settings to safe defaults. */
export async function resetSettings(): Promise<SpielSettings> {
  return invoke<SpielSettings>("reset_settings");
}

/** Get privacy status summary. */
export async function getPrivacyStatus(): Promise<PrivacyStatus> {
  return invoke<PrivacyStatus>("get_privacy_status");
}
