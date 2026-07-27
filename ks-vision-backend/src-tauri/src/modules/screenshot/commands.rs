use base64::{Engine, engine::general_purpose::STANDARD};
use crate::modules::screenshot::capture;
use crate::modules::screenshot::scroll;
use crate::modules::screenshot::manager;

fn to_data_url(bytes: Vec<u8>) -> String {
    let b64 = STANDARD.encode(&bytes);
    format!("data:image/png;base64,{}", b64)
}

#[tauri::command]
pub async fn capture_active_window_cmd() -> Result<String, String> {
    let bytes = capture::capture_active_window()?;
    Ok(to_data_url(bytes))
}

#[tauri::command]
pub async fn capture_fullscreen_cmd() -> Result<String, String> {
    let bytes = capture::capture_fullscreen()?;
    Ok(to_data_url(bytes))
}

#[tauri::command]
pub async fn capture_region_cmd(x: i32, y: i32, w: u32, h: u32) -> Result<String, String> {
    let bytes = capture::capture_region(x, y, w, h)?;
    Ok(to_data_url(bytes))
}

#[tauri::command]
pub async fn start_scroll_capture_cmd() -> Result<String, String> {
    let bytes = scroll::capture_scroll_and_stitch()?;
    Ok(to_data_url(bytes))
}

#[tauri::command]
pub fn open_region_selector_cmd(app: tauri::AppHandle) -> Result<(), String> {
    manager::open_region_selector(&app)
}
