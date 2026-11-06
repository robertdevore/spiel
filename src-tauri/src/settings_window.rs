use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub fn show(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.center();
        return;
    }

    match WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("Spiel")
        .inner_size(420.0, 600.0)
        .min_inner_size(380.0, 480.0)
        .resizable(true)
        .fullscreen(false)
        .center()
        .visible(true)
        .build()
    {
        Ok(win) => {
            let _ = win.set_focus();
        }
        Err(error) => {
            eprintln!("[spiel] failed to open settings window: {error}");
        }
    }
}
