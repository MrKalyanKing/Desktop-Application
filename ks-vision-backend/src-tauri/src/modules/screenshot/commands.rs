use std::time::Duration;
use base64::{Engine, engine::general_purpose::STANDARD};
use tauri::AppHandle;
use crate::modules::screenshot::capture;
use crate::modules::transcription::gemini_voice;

fn to_data_url(bytes: Vec<u8>) -> String {
    let b64 = STANDARD.encode(&bytes);
    format!("data:image/jpeg;base64,{}", b64)
}

async fn run_capture<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::time::timeout(
        Duration::from_secs(8),
        tokio::task::spawn_blocking(f),
    )
    .await
    .map_err(|_| "Screenshot timed out. Try again.".to_string())?
    .map_err(|e| format!("Capture task failed: {e}"))?
}

/// Same path as Ctrl+Shift+F — primary monitor GDI. Do not call window.hwnd()/hide()
/// from this invoke (that deadlocks the overlay).
async fn capture_screen_jpeg() -> Result<String, String> {
    let bytes = run_capture(capture::capture_fullscreen_reliable).await?;
    Ok(to_data_url(bytes))
}

#[tauri::command]
pub async fn capture_active_window_cmd() -> Result<String, String> {
    capture_screen_jpeg().await
}

#[tauri::command]
pub async fn capture_fullscreen_cmd() -> Result<String, String> {
    capture_screen_jpeg().await
}

#[tauri::command]
pub async fn capture_region_cmd(_x: i32, _y: i32, _w: u32, _h: u32) -> Result<String, String> {
    capture_screen_jpeg().await
}

#[tauri::command]
pub async fn start_scroll_capture_cmd() -> Result<String, String> {
    capture_screen_jpeg().await
}

#[tauri::command]
pub fn open_region_selector_cmd(_app: AppHandle) -> Result<(), String> {
    // Creating a second window from a hotkey invoke freezes the overlay.
    // Region hotkey should call capture_fullscreen from the frontend instead.
    Err("Use Ctrl+Shift+F for screen capture.".into())
}

#[tauri::command]
pub async fn analyze_screenshot_cmd(app: AppHandle, image_base64: String) -> Result<String, String> {
    let base64_str = if image_base64.starts_with("data:image/") {
        image_base64
            .split(',')
            .nth(1)
            .ok_or_else(|| "Invalid image data URL".to_string())?
    } else {
        image_base64.as_str()
    };
    let bytes = STANDARD
        .decode(base64_str)
        .map_err(|e| format!("Failed to decode screenshot: {e}"))?;
    tokio::time::timeout(
        Duration::from_secs(45),
        gemini_voice::answer_from_screenshot(app, bytes),
    )
    .await
    .map_err(|_| "Screenshot analysis timed out.".to_string())?
}
