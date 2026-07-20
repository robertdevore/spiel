// Prevents an extra console window on Windows in release. Harmless on macOS.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if spiel_lib::run_transcription_worker_from_args(&args) {
        return;
    }
    spiel_lib::run();
}
