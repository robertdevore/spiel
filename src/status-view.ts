import type { Config, Phase, StatusSnapshot } from "./contracts";
import { escapeHtml } from "./ui";

const PHASE_LABEL: Record<Phase, string> = {
  idle: "Idle",
  recording: "Recording…",
  transcribing: "Transcribing…",
  inserting: "Inserting…",
  error: "Error",
};

export function headerEl(): HTMLElement {
  const el = document.createElement("header");
  el.innerHTML = `<h1>Spiel</h1><span class="tag">local dictation</span>`;
  return el;
}

export function errorBanner(msg: string, onDismiss: () => void): HTMLElement {
  const el = document.createElement("div");
  el.className = "card error-banner";
  const text = document.createElement("div");
  text.className = "warn";
  text.textContent = msg;
  const dismiss = document.createElement("button");
  dismiss.textContent = "Dismiss";
  dismiss.onclick = onDismiss;
  el.appendChild(text);
  el.appendChild(dismiss);
  return el;
}

export function statusCard(s: StatusSnapshot, c: Config, elapsedMs: number, onToggle: () => void): { element: HTMLElement; updateElapsed: (elapsedMs: number) => void } {
  const card = document.createElement("div");
  card.className = "card";
  const recording = s.phase === "recording";
  const busy = s.phase === "transcribing" || s.phase === "inserting";
  const elapsed = recording ? ` ${(elapsedMs / 1000).toFixed(1)}s` : "";

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
  btn.onclick = onToggle;
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
  return {
    element: card,
    updateElapsed: (ms) => {
      statusText.textContent = `${PHASE_LABEL.recording} ${(ms / 1000).toFixed(1)}s`;
    },
  };
}

export function accessibilityCard(s: StatusSnapshot, onGrant: () => void): HTMLElement {
  const card = document.createElement("div");
  card.className = "card";
  card.innerHTML = `<p class="section-title">Accessibility</p>
    <div class="privacy">Auto-paste needs Accessibility permission so Spiel can press Cmd+V
    in the focused app. ${s.accessibility_trusted ? "Granted ✓" : "Not granted yet."}
    Until then, transcripts are placed on the clipboard for a manual paste.</div>`;
  if (!s.accessibility_trusted) {
    const btn = document.createElement("button");
    btn.textContent = "Grant Accessibility…";
    btn.onclick = onGrant;
    card.appendChild(btn);
  }
  return card;
}

export function transcriptCard(lastTranscript: string): HTMLElement {
  const card = document.createElement("div");
  card.className = "card";
  const body = lastTranscript
    ? `<div class="transcript">${escapeHtml(lastTranscript)}</div>`
    : `<div class="transcript empty">Your last transcript will appear here.</div>`;
  card.innerHTML = `<p class="section-title">Last transcript</p>${body}`;
  return card;
}

export function privacyEl(): HTMLElement {
  const el = document.createElement("div");
  el.className = "privacy";
  el.innerHTML = `Spiel records only while dictating, transcribes on your Mac with Whisper,
    and writes nothing to disk except the model file and this settings file. No audio or
    text ever leaves your device. The only network use is the one-time model download above.`;
  return el;
}
