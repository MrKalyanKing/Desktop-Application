use tauri::{AppHandle, menu::MenuEvent, Manager, Emitter};
use crate::modules::window::ghost::{enter_ghost_mode, exit_ghost_mode};

pub fn handle_tray_menu_event(app: &AppHandle, event: MenuEvent) {
    let id = event.id.as_ref();
    match id {
        "show" => {
            if let Some(win) = app.get_webview_window("main") {
                let _ = exit_ghost_mode(&win);
            }
        }
        "hide" => {
            if let Some(win) = app.get_webview_window("main") {
                let _ = enter_ghost_mode(&win);
            }
        }
        "settings" => {
            let _ = app.emit("open-settings", ());
            if let Some(win) = app.get_webview_window("main") {
                let _ = exit_ghost_mode(&win);
            }
        }
        "status" => {
            let _ = app.emit("check-ai-status", ());
            if let Some(win) = app.get_webview_window("main") {
                let _ = exit_ghost_mode(&win);
            }
        }
        "restart" => {
            let _ = app.emit("restart-gemini", ());
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}
