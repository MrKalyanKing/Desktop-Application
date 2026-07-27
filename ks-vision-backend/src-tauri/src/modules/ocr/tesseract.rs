use std::process::Command;
use std::fs;
use std::env;
use crate::modules::image::preprocess::preprocess_image;

pub fn run_ocr(image_bytes: &[u8], preprocess: bool) -> Result<String, String> {
    let processed_bytes = if preprocess {
        preprocess_image(image_bytes)?
    } else {
        image_bytes.to_vec()
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    
    let temp_dir = env::temp_dir();
    let input_path = temp_dir.join(format!("ks_ocr_in_{}.png", now));
    let output_base = temp_dir.join(format!("ks_ocr_out_{}", now));
    let output_txt = temp_dir.join(format!("ks_ocr_out_{}.txt", now));

    fs::write(&input_path, &processed_bytes)
        .map_err(|e| format!("Failed to write temp input image: {}", e))?;

    let output = Command::new("tesseract")
        .arg(&input_path)
        .arg(&output_base)
        .output();

    let _ = fs::remove_file(&input_path);

    match output {
        Ok(out) => {
            if out.status.success() {
                if output_txt.exists() {
                    let text = fs::read_to_string(&output_txt)
                        .map_err(|e| format!("Failed to read OCR output file: {}", e))?;
                    let _ = fs::remove_file(&output_txt);
                    Ok(text)
                } else {
                    Err("Tesseract succeeded but output file was not found".to_string())
                }
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                Err(format!("Tesseract failed: {}", stderr))
            }
        }
        Err(e) => {
            Err(format!(
                "Failed to execute tesseract binary. Make sure Tesseract OCR is installed and available in your system PATH: {}",
                e
            ))
        }
    }
}
