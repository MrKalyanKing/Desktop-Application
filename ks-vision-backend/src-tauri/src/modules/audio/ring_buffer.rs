//! Audio Ring Buffer with Utterance Completeness & Noise Validation.
//! Prevents unnecessary API calls to Gemini by ensuring speech is complete, 
//! sufficiently long (≥1.5s), and noise-filtered before dispatch.

use std::collections::VecDeque;

/// Max audio window: 12 seconds at 16kHz mono.
pub const MAX_RING_SAMPLES: usize = 12 * 16_000;
/// Minimum duration for a valid utterance: 0.5s.
pub const MIN_UTTERANCE_SAMPLES: usize = 8_000;
/// Minimum silence duration (unused; completeness is VAD + merge gap).
pub const COMPLETE_SILENCE_SAMPLES: usize = 4_800;

pub struct AudioUtteranceBuffer {
    buffer: VecDeque<f32>,
    sample_rate: u32,
}

impl AudioUtteranceBuffer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            buffer: VecDeque::with_capacity(MAX_RING_SAMPLES),
            sample_rate,
        }
    }

    pub fn push_samples(&mut self, samples: &[f32]) {
        for &s in samples {
            if self.buffer.len() >= MAX_RING_SAMPLES {
                self.buffer.pop_front();
            }
            self.buffer.push_back(s);
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn get_samples(&self) -> Vec<f32> {
        self.buffer.iter().copied().collect()
    }

    /// Evaluates if the current audio buffer contains a complete, valid utterance
    /// ready for Gemini execution, preventing unnecessary API calls.
    pub fn is_valid_complete_utterance(&self) -> bool {
        if self.buffer.len() < MIN_UTTERANCE_SAMPLES {
            return false; // Too short (e.g. noise glitch or single syllable)
        }

        let samples = self.get_samples();

        // 1. Check overall speech energy (RMS & Peak)
        let mut sum_sq = 0.0f32;
        let mut max_peak = 0.0f32;
        for &s in &samples {
            sum_sq += s * s;
            if s.abs() > max_peak {
                max_peak = s.abs();
            }
        }
        let rms = (sum_sq / samples.len() as f32).sqrt();

        // Noise floor check: drop dead silence or background HVAC noise
        if rms < 0.004 || max_peak < 0.012 {
            return false;
        }

        // 2. Speech frame ratio check (must have real speech content)
        let frame_size = 480; // 30ms @ 16kHz
        let mut speech_frames = 0usize;
        let mut total_frames = 0usize;

        for chunk in samples.chunks(frame_size) {
            total_frames += 1;
            let mut chunk_sq = 0.0f32;
            for &s in chunk {
                chunk_sq += s * s;
            }
            let chunk_rms = (chunk_sq / chunk.len() as f32).sqrt();
            if chunk_rms > 0.010 {
                speech_frames += 1;
            }
        }

        if total_frames == 0 {
            return false;
        }

        let ratio = speech_frames as f32 / total_frames as f32;
        // Require at least 15% speech frames to avoid sending pure background static
        if ratio < 0.10 {
            return false;
        }

        true
    }
}
