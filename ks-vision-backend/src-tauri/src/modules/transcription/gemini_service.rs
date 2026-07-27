use serde::{Deserialize, Serialize};
use base64::Engine;
use crate::modules::audio::commands::CaptureStateResponse;

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
        // Try environment first
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

        let wav_bytes = write_wav_to_bytes(samples, 16000);
        let b64_data = base64::engine::general_purpose::STANDARD.encode(&wav_bytes);

        // Format previous context
        let mut context_prompt = String::from("Previous conversation context (last 3 utterances):\n");
        if previous_context.is_empty() {
            context_prompt.push_str("None\n");
        } else {
            for chunk in previous_context.iter().take(3) {
                context_prompt.push_str(&format!("[{} (Speaker: {})]: {}\n", source_label, chunk.speaker, chunk.text));
            }
        }

        let system_instruction = "You are an expert speech-to-text system. Transcribe the following audio with these rules:
1. PUNCTUATION: Add proper punctuation, commas, and sentence breaks based on natural pauses.
2. RESTARTS: If the speaker starts a sentence, pauses, and restarts ('I mean...', 'Actually...'), transcribe ONLY the final corrected version. Do not include false starts.
3. FILLERS: Remove verbal fillers ('um', 'uh', 'like', 'you know') unless they carry semantic meaning.
4. QUESTIONS: Ensure questions end with '?'.
5. NAMES/TERMS: Preserve technical terms, acronyms, and proper nouns exactly as spoken.
6. UNCERTAIN: If a word is unclear, mark it with [unclear: 'sounds-like'].
7. SPEAKER STYLE: The speaker may pause to think. Do not break their thought into separate sentences if the pauses are natural thinking pauses.
8. CONTEXT: Previous conversation context is provided. Use it to disambiguate homophones and predict likely words.";

        let user_prompt = format!(
            "{}\nTranscribe this audio. The speaker ({}) is currently in 'thinking/speaking' mode with natural pauses. Output ONLY the transcription text, no preamble or summary.",
            context_prompt, speaker_id
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
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
            }]
        });

        let manager = crate::modules::ai::model_manager::GeminiModelManager::global();
        let mut attempts = 0;

        loop {
            let active_model = manager.select_model();
            println!("[TRANSCRIBE] Using: {}", active_model);

            let url = format!(
                "https://generativelanguage.googleapis.com/v1/models/{}:generateContent?key={}",
                active_model, api_key
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

            let text = gemini_resp.candidates
                .and_then(|c| c.into_iter().next())
                .and_then(|c| c.content)
                .and_then(|c| c.parts)
                .and_then(|p| p.into_iter().next())
                .and_then(|p| p.text)
                .unwrap_or_default()
                .trim()
                .to_string();

            let duration_sec = samples.len() as f32 / 16000.0;
            let confidence = calculate_confidence(&text, duration_sec);

            return Ok((text, confidence));
        }
    }
}

pub fn write_wav_to_bytes(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let mut spec = Vec::new();
    
    // RIFF header
    spec.extend_from_slice(b"RIFF");
    let num_samples = samples.len();
    let data_size = num_samples * 2; // 16-bit PCM = 2 bytes per sample
    let file_size = 36 + data_size;
    spec.extend_from_slice(&(file_size as u32).to_le_bytes());
    spec.extend_from_slice(b"WAVE");
    
    // fmt chunk
    spec.extend_from_slice(b"fmt ");
    spec.extend_from_slice(&(16u32).to_le_bytes()); // subchunk size
    spec.extend_from_slice(&(1u16).to_le_bytes());   // audio format: 1 (PCM)
    spec.extend_from_slice(&(1u16).to_le_bytes());   // channels: 1 (mono)
    spec.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate: u32 = sample_rate * 1 * 2;
    spec.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align: u16 = 1 * 2;
    spec.extend_from_slice(&block_align.to_le_bytes());
    spec.extend_from_slice(&(16u16).to_le_bytes()); // bits per sample: 16
    
    // data chunk
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
