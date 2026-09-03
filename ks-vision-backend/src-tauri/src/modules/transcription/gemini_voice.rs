//! Gemini multimodal voice: WAV + optional JPEG, streamed tokens to the UI.

use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

use crate::modules::ai::model_manager::{GeminiModelManager, ModelCapability};
use crate::modules::audio::preprocess;
use crate::modules::cognition::scenario_engine::{ScenarioEngine, ScenarioType};
use crate::modules::database::history as conversation_history;
use crate::modules::screenshot::capture::capture_voice_context_jpeg;
use crate::modules::transcription::wav::write_wav_to_bytes;

const MAX_SAMPLES: usize = 12 * 16000;
const MIN_API_SAMPLES: usize = 8_000; // 0.5s @ 16kHz
const MIN_RMS: f32 = 0.004;
const MIN_PEAK: f32 = 0.012;
const MIN_SPEECH_RATIO: f32 = 0.10;

static REQUEST_SEQ: AtomicU64 = AtomicU64::new(1);
static HTTP: OnceLock<Client> = OnceLock::new();
static API_KEY_CACHE: OnceLock<Mutex<String>> = OnceLock::new();
static RECENT_TURNS: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();

const RECENT_TURN_LIMIT: usize = 8;
const TURN_SNIPPET: usize = 500;

fn recent_turns() -> &'static Mutex<VecDeque<String>> {
    RECENT_TURNS.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn clip_turn(text: &str) -> String {
    let t = text.trim();
    if t.chars().count() <= TURN_SNIPPET {
        t.to_string()
    } else {
        t.chars().take(TURN_SNIPPET).collect::<String>() + "…"
    }
}

/// Store the last few Q&A turns so follow-ups stay on the same topic.
pub fn remember_turn(question: Option<&str>, answer: &str) {
    let Ok(mut q) = recent_turns().lock() else {
        return;
    };
    if let Some(host) = question.map(str::trim).filter(|s| !s.is_empty()) {
        q.push_back(format!("Host: {}", clip_turn(host)));
    }
    let ans = answer.trim();
    if !ans.is_empty() {
        q.push_back(format!("KS-Vision: {}", clip_turn(ans)));
    }
    while q.len() > RECENT_TURN_LIMIT {
        q.pop_front();
    }
}

fn format_recent_context(app: &tauri::AppHandle) -> String {
    if let Ok(q) = recent_turns().lock() {
        if !q.is_empty() {
            return format!(
                "Recent turns (follow-ups only; answer the current spoken question):\n{}",
                q.iter().cloned().collect::<Vec<_>>().join("\n")
            );
        }
    }
    if let Ok(hist) = conversation_history::load_history(app, 4) {
        if !hist.is_empty() {
            let lines: Vec<String> = hist
                .iter()
                .map(|m| format!("{}: {}", m.role, clip_turn(&m.content)))
                .collect();
            return format!(
                "Recent turns (follow-ups only; answer the current spoken question):\n{}",
                lines.join("\n")
            );
        }
    }
    String::new()
}

fn http_client() -> &'static Client {
    HTTP.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(45))
            .pool_max_idle_per_host(4)
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

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

fn speech_frame_ratio(samples: &[f32], frame: usize) -> f32 {
    if samples.is_empty() || frame == 0 {
        return 0.0;
    }
    let mut speech = 0usize;
    let mut total = 0usize;
    for chunk in samples.chunks(frame) {
        total += 1;
        let mut sq = 0.0f32;
        for &s in chunk {
            sq += s * s;
        }
        if (sq / chunk.len() as f32).sqrt() > 0.010 {
            speech += 1;
        }
    }
    if total == 0 {
        0.0
    } else {
        speech as f32 / total as f32
    }
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

fn load_api_key_from_disk() -> String {
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
                    if key == "GEMINI_API_KEY" || key == "VITE_GEMINI_API_KEY" {
                        if !value.is_empty() && value != "YOUR_GEMINI_API_KEY_HERE" {
                            return value.to_string();
                        }
                    }
                }
            }
        }
    }
    String::new()
}

fn resolve_api_key(app: &tauri::AppHandle) -> String {
    let cache = API_KEY_CACHE.get_or_init(|| Mutex::new(String::new()));
    {
        let g = cache.lock().unwrap();
        if !g.is_empty() {
            return g.clone();
        }
    }
    let key = crate::modules::settings::storage::get_gemini_api_key(app)
        .filter(|k| !k.is_empty() && k != "YOUR_GEMINI_API_KEY_HERE")
        .unwrap_or_else(load_api_key_from_disk);
    if !key.is_empty() {
        *cache.lock().unwrap() = key.clone();
    }
    key
}

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

fn extract_text(json: &str) -> Option<String> {
    if let Ok(body) = serde_json::from_str::<GeminiResponse>(json) {
        let text = body
            .candidates?
            .into_iter()
            .next()?
            .content?
            .parts?
            .into_iter()
            .filter_map(|p| p.text)
            .collect::<String>();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    } else {
        None
    }
}

