//! F9 long-page capture: PageDown + stitch, confined to one target window.
//!
//! Safety rules:
//! - The target is fixed before the loop starts; focus is never forced.
//! - PageDown is sent only while holding the scroll guard, and stopping takes the same
//!   lock, so nothing is sent after a stop (F3, timeout, focus change).
//! - A changed frame is evidence that the target scrolled, not proof of where the
//!   synthetic key went.

use std::thread::sleep;
use std::time::{Duration, Instant};

use image::{DynamicImage, GenericImageView};

use crate::modules::image::stitch::stitch_screenshots;
use crate::modules::shortcuts::ops::ScrollGuard;

#[link(name = "user32")]
extern "system" {
    fn keybd_event(b_vk: u8, b_scan: u8, dw_flags: u32, dw_extra_info: usize);
    fn GetForegroundWindow() -> isize;
}

const VK_NEXT: u8 = 0x22; // PageDown key code
const KEYEVENTF_KEYUP: u32 = 0x0002;
const MAX_FRAMES: usize = 15;
const SETTLE: Duration = Duration::from_millis(600);
const RECHECK: Duration = Duration::from_millis(400);

pub struct ScrollOutcome {
    pub image_png: Vec<u8>,
    pub frames: usize,
    /// "ended" (no further movement) or "partial" (interrupted).
    pub status: &'static str,
    pub reason: String,
}

enum Stop {
    NoMovement,
    Interrupted(String),
}

fn stopped(guard: &ScrollGuard) -> bool {
    *guard.lock().unwrap()
}

/// Sleep in 50 ms slices so a stop is noticed quickly.
fn wait(total: Duration, guard: &ScrollGuard, deadline: Instant) -> Result<(), Stop> {
    let end = Instant::now() + total;
    while Instant::now() < end {
        if stopped(guard) {
            return Err(Stop::Interrupted("cancelled".into()));
        }
        if Instant::now() >= deadline {
            return Err(Stop::Interrupted("timed out".into()));
        }
        sleep(Duration::from_millis(50));
    }
    Ok(())
}

/// Returns false (and sends nothing) if the guard is already stopped.
fn page_down_guarded(guard: &ScrollGuard, step: usize) -> bool {
    let g = guard.lock().unwrap();
    if *g {
        return false;
    }
    unsafe {
        keybd_event(VK_NEXT, 0, 0, 0);
        keybd_event(VK_NEXT, 0, KEYEVENTF_KEYUP, 0);
    }
    println!("[SCROLL] pagedown {}", step);
    true
}

fn grab(target: isize) -> Result<DynamicImage, String> {
    #[cfg(target_os = "windows")]
    {
        crate::modules::screenshot::gdi::capture_window(target).map(DynamicImage::ImageRgba8)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = target;
        Err("Scroll capture is only supported on Windows".into())
    }
}

