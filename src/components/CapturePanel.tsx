import { useState, useCallback, useEffect } from "react";
import type { AppFlowState, HotkeyStatus, RecordingStatus, RecordingStateType, InsertResult, TranscriptionStateData, WhisperConfig, CleanupStateData, ModeDefinition, CleanupProviderInfo, TextModeKind, CleanupProviderKind } from "../lib/types";
import { normalizeShortcutLabel, formatLastTriggered, formatTriggerCount } from "../lib/hotkeys";
import { startRecording, stopRecording, getRecordingStatus, clearLastRecording, copyToClipboard, insertViaClipboard, transcribeLastRecordingMock, clearTranscript, validateWhisperConfig, getWhisperConfig, updateWhisperConfig, transcribeLastRecordingLocal, runCleanup, clearFinalText, saveHistoryEntry } from "../lib/api";

interface CapturePanelProps {
  flowState: AppFlowState;
  onStateChange: (state: AppFlowState) => void;
  hotkeyStatus: HotkeyStatus | null;
  recordingStatus: RecordingStatus | null;
  insertionText: string;
  onInsertionTextChange: (text: string) => void;
  lastInsertResult: InsertResult | null;
  onInsertResult: (result: InsertResult | null) => void;
  transcriptionState: TranscriptionStateData | null;
  onTranscriptionStateChange: (state: TranscriptionStateData | null) => void;
  cleanupState: CleanupStateData | null;
  onCleanupStateChange: (state: CleanupStateData | null) => void;
  modes: ModeDefinition[];
  providers: CleanupProviderInfo[];
}

/** Map Rust recording state to UI flow state */
function recordingStateToFlow(rs: RecordingStateType): AppFlowState {
  switch (rs) {
    case "recording": return "recording";
    case "stopping": return "processing";
    case "complete": return "complete";
    case "error": return "error";
    default: return "idle";
  }
}

/** Format milliseconds as MM:SS */
function formatElapsed(ms: number): string {
  const totalSec = Math.floor(ms / 1000);
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  return `${String(min).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
}

/** Format bytes to human-readable size */
function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1048576).toFixed(1)} MB`;
}

