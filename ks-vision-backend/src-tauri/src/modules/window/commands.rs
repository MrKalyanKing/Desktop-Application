use tauri::AppHandle;
use crate::modules::config::settings;

#[tauri::command]
pub fn save_position(app: AppHandle, x: i32, y: i32) -> Result<(), String> {
    let mut s = settings::load_settings(&app);
    s.x = x;
    s.y = y;
    settings::save_settings(&app, &s)
}
