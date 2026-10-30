//! Overlay show/hide/focus/move and target-window resolution for shortcuts.

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

use super::ShortcutState;

#[cfg(target_os = "windows")]
mod win32 {
    #[link(name = "user32")]
    extern "system" {
        pub fn GetForegroundWindow() -> isize;
        pub fn SetForegroundWindow(hwnd: isize) -> i32;
        pub fn IsWindow(hwnd: isize) -> i32;
        pub fn IsWindowVisible(hwnd: isize) -> i32;
        pub fn ShowWindow(hwnd: isize, cmd: i32) -> i32;
        pub fn SetWindowPos(
            hwnd: isize,
            after: isize,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> i32;
        pub fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
        pub fn AttachThreadInput(attach: u32, attach_to: u32, on: i32) -> i32;
        pub fn BringWindowToTop(hwnd: isize) -> i32;
    }
    pub const SW_HIDE: i32 = 0;
    pub const SW_SHOWNOACTIVATE: i32 = 4;
    pub const HWND_TOPMOST: isize = -1;
    pub const SWP_NOSIZE: u32 = 0x0001;
    pub const SWP_NOMOVE: u32 = 0x0002;
    pub const SWP_NOACTIVATE: u32 = 0x0010;
}

pub fn foreground_window() -> isize {
    #[cfg(target_os = "windows")]
    unsafe {
        return win32::GetForegroundWindow();
    }
    #[cfg(not(target_os = "windows"))]
    0
}

/// Explicit focus request (F2, region selector). Plain SetForegroundWindow first; if
/// Windows' foreground lock refuses it, briefly attach to the foreground thread's input
/// queue. Never used for F9, which must not change focus.
pub fn bring_to_foreground(hwnd: isize) -> bool {
    #[cfg(target_os = "windows")]
    unsafe {
        if hwnd == 0 {
            return false;
        }
        if win32::SetForegroundWindow(hwnd) != 0 && win32::GetForegroundWindow() == hwnd {
            return true;
        }
        let fg = win32::GetForegroundWindow();
        let fg_thread = win32::GetWindowThreadProcessId(fg, std::ptr::null_mut());
        let our_thread = win32::GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
        let attached = fg_thread != 0
            && fg_thread != our_thread
            && win32::AttachThreadInput(fg_thread, our_thread, 1) != 0;
        win32::BringWindowToTop(hwnd);
        win32::SetForegroundWindow(hwnd);
        if attached {
            win32::AttachThreadInput(fg_thread, our_thread, 0);
        }
        return win32::GetForegroundWindow() == hwnd;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hwnd;
        false
    }
}

/// True for any window owned by this process (overlay, region selector, devtools).
pub fn is_ours(hwnd: isize) -> bool {
    #[cfg(target_os = "windows")]
    {
        if hwnd == 0 {
            return false;
        }
        let mut pid = 0u32;
        unsafe { win32::GetWindowThreadProcessId(hwnd, &mut pid) };
        return pid == std::process::id();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hwnd;
        false
    }
}

pub fn is_valid_target(hwnd: isize) -> bool {
    #[cfg(target_os = "windows")]
    unsafe {
        return hwnd != 0
            && win32::IsWindow(hwnd) != 0
            && win32::IsWindowVisible(hwnd) != 0
            && !is_ours(hwnd);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hwnd;
        false
    }
}

/// Target for window/scroll capture: the foreground app, or — when the overlay holds
/// focus after F2 — the app that was in front before it.
pub fn resolve_target_window(state: &ShortcutState) -> Option<isize> {
    let fg = foreground_window();
    if is_valid_target(fg) {
        return Some(fg);
    }
    let prev = *state.prev_foreground.lock().unwrap();
    if is_valid_target(prev) {
        Some(prev)
    } else {
        None
    }
}

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}

/// Show without activating (F12, tray "Show"), or show and focus (F2).
pub fn show_overlay(app: &AppHandle, focus: bool) -> Result<(), String> {
    let state = app.state::<ShortcutState>();
    let _vis = state.visibility_lock.lock().unwrap();
    let win = main_window(app).ok_or("Main window not found")?;

    if focus {
        let fg = foreground_window();
        if is_valid_target(fg) {
            *state.prev_foreground.lock().unwrap() = fg;
        }
        println!("[FOCUS] focus request; previous foreground={}", fg);
    }

    #[cfg(target_os = "windows")]
    {
        let hwnd = state.main_hwnd();
        if hwnd != 0 {
            unsafe {
                win32::ShowWindow(hwnd, win32::SW_SHOWNOACTIVATE);
                win32::SetWindowPos(
                    hwnd,
                    win32::HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    win32::SWP_NOMOVE | win32::SWP_NOSIZE | win32::SWP_NOACTIVATE,
                );
            }
        } else {
            win.show().map_err(|e| format!("Failed to show window: {e}"))?;
        }
    }
    #[cfg(not(target_os = "windows"))]
    win.show().map_err(|e| format!("Failed to show window: {e}"))?;

    let _ = win.set_skip_taskbar(true);
    let _ = win.set_cursor_visible(false);
    if focus {
        let _ = win.set_focus();
        let hwnd = state.main_hwnd();
        if hwnd != 0 && !bring_to_foreground(hwnd) {
            // Tauri's own set_focus may still land; only report a real refusal.
            std::thread::sleep(std::time::Duration::from_millis(200));
            if foreground_window() != hwnd {
                eprintln!("[FOCUS] Windows refused to focus the overlay");
            }
        }
    }

    let _ = app.emit("ks://visibility", serde_json::json!({ "visible": true }));
    Ok(())
}

