//! Keyboard-first control: the single global-shortcut registry (Rust owns every
//! global key; the frontend only reacts to `ks://shortcut` events).

pub mod ops;
pub mod registry;
pub mod window;

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    FocusInput,
    Cancel,
    CopyAnswer,
    Regenerate,
    CaptureFull,
    CaptureWindow,
    CaptureRegion,
    CaptureScroll,
    ToggleVoice,
    CycleAudio,
    ToggleOverlay,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Quit,
}

impl Action {
    /// Event id sent to the frontend dispatcher.
    pub fn id(self) -> &'static str {
        match self {
            Action::FocusInput => "focus-input",
            Action::Cancel => "cancel",
            Action::CopyAnswer => "copy-answer",
            Action::Regenerate => "regenerate",
            Action::CaptureFull => "capture-full",
            Action::CaptureWindow => "capture-window",
            Action::CaptureRegion => "capture-region",
            Action::CaptureScroll => "capture-scroll",
            Action::ToggleVoice => "toggle-voice",
            Action::CycleAudio => "cycle-audio",
            Action::ToggleOverlay => "toggle-overlay",
            Action::MoveUp => "move-up",
            Action::MoveDown => "move-down",
            Action::MoveLeft => "move-left",
            Action::MoveRight => "move-right",
            Action::Quit => "quit",
        }
    }

    fn debounce_ms(self) -> u128 {
        match self {
            Action::MoveUp | Action::MoveDown | Action::MoveLeft | Action::MoveRight => 40,
            _ => 300,
        }
    }
}

pub struct Binding {
    pub label: &'static str,
    pub modifiers: Option<Modifiers>,
    pub code: Code,
    pub action: Action,
}

impl Binding {
    pub fn shortcut(&self) -> Shortcut {
        Shortcut::new(self.modifiers, self.code)
    }
}

pub const BINDINGS: &[Binding] = &[
    Binding { label: "F2", modifiers: None, code: Code::F2, action: Action::FocusInput },
    // Requested alias for F2. Note: takes "Save As" (Ctrl+Shift+S) from other apps.
    Binding { label: "Ctrl+Shift+S", modifiers: Some(Modifiers::CONTROL.union(Modifiers::SHIFT)), code: Code::KeyS, action: Action::FocusInput },
    Binding { label: "F6", modifiers: None, code: Code::F6, action: Action::CaptureFull },
    Binding { label: "F7", modifiers: None, code: Code::F7, action: Action::CaptureWindow },
    Binding { label: "F8", modifiers: None, code: Code::F8, action: Action::CaptureRegion },
    Binding { label: "F9", modifiers: None, code: Code::F9, action: Action::CaptureScroll },
    Binding { label: "F10", modifiers: None, code: Code::F10, action: Action::ToggleVoice },
    Binding { label: "F11", modifiers: None, code: Code::F11, action: Action::CycleAudio },
    // Show/hide. Replaces F12, which Windows reserves for debuggers.
    // Quit completely (no tray / background process). Start again with the Start Menu key.
    Binding { label: "Ctrl+Shift+P", modifiers: Some(Modifiers::CONTROL.union(Modifiers::SHIFT)), code: Code::KeyP, action: Action::Quit },
    Binding { label: "Ctrl+Shift+E", modifiers: Some(Modifiers::CONTROL.union(Modifiers::SHIFT)), code: Code::KeyE, action: Action::ToggleOverlay },
    Binding { label: "F3", modifiers: None, code: Code::F3, action: Action::Cancel },
    Binding { label: "F4", modifiers: None, code: Code::F4, action: Action::CopyAnswer },
    Binding { label: "F5", modifiers: None, code: Code::F5, action: Action::Regenerate },
    Binding { label: "Alt+Up", modifiers: Some(Modifiers::ALT), code: Code::ArrowUp, action: Action::MoveUp },
    Binding { label: "Alt+Down", modifiers: Some(Modifiers::ALT), code: Code::ArrowDown, action: Action::MoveDown },
    Binding { label: "Alt+Left", modifiers: Some(Modifiers::ALT), code: Code::ArrowLeft, action: Action::MoveLeft },
    Binding { label: "Alt+Right", modifiers: Some(Modifiers::ALT), code: Code::ArrowRight, action: Action::MoveRight },
];

#[derive(Clone, serde::Serialize)]
pub struct KeyStatus {
    pub key: &'static str,
    pub action: &'static str,
    pub registered: bool,
    pub error: Option<String>,
}

pub struct ShortcutState {
    main_hwnd: AtomicIsize,
    pub prev_foreground: Mutex<isize>,
    /// Latest completed answer for F4: (op id, text). Higher op id wins.
    pub last_answer: Mutex<Option<(u64, String)>>,
    pub registered: Mutex<HashSet<&'static str>>,
    pub status: Mutex<HashMap<&'static str, KeyStatus>>,
    pub debounce: Mutex<HashMap<Action, Instant>>,
    /// Serialises show/hide.
    pub visibility_lock: Mutex<()>,
}

