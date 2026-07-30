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

#[tauri::command]
pub async fn test_gemini_connection(api_key: String) -> Result<String, String> {
    let trimmed_key = api_key.trim();
    if trimmed_key.is_empty() {
        return Err("API Key cannot be empty".to_string());
    }

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        trimmed_key
    );

    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await;

    match resp {
        Ok(r) => {
            if r.status().is_success() {
                Ok("Connected successfully.".to_string())
            } else if r.status().as_u16() == 400 || r.status().as_u16() == 403 || r.status().as_u16() == 401 {
                Err("Invalid API Key.".to_string())
            } else {
                Err(format!("Unable to reach Gemini. HTTP Status: {}", r.status()))
            }
        }
        Err(_) => {
            Err("Unable to reach Gemini.".to_string())
        }
    }
}

