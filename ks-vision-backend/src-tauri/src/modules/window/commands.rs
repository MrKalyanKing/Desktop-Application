use tauri::{AppHandle, WebviewWindow};
use crate::modules::config::settings;

#[tauri::command]
pub fn save_position(app: AppHandle, x: i32, y: i32) -> Result<(), String> {
    let mut s = settings::load_settings(&app);
    s.x = x;
    s.y = y;
    settings::save_settings(&app, &s)
}

/// Hide/show the OS cursor. When hidden over a capture-excluded window,
/// screen-share viewers do not see a system pointer; the FE draws a local-only cursor.
#[tauri::command]
pub fn set_os_cursor_visible(window: WebviewWindow, visible: bool) -> Result<(), String> {
    window
        .set_cursor_visible(visible)
        .map_err(|e| format!("Failed to set cursor visibility: {e}"))
}
