//! Direct Gemini multimodal voice answering — NO speech-to-text stage.
//! One complete utterance (≥1s) → one API request.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::Deserialize;

use crate::modules::ai::model_manager::GeminiModelManager;
use crate::modules::audio::chunk_optimizer::MIN_UTTERANCE_SAMPLES;
use crate::modules::audio::preprocess;
use crate::modules::transcription::wav::write_wav_to_bytes;

const VOICE_SYSTEM_MIC: &str = "You are an AI Meeting Copilot. The user is speaking to you. \
Listen to the audio and answer directly and concisely (under 80 words). \
If the audio is silence, noise, or not a real question/request, reply with exactly: SKIP";

const VOICE_SYSTEM_MEETING: &str = "You are an AI Meeting Copilot listening to meeting/system audio. \
If there is a clear question or actionable request for the candidate/user, answer it under 80 words. \
If there is no actionable question, reply with exactly: SKIP";

const MAX_OUTPUT_TOKENS: u32 = 220;
const MAX_SAMPLES: usize = 12 * 16000; // 12s cap

static REQUEST_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    #[serde(default)]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Deserialize)]
struct UsageMetadata {
    #[serde(default, rename = "totalTokenCount")]
    total_token_count: Option<u32>,
    #[serde(default, rename = "promptTokenCount")]
    prompt_token_count: Option<u32>,
    #[serde(default, rename = "candidatesTokenCount")]
    candidates_token_count: Option<u32>,
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

/// Send one complete utterance to Gemini. Returns None for SKIP / too-short / silence.
pub async fn answer_from_audio(
    samples: &[f32],
    sample_rate: u32,
    from_system_audio: bool,
) -> Result<Option<String>, String> {
    let prepared = preprocess::prepare_for_voice(samples);
    if prepared.len() < MIN_UTTERANCE_SAMPLES {
        return Ok(None);
    }

    let api_key = load_api_key();
    if api_key.is_empty() {
        return Err("Gemini API key is not configured.".into());
    }

    let clipped = if prepared.len() > MAX_SAMPLES {
        &prepared[prepared.len() - MAX_SAMPLES..]
    } else {
        prepared.as_slice()
    };

    let duration_s = clipped.len() as f32 / sample_rate as f32;
    let wav = write_wav_to_bytes(clipped, sample_rate);
    let b64 = B64.encode(&wav);
    let system = if from_system_audio {
        VOICE_SYSTEM_MEETING
    } else {
        VOICE_SYSTEM_MIC
    };

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
    let request_id = REQUEST_SEQ.fetch_add(1, Ordering::Relaxed);

    while let Some(model) = manager.next_model(None, &tried, true) {
        tried.insert(model.clone());

        println!("[API REQUEST]");
        println!("Model: {}", model);
        println!("Audio Duration: {:.2} s", duration_s);
        println!("Request Id: voice-{}", request_id);

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );

        let t0 = Instant::now();
        let resp = match client.post(&url).json(&payload).send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("network error on {}: {}", model, e);
                eprintln!("[API ERROR] {}", last_err);
                manager.blacklist_model(&model, &last_err, Some(std::time::Duration::from_secs(15)));
                continue;
            }
        };

        let status = resp.status();
        if !status.is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            last_err = format!("{} ({}): {}", model, status, err_text);
            eprintln!("[API ERROR] {}", last_err);

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

        let latency_ms = t0.elapsed().as_millis();
        let tokens = body
            .usage_metadata
            .as_ref()
            .and_then(|u| u.total_token_count)
            .unwrap_or(0);
        let prompt_tokens = body
            .usage_metadata
            .as_ref()
            .and_then(|u| u.prompt_token_count)
            .unwrap_or(0);
        let out_tokens = body
            .usage_metadata
            .as_ref()
            .and_then(|u| u.candidates_token_count)
            .unwrap_or(0);

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

        println!("[API RESPONSE]");
        println!("Model: {}", model);
        println!("Latency: {} ms", latency_ms);
        println!(
            "Tokens Used: {} (prompt={}, out={})",
            tokens, prompt_tokens, out_tokens
        );

        if is_skip_reply(&text) {
            return Ok(None);
        }

        return Ok(Some(text));
    }

    Err(format!(
        "All Gemini voice models exhausted. Last error: {}",
        last_err
    ))
}
