import type { AppFlowState, CleanupStateData } from "../lib/types";

interface TranscriptPreviewProps {
  flowState: AppFlowState;
  rawTranscript: string;
  finalText: string;
  cleanupState: CleanupStateData | null;
}

/**
 * Displays the current transcript preview.
 * Shows raw transcript and final text separately.
 * In Phase 7, the cleanup section in CapturePanel handles the primary display;
 * this provides a secondary view for larger text areas.
 */
export default function TranscriptPreview({
  flowState,
  rawTranscript,
  finalText,
  cleanupState,
}: TranscriptPreviewProps) {
  const hasCleanupResult = cleanupState?.last_result != null;
  const cleanupResult = cleanupState?.last_result;

  // Use cleanup result data if available, otherwise fall back to props
  const rawText = cleanupResult?.raw_text ?? rawTranscript;
  const final = cleanupResult?.final_text ?? finalText;
  const hasContent = rawText || final;

  return (
    <section className="transcript-preview">
      <h3 className="section-title">Transcript Preview</h3>

      {!hasContent && flowState === "idle" && (
        <div className="empty-state">
          <p>No transcript yet. Record audio, transcribe, then run cleanup to see your text here.</p>
          <p className="empty-hint">
            Phase 7 — text modes and cleanup are now available. Record → Transcribe → Run Cleanup.
          </p>
        </div>
      )}

      {flowState === "recording" && (
        <div className="recording-indicator">
          <span className="pulse" />
          <p>Recording in progress...</p>
        </div>
      )}

      {flowState === "processing" && (
        <div className="processing-indicator">
          <span className="spinner" />
          <p>Processing...</p>
        </div>
      )}

      {hasContent && flowState !== "recording" && (
        <div className="transcript-results">
          <div className="transcript-box">
            <h4>Raw Transcript</h4>
            <pre className="transcript-text">
              {rawText || "(No raw transcript)"}
            </pre>
          </div>
          <div className="transcript-box">
            <h4>
              Final Text
              {hasCleanupResult && cleanupResult && (
                <span className="cleanup-badge-inline">
                  {cleanupResult.mode.replace(/_/g, " ")} · {cleanupResult.provider.replace(/_/g, " ")}
                </span>
              )}
            </h4>
            <pre className="transcript-text cleaned">
              {final || "(Run cleanup to generate final text)"}
            </pre>
            {hasCleanupResult && cleanupResult && (
              <div className="cleanup-meta-inline">
                <span className="meta-item">
                  {cleanupResult.is_mock ? "\u26a0\ufe0f Mock" : "\u2705 Real"} cleanup
                </span>
                {cleanupResult.changed && <span className="meta-item">Modified</span>}
                <span className="meta-item">{cleanupResult.duration_ms}ms</span>
                {cleanupResult.warnings.length > 0 && (
                  <span className="meta-item warning" title={cleanupResult.warnings.join("\n")}>
                    {cleanupResult.warnings.length} warning{cleanupResult.warnings.length !== 1 ? "s" : ""}
                  </span>
                )}
              </div>
            )}
          </div>
        </div>
      )}

      {flowState === "error" && (
        <div className="error-state">
          <p>\u26a0\ufe0f An error occurred during transcription.</p>
          <p className="error-hint">
            Check the Capture Panel for details and try again.
          </p>
        </div>
      )}
    </section>
  );
}
