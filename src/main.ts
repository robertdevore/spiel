import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";

// ── Types mirroring the Rust backend ────────────────────────────────────────
type Phase = "idle" | "recording" | "transcribing" | "inserting" | "error";

interface StatusSnapshot {
  phase: Phase;
  message: string | null;
  needs_accessibility: boolean;
  recording_elapsed_ms: number;
  model_id: string;
  model_installed: boolean;
  accessibility_trusted: boolean;
}

interface Config {
  hotkey: string;
  model: string;
  language: string;
  auto_paste: boolean;
  restore_clipboard: boolean;
  keep_model_loaded: boolean;
  transcription_threads: number;
  max_seconds: number;
}

interface MemoryProfile {
  id: string;
  label: string;
  summary: string;
  patch: Partial<Config>;
}

interface ModelView {
  id: string;
  label: string;
  approx_mb: number;
  note: string;
  installed: boolean;
  is_current: boolean;
}

interface TranscriptEvent {
  text: string;
  outcome: {
    pasted: boolean;
    clipboard_only: boolean;
    restored_previous: boolean;
    needs_accessibility: boolean;
  };
}

interface PerfSample {
  wall_time_ms: number;
  capture_ms: number;
  transcribe_ms: number;
  insert_ms: number;
  total_ms: number;
  audio_samples: number;
  text_chars: number;
  outcome: string;
}

interface PerfSnapshot {
  enabled: boolean;
  budget_ms: number;
  sample_count: number;
  average_total_ms: number;
  p95_total_ms: number;
  max_total_ms: number;
  over_budget_count: number;
  last: PerfSample | null;
}

interface PerfEvent {
  capture_ms: number;
  transcribe_ms: number;
  insert_ms: number;
  total_ms: number;
  text_chars: number;
  outcome: string;
}

interface LanguageOption {
  value: string;
  label: string;
}

// ── Module state ────────────────────────────────────────────────────────────
let status: StatusSnapshot | null = null;
let config: Config | null = null;
let models: ModelView[] = [];
let lastTranscript = "";
let lastError = "";
let perf: PerfSnapshot | null = null;
let refreshInFlight = false;
let refreshRequested = false;
let renderQueued = false;
let recordingAnchorMs: number | null = null;
let recordingBaseElapsedMs = 0;
const downloads: Record<string, { downloaded: number; total: number | null }> = {};

const app = document.getElementById("app")!;

// ── Helpers ─────────────────────────────────────────────────────────────────
const PHASE_LABEL: Record<Phase, string> = {
  idle: "Idle",
  recording: "Recording…",
  transcribing: "Transcribing…",
  inserting: "Inserting…",
  error: "Error",
};

const LANGUAGE_OPTIONS: LanguageOption[] = [
  { value: "auto", label: "Auto-detect" },
  { value: "en", label: "English" },
  { value: "es", label: "Spanish" },
  { value: "fr", label: "French" },
  { value: "de", label: "German" },
  { value: "it", label: "Italian" },
  { value: "pt", label: "Portuguese" },
  { value: "ru", label: "Russian" },
  { value: "ja", label: "Japanese" },
  { value: "zh", label: "Chinese" },
];

const MEMORY_PROFILES: MemoryProfile[] = [
  {
    id: "low",
    label: "Low Memory",
    summary: "Tiny English model, unload after each run, 1 thread.",
    patch: { model: "tiny.en", keep_model_loaded: false, transcription_threads: 1 },
  },
  {
    id: "balanced",
    label: "Balanced",
    summary: "Base English model, unload after each run, 2 threads.",
    patch: { model: "base.en", keep_model_loaded: false, transcription_threads: 2 },
  },
  {
    id: "quality",
    label: "Quality",
    summary: "Small multilingual model, unload after each run, 2 threads.",
    patch: { model: "small", keep_model_loaded: false, transcription_threads: 2 },
  },
  {
    id: "global",
    label: "Global",
    summary: "Medium multilingual model, unload after each run, 2 threads.",
    patch: { model: "medium", keep_model_loaded: false, transcription_threads: 2 },
  },
];

