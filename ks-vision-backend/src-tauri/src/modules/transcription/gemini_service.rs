use serde::{Deserialize, Serialize};
use base64::Engine;
use crate::modules::audio::preprocess;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TranscriptChunk {
    pub text: String,
    pub speaker: String,
    pub timestamp: u64,
}

pub struct GeminiTranscriptionService;

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

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

impl GeminiTranscriptionService {
    pub fn new() -> Self {
        Self
    }

    pub fn load_api_key(&self) -> String {
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

        let paths = vec![".env", "../.env", "src-tauri/.env", "../src-tauri/.env"];
        for path in paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some(pos) = line.find('=') {
                        let key = line[..pos].trim();
                        let value = line[pos + 1..].trim();
                        let value = value.trim_matches('"').trim_matches('\'');
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

    pub async fn transcribe(
        &self,
        samples: &[f32],
        source_label: &str,
        speaker_id: &str,
        previous_context: &[TranscriptChunk],
    ) -> Result<(String, f32), String> {
        let api_key = self.load_api_key();
        if api_key.is_empty() {
            return Err("Gemini API key is not configured. Please add GEMINI_API_KEY to your .env file.".to_string());
        }

        // Noise gate + trim + normalize BEFORE upload (Whisper-like hygiene without local Whisper)
        let prepared = preprocess::prepare_for_stt(samples);
        if prepared.len() < 1600 {
            // < ~100ms of speech after trim — skip
            return Ok((String::new(), 0.0));
        }

        // Cap upload size (~20s @ 16k) to protect latency/quota
        let max_samples = 20 * 16000;
        let prepared = if prepared.len() > max_samples {
            prepared[prepared.len() - max_samples..].to_vec()
        } else {
            prepared
        };

        let wav_bytes = write_wav_to_bytes(&prepared, 16000);
        let b64_data = base64::engine::general_purpose::STANDARD.encode(&wav_bytes);

        let mut context_prompt = String::from("Previous conversation context (last 3 utterances):\n");
        if previous_context.is_empty() {
            context_prompt.push_str("None\n");
        } else {
            for chunk in previous_context.iter().take(3) {
                context_prompt.push_str(&format!(
                    "[{} (Speaker: {})]: {}\n",
                    source_label, chunk.speaker, chunk.text
                ));
            }
        }

        let mut final_text = String::new();
        let mut using_local_stt = false;
        let local_url = std::env::var("LOCAL_WHISPER_URL")
            .unwrap_or_else(|_| "http://localhost:8080/v1/audio/transcriptions".to_string());
        
        let duration_sec = prepared.len() as f32 / 16000.0;
        
        println!("\n[TRANSCRIBE] ==================================================");
        println!("[TRANSCRIBE] 🎯 Attempting local Whisper.cpp STT at {}...", local_url);
        println!("[TRANSCRIBE] ⏱️ Audio duration: {:.2}s ({} samples)", duration_sec, prepared.len());
        
        match call_local_whisper(&local_url, wav_bytes.clone()).await {
            Ok(text) => {
                final_text = text;
                using_local_stt = true;
                println!("[TRANSCRIBE] ✅ SUCCESS! Local Whisper STT transcribed: \"{}\"", final_text);
                println!("[TRANSCRIBE] 🚀 Bypassing Gemini STT to save quota and reduce latency.");
                println!("[TRANSCRIBE] ==================================================\n");
            }
            Err(err) => {
                println!("[TRANSCRIBE WARNING] ⚠️ Local Whisper STT unavailable or failed: {}", err);
                println!("[TRANSCRIBE INFO] 🔄 Falling back to Gemini Multimodal Audio STT...");
                println!("[TRANSCRIBE] ==================================================\n");
            }
        }

        if !using_local_stt {
            let is_system = source_label.eq_ignore_ascii_case("System");
            let system_instruction = if is_system {
                "You are a production-grade speech-to-text engine for remote meeting / video call audio \
(Google Meet, Microsoft Teams, Zoom, YouTube, browser speakers). The audio is often compressed, \
echoey, or mixed with light background noise. Your job is maximum word accuracy.

RULES:
1. Transcribe EVERY intelligible word, including short answers (yes/no/ok), numbers, names, \
acronyms, and technical terms exactly as spoken.
2. Prefer the most likely word given conversational context; use previous utterances to resolve \
homophones and clipped syllables.
3. If a word is partly masked by noise but recoverable, write the best guess WITHOUT brackets.
4. Only use [unclear: 'approx'] when a word is truly unintelligible.
5. Remove fillers (um/uh) and false starts; keep meaningful content.
6. Add punctuation and '?' on questions.
7. Ignore non-speech (music beds, UI beeps, hold music) — if no human speech, output exactly: (silence)
8. Output ONLY the transcript text — no preamble, labels, or commentary."
            } else {
                "You are a production-grade speech-to-text engine for close-talk microphone audio.

RULES:
1. Transcribe accurately with punctuation; preserve names, numbers, acronyms, technical terms.
2. Drop false starts and fillers (um/uh) unless meaningful.
3. Use context for homophones; mark truly unintelligible words as [unclear: 'approx'].
4. If no speech, output exactly: (silence)
5. Output ONLY the transcript text."
            };

            let user_prompt = format!(
                "{}\nSource: {} | Speaker: {}\n\
Transcribe this audio carefully. Capture minute details of what was spoken. \
Output ONLY the transcription text.",
                context_prompt, source_label, speaker_id
            );

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(45))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());

            let payload = serde_json::json!({
                "systemInstruction": {
                    "parts": [{ "text": system_instruction }]
                },
                "contents": [{
                    "parts": [
                        { "text": user_prompt },
                        {
                            "inlineData": {
                                "mimeType": "audio/wav",
                                "data": b64_data
                            }
                        }
                    ]
                }],
                "generationConfig": {
                    "temperature": 0.1,
                    "maxOutputTokens": 2048
                }
            });

            let manager = crate::modules::ai::model_manager::GeminiModelManager::global();
            let mut attempts = 0;

            loop {
                let active_model = manager.select_model();
                println!("[TRANSCRIBE] Using: {} ({} samples)", active_model, prepared.len());

                let url = format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                    active_model, api_key
                );

                let resp = match client.post(&url).json(&payload).send().await {
                    Ok(r) => r,
                    Err(e) => return Err(format!("Failed to send transcription request: {}", e)),
                };

                let status_code = resp.status();
                if !status_code.is_success() {
                    let err_text = resp.text().await.unwrap_or_default();
                    eprintln!(
                        "[TRANSCRIBE ERROR] Gemini API returned status {}: {}",
                        status_code, err_text
                    );

                    if manager.is_quota_error(status_code, &err_text) {
                        let delay = manager.parse_retry_delay(&err_text);
                        manager.blacklist_model(&active_model, &err_text, delay);

                        attempts += 1;
                        if attempts < manager.models_len() {
                            let next_model = manager.select_model();
                            println!("[MODEL MANAGER] 429 received -> Switching to: {}", next_model);
                            continue;
                        }
                    }

                    return Err(format!(
                        "Gemini API returned error status {}: {}",
                        status_code, err_text
                    ));
                }

                let gemini_resp: GeminiResponse = resp
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

                let mut text = gemini_resp
                    .candidates
                    .and_then(|c| c.into_iter().next())
                    .and_then(|c| c.content)
                    .and_then(|c| c.parts)
                    .and_then(|p| p.into_iter().next())
                    .and_then(|p| p.text)
                    .unwrap_or_default()
                    .trim()
                    .to_string();

                // Normalize empty / silence markers
                let lower = text.to_lowercase();
                if lower.is_empty()
                    || lower == "(silence)"
                    || lower == "silence"
                    || lower == "[silence]"
                    || lower == "..."
                {
                    return Ok((String::new(), 0.0));
                }

                // Strip accidental model preambles
                for prefix in ["Transcript:", "Transcription:", "Output:"] {
                    if let Some(rest) = text.strip_prefix(prefix) {
                        text = rest.trim().to_string();
                    }
                }

                let confidence = calculate_confidence(&text, duration_sec);
                return Ok((text, confidence));
            }
        } // Close if !using_local_stt

