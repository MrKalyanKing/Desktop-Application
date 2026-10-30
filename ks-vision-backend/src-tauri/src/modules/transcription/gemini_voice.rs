//! Gemini multimodal voice: WAV + optional JPEG, streamed tokens to the UI.

use std::collections::{HashMap, HashSet, VecDeque};
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
use crate::modules::screenshot::capture::capture_voice_context_jpeg;
use crate::modules::transcription::wav::write_wav_to_bytes;

const MAX_SAMPLES: usize = 20 * 16000;
const MIN_API_SAMPLES: usize = 8_000; // 0.5s @ 16kHz
const MIN_RMS: f32 = 0.004;
const MIN_PEAK: f32 = 0.012;
const MIN_SPEECH_RATIO: f32 = 0.10;

static REQUEST_SEQ: AtomicU64 = AtomicU64::new(1);
static HTTP: OnceLock<Client> = OnceLock::new();
static API_KEY_CACHE: OnceLock<Mutex<String>> = OnceLock::new();
static RECENT_TURNS: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();
static THINKING_VARIANT: OnceLock<Mutex<HashMap<String, u8>>> = OnceLock::new();

const RECENT_TURN_LIMIT: usize = 4;
const ANSWER_GIST: usize = 140;
const TURN_SNIPPET: usize = 350;

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

/// Store the last few Q&A turns so a real follow-up ("why?", "and in Playwright?")
/// can be resolved. Answers are kept short: a long previous answer in the prompt
/// pulls the model back to the old topic.
pub fn remember_turn(question: Option<&str>, answer: &str) {
    let Ok(mut q) = recent_turns().lock() else {
        return;
    };
    if let Some(host) = question.map(str::trim).filter(|s| !s.is_empty()) {
        q.push_back(format!("Q: {}", clip_turn(host)));
    }
    let ans: String = answer.trim().chars().take(ANSWER_GIST).collect();
    if !ans.is_empty() {
        q.push_back(format!("A (gist): {ans}…"));
    }
    while q.len() > RECENT_TURN_LIMIT {
        q.pop_front();
    }
}

/// Text part sent before the audio. Context first, instruction last: the model
/// weighs the final instruction most.
pub(crate) fn listen_text(has_screen: bool) -> String {
    let mut listen = format_recent_context();
    listen.push_str(
        "NEW AUDIO follows. Answer only the question spoken in this audio, using the live listening rules.",
    );
    if has_screen {
        listen.push_str(" A screenshot is attached; use it only if they ask about the screen.");
    }
    listen
}

pub(crate) fn clear_turns() {
    if let Ok(mut q) = recent_turns().lock() {
        q.clear();
    }
}

/// Earlier turns of this session only — never stale history from a previous run.
fn format_recent_context() -> String {
    match recent_turns().lock() {
        Ok(q) if !q.is_empty() => format!(
            "EARLIER IN THIS CONVERSATION (context only; these are already answered — do not answer them again):\n{}\n",
            q.iter().cloned().collect::<Vec<_>>().join("\n")
        ),
        _ => String::new(),
    }
}

fn http_client() -> &'static Client {
    HTTP.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(45))
            .connect_timeout(Duration::from_secs(5))
            .pool_max_idle_per_host(4)
            .pool_idle_timeout(Duration::from_secs(300))
            .tcp_keepalive(Duration::from_secs(30))
            .tcp_nodelay(true)
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

pub(crate) fn load_api_key_from_disk() -> String {
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

/// Result of one voice request.
#[derive(Debug, Clone)]
pub struct VoiceAnswer {
    /// What the model heard (`Q:` line), if it reported one.
    pub question: Option<String>,
    pub answer: String,
    pub first_token_ms: u64,
    pub total_ms: u64,
}

/// Parse state of the streamed `Q: <heard question>\n<answer>` header.
#[derive(Debug, PartialEq)]
pub enum HeardHeader<'a> {
    /// Not enough text yet to decide.
    Pending,
    /// Header present: the question and the answer body after it.
    Heard { question: &'a str, body: &'a str },
    /// No header: the whole text is the answer.
    None(&'a str),
}

pub fn parse_heard_header(text: &str) -> HeardHeader<'_> {
    let t = text.trim_start();
    if t.len() < 2 {
        return if t.is_empty() || t.eq_ignore_ascii_case("q") {
            HeardHeader::Pending
        } else {
            HeardHeader::None(t)
        };
    }
    if !t.is_char_boundary(2) || !t[..2].eq_ignore_ascii_case("q:") {
        return HeardHeader::None(t);
    }
    match t.find('\n') {
        Some(nl) => HeardHeader::Heard {
            question: t[2..nl].trim(),
            body: t[nl + 1..].trim_start(),
        },
        None => HeardHeader::Pending,
    }
}

