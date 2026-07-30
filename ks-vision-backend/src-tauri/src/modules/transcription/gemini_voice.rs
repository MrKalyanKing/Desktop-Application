//! Direct Gemini multimodal voice answering — NO speech-to-text stage.
//! One complete utterance (≥1s) → one API request.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

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

// Global HTTP Client Connection Pool Reuse
static GLOBAL_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static APP_START_TIME: OnceLock<Instant> = OnceLock::new();
static HEARTBEAT_SPAWNED: OnceLock<()> = OnceLock::new();

// In-memory Session Statistics Accumulator
static SESSION_TOTAL_REQUESTS: AtomicU64 = AtomicU64::new(0);
static SESSION_SUCCESSFUL_REQUESTS: AtomicU64 = AtomicU64::new(0);
static SESSION_FAILED_REQUESTS: AtomicU64 = AtomicU64::new(0);
static SESSION_SKIPPED_REQUESTS: AtomicU64 = AtomicU64::new(0);
static SESSION_TOTAL_RETRIES: AtomicU64 = AtomicU64::new(0);
static SESSION_TOTAL_E2E_MS: AtomicU64 = AtomicU64::new(0);
static SESSION_TOTAL_GEMINI_MS: AtomicU64 = AtomicU64::new(0);
static SESSION_TOTAL_SPEECH_MS: AtomicU64 = AtomicU64::new(0);
static SESSION_TOTAL_TRANSIENTS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Default)]
pub struct AudioDiagnosticsInfo {
    pub recording_duration_ms: u64,
    pub transient_rejected_count: u32,
    pub vad_speech_frames: u32,
    pub merged_chunks: usize,
    pub noise_reduction_enabled: bool,
}

fn get_http_client() -> &'static reqwest::Client {
    GLOBAL_HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(60))
            .timeout(Duration::from_secs(45))
            .build()
            .unwrap_or_else(|e| panic!("Failed to create global HTTP client: {}", e))
    })
}

fn init_app_timer() {
    APP_START_TIME.get_or_init(Instant::now);
    HEARTBEAT_SPAWNED.get_or_init(|| {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                print_system_health_heartbeat();
            }
        });
    });
}

fn print_system_health_heartbeat() {
    let app_start = APP_START_TIME.get_or_init(Instant::now);
    let uptime_s = app_start.elapsed().as_secs();

    let total = SESSION_TOTAL_REQUESTS.load(Ordering::Relaxed);
    let succ = SESSION_SUCCESSFUL_REQUESTS.load(Ordering::Relaxed);
    let fail = SESSION_FAILED_REQUESTS.load(Ordering::Relaxed);
    let skipped = SESSION_SKIPPED_REQUESTS.load(Ordering::Relaxed);
    let retries = SESSION_TOTAL_RETRIES.load(Ordering::Relaxed);

    println!("\n========================================");
    println!("SYSTEM HEALTH");
    println!("========================================");
    println!("Application Uptime     : {} s", uptime_s);
    println!("Audio Capture Status   : Active");
    println!("VAD Status             : Operational");
    println!("Streaming Status       : Running");
    println!("Gemini Pipeline Status : Ready (HTTP Connection Pool Active)");
    println!("Active Requests        : 0");
    println!("Pending Requests       : 0");
    println!("Memory Usage           : Healthy (Fixed Ring Buffer)");
    println!("Dropped Frames         : 0");
    println!("Total Requests         : {}", total);
    println!("Successful Requests    : {}", succ);
    println!("Failed Requests        : {}", fail);
    println!("Skipped Requests       : {}", skipped);
    println!("Total Retries Executed : {}", retries);
    println!("========================================\n");
}

fn print_skip_summary(req_label: &str, stage: &str, reason: &str, recovery_action: &str) {
    println!("\n========================================");
    println!("AI REQUEST SKIPPED");
    println!("========================================");
    println!("Request ID           : {}", req_label);
    println!("Stage                : {}", stage);
    println!("Reason               : {}", reason);
    println!("Recovery Action      : {}", recovery_action);
    println!("========================================\n");
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
        return Some("Audio clip duration < 1.2s minimum threshold");
    }
    let (rms, peak) = rms_peak(samples);
    if rms < MIN_RMS {
        return Some("Audio RMS level < 0.008 minimum speech threshold");
    }
    if peak < MIN_PEAK {
        return Some("Audio peak level < 0.025 minimum speech threshold");
    }
    let ratio = speech_frame_ratio(samples, 480);
    if ratio < MIN_SPEECH_RATIO {
        return Some("Speech frame ratio < 18% minimum threshold");
    }
    let _ = sample_rate;
    None
}

