use tauri::{AppHandle, menu::MenuEvent, Manager, Emitter};

pub fn handle_tray_menu_event(app: &AppHandle, event: MenuEvent) {
    let id = event.id.as_ref();
    match id {
        "show" => {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }
        "hide" => {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.hide();
            }
        }
        "settings" => {
            let _ = app.emit("open-settings", ());
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }
        "status" => {
            let _ = app.emit("check-ai-status", ());
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }
        "restart" => {
            let _ = app.emit("restart-ollama", ());
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}
