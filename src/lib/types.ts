/** Capability implementation status from the Rust backend */
export interface CapabilityStatus {
  name: string;
  status: "implemented" | "planned" | "placeholder";
}

/** Response from get_app_status command */
export interface AppStatusResponse {
  app_name: string;
  version: string;
  phase: string;
  capabilities: CapabilityStatus[];
}

/** Response from get_app_info command */
export interface AppInfoResponse {
  name: string;
  tagline: string;
  description: string;
  phase: string;
  privacy_note: string;
}

/** Response from echo_preview_text command */
export interface EchoResponse {
  original: string;
  echoed: string;
  char_count: number;
}

/** Hotkey registration and trigger status from the Rust backend */
export interface HotkeyStatus {
  shortcut: string;
  registered: boolean;
  error: string | null;
  last_triggered: string | null;
  trigger_count: number;
}

/** UI-level app states for the capture flow */
export type AppFlowState = "idle" | "recording" | "processing" | "complete" | "error";

/** Recording state from the Rust backend */
export type RecordingStateType = "idle" | "recording" | "stopping" | "complete" | "error";

/** Last recording metadata */
export interface LastRecording {
  file_path: string;
  filename: string;
  duration_ms: number;
  sample_rate: number;
  channels: number;
  size_bytes: number;
  created_at: string;
  device_name: string | null;
}

/** Response from get_recording_status / start_recording / stop_recording */
export interface RecordingStatus {
  state: RecordingStateType;
  elapsed_ms: number;
  started_at: string | null;
  last_recording: LastRecording | null;
  error: string | null;
}

/** Supported text processing modes (Phase 7 — all implemented) */
export type TextMode =
  | "raw_dictation"
  | "clean_notes"
  | "ai_prompt"
  | "developer_review"
  | "thought_piece";

/** Hotkey behavior mode */
export type HotkeyBehavior = "toggle" | "hold_to_talk";

/** Result of a clipboard insertion operation */
export interface InsertResult {
  success: boolean;
  copied: boolean;
  paste_attempted: boolean;
  paste_succeeded: boolean;
  clipboard_restored: boolean;
  previous_clipboard_had_text: boolean;
  error: string | null;
  warning: string | null;
}

/** Transcription engine kinds */
export type EngineKind = "mock" | "local_whisper" | "cloud";

/** Engine info from the backend */
export interface EngineInfo {
  kind: EngineKind;
  label: string;
  implemented: boolean;
}

/** Transcription status */
export type TranscriptionStatusType = "idle" | "pending" | "transcribing" | "complete" | "error";

/** Transcription result from the backend */
export interface TranscriptionResult {
  raw_text: string;
  engine_name: string;
  engine_kind: EngineKind;
  audio_file_path: string;
  duration_ms: number;
  is_mock: boolean;
  created_at: string;
  error: string | null;
}

/** Transcription state from the backend */
export interface TranscriptionStateData {
  status: TranscriptionStatusType;
  last_result: TranscriptionResult | null;
  error: string | null;
  available_engines: EngineInfo[];
}

/** Local Whisper configuration */
export interface WhisperConfig {
  binary_path: string;
  model_path: string;
  language: string | null;
}

// ── Phase 7: Cleanup Types ─────────────────────────────────

/** Text mode kinds */
export type TextModeKind =
  | "raw_dictation"
  | "clean_notes"
  | "ai_prompt"
  | "developer_review"
  | "thought_piece";

/** Cleanup provider kinds */
export type CleanupProviderKind =
  | "basic"
  | "mock_ai"
  | "openai_planned"
  | "local_llm_planned";

/** Cleanup status */
export type CleanupStatusType =
  | "idle"
  | "waiting_for_transcript"
  | "cleaning"
  | "complete"
  | "error"
  | "canceled";

/** Display-friendly mode definition from backend */
export interface ModeDefinition {
  kind: TextModeKind;
  label: string;
  description: string;
  implemented: boolean;
}

/** Cleanup provider info from backend */
export interface CleanupProviderInfo {
  kind: CleanupProviderKind;
  label: string;
  description: string;
  implemented: boolean;
}