pub fn capture_scroll_and_stitch(
    target: isize,
    guard: ScrollGuard,
    deadline: Instant,
) -> Result<ScrollOutcome, String> {
    if unsafe { GetForegroundWindow() } != target {
        return Err("Bring the target app to the front (Esc) and press F9 again".into());
    }

    let mut images = vec![grab(target)?];
    let mut stop: Option<Stop> = None;

    for step in 1..MAX_FRAMES {
        if stopped(&guard) {
            stop = Some(Stop::Interrupted("cancelled".into()));
            break;
        }
        if Instant::now() >= deadline {
            stop = Some(Stop::Interrupted("timed out".into()));
            break;
        }
        if unsafe { GetForegroundWindow() } != target {
            stop = Some(Stop::Interrupted("focus moved to another window".into()));
            break;
        }
        if !page_down_guarded(&guard, step) {
            stop = Some(Stop::Interrupted("cancelled".into()));
            break;
        }
        if let Err(s) = wait(SETTLE, &guard, deadline) {
            stop = Some(s);
            break;
        }

        let mut next = match grab(target) {
            Ok(img) => img,
            Err(e) => {
                stop = Some(Stop::Interrupted(format!("capture failed: {e}")));
                break;
            }
        };
        if stopped(&guard) {
            stop = Some(Stop::Interrupted("cancelled".into()));
            break;
        }

        let last = images.last().unwrap();
        if frames_match(last, &next) {
            // Slow renderers: look once more before concluding nothing moved.
            if let Err(s) = wait(RECHECK, &guard, deadline) {
                stop = Some(s);
                break;
            }
            next = match grab(target) {
                Ok(img) => img,
                Err(e) => {
                    stop = Some(Stop::Interrupted(format!("capture failed: {e}")));
                    break;
                }
            };
            if frames_match(images.last().unwrap(), &next) {
                stop = Some(Stop::NoMovement);
                break;
            }
        }
        images.push(next);
    }

    let frames = images.len();
    println!("[SCROLL] stop after {} frame(s)", frames);

    let (status, reason) = match stop {
        Some(Stop::NoMovement) if frames >= 2 => (
            "ended",
            format!(
                "Captured {frames} screens; scrolling stopped (end of page or the app stopped scrolling)"
            ),
        ),
        Some(Stop::NoMovement) => {
            return Err(
                "Page did not scroll (already at the end, or the app ignores Page Down)".into(),
            )
        }
        Some(Stop::Interrupted(why)) if frames >= 2 => {
            ("partial", format!("Partial capture ({frames} screens): {why}"))
        }
        Some(Stop::Interrupted(why)) => {
            return Err(format!("Scroll capture stopped before a second screen: {why}"))
        }
        None => (
            "partial",
            format!("Partial capture ({frames} screens): reached the {MAX_FRAMES}-screen limit"),
        ),
    };

    Ok(ScrollOutcome {
        image_png: stitch_screenshots(images)?,
        frames,
        status,
        reason,
    })
}

/// Sampled comparison tolerating a blinking caret or cursor (<0.5% of samples differ).
fn frames_match(a: &DynamicImage, b: &DynamicImage) -> bool {
    let (w1, h1) = a.dimensions();
    let (w2, h2) = b.dimensions();
    if w1 != w2 || h1 != h2 {
        return false;
    }
    let (mut total, mut diff) = (0u32, 0u32);
    for y in (0..h1).step_by(16) {
        for x in (0..w1).step_by(16) {
            total += 1;
            if a.get_pixel(x, y) != b.get_pixel(x, y) {
                diff += 1;
            }
        }
    }
    total > 0 && (diff as f32) / (total as f32) < 0.005
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use std::sync::{Arc, Mutex};

    #[test]
    fn stopped_guard_sends_no_pagedown() {
        // A stopped guard must refuse before keybd_event is reached.
        let guard: ScrollGuard = Arc::new(Mutex::new(true));
        assert!(!page_down_guarded(&guard, 1));
    }

    #[test]
    fn wait_notices_stop_and_deadline() {
        let guard: ScrollGuard = Arc::new(Mutex::new(true));
        let far = Instant::now() + Duration::from_secs(60);
        assert!(matches!(wait(SETTLE, &guard, far), Err(Stop::Interrupted(r)) if r == "cancelled"));

        let guard: ScrollGuard = Arc::new(Mutex::new(false));
        let past = Instant::now();
        assert!(matches!(wait(SETTLE, &guard, past), Err(Stop::Interrupted(r)) if r == "timed out"));
    }

    #[test]
    fn frames_match_tolerates_caret_but_not_scrolling() {
        let base = RgbaImage::from_pixel(320, 320, Rgba([255, 255, 255, 255]));
        let mut caret = base.clone();
        caret.put_pixel(0, 0, Rgba([0, 0, 0, 255])); // one sampled pixel of 400
        let mut scrolled = base.clone();
        for y in 0..160 {
            for x in 0..320 {
                scrolled.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }
        let (a, b, c) = (
            DynamicImage::ImageRgba8(base),
            DynamicImage::ImageRgba8(caret),
            DynamicImage::ImageRgba8(scrolled),
        );
        assert!(frames_match(&a, &b));
        assert!(!frames_match(&a, &c));
    }
}