/// While streaming, hold back text that may still turn into a bare `SKIP`.
fn could_be_skip(body: &str) -> bool {
    let b = body.trim();
    b.len() <= 4 && "SKIP".starts_with(&b.to_ascii_uppercase())
}

fn voice_screen_enabled() -> bool {
    matches!(
        std::env::var("KS_VOICE_SCREEN").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}

/// Incremental parser for Gemini `alt=sse` streams. Events end with a blank line,
/// which Gemini sends as `\r\n\r\n` — splitting only on `\n\n` meant no text was
/// seen until the stream closed, so answers appeared all at once.
/// Works on bytes so a UTF-8 character split across network chunks stays intact.
#[derive(Default)]
pub(crate) struct SseTextParser {
    buf: Vec<u8>,
}

impl SseTextParser {
    /// Feed a network chunk; returns the text of every event it completed.
    pub fn push(&mut self, chunk: &[u8]) -> String {
        self.buf.extend_from_slice(chunk);
        let mut out = String::new();
        while let Some((end, sep)) = find_event_end(&self.buf) {
            let event: Vec<u8> = self.buf.drain(..end + sep).collect();
            out.push_str(&event_text(&event[..end]));
        }
        out
    }

    /// Text of a final event that arrived without a trailing blank line.
    pub fn finish(&mut self) -> String {
        let rest = std::mem::take(&mut self.buf);
        event_text(&rest)
    }
}

fn find_event_end(buf: &[u8]) -> Option<(usize, usize)> {
    let lf = buf.windows(2).position(|w| w == b"\n\n").map(|i| (i, 2));
    let crlf = buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| (i, 4));
    match (lf, crlf) {
        (Some(a), Some(b)) => Some(if a.0 <= b.0 { a } else { b }),
        (a, b) => a.or(b),
    }
}

fn event_text(event: &[u8]) -> String {
    let event = String::from_utf8_lossy(event);
    let mut out = String::new();
    for line in event.lines() {
        let Some(data) = line.trim().strip_prefix("data:").map(str::trim) else {
            continue;
        };
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        if let Some(text) = extract_text(data) {
            out.push_str(&text);
        }
    }
    out
}

/// Thinking adds seconds before the first token; ask for the minimum each model accepts.
/// 0 = thinkingLevel minimal, 1 = thinkingBudget 0, 2 = omit.
pub(crate) fn thinking_variant(model: &str) -> u8 {
    THINKING_VARIANT
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .ok()
        .and_then(|m| m.get(model).copied())
        .unwrap_or(0)
}

pub(crate) fn set_thinking_variant(model: &str, v: u8) {
    if let Ok(mut m) = THINKING_VARIANT.get_or_init(|| Mutex::new(HashMap::new())).lock() {
        m.insert(model.to_string(), v);
    }
}

pub(crate) fn generation_config(max_tokens: u32, variant: u8) -> serde_json::Value {
    let mut cfg = serde_json::json!({
        "temperature": 0.3,
        "maxOutputTokens": max_tokens
    });
    if let Some(t) = thinking_config(variant) {
        cfg["thinkingConfig"] = t;
    }
    cfg
}

pub(crate) fn thinking_config(variant: u8) -> Option<serde_json::Value> {
    match variant {
        0 => Some(serde_json::json!({ "thinkingLevel": "minimal" })),
        1 => Some(serde_json::json!({ "thinkingBudget": 0 })),
        _ => None,
    }
}

#[cfg(test)]
mod sse_tests {
    use super::SseTextParser;

    fn event(text: &str, sep: &str) -> String {
        format!(
            "data: {{\"candidates\":[{{\"content\":{{\"parts\":[{{\"text\":\"{text}\"}}]}}}}]}}{sep}"
        )
    }

    #[test]
    fn emits_each_event_as_it_arrives_with_crlf() {
        let mut p = SseTextParser::default();
        assert_eq!(p.push(event("Hel", "\r\n\r\n").as_bytes()), "Hel");
        assert_eq!(p.push(event("lo", "\r\n\r\n").as_bytes()), "lo");
        assert_eq!(p.finish(), "");
    }

    #[test]
    fn handles_lf_and_events_split_across_chunks() {
        let mut p = SseTextParser::default();
        let all = event("one ", "\n\n") + &event("two", "\n\n");
        let (a, b) = all.as_bytes().split_at(30);
        let mut out = p.push(a);
        out.push_str(&p.push(b));
        assert_eq!(out, "one two");
    }

    #[test]
    fn keeps_multibyte_characters_split_across_chunks() {
        let mut p = SseTextParser::default();
        let e = event("naïve — ok", "\r\n\r\n");
        let bytes = e.as_bytes();
        let cut = e.find('ï').unwrap() + 1; // inside the 2-byte char
        let mut out = p.push(&bytes[..cut]);
        out.push_str(&p.push(&bytes[cut..]));
        assert_eq!(out, "naïve — ok");
    }

