use image::{GenericImageView, DynamicImage};

pub fn find_vertical_overlap(prev_img: &DynamicImage, next_img: &DynamicImage) -> Option<u32> {
    let (w1, h1) = prev_img.dimensions();
    let (w2, h2) = next_img.dimensions();

    if w1 != w2 {
        return None;
    }

    let template_height = 30.min(h1).min(h2);
    let prev_start_y = h1 - template_height;
    
    let mut best_y = None;
    let mut min_sad = u64::MAX;

    // Scan top region of next_img to find a match for the bottom template
    let scan_limit = (h2 - template_height).min(300);

    let step_x = 4;

    for y_offset in 0..scan_limit {
        let mut sad = 0u64;
        
        for ty in 0..template_height {
            let prev_y = prev_start_y + ty;
            let next_y = y_offset + ty;
            
            for x in (0..w1).step_by(step_x) {
                let p1 = prev_img.get_pixel(x, prev_y);
                let p2 = next_img.get_pixel(x, next_y);
                
                sad += (p1[0] as i32 - p2[0] as i32).abs() as u64;
                sad += (p1[1] as i32 - p2[1] as i32).abs() as u64;
                sad += (p1[2] as i32 - p2[2] as i32).abs() as u64;
            }
        }

        if sad < min_sad {
            min_sad = sad;
            best_y = Some(y_offset);
        }
    }

    let total_samples = ((w1 as f32 / step_x as f32).ceil() as u64) * template_height as u64;
    let max_allowed_sad = total_samples * 20; // Allow slight differences due to rendering anti-aliasing

    if min_sad < max_allowed_sad {
        if let Some(by) = best_y {
            // Overlap is the height of prev_img minus matching offset
            // Meaning we keep only top part of prev_img up to display, and stitch next_img at that index
            let overlap_size = h1.saturating_sub(by);
            if overlap_size > 0 && overlap_size < h1 {
                return Some(overlap_size);
            }
        }
    }

    None
}
