use tauri::{AppHandle, WebviewWindow};

/// Hide the main window and remove it from the taskbar (ghost mode).
pub fn enter_ghost_mode(window: &WebviewWindow) -> Result<(), String> {
    window
        .hide()
        .map_err(|e| format!("Failed to hide window: {e}"))?;
    window
        .set_skip_taskbar(true)
        .map_err(|e| format!("Failed to skip taskbar: {e}"))?;
    Ok(())
}

/// Show the main window and focus it. Stays out of the taskbar (overlay).
pub fn exit_ghost_mode(window: &WebviewWindow) -> Result<(), String> {
    window
        .show()
        .map_err(|e| format!("Failed to show window: {e}"))?;
    window
        .set_focus()
        .map_err(|e| format!("Failed to focus window: {e}"))?;
    let _ = window.set_cursor_visible(false);
    Ok(())
}

/// Show/hide through the shortcut module so visible-only keys stay in sync.
pub fn toggle_ghost_mode(app: &AppHandle) -> Result<bool, String> {
    // Returns true when the overlay is now hidden (ghost mode).
    crate::modules::shortcuts::window::toggle_overlay(app).map(|visible| !visible)
}

#[tauri::command]
pub fn show_window(app: AppHandle) -> Result<(), String> {
    crate::modules::shortcuts::window::show_overlay(&app, false)
}

#[tauri::command]
pub fn hide_window(app: AppHandle) -> Result<(), String> {
    crate::modules::shortcuts::window::hide_overlay(&app)
}

#[tauri::command]
pub fn toggle_ghost_mode_cmd(app: AppHandle) -> Result<bool, String> {
    toggle_ghost_mode(&app)
}