    #[test]
    fn final_event_without_blank_line_is_not_lost() {
        let mut p = SseTextParser::default();
        assert_eq!(p.push(event("tail", "").as_bytes()), "");
        assert_eq!(p.finish(), "tail");
    }
}

/// Short, actionable message for the UI.
pub fn friendly_error(status: Option<reqwest::StatusCode>, body: &str) -> String {
    let lower = body.to_lowercase();
    match status.map(|s| s.as_u16()) {
        Some(401) | Some(403) => "Gemini rejected the API key — check Settings → AI.".into(),
        Some(400) if lower.contains("api key") => {
            "Gemini rejected the API key — check Settings → AI.".into()
        }
        Some(429) => "Gemini rate limit reached — answers resume in a moment.".into(),
        Some(s) if s >= 500 => "Gemini is temporarily unavailable — retrying next question.".into(),
        None => "Network problem reaching Gemini — check your connection.".into(),
        _ => {
            let snippet: String = body.chars().take(160).collect();
            format!("Gemini request failed: {snippet}")
        }
    }
}

/// Open the TLS/HTTP2 connection ahead of the first question (saves a handshake).
pub fn prewarm(app: &tauri::AppHandle) {
    let api_key = resolve_api_key(app);
    if api_key.is_empty() {
        return;
    }
    tokio::spawn(async move {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models?pageSize=1&key={}",
            api_key
        );
        let _ = http_client().get(&url).send().await;
    });
}

