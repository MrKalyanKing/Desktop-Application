//! Transcription facade: embedded Whisper (in-process) → cleaned TEXT → Gemini text-only.
//! No HTTP :8080, no CLI, no Gemini audio for STT.

use serde::{Deserialize, Serialize};
use crate::modules::audio::preprocess;
use super::embedded_whisper;
use super::postprocess;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TranscriptChunk {
    pub text: String,
    pub speaker: String,
    pub timestamp: u64,
}

pub struct GeminiTranscriptionService;

impl GeminiTranscriptionService {
    pub fn new() -> Self {
        Self
    }

    /// Speech → embedded Whisper → postprocess → (text, confidence).
    /// Gemini never receives audio on this path.
    pub async fn transcribe(
        &self,
        samples: &[f32],
        source_label: &str,
        speaker_id: &str,
        previous_context: &[TranscriptChunk],
    ) -> Result<(String, f32), String> {
        println!("\n[TRANSCRIBE] ==================================================");
        println!(
            "[TRANSCRIBE] source={} speaker={} samples={} (embedded Whisper)",
            source_label,
            speaker_id,
            samples.len()
        );

        if !embedded_whisper::is_ready() {
            // Best-effort late init (packaged / cwd)
            let _ = embedded_whisper::init_embedded_whisper(None);
        }
        if !embedded_whisper::is_ready() {
            return Err(
                "Speech engine not ready. The bundled Whisper model failed to load.".into(),
            );
        }

        let prepared = preprocess::prepare_for_stt(samples);
        if prepared.len() < 1600 {
            println!("[VAD] Segment too short after trim — skip");
            println!("[TRANSCRIBE] ==================================================\n");
            return Ok((String::new(), 0.0));
        }

        println!(
            "[VAD] Speech segment ready ({:.2}s)",
            prepared.len() as f32 / 16000.0
        );

        let (text, confidence) =
            embedded_whisper::transcribe_pcm_async(prepared.clone(), 16000).await?;

        if text.trim().is_empty() {
            println!("[Embedded Whisper] Empty transcript");
            println!("[TRANSCRIBE] ==================================================\n");
            return Ok((String::new(), 0.0));
        }

        let mut ctx = String::new();
        let recent: Vec<&TranscriptChunk> = previous_context.iter().rev().take(2).collect();
        for chunk in recent.into_iter().rev() {
            ctx.push_str(&chunk.text);
            ctx.push(' ');
        }

        let cleaned = postprocess::postprocess_transcript(&text, &ctx);
        let confidence = if confidence > 0.0 {
            confidence
        } else {
            calculate_confidence(&cleaned, prepared.len() as f32 / 16000.0)
        };

        println!("[Embedded Whisper] Confidence: {:.2}", confidence);
        println!("[Transcript] {}", cleaned);
        println!("[Gemini] Text only — never audio");
        println!("[TRANSCRIBE] ==================================================\n");

        let lower = cleaned.to_lowercase();
        if lower.is_empty() || lower == "(silence)" || lower == "silence" {
            return Ok((String::new(), 0.0));
        }

        Ok((cleaned, confidence))
    }
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
