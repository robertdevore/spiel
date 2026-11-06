import type { Config, ReadinessSnapshot, StartupHealthSnapshot } from "./contracts";
import { fmtBytes } from "./ui";

interface SetupActions {
  download: (modelId: string) => void;
  saveConfig: (patch: Partial<Config>) => void;
  grantAccessibility: () => void;
  warmModel: () => void;
}

export function setupWizardCard(c: Config, readiness: ReadinessSnapshot | null, startupHealth: StartupHealthSnapshot | null, actions: SetupActions): HTMLElement {
  const card = document.createElement("div");
  card.className = "card";
  card.innerHTML = `<p class="section-title">Setup Wizard</p>`;

  const intro = document.createElement("div");
  intro.className = "privacy";
  intro.textContent =
    "This checklist keeps Spiel quiet and dependable on first run: install the right model, confirm permissions, and optionally warm the model path before your first dictation.";
  card.appendChild(intro);

  if (startupHealth?.startup_warnings.length) {
    for (const warning of startupHealth.startup_warnings) {
      const warn = document.createElement("div");
      warn.className = "warn";
      warn.textContent = warning;
      card.appendChild(warn);
    }
  }

  if (readiness && !readiness.current_model_installed) {
    const recommendedModel = readiness.recommended_model;
    const row = document.createElement("div");
    row.className = "model";
    const meta = document.createElement("div");
    meta.className = "meta";
    meta.innerHTML = `<span class="name">Install a speech model</span><span class="note">${readiness.recommended_model_reason}</span>`;
    row.appendChild(meta);
    const btn = document.createElement("button");
    btn.textContent = `Download ${recommendedModel}`;
    btn.onclick = () => actions.download(recommendedModel);
    row.appendChild(btn);
    card.appendChild(row);
  }

  if (readiness && readiness.current_model !== readiness.recommended_model) {
    const recommendedModel = readiness.recommended_model;
    const row = document.createElement("div");
    row.className = "model";
    const meta = document.createElement("div");
    meta.className = "meta";
    meta.innerHTML = `<span class="name">Recommended model</span><span class="note">Current language: ${c.language}. Suggested: ${recommendedModel}.</span>`;
    row.appendChild(meta);
    const btn = document.createElement("button");
    btn.textContent = "Use Recommendation";
    btn.onclick = () => actions.saveConfig({ model: recommendedModel });
    row.appendChild(btn);
    card.appendChild(row);
  }

  if (readiness?.accessibility_supported && !readiness.accessibility_trusted) {
    const row = document.createElement("div");
    row.className = "model";
    const meta = document.createElement("div");
    meta.className = "meta";
    meta.innerHTML = `<span class="name">Grant Accessibility</span><span class="note">Needed for seamless auto-paste. Without it, Spiel falls back to clipboard-only insertion.</span>`;
    row.appendChild(meta);
    const btn = document.createElement("button");
    btn.textContent = "Grant";
    btn.onclick = actions.grantAccessibility;
    row.appendChild(btn);
    card.appendChild(row);
  }

  const warmupRow = document.createElement("div");
  warmupRow.className = "model";
  const warmMeta = document.createElement("div");
  warmMeta.className = "meta";
  warmMeta.innerHTML = `<span class="name">Warm the current model</span><span class="note">Useful after a fresh install or when you want to validate the load path before dictating.</span>`;
  warmupRow.appendChild(warmMeta);
  const warmBtn = document.createElement("button");
  warmBtn.textContent = "Warm Now";
  warmBtn.onclick = actions.warmModel;
  warmupRow.appendChild(warmBtn);
  card.appendChild(warmupRow);

  return card;
}

export function readinessCard(readiness: ReadinessSnapshot | null): HTMLElement {
  const card = document.createElement("div");
  card.className = "card";
  card.innerHTML = `<p class="section-title">Readiness</p>`;

  if (!readiness) {
    const line = document.createElement("div");
    line.className = "privacy";
    line.textContent = "Readiness diagnostics are not available yet.";
    card.appendChild(line);
    return card;
  }

  const row = document.createElement("div");
  row.className = "privacy";
  const modelPath = document.createElement("div");
  modelPath.textContent = `Model directory: ${readiness.model_dir}`;
  const writable = document.createElement("div");
  writable.textContent = `Model directory writable: ${readiness.model_dir_writable ? "yes" : "no"}`;
  const configPath = document.createElement("div");
  configPath.textContent = `Config file: ${readiness.config_file}`;
  const configWritable = document.createElement("div");
  configWritable.textContent = `Config writable: ${readiness.config_writable ? "yes" : "no"}`;
  const hotkey = document.createElement("div");
  hotkey.textContent = `Current hotkey valid: ${readiness.hotkey_valid ? "yes" : "no"}`;
  const activeModel = document.createElement("div");
  activeModel.textContent =
    `Current model: ${readiness.current_model} (${readiness.current_model_status})`;
  const access = document.createElement("div");
  if (!readiness.accessibility_supported) {
    access.textContent = "Accessibility: unsupported on this platform";
  } else {
    access.textContent = `Accessibility trusted: ${readiness.accessibility_trusted ? "yes" : "no"}`;
  }
  const downloading = document.createElement("div");
  downloading.textContent = `Model download active: ${readiness.active_download ? "yes" : "no"}`;
  const storage = document.createElement("div");
  storage.textContent = `Model store: ${readiness.model_store_file_count} files, ${fmtBytes(readiness.model_store_bytes)}`;
  const recommendation = document.createElement("div");
  recommendation.textContent = `Recommended model: ${readiness.recommended_model}`;
  row.append(
    modelPath,
    writable,
    configPath,
    configWritable,
    storage,
    hotkey,
    activeModel,
    recommendation,
    access,
    downloading,
  );
  card.appendChild(row);
  if (readiness.current_model_reason) {
    const reason = document.createElement("div");
    reason.className = "privacy";
    reason.textContent = `Current model detail: ${readiness.current_model_reason}`;
    card.appendChild(reason);
  }
  return card;
}