        let confidence = calculate_confidence(&final_text, duration_sec);
        Ok((final_text, confidence))
    }
}

pub fn write_wav_to_bytes(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let mut spec = Vec::new();

    spec.extend_from_slice(b"RIFF");
    let num_samples = samples.len();
    let data_size = num_samples * 2;
    let file_size = 36 + data_size;
    spec.extend_from_slice(&(file_size as u32).to_le_bytes());
    spec.extend_from_slice(b"WAVE");

    spec.extend_from_slice(b"fmt ");
    spec.extend_from_slice(&(16u32).to_le_bytes());
    spec.extend_from_slice(&(1u16).to_le_bytes());
    spec.extend_from_slice(&(1u16).to_le_bytes());
    spec.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate: u32 = sample_rate * 1 * 2;
    spec.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align: u16 = 1 * 2;
    spec.extend_from_slice(&block_align.to_le_bytes());
    spec.extend_from_slice(&(16u16).to_le_bytes());

    spec.extend_from_slice(b"data");
    spec.extend_from_slice(&(data_size as u32).to_le_bytes());

    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let scaled = (clamped * 32767.0) as i16;
        spec.extend_from_slice(&scaled.to_le_bytes());
    }

    spec
}

pub fn calculate_confidence(text: &str, audio_duration_sec: f32) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    let total_words = words.len();
    if total_words == 0 {
        return 0.0;
    }

    let mut unclear_count = 0;
    for word in &words {
        if word.contains("[unclear") {
            unclear_count += 1;
        }
    }

    let unclear_ratio = unclear_count as f32 / total_words as f32;
    let mut confidence = 1.0 - unclear_ratio;

    // Sparse words for long audio → likely incomplete STT
    let expected_min_words = (audio_duration_sec * 1.2) as usize; // ~72 wpm floor
    if total_words + 2 < expected_min_words && audio_duration_sec > 4.0 {
        confidence -= 0.25;
    }

    if total_words < 3 && audio_duration_sec > 3.0 {
        confidence -= 0.3;
    }

    confidence.clamp(0.0, 1.0)
}

async fn call_local_whisper(url: &str, wav_bytes: Vec<u8>) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(1500)) // Fast timeout for local network
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let form = reqwest::multipart::Form::new()
        .part("file", part);

    let response = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Local Whisper API returned status {}", response.status()));
    }

    #[derive(serde::Deserialize)]
    struct WhisperResponse {
        text: String,
    }

    let resp_json: WhisperResponse = response.json()
        .await
        .map_err(|e| format!("Failed to parse JSON response: {}", e))?;

    Ok(resp_json.text.trim().to_string())
}
