use std::thread::sleep;
use std::time::{Duration, Instant};
use image::{DynamicImage, GenericImageView};
use crate::modules::screenshot::capture::capture_active_window_reliable;
use crate::modules::image::stitch::stitch_screenshots;

#[link(name = "user32")]
extern "system" {
    fn keybd_event(b_vk: u8, b_scan: u8, dw_flags: u32, dw_extra_info: usize);
}

const VK_NEXT: u8 = 0x22; // PageDown key code
const KEYEVENTF_KEYUP: u32 = 0x0002;

fn simulate_page_down() {
    unsafe {
        // Press key
        keybd_event(VK_NEXT, 0, 0, 0);
        // Release key
        keybd_event(VK_NEXT, 0, KEYEVENTF_KEYUP, 0);
    }
}

pub fn capture_scroll_and_stitch() -> Result<Vec<u8>, String> {
    let start_time = Instant::now();
    let timeout = Duration::from_secs(30);
    let max_captures = 15; // Set to 15 to stay within limits comfortably

    let mut images = Vec::new();
    
    // 1. Initial capture
    let init_bytes = capture_active_window_reliable()?;
    let mut last_img = image::load_from_memory(&init_bytes)
        .map_err(|e| format!("Failed to parse initial capture: {}", e))?;
    images.push(last_img.clone());

    for _ in 1..max_captures {
        if start_time.elapsed() >= timeout {
            break;
        }

        // 2. Perform scroll
        simulate_page_down();
        
        // Sleep to allow UI render transitions to complete
        sleep(Duration::from_millis(600));

        // 3. Capture next viewport
        let next_bytes = match capture_active_window_reliable() {
            Ok(bytes) => bytes,
            Err(_) => break, // Stop if active window is closed or moved
        };

        let next_img = image::load_from_memory(&next_bytes)
            .map_err(|e| format!("Failed to parse scrolled viewport: {}", e))?;

        // 4. Compare pixels: check if we reached the bottom (completely identical viewports)
        if are_images_identical(&last_img, &next_img) {
            break;
        }

        images.push(next_img.clone());
        last_img = next_img;
    }

    // 5. Stitch all Collected frames
    stitch_screenshots(images)
}

// Quick pixel comparison to detect bottom boundaries
fn are_images_identical(img1: &DynamicImage, img2: &DynamicImage) -> bool {
    let (w1, h1) = img1.dimensions();
    let (w2, h2) = img2.dimensions();

    if w1 != w2 || h1 != h2 {
        return false;
    }

    // Sample pixels across screen to check similarity
    let step_x = 20;
    let step_y = 20;

    for y in (0..h1).step_by(step_y) {
        for x in (0..w1).step_by(step_x) {
            if img1.get_pixel(x, y) != img2.get_pixel(x, y) {
                return false;
            }
        }
    }

    true
}
