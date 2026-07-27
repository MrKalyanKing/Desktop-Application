use image::{DynamicImage, GenericImageView, ImageBuffer, ImageFormat};
use crate::modules::image::overlap::find_vertical_overlap;
use std::io::Cursor;

pub fn stitch_screenshots(images: Vec<DynamicImage>) -> Result<Vec<u8>, String> {
    if images.is_empty() {
        return Err("No images provided for stitching".to_string());
    }

    if images.len() == 1 {
        let mut output_bytes = Vec::new();
        images[0].write_to(&mut Cursor::new(&mut output_bytes), ImageFormat::Png)
            .map_err(|e| format!("Failed to write single image: {}", e))?;
        return Ok(output_bytes);
    }

    let mut stitched = images[0].clone();

    for next_img in images.into_iter().skip(1) {
        let (w1, h1) = stitched.dimensions();
        let (_, h2) = next_img.dimensions();

        let overlap = find_vertical_overlap(&stitched, &next_img);

        if let Some(overlap_height) = overlap {
            let crop_start_y = overlap_height;
            if crop_start_y >= h2 {
                continue;
            }
            let append_height = h2 - crop_start_y;
            let new_height = h1 + append_height;

            let mut canvas = ImageBuffer::new(w1, new_height);
            
            for y in 0..h1 {
                for x in 0..w1 {
                    canvas.put_pixel(x, y, stitched.get_pixel(x, y));
                }
            }

            for y in 0..append_height {
                for x in 0..w1 {
                    let next_y = crop_start_y + y;
                    canvas.put_pixel(x, h1 + y, next_img.get_pixel(x, next_y));
                }
            }

            stitched = DynamicImage::ImageRgba8(canvas);
        } else {
            let new_height = h1 + h2;
            let mut canvas = ImageBuffer::new(w1, new_height);

            for y in 0..h1 {
                for x in 0..w1 {
                    canvas.put_pixel(x, y, stitched.get_pixel(x, y));
                }
            }

            for y in 0..h2 {
                for x in 0..w1 {
                    canvas.put_pixel(x, h1 + y, next_img.get_pixel(x, y));
                }
            }

            stitched = DynamicImage::ImageRgba8(canvas);
        }
    }

    let mut output_bytes = Vec::new();
    stitched.write_to(&mut Cursor::new(&mut output_bytes), ImageFormat::Png)
        .map_err(|e| format!("Failed to write stitched image: {}", e))?;

    Ok(output_bytes)
}