/** Cleanup error from backend */
export interface CleanupError {
  code: string;
  message: string;
  recoverable: boolean;
  details: string | null;
}

/** Cleanup result from backend */
export interface CleanupResult {
  raw_text: string;
  final_text: string;
  mode: TextModeKind;
  provider: CleanupProviderKind;
  is_mock: boolean;
  changed: boolean;
  created_at: string;
  completed_at: string;
  duration_ms: number;
  warnings: string[];
  error: CleanupError | null;
}

/** Cleanup state from backend */
export interface CleanupStateData {
  status: CleanupStatusType;
  last_result: CleanupResult | null;
  error: string | null;
  selected_mode: TextModeKind | null;
  selected_provider: CleanupProviderKind | null;
}

// ── Phase 8: History Types ─────────────────────────────────

/** A saved history entry */
export interface HistoryEntry {
  id: number;
  created_at: string;
  updated_at: string;
  title: string;
  raw_text: string;
  final_text: string;
  mode: string;
  cleanup_provider: string;
  transcription_engine: string;
  transcription_engine_kind: string;
  audio_file_path: string | null;
  audio_duration_ms: number | null;
  transcript_duration_ms: number | null;
  cleanup_duration_ms: number | null;
  is_mock_transcript: boolean;
  is_mock_cleanup: boolean;
  error: string | null;
}

/** Request to save a history entry */
export interface SaveHistoryRequest {
  raw_text: string;
  final_text: string;
  mode: string;
  cleanup_provider: string;
  transcription_engine: string;
  transcription_engine_kind: string;
  audio_file_path: string | null;
  audio_duration_ms: number | null;
  transcript_duration_ms: number | null;
  cleanup_duration_ms: number | null;
  is_mock_transcript: boolean;
  is_mock_cleanup: boolean;
  error: string | null;
}

/** History state from backend */
export interface HistoryStateData {
  enabled: boolean;
  entry_count: number;
  error: string | null;
}

// ── Phase 9: Settings Types ─────────────────────────────────

/** Application settings */
export interface SpielSettings {
  hotkey: string;
  history_enabled: boolean;
  clipboard_restore_enabled: boolean;
  default_transcription_engine: string;
  default_text_mode: string;
  default_cleanup_provider: string;
  local_whisper_binary_path: string;
  local_whisper_model_path: string;
  local_only_mode: boolean;
  debug_logging_enabled: boolean;
  created_at: string;
  updated_at: string;
}

/** Request to update settings */
export interface UpdateSettingsRequest {
  hotkey?: string | null;
  history_enabled?: boolean | null;
  clipboard_restore_enabled?: boolean | null;
  default_transcription_engine?: string | null;
  default_text_mode?: string | null;
  default_cleanup_provider?: string | null;
  local_whisper_binary_path?: string | null;
  local_whisper_model_path?: string | null;
  local_only_mode?: boolean | null;
  debug_logging_enabled?: boolean | null;
}

/** Privacy status summary */
export interface PrivacyStatus {
  local_only_mode: boolean;
  history_enabled: boolean;
  clipboard_restore_enabled: boolean;
  debug_logging_enabled: boolean;
  cloud_available: boolean;
  openai_available: boolean;
  local_llm_available: boolean;
  history_encrypted: boolean;
  clipboard_contents_stored: boolean;
  api_keys_stored: boolean;
  network_calls_possible: boolean;
}

// ── Phase 10: Workflow Types ────────────────────────────────

export type WorkflowStep =
  | "idle"
  | "recording"
  | "recording_stopping"
  | "recording_complete"
  | "transcribing"
  | "transcription_complete"
  | "cleaning"
  | "cleanup_complete"
  | "inserting"
  | "insert_attempted"
  | "saved_to_history"
  | "canceled"
  | "error";

export interface WorkflowStateData {
  step: WorkflowStep;
  error: string | null;
  last_recording_filename: string | null;
  last_recording_duration_ms: number | null;
  last_transcript_raw: string | null;
  last_final_text: string | null;
  last_history_entry_id: number | null;
  insertion_attempted: boolean;
  insertion_result_message: string | null;
}