impl ShortcutState {
    pub fn new() -> Self {
        Self {
            main_hwnd: AtomicIsize::new(0),
            prev_foreground: Mutex::new(0),
            last_answer: Mutex::new(None),
            registered: Mutex::new(HashSet::new()),
            status: Mutex::new(HashMap::new()),
            debounce: Mutex::new(HashMap::new()),
            visibility_lock: Mutex::new(()),
        }
    }

    pub fn main_hwnd(&self) -> isize {
        self.main_hwnd.load(Ordering::Relaxed)
    }

    /// Store a completed answer for F4 unless it is cancelled, empty, or older (lower op
    /// id = started earlier) than the one already stored.
    pub fn offer_answer(&self, op_id: u64, text: &str, cancelled: bool) -> bool {
        let text = text.trim();
        if text.is_empty() || cancelled {
            return false;
        }
        let mut last = self.last_answer.lock().unwrap();
        if matches!(last.as_ref(), Some((current, _)) if op_id < *current) {
            return false;
        }
        *last = Some((op_id, text.to_string()));
        true
    }

    /// Consistent snapshot of the answer F4 copies.
    pub fn answer_snapshot(&self) -> Option<String> {
        self.last_answer.lock().unwrap().as_ref().map(|(_, t)| t.clone())
    }
}

/// Cache the overlay HWND once at startup (never call window.hwnd() inside invokes).
pub fn init(app: &AppHandle) {
    let state = app.state::<ShortcutState>();
    #[cfg(target_os = "windows")]
    if let Some(win) = app.get_webview_window("main") {
        if let Ok(hwnd) = win.hwnd() {
            state.main_hwnd.store(hwnd.0 as isize, Ordering::Relaxed);
        }
    }
    let _ = state;
    registry::register_all(app);

    // Opened on purpose (Start Menu key / shortcut): show it, unless the user chose
    // "start minimized" in Settings.
    if !start_minimized(app) {
        if let Err(e) = window::show_overlay(app, true) {
            eprintln!("[OVERLAY] show on launch failed: {e}");
        }
    }
}

fn start_minimized(app: &AppHandle) -> bool {
    crate::modules::settings::storage::load_settings(app)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|v| v.get("startupMinimized").and_then(|b| b.as_bool()))
        .unwrap_or(false)
}

#[tauri::command]
pub fn get_shortcut_status_cmd(state: State<'_, ShortcutState>) -> Vec<KeyStatus> {
    let status = state.status.lock().unwrap();
    BINDINGS
        .iter()
        .filter_map(|b| status.get(b.label).cloned())
        .collect()
}

/// Called only from completed-success paths. Cancelled ids and answers older than the
/// stored one are ignored, so F4 always copies the newest completed answer.
#[tauri::command]
pub fn set_last_answer_cmd(
    op_id: u64,
    text: String,
    state: State<'_, ShortcutState>,
    ops: State<'_, ops::OpRegistry>,
) -> bool {
    let accepted = state.offer_answer(op_id, &text, ops.is_cancelled(op_id));
    println!(
        "[ANSWER] op={} chars={} {}",
        op_id,
        text.trim().chars().count(),
        if accepted { "cached for F4" } else { "ignored" }
    );
    accepted
}

#[tauri::command]
pub fn show_overlay_cmd(app: AppHandle, focus: Option<bool>) -> Result<(), String> {
    window::show_overlay(&app, focus.unwrap_or(false))
}

#[tauri::command]
pub fn hide_overlay_cmd(app: AppHandle) -> Result<(), String> {
    window::hide_overlay(&app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_answer_is_not_overwritten_by_older_completion() {
        let s = ShortcutState::new();
        assert!(s.offer_answer(5, "newer", false));
        assert!(!s.offer_answer(3, "older finishing late", false));
        assert_eq!(s.answer_snapshot().as_deref(), Some("newer"));
        assert!(s.offer_answer(7, "newest", false));
        assert_eq!(s.answer_snapshot().as_deref(), Some("newest"));
    }

    #[test]
    fn cancelled_or_empty_answers_never_reach_the_cache() {
        let s = ShortcutState::new();
        assert!(!s.offer_answer(1, "cancelled", true));
        assert!(!s.offer_answer(2, "   ", false));
        assert_eq!(s.answer_snapshot(), None);
    }

    #[test]
    fn bindings_are_unique_and_f12_is_not_used() {
        let mut labels = std::collections::HashSet::new();
        for b in BINDINGS {
            assert!(labels.insert(b.label), "duplicate binding {}", b.label);
        }
        assert!(!labels.contains("F12"), "F12 is reserved by Windows");
        assert!(labels.contains("Ctrl+Shift+E"));
        assert!(labels.contains("Ctrl+Shift+P"));
    }
}
