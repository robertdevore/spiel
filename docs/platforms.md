# Spiel — Platform Notes

## macOS (Primary Target)

### Verified
- Tauri v2 runtime ✅
- Global hotkey (Alt+Shift+Space) ✅
- Microphone recording (CPAL) ✅
- Clipboard read/write ✅
- Local Whisper process invocation ✅

### Permissions
- Microphone: macOS prompts on first recording attempt
- Accessibility: Not required (paste is manual via Cmd+V)
- Filesystem: App-local data only (database, settings)
- Network: None required

### Known Limitations
- Code signing not configured — Gatekeeper shows warning
- Notarization not configured
- Hold-to-talk mode not reliable (release detection issue)
- Temp audio files in system temp directory (not auto-cleaned)

### Database/Settings Location
`~/Library/Application Support/com.spiel.app/spiel_history.db`

---

## Windows

### Expected Behavior
- Tauri v2 runtime should work
- Global hotkey should register
- Microphone should work via CPAL
- Clipboard should work via Windows clipboard API
- Local Whisper should work if binary is Windows-compatible

### Known Unverified Areas
- Full build not tested on Windows
- Installer (.msi) not tested
- Hotkey registration behavior unknown
- Microphone device enumeration unknown
- Local Whisper binary path conventions differ (`.exe` extension)

### Configuration Notes
- Local Whisper binary path: use Windows path format (e.g., `C:\tools\whisper-cpp.exe`)
- Model path: use Windows path format
- Database location: `%APPDATA%\com.spiel.app\spiel_history.db`

---

## Linux

### Expected Behavior
- Tauri v2 runtime should work (requires webkit2gtk)
- Global hotkey may have limitations on some window managers
- Microphone should work via CPAL (ALSA/PulseAudio)
- Clipboard should work via xclip/wl-clipboard

### Known Caveats
- **Wayland**: Hotkey registration may not work without additional configuration
- **X11**: Hotkey should work but may conflict with other shortcuts
- **Clipboard**: May require `xclip` (X11) or `wl-clipboard` (Wayland) installed
- **Local Whisper**: Must be compiled for Linux; path conventions differ

### Known Unverified Areas
- Full build not tested on Linux
- AppImage/.deb not tested
- Hotkey behavior on different WMs unknown
- Microphone permissions handled differently (no system prompt)

### Configuration Notes
- Local Whisper binary path: `/usr/local/bin/whisper-cpp` or similar
- Model path: `~/models/ggml-base.en.bin` or similar
- Database location: `~/.local/share/com.spiel.app/spiel_history.db`
