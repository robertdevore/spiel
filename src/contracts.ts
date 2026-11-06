export type Phase = "idle" | "recording" | "transcribing" | "inserting" | "error";

export interface StatusSnapshot {
  phase: Phase;
  message: string | null;
  needs_accessibility: boolean;
  recording_elapsed_ms: number;
  model_id: string;
  model_installed: boolean;
  accessibility_trusted: boolean;
  accessibility_supported: boolean;
}

export interface Config {
  hotkey: string;
  model: string;
  language: string;
  auto_paste: boolean;
  restore_clipboard: boolean;
  keep_model_loaded: boolean;
  transcription_threads: number;
  max_seconds: number;
}

export interface ModelView {
  id: string;
  label: string;
  approx_mb: number;
  note: string;
  installed: boolean;
  install_status: string;
  install_bytes: number;
  install_modified_ms: number | null;
  install_reason: string;
  is_current: boolean;
}

export interface TranscriptEvent {
  text: string;
  outcome: {
    pasted: boolean;
    clipboard_only: boolean;
    restored_previous: boolean;
    needs_accessibility: boolean;
  };
}

export interface PerfSample {
  wall_time_ms: number;
  capture_ms: number;
  transcribe_ms: number;
  insert_ms: number;
  total_ms: number;
  audio_samples: number;
  text_chars: number;
  outcome: string;
}

export interface PerfSnapshot {
  enabled: boolean;
  budget_ms: number;
  sample_count: number;
  average_total_ms: number;
  p50_total_ms: number;
  p95_total_ms: number;
  max_total_ms: number;
  over_budget_count: number;
  average_capture_ms: number;
  average_transcribe_ms: number;
  average_insert_ms: number;
  pasted_count: number;
  clipboard_only_count: number;
  insert_error_count: number;
  download_sample_count: number;
  average_download_ms: number;
  p95_download_ms: number;
  max_download_ms: number;
  last: PerfSample | null;
  last_download: {
    wall_time_ms: number;
    total_ms: number;
    downloaded_bytes: number;
    expected_bytes: number | null;
    outcome: string;
  } | null;
}

export interface MemorySnapshot {
  rss_bytes: number;
  source: string;
}

export interface PerfEvent {
  capture_ms: number;
  transcribe_ms: number;
  insert_ms: number;
  total_ms: number;
  text_chars: number;
  outcome: string;
}

export interface ReadinessSnapshot {
  model_dir: string;
  model_dir_writable: boolean;
  config_file: string;
  config_writable: boolean;
  config_path_safe: boolean;
  model_dir_safe: boolean;
  current_model: string;
  current_model_installed: boolean;
  current_model_status: string;
  current_model_reason: string;
  model_store_bytes: number;
  model_store_file_count: number;
  hotkey_valid: boolean;
  accessibility_supported: boolean;
  accessibility_trusted: boolean;
  active_download: boolean;
  recommended_model: string;
  recommended_model_reason: string;
  setup_steps_remaining: number;
}

export interface StartupHealthSnapshot {
  checked_at_ms: number;
  config_file: string;
  config_path_safe: boolean;
  config_writable: boolean;
  model_dir: string;
  model_dir_safe: boolean;
  model_dir_writable: boolean;
  current_model: string;
  current_model_status: string;
  current_model_reason: string;
  hotkey_valid: boolean;
  accessibility_supported: boolean;
  accessibility_trusted: boolean;
  recommended_model: string;
  recommended_model_reason: string;
  removed_partial_files: number;
  removed_sidecar_files: number;
  startup_warnings: string[];
}

export interface ModelProgressEvent {
  model_id: string;
  downloaded: number;
  total: number | null;
}

export interface ModelDoneEvent {
  model_id: string;
  ok: boolean;
  error: string | null;
  outcome: string;
  downloaded_bytes: number;
  expected_bytes: number | null;
  checksum_source: string;
}
