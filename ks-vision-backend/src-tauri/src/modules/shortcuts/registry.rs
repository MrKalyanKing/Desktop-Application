//! Global-shortcut registration and dispatch.
//!
//! The plugin calls handlers while holding its own shortcut-map lock, and
//! register/unregister take that same lock. Handlers therefore only debounce and log,
//! then hand the action to another thread.

use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState as KeyState};

use super::ops::OpRegistry;
use super::{window, Action, Binding, KeyStatus, ShortcutState, BINDINGS};

pub fn notice(app: &AppHandle, level: &str, message: impl Into<String>) {
    let _ = app.emit(
        "ks://notice",
        serde_json::json!({ "level": level, "message": message.into() }),
    );
}

/// Register every binding once, at startup. Keys stay registered for the app's lifetime.
pub fn register_all(app: &AppHandle) {
    let state = app.state::<ShortcutState>();
    let gs = app.global_shortcut();
    for b in BINDINGS {
        let mut registered = state.registered.lock().unwrap();
        if registered.contains(b.label) || gs.is_registered(b.shortcut()) {
            registered.insert(b.label);
            continue;
        }
        let action = b.action;
        let label = b.label;
        let result = gs.on_shortcut(b.shortcut(), move |app, _shortcut, event| {
            if event.state != KeyState::Pressed {
                return;
            }
            on_pressed(app, label, action);
        });
        let status = match result {
            Ok(()) => {
                registered.insert(b.label);
                println!("[SHORTCUT] {} registered ({})", b.label, b.action.id());
                status_of(b, true, None)
            }
            Err(e) => {
                let err = e.to_string();
                eprintln!("[SHORTCUT] {} FAILED: {}", b.label, err);
                notice(app, "error", format!("{} unavailable: {}", b.label, err));
                status_of(b, false, Some(err))
            }
        };
        drop(registered);
        state.status.lock().unwrap().insert(b.label, status.clone());
        let _ = app.emit("ks://shortcut-status", status);
    }
}

fn status_of(b: &Binding, registered: bool, error: Option<String>) -> KeyStatus {
    KeyStatus {
        key: b.label,
        action: b.action.id(),
        registered,
        error,
    }
}

fn on_pressed(app: &AppHandle, label: &'static str, action: Action) {
    let state = app.state::<ShortcutState>();
    {
        let mut debounce = state.debounce.lock().unwrap();
        let now = Instant::now();
        if let Some(last) = debounce.get(&action) {
            if now.duration_since(*last).as_millis() < action.debounce_ms() {
                return;
            }
        }
        debounce.insert(action, now);
    }
    println!("[SHORTCUT] {} pressed", label);
    let app = app.clone();
    std::thread::spawn(move || dispatch(&app, action));
}

fn emit_action(app: &AppHandle, action: Action, target_hwnd: Option<isize>) {
    let _ = app.emit(
        "ks://shortcut",
        serde_json::json!({ "action": action.id(), "targetHwnd": target_hwnd }),
    );
}

fn dispatch(app: &AppHandle, action: Action) {
    let state = app.state::<ShortcutState>();
    match action {
        Action::ToggleOverlay => {
            if let Err(e) = window::toggle_overlay(app) {
                notice(app, "error", e);
            }
        }
        Action::Quit => {
            println!("[SHORTCUT] quitting KS Vision");
            app.exit(0);
        }
        Action::FocusInput => match window::show_overlay(app, true) {
            Ok(()) => emit_action(app, action, None),
            Err(e) => notice(app, "error", e),
        },
        Action::MoveUp | Action::MoveDown | Action::MoveLeft | Action::MoveRight => {
            let (dx, dy) = match action {
                Action::MoveUp => (0, -1),
                Action::MoveDown => (0, 1),
                Action::MoveLeft => (-1, 0),
                _ => (1, 0),
            };
            if let Err(e) = window::move_overlay(app, dx, dy) {
                notice(app, "error", format!("Could not move overlay: {e}"));
            }
        }
        Action::CopyAnswer => {
            match state.answer_snapshot() {
                None => notice(app, "info", "No completed answer yet"),
                Some(text) => match app.clipboard().write_text(text) {
                    Ok(()) => notice(app, "success", "Answer copied"),
                    Err(e) => notice(app, "error", format!("Copy failed: {e}")),
                },
            }
        }
        Action::Cancel => {
            let ids = app.state::<OpRegistry>().cancel_snapshot();
            println!("[SHORTCUT] F3 cancelled ops {:?}", ids);
            let _ = app.emit("ks://cancelled", serde_json::json!({ "ids": ids }));
            if ids.is_empty() {
                notice(app, "info", "Nothing to cancel");
            } else {
                let n = ids.len();
                notice(
                    app,
                    "info",
                    format!("Stopped {} answer{}", n, if n == 1 { "" } else { "s" }),
                );
            }
        }
        Action::CaptureWindow => match window::resolve_target_window(&state) {
            Some(hwnd) => emit_action(app, action, Some(hwnd)),
            None => notice(app, "error", "No application window to capture"),
        },
        Action::CaptureScroll => {
            // Never force focus: the target must already be in front.
            let fg = window::foreground_window();
            if window::is_valid_target(fg) {
                emit_action(app, action, Some(fg));
            } else {
                println!("[SCROLL] rejected: target not in front");
                notice(
                    app,
                    "error",
                    "Bring the target app to the front (Esc) and press F9 again",
                );
            }
        }
        Action::Regenerate
        | Action::CaptureFull
        | Action::CaptureRegion
        | Action::ToggleVoice
        | Action::CycleAudio => emit_action(app, action, None),
    }
}
