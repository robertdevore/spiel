import { useState, useEffect, useCallback } from "react";
import type { SpielSettings, PrivacyStatus, UpdateSettingsRequest } from "../lib/types";
import { getSettings, updateSettings, resetSettings, getPrivacyStatus, setHistoryEnabled } from "../lib/api";

export default function SettingsPanel() {
  const [settings, setSettings] = useState<SpielSettings | null>(null);
  const [privacy, setPrivacy] = useState<PrivacyStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [savedMsg, setSavedMsg] = useState<string | null>(null);

  const loadAll = useCallback(async () => {
    try {
      const [s, p] = await Promise.all([getSettings(), getPrivacyStatus()]);
      setSettings(s);
      setPrivacy(p);
      setError(null);
    } catch (e) { setError(String(e)); }
  }, []);

  useEffect(() => { loadAll(); }, [loadAll]);

  const handleSave = useCallback(async () => {
    if (!settings) return;
    setSavedMsg(null);
    try {
      const update: UpdateSettingsRequest = {
        history_enabled: settings.history_enabled,
        clipboard_restore_enabled: settings.clipboard_restore_enabled,
        default_transcription_engine: settings.default_transcription_engine,
        default_text_mode: settings.default_text_mode,
        default_cleanup_provider: settings.default_cleanup_provider,
        local_whisper_binary_path: settings.local_whisper_binary_path,
        local_whisper_model_path: settings.local_whisper_model_path,
        local_only_mode: settings.local_only_mode,
        debug_logging_enabled: settings.debug_logging_enabled,
      };
      const result = await updateSettings(update);
      setSettings(result);
      setSavedMsg("Settings saved.");
      // Sync history enabled state
      await setHistoryEnabled(result.history_enabled);
    } catch (e) { setError(String(e)); }
  }, [settings]);

  const handleReset = useCallback(async () => {
    try {
      const result = await resetSettings();
      setSettings(result);
      setSavedMsg("Settings reset to safe defaults.");
    } catch (e) { setError(String(e)); }
  }, []);

  const update = (field: keyof SpielSettings, value: string | boolean) => {
    if (!settings) return;
    setSettings({ ...settings, [field]: value });
    setSavedMsg(null);
  };

  if (!settings) {
    return (
      <section className="settings-panel">
        <h3 className="section-title">Settings</h3>
        <p className="section-hint">Loading settings...</p>
      </section>
    );
  }

  return (
    <section className="settings-panel">
      <h3 className="section-title">Settings</h3>
      <p className="section-hint">
        Settings persist across app restarts. Stored locally — no cloud sync.
      </p>

      {error && <div className="recording-error"><p>{error}</p></div>}
      {savedMsg && <div className="config-valid">{savedMsg}</div>}

      {/* Privacy Group */}
      <div className="settings-group">
        <h4 className="settings-group-title">Privacy</h4>
        <div className="settings-row">
          <span className="settings-label">Local-only mode</span>
          <label className="settings-toggle">
            <input type="checkbox" checked={settings.local_only_mode}
              onChange={(e) => update("local_only_mode", e.target.checked)} />
            {settings.local_only_mode ? "On" : "Off"}
          </label>
        </div>
        {privacy && (
          <div className="privacy-summary">
            <p className="privacy-item">☁️ Cloud available: {privacy.cloud_available ? "Yes" : "No"}</p>
            <p className="privacy-item">🤖 OpenAI available: {privacy.openai_available ? "Yes" : "No"}</p>
            <p className="privacy-item">🔒 History encrypted: {privacy.history_encrypted ? "Yes" : "No"}</p>
            <p className="privacy-item">📋 Clipboard stored: {privacy.clipboard_contents_stored ? "Yes" : "No"}</p>
            <p className="privacy-item">🔑 API keys stored: {privacy.api_keys_stored ? "Yes" : "No"}</p>
            <p className="privacy-item">🌐 Network calls: {privacy.network_calls_possible ? "Yes" : "No"}</p>
          </div>
        )}
      </div>

      {/* History Group */}
      <div className="settings-group">
        <h4 className="settings-group-title">History</h4>
        <div className="settings-row">
          <span className="settings-label">Save history</span>
          <label className="settings-toggle">
            <input type="checkbox" checked={settings.history_enabled}
              onChange={(e) => update("history_enabled", e.target.checked)} />
            {settings.history_enabled ? "Enabled" : "Disabled"}
          </label>
        </div>
      </div>

      {/* Clipboard Group */}
      <div className="settings-group">
        <h4 className="settings-group-title">Clipboard</h4>
        <div className="settings-row">
          <span className="settings-label">Restore clipboard after paste</span>
          <label className="settings-toggle">
            <input type="checkbox" checked={settings.clipboard_restore_enabled}
              onChange={(e) => update("clipboard_restore_enabled", e.target.checked)} />
            {settings.clipboard_restore_enabled ? "On" : "Off"}
          </label>
        </div>
      </div>

      {/* Transcription Group */}
      <div className="settings-group">
        <h4 className="settings-group-title">Transcription</h4>
        <div className="settings-row">
          <span className="settings-label">Default engine</span>
          <select className="settings-select" value={settings.default_transcription_engine}
            onChange={(e) => update("default_transcription_engine", e.target.value)}>
            <option value="mock">Mock</option>
            <option value="local_whisper">Local Whisper</option>
            <option value="cloud" disabled>Cloud (not available)</option>
          </select>
        </div>
      </div>

      {/* Local Whisper Group */}
      <div className="settings-group">
        <h4 className="settings-group-title">Local Whisper</h4>
        <div className="settings-row-col">
          <label className="config-label">Binary Path</label>
          <input className="config-input" type="text" value={settings.local_whisper_binary_path}
            onChange={(e) => update("local_whisper_binary_path", e.target.value)}
            placeholder="/usr/local/bin/whisper-cpp" />
        </div>
        <div className="settings-row-col">
          <label className="config-label">Model Path</label>
          <input className="config-input" type="text" value={settings.local_whisper_model_path}
            onChange={(e) => update("local_whisper_model_path", e.target.value)}
            placeholder="/path/to/ggml-base.en.bin" />
        </div>
      </div>

      {/* Defaults Group */}
      <div className="settings-group">
        <h4 className="settings-group-title">Defaults</h4>
        <div className="settings-row">
          <span className="settings-label">Text mode</span>
          <select className="settings-select" value={settings.default_text_mode}
            onChange={(e) => update("default_text_mode", e.target.value)}>
            <option value="raw_dictation">Raw Dictation</option>
            <option value="clean_notes">Clean Notes</option>
            <option value="ai_prompt">AI Prompt</option>
            <option value="developer_review">Developer Review</option>
            <option value="thought_piece">Thought Piece</option>
          </select>
        </div>
        <div className="settings-row">
          <span className="settings-label">Cleanup provider</span>
          <select className="settings-select" value={settings.default_cleanup_provider}
            onChange={(e) => update("default_cleanup_provider", e.target.value)}>
            <option value="basic">Basic (deterministic, local)</option>
            <option value="mock_ai">Mock AI (testing)</option>
            <option value="openai_planned" disabled>OpenAI (not available)</option>
            <option value="local_llm_planned" disabled>Local LLM (not available)</option>
          </select>
        </div>
      </div>

      {/* Debug Group */}
      <div className="settings-group">
        <h4 className="settings-group-title">Debug</h4>
        <div className="settings-row">
          <span className="settings-label">Debug logging</span>
          <label className="settings-toggle">
            <input type="checkbox" checked={settings.debug_logging_enabled}
              onChange={(e) => update("debug_logging_enabled", e.target.checked)} />
            {settings.debug_logging_enabled ? "On" : "Off"}
          </label>
        </div>
      </div>

      {/* Actions */}
      <div className="settings-actions">
        <button className="btn-insert primary" onClick={handleSave}>💾 Save Settings</button>
        <button className="btn-clear" onClick={handleReset}>↩ Reset to Defaults</button>
      </div>
    </section>
  );
}
