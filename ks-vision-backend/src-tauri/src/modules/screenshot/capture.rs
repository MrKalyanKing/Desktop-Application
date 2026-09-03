use screenshots::Screen;
use std::io::Cursor;
use image::codecs::jpeg::JpegEncoder;
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

fn rgba_to_png(img: image::RgbaImage) -> Result<Vec<u8>, String> {
    image_to_jpeg(img)
}

/// Preferred capture on Windows (GDI). DXGI fallback otherwise.
pub fn capture_active_window_reliable() -> Result<Vec<u8>, String> {
    capture_active_window_reliable_skip(0)
}

pub fn capture_active_window_reliable_skip(skip_hwnd: isize) -> Result<Vec<u8>, String> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(img) = crate::modules::screenshot::gdi::capture_foreground_window(skip_hwnd) {
            return rgba_to_png(img);
        }
    }
    let _ = skip_hwnd;
    capture_active_window()
        .or_else(|_| capture_fullscreen())
        .and_then(|png| {
            let img = image::load_from_memory(&png).map_err(|e| e.to_string())?;
            image_to_jpeg(img.to_rgba8())
        })
}

pub fn capture_fullscreen_reliable() -> Result<Vec<u8>, String> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(img) = crate::modules::screenshot::gdi::capture_primary_screen() {
            return rgba_to_png(img);
        }
    }
    capture_fullscreen().and_then(|png| {
        let img = image::load_from_memory(&png).map_err(|e| e.to_string())?;
        image_to_jpeg(img.to_rgba8())
    })
}

pub fn capture_region_reliable(x: i32, y: i32, w: u32, h: u32) -> Result<Vec<u8>, String> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(img) = crate::modules::screenshot::gdi::capture_region_gdi(x, y, w, h) {
            return rgba_to_png(img);
        }
    }
    capture_region(x, y, w, h).and_then(|png| {
        let img = image::load_from_memory(&png).map_err(|e| e.to_string())?;
        image_to_jpeg(img.to_rgba8())
    })
}

fn image_to_jpeg(img: image::RgbaImage) -> Result<Vec<u8>, String> {
    let dynimg = image::DynamicImage::ImageRgba8(img);
    let dynimg = if dynimg.width() > 1280 || dynimg.height() > 1280 {
        dynimg.resize(1280, 1280, image::imageops::FilterType::Triangle)
    } else {
        dynimg
    };
    let rgb = dynimg.to_rgb8();
    let mut buf = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut buf, 70);
    encoder
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ColorType::Rgb8,
        )
        .map_err(|e| e.to_string())?;
    Ok(buf)
}

/// JPEG (~1280px, q70) for voice multimodal context. No OCR.
pub fn capture_voice_context_jpeg() -> Result<Vec<u8>, String> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(img) = crate::modules::screenshot::gdi::capture_foreground_window(0)
            .or_else(|_| crate::modules::screenshot::gdi::capture_primary_screen())
        {
            return image_to_jpeg(img);
        }
    }
    let png = capture_active_window_reliable()?;
    let dynimg = image::load_from_memory(&png).map_err(|e| e.to_string())?;
    image_to_jpeg(dynimg.to_rgba8())
}
