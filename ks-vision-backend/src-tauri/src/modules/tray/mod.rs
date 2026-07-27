pub mod menu;
pub mod handlers;

use tauri::{App, tray::TrayIconBuilder};

pub fn init_tray(app: &mut App) -> Result<(), tauri::Error> {
    let handle = app.handle();
    let menu = menu::build_tray_menu(handle)?;
    let icon = app.default_window_icon().cloned();
    
    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .on_menu_event(|app_handle, event| {
            handlers::handle_tray_menu_event(app_handle, event);
        });

    if let Some(ic) = icon {
        builder = builder.icon(ic);
    }

    let _ = builder.build(app)?;
    Ok(())
}
