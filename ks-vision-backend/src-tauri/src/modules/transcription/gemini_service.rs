//! Transcription facade: LOCAL Whisper only → cleaned TEXT.
//! Gemini never receives WAV/PCM/base64 audio from this path.

use serde::{Deserialize, Serialize};
use crate::modules::audio::preprocess;
use super::whisper_engine::WhisperEngine;
use super::postprocess;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TranscriptChunk {
    pub text: String,
    pub speaker: String,
    pub timestamp: u64,
}

/// Kept name for compatibility with AudioState wiring.
pub struct GeminiTranscriptionService {
    whisper: WhisperEngine,
}

impl GeminiTranscriptionService {
    pub fn new() -> Self {
        Self {
            whisper: WhisperEngine::new(),
        }
    }

    /// Speech → local Whisper → postprocess → (text, confidence).
    /// Never uploads audio to Gemini.
    pub async fn transcribe(
        &self,
        samples: &[f32],
        source_label: &str,
        speaker_id: &str,
        previous_context: &[TranscriptChunk],
    ) -> Result<(String, f32), String> {
        println!("\n[TRANSCRIBE] ==================================================");
        println!(
            "[TRANSCRIBE] source={} speaker={} samples={}",
            source_label,
            speaker_id,
            samples.len()
        );

        let prepared = preprocess::prepare_for_stt(samples);
        if prepared.len() < 1600 {
            println!("[VAD] Segment too short after trim — skip");
            println!("[TRANSCRIBE] ==================================================\n");
            return Ok((String::new(), 0.0));
        }

        let max_samples = 20 * 16000;
        let prepared = if prepared.len() > max_samples {
            prepared[prepared.len() - max_samples..].to_vec()
        } else {
            prepared
        };

        println!("[VAD] Speech segment ready ({:.2}s)", prepared.len() as f32 / 16000.0);
        println!("[Whisper] Transcribing...");

        let result = self.whisper.transcribe(&prepared, 16000).await?;

        if result.text.trim().is_empty() {
            println!("[Whisper] Empty transcript");
            println!("[TRANSCRIBE] ==================================================\n");
            return Ok((String::new(), 0.0));
        }

        let mut ctx = String::new();
        for chunk in previous_context.iter().take(3) {
            ctx.push_str(&chunk.text);
            ctx.push(' ');
        }

        let cleaned = postprocess::postprocess_transcript(&result.text, &ctx);
        let confidence = if result.confidence > 0.0 {
            result.confidence
        } else {
            calculate_confidence(&cleaned, prepared.len() as f32 / 16000.0)
        };

        println!("[Whisper] Confidence: {:.2}", confidence);
        println!("[Transcript] Updated: {}", cleaned);
        println!("[Gemini] Sending text only — NEVER audio");
        println!("[TRANSCRIBE] ==================================================\n");

        // Silence / no-speech markers
        let lower = cleaned.to_lowercase();
        if lower.is_empty() || lower == "(silence)" || lower == "silence" {
            return Ok((String::new(), 0.0));
        }

        Ok((cleaned, confidence))
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

    let expected_min_words = (audio_duration_sec * 1.2) as usize;
    if total_words + 2 < expected_min_words && audio_duration_sec > 4.0 {
        confidence -= 0.25;
    }

    if total_words < 3 && audio_duration_sec > 3.0 {
        confidence -= 0.3;
    }

    confidence.clamp(0.0, 1.0)
}
