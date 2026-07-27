use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;
use crate::modules::ocr::tesseract;
use crate::modules::ocr::detector;
use image::GenericImageView;
use std::time::Instant;

#[derive(Serialize, Debug, Clone)]
pub struct OcrResponse {
    pub text: String,
    pub content_type: String,
    pub language: String,
    pub confidence: u32,
    pub width: u32,
    pub height: u32,
    pub capture_time_ms: u64,
}

#[tauri::command]
pub async fn perform_ocr_cmd(image_base64: String, preprocess: bool) -> Result<OcrResponse, String> {
    let start_time = Instant::now();

    let base64_str = if image_base64.starts_with("data:image/") {
        image_base64.split(',').nth(1).ok_or("Invalid base64 image data format")?
    } else {
        &image_base64
    };

    let bytes = STANDARD.decode(base64_str)
        .map_err(|e| format!("Failed to decode base64 screenshot buffer: {}", e))?;

    let (width, height) = match image::load_from_memory(&bytes) {
        Ok(img) => img.dimensions(),
        Err(_) => (0, 0),
    };

    let text = tesseract::run_ocr(&bytes, preprocess)?;
    let classification = detector::detect_content_type(&text);
    let capture_time_ms = start_time.elapsed().as_millis() as u64;

    Ok(OcrResponse {
        text,
        content_type: classification.content_type,
        language: classification.language,
        confidence: classification.confidence,
        width,
        height,
        capture_time_ms,
    })
}
