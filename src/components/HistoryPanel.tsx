import { useState, useEffect, useCallback } from "react";
import type { HistoryEntry, HistoryStateData } from "../lib/types";
import { listHistoryEntries, getHistoryEntry, deleteHistoryEntry, clearHistory, getHistoryStatus, setHistoryEnabled, copyToClipboard, insertViaClipboard } from "../lib/api";

interface HistoryPanelProps {}

function formatDate(iso: string): string {
  try {
    const d = new Date(iso);
    const now = new Date();
    const diffMs = now.getTime() - d.getTime();
    const diffMin = Math.floor(diffMs / 60000);
    if (diffMin < 1) return "Just now";
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHrs = Math.floor(diffMin / 60);
    if (diffHrs < 24) return `${diffHrs}h ago`;
    return d.toLocaleDateString();
  } catch {
    return iso.slice(0, 10);
  }
}

export default function HistoryPanel(_props: HistoryPanelProps) {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [selectedEntry, setSelectedEntry] = useState<HistoryEntry | null>(null);
  const [historyState, setHistoryState] = useState<HistoryStateData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showClearConfirm, setShowClearConfirm] = useState(false);

  const loadEntries = useCallback(async () => {
    try {
      const [list, status] = await Promise.all([
        listHistoryEntries(10),
        getHistoryStatus(),
      ]);
      setEntries(list);
      setHistoryState(status);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => { loadEntries(); }, [loadEntries]);

  const handleViewEntry = useCallback(async (id: number) => {
    try {
      const entry = await getHistoryEntry(id);
      setSelectedEntry(entry);
      setError(null);
    } catch (e) { setError(String(e)); }
  }, []);

  const handleDeleteEntry = useCallback(async (id: number) => {
    try {
      await deleteHistoryEntry(id);
      setSelectedEntry(null);
      loadEntries();
    } catch (e) { setError(String(e)); }
  }, [loadEntries]);

  const handleClearAll = useCallback(async () => {
    try {
      await clearHistory();
      setSelectedEntry(null);
      setShowClearConfirm(false);
      loadEntries();
    } catch (e) { setError(String(e)); }
  }, [loadEntries]);

  const handleToggleEnabled = useCallback(async () => {
    try {
      const newEnabled = !(historyState?.enabled ?? true);
      const status = await setHistoryEnabled(newEnabled);
      setHistoryState(status);
    } catch (e) { setError(String(e)); }
  }, [historyState?.enabled]);

  const handleCopyFinal = useCallback(async () => {
    if (!selectedEntry?.final_text) return;
    try { await copyToClipboard(selectedEntry.final_text); }
    catch (e) { setError(String(e)); }
  }, [selectedEntry]);

  const handleInsertFinal = useCallback(async () => {
    if (!selectedEntry?.final_text) return;
    try { await insertViaClipboard(selectedEntry.final_text, true); }
    catch (e) { setError(String(e)); }
  }, [selectedEntry]);

  const isEnabled = historyState?.enabled ?? true;
  const entryCount = historyState?.entry_count ?? entries.length;

  return (
    <section className="history-panel">
      <h3 className="section-title">History</h3>
      <div className="history-status-row">
        <span className="history-status-text">
          History: {isEnabled ? "Enabled" : "Disabled"} · {entryCount} entr{entryCount !== 1 ? "ies" : "y"}
        </span>
        <button className="btn-toggle" onClick={handleToggleEnabled}>
          {isEnabled ? "Disable" : "Enable"}
        </button>
        {entryCount > 0 && (
          <button className="btn-clear-history" onClick={() => setShowClearConfirm(true)}>Clear All</button>
        )}
      </div>
      <p className="section-hint">
        History is stored locally on this device. No cloud sync, no accounts.
        Previous clipboard contents are never saved.
      </p>
      {error && <div className="recording-error"><p>{error}</p></div>}
      {showClearConfirm && (
        <div className="clear-confirm">
          <p>Permanently delete all {entryCount} history entries? This cannot be undone.</p>
          <div style={{ display: "flex", gap: 6 }}>
            <button className="btn-insert" onClick={handleClearAll}>Yes, Clear All</button>
            <button className="btn-clear" onClick={() => setShowClearConfirm(false)}>Cancel</button>
          </div>
        </div>
      )}
      {selectedEntry && (
        <div className="history-detail">
          <div className="history-detail-header">
            <h4>{selectedEntry.title || "Untitled"}</h4>
            <button className="btn-back" onClick={() => setSelectedEntry(null)}>← Back</button>
          </div>
          <div className="history-detail-meta">
            <span className="history-badge">{selectedEntry.mode.replace(/_/g, " ") || "unknown"}</span>
            <span className="history-badge">{selectedEntry.cleanup_provider.replace(/_/g, " ") || "unknown"}</span>
            {selectedEntry.is_mock_transcript && <span className="history-badge mock">Mock Transcript</span>}
            {selectedEntry.is_mock_cleanup && <span className="history-badge mock">Mock Cleanup</span>}
            <span className="history-date-detail">{formatDate(selectedEntry.created_at)}</span>
          </div>
          <div className="history-detail-text">
            <h5>Raw Transcript</h5>
            <pre className="transcript-text">{selectedEntry.raw_text || "(empty)"}</pre>
            <h5>Final Text</h5>
            <pre className="transcript-text cleaned">{selectedEntry.final_text || "(empty)"}</pre>
          </div>
          <div className="history-detail-actions">
            <button className="btn-insert" onClick={handleCopyFinal}>📋 Copy Final</button>
            <button className="btn-insert primary" onClick={handleInsertFinal}>↩️ Insert Final</button>
            <button className="btn-clear" onClick={() => handleDeleteEntry(selectedEntry.id)}>🗑 Delete</button>
          </div>
        </div>
      )}
      {!selectedEntry && (
        <div className="history-list">
          {entries.length === 0 ? (
            <div className="empty-state">
              <p>No history entries yet.</p>
              <p className="empty-hint">Save a session after transcribing and cleaning up to see it here.</p>
            </div>
          ) : (
            entries.map((entry) => (
              <div key={entry.id} className="history-entry" onClick={() => handleViewEntry(entry.id)}>
                <span className="history-preview">{entry.title || "Untitled"}{entry.is_mock_transcript && " ⚠️"}</span>
                <span className="history-date">{formatDate(entry.created_at)}</span>
              </div>
            ))
          )}
        </div>
      )}
    </section>
  );
}