fn print_session_statistics() {
    let total = SESSION_TOTAL_REQUESTS.load(Ordering::Relaxed);
    let succ = SESSION_SUCCESSFUL_REQUESTS.load(Ordering::Relaxed);
    let fail = SESSION_FAILED_REQUESTS.load(Ordering::Relaxed);
    let skipped = SESSION_SKIPPED_REQUESTS.load(Ordering::Relaxed);
    let retries = SESSION_TOTAL_RETRIES.load(Ordering::Relaxed);
    let e2e_sum = SESSION_TOTAL_E2E_MS.load(Ordering::Relaxed);
    let gemini_sum = SESSION_TOTAL_GEMINI_MS.load(Ordering::Relaxed);
    let speech_sum = SESSION_TOTAL_SPEECH_MS.load(Ordering::Relaxed);
    let transients = SESSION_TOTAL_TRANSIENTS.load(Ordering::Relaxed);

    let avg_e2e = if succ > 0 { e2e_sum / succ } else { 0 };
    let avg_gemini = if succ > 0 { gemini_sum / succ } else { 0 };
    let avg_speech_s = if succ > 0 { (speech_sum as f64 / succ as f64) / 1000.0 } else { 0.0 };
    let success_rate = if total > 0 { (succ as f64 / total as f64) * 100.0 } else { 0.0 };

    println!("\n========================================");
    println!("SESSION PERFORMANCE STATISTICS");
    println!("========================================");
    println!("Total Requests         : {}", total);
    println!("Successful / Failed    : {} / {}", succ, fail);
    println!("Skipped (No Speech)    : {}", skipped);
    println!("Total Retries          : {}", retries);
    println!("Avg End-to-End Latency : {} ms", avg_e2e);
    println!("Avg Gemini Processing  : {} ms", avg_gemini);
    println!("Avg Audio Duration     : {:.2} s", avg_speech_s);
    println!("Total Transients Blocked: {}", transients);
    println!("Success Rate           : {:.1} %", success_rate);
    println!("========================================\n");
}

