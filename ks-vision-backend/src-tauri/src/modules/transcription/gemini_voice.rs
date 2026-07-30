//! Direct Gemini multimodal voice answering — NO speech-to-text stage.
//! One complete utterance (≥1s) → one API request.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::Deserialize;

use crate::modules::ai::model_manager::{GeminiModelManager, ModelCapability};
use crate::modules::audio::preprocess;
use crate::modules::transcription::wav::write_wav_to_bytes;

const VOICE_SYSTEM_MIC: &str = "You are an AI Meeting Copilot roleplaying as software engineer Kalyan Badhavath. The user is speaking to you. \
When asked about your projects (Ember360, Banking Simulation, Ride Booking), resume, experience, or technical stack, you MUST answer directly in the first person ('I', 'my') as Kalyan. Do NOT give advice or meta-suggestions (like 'Here is how to answer...'). \
Listen carefully to the FULL audio — including technical terms (NestJS, Java, JavaScript, React.js, PostgreSQL, MongoDB, JWT, TypeORM, Express.js). \
Keep replies extremely direct, concise, and under 8 to 10 lines (maximum 50 to 70 words). Always give a useful reply — never say SKIP.";

const VOICE_SYSTEM_MEETING: &str = "You are an AI Meeting Copilot listening to another person speaking in a meeting with software engineer Kalyan Badhavath. \
When asked to contribute, explain your projects (Ember360, Banking Simulation, Ride Booking), resume, or experience, you MUST answer directly in the first person ('I', 'my') as Kalyan. Do NOT give advice or suggestions on how Kalyan should answer. \
Listen carefully to the FULL clip — preserve technical vocabulary exactly. \
Keep replies extremely direct, concise, and under 8 to 10 lines (maximum 50 to 70 words). Always give a useful reply — never say SKIP.";

const MAX_OUTPUT_TOKENS: u32 = 280;
const MAX_SAMPLES: usize = 12 * 16000; // 12s cap
/// Local gate — junk that would be SKIP'd must never hit the API.
const MIN_API_SAMPLES: usize = 19_200; // 1.2s @ 16k
const MIN_RMS: f32 = 0.008;
const MIN_PEAK: f32 = 0.025;
const MIN_SPEECH_RATIO: f32 = 0.18; // ≥18% of frames must look like speech

static REQUEST_SEQ: AtomicU64 = AtomicU64::new(1);

fn voice_debug_enabled() -> bool {
    matches!(
        std::env::var("VOICE_DEBUG")
            .or_else(|_| std::env::var("KS_VISION_VOICE_DEBUG"))
            .as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}

fn rms_peak(samples: &[f32]) -> (f32, f32) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    let mut sum = 0.0f32;
    let mut peak = 0.0f32;
    for &s in samples {
        sum += s * s;
        peak = peak.max(s.abs());
    }
    ((sum / samples.len() as f32).sqrt(), peak)
}

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

fn speech_frame_ratio(samples: &[f32], frame: usize) -> f32 {
    if samples.is_empty() || frame == 0 {
        return 0.0;
    }
    let mut total = 0usize;
    let mut speech = 0usize;
    let mut floor = 0.006f32;
    for chunk in samples.chunks(frame) {
        total += 1;
        let mut sum = 0.0f32;
        for &s in chunk {
            sum += s * s;
        }
        let rms = (sum / chunk.len() as f32).sqrt();
        if rms > floor * 2.2 {
            speech += 1;
            floor = (floor * 0.995).max(0.002);
        } else {
            floor = floor * 0.97 + rms * 0.03;
        }
    }
    if total == 0 {
        0.0
    } else {
        speech as f32 / total as f32
    }
}

/// Local pre-filter: return Some(reason) if we must NOT call Gemini.
fn local_skip_reason(samples: &[f32], sample_rate: u32) -> Option<&'static str> {
    if samples.len() < MIN_API_SAMPLES {
        return Some("too short");
    }
    let (rms, peak) = rms_peak(samples);
    if rms < MIN_RMS {
        return Some("too quiet (rms)");
    }
    if peak < MIN_PEAK {
        return Some("too quiet (peak)");
    }
    let ratio = speech_frame_ratio(samples, 480);
    if ratio < MIN_SPEECH_RATIO {
        return Some("not enough speech");
    }
    let _ = sample_rate;
    None
}

