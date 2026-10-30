use std::time::{Duration, Instant};
use base64::{Engine, engine::general_purpose::STANDARD};
use tauri::{AppHandle, Manager, State};
use crate::modules::screenshot::capture;
use crate::modules::shortcuts::{self, ops::OpRegistry, ShortcutState};
use crate::modules::transcription::gemini_voice;

const CAPTURE_TIMEOUT: Duration = Duration::from_secs(8);

fn to_data_url(bytes: Vec<u8>) -> String {
    let b64 = STANDARD.encode(&bytes);
    format!("data:image/jpeg;base64,{}", b64)
}

async fn run_capture<T, F>(timeout: Duration, f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::time::timeout(timeout, tokio::task::spawn_blocking(f))
        .await
        .map_err(|_| "Screenshot timed out. Try again.".to_string())?
        .map_err(|e| format!("Capture task failed: {e}"))?
}

/// F6: primary monitor GDI. Do not call window.hwnd()/hide() from this invoke
/// (that deadlocks the overlay).
#[tauri::command]
pub async fn capture_fullscreen_cmd() -> Result<String, String> {
    let bytes = run_capture(CAPTURE_TIMEOUT, capture::capture_fullscreen_reliable).await?;
    if let Ok(img) = image::load_from_memory(&bytes) {
        println!("[CAPTURE] F6 full screen jpeg={}x{}", img.width(), img.height());
    }
    Ok(to_data_url(bytes))
}

/// F7: the target window's visible frame only. No full-screen fallback.
#[tauri::command]
pub async fn capture_active_window_cmd(
    target_hwnd: Option<i64>,
    state: State<'_, ShortcutState>,
) -> Result<String, String> {
    let target = match target_hwnd {
        Some(h) => h as isize,
        None => shortcuts::window::resolve_target_window(&state)
            .ok_or("No application window to capture")?,
    };
    if !shortcuts::window::is_valid_target(target) {
        return Err("Target window is no longer available".into());
    }
    let bytes = run_capture(CAPTURE_TIMEOUT, move || {
        #[cfg(target_os = "windows")]
        {
            let bounds = crate::modules::screenshot::gdi::window_frame_bounds(target)?;
            let img = crate::modules::screenshot::gdi::capture_window(target)?;
            println!(
                "[CAPTURE] F7 raw={}x{} bounds=({},{})-({},{}) {}x{}",
                img.width(),
                img.height(),
                bounds.left,
                bounds.top,
                bounds.right,
                bounds.bottom,
                bounds.right - bounds.left,
                bounds.bottom - bounds.top
            );
            capture::image_to_jpeg(img)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = target;
            Err("Window capture is only supported on Windows".to_string())
        }
    })
    .await?;
    Ok(to_data_url(bytes))
}

/// F8: physical screen coordinates from the region selector.
#[tauri::command]
pub async fn capture_region_cmd(x: i32, y: i32, w: u32, h: u32) -> Result<String, String> {
    if w < 2 || h < 2 {
        return Err("Selected region is too small".into());
    }
    let bytes = run_capture(CAPTURE_TIMEOUT, move || {
        println!("[CAPTURE] F8 region=({},{}) {}x{}", x, y, w, h);
        capture::capture_region_reliable(x, y, w, h)
    })
    .await?;
    Ok(to_data_url(bytes))
}

#[derive(serde::Serialize)]
pub struct ScrollCaptureResult {
    image: String,
    frames: usize,
    status: &'static str,
    reason: String,
}

/// F9: scroll + stitch inside the target window. Budget 35s (KS_SCROLL_BUDGET_SECS
/// overrides it for testing). The blocking loop cannot be aborted, so on the outer
/// timeout we set the scroll guard and wait for it to exit cooperatively.
#[tauri::command]
pub async fn start_scroll_capture_cmd(
    target_hwnd: i64,
    op_id: u64,
    ops: State<'_, OpRegistry>,
) -> Result<ScrollCaptureResult, String> {
    let guard = ops
        .scroll_guard(op_id)
        .ok_or("Scroll capture was cancelled")?;
    let budget = std::env::var("KS_SCROLL_BUDGET_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(35);
    let deadline = Instant::now() + Duration::from_secs(budget);
    let target = target_hwnd as isize;

    let loop_guard = guard.clone();
    let mut handle = tokio::task::spawn_blocking(move || {
        crate::modules::screenshot::scroll::capture_scroll_and_stitch(target, loop_guard, deadline)
    });

    let joined = match tokio::time::timeout(Duration::from_secs(budget + 5), &mut handle).await {
        Ok(j) => j,
        Err(_) => {
            *guard.lock().unwrap() = true;
            match tokio::time::timeout(Duration::from_secs(1), &mut handle).await {
                Ok(j) => {
                    println!("[SCROLL] exited cleanly after timeout");
                    j
                }
                Err(_) => {
                    println!("[SCROLL] still exiting (stitch work only, no input)");
                    return Err("Scroll capture timed out".into());
                }
            }
        }
    };
    let outcome = match joined.map_err(|e| format!("Scroll task failed: {e}"))? {
        Ok(o) => o,
        Err(e) => {
            println!("[SCROLL] failed: {}", e);
            return Err(e);
        }
    };
    println!("[SCROLL] {} frames={} reason={}", outcome.status, outcome.frames, outcome.reason);

    let jpeg = tokio::task::spawn_blocking(move || {
        let img = image::load_from_memory(&outcome.image_png).map_err(|e| e.to_string())?;
        capture::image_to_jpeg(img.to_rgba8()).map(|j| (j, outcome))
    })
    .await
    .map_err(|e| e.to_string())??;
    let (bytes, outcome) = jpeg;
    Ok(ScrollCaptureResult {
        image: to_data_url(bytes),
        frames: outcome.frames,
        status: outcome.status,
        reason: outcome.reason,
    })
}

/// Async on purpose: creating a window from a sync command deadlocks on Windows.
#[tauri::command]
pub async fn open_region_selector_cmd(app: AppHandle) -> Result<(), String> {
    crate::modules::screenshot::manager::open_region_selector(&app)
}

#[tauri::command]
pub async fn analyze_screenshot_cmd(
    app: AppHandle,
    image_base64: String,
    op_id: Option<u64>,
) -> Result<String, String> {
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
    let token = op_id.and_then(|id| app.state::<OpRegistry>().token(id));
    let analysis = tokio::time::timeout(
        Duration::from_secs(45),
        gemini_voice::answer_from_screenshot(app.clone(), bytes),
    );
    match token {
        // Dropping the analysis future aborts the in-flight HTTP request.
        Some(token) => tokio::select! {
            _ = token.cancelled() => Err("cancelled".into()),
            r = analysis => r.map_err(|_| "Screenshot analysis timed out.".to_string())?,
        },
        None => analysis
            .await
            .map_err(|_| "Screenshot analysis timed out.".to_string())?,
    }
}
