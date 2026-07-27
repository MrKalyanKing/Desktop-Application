#[tauri::command]
pub fn load_settings_cmd(app: tauri::AppHandle) -> Result<String, String> {
    super::storage::load_settings(&app)
}

#[tauri::command]
pub fn save_settings_cmd(app: tauri::AppHandle, settings: String) -> Result<(), String> {
    super::storage::save_settings(&app, &settings)
}

#[tauri::command]
pub fn set_startup_enabled_cmd(enabled: bool) -> Result<(), String> {
    super::storage::set_startup_enabled(enabled)
}

#[tauri::command]
pub fn is_startup_enabled_cmd() -> Result<bool, String> {
    super::storage::is_startup_enabled()
}
