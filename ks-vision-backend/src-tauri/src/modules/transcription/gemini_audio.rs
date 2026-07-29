//! Gemini multimodal audio STT — local Whisper fully removed.
//! Sends optimized WAV clips only (never continuous silence).

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::Deserialize;
use crate::modules::ai::model_manager::GeminiModelManager;
use crate::modules::audio::perf_metrics;
use crate::modules::transcription::gemini_service::write_wav_to_bytes;

const STT_SYSTEM: &str = "Transcribe the audio to plain English text only. \
Fix obvious grammar lightly. Output transcript text alone — no labels, quotes, or commentary. \
If silence or unintelligible, output exactly: (silence)";

const MAX_OUTPUT_TOKENS: u32 = 256;
const MIN_SAMPLES: usize = 1600; // 100ms @ 16k
const MAX_SAMPLES: usize = 15 * 16000; // 15s cap — token control

#[derive(Debug, Clone)]
pub struct AudioSttResult {
    pub text: String,
    pub confidence: f32,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
}

#[derive(Deserialize)]
struct GeminiContent {
    parts: Option<Vec<GeminiPart>>,
}

#[derive(Deserialize)]
struct GeminiPart {
    text: Option<String>,
}

fn load_api_key() -> String {
    if let Ok(key) = std::env::var("GEMINI_API_KEY") {
        if !key.is_empty() && key != "YOUR_GEMINI_API_KEY_HERE" {
            return key;
        }
    }
    if let Ok(key) = std::env::var("VITE_GEMINI_API_KEY") {
        if !key.is_empty() && key != "YOUR_GEMINI_API_KEY_HERE" {
            return key;
        }
    }
    for path in [".env", "../.env", "src-tauri/.env", "../src-tauri/.env"] {
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some(pos) = line.find('=') {
                    let key = line[..pos].trim();
                    let value = line[pos + 1..]
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'');
                    if (key == "GEMINI_API_KEY" || key == "VITE_GEMINI_API_KEY")
                        && !value.is_empty()
                        && value != "YOUR_GEMINI_API_KEY_HERE"
                    {
                        return value.to_string();
                    }
                }
            }
        }
    }
    String::new()
}

/// Transcribe an optimized 16 kHz mono float clip via Gemini multimodal audio.
pub async fn transcribe_audio(samples: &[f32], sample_rate: u32) -> Result<AudioSttResult, String> {
    if samples.len() < MIN_SAMPLES {
        return Ok(AudioSttResult {
            text: String::new(),
            confidence: 0.0,
        });
    }

    let t0 = perf_metrics::now();
    let api_key = load_api_key();
    if api_key.is_empty() {
        return Err("Gemini API key is not configured.".into());
    }

    // Cap duration to control multimodal token usage
    let clipped = if samples.len() > MAX_SAMPLES {
        &samples[samples.len() - MAX_SAMPLES..]
    } else {
        samples
    };

    let wav = write_wav_to_bytes(clipped, sample_rate);
    let b64 = B64.encode(&wav);

    let payload = serde_json::json!({
        "systemInstruction": {
            "parts": [{ "text": STT_SYSTEM }]
        },
        "contents": [{
            "role": "user",
            "parts": [
                { "text": "Audio:" },
                {
                    "inline_data": {
                        "mime_type": "audio/wav",
                        "data": b64
                    }
                }
            ]
        }],
        "generationConfig": {
            "temperature": 0.1,
            "maxOutputTokens": MAX_OUTPUT_TOKENS
        }
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|e| e.to_string())?;

    let manager = GeminiModelManager::global();
    let mut attempts = 0usize;

    loop {
        let model = manager.select_model();
        println!(
            "[GEMINI AUDIO STT] model={} samples={} ({:.2}s)",
            model,
            clipped.len(),
            clipped.len() as f32 / sample_rate as f32
        );

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );

        let resp = client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Gemini audio STT network error: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            eprintln!(
                "[GEMINI AUDIO STT ERROR] {}: {}",
                status, err_text
            );

            let should_switch = manager.is_quota_error(status, &err_text)
                || manager.is_model_unavailable(status, &err_text);
            if should_switch {
                let delay = manager.parse_retry_delay(&err_text);
                let reason =
                    GeminiModelManager::switch_reason(status, &err_text);
                manager.blacklist_model(&model, &err_text, delay);
                attempts += 1;
                if attempts < manager.models_len() {
                    let next = manager.select_model_preferring(None);
                    manager.log_switch(&model, &next, &reason);
                    continue;
                }
            }
            return Err(format!(
                "Gemini audio STT failed ({}): {}",
                status, err_text
            ));
        }

        let body: GeminiResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Gemini audio STT JSON: {}", e))?;

        let text = body
            .candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content)
            .and_then(|c| c.parts)
            .and_then(|p| p.into_iter().next())
            .and_then(|p| p.text)
            .unwrap_or_default()
            .trim()
            .to_string();

        perf_metrics::log_stage("gemini_audio_stt", "audio", t0);

        let lower = text.to_lowercase();
        if lower.is_empty()
            || lower == "(silence)"
            || lower == "silence"
            || lower.contains("[silence]")
        {
            return Ok(AudioSttResult {
                text: String::new(),
                confidence: 0.0,
            });
        }

        // Strip accidental labels if model ignores instructions
        let cleaned = text
            .trim_start_matches("TRANSCRIPT:")
            .trim_start_matches("Transcript:")
            .trim()
            .trim_matches('"')
            .to_string();

        let confidence = estimate_confidence(&cleaned, clipped.len() as f32 / sample_rate as f32);
        println!("[GEMINI AUDIO STT] text={}", cleaned);

        return Ok(AudioSttResult {
            text: cleaned,
            confidence,
        });
    }
}

fn estimate_confidence(text: &str, duration_sec: f32) -> f32 {
    let words = text.split_whitespace().count();
    if words == 0 {
        return 0.0;
    }
    let mut c = 0.85f32;
    if words < 2 && duration_sec > 2.0 {
        c -= 0.25;
    }
    if text.contains('[') {
        c -= 0.15;
    }
    c.clamp(0.0, 1.0)
}