/// Process utterance: JPEG in parallel with WAV, stream Gemini tokens.
pub async fn answer_from_audio(
    app: tauri::AppHandle,
    samples: &[f32],
    sample_rate: u32,
    from_system_audio: bool,
    cancel: CancellationToken,
) -> Result<Option<String>, String> {
    let prepared = preprocess::prepare_for_voice(samples);

    if let Some(reason) = local_skip_reason(&prepared, sample_rate) {
        if voice_debug_enabled() {
            println!("[LOCAL SKIP] {} ({:.2}s)", reason, prepared.len() as f32 / sample_rate.max(1) as f32);
        }
        return Ok(None);
    }

    let api_key = resolve_api_key(&app);
    if api_key.is_empty() {
        return Err("Gemini API key is not configured.".into());
    }

    let clipped = if prepared.len() > MAX_SAMPLES {
        &prepared[prepared.len() - MAX_SAMPLES..]
    } else {
        prepared.as_slice()
    };

    if let Some(reason) = local_skip_reason(clipped, sample_rate) {
        if voice_debug_enabled() {
            println!("[LOCAL SKIP] {}", reason);
        }
        return Ok(None);
    }

    let jpeg_task = tokio::task::spawn_blocking(capture_voice_context_jpeg);
    let wav = write_wav_to_bytes(clipped, sample_rate);
    let b64_audio = B64.encode(&wav);
    let jpeg = jpeg_task.await.ok().and_then(|r| r.ok());
    let has_screen = jpeg.is_some();
    let b64_jpeg = jpeg.map(|j| B64.encode(&j));

    let system_instruction = ScenarioEngine::live_system_prompt(from_system_audio);
    let max_tokens = ScenarioType::General.max_output_tokens();
    let recent = format_recent_context(&app);

    let mut listen = String::from(
        "Listen to this audio and answer the current spoken question using KS-Vision principles. Match length and example source to the situation. Use the screenshot only if they ask about what is on screen.",
    );
    if !recent.is_empty() {
        listen.push('\n');
        listen.push_str(&recent);
    }

    let mut parts = vec![serde_json::json!({ "text": listen })];
    if let Some(img) = b64_jpeg {
        parts.push(serde_json::json!({
            "inlineData": { "mimeType": "image/jpeg", "data": img }
        }));
    }
    parts.push(serde_json::json!({
        "inlineData": { "mimeType": "audio/wav", "data": b64_audio }
    }));

    let payload = serde_json::json!({
        "systemInstruction": {
            "parts": [{ "text": system_instruction }]
        },
        "contents": [{
            "role": "user",
            "parts": parts
        }],
        "generationConfig": {
            "temperature": 0.3,
            "maxOutputTokens": max_tokens
        }
    });

    let request_id = REQUEST_SEQ.fetch_add(1, Ordering::Relaxed);
    let request_id_str = format!("voice-{}", request_id);
    let _ = app.emit(
        "voice-gemini-started",
        serde_json::json!({
            "requestId": request_id_str,
            "usingScreen": has_screen,
        }),
    );

    let capability = if has_screen {
        ModelCapability::Multimodal
    } else {
        ModelCapability::Audio
    };

    let manager = GeminiModelManager::global();
    let mut tried: HashSet<String> = HashSet::new();
    let mut last_err = String::from("No Gemini model available");
    let client = http_client();

    while let Some(model) = manager.select_for(capability, Some(manager.primary_model()), &tried) {
        if cancel.is_cancelled() {
            return Err("cancelled".into());
        }
        tried.insert(model.clone());

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            model, api_key
        );

        let t0 = Instant::now();
        let resp = match client.post(&url).json(&payload).send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("Network error on {}: {}", model, e);
                manager.blacklist_model(&model, &last_err, Some(Duration::from_secs(15)));
                continue;
            }
        };

        let status = resp.status();
        if !status.is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            last_err = format!("{} ({}): {}", model, status, err_text);
            continue;
        }

        let mut stream = resp.bytes_stream();
        let mut accumulated = String::new();
        let mut sse_buf = String::new();

        while let Some(chunk_res) = stream.next().await {
            if cancel.is_cancelled() {
                return Err("cancelled".into());
            }
            let chunk = chunk_res.map_err(|e| format!("Stream read error: {}", e))?;
            sse_buf.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(idx) = sse_buf.find("\n\n") {
                let event = sse_buf[..idx].to_string();
                sse_buf = sse_buf[idx + 2..].to_string();
                for line in event.lines() {
                    let line = line.trim();
                    let data = if let Some(rest) = line.strip_prefix("data:") {
                        rest.trim()
                    } else {
                        continue;
                    };
                    if data.is_empty() || data == "[DONE]" {
                        continue;
                    }
                    if let Some(delta) = extract_text(data) {
                        accumulated.push_str(&delta);
                        let _ = app.emit(
                            "voice-gemini-chunk",
                            serde_json::json!({
                                "requestId": request_id_str,
                                "text": accumulated,
                                "delta": delta,
                                "partial": true,
                                "latencyMs": t0.elapsed().as_millis(),
                                "usingScreen": has_screen,
                            }),
                        );
                    }
                }
            }
        }

        // Drain leftover JSON (non-SSE array fallback)
        if accumulated.is_empty() && !sse_buf.trim().is_empty() {
            for piece in sse_buf.split("data:") {
                let piece = piece.trim().trim_end_matches(',').trim();
                if let Some(delta) = extract_text(piece) {
                    accumulated.push_str(&delta);
                }
            }
            if !accumulated.is_empty() {
                let _ = app.emit(
                    "voice-gemini-chunk",
                    serde_json::json!({
                        "requestId": request_id_str,
                        "text": accumulated,
                        "partial": true,
                        "latencyMs": t0.elapsed().as_millis(),
                        "usingScreen": has_screen,
                    }),
                );
            }
        }

        let text = accumulated.trim().to_string();
        if text.is_empty() || text.eq_ignore_ascii_case("SKIP") {
            return Ok(None);
        }

        manager.record_success(&model, t0.elapsed().as_millis() as u64);
        let scenario = ScenarioEngine::classify_intent(&text);
        let _ = app.emit(
            "voice-scenario",
            serde_json::json!({
                "requestId": request_id_str,
                "scenario": scenario.label(),
            }),
        );
        println!("[API SUCCESS] {} in {}ms", request_id_str, t0.elapsed().as_millis());
        return Ok(Some(text));
    }

    Err(format!("Gemini request failed: {}", last_err))
}

