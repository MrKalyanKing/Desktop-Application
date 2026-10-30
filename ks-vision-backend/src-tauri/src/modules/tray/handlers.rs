use tauri::{AppHandle, menu::MenuEvent, Emitter};
use crate::modules::shortcuts::window::{hide_overlay, show_overlay};

pub fn handle_tray_menu_event(app: &AppHandle, event: MenuEvent) {
    let id = event.id.as_ref();
    match id {
        "show" => {
            let _ = show_overlay(app, true);
        }
        "hide" => {
            let _ = hide_overlay(app);
        }
        "settings" => {
            let _ = app.emit("open-settings", ());
            let _ = show_overlay(app, true);
        }
        "status" => {
            let _ = app.emit("check-ai-status", ());
            let _ = show_overlay(app, true);
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
