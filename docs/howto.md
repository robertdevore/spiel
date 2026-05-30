# Spiel — How to Use

A step-by-step guide for getting thoughts out of your head and into text wherever your cursor is.

---

## Quick Start

1. Launch Spiel
1. Press **Ctrl+Shift+Space** to start recording
2. Speak your thoughts
3. Press **Ctrl+Shift+Space** again to stop
5. Click **Transcribe** to convert speech to text
6. Click **Cleanup** to polish the transcript
7. Click **Copy** then **Cmd+V / Ctrl+V** to paste at your cursor

---

## Setup

### Prerequisites

- Rust 1.86+ and Node.js 18+ installed
- macOS 12+ (primary), Windows/Linux supported but not fully tested

### Install & Launch

```bash
cd spiel
npm install
npm run tauri dev
```

---

## The Core Loop

```
hotkey → talk → transcript → cleaned text → inserted at cursor
```

### Step 1: Record Audio

- Press **Ctrl+Shift+Space** to start recording (or click the **Start Recording** button)
- A live elapsed timer appears while recording
- Press **Ctrl+Shift+Space** again to stop (or click **Stop Recording**)
- After stopping, you'll see metadata: filename, duration, file size, sample rate

### Step 2: Transcribe

Choose your transcription engine:

| Engine | Type | Requirements |
|--------|------|-------------|
| **Mock** | Local testing | None — always available |
| **Local Whisper** | Local STT | Configure binary + model paths in Settings |
| **OpenAI** | Cloud STT | API key configured, local-only mode off |

- Click **Transcribe (Mock)** for instant test output
- Click **Transcribe (Local)** to use whisper.cpp on your machine
- Click **Transcribe (OpenAI)** to use OpenAI's Whisper API (requires setup — see below)

The raw transcript appears in the transcript panel.

### Step 3: Clean Up Text

Choose a text mode and cleanup provider:

**Text Modes:**

| Mode | What it does |
|------|-------------|
| **Raw Dictation** | Light punctuation, preserves wording |
| **Clean Notes** | Removes filler words, organizes into paragraphs |
| **AI Prompt** | Structures notes as a prompt for AI tools |
| **Developer Review** | Formats feedback with Issue/Fix/Acceptance sections |
| **Thought Piece** | Structures long thoughts into a draft/outline |

**Cleanup Providers:**

| Provider | Type | Notes |
|----------|------|-------|
| **Basic** | Local, deterministic | Always available, no AI |
| **Mock AI** | Local, simulated | Testing only, clearly labeled |
| **OpenAI** | Cloud AI | Requires API key + local-only mode off |

- Select a mode from the dropdown
- Select a provider
- Click **Run Cleanup**
- The final text appears below the raw transcript

### Step 4: Insert at Cursor

- Review the final text
- Click **Copy** to place it on your clipboard
- Switch to your target app (editor, email, chat, etc.)
- Press **Cmd+V** (macOS) or **Ctrl+V** (Windows/Linux) to paste
- Spiel never presses Enter or submits forms automatically

---

## Using the History Panel

Spiel saves your sessions locally in a SQLite database.

### Save to History

- After transcribing and cleaning up, click **Save to History**
- Entries include: raw text, final text, mode, provider, timestamps

### Browse History

- Open the **History** panel
- See your 10 most recent entries (newest first)
- Click an entry to view its raw text, final text, and metadata

### Manage History

- Click **Copy** or **Insert** to reuse text from a past session
- Click **Delete** to remove a single entry
- Click **Clear All** to delete everything (asks for confirmation)
- Toggle **History Enabled** off to prevent future saves (existing entries preserved)

---

## Using the Settings Panel

Open **Settings** to configure Spiel.

### General

- **Hotkey**: Change the global shortcut (default: Ctrl+Shift+Space)
- **Default Transcription Engine**: mock, local_whisper, or openai
- **Default Text Mode**: Your preferred cleanup mode
- **Default Cleanup Provider**: basic, mock_ai, or openai

### Local Whisper

- **Binary Path**: Path to your whisper.cpp executable
- **Model Path**: Path to a GGML model file (e.g., `ggml-base.en.bin`)
- **Language**: Optional language code (e.g., `en`)
- Click **Validate** to check your configuration

### Privacy & Cloud

