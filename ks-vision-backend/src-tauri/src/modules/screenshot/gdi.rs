//! Windows GDI capture. DXGI (`screenshots` crate) fails when the overlay uses
//! WDA_EXCLUDEFROMCAPTURE and is the foreground window.

#![cfg(target_os = "windows")]

use image::RgbaImage;

const SM_CXSCREEN: i32 = 0;
const SM_CYSCREEN: i32 = 1;
const SRCCOPY: u32 = 0x00CC0020;
const DIB_RGB_COLORS: u32 = 0;

#[repr(C)]
struct BitmapInfoHeader {
    bi_size: u32,
    bi_width: i32,
    bi_height: i32,
    bi_planes: u16,
    bi_bit_count: u16,
    bi_compression: u32,
    bi_size_image: u32,
    bi_x_pels_per_meter: i32,
    bi_y_pels_per_meter: i32,
    bi_clr_used: u32,
    bi_clr_important: u32,
}

#[link(name = "user32")]
extern "system" {
    fn GetDC(hwnd: isize) -> isize;
    fn ReleaseDC(hwnd: isize, hdc: isize) -> i32;
    fn GetSystemMetrics(index: i32) -> i32;
    fn GetForegroundWindow() -> isize;
    fn GetWindowRect(hwnd: isize, rect: *mut super::capture::RECT) -> i32;
    fn IsWindowVisible(hwnd: isize) -> i32;
    fn GetWindow(hwnd: isize, cmd: u32) -> isize;
    fn GetAncestor(hwnd: isize, flags: u32) -> isize;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateCompatibleDC(hdc: isize) -> isize;
    fn CreateCompatibleBitmap(hdc: isize, w: i32, h: i32) -> isize;
    fn SelectObject(hdc: isize, obj: isize) -> isize;
    fn BitBlt(
        hdc: isize,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        src: isize,
        sx: i32,
        sy: i32,
        rop: u32,
    ) -> i32;
    fn GetDIBits(
        hdc: isize,
        hbmp: isize,
        start: u32,
        lines: u32,
        bits: *mut u8,
        bmi: *mut BitmapInfoHeader,
        usage: u32,
    ) -> i32;
    fn DeleteObject(obj: isize) -> i32;
    fn DeleteDC(hdc: isize) -> i32;
}

fn bitmap_to_rgba(hdc: isize, hbmp: isize, w: i32, h: i32) -> Result<RgbaImage, String> {
    if w <= 0 || h <= 0 {
        return Err("Capture size is empty".into());
    }
    let mut bmi = BitmapInfoHeader {
        bi_size: std::mem::size_of::<BitmapInfoHeader>() as u32,
        bi_width: w,
        bi_height: -h,
        bi_planes: 1,
        bi_bit_count: 32,
        bi_compression: 0,
        bi_size_image: 0,
        bi_x_pels_per_meter: 0,
        bi_y_pels_per_meter: 0,
        bi_clr_used: 0,
        bi_clr_important: 0,
    };
    let mut buf = vec![0u8; (w as usize) * (h as usize) * 4];
    let copied = unsafe { GetDIBits(hdc, hbmp, 0, h as u32, buf.as_mut_ptr(), &mut bmi, DIB_RGB_COLORS) };
    if copied == 0 {
        return Err("GetDIBits failed".into());
    }
    for px in buf.chunks_exact_mut(4) {
        px.swap(0, 2); // BGRA -> RGBA
    }
    RgbaImage::from_raw(w as u32, h as u32, buf).ok_or_else(|| "Invalid bitmap buffer".into())
}

fn capture_screen_rect(x: i32, y: i32, w: i32, h: i32) -> Result<RgbaImage, String> {
    let (w, h) = (w.min(7680).max(1), h.min(4320).max(1));
    unsafe {
        let hdc_screen = GetDC(0);
        if hdc_screen == 0 {
            return Err("GetDC failed".into());
        }
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbmp = CreateCompatibleBitmap(hdc_screen, w, h);
        if hdc_mem == 0 || hbmp == 0 {
            let _ = ReleaseDC(0, hdc_screen);
            return Err("CreateCompatibleBitmap failed".into());
        }
        let old = SelectObject(hdc_mem, hbmp);
        let ok = BitBlt(hdc_mem, 0, 0, w, h, hdc_screen, x, y, SRCCOPY);
        SelectObject(hdc_mem, old);
        let img = if ok != 0 {
            bitmap_to_rgba(hdc_screen, hbmp, w, h)
        } else {
            Err("BitBlt failed".into())
        };
        DeleteObject(hbmp);
        DeleteDC(hdc_mem);
        ReleaseDC(0, hdc_screen);
        img
    }
}

pub fn capture_primary_screen() -> Result<RgbaImage, String> {
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);
        if w <= 0 || h <= 0 {
            return Err("No primary screen metrics".into());
        }
        capture_screen_rect(0, 0, w, h)
    }
}

fn is_overlay(hwnd: isize, skip: isize) -> bool {
    if skip == 0 || hwnd == 0 {
        return false;
    }
    if hwnd == skip {
        return true;
    }
    unsafe { GetAncestor(hwnd, 2) == skip }
}

pub fn capture_foreground_window(skip_hwnd: isize) -> Result<RgbaImage, String> {
    unsafe {
        let mut hwnd = GetForegroundWindow();
        if is_overlay(hwnd, skip_hwnd) {
            let mut next = GetWindow(hwnd, 2);
            let mut hops = 0;
            while next != 0 && hops < 24 {
                if !is_overlay(next, skip_hwnd) && IsWindowVisible(next) != 0 {
                    hwnd = next;
                    break;
                }
                next = GetWindow(next, 2);
                hops += 1;
            }
            if is_overlay(hwnd, skip_hwnd) {
                return capture_primary_screen();
            }
        }
        if hwnd == 0 || IsWindowVisible(hwnd) == 0 {
            return capture_primary_screen();
        }
        let mut rect = super::capture::RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return capture_primary_screen();
        }
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        if w < 64 || h < 64 {
            return capture_primary_screen();
        }
        capture_screen_rect(rect.left, rect.top, w, h)
    }
}

pub fn capture_region_gdi(x: i32, y: i32, w: u32, h: u32) -> Result<RgbaImage, String> {
    if w == 0 || h == 0 {
        return Err("Region is empty".into());
    }
    capture_screen_rect(x, y, w as i32, h as i32)
}
