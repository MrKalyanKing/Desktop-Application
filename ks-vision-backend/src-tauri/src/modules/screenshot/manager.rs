use tauri::{AppHandle, WebviewWindowBuilder, WebviewUrl, Manager};

pub fn open_region_selector(app: &AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("region_selector") {
        let _ = win.set_focus();
        return Ok(());
    }

    let _window = WebviewWindowBuilder::new(
        app,
        "region_selector",
        WebviewUrl::App("index.html#region-selector".into())
    )
    .title("Region Selection Overlay")
    .fullscreen(true)
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .resizable(false)
    .visible(false)
    .skip_taskbar(true)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}
