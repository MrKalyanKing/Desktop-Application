//! Direct Gemini multimodal voice answering — NO speech-to-text stage.
//! Optimized audio clips only (VAD + silence trim + duration cap).

use std::collections::HashSet;

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::Deserialize;

use crate::modules::ai::model_manager::GeminiModelManager;
use crate::modules::audio::perf_metrics;
use crate::modules::audio::preprocess;
use crate::modules::transcription::wav::write_wav_to_bytes;

const VOICE_SYSTEM_MIC: &str = "You are an AI Meeting Copilot. The user is speaking to you. \
Listen to the audio and answer directly and concisely (under 80 words). \
If the audio is silence, noise, or not a real question/request, reply with exactly: SKIP";

const VOICE_SYSTEM_MEETING: &str = "You are an AI Meeting Copilot listening to meeting/system audio. \
If there is a clear question or actionable request for the candidate/user, answer it under 80 words. \
If there is no actionable question, reply with exactly: SKIP";

const MAX_OUTPUT_TOKENS: u32 = 220;
const MIN_SAMPLES: usize = 1600;
const MAX_SAMPLES: usize = 12 * 16000; // 12s — token control

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

fn is_skip_reply(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return true;
    }
    let upper = t.to_uppercase();
    upper == "SKIP" || (upper.starts_with("SKIP") && t.len() < 12)
}

/// Send optimized speech audio directly to Gemini for an answer.
/// Returns None when Gemini replies SKIP / empty (no actionable speech).
pub async fn answer_from_audio(
    samples: &[f32],
    sample_rate: u32,
    from_system_audio: bool,
) -> Result<Option<String>, String> {
    let prepared = preprocess::prepare_for_voice(samples);
    if prepared.len() < MIN_SAMPLES {
        return Ok(None);
    }

    let t0 = perf_metrics::now();
    let api_key = load_api_key();
    if api_key.is_empty() {
        return Err("Gemini API key is not configured.".into());
    }

    let clipped = if prepared.len() > MAX_SAMPLES {
        &prepared[prepared.len() - MAX_SAMPLES..]
    } else {
        prepared.as_slice()
    };

    let wav = write_wav_to_bytes(clipped, sample_rate);
    let b64 = B64.encode(&wav);
    let system = if from_system_audio {
        VOICE_SYSTEM_MEETING
    } else {
        VOICE_SYSTEM_MIC
    };

    // REST JSON uses camelCase: inlineData / mimeType (not snake_case).
    let payload = serde_json::json!({
        "systemInstruction": {
            "parts": [{ "text": system }]
        },
        "contents": [{
            "role": "user",
            "parts": [
                { "text": "Respond to this audio." },
                {
                    "inlineData": {
                        "mimeType": "audio/wav",
                        "data": b64
                    }
                }
            ]
        }],
        "generationConfig": {
            "temperature": 0.4,
            "maxOutputTokens": MAX_OUTPUT_TOKENS
        }
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|e| e.to_string())?;

    let manager = GeminiModelManager::global();
    let mut tried: HashSet<String> = HashSet::new();
    let mut last_err = String::from("No Gemini voice model available");

    while let Some(model) = manager.next_model(None, &tried, true) {
        tried.insert(model.clone());
        println!(
            "[GEMINI VOICE] direct audio→answer model={} samples={:.2}s system={} try={}/{}",
            model,
            clipped.len() as f32 / sample_rate as f32,
            from_system_audio,
            tried.len(),
            manager.voice_models_len()
        );

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );

        let resp = match client.post(&url).json(&payload).send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("Gemini voice network error on {}: {}", model, e);
                eprintln!("[GEMINI VOICE ERROR] {}", last_err);
                // Network blips — try next model once, brief soft blacklist
                manager.blacklist_model(&model, &last_err, Some(std::time::Duration::from_secs(15)));
                if let Some(next) = manager.next_model(None, &tried, true) {
                    manager.log_switch(&model, &next, "network error");
                }
                continue;
            }
        };

        let status = resp.status();
        if !status.is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            last_err = format!("Gemini voice failed on {} ({}): {}", model, status, err_text);
            eprintln!("[GEMINI VOICE ERROR] {}", last_err);

            if manager.should_fallback(status, &err_text) {
                let delay = manager.parse_retry_delay(&err_text);
                let reason = GeminiModelManager::switch_reason(status, &err_text);
                manager.blacklist_model(&model, &err_text, delay);
                if let Some(next) = manager.next_model(None, &tried, true) {
                    manager.log_switch(&model, &next, &reason);
                    continue;
                }
            }
            return Err(last_err);
        }

        let body: GeminiResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Gemini voice JSON: {}", e))?;

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

        perf_metrics::log_stage("gemini_voice_direct", &model, t0);

        if is_skip_reply(&text) {
            println!("[GEMINI VOICE] SKIP (no actionable speech) model={}", model);
            return Ok(None);
        }

        println!(
            "[GEMINI VOICE] answer model={} len={}",
            model,
            text.len()
        );
        return Ok(Some(text));
    }

    Err(format!(
        "All Gemini voice models exhausted. Last error: {}",
        last_err
    ))
}