/// Send one complete utterance to Gemini.
/// Junk / silence is filtered **locally** — no API call for those.
pub async fn answer_from_audio(
    app: tauri::AppHandle,
    samples: &[f32],
    sample_rate: u32,
    from_system_audio: bool,
) -> Result<Option<String>, String> {
    let prepared = preprocess::prepare_for_voice(samples);

    if let Some(reason) = local_skip_reason(&prepared, sample_rate) {
        println!(
            "[LOCAL SKIP] {} ({:.2}s) — not calling Gemini",
            reason,
            prepared.len() as f32 / sample_rate.max(1) as f32
        );
        return Ok(None);
    }

    let api_key = crate::modules::settings::storage::get_gemini_api_key(&app)
        .unwrap_or_else(|| load_api_key());
    if api_key.is_empty() {
        return Err("Gemini API key is not configured.".into());
    }

    let clipped = if prepared.len() > MAX_SAMPLES {
        &prepared[prepared.len() - MAX_SAMPLES..]
    } else {
        prepared.as_slice()
    };

    // Final guard after clip — still no network if somehow empty.
    if let Some(reason) = local_skip_reason(clipped, sample_rate) {
        println!(
            "[LOCAL SKIP] {} ({:.2}s) — not calling Gemini",
            reason,
            clipped.len() as f32 / sample_rate.max(1) as f32
        );
        return Ok(None);
    }

    let duration_s = clipped.len() as f32 / sample_rate as f32;
    let capture_s = samples.len() as f32 / sample_rate as f32;
    let prepared_s = prepared.len() as f32 / sample_rate as f32;
    let (cap_rms, cap_peak) = rms_peak(samples);
    let (prep_rms, prep_peak) = rms_peak(clipped);
    let wav = write_wav_to_bytes(clipped, sample_rate);

    if voice_debug_enabled() {
        println!("========== VOICE DEBUG ==========");
        println!("Capture Duration: {:.3} s ({} samples)", capture_s, samples.len());
        println!(
            "Processed Duration: {:.3} s ({} samples)",
            prepared_s,
            prepared.len()
        );
        println!(
            "Uploaded Duration: {:.3} s ({} samples)",
            duration_s,
            clipped.len()
        );
        println!("Sample Rate: {} Hz mono", sample_rate);
        println!("Bytes Uploaded: {}", wav.len());
        println!(
            "Capture RMS/Peak: {:.4} / {:.4}",
            cap_rms, cap_peak
        );
        println!(
            "Uploaded RMS/Peak: {:.4} / {:.4}",
            prep_rms, prep_peak
        );
        println!(
            "Duration Delta (capture→upload): {:.3} s",
            capture_s - duration_s
        );
        println!("Reasoning Prompt: answer-directly (no separate transcript stage)");
        println!("Expected Speech: (set VOICE_DEBUG_EXPECTED to compare)");
        if let Ok(expected) = std::env::var("VOICE_DEBUG_EXPECTED") {
            if !expected.is_empty() {
                println!("Expected Speech: {}", expected);
            }
        }
    }
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
    let capability = ModelCapability::Audio;
    let mut tried: HashSet<String> = HashSet::new();
    let mut last_err = String::from("No audio-capable Gemini model available");
    let request_id = REQUEST_SEQ.fetch_add(1, Ordering::Relaxed);
    let mut logged_selection = false;

    // Capability-based: only models that support Audio. Never send audio to text-only.
    while let Some(model) = manager.select_for(capability, Some(manager.primary_model()), &tried)
    {
        tried.insert(model.clone());
        if !logged_selection {
            manager.log_selected(&model, capability);
            logged_selection = true;
        }

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
                if let Some(next) = manager.select_for(capability, None, &tried) {
                    manager.log_fallback(&model, &next, "network error");
                }
                continue;
            }
        };

        let status = resp.status();
        if !status.is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            last_err = format!("{} ({}): {}", model, status, err_text);
            eprintln!("[API ERROR] {}", last_err);

            if manager.should_fallback(status, &err_text) {
                let reason = manager.register_failure(&model, capability, status, &err_text);
                if let Some(next) = manager.select_for(capability, None, &tried) {
                    manager.log_fallback(&model, &next, &reason);
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

        manager.record_success(&model, latency_ms as u64);

        if voice_debug_enabled() {
            println!("Final Answer: {}", text);
            println!(
                "Recognized Transcript: (not available — pipeline is audio→answer; no ASR stage)"
            );
            println!("Transcript Match: N/A (enable separate ASR debug probe to measure)");
            println!("=================================");
        }

        // Model should never SKIP (prompt forbids it). If it still does, drop silently —
        // we already paid for this call; local filter should prevent most of these.
        if is_skip_reply(&text) {
            println!(
                "[API SKIP] unexpected SKIP from model for voice-{} — showing nothing",
                request_id
            );
            return Ok(None);
        }

        println!("[API ANSWER] voice-{}: {}", request_id, text);
        return Ok(Some(text));
    }

    Err(format!(
        "No multimodal/audio model available for voice. Last error: {}",
        last_err
    ))
}
