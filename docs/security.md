# Spiel — Security & Privacy

## Privacy Model

Spiel is **local-first and local-only by default**. No data leaves your device.

### What Spiel Stores Locally
- **Transcript text**: Raw transcripts and final cleaned text (only when saved to history)
- **Settings**: App preferences in SQLite (hotkey, engine, mode defaults)
- **Audio files**: Temporary WAV files in system temp directory (not auto-cleaned)
- **Database**: SQLite file in app-local data directory

### What Spiel Never Stores
- Previous clipboard contents
- API keys or secrets
- Network credentials
- Account information
- Telemetry or analytics data
- Audio file contents in the database (paths only, nullable)

### What Spiel Never Does
- Makes network calls
- Calls OpenAI or any cloud API
- Runs local LLMs
- Syncs data to any server
- Uploads audio or transcripts
- Sends analytics or telemetry
- Presses Enter or submits forms after insertion
- Pastes automatically (manual only, unless explicitly enabled in settings)

## Security Review

### Dependencies
| Dependency | Purpose | Risk |
|-----------|---------|------|
| rusqlite (bundled) | Local SQLite | Low — statically linked, no network |
| CPAL | Audio capture | Low — system audio API |
| hound | WAV encoding | Low — file I/O only |
| tauri-plugin-global-shortcut | Hotkey | Low — OS-level shortcut |
| tauri-plugin-clipboard-manager | Clipboard | Low — text read/write only |

### Command Surface
44 Tauri commands exposed to the frontend. All are narrow and explicit:
- No arbitrary shell execution
- No arbitrary filesystem access
- No arbitrary SQL execution
- No process spawning except local Whisper (user-configured binary)

### Local Whisper Safety
- Binary path is user-configured
- Binary is invoked via `std::process::Command` (no shell)
- Arguments passed as `Vec<String>` (no string interpolation)
- Binary and model paths validated before execution
- Model path must exist and be a file

### Data Storage
- SQLite database in app-local data directory
- WAL mode enabled for performance
- Not encrypted (documented limitation)
- Schema versioned for safe migrations

### Permissions
3 capabilities only:
| Capability | Justification |
|-----------|---------------|
| core:default | Tauri runtime |
| global-shortcut:default | Cmd+Option+. recording toggle |
| clipboard-manager:default | Text copy/paste for insertion |

No network, shell, filesystem, updater, or accessibility permissions.

## Known Limitations
- History and settings are not encrypted at rest
- Temp audio files persist until manually cleaned or system reboot
- No secure keychain integration (no API keys to store regardless)
- macOS code signing and notarization not configured
- No content security policy hardening beyond defaults
