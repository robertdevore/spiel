import type { Config, ReadinessSnapshot } from "./contracts";
import { textField, numberField, toggleField } from "./ui";

interface MemoryProfile {
  id: string;
  label: string;
  summary: string;
  patch: Partial<Config>;
}

interface SettingsActions {
  saveConfig: (patch: Partial<Config>) => void;
  unloadModel: () => void;
  warmModel: () => void;
}

const LANGUAGE_OPTIONS: { value: string; label: string }[] = [
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
    summary: "Tiny English model, unload after each run, 1 thread, 60s cap.",
    patch: { model: "tiny.en", keep_model_loaded: false, transcription_threads: 1, max_seconds: 60 },
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

export function settingsCard(c: Config, readiness: ReadinessSnapshot | null, actions: SettingsActions): HTMLElement {
  const card = document.createElement("div");
  card.className = "card";
  card.innerHTML = `<p class="section-title">Settings</p>`;

  card.appendChild(
    textField("Hotkey", c.hotkey, (v) => actions.saveConfig({ hotkey: v }), "e.g. Cmd+Alt+D"),
  );
  const hkHint = document.createElement("div");
  hkHint.className = "privacy";
  hkHint.textContent =
    "Modifiers + a key, e.g. Cmd+Alt+D or Cmd+Shift+Space (letters/named keys only — '?' won't work). " +
    "Start and stop dictation from the hotkey, menu bar, or this button. Spiel restores focus before pasting so the transcript lands at your cursor.";
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
    if (langInput.value !== c.language) {
      actions.saveConfig({ language: langInput.value });
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
  if (readiness) {
    const langHint = document.createElement("div");
    langHint.className = "privacy";
    langHint.textContent =
      `Recommended for "${c.language}": ${readiness.recommended_model}. ${readiness.recommended_model_reason}`;
    card.appendChild(langHint);
  }

  card.appendChild(
    toggleField("Auto-paste at cursor", c.auto_paste, (v) => actions.saveConfig({ auto_paste: v })),
  );
  card.appendChild(
    toggleField("Restore previous clipboard", c.restore_clipboard, (v) =>
      actions.saveConfig({ restore_clipboard: v }),
    ),
  );
  card.appendChild(
    toggleField("Keep model loaded in memory", c.keep_model_loaded, (v) =>
      actions.saveConfig({ keep_model_loaded: v }),
    ),
  );
  card.appendChild(
    numberField("Transcription threads", c.transcription_threads, 1, 8, (v) =>
      actions.saveConfig({ transcription_threads: v }),
    ),
  );
  card.appendChild(
    numberField("Max recording (seconds)", c.max_seconds, 5, 600, (v) =>
      actions.saveConfig({ max_seconds: v }),
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
    applyBtn.onclick = () => actions.saveConfig(profile.patch);
    row.appendChild(applyBtn);
    card.appendChild(row);
  }

  const unloadBtn = document.createElement("button");
  unloadBtn.textContent = "Unload Model From Memory Now";
  unloadBtn.onclick = actions.unloadModel;
  card.appendChild(unloadBtn);
  const warmBtn = document.createElement("button");
  warmBtn.textContent = "Warm Current Model";
  warmBtn.onclick = actions.warmModel;
  card.appendChild(warmBtn);
  const memHint = document.createElement("div");
  memHint.className = "privacy";
  memHint.textContent =
    "For lower memory: use Tiny model, keep model loaded OFF, and set threads to 1-2. " +
    "For lower first-transcription latency: keep model loaded ON and warm the current model after startup.";
  card.appendChild(memHint);
  return card;
}