pub fn hide_overlay(app: &AppHandle) -> Result<(), String> {
    println!("[OVERLAY] hide");
    let state = app.state::<ShortcutState>();
    let _vis = state.visibility_lock.lock().unwrap();
    let win = main_window(app).ok_or("Main window not found")?;
    // Shown via Win32 (no activation), so Tauri's cached visibility may say "hidden"
    // and skip its own hide. Hide through Win32 as well.
    #[cfg(target_os = "windows")]
    {
        let hwnd = state.main_hwnd();
        if hwnd != 0 {
            unsafe { win32::ShowWindow(hwnd, win32::SW_HIDE) };
        }
    }
    win.hide().map_err(|e| format!("Failed to hide window: {e}"))?;
    let _ = win.set_skip_taskbar(true);
    let _ = app.emit("ks://visibility", serde_json::json!({ "visible": false }));
    Ok(())
}

pub fn is_overlay_visible(app: &AppHandle) -> bool {
    #[cfg(target_os = "windows")]
    {
        let hwnd = app.state::<ShortcutState>().main_hwnd();
        if hwnd != 0 {
            return unsafe { win32::IsWindowVisible(hwnd) != 0 };
        }
    }
    main_window(app)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}

pub fn toggle_overlay(app: &AppHandle) -> Result<bool, String> {
    if is_overlay_visible(app) {
        hide_overlay(app)?;
        Ok(false)
    } else {
        show_overlay(app, false)?;
        Ok(true)
    }
}

/// Move by one step (24 logical px), clamped to the work area of the monitor that
/// contains the window centre after the move (lets the overlay cross monitors).
pub fn move_overlay(app: &AppHandle, dx: i32, dy: i32) -> Result<(), String> {
    let win = main_window(app).ok_or("Main window not found")?;
    let pos = win.outer_position().map_err(|e| e.to_string())?;
    let size = win.outer_size().map_err(|e| e.to_string())?;
    let scale = win.scale_factor().unwrap_or(1.0);
    let step = (24.0 * scale).round() as i32;

    let mut x = pos.x + dx * step;
    let mut y = pos.y + dy * step;
    let (w, h) = (size.width as i32, size.height as i32);
    let (cx, cy) = (x + w / 2, y + h / 2);

    let monitors = win.available_monitors().unwrap_or_default();
    let target = monitors
        .iter()
        .find(|m| {
            let a = m.work_area();
            cx >= a.position.x
                && cx < a.position.x + a.size.width as i32
                && cy >= a.position.y
                && cy < a.position.y + a.size.height as i32
        })
        .cloned()
        .or_else(|| win.current_monitor().ok().flatten());

    if let Some(m) = target {
        let a = m.work_area();
        let (ax, ay) = (a.position.x, a.position.y);
        let (aw, ah) = (a.size.width as i32, a.size.height as i32);
        x = x.clamp(ax, (ax + aw - w).max(ax));
        y = y.clamp(ay, (ay + ah - h).max(ay));
    }

    win.set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())
}

/// Esc in the overlay: give focus back to the app that was in front before F2.
/// Returns "restored", "failed" (Windows refused or the window is gone) or "none".
#[tauri::command]
pub fn release_focus_cmd(app: AppHandle) -> String {
    let state = app.state::<ShortcutState>();
    let prev = std::mem::take(&mut *state.prev_foreground.lock().unwrap());
    let result = if prev == 0 {
        "none"
    } else if !is_valid_target(prev) {
        "failed"
    } else if bring_back(prev) {
        "restored"
    } else {
        "failed"
    };
    println!("[FOCUS] release prev={} -> {}", prev, result);
    result.into()
}

fn bring_back(prev: isize) -> bool {
    #[cfg(target_os = "windows")]
    unsafe {
        return win32::SetForegroundWindow(prev) != 0;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = prev;
        false
    }
}