function fmtBytes(n: number): string {
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

async function refreshAll() {
  if (refreshInFlight) {
    refreshRequested = true;
    return;
  }
  refreshInFlight = true;
  try {
    [status, config, models] = await Promise.all([
      invoke<StatusSnapshot>("get_status"),
      invoke<Config>("get_config"),
      invoke<ModelView[]>("list_models"),
    ]);
    perf = await invoke<PerfSnapshot>("get_perf_snapshot");
    syncRecordingClock();
  } catch (e) {
    console.error("Backend unavailable:", e);
  } finally {
    refreshInFlight = false;
  }
  render();
  if (refreshRequested) {
    refreshRequested = false;
    void refreshAll();
  }
}

async function refreshStatusOnly() {
  try {
    status = await invoke<StatusSnapshot>("get_status");
    syncRecordingClock();
    render();
  } catch (e) {
    console.error("Status refresh failed:", e);
  }
}

async function refreshPerfOnly() {
  try {
    perf = await invoke<PerfSnapshot>("get_perf_snapshot");
    render();
  } catch (e) {
    console.error("Perf refresh failed:", e);
  }
}

async function refreshModelsOnly() {
  try {
    models = await invoke<ModelView[]>("list_models");
    render();
  } catch (e) {
    console.error("Model refresh failed:", e);
  }
}

async function saveConfig(patch: Partial<Config>) {
  if (!config) return;
  const next = { ...config, ...patch };
  try {
    config = await invoke<Config>("update_config", { config: next });
    await refreshStatusOnly();
    if (patch.model !== undefined) {
      await refreshModelsOnly();
    } else {
      render();
    }
  } catch (e) {
    setError(`Could not save settings: ${e}`);
  }
}

function syncRecordingClock() {
  if (!status || status.phase !== "recording") {
    recordingAnchorMs = null;
    recordingBaseElapsedMs = 0;
    return;
  }
  recordingBaseElapsedMs = status.recording_elapsed_ms;
  recordingAnchorMs = Date.now();
}

function currentElapsedMs(s: StatusSnapshot): number {
  if (s.phase !== "recording") return 0;
  if (recordingAnchorMs == null) return s.recording_elapsed_ms;
  return recordingBaseElapsedMs + Math.max(0, Date.now() - recordingAnchorMs);
}

function queueRender() {
  if (renderQueued) return;
  renderQueued = true;
  requestAnimationFrame(() => {
    renderQueued = false;
    render();
  });
}

// ── Render ──────────────────────────────────────────────────────────────────
function render() {
  if (!status || !config) {
    app.innerHTML = `<div class="card">Connecting to Spiel…</div>`;
    return;
  }
  const s = status;
  const c = config;

  app.innerHTML = "";
  app.appendChild(headerEl());
  if (lastError) app.appendChild(errorBanner(lastError));
  app.appendChild(statusCard(s, c));
  if (s.needs_accessibility || !s.accessibility_trusted) {
    app.appendChild(accessibilityCard(s));
  }
  app.appendChild(transcriptCard());
  app.appendChild(modelsCard());
  app.appendChild(settingsCard(c));
  if (perf?.enabled) app.appendChild(perfCard(perf));
  app.appendChild(privacyEl());
}

function headerEl(): HTMLElement {
  const el = document.createElement("header");
  el.innerHTML = `<h1>Spiel</h1><span class="tag">local dictation</span>`;
  return el;
}

function errorBanner(msg: string): HTMLElement {
  const el = document.createElement("div");
  el.className = "card error-banner";
  const text = document.createElement("div");
  text.className = "warn";
  text.textContent = msg;
  const dismiss = document.createElement("button");
  dismiss.textContent = "Dismiss";
  dismiss.onclick = () => {
    lastError = "";
    render();
  };
  el.appendChild(text);
  el.appendChild(dismiss);
  return el;
}

function setError(msg: string) {
  lastError = msg;
  render();
}

function statusCard(s: StatusSnapshot, c: Config): HTMLElement {
  const card = document.createElement("div");
  card.className = "card";
  const recording = s.phase === "recording";
  const busy = s.phase === "transcribing" || s.phase === "inserting";
  const elapsed = recording ? ` ${(currentElapsedMs(s) / 1000).toFixed(1)}s` : "";

  const row = document.createElement("div");
  row.className = "status-row";
  const dot = document.createElement("span");
  dot.className = `dot ${s.phase}`;
  const statusText = document.createElement("span");
  statusText.className = "status-text";
  statusText.textContent = `${PHASE_LABEL[s.phase]}${elapsed}`;
  row.appendChild(dot);
  row.appendChild(statusText);
  card.appendChild(row);

  const statusMsg = document.createElement("div");
  statusMsg.className = "status-msg";
  statusMsg.textContent = s.message ?? "";
  card.appendChild(statusMsg);

  const btn = document.createElement("button");
  btn.className = `big-btn ${recording ? "recording" : "primary"}`;
  btn.textContent = recording ? "Stop & Insert" : "Start Dictation";
  btn.disabled = busy || !s.model_installed;
  btn.onclick = () => invoke("toggle_dictation").catch((e) => console.error(e));
  card.appendChild(btn);

  const hint = document.createElement("div");
  hint.className = "privacy";
  hint.append("Press ");
  const key = document.createElement("span");
  key.className = "kbd";
  key.textContent = c.hotkey;
  hint.appendChild(key);
  hint.append(" anywhere to toggle.");
  card.appendChild(hint);

  if (!s.model_installed) {
    const warn = document.createElement("div");
    warn.className = "warn";
    warn.textContent = "Download a speech model below to start dictating.";
    card.appendChild(warn);
  }
  return card;
}

function accessibilityCard(s: StatusSnapshot): HTMLElement {
  const card = document.createElement("div");
  card.className = "card";
  card.innerHTML = `<p class="section-title">Accessibility</p>
    <div class="privacy">Auto-paste needs Accessibility permission so Spiel can press Cmd+V
    in the focused app. ${s.accessibility_trusted ? "Granted ✓" : "Not granted yet."}
    Until then, transcripts are placed on the clipboard for a manual paste.</div>`;
  if (!s.accessibility_trusted) {
    const btn = document.createElement("button");
    btn.textContent = "Grant Accessibility…";
    btn.onclick = async () => {
      await invoke("request_accessibility").catch((e) => console.error(e));
      setTimeout(refreshStatusOnly, 500);
    };
    card.appendChild(btn);
  }
  return card;
}

function transcriptCard(): HTMLElement {
  const card = document.createElement("div");
  card.className = "card";
  const body = lastTranscript
    ? `<div class="transcript">${escapeHtml(lastTranscript)}</div>`
    : `<div class="transcript empty">Your last transcript will appear here.</div>`;
  card.innerHTML = `<p class="section-title">Last transcript</p>${body}`;
  return card;
}

function modelsCard(): HTMLElement {
  const card = document.createElement("div");
  card.className = "card";
  card.innerHTML = `<p class="section-title">Speech model</p>`;

  for (const m of models) {
    const row = document.createElement("div");
    row.className = `model${m.is_current ? " current" : ""}`;
    const meta = document.createElement("div");
    meta.className = "meta";
    meta.innerHTML = `<span class="name">${m.label} · ~${m.approx_mb} MB</span>
      <span class="note">${m.note}</span>`;
    row.appendChild(meta);

    const dl = downloads[m.id];
    if (dl) {
      const pct = dl.total ? Math.round((dl.downloaded / dl.total) * 100) : 0;
      const wrap = document.createElement("div");
      wrap.style.flex = "1";
      wrap.innerHTML = `<div class="progress"><div style="width:${pct}%"></div></div>
        <div class="note">${fmtBytes(dl.downloaded)}${dl.total ? " / " + fmtBytes(dl.total) : ""}</div>`;
      row.appendChild(wrap);
    } else if (m.installed) {
      if (m.is_current) {
        const badge = document.createElement("span");
        badge.className = "badge";
        badge.textContent = "Active ✓";
        row.appendChild(badge);
      } else {
        const useBtn = document.createElement("button");
        useBtn.textContent = "Use";
        useBtn.onclick = () => saveConfig({ model: m.id });
        row.appendChild(useBtn);

        const deleteBtn = document.createElement("button");
        deleteBtn.className = "danger";
        deleteBtn.textContent = "Delete";
        deleteBtn.onclick = () => {
          if (confirm(`Delete model ${m.label}? This frees disk and unloads any cache.`)) {
            invoke("delete_model", { modelId: m.id }).catch((e) => {
              setError(`Could not delete model: ${e}`);
            }).finally(refreshModelsOnly);
          }
        };
        row.appendChild(deleteBtn);
      }
    } else {
      const dlBtn = document.createElement("button");
      dlBtn.textContent = "Download";
      dlBtn.onclick = () => {
        lastError = "";
        downloads[m.id] = { downloaded: 0, total: null };
        render();
        invoke("download_model", { modelId: m.id }).catch((e) => {
          delete downloads[m.id];
          setError(`Download failed: ${e}`);
        });
      };
      row.appendChild(dlBtn);
    }
    card.appendChild(row);
  }
  return card;
}

function settingsCard(c: Config): HTMLElement {
  const card = document.createElement("div");
  card.className = "card";
  card.innerHTML = `<p class="section-title">Settings</p>`;

  card.appendChild(
    textField("Hotkey", c.hotkey, (v) => saveConfig({ hotkey: v }), "e.g. Cmd+Alt+D"),
  );
  const hkHint = document.createElement("div");
  hkHint.className = "privacy";
  hkHint.textContent =
    "Modifiers + a key, e.g. Cmd+Alt+D or Cmd+Shift+Space (letters/named keys only — '?' won't work). " +
    "Trigger dictation with the hotkey, not the menu-bar Start/Stop: clicking the menu bar moves focus off your text field, so auto-paste has nowhere to land.";
  card.appendChild(hkHint);

  const langWrap = document.createElement("label");
  langWrap.className = "field";
  langWrap.textContent = "Language";
  const langInputWrap = document.createElement("div");
  langInputWrap.className = "row";
  const langInput = document.createElement("input");
  langInput.type = "text";
  langInput.value = c.language;
  langInput.placeholder = "en or auto";
  langInput.setAttribute("list", "language-options");
  langInput.onblur = () => {
    const normalized = normalizeLanguage(langInput.value);
    langInput.value = normalized;
    if (normalized !== c.language) {
      saveConfig({ language: normalized });
    }
  };
  langInput.onkeydown = (event) => {
    if (event.key === "Enter") {
      langInput.blur();
    }
  };
  const languageList = document.createElement("datalist");
  languageList.id = "language-options";
  for (const option of LANGUAGE_OPTIONS) {
    const opt = document.createElement("option");
    opt.value = option.value;
    opt.label = option.label;
    languageList.appendChild(opt);
  }
  langInputWrap.appendChild(langInput);
  langInputWrap.appendChild(languageList);
  langWrap.appendChild(langInputWrap);
  card.appendChild(langWrap);

  card.appendChild(
    toggleField("Auto-paste at cursor", c.auto_paste, (v) => saveConfig({ auto_paste: v })),
  );
  card.appendChild(
    toggleField("Restore previous clipboard", c.restore_clipboard, (v) =>
      saveConfig({ restore_clipboard: v }),
    ),
  );
  card.appendChild(
    toggleField("Keep model loaded in memory", c.keep_model_loaded, (v) =>
      saveConfig({ keep_model_loaded: v }),
    ),
  );
  card.appendChild(
    numberField("Transcription threads", c.transcription_threads, 1, 8, (v) =>
      saveConfig({ transcription_threads: v }),
    ),
  );
  card.appendChild(
    numberField("Max recording (seconds)", c.max_seconds, 5, 600, (v) =>
      saveConfig({ max_seconds: v }),
    ),
  );

  const profileTitle = document.createElement("div");
  profileTitle.className = "section-title";
  profileTitle.textContent = "Memory / Quality Profiles";
  card.appendChild(profileTitle);

  for (const profile of MEMORY_PROFILES) {
    const row = document.createElement("div");
    row.className = "model";
    const meta = document.createElement("div");
    meta.className = "meta";
    meta.innerHTML = `<span class="name">${profile.label}</span><span class="note">${profile.summary}</span>`;
    row.appendChild(meta);
    const applyBtn = document.createElement("button");
    applyBtn.textContent = "Apply";
    applyBtn.onclick = () => saveConfig(profile.patch);
    row.appendChild(applyBtn);
    card.appendChild(row);
  }

  const unloadBtn = document.createElement("button");
  unloadBtn.textContent = "Unload Model From Memory Now";
  unloadBtn.onclick = async () => {
    await invoke("unload_model_from_memory").catch((e) =>
      setError(`Could not unload model from memory: ${e}`),
    );
    await refreshStatusOnly();
  };
  card.appendChild(unloadBtn);
  const memHint = document.createElement("div");
  memHint.className = "privacy";
  memHint.textContent =
    "For lower memory: use Tiny model, keep model loaded OFF, and set threads to 1-2. " +
    "This reduces idle RAM significantly at the cost of slower first transcription after each stop.";
  card.appendChild(memHint);
  return card;
}

function privacyEl(): HTMLElement {
  const el = document.createElement("div");
  el.className = "privacy";
  el.innerHTML = `Spiel records only while dictating, transcribes on your Mac with Whisper,
    and writes nothing to disk except the model file and this settings file. No audio or
    text ever leaves your device. The only network use is the one-time model download above.`;
  return el;
}

function perfCard(p: PerfSnapshot): HTMLElement {
  const card = document.createElement("div");
  card.className = "card";
  card.innerHTML = `<p class="section-title">Performance Profile</p>`;

  const summary = document.createElement("div");
  summary.className = "privacy";
  summary.textContent =
    `Samples: ${p.sample_count} · Avg: ${p.average_total_ms}ms · P95: ${p.p95_total_ms}ms · ` +
    `Max: ${p.max_total_ms}ms · Over budget (${p.budget_ms}ms): ${p.over_budget_count}`;
  card.appendChild(summary);

  if (p.last) {
    const last = document.createElement("div");
    last.className = "privacy";
    last.textContent =
      `Last: total ${p.last.total_ms}ms (capture ${p.last.capture_ms}ms, transcribe ${p.last.transcribe_ms}ms, ` +
      `insert ${p.last.insert_ms}ms) · chars ${p.last.text_chars} · outcome ${p.last.outcome}`;
    card.appendChild(last);
  }

  const controls = document.createElement("div");
  controls.className = "row";
  const clearBtn = document.createElement("button");
  clearBtn.textContent = "Clear Profile Samples";
  clearBtn.onclick = async () => {
    await invoke("clear_perf_samples").catch((e) => setError(`Could not clear profile samples: ${e}`));
    await refreshPerfOnly();
  };
  controls.appendChild(clearBtn);
  card.appendChild(controls);

  return card;
}

// ── Small field builders ────────────────────────────────────────────────────
function textField(
  label: string,
  value: string,
  onCommit: (v: string) => void,
  placeholder = "",
): HTMLElement {
  const wrap = document.createElement("label");
  wrap.className = "field";
  wrap.textContent = label;
  const input = document.createElement("input");
  input.type = "text";
  input.value = value;
  input.placeholder = placeholder;
  const commit = () => {
    if (input.value.trim() && input.value !== value) onCommit(input.value.trim());
  };
  input.onblur = commit;
  input.onkeydown = (e) => {
    if (e.key === "Enter") input.blur();
  };
  wrap.appendChild(input);
  return wrap;
}

function numberField(
  label: string,
  value: number,
  min: number,
  max: number,
  onCommit: (v: number) => void,
): HTMLElement {
  const wrap = document.createElement("label");
  wrap.className = "field";
  wrap.textContent = label;
  const input = document.createElement("input");
  input.type = "number";
  input.min = String(min);
  input.max = String(max);
  input.value = String(value);
  input.onblur = () => {
    const raw = Number(input.value);
    const normalized = Number.isFinite(raw) ? Math.round(raw) : value;
    const v = Math.max(min, Math.min(max, normalized));
    input.value = String(v);
    if (v !== value) onCommit(v);
  };
  wrap.appendChild(input);
  return wrap;
}

function toggleField(label: string, value: boolean, onChange: (v: boolean) => void): HTMLElement {
  const wrap = document.createElement("div");
  wrap.className = "toggle";
  wrap.textContent = label;
  const input = document.createElement("input");
  input.type = "checkbox";
  input.checked = value;
  input.onchange = () => onChange(input.checked);
  wrap.appendChild(input);
  return wrap;
}

function escapeHtml(s: string): string {
  const div = document.createElement("div");
  div.textContent = s;
  return div.innerHTML;
}

function normalizeLanguage(raw: string): string {
  const value = raw.trim().toLowerCase();
  if (!value) {
    return "auto";
  }
  if (value === "auto") {
    return "auto";
  }
  const [primary] = value.split(/[-_]/);
  return /^[a-z]{2}$/.test(primary) ? primary : "auto";
}

// ── Wire up events ──────────────────────────────────────────────────────────
async function init() {
  await listen<StatusSnapshot>("status", (e) => {
    status = e.payload;
    syncRecordingClock();
    queueRender();
  });
  await listen<TranscriptEvent>("transcript", (e) => {
    lastTranscript = e.payload.text;
    queueRender();
  });
  await listen<PerfEvent>("perf", () => {
    refreshPerfOnly();
  });
  await listen<{ model_id: string; downloaded: number; total: number | null }>(
    "model-progress",
    (e) => {
      downloads[e.payload.model_id] = {
        downloaded: e.payload.downloaded,
        total: e.payload.total,
      };
      queueRender();
    },
  );
  await listen<{ model_id: string; ok: boolean; error: string | null }>("model-done", (e) => {
    delete downloads[e.payload.model_id];
    if (!e.payload.ok) {
      lastError = `Model download failed: ${e.payload.error ?? "unknown error"}`;
    } else {
      lastError = "";
    }
    refreshAll();
  });

  // Keep the on-screen elapsed timer smooth without polling backend status.
  setInterval(() => {
    if (status?.phase === "recording") queueRender();
  }, 100);

  await refreshAll();
}

init();