export default function CapturePanel({
  flowState,
  onStateChange,
  hotkeyStatus,
  recordingStatus,
  insertionText,
  onInsertionTextChange,
  lastInsertResult,
  onInsertResult,
  transcriptionState,
  onTranscriptionStateChange,
  cleanupState,
  onCleanupStateChange,
  modes,
  providers,
}: CapturePanelProps) {
  const [actionError, setActionError] = useState<string | null>(null);
  const [isBusy, setIsBusy] = useState(false);
  const [restoreClipboard, setRestoreClipboard] = useState(true);
  const [transcribeError, setTranscribeError] = useState<string | null>(null);
  const [whisperConfig, setWhisperConfig] = useState<WhisperConfig>({ binary_path: "", model_path: "", language: null });
  const [configValidMsg, setConfigValidMsg] = useState<string | null>(null);
  const [cleanupError, setCleanupError] = useState<string | null>(null);
  const [selectedCleanupMode, setSelectedCleanupMode] = useState<TextModeKind>("raw_dictation");
  const [selectedCleanupProvider, setSelectedCleanupProvider] = useState<CleanupProviderKind>("basic");

  const rs = recordingStatus;
  const isRecording = rs?.state === "recording";
  const isComplete = rs?.state === "complete";
  const hasError = rs?.state === "error";
  const lastRec = rs?.last_recording ?? null;

  // Keep parent flowState in sync with recording status
  useEffect(() => {
    if (rs) {
      onStateChange(recordingStateToFlow(rs.state));
    }
  }, [rs?.state]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleStart = useCallback(async () => {
    setActionError(null);
    setIsBusy(true);
    try {
      await startRecording();
    } catch (e) {
      setActionError(String(e));
    } finally {
      setIsBusy(false);
    }
  }, []);

  const handleStop = useCallback(async () => {
    setActionError(null);
    setIsBusy(true);
    try {
      await stopRecording();
    } catch (e) {
      setActionError(String(e));
    } finally {
      setIsBusy(false);
    }
  }, []);

  const handleClear = useCallback(async () => {
    try {
      await clearLastRecording();
      // Poll once to refresh status
      const status = await getRecordingStatus();
      onStateChange(recordingStateToFlow(status.state));
    } catch (e) {
      setActionError(String(e));
    }
  }, [onStateChange]);

  const handleCopy = useCallback(async () => {
    setActionError(null);
    onInsertResult(null);
    setIsBusy(true);
    try {
      const result = await copyToClipboard(insertionText);
      onInsertResult(result);
    } catch (e) {
      setActionError(String(e));
    } finally {
      setIsBusy(false);
    }
  }, [insertionText, onInsertResult]);

  const handleInsert = useCallback(async () => {
    setActionError(null);
    onInsertResult(null);
    setIsBusy(true);
    try {
      const result = await insertViaClipboard(insertionText, restoreClipboard);
      onInsertResult(result);
    } catch (e) {
      setActionError(String(e));
    } finally {
      setIsBusy(false);
    }
  }, [insertionText, restoreClipboard, onInsertResult]);

  const handleTranscribe = useCallback(async () => {
    setTranscribeError(null);
    setIsBusy(true);
    try {
      const result = await transcribeLastRecordingMock();
      onTranscriptionStateChange({
        status: "complete",
        last_result: result,
        error: null,
        available_engines: transcriptionState?.available_engines ?? [],
      });
    } catch (e) {
      setTranscribeError(String(e));
    } finally {
      setIsBusy(false);
    }
  }, [onTranscriptionStateChange, transcriptionState?.available_engines]);

  const handleClearTranscript = useCallback(async () => {
    try {
      const state = await clearTranscript();
      onTranscriptionStateChange(state);
      setTranscribeError(null);
    } catch (e) {
      setTranscribeError(String(e));
    }
  }, [onTranscriptionStateChange]);

  const hasRecording = recordingStatus?.last_recording != null;
  const ts = transcriptionState;
  const hasTranscript = ts?.last_result != null;
  const isTranscribing = ts?.status === "transcribing";

  // Load whisper config on mount
  useEffect(() => {
    getWhisperConfig().then(setWhisperConfig).catch(() => {});
  }, []);

  const handleValidateConfig = useCallback(async () => {
    setConfigValidMsg(null);
    try {
      const msg = await validateWhisperConfig();
      setConfigValidMsg(msg);
      setTranscribeError(null);
    } catch (e) {
      setConfigValidMsg(null);
      setTranscribeError(String(e));
    }
  }, []);

  const handleUpdateConfig = useCallback(async (field: keyof WhisperConfig, value: string) => {
    const updated = { ...whisperConfig, [field]: value };
    setWhisperConfig(updated);
    setConfigValidMsg(null);
    try {
      const result = await updateWhisperConfig(
        updated.binary_path,
        updated.model_path,
        updated.language || null,
      );
      setWhisperConfig(result);
    } catch (e) {
      setTranscribeError(String(e));
    }
  }, [whisperConfig]);

  const handleTranscribeLocal = useCallback(async () => {
    setTranscribeError(null);
    setIsBusy(true);
    try {
      const result = await transcribeLastRecordingLocal();
      onTranscriptionStateChange({
        status: "complete",
        last_result: result,
        error: null,
        available_engines: transcriptionState?.available_engines ?? [],
      });
    } catch (e) {
      setTranscribeError(String(e));
    } finally {
      setIsBusy(false);
    }
  }, [onTranscriptionStateChange, transcriptionState?.available_engines]);

  // ── Cleanup handlers (Phase 7) ──────────────────────────

  const handleRunCleanup = useCallback(async () => {
    setCleanupError(null);
    const rawText = transcriptionState?.last_result?.raw_text ?? insertionText;
    if (!rawText.trim()) {
      setCleanupError("No transcript available. Transcribe first, or type/paste raw text in the Insertion Text field above.");
      return;
    }
    setIsBusy(true);
    try {
      const result = await runCleanup(rawText, selectedCleanupMode, selectedCleanupProvider);
      onCleanupStateChange({
        status: "complete",
        last_result: result,
        error: null,
        selected_mode: selectedCleanupMode,
        selected_provider: selectedCleanupProvider,
      });
    } catch (e) {
      setCleanupError(String(e));
      onCleanupStateChange({
        status: "error",
        last_result: cleanupState?.last_result ?? null,
        error: String(e),
        selected_mode: selectedCleanupMode,
        selected_provider: selectedCleanupProvider,
      });
    } finally {
      setIsBusy(false);
    }
  }, [transcriptionState, insertionText, selectedCleanupMode, selectedCleanupProvider, onCleanupStateChange, cleanupState]);

  const handleCopyFinal = useCallback(async () => {
    const finalText = cleanupState?.last_result?.final_text;
    if (!finalText?.trim()) return;
    setActionError(null);
    try {
      const result = await copyToClipboard(finalText);
      onInsertResult(result);
    } catch (e) {
      setActionError(String(e));
    }
  }, [cleanupState, onInsertResult]);

  const handleInsertFinal = useCallback(async () => {
    const finalText = cleanupState?.last_result?.final_text;
    if (!finalText?.trim()) return;
    setActionError(null);
    try {
      const result = await insertViaClipboard(finalText, restoreClipboard);
      onInsertResult(result);
    } catch (e) {
      setActionError(String(e));
    }
  }, [cleanupState, restoreClipboard, onInsertResult]);

  const handleClearFinal = useCallback(async () => {
    try {
      const state = await clearFinalText();
      onCleanupStateChange(state);
      setCleanupError(null);
    } catch (e) {
      setCleanupError(String(e));
    }
  }, [onCleanupStateChange]);

  // ── History handler (Phase 8) ──────────────────────────

  const handleSaveToHistory = useCallback(async () => {
    const rawText = transcriptionState?.last_result?.raw_text ?? "";
    const finalText = cleanupState?.last_result?.final_text ?? "";
    if (!rawText.trim() && !finalText.trim()) {
      setCleanupError("No content to save. Transcribe and run cleanup first.");
      return;
    }
    setCleanupError(null);
    setIsBusy(true);
    try {
      await saveHistoryEntry({
        raw_text: rawText,
        final_text: finalText,
        mode: cleanupState?.last_result?.mode ?? "",
        cleanup_provider: cleanupState?.last_result?.provider ?? "",
        transcription_engine: transcriptionState?.last_result?.engine_name ?? "",
        transcription_engine_kind: transcriptionState?.last_result?.engine_kind ?? "",
        audio_file_path: recordingStatus?.last_recording?.file_path ?? null,
        audio_duration_ms: recordingStatus?.last_recording?.duration_ms ?? null,
        transcript_duration_ms: transcriptionState?.last_result?.duration_ms ?? null,
        cleanup_duration_ms: cleanupState?.last_result?.duration_ms ?? null,
        is_mock_transcript: transcriptionState?.last_result?.is_mock ?? false,
        is_mock_cleanup: cleanupState?.last_result?.is_mock ?? false,
        error: null,
      });
    } catch (e) {
      setCleanupError(String(e));
    } finally {
      setIsBusy(false);
    }
  }, [transcriptionState, cleanupState, recordingStatus]);

  const clResult = cleanupState?.last_result;
  const hasFinalText = clResult != null && clResult.final_text.trim().length > 0;

  const shortcutLabel = hotkeyStatus
    ? normalizeShortcutLabel(hotkeyStatus.shortcut)
    : "Cmd+Option+.";

  const isRegistered = hotkeyStatus?.registered ?? false;

  const stateLabels: Record<AppFlowState, string> = {
    idle: "Ready — press Record to start",
    recording: "Recording...",
    processing: "Processing...",
    complete: "Recording complete",
    error: "Recording error",
  };

  const stateColors: Record<AppFlowState, string> = {
    idle: "#4a9eff",
    recording: "#ff4a4a",
    processing: "#ffaa00",
    complete: "#4aff88",
    error: "#ff4a4a",
  };

  return (
    <section className="capture-panel">
      {/* Status row */}
      <div className="status-row">
        <span className="status-dot" style={{ background: stateColors[flowState] }} />
        <span className="status-label">{stateLabels[flowState]}</span>
        {isRecording && rs && (
          <span className="elapsed-timer">{formatElapsed(rs.elapsed_ms)}</span>
        )}
      </div>

      {/* Hotkey registration status */}
      <div className="hotkey-status-section">
        <div className="hotkey-status-row">
          <span className={`hotkey-status-dot ${isRegistered ? "registered" : "error"}`} />
          <span className="hotkey-status-text">
            Global hotkey: {isRegistered ? "registered" : "not registered"}
          </span>
        </div>
        <div className="hotkey-shortcut-display">
          <kbd>{shortcutLabel}</kbd>
          <span className="hint-text">
            Press {shortcutLabel} anywhere to start/stop recording
          </span>
        </div>
        {hotkeyStatus?.error && (
          <div className="hotkey-error"><p>{hotkeyStatus.error}</p></div>
        )}
        {isRegistered && hotkeyStatus && (
          <div className="hotkey-stats">
            <span className="hotkey-stat">{formatTriggerCount(hotkeyStatus.trigger_count)}</span>
            <span className="hotkey-stat">Last triggered: {formatLastTriggered(hotkeyStatus.last_triggered)}</span>
          </div>
        )}
      </div>

      {/* Recording controls */}
      <div className="capture-controls">
        {!isRecording ? (
          <button
            className="btn-record"
            onClick={handleStart}
            disabled={isBusy || isRecording}
          >
            🎤 Start Recording
          </button>
        ) : (
          <button
            className="btn-record is-recording"
            onClick={handleStop}
            disabled={isBusy}
          >
            ⏹ Stop Recording
          </button>
        )}
      </div>

      {/* Action error */}
      {actionError && (
        <div className="recording-error">
          <p>{actionError}</p>
        </div>
      )}

      {/* Recording state error from backend */}
      {hasError && rs?.error && (
        <div className="recording-error">
          <p>{rs.error}</p>
        </div>
      )}

      {/* Last recording metadata */}
      {isComplete && lastRec && (
        <div className="last-recording-card">
          <h4>Last Recording</h4>
          <div className="recording-meta-grid">
            <div className="meta-item">
              <span className="meta-label">File</span>
              <span className="meta-value" title={lastRec.file_path}>{lastRec.filename}</span>
            </div>
            <div className="meta-item">
              <span className="meta-label">Duration</span>
              <span className="meta-value">{formatElapsed(lastRec.duration_ms)}</span>
            </div>
            <div className="meta-item">
              <span className="meta-label">Size</span>
              <span className="meta-value">{formatSize(lastRec.size_bytes)}</span>
            </div>
            <div className="meta-item">
              <span className="meta-label">Quality</span>
              <span className="meta-value">{lastRec.sample_rate} Hz, {lastRec.channels}ch</span>
            </div>
            {lastRec.device_name && (
              <div className="meta-item full-width">
                <span className="meta-label">Device</span>
                <span className="meta-value">{lastRec.device_name}</span>
              </div>
            )}
          </div>
          <button className="btn-clear" onClick={handleClear}>
            Clear
          </button>
        </div>
      )}

      {/* ── Insertion Section (Phase 4) ──────────────────── */}
      <div className="insertion-section">
        <h4 className="insertion-title">Insertion Text</h4>
        <textarea
          className="insertion-textarea"
          value={insertionText}
          onChange={(e) => onInsertionTextChange(e.target.value)}
          placeholder="Type or paste text to copy/insert. Transcription will fill this automatically in a future phase."
          rows={4}
        />
        <div className="insertion-controls">
          <button
            className="btn-insert"
            onClick={handleCopy}
            disabled={isBusy || !insertionText.trim()}
          >
            📋 Copy
          </button>
          <button
            className="btn-insert primary"
            onClick={handleInsert}
            disabled={isBusy || !insertionText.trim()}
          >
            ↩️ Insert at Cursor
          </button>
        </div>
        <label className="insertion-toggle">
          <input
            type="checkbox"
            checked={restoreClipboard}
            onChange={(e) => setRestoreClipboard(e.target.checked)}
          />
          Restore previous clipboard after paste
        </label>

        {/* Insertion result */}
        {lastInsertResult && (
          <div className={`insert-result ${lastInsertResult.success ? "success" : "error"}`}>
            {lastInsertResult.warning && <p>{lastInsertResult.warning}</p>}
            {lastInsertResult.error && <p className="insert-error-text">{lastInsertResult.error}</p>}
            {lastInsertResult.copied && !lastInsertResult.warning && (
              <p>✓ Text copied to clipboard. Paste with Cmd+V / Ctrl+V.</p>
            )}
          </div>
        )}
      </div>

      {/* ── Transcription Section (Phase 5) ───────────────── */}
      <div className="transcription-section">
        <h4 className="insertion-title">Transcription</h4>
        <div className="transcription-engines">
          {ts?.available_engines?.map((eng) => (
            <span key={eng.kind} className={`engine-badge ${eng.implemented ? "implemented" : "planned"}`}>
              {eng.label}
            </span>
          )) ?? <span className="engine-badge planned">Loading...</span>}
        </div>

        {/* Whisper config */}
        <div className="whisper-config">
          <label className="config-label">Binary Path</label>
          <input
            className="config-input"
            type="text"
            value={whisperConfig.binary_path}
            onChange={(e) => handleUpdateConfig("binary_path", e.target.value)}
            placeholder="/usr/local/bin/whisper-cpp"
          />
          <label className="config-label">Model Path</label>
          <input
            className="config-input"
            type="text"
            value={whisperConfig.model_path}
            onChange={(e) => handleUpdateConfig("model_path", e.target.value)}
            placeholder="/path/to/ggml-base.en.bin"
          />
          <div className="config-row">
            <button className="btn-insert" onClick={handleValidateConfig} disabled={isBusy}>
              Validate Config
            </button>
          </div>
          {configValidMsg && <p className="config-valid">{configValidMsg}</p>}
        </div>

        {/* Transcribe buttons */}
        <div style={{ display: "flex", gap: 6, marginTop: 8 }}>
          <button
            className="btn-insert"
            onClick={handleTranscribe}
            disabled={isBusy || !hasRecording || isTranscribing}
            style={{ flex: 1 }}
          >
            Mock
          </button>
          <button
            className="btn-insert primary"
            onClick={handleTranscribeLocal}
            disabled={isBusy || !hasRecording || isTranscribing || !whisperConfig.binary_path}
            style={{ flex: 1 }}
          >
            {isTranscribing ? "⏳ ..." : "Whisper"}
          </button>
        </div>
        {!hasRecording && <p className="transcription-hint">Record audio first, then transcribe.</p>}
        {transcribeError && <div className="recording-error"><p>{transcribeError}</p></div>}
        {ts?.error && <div className="recording-error"><p>{ts.error}</p></div>}
        {hasTranscript && ts.last_result && (
          <div className="transcript-result">
            <div className="transcript-header">
              <span className="transcript-engine">
                {ts.last_result.engine_name}
                {ts.last_result.is_mock && <span className="badge-planned" style={{ marginLeft: 6 }}>mock</span>}
              </span>
              <span className="transcript-meta">
                {ts.last_result.duration_ms > 0 ? `~${Math.round(ts.last_result.duration_ms / 1000)}s` : ""}{" • "}{ts.last_result.audio_file_path.split("/").pop()}
              </span>
            </div>
            <pre className="transcript-text">{ts.last_result.raw_text}</pre>
            <button className="btn-clear" onClick={handleClearTranscript} style={{ marginTop: 6 }}>Clear Transcript</button>
          </div>
        )}
        <p className="transcription-notice">
          Local Whisper runs on your machine — configure binary/model paths above.
          Mock engine always available. Cloud transcription is planned for a future phase.
          No audio is uploaded.
        </p>
      </div>

      {/* ── Cleanup Section (Phase 7) ────────────────────── */}
      <div className="cleanup-section">
        <h4 className="insertion-title">Text Cleanup</h4>

        {/* Mode selector */}
        <div className="cleanup-select-row">
          <label className="config-label">Mode</label>
          <select
            className="config-input"
            value={selectedCleanupMode}
            onChange={(e) => setSelectedCleanupMode(e.target.value as TextModeKind)}
          >
            {modes.length > 0 ? modes.map((m) => (
              <option key={m.kind} value={m.kind}>{m.label}</option>
            )) : (
              <>
                <option value="raw_dictation">Raw Dictation</option>
                <option value="clean_notes">Clean Notes</option>
                <option value="ai_prompt">AI Prompt</option>
                <option value="developer_review">Developer Review</option>
                <option value="thought_piece">Thought Piece</option>
              </>
            )}
          </select>
        </div>

        {/* Provider selector */}
        <div className="cleanup-select-row">
          <label className="config-label">Provider</label>
          <select
            className="config-input"
            value={selectedCleanupProvider}
            onChange={(e) => setSelectedCleanupProvider(e.target.value as CleanupProviderKind)}
          >
            {providers.length > 0 ? providers.map((p) => (
              <option key={p.kind} value={p.kind} disabled={!p.implemented}>
                {p.label}
              </option>
            )) : (
              <>
                <option value="basic">Basic (deterministic, local)</option>
                <option value="mock_ai">Mock AI (testing only)</option>
                <option value="openai_planned" disabled>OpenAI (planned — not available)</option>
                <option value="local_llm_planned" disabled>Local LLM (planned — not available)</option>
              </>
            )}
          </select>
        </div>

        {/* Run cleanup button */}
        <button
          className="btn-insert primary"
          onClick={handleRunCleanup}
          disabled={isBusy || cleanupState?.status === "cleaning"}
          style={{ width: "100%", marginTop: 8 }}
        >
          {cleanupState?.status === "cleaning" ? "⏳ Cleaning..." : "🔧 Run Cleanup"}
        </button>

        {/* Cleanup error */}
        {cleanupError && (
          <div className="recording-error"><p>{cleanupError}</p></div>
        )}
        {cleanupState?.error && !cleanupError && (
          <div className="recording-error"><p>{cleanupState.error}</p></div>
        )}

        {/* Final text display */}
        {hasFinalText && clResult && (
          <div className="cleanup-result">
            <div className="cleanup-meta">
              <span className="cleanup-badge">
                {clResult.mode.replace(/_/g, " ")}
              </span>
              <span className={`cleanup-badge ${clResult.is_mock ? "mock" : "real"}`}>
                {clResult.is_mock ? "⚠️ Mock" : "✅ Basic"}
              </span>
              {clResult.changed && <span className="cleanup-badge">Modified</span>}
              <span className="cleanup-badge">{clResult.duration_ms}ms</span>
            </div>
            {clResult.warnings.length > 0 && (
              <div className="cleanup-warnings">
                {clResult.warnings.map((w, i) => (
                  <p key={i} className="cleanup-warning">{w}</p>
                ))}
              </div>
            )}
            <pre className="transcript-text cleaned">{clResult.final_text}</pre>
            <div className="cleanup-actions">
              <button className="btn-insert" onClick={handleCopyFinal}>
                📋 Copy Final
              </button>
              <button className="btn-insert primary" onClick={handleInsertFinal}>
                ↩️ Insert Final
              </button>
              <button className="btn-insert" onClick={handleSaveToHistory}>
                💾 Save to History
              </button>
              <button className="btn-clear" onClick={handleClearFinal}>
                Clear
              </button>
            </div>
          </div>
        )}

        {!hasFinalText && (
          <p className="transcription-hint" style={{ marginTop: 8 }}>
            Transcribe first, then run cleanup to generate final text.
            Basic cleanup is deterministic and runs entirely on your device.
            AI cleanup providers (OpenAI, local LLM) are planned for future phases.
          </p>
        )}
      </div>

      {/* Phase notice */}
      <div className="phase-notice">
        <p>
          Phase 8 adds local SQLite history — save, view, and manage past transcripts.
          All history is stored locally on your device. No cloud sync, no accounts.
          Save sessions after cleanup using the "Save to History" button.
        </p>
      </div>

      {/* Debug flow state simulator */}
      <div className="flow-simulator">
        <span className="sim-label">Simulate flow state:</span>
        <div className="sim-buttons">
          {(["idle", "recording", "processing", "complete", "error"] as AppFlowState[]).map((s) => (
            <button key={s} className={`sim-btn ${flowState === s ? "active" : ""}`} onClick={() => onStateChange(s)}>
              {s}
            </button>
          ))}
        </div>
        <p className="sim-note">
          Flow state simulator — for UI testing. Real recording uses the buttons above.
        </p>
      </div>
    </section>
  );
}