/// Send one utterance to Gemini and stream the answer to the UI.
pub async fn answer_from_audio(
    app: tauri::AppHandle,
    samples: &[f32],
    sample_rate: u32,
    from_system_audio: bool,
    cancel: CancellationToken,
    op_id: u64,
) -> Result<Option<VoiceAnswer>, String> {
    let started = Instant::now();
    let prepared = preprocess::prepare_for_voice(samples);
    let clipped = if prepared.len() > MAX_SAMPLES {
        &prepared[prepared.len() - MAX_SAMPLES..]
    } else {
        prepared.as_slice()
    };

    if let Some(reason) = local_skip_reason(clipped, sample_rate) {
        if voice_debug_enabled() {
            println!("[LOCAL SKIP] {} ({:.2}s)", reason, clipped.len() as f32 / sample_rate.max(1) as f32);
        }
        return Ok(None);
    }

    let api_key = resolve_api_key(&app);
    if api_key.is_empty() {
        return Err("Gemini API key is not configured — add it in Settings → AI.".into());
    }

    // Screen context is opt-in: attaching it to every question slowed answers and
    // pulled them toward whatever happened to be on screen.
    let jpeg_task = voice_screen_enabled().then(|| tokio::task::spawn_blocking(capture_voice_context_jpeg));
    let wav = write_wav_to_bytes(clipped, sample_rate);
    let b64_audio = B64.encode(&wav);
    let jpeg = match jpeg_task {
        Some(t) => t.await.ok().and_then(|r| r.ok()),
        None => None,
    };
    let has_screen = jpeg.is_some();
    let b64_jpeg = jpeg.map(|j| B64.encode(&j));

    let system_instruction = ScenarioEngine::live_system_prompt(from_system_audio);
    let max_tokens = ScenarioType::General.max_output_tokens();
    let listen = listen_text(has_screen);

    let mut parts = vec![serde_json::json!({ "text": listen })];
    if let Some(img) = b64_jpeg {
        parts.push(serde_json::json!({
            "inlineData": { "mimeType": "image/jpeg", "data": img }
        }));
    }
    parts.push(serde_json::json!({
        "inlineData": { "mimeType": "audio/wav", "data": b64_audio }
    }));

    let mut payload = serde_json::json!({
        "systemInstruction": {
            "parts": [{ "text": system_instruction }]
        },
        "contents": [{
            "role": "user",
            "parts": parts
        }],
        "generationConfig": generation_config(max_tokens, 0)
    });

    let request_id = REQUEST_SEQ.fetch_add(1, Ordering::Relaxed);
    let request_id_str = format!("voice-{}", request_id);
    let source_key = if from_system_audio { "system" } else { "voice" };
    let _ = app.emit(
        "voice-gemini-started",
        serde_json::json!({
            "requestId": request_id_str,
            "opId": op_id,
            "usingScreen": has_screen,
            "audioMs": clipped.len() as u64 * 1000 / sample_rate.max(1) as u64,
            "source": source_key,
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

        let t0 = Instant::now();
        let mut variant = thinking_variant(&model);
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            model, api_key
        );
        let resp = loop {
            payload["generationConfig"] = generation_config(max_tokens, variant);
            // Race the request against the token so F3 / barge-in stop it immediately.
            let sent = tokio::select! {
                _ = cancel.cancelled() => return Err("cancelled".into()),
                r = client.post(&url).json(&payload).send() => r,
            };
            match sent {
                Ok(r) if r.status().is_success() => break Ok(r),
                Ok(r) => {
                    let status = r.status();
                    let body = r.text().await.unwrap_or_default();
                    // Model does not accept this thinking setting: try the next form.
                    if status == reqwest::StatusCode::BAD_REQUEST
                        && variant < 2
                        && body.to_lowercase().contains("thinking")
                    {
                        variant += 1;
                        set_thinking_variant(&model, variant);
                        continue;
                    }
                    break Err((Some(status), body));
                }
                Err(e) => break Err((None, e.to_string())),
            }
        };
        tried.insert(model.clone());

        let resp = match resp {
            Ok(r) => r,
            Err((status, body)) => {
                eprintln!(
                    "[VOICE] {} failed ({:?}): {}",
                    model,
                    status,
                    body.chars().take(300).collect::<String>()
                );
                last_err = friendly_error(status, &body);
                match status {
                    Some(s) => {
                        manager.register_failure(&model, capability, s, &body);
                    }
                    None => manager.blacklist_model(&model, &body, Some(Duration::from_secs(15))),
                }
                continue;
            }
        };

        let mut stream = resp.bytes_stream();
        let mut accumulated = String::new();
        let mut sse = SseTextParser::default();
        let mut first_token_ms: Option<u64> = None;
        let mut heard_sent = false;

        loop {
            let chunk_res = tokio::select! {
                _ = cancel.cancelled() => return Err("cancelled".into()),
                next = stream.next() => match next {
                    Some(c) => c,
                    None => break,
                },
            };
            let chunk = chunk_res.map_err(|e| format!("Stream read error: {}", e))?;
            let delta = sse.push(&chunk);
            if delta.is_empty() {
                continue;
            }
            accumulated.push_str(&delta);
            let ttft = *first_token_ms.get_or_insert(started.elapsed().as_millis() as u64);

            let body = match parse_heard_header(&accumulated) {
                HeardHeader::Pending => continue,
                HeardHeader::Heard { question, body } => {
                    if !heard_sent && !question.is_empty() {
                        heard_sent = true;
                        let _ = app.emit(
                            "voice-heard",
                            serde_json::json!({
                                "requestId": request_id_str,
                                "opId": op_id,
                                "question": question,
                                "source": source_key,
                            }),
                        );
                    }
                    body
                }
                HeardHeader::None(body) => body,
            };
            if body.is_empty() || could_be_skip(body) {
                continue;
            }
            let _ = app.emit(
                "voice-gemini-chunk",
                serde_json::json!({
                    "requestId": request_id_str,
                    "opId": op_id,
                    "text": body,
                    "partial": true,
                    "ttftMs": ttft,
                    "latencyMs": t0.elapsed().as_millis(),
                    "usingScreen": has_screen,
                }),
            );
        }

        accumulated.push_str(&sse.finish());

        let (question, answer) = match parse_heard_header(&accumulated) {
            HeardHeader::Heard { question, body } => (
                Some(question.to_string()).filter(|q| !q.is_empty()),
                body.trim().to_string(),
            ),
            // "Q: …" with no answer line.
            HeardHeader::Pending => (None, String::new()),
            HeardHeader::None(body) => (None, body.trim().to_string()),
        };

        manager.record_success(&model, t0.elapsed().as_millis() as u64);
        let total_ms = started.elapsed().as_millis() as u64;
        let first_token_ms = first_token_ms.unwrap_or(total_ms);
        if answer.is_empty() || answer.eq_ignore_ascii_case("SKIP") {
            if voice_debug_enabled() {
                println!("[VOICE] {} skipped by model (heard: {:?})", request_id_str, question);
            }
            let _ = app.emit(
                "voice-skipped",
                serde_json::json!({ "requestId": request_id_str, "opId": op_id }),
            );
            return Ok(None);
        }

        let scenario = ScenarioEngine::classify_intent(question.as_deref().unwrap_or(&answer));
        let _ = app.emit(
            "voice-scenario",
            serde_json::json!({
                "requestId": request_id_str,
                "scenario": scenario.label(),
            }),
        );
        println!(
            "[API SUCCESS] {} on {} first-token {}ms total {}ms",
            request_id_str, model, first_token_ms, total_ms
        );
        return Ok(Some(VoiceAnswer {
            question,
            answer,
            first_token_ms,
            total_ms,
        }));
    }

    Err(last_err)
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
                "text": "You are KS-Vision. Describe and answer based on the screenshot. Be concise. Extract visible text, errors, and UI. Follow KS-Vision principles: concept first if they ask a technical question; do not invent project experience unless the screen is clearly about that work."
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
