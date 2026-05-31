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
  max_seconds: number;
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

// ── Module state ────────────────────────────────────────────────────────────
let status: StatusSnapshot | null = null;
let config: Config | null = null;
let models: ModelView[] = [];
let lastTranscript = "";
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

function fmtBytes(n: number): string {
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

async function refreshAll() {
  try {
    [status, config, models] = await Promise.all([
      invoke<StatusSnapshot>("get_status"),
      invoke<Config>("get_config"),
      invoke<ModelView[]>("list_models"),
    ]);
  } catch (e) {
    console.error("Backend unavailable:", e);
  }
  render();
}

async function saveConfig(patch: Partial<Config>) {
  if (!config) return;
  const next = { ...config, ...patch };
  try {
    config = await invoke<Config>("update_config", { config: next });
  } catch (e) {
    alert(`Could not save settings: ${e}`);
  }
  await refreshAll();
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
  app.appendChild(statusCard(s, c));
  if (s.needs_accessibility || !s.accessibility_trusted) {
    app.appendChild(accessibilityCard(s));
  }
  app.appendChild(transcriptCard());
  app.appendChild(modelsCard());
  app.appendChild(settingsCard(c));
  app.appendChild(privacyEl());
}

function headerEl(): HTMLElement {
  const el = document.createElement("header");
  el.innerHTML = `<h1>Spiel</h1><span class="tag">local dictation</span>`;
  return el;
}

function statusCard(s: StatusSnapshot, c: Config): HTMLElement {
  const card = document.createElement("div");
  card.className = "card";
  const recording = s.phase === "recording";
  const busy = s.phase === "transcribing" || s.phase === "inserting";
  const elapsed = recording ? ` ${(s.recording_elapsed_ms / 1000).toFixed(1)}s` : "";

  card.innerHTML = `
    <div class="status-row">
      <span class="dot ${s.phase}"></span>
      <span class="status-text">${PHASE_LABEL[s.phase]}${elapsed}</span>
    </div>
    <div class="status-msg">${s.message ?? ""}</div>
  `;

  const btn = document.createElement("button");
  btn.className = `big-btn ${recording ? "recording" : "primary"}`;
  btn.textContent = recording ? "Stop & Insert" : "Start Dictation";
  btn.disabled = busy || !s.model_installed;
  btn.onclick = () => invoke("toggle_dictation").catch((e) => console.error(e));
  card.appendChild(btn);

  const hint = document.createElement("div");
  hint.className = "privacy";
  hint.innerHTML = `Press <span class="kbd">${c.hotkey}</span> anywhere to toggle.`;
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
      setTimeout(refreshAll, 500);
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
      }
    } else {
      const dlBtn = document.createElement("button");
      dlBtn.textContent = "Download";
      dlBtn.onclick = () => {
        downloads[m.id] = { downloaded: 0, total: null };
        render();
        invoke("download_model", { modelId: m.id }).catch((e) => {
          delete downloads[m.id];
          alert(`Download failed: ${e}`);
          render();
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

  const langWrap = document.createElement("label");
  langWrap.className = "field";
  langWrap.innerHTML = `Language`;
  const langSel = document.createElement("select");
  for (const [val, label] of [
    ["en", "English"],
    ["auto", "Auto-detect"],
  ]) {
    const opt = document.createElement("option");
    opt.value = val;
    opt.textContent = label;
    if (c.language === val) opt.selected = true;
    langSel.appendChild(opt);
  }
  langSel.onchange = () => saveConfig({ language: langSel.value });
  langWrap.appendChild(langSel);
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
    numberField("Max recording (seconds)", c.max_seconds, 5, 600, (v) =>
      saveConfig({ max_seconds: v }),
    ),
  );
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
    const v = Math.max(min, Math.min(max, Number(input.value) || value));
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

// ── Wire up events ──────────────────────────────────────────────────────────
async function init() {
  await listen<StatusSnapshot>("status", (e) => {
    status = e.payload;
    render();
  });
  await listen<TranscriptEvent>("transcript", (e) => {
    lastTranscript = e.payload.text;
    render();
  });
  await listen<{ model_id: string; downloaded: number; total: number | null }>(
    "model-progress",
    (e) => {
      downloads[e.payload.model_id] = {
        downloaded: e.payload.downloaded,
        total: e.payload.total,
      };
      render();
    },
  );
  await listen<{ model_id: string; ok: boolean; error: string | null }>("model-done", (e) => {
    delete downloads[e.payload.model_id];
    if (!e.payload.ok && e.payload.error) alert(`Model download failed: ${e.payload.error}`);
    refreshAll();
  });

  // Keep the elapsed timer ticking while recording.
  setInterval(() => {
    if (status?.phase === "recording") refreshAll();
  }, 250);

  await refreshAll();
}

init();
