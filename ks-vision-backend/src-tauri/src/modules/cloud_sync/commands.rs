use serde::Serialize;
use tauri::AppHandle;

use super::queue::{flush, load_config, pending_count, save_config};

#[derive(Serialize)]
pub struct CloudStatus {
    pub enabled: bool,
    pub connected: bool,
    pub api_base_url: String,
    pub active_session_id: String,
    pub pending: usize,
}

#[tauri::command]
pub fn cloud_get_status(app: AppHandle) -> CloudStatus {
    let config = load_config(&app);
    CloudStatus {
        enabled: config.enabled,
        connected: !config.access_token.is_empty(),
        api_base_url: config.api_base_url,
        active_session_id: config.active_session_id,
        pending: pending_count(&app),
    }
}

#[tauri::command]
pub fn cloud_configure(app: AppHandle, api_base_url: String, enabled: bool) -> Result<CloudStatus, String> {
    let mut config = load_config(&app);
    config.api_base_url = api_base_url;
    config.enabled = enabled;
    save_config(&app, &config)?;
    Ok(cloud_get_status(app))
}

#[tauri::command]
pub fn cloud_set_session(app: AppHandle, session_id: String) -> Result<CloudStatus, String> {
    let mut config = load_config(&app);
    config.active_session_id = session_id;
    save_config(&app, &config)?;
    Ok(cloud_get_status(app))
}

#[tauri::command]
pub fn cloud_logout(app: AppHandle) -> Result<CloudStatus, String> {
    let mut config = load_config(&app);
    config.access_token.clear();
    config.refresh_token.clear();
    config.device_id.clear();
    save_config(&app, &config)?;
    Ok(cloud_get_status(app))
}

#[tauri::command]
pub async fn cloud_login(
    app: AppHandle,
    email: String,
    password: String,
    device_name: String,
) -> Result<CloudStatus, String> {
    let mut config = load_config(&app);
    let url = format!(
        "{}/auth/desktop/register",
        config.api_base_url.trim_end_matches('/')
    );
    let fingerprint = format!("{}-{}", device_name, std::env::var("COMPUTERNAME").unwrap_or_else(|_| "desktop".into()));
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .json(&serde_json::json!({
            "email": email,
            "password": password,
            "deviceName": device_name,
            "deviceFingerprint": fingerprint
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("desktop login failed: {}", response.status()));
    }
    let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    config.access_token = body["accessToken"].as_str().unwrap_or_default().to_string();
    config.refresh_token = body["refreshToken"].as_str().unwrap_or_default().to_string();
    config.device_id = body["device"]["id"].as_str().unwrap_or_default().to_string();
    config.enabled = true;
    save_config(&app, &config)?;
    Ok(cloud_get_status(app))
}

#[tauri::command]
pub async fn cloud_flush_queue(app: AppHandle) -> Result<usize, String> {
    flush(&app).await
}