- **Local-Only Mode** (default: ON) — Blocks all cloud providers. Turn off to use OpenAI.
- **Cloud Providers Enabled** (default: OFF) — Must be on for OpenAI features.
- **Clipboard Restore Enabled** — Saves and restores previous clipboard content when inserting.
- **Debug Logging** — Off by default. Turn on only for troubleshooting.

### Workflow Automation

All default OFF for safety:

- **Auto-Transcribe After Recording** — Automatically runs transcription when recording stops
- **Auto-Cleanup After Transcription** — Automatically runs cleanup after transcription
- **Auto-Save to History** — Automatically saves after cleanup
- **Auto-Insert After Cleanup** — Automatically copies to clipboard (never presses Enter)

---

## Setting Up OpenAI (Optional)

OpenAI transcription and cleanup are **off by default** and require explicit setup.

### 1. Get an API Key

- Go to [platform.openai.com/api-keys](https://platform.openai.com/api-keys)
- Create a new secret key
- Copy it (starts with `sk-`)

### 2. Disable Local-Only Mode

- Open **Settings**
- Turn off **Local-Only Mode**
- Turn on **Cloud Providers Enabled**

### 3. Configure the API Key

- In Settings, find the **OpenAI API Key** section
- Paste your key and save
- The UI shows "Configured (…abcd)" — the full key is never displayed again
- Click **Validate** to confirm the key works

### 4. Use OpenAI

- Select **OpenAI** as your transcription engine or cleanup provider
- The UI shows a ☁️ cloud badge next to OpenAI options
- Your audio (transcription) or text (cleanup) will be sent to `api.openai.com`

### 5. Remove OpenAI Access

- Click **Delete API Key** in Settings to remove your key
- Turn **Local-Only Mode** back on
- Turn **Cloud Providers Enabled** off

### What Data Is Sent to OpenAI

| Feature | Data Sent | Endpoint |
|---------|----------|----------|
| Transcription | Audio file (WAV) | `api.openai.com/v1/audio/transcriptions` |
| Cleanup | Transcript text only | `api.openai.com/v1/chat/completions` |

- No clipboard contents are sent
- No history entries are sent unless you explicitly run cleanup on them
- No telemetry, analytics, or account data is ever sent

---

## Privacy Notes

- **Local-first by default**: Everything works offline
- **History is local**: Stored in SQLite on your machine, never synced
- **API key is session-only**: Stored in memory, lost on app restart (not written to disk)
- **No accounts**: No login, no registration, no cloud sync
- **No telemetry**: Spiel never phones home
- **Clipboard privacy**: Previous clipboard contents are saved/restored (best-effort) but never stored in history

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| **Ctrl+Shift+Space** | Start / Stop recording |
| **Cmd+V / Ctrl+V** | Paste copied text at cursor |

---

## Where Data Is Stored

| Data | Location |
|------|----------|
| History & Settings | `~/Library/Application Support/com.spiel.app/spiel_history.db` (macOS) |
| Temp audio files | System temp directory (not auto-cleaned) |
| API keys | In memory only (lost on restart) |

---

## Troubleshooting

### "Local-only mode is on. Cloud providers are disabled."

Turn off **Local-Only Mode** in Settings and enable **Cloud Providers Enabled**.

### "OpenAI API key is not configured."

Add your API key in Settings, then click Validate.

### "Hotkey not registered"

Another app may be using Ctrl+Shift+Space. Change the shortcut in Settings.

### "Recording failed"

Check your microphone permissions in System Settings → Privacy → Microphone.

### "Local Whisper not configured"

Set the binary path and model path in Settings → Local Whisper. The binary must be a working whisper.cpp executable.

### "Database unavailable"

This is an internal error. Quit and restart Spiel. Your database is at the path shown above — it can be deleted to start fresh (history will be lost).

---

## Building from Source

```bash
npm install           # Install dependencies
npm run tauri dev     # Development mode with hot reload
npm run tauri build   # Production build (outputs .app/.dmg)
```

Build verification (no GUI required):
```bash
npx tsc --noEmit                     # TypeScript check
npm run build                        # Frontend build
cd src-tauri && cargo check          # Rust check
cd src-tauri && cargo build          # Rust build
cd src-tauri && cargo fmt --check    # Format check
cd src-tauri && cargo test           # Run tests
```
