use tauri::{Manager, WebviewWindow};
use crate::modules::config::settings;

pub fn init_window_position(window: &WebviewWindow) -> Result<(), String> {
    let app_handle = window.app_handle();
    let s = settings::load_settings(app_handle);
    
    if s.x >= 0 && s.y >= 0 {
        let _ = window.set_position(tauri::PhysicalPosition::new(s.x, s.y));
    } else {
        if let Ok(Some(monitor)) = window.primary_monitor() {
            let size = monitor.size();
            let scale_factor = monitor.scale_factor();
            
            // Default size is 300x150
            let w = (300.0 * scale_factor) as u32;
            let h = (150.0 * scale_factor) as u32;
            let pad_x = (20.0 * scale_factor) as u32;
            let pad_y = (40.0 * scale_factor) as u32;
            
            let x = size.width.saturating_sub(w).saturating_sub(pad_x);
            let y = size.height.saturating_sub(h).saturating_sub(pad_y);
            
            let _ = window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
        }
    }
    Ok(())
}