/// Explain a captured screenshot with Gemini vision (no Tesseract).
pub async fn answer_from_screenshot(
    app: tauri::AppHandle,
    image_bytes: Vec<u8>,
) -> Result<String, String> {
    let api_key = resolve_api_key(&app);
    if api_key.is_empty() {
        return Err("Gemini API key is not configured.".into());
    }

    let dynimg = image::load_from_memory(&image_bytes).map_err(|e| format!("Invalid screenshot: {e}"))?;
    let dynimg = if dynimg.width() > 1280 || dynimg.height() > 1280 {
        dynimg.resize(1280, 1280, image::imageops::FilterType::Triangle)
    } else {
        dynimg
    };
    let rgb = dynimg.to_rgb8();
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 70)
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ColorType::Rgb8,
        )
        .map_err(|e| e.to_string())?;

    let b64 = B64.encode(&jpeg);
    let payload = serde_json::json!({
        "systemInstruction": {
            "parts": [{
                "text": "You are KS-Vision. Describe and answer based on the screenshot. Be concise. Extract visible text, errors, and UI. Follow KS-Vision principles: concept first if they ask a technical question; do not invent Ember360 or other project experience unless the screen is clearly about that work."
            }]
        },
        "contents": [{
            "role": "user",
            "parts": [
                { "text": "Explain what is on this screen and any useful next steps." },
                { "inlineData": { "mimeType": "image/jpeg", "data": b64 } }
            ]
        }],
        "generationConfig": {
            "temperature": 0.3,
            "maxOutputTokens": 512
        }
    });

    let manager = GeminiModelManager::global();
    let mut tried = HashSet::new();
    let mut last_err = String::from("No vision model available");
    let client = http_client();

    while let Some(model) = manager.select_for(ModelCapability::Image, Some(manager.primary_model()), &tried) {
        tried.insert(model.clone());
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );
        let resp = match client.post(&url).json(&payload).send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("Network error on {model}: {e}");
                continue;
            }
        };
        if !resp.status().is_success() {
            last_err = format!("{} ({}): {}", model, resp.status(), resp.text().await.unwrap_or_default());
            continue;
        }
        let body: GeminiResponse = resp.json().await.map_err(|e| e.to_string())?;
        let text = body
            .candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content)
            .and_then(|c| c.parts)
            .map(|p| p.into_iter().filter_map(|x| x.text).collect::<String>())
            .unwrap_or_default()
            .trim()
            .to_string();
        if text.is_empty() {
            last_err = "Empty vision response".into();
            continue;
        }
        return Ok(text);
    }
    Err(last_err)
}

/// Pull `Q:` from anywhere in the first few lines (meeting path).
pub fn extract_system_question(answer: &str) -> (String, Option<String>) {
    let mut question = None;
    let mut body_lines = Vec::new();
    for (i, line) in answer.lines().enumerate() {
        let t = line.trim();
        if question.is_none() && t.len() >= 2 && t[..2].eq_ignore_ascii_case("q:") {
            question = Some(t[2..].trim().to_string());
            continue;
        }
        if i < 8 && t.to_lowercase().starts_with("question:") {
            question = Some(t[9..].trim().to_string());
            continue;
        }
        body_lines.push(line);
    }
    let body = body_lines.join("\n").trim().to_string();
    if body.is_empty() {
        (answer.to_string(), question)
    } else {
        (body, question)
    }
}
