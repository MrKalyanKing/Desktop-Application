use screenshots::Screen;
use std::io::Cursor;
use image::ImageFormat;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[link(name = "user32")]
extern "system" {
    fn GetForegroundWindow() -> isize;
    fn GetWindowRect(hwnd: isize, rect: *mut RECT) -> i32;
}

pub fn get_active_window_rect() -> Option<RECT> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == 0 {
            return None;
        }
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        if GetWindowRect(hwnd, &mut rect) != 0 {
            Some(rect)
        } else {
            None
        }
    }
}

fn image_to_png(img: image::RgbaImage) -> Result<Vec<u8>, String> {
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(png)
}

pub fn capture_active_window() -> Result<Vec<u8>, String> {
    let rect = get_active_window_rect().ok_or("Failed to detect active window handle")?;
    let x = rect.left;
    let y = rect.top;
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;

    if width <= 0 || height <= 0 {
        return Err("Foreground window has invalid dimensions".to_string());
    }

    let screens = Screen::all().map_err(|e| e.to_string())?;
    
    let screen = screens.into_iter().find(|s| {
        let display = &s.display_info;
        let sx = display.x;
        let sy = display.y;
        let sw = display.width as i32;
        let sh = display.height as i32;
        x >= sx && x < sx + sw && y >= sy && y < sy + sh
    }).ok_or("Active window is located outside visible displays")?;

    let rx = x - screen.display_info.x;
    let ry = y - screen.display_info.y;
    
    let r_width = width.min(screen.display_info.width as i32 - rx);
    let r_height = height.min(screen.display_info.height as i32 - ry);

    if r_width <= 0 || r_height <= 0 {
        return Err("Foreground window is out of display boundaries".to_string());
    }

    let image = screen.capture_area(rx, ry, r_width as u32, r_height as u32)
        .map_err(|e| e.to_string())?;
    let png = image_to_png(image)?;
    Ok(png)
}

pub fn capture_fullscreen() -> Result<Vec<u8>, String> {
    let screens = Screen::all().map_err(|e| e.to_string())?;
    let screen = screens.into_iter().next().ok_or("No screen displays available")?;
    let image = screen.capture().map_err(|e| e.to_string())?;
    let png = image_to_png(image)?;
    Ok(png)
}

pub fn capture_region(x: i32, y: i32, w: u32, h: u32) -> Result<Vec<u8>, String> {
    if w == 0 || h == 0 {
        return Err("Capture region dimensions must be greater than zero".to_string());
    }

    let screens = Screen::all().map_err(|e| e.to_string())?;
    
    let cx = x + (w as i32 / 2);
    let cy = y + (h as i32 / 2);

    let screen = screens.into_iter().find(|s| {
        let d = &s.display_info;
        cx >= d.x && cx < d.x + d.width as i32 && cy >= d.y && cy < d.y + d.height as i32
    }).ok_or("Selected region center lies outside visible displays")?;

    let rx = x - screen.display_info.x;
    let ry = y - screen.display_info.y;

    let r_width = (w as i32).min(screen.display_info.width as i32 - rx);
    let r_height = (h as i32).min(screen.display_info.height as i32 - ry);

    if r_width <= 0 || r_height <= 0 {
        return Err("Selected region is outside displays boundary".to_string());
    }

    let image = screen.capture_area(rx, ry, r_width as u32, r_height as u32)
        .map_err(|e| e.to_string())?;
    let png = image_to_png(image)?;
    Ok(png)
}
