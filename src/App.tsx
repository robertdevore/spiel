import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import AppHeader from "./components/AppHeader";
import CapturePanel from "./components/CapturePanel";
import ModeSelector from "./components/ModeSelector";
import TranscriptPreview from "./components/TranscriptPreview";
import HistoryPanel from "./components/HistoryPanel";
import SettingsPanel from "./components/SettingsPanel";
import PrivacyNotice from "./components/PrivacyNotice";
import { getAppStatus, getAppInfo, getHotkeyStatus, getRecordingStatus, getTranscriptionStatus, getTextModes, getCleanupProviders, getCleanupStatus } from "./lib/api";
import type { AppFlowState, TextMode, AppStatusResponse, AppInfoResponse, HotkeyStatus, RecordingStatus, InsertResult, TranscriptionStateData, CleanupStateData, ModeDefinition, CleanupProviderInfo } from "./lib/types";
import "./styles/app.css";

/**
 * Root application component for Spiel.
 * Manages top-level UI state and coordinates component rendering.
 */
export default function App() {
  const [flowState, setFlowState] = useState<AppFlowState>("idle");
  const [selectedMode, setSelectedMode] = useState<TextMode>("raw_dictation");
  const [rawTranscript, setRawTranscript] = useState("");
  const [finalText, setFinalText] = useState("");
  const [appStatus, setAppStatus] = useState<AppStatusResponse | null>(null);
  const [appInfo, setAppInfo] = useState<AppInfoResponse | null>(null);
  const [backendError, setBackendError] = useState<string | null>(null);
  const [hotkeyStatus, setHotkeyStatus] = useState<HotkeyStatus | null>(null);
  const [recordingStatus, setRecordingStatus] = useState<RecordingStatus | null>(null);
  const [insertionText, setInsertionText] = useState("");
  const [lastInsertResult, setLastInsertResult] = useState<InsertResult | null>(null);
  const [transcriptionState, setTranscriptionState] = useState<TranscriptionStateData | null>(null);
  const [cleanupState, setCleanupState] = useState<CleanupStateData | null>(null);
  const [modes, setModes] = useState<ModeDefinition[]>([]);
  const [providers, setProviders] = useState<CleanupProviderInfo[]>([]);

  // Fetch backend status on mount
  useEffect(() => {
    let cancelled = false;

    async function loadBackendData() {
      try {
        const [status, info, hkStatus, recStatus, transStatus, modeList, providerList, clStatus] = await Promise.all([
          getAppStatus(),
          getAppInfo(),
          getHotkeyStatus(),
          getRecordingStatus(),
          getTranscriptionStatus(),
          getTextModes(),
          getCleanupProviders(),
          getCleanupStatus(),
        ]);
        if (!cancelled) {
          setAppStatus(status);
          setAppInfo(info);
          setHotkeyStatus(hkStatus);
          setRecordingStatus(recStatus);
          setTranscriptionState(transStatus);
          setModes(modeList);
          setProviders(providerList);
          setCleanupState(clStatus);
          setBackendError(null);
        }
      } catch (err) {
        if (!cancelled) {
          console.error("Failed to connect to Tauri backend:", err);
          setBackendError(
            "Could not connect to the Spiel backend. Are you running inside Tauri? (This is expected when running in a browser for UI development.)"
          );
        }
      }
    }

    loadBackendData();
    return () => { cancelled = true; };
  }, []);

  // Listen for global hotkey trigger events from the Rust backend
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    async function setupListener() {
      try {
        unlisten = await listen("hotkey-triggered", () => {
          // Toggle between idle and recording placeholder states
          setFlowState((prev) => {
            if (prev === "recording") {
              return "idle";
            }
            return "recording";
          });

          // Refresh hotkey status to get updated trigger count
          getHotkeyStatus().then(setHotkeyStatus).catch(() => {
            // Silently ignore — status refresh is best-effort
          });
        });
      } catch (err) {
        console.error("Failed to listen for hotkey-triggered events:", err);
      }
    }

    setupListener();
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // Poll recording status frequently while recording for elapsed time updates
  useEffect(() => {
    const isRecording = recordingStatus?.state === "recording";
    const interval = setInterval(() => {
      if (!backendError) {
        getRecordingStatus().then(setRecordingStatus).catch(() => {});
      }
    }, isRecording ? 200 : 2000);
    return () => clearInterval(interval);
  }, [backendError, recordingStatus?.state]);

  const handleStateChange = useCallback((state: AppFlowState) => {
    setFlowState(state);
    // Simulate placeholder transcript data for demo
    if (state === "complete") {
      setRawTranscript(
        "Um, so I was thinking about the architecture... we should probably use a modular approach where each component is independently testable."
      );
      setFinalText(
        "I've been thinking about the architecture. We should use a modular approach where each component is independently testable."
      );
    } else if (state === "idle") {
      setRawTranscript("");
      setFinalText("");
    }
  }, []);

  return (
    <div className="app-container">
      <AppHeader
        appName={appInfo?.name ?? "Spiel"}
        tagline={appInfo?.tagline ?? "Get the thought out. Put it where your cursor is."}
      />

      <main className="app-main">
        {/* Backend connection status */}
        {backendError && (
          <div className="backend-notice">
            <p>{backendError}</p>
          </div>
        )}

        {appStatus && !backendError && (
          <div className="backend-ok">
            <span className="status-dot small" style={{ background: "#4aff88" }} />
            <span>
              Backend connected — {appStatus.app_name} v{appStatus.version} ({appStatus.phase})
            </span>
          </div>
        )}

        {/* Capture Panel — now with cleanup section */}
        <CapturePanel
          flowState={flowState}
          onStateChange={handleStateChange}
          hotkeyStatus={hotkeyStatus}
          recordingStatus={recordingStatus}
          insertionText={insertionText}
          onInsertionTextChange={setInsertionText}
          lastInsertResult={lastInsertResult}
          onInsertResult={setLastInsertResult}
          transcriptionState={transcriptionState}
          onTranscriptionStateChange={setTranscriptionState}
          cleanupState={cleanupState}
          onCleanupStateChange={setCleanupState}
          modes={modes}
          providers={providers}
        />

        {/* Mode Selector — wired to real mode definitions */}
        <ModeSelector
          selectedMode={selectedMode}
          onModeChange={setSelectedMode}
          modes={modes}
        />

        {/* Transcript Preview — raw + final text */}
        <TranscriptPreview
          flowState={flowState}
          rawTranscript={rawTranscript}
          finalText={finalText}
          cleanupState={cleanupState}
        />

        {/* History — placeholder for Phase 8 */}
        <HistoryPanel />

        {/* Settings — placeholder for Phase 9 */}
        <SettingsPanel />

        {/* Privacy Notice */}
        <PrivacyNotice />

        {/* Capability Status Summary */}
        {appStatus && (
          <section className="capability-summary">
            <h3 className="section-title">Capability Status</h3>
            <div className="capability-grid">
              {appStatus.capabilities.map((cap) => (
                <div key={cap.name} className={`capability-badge ${cap.status}`}>
                  <span className="cap-name">{cap.name}</span>
                  <span className="cap-status">{cap.status}</span>
                </div>
              ))}
            </div>
          </section>
        )}
      </main>

      <footer className="app-footer">
        <p>
          Spiel v{appStatus?.version ?? "0.1.0"} — Phase 1 Foundation Build
        </p>
        <p className="footer-tagline">
          Spoken thoughts, usable text, anywhere.
        </p>
      </footer>
    </div>
  );
}