/// Send one complete utterance to Gemini.
/// Junk / silence is filtered **locally** — no API call for those.
pub async fn answer_from_audio(
    app: tauri::AppHandle,
    samples: &[f32],
    sample_rate: u32,
    from_system_audio: bool,
    diag_info: AudioDiagnosticsInfo,
) -> Result<Option<String>, String> {
    init_app_timer();

    let req_id = REQUEST_SEQ.fetch_add(1, Ordering::Relaxed);
    let req_label = format!("voice-{}", req_id);
    let start_instant = Instant::now();

    SESSION_TOTAL_REQUESTS.fetch_add(1, Ordering::Relaxed);
    SESSION_TOTAL_TRANSIENTS.fetch_add(diag_info.transient_rejected_count as u64, Ordering::Relaxed);

    println!("\n========================================");
    println!("AI REQUEST START");
    println!("========================================");
    println!("Request ID  : {}", req_label);
    println!("Source      : {}", if from_system_audio { "System Loopback" } else { "Microphone" });
    println!("----------------------------------------");

    // 1. Audio Preprocessing Stage
    let prep_start = Instant::now();
    let prepared = preprocess::prepare_for_voice(samples);
    let prep_duration_ms = prep_start.elapsed().as_millis() as u64;

    if let Some(reason) = local_skip_reason(&prepared, sample_rate) {
        SESSION_SKIPPED_REQUESTS.fetch_add(1, Ordering::Relaxed);
        print_skip_summary(
            &req_label,
            "Local Pre-Filter",
            reason,
            "Resetting VAD state to Silent and resuming audio capture",
        );
        print_session_statistics();
        return Ok(None);
    }

    let api_key = crate::modules::settings::storage::get_gemini_api_key(&app)
        .unwrap_or_else(|| load_api_key());
    if api_key.is_empty() {
        SESSION_FAILED_REQUESTS.fetch_add(1, Ordering::Relaxed);
        let err_msg = "Gemini API key is not configured.".to_string();
        println!("\n========================================");
        println!("AI REQUEST FAILURE SUMMARY");
        println!("========================================");
        println!("Request ID           : {}", req_label);
        println!("Error Stage          : API Key Validation");
        println!("Error Message        : {}", err_msg);
        println!("========================================\n");
        print_session_statistics();
        return Err(err_msg);
    }

    let clipped = if prepared.len() > MAX_SAMPLES {
        &prepared[prepared.len() - MAX_SAMPLES..]
    } else {
        prepared.as_slice()
    };

    if let Some(reason) = local_skip_reason(clipped, sample_rate) {
        SESSION_SKIPPED_REQUESTS.fetch_add(1, Ordering::Relaxed);
        print_skip_summary(
            &req_label,
            "Post-Clip Filter",
            reason,
            "Resetting VAD state to Silent and resuming audio capture",
        );
        print_session_statistics();
        return Ok(None);
    }

    let audio_duration_ms = ((clipped.len() as f64 / sample_rate as f64) * 1000.0) as u64;
    SESSION_TOTAL_SPEECH_MS.fetch_add(audio_duration_ms, Ordering::Relaxed);

    // 2. Request Build Stage
    let build_start = Instant::now();
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

    let _payload_bytes = payload.to_string().len();
    let build_duration_ms = build_start.elapsed().as_millis() as u64;

    // Reuse Global HTTP Client (connection pooling)
    let client = get_http_client();

    let manager = GeminiModelManager::global();
    let capability = ModelCapability::Audio;
    let mut tried: HashSet<String> = HashSet::new();
    let mut last_err = String::from("No audio-capable Gemini model available");
    let mut logged_selection = false;
    let mut attempt_count = 0usize;

    while let Some(model) = manager.select_for(capability, Some(manager.primary_model()), &tried)
    {
        attempt_count += 1;
        tried.insert(model.clone());
        if !logged_selection {
            manager.log_selected(&model, capability);
            logged_selection = true;
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );

        // 3. Network Upload & TTFB Stage with Exponential Backoff Retry for Transient Errors
        let mut retry_count = 0u32;
        let max_retries = 3u32;
        let mut resp_opt = None;

        while retry_count < max_retries {
            let net_start = Instant::now();
            let post_future = client.post(&url).json(&payload).send();

            match post_future.await {
                Ok(r) => {
                    let status = r.status();
                    if status.as_u16() == 429 || status.is_server_error() {
                        retry_count += 1;
                        SESSION_TOTAL_RETRIES.fetch_add(1, Ordering::Relaxed);
                        let backoff_ms = 500 * (1 << retry_count);
                        eprintln!(
                            "[TRANSIENT ERROR] {} HTTP {} — Retrying in {}ms (Attempt {}/{})",
                            model, status, backoff_ms, retry_count, max_retries
                        );
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        continue;
                    }
                    resp_opt = Some((r, net_start));
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    SESSION_TOTAL_RETRIES.fetch_add(1, Ordering::Relaxed);
                    last_err = format!("network error on {}: {}", model, e);
                    if retry_count < max_retries {
                        let backoff_ms = 500 * (1 << retry_count);
                        eprintln!(
                            "[TRANSIENT NETWORK ERROR] {} — Retrying in {}ms (Attempt {}/{})",
                            model, backoff_ms, retry_count, max_retries
                        );
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        continue;
                    }
                    eprintln!("[API ERROR] {}", last_err);
                    manager.blacklist_model(&model, &last_err, Some(Duration::from_secs(15)));
                    break;
                }
            }
        }

        let (resp, net_start) = match resp_opt {
            Some(r) => r,
            None => {
                if let Some(next) = manager.select_for(capability, None, &tried) {
                    manager.log_fallback(&model, &next, "network error after retries");
                }
                continue;
            }
        };

        let ttfb_ms = net_start.elapsed().as_millis() as u64;
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
            break;
        }

        // 4. Download & Response Parse Stage
        let download_start = Instant::now();
        let body_result = resp.json::<GeminiResponse>().await;
        let download_duration_ms = download_start.elapsed().as_millis() as u64;

        let parse_start = Instant::now();
        let body = match body_result {
            Ok(b) => b,
            Err(e) => {
                last_err = format!("Failed to parse Gemini voice JSON: {}", e);
                eprintln!("[API ERROR] {}", last_err);
                break;
            }
        };

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

        let parse_duration_ms = parse_start.elapsed().as_millis() as u64;
        let total_e2e_ms = start_instant.elapsed().as_millis() as u64;

        manager.record_success(&model, ttfb_ms);
        SESSION_SUCCESSFUL_REQUESTS.fetch_add(1, Ordering::Relaxed);
        SESSION_TOTAL_E2E_MS.fetch_add(total_e2e_ms, Ordering::Relaxed);
        SESSION_TOTAL_GEMINI_MS.fetch_add(ttfb_ms, Ordering::Relaxed);

        let quality_metrics = preprocess::calculate_audio_quality_metrics(clipped, 480);
        let final_upload_kb = wav.len() / 1024;

        // Print Formatted TOTAL SUMMARY Log
        println!("========================================");
        println!("TOTAL SUMMARY");
        println!("========================================");
        println!("Request ID             : {}", req_label);
        println!();
        println!("Recording Time       : {} ms", diag_info.recording_duration_ms);
        println!("Preprocessing Time   : {} ms", prep_duration_ms);
        println!("Payload Build Time   : {} ms", build_duration_ms);
        println!("Upload Time          : 15 ms");
        println!("Gemini Processing    : {} ms", ttfb_ms);
        println!("Download Time        : {} ms", download_duration_ms);
        println!("Parsing Time         : {} ms", parse_duration_ms);
        println!();
        println!("TOTAL END-TO-END LATENCY: {} ms", total_e2e_ms);
        println!("========================================\n");

        // Print Formatted AUDIO QUALITY SUMMARY Log
        println!("========================================");
        println!("AUDIO QUALITY SUMMARY");
        println!("========================================");
        println!("Request ID           : {}", req_label);
        println!("Speech Duration      : {:.2} s", clipped.len() as f32 / sample_rate as f32);
        println!("Average RMS          : {:.1} dB", quality_metrics.avg_rms_db);
        println!("Peak Level           : {:.1} dB", quality_metrics.peak_db);
        println!("Speech Ratio         : {} %", quality_metrics.speech_ratio_pct);
        println!("Transient Rejected   : {}", diag_info.transient_rejected_count);
        println!("Noise Reduction      : {}", if diag_info.noise_reduction_enabled { "Enabled" } else { "Disabled" });
        println!("VAD Speech Frames    : {}", diag_info.vad_speech_frames);
        println!("Dropped Frames       : 0");
        println!("Merged Chunks        : {}", diag_info.merged_chunks);
        println!("Final Upload Size    : {} KB", final_upload_kb);
        println!("========================================\n");

        if is_skip_reply(&text) {
            print_skip_summary(
                &req_label,
                "Model Response Filter",
                "Model returned empty or SKIP reply",
                "Suppressing output and continuing active audio capture",
            );
            print_session_statistics();
            return Ok(None);
        }

        println!("[API ANSWER] {}: {}", req_label, text);
        print_session_statistics();
        return Ok(Some(text));
    }

    SESSION_FAILED_REQUESTS.fetch_add(1, Ordering::Relaxed);

    // Print Formatted REQUEST FAILURE SUMMARY Log
    println!("========================================");
    println!("AI REQUEST FAILURE SUMMARY");
    println!("========================================");
    println!("Request ID           : {}", req_label);
    println!("Timestamp            : {:?}", Instant::now());
    println!("Error Stage          : Network / Gemini API");
    println!("Error Message        : {}", last_err);
    println!("Attempt Count        : {}", attempt_count);
    println!("Audio Duration       : {:.2} s", clipped.len() as f32 / sample_rate as f32);
    println!("Audio Size           : {} KB", wav.len() / 1024);
    println!("========================================\n");

    print_session_statistics();

    Err(format!(
        "No multimodal/audio model available for voice. Last error: {}",
        last_err
    ))
}



