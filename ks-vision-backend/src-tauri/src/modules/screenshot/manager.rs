use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

/// Open the keyboard region selector covering the monitor that holds the overlay.
/// Must be called from an async command (sync window creation deadlocks on Windows).
pub fn open_region_selector(app: &AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("region_selector") {
        let _ = win.set_focus();
        return Ok(());
    }

    let monitor = app
        .get_webview_window("main")
        .and_then(|w| w.current_monitor().ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or("No monitor found for region selection")?;

    let window = WebviewWindowBuilder::new(
        app,
        "region_selector",
        WebviewUrl::App("index.html#region-selector".into())
    )
    .title("Region Selection Overlay")
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .resizable(false)
    .visible(false)
    .skip_taskbar(true)
    .build()
    .map_err(|e| e.to_string())?;

    let pos = monitor.position();
    let size = monitor.size();
    let _ = window.set_position(PhysicalPosition::new(pos.x, pos.y));
    let _ = window.set_size(PhysicalSize::new(size.width, size.height));
    crate::modules::window::stealth::protect_from_capture(&window);
    let _ = window.show();
    // The selector is keyboard-driven, so it needs focus (explicit user request).
    let _ = window.set_focus();
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        if !crate::modules::shortcuts::window::bring_to_foreground(hwnd.0 as isize) {
            eprintln!("[FOCUS] Windows refused to focus the region selector");
        }
    }

    Ok(())
}
