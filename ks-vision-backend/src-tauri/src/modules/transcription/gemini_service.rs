use serde::{Deserialize, Serialize};
use base64::Engine;
use crate::modules::transcription::transcript_corrector::TranscriptCorrector;

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
        let gemini_key = self.load_api_key();
        if gemini_key.is_empty() {
            return Err("Gemini API key is not configured. Please add GEMINI_API_KEY to your .env file.".to_string());
        }

        let wav_bytes = write_wav_to_bytes(samples, 16000);
        let duration_sec = samples.len() as f32 / 16000.0;

        // Build context prompt to help resolve technical names & ambiguous keywords
        let mut context_prompt = String::from("Previous conversation context (last 3 utterances):\n");
        if previous_context.is_empty() {
            context_prompt.push_str("None\n");
        } else {
            for chunk in previous_context.iter().take(3) {
                context_prompt.push_str(&format!("[{} (Speaker: {})]: {}\n", source_label, chunk.speaker, chunk.text));
            }
        }

        let mut raw_text_temp = String::new();
        let mut using_local_stt = false;

        let local_url = std::env::var("LOCAL_WHISPER_URL")
            .unwrap_or_else(|_| "http://localhost:8080/v1/audio/transcriptions".to_string());

        // 1. Attempt local Whisper.cpp STT
        println!("[TRANSCRIBE] Attempting local Whisper.cpp STT at {}...", local_url);
        match call_local_whisper(&local_url, wav_bytes.clone()).await {
            Ok(text) => {
                raw_text_temp = text;
                using_local_stt = true;
                println!("[TRANSCRIBE] Local Whisper STT succeeded: \"{}\"", raw_text_temp);
            }
            Err(err) => {
                println!("[TRANSCRIBE INFO] Local Whisper STT unavailable ({:?}). Using Gemini fallback...", err);
            }
        }

        let raw_text = if using_local_stt {
            raw_text_temp
        } else {
            // 2. Fallback to Gemini multimodal audio API
            println!("[TRANSCRIBE] Using Gemini multimodal audio API...");
            let b64_data = base64::engine::general_purpose::STANDARD.encode(&wav_bytes);
            
            let system_instruction = "You are an expert speech-to-text system. Transcribe the following audio with these rules:
1. PUNCTUATION: Add proper punctuation, commas, and sentence breaks.
2. RESTARTS: If the speaker starts a sentence, pauses, and restarts, transcribe ONLY the final corrected version. Do not include false starts.
3. FILLERS: Remove verbal fillers ('um', 'uh', 'like', 'you know').
4. QUESTIONS: Ensure questions end with '?'.
5. NAMES/TERMS: Preserve technical terms, acronyms, and proper nouns exactly as spoken.
6. SPEAKER STYLE: Do not break the speaker's thoughts into separate sentences if pauses are natural thinking pauses.
7. CONTEXT: Previous conversation context is provided. Use it to disambiguate homophones.";

            let user_prompt = format!(
                "{}\nTranscribe this audio. The speaker ({}) is currently in 'thinking/speaking' mode with natural pauses. Output ONLY the transcription text, no preamble.",
                context_prompt, speaker_id
            );

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
                }]
            });

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(12))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());

            let manager = crate::modules::ai::model_manager::GeminiModelManager::global();
            let mut attempts = 0;
            let final_text;

            loop {
                let active_model = manager.select_model();
                println!("[TRANSCRIBE] Using: {}", active_model);

                let url = format!(
                    "https://generativelanguage.googleapis.com/v1/models/{}:generateContent?key={}",
                    active_model, gemini_key
                );

                let resp = match client.post(&url).json(&payload).send().await {
                    Ok(r) => r,
                    Err(e) => return Err(format!("Failed to send transcription request: {}", e)),
                };

                let status_code = resp.status();
                if !status_code.is_success() {
                    let err_text = resp.text().await.unwrap_or_default();
                    eprintln!("[TRANSCRIBE ERROR] Gemini API returned status {}: {}", status_code, err_text);

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
                    return Err(format!("Gemini API returned error status {}: {}", status_code, err_text));
                }

                let gemini_resp: GeminiResponse = resp.json()
                    .await
                    .map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

                final_text = gemini_resp.candidates
                    .and_then(|c| c.into_iter().next())
                    .and_then(|c| c.content)
                    .and_then(|c| c.parts)
                    .and_then(|p| p.into_iter().next())
                    .and_then(|p| p.text)
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                break;
            }
            final_text
        };

        // 3. Confidence Evaluation & Correction Engine
        let confidence = calculate_confidence(&raw_text, duration_sec);

        if confidence < 0.75 {
            // Low confidence: Invoke Gemini to validate and semantically repair transcription
            println!("[TRANSCRIBE] Low confidence ({:.2}) -> Triggering Gemini semantic validation...", confidence);
            match correct_transcript_with_gemini(&gemini_key, &raw_text, &context_prompt).await {
                Ok(corrected) => {
                    let boosted = TranscriptCorrector::correct(&corrected);
                    Ok((boosted, confidence.max(0.75))) // Boost confidence post-validation
                }
                Err(err) => {
                    eprintln!("[TRANSCRIBE WARNING] Gemini semantic validation failed: {}", err);
                    let boosted = TranscriptCorrector::correct(&raw_text);
                    Ok((boosted, confidence))
                }
            }
        } else {
            // High confidence: Directly run local technical term boosting, skipping extra remote call
            let boosted = TranscriptCorrector::correct(&raw_text);
            Ok((boosted, confidence))
        }
    }
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

async fn correct_transcript_with_gemini(api_key: &str, text: &str, context_prompt: &str) -> Result<String, String> {
    let manager = crate::modules::ai::model_manager::GeminiModelManager::global();
    let active_model = manager.select_model();
    
    let url = format!(
        "https://generativelanguage.googleapis.com/v1/models/{}:generateContent?key={}",
        active_model, api_key
    );

    let system_instruction = "You are an advanced audio transcript correction system. Correct acoustic recognition typos, spelling errors, and grammar fragmentations while preserving technical terms (e.g. SQLite, Rust, Tauri). Do not alter the meaning. Return ONLY the final corrected text.";
    
    let payload = serde_json::json!({
        "systemInstruction": {
            "parts": [{ "text": system_instruction }]
        },
        "contents": [{
            "parts": [{ "text": format!("Context:\n{}\n\nLow-confidence Transcript:\n\"{}\"\n\nCorrected output:", context_prompt, text) }]
        }]
    });

    let client = reqwest::Client::new();
    let resp = client.post(&url).json(&payload).send().await
        .map_err(|e| format!("Failed to send validation request: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Validation failed with status {}", resp.status()));
    }

    let gemini_resp: GeminiResponse = resp.json().await
        .map_err(|e| format!("Failed to parse validation response: {}", e))?;

    let corrected_text = gemini_resp.candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content)
        .and_then(|c| c.parts)
        .and_then(|p| p.into_iter().next())
        .and_then(|p| p.text)
        .unwrap_or_default()
        .trim()
        .to_string();

    Ok(corrected_text)
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
    
    if total_words < 3 && audio_duration_sec > 3.0 {
        confidence -= 0.3;
    }
    
    confidence.clamp(0.0, 1.0)
}
