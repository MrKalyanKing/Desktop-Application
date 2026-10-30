# KS Vision keyboard shortcuts

All global shortcuts are registered once, in Rust (`ks-vision-backend/src-tauri/src/modules/shortcuts`), when the app starts, and stay registered until it quits. No click is needed. The frontend only reacts to `ks://shortcut` events.

## Shortcut map

| Key | Action |
|---|---|
| **F2** | Show the overlay and focus the question input. Saves the app you were in, so Esc can return to it. |
| **Ctrl+Shift+S** | Same as F2. Note: this takes "Save As" away from every other app while KS Vision runs. |
| **F6** | Capture the full (primary) screen and send it to AI. |
| **F7** | Capture the active app window (its visible frame) and send it to AI. |
| **F8** | Open region selection (see below), then capture the region and send it to AI. |
| **F9** | Long-page capture: Page Down + stitch inside the app that is in front, then send it to AI. |
| **F10** | Voice capture on/off. |
| **F11** | Audio source: Mic → System → Both → Mic. |
| **Ctrl+Shift+E** | Show/hide the overlay without taking focus. |
| **Ctrl+Shift+P** | Quit KS Vision completely. No hidden window, tray icon or background process remains. |
| **Ctrl+Alt+K** | **Start KS Vision** when it is not running. This is a Windows Start Menu shortcut key (see below). If it is already running, the overlay is shown instead of starting a second copy. |
| **F3** | Stop every answer that is being generated (typed, screenshot, voice). Recording keeps running. |
| **F4** | Copy the latest completed answer. |
| **F5** | Ask the last submitted question again. |
| **Alt+↑ ↓ ← →** | Move the overlay 24 px. It stays inside the monitor's work area and can cross to another monitor. |

Every key above is active the whole time KS Vision runs, whether the overlay is shown or hidden. F12 is not used because Windows reserves it (see below). Ctrl+Shift+E does its job.

### Starting the app with a key

A closed app cannot listen for keys, so **Ctrl+Alt+K** belongs to a Start Menu shortcut (`%APPDATA%\Microsoft\Windows\Start Menu\Programs\KS Vision.lnk`). Windows itself handles that key and launches `ks-vision-backend\src-tauri\target\release\app.exe`. The overlay opens on launch, unless **Start minimized** is on in Settings.

- After changing the code, rebuild with `npx tauri build --no-bundle` (in `ks-vision-backend`) so the shortcut runs the latest version.
- To change the start key, right-click the shortcut → Properties → **Shortcut key**.

### Inside the overlay (only when it has focus)

| Key | Action |
|---|---|
| Enter / Shift+Enter | Send the question / new line |
| Esc | Leave the input and return focus to the app you were in before F2. This is best-effort: Windows can refuse, and you then see "Couldn't return focus". |
| Esc twice | Hide the overlay (when focus could not be returned) |
| Alt+F4 | Hide the overlay |
| ↑ ↓ Home End | Scroll the answer when the input is **empty** or the answer pane has focus. With text in the input they move the cursor as usual. |
| Page Up / Page Down | Always scroll the answer |

Arrow keys are never global shortcuts. When the overlay does not have focus, they go to your app.

### Region selection (F8)

The selector opens on the monitor that holds the overlay, with a centred rectangle.

- Arrows move the rectangle 20 px, and Shift+Arrows resize it.
- Hold **Ctrl** with either for a 2 px fine step.
- Enter captures, and Esc cancels.
- Mouse drag still works.

Coordinates are converted to physical pixels using that monitor's display scale. Selecting across two monitors is not supported.

## Behaviour details

- **No overlap.** F6–F9 do nothing while a capture or its analysis is running. F10 and F11 do nothing while an audio start or stop is still pending. Each key is also debounced (300 ms; 40 ms for moves).
- **F3 cancels exactly what was running when you pressed it.** Every answer gets an id before it starts. Answers started after the press continue, and late results from cancelled answers are dropped. They never reach the screen, history, auto-copy or the F4 cache. One status line reports "Stopped N answer(s)" or "Nothing to cancel".
- **F4** copies the newest *completed* answer, meaning the one from the most recently started question. Partial, failed and cancelled output is never stored.
- **F7** captures the window's visible frame: title bar and 1 px border, with no drop shadow or invisible resize border. It is a screen capture, so **anything covering the target window appears in the image**. KS Vision itself is excluded from captures. A minimized window or no valid target gives an error, never a full-screen fallback.
- **F9**
  - It only scrolls the app that was in front when you pressed it, and it never takes focus. If KS Vision has focus, press Esc first.
  - It stops at once if focus moves to another window, on F3, or after 35 s.
  - Results are reported as one of:
    - **Ended:** "Captured N screens; scrolling stopped (end of page or the app stopped scrolling)". It never claims the bottom was reached.
    - **Partial:** interrupted. The capture is still sent and marked partial.
    - **Failed:** "Page did not scroll (already at the end, or the app ignores Page Down)". Nothing is sent.
  - A changed frame is evidence that the page scrolled, not proof of where the key press went.
- **F11.** With voice off, it only selects the source for the next F10. With voice on, it switches, then reads the real state back. The "Live · Mic / System / Mic+System" badge always shows what is really recording. An amber dot means that differs from your selection, for example "System audio unavailable — using Mic".
- **Auto-hide** is controlled by `widget.autoHideDuration` in seconds, and 0 (the default) means off. When it is on, the delay is never shorter than 60 s. The overlay never auto-hides while an answer is being generated, a capture is running, voice is recording, or the overlay has focus.

## Known limitations and conflicts

- **F12 is not used.** Windows reserves F12 for debuggers by default (`UserDebuggerHotKey = 0`) and refuses to register it, so show/hide is on **Ctrl+Shift+E** instead. If any other key ever fails to register (for example, another app already owns it), the overlay shows a persistent "<key> unavailable" badge and the console logs `[SHORTCUT] <key> FAILED`. Keys are never remapped automatically.
- **Fn / Fn Lock.** On many laptops the F-row sends brightness or volume commands unless Fn or Fn Lock is used, and Windows never sees an F-key. Software cannot detect or override this. To check what arrives, watch the console for `[SHORTCUT] Fx pressed`: no line means the key never reached Windows.
- **Keys taken from other apps while KS Vision runs:** F2 (rename), F3 (find next), F4, F5 (refresh), F6 (address bar), F7 (caret browsing), F8, F9, F10 (menu bar), F11 (full screen), Ctrl+Shift+S (Save As), Ctrl+Shift+E, Ctrl+Shift+P (VS Code / browser command palette), and Alt+←/→/↑/↓ (browser Back/Forward, Explorer Up). While KS Vision is running, those keys do the KS Vision action in every app.
- **Focus.** Showing with Ctrl+Shift+E or the tray does not activate the overlay. F2, Ctrl+Shift+S and F8 do take focus (on request). When Windows' foreground lock refuses, the app briefly attaches to the foreground thread's input. Restoring focus with Esc can still be refused by Windows.
- **Covered windows.** F7, F9 and F8 capture what is on screen. Another window on top of the target appears in the image.
