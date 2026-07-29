use crate::modules::audio::vad_engine::{VADEngine, is_pitch_rising, estimate_pitch};
use crate::modules::audio::capture_engine::AudioSource;
use crate::modules::audio::speaker_tracker::SpeakerId;

#[derive(Debug, Clone)]
pub struct ProsodyFeatures {
    pub mean_pitch: f32,
    pub pitch_range: f32,
    pub energy_contour: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct ConfidenceHint {
    pub label: String,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct UtteranceSegment {
    pub audio_data: Vec<f32>, // Standardized 16kHz mono samples
    pub source: AudioSource,
    pub speaker_id: Option<SpeakerId>,
    pub prosody: ProsodyFeatures,
    pub confidence_hints: Vec<ConfidenceHint>,
}

pub struct SegmentationEngine {
    vad: VADEngine,
}

impl SegmentationEngine {
    pub fn new() -> Self {
        Self {
            vad: VADEngine::new(),
        }
    }

    pub fn get_vad(&self) -> &VADEngine {
        &self.vad
    }

    /// Evaluates if an utterance boundary has been reached based on the "Human Speech" algorithm.
    /// A boundary is declared when ANY TWO of these conditions are met:
    /// 1. Temporal: Silence duration > timeout (handled by VAD transitioning to Silent)
    /// 2. Prosodic: Final pitch contour falls > 20% or rises then falls
    /// 3. Semantic: Last few words contain sentence-final particles or ends with complete punctuation
    pub fn check_boundary(
        &self,
        samples: &[f32],
        vad_silence_triggered: bool,
        interim_transcript: &str,
    ) -> bool {
        let mut conditions_met = 0;

        // 1. Temporal Condition
        if vad_silence_triggered {
            conditions_met += 1;
        }

        // 2. Prosodic Condition
        if check_prosodic_boundary(samples, 16000) {
            conditions_met += 1;
        }

        // 3. Semantic Condition
        if check_semantic_boundary(interim_transcript) {
            conditions_met += 1;
        }

        conditions_met >= 2
    }

    /// Detects sentence restarts and returns the number of samples to discard from the front.
    /// Discards false-start audio, leaving 1.5 seconds of context before the restart word.
    pub fn detect_restart(&self, interim_transcript: &str, total_samples: usize) -> Option<usize> {
        let restart_patterns = [
            r"\b(i mean)\b",
            r"\b(actually)\b",
            r"\b(wait)\b",
            r"\b(no no)\b",
            r"\b(sorry)\b",
            r"\b(let me rephrase)\b",
            r"\b(what i meant was)\b",
            r"\b(to be clear)\b",
            r"\b(rather)\b",
        ];

        let text_lower = interim_transcript.to_lowercase();
        let mut match_pos = None;

        for &pattern in &restart_patterns {
            if let Some(idx) = text_lower.find(pattern.trim_matches('\\').trim_matches('b')) {
                if match_pos.is_none() || idx < match_pos.unwrap() {
                    match_pos = Some(idx);
                }
            }
        }

        if let Some(pos) = match_pos {
            // Find word position (estimate the fraction of the utterance elapsed)
            let total_chars = text_lower.len();
            if total_chars == 0 {
                return None;
            }
            
            let progress_fraction = pos as f64 / total_chars as f64;
            let restart_sample_idx = (total_samples as f64 * progress_fraction) as usize;

            // Rewind by 1.5 seconds before the restart word (1.5s * 16000 samples/s = 24000 samples)
            let context_samples = 24000;
            let discard_count = if restart_sample_idx > context_samples {
                restart_sample_idx - context_samples
            } else {
                0
            };

            if discard_count > 0 && discard_count < total_samples {
                return Some(discard_count);
            }
        }

        None
    }
}

pub fn check_prosodic_boundary(samples: &[f32], sample_rate: u32) -> bool {
    let chunk_size = 800; // 50ms at 16kHz
    let mut pitches = Vec::new();
    for chunk in samples.chunks(chunk_size) {
        if chunk.len() >= 320 {
            if let Some(p) = estimate_pitch(chunk, sample_rate) {
                pitches.push(p);
            }
        }
    }
    if pitches.len() < 4 {
        return false;
    }
    
    let sum: f32 = pitches.iter().sum();
    let mean = sum / pitches.len() as f32;
    let last = pitches[pitches.len() - 1];
    
    // Final pitch contour falls > 20% from mean
    if last < mean * 0.8 {
        return true;
    }
    
    // Rises then falls (question end)
    let len = pitches.len();
    if pitches[len - 2] > pitches[len - 3] && pitches[len - 1] < pitches[len - 2] {
        return true;
    }
    
    false
}

pub fn check_semantic_boundary(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Ends with complete sentence punctuation
    if trimmed.ends_with('.') || trimmed.ends_with('?') || trimmed.ends_with('!') {
        return true;
    }

    // Check if the last 3 words contain particles
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    let semantic_particles = ["right?", "right", "correct?", "correct", "please", "thanks", "thank you"];
    
    let start_idx = if words.len() > 3 { words.len() - 3 } else { 0 };
    for i in start_idx..words.len() {
        let cleaned = words[i].trim_matches(|c: char| !c.is_alphanumeric() && c != '?').to_lowercase();
        for &particle in &semantic_particles {
            if cleaned == particle {
                return true;
            }
        }
    }
    
    false
}
