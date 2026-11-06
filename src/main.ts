import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Config, ModelView, StatusSnapshot, PerfSnapshot, MemorySnapshot, ReadinessSnapshot, StartupHealthSnapshot, TranscriptEvent, PerfEvent, ModelProgressEvent, ModelDoneEvent } from "./contracts";
import { RecordingClock } from "./recording-clock";
import { headerEl, errorBanner, statusCard, accessibilityCard, transcriptCard, privacyEl } from "./status-view";
import { setupWizardCard, readinessCard } from "./setup-view";
import { modelsCard, type Downloads } from "./models-view";
import { settingsCard } from "./settings-view";
import { perfCard } from "./perf-view";
import "./styles.css";

let status: StatusSnapshot | null = null;
let config: Config | null = null;
let models: ModelView[] = [];
let lastTranscript = "";
let lastError = "";
let perf: PerfSnapshot | null = null;
let memory: MemorySnapshot | null = null;
let readiness: ReadinessSnapshot | null = null;
let startupHealth: StartupHealthSnapshot | null = null;
let refreshInFlight = false;
let refreshRequested = false;
let renderQueued = false;
const recordingClock = new RecordingClock();
let updateRecordingElapsed: ((elapsedMs: number) => void) | null = null;
const downloads: Downloads = {};

const app = document.getElementById("app")!;

async function refreshAll() {
  if (refreshInFlight) {
    refreshRequested = true;
    return;
  }
  refreshInFlight = true;
  try {
    [status, config, models, readiness, startupHealth] = await Promise.all([
      invoke<StatusSnapshot>("get_status"),
      invoke<Config>("get_config"),
      invoke<ModelView[]>("list_models"),
      invoke<ReadinessSnapshot>("get_readiness"),
      invoke<StartupHealthSnapshot>("get_startup_health"),
    ]);
    perf = await invoke<PerfSnapshot>("get_perf_snapshot");
    memory = await invoke<MemorySnapshot>("get_memory_snapshot");
    recordingClock.sync(status);
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
    readiness = await invoke<ReadinessSnapshot>("get_readiness");
    recordingClock.sync(status);
    render();
  } catch (e) {
    console.error("Status refresh failed:", e);
  }
}

async function refreshPerfOnly() {
  try {
    perf = await invoke<PerfSnapshot>("get_perf_snapshot");
    memory = await invoke<MemorySnapshot>("get_memory_snapshot");
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

function downloadModel(modelId: string) {
  lastError = "";
  downloads[modelId] = { downloaded: 0, total: null };
  render();
  void invoke("download_model", { modelId }).catch((e) => {
    delete downloads[modelId];
    setError(`Download failed: ${e}`);
  });
}

async function grantAccessibility() {
  await invoke("request_accessibility").catch(console.error);
  await refreshStatusOnly();
}

async function warmModel(): Promise<string | null> {
  const message = await invoke<string>("warm_up_model").catch((e) => {
    setError(`Could not warm model: ${e}`);
    return null;
  });
  if (message) lastError = "";
  return message;
}

async function warmSetupModel() {
  const message = await warmModel();
  if (message) {
    status = status ? { ...status, message } : status;
    render();
  }
}

async function warmSettingsModel() {
  if (await warmModel()) await refreshStatusOnly();
}

async function unloadModel() {
  await invoke("unload_model_from_memory").catch((e) => setError(`Could not unload model from memory: ${e}`));
  await refreshStatusOnly();
}

async function deleteModel(modelId: string) {
  await invoke("delete_model", { modelId })
    .catch((e) => setError(`Could not delete model: ${e}`))
    .finally(refreshModelsOnly);
}

async function clearProfile() {
  await invoke("clear_perf_samples").catch((e) => setError(`Could not clear profile samples: ${e}`));
  await refreshPerfOnly();
}

function queueRender() {
  if (renderQueued) return;
  renderQueued = true;
  requestAnimationFrame(() => {
    renderQueued = false;
    render();
  });
}

function render() {
  if (!status || !config) {
    app.innerHTML = `<div class="card">Connecting to Spiel…</div>`;
    return;
  }
  const s = status;
  const c = config;

  updateRecordingElapsed = null;
  app.innerHTML = "";
  app.appendChild(headerEl());
  if (lastError) app.appendChild(errorBanner(lastError, () => { lastError = ""; render(); }));
  const statusView = statusCard(s, c, recordingClock.elapsed(s), () => {
    void invoke("toggle_dictation").catch(console.error);
  });
  app.appendChild(statusView.element);
  if (s.phase === "recording") updateRecordingElapsed = statusView.updateElapsed;
  if (readiness?.setup_steps_remaining || startupHealth?.startup_warnings.length) {
    app.appendChild(setupWizardCard(c, readiness, startupHealth, { download: downloadModel, saveConfig, grantAccessibility, warmModel: warmSetupModel }));
  }
  app.appendChild(readinessCard(readiness));
  if (s.needs_accessibility || (s.accessibility_supported && !s.accessibility_trusted)) {
    app.appendChild(accessibilityCard(s, grantAccessibility));
  }
  app.appendChild(transcriptCard(lastTranscript));
  app.appendChild(modelsCard(models, downloads, { download: downloadModel, useModel: (model) => saveConfig({ model }), deleteModel }));
  app.appendChild(settingsCard(c, readiness, { saveConfig, unloadModel, warmModel: warmSettingsModel }));
  if (perf?.enabled) app.appendChild(perfCard(perf, memory, clearProfile));
  app.appendChild(privacyEl());
}

function setError(msg: string) {
  lastError = msg;
  render();
}

async function init() {
  await listen<StatusSnapshot>("status", (e) => {
    status = e.payload;
    recordingClock.sync(status);
    queueRender();
  });
  await listen<TranscriptEvent>("transcript", (e) => {
    lastTranscript = e.payload.text;
    queueRender();
  });
  await listen<PerfEvent>("perf", () => {
    refreshPerfOnly();
  });
  await listen<StartupHealthSnapshot>("startup-health", (e) => {
    startupHealth = e.payload;
    queueRender();
  });
  await listen<ModelProgressEvent>(
    "model-progress",
    (e) => {
      downloads[e.payload.model_id] = {
        downloaded: e.payload.downloaded,
        total: e.payload.total,
      };
      queueRender();
    },
  );
  await listen<ModelDoneEvent>("model-done", (e) => {
    delete downloads[e.payload.model_id];
    if (!e.payload.ok) {
      lastError =
        `Model download failed (${e.payload.outcome}): ${e.payload.error ?? "unknown error"}`;
    } else {
      lastError = "";
    }
    refreshAll();
  });

  // Update only the elapsed label. Rebuilding the complete settings/model DOM
  // ten times per second consumed unnecessary CPU while recording.
  setInterval(() => {
    if (status?.phase === "recording" && updateRecordingElapsed) {
      updateRecordingElapsed(recordingClock.elapsed(status));
    }
  }, 250);

  window.addEventListener("focus", () => { void refreshAll(); });
  await refreshAll();
}

init();
