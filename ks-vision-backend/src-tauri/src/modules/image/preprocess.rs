use image::{DynamicImage, GenericImageView, ImageFormat};
use std::io::Cursor;

pub fn preprocess_image(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut img = image::load_from_memory(image_bytes)
        .map_err(|e| format!("Failed to load image: {}", e))?;

    // 1. Automatic Scaling (upscale if small to improve OCR accuracy)
    let (width, height) = img.dimensions();
    if width < 1500 {
        let scale_factor = 1500.0 / width as f32;
        let new_width = 1500;
        let new_height = (height as f32 * scale_factor) as u32;
        img = img.resize(new_width, new_height, image::imageops::FilterType::CatmullRom);
    }

    // 2. Grayscale
    let mut gray = img.grayscale();

    // 3. Contrast enhancement
    gray = gray.adjust_contrast(15.0);

    // 4. Sharpen
    gray = gray.unsharpen(1.0, 8);

    // 5. Binarization (threshold filter)
    let luma = gray.to_luma8();
    let mut binary = image::ImageBuffer::new(luma.width(), luma.height());
    let threshold = 127;
    for (x, y, pixel) in luma.enumerate_pixels() {
        let val = pixel[0];
        let bin_val = if val > threshold { 255 } else { 0 };
        binary.put_pixel(x, y, image::Luma([bin_val]));
    }
    let final_img = DynamicImage::ImageLuma8(binary);

    let mut output_bytes = Vec::new();
    final_img.write_to(&mut Cursor::new(&mut output_bytes), ImageFormat::Png)
        .map_err(|e| format!("Failed to write processed image: {}", e))?;

    Ok(output_bytes)
}
