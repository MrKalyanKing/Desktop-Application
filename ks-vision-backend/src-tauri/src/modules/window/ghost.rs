use tauri::{AppHandle, Manager, WebviewWindow};

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

/// Show the main window, restore taskbar presence, and focus it.
pub fn exit_ghost_mode(window: &WebviewWindow) -> Result<(), String> {
    window
        .show()
        .map_err(|e| format!("Failed to show window: {e}"))?;
    window
        .set_skip_taskbar(false)
        .map_err(|e| format!("Failed to restore taskbar: {e}"))?;
    window
        .set_focus()
        .map_err(|e| format!("Failed to focus window: {e}"))?;
    let _ = window.set_cursor_visible(false);
    Ok(())
}

pub fn toggle_ghost_mode(app: &AppHandle) -> Result<bool, String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;

    let visible = window
        .is_visible()
        .map_err(|e| format!("Failed to read visibility: {e}"))?;

    if visible {
        enter_ghost_mode(&window)?;
        Ok(true) // now in ghost mode
    } else {
        exit_ghost_mode(&window)?;
        Ok(false) // now visible
    }
}

#[tauri::command]
pub fn show_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;
    exit_ghost_mode(&window)
}

#[tauri::command]
pub fn hide_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;
    enter_ghost_mode(&window)
}

#[tauri::command]
pub fn toggle_ghost_mode_cmd(app: AppHandle) -> Result<bool, String> {
    toggle_ghost_mode(&app)
}
