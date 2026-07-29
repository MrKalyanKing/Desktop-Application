//! Pack VAD fragments into ONE sentence per Gemini request.

use std::time::{Duration, Instant};

/// ~0.8s @ 16 kHz — full short sentence / question.
pub const MIN_UTTERANCE_SAMPLES: usize = 12_800;
/// Merge early VAD cuts of the same sentence before sending.
pub const MERGE_WINDOW: Duration = Duration::from_millis(2_000);

pub struct ChunkOptimizer {
    pending: Vec<f32>,
    pending_started: Option<Instant>,
}

impl ChunkOptimizer {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            pending_started: None,
        }
    }

    /// Merge short fragments into one sentence; return only when ready for a single API call.
    pub fn push_utterance(&mut self, samples: &[f32]) -> Option<Vec<f32>> {
        if samples.is_empty() || is_mostly_silence(samples) {
            return None;
        }

        // Always coalesce into pending while under merge window — one sentence = one request.
        if self.pending.is_empty() {
            self.pending_started = Some(Instant::now());
        }
        self.pending.extend_from_slice(samples);

        let long_enough = self.pending.len() >= MIN_UTTERANCE_SAMPLES;
        let merge_expired = self
            .pending_started
            .map(|t| t.elapsed() >= MERGE_WINDOW)
            .unwrap_or(false);

        // If this fragment alone is already a full sentence and nothing pending before it
        // was empty except what we just added — send when long enough AND (silence merge done
        // is handled by VAD). Send immediately when clearly a complete long utterance.
        if long_enough && samples.len() >= MIN_UTTERANCE_SAMPLES && self.pending.len() == samples.len()
        {
            return self.take();
        }

        // Short pieces: wait for more speech or merge window.
        if long_enough && merge_expired {
            return self.take();
        }
        if long_enough && samples.len() >= MIN_UTTERANCE_SAMPLES {
            // Another full utterance arrived while pending had shorts — flush combined.
            return self.take();
        }

        None
    }

    pub fn flush_pending(&mut self) -> Option<Vec<f32>> {
        if self.pending.len() < 9_600 || is_mostly_silence(&self.pending) {
            self.pending.clear();
            self.pending_started = None;
            return None;
        }
        self.take()
    }

    pub fn discard_expired_short(&mut self) {}

    pub fn flush_if_ready(&mut self) -> Option<Vec<f32>> {
        let expired = self
            .pending_started
            .map(|t| t.elapsed() >= MERGE_WINDOW)
            .unwrap_or(false);
        if !expired || self.pending.is_empty() {
            return None;
        }
        if self.pending.len() < 9_600 || is_mostly_silence(&self.pending) {
            // Keep waiting a bit more only if extremely short; else clear noise.
            if self.pending.len() < 4_800 {
                self.pending.clear();
                self.pending_started = None;
            }
            return None;
        }
        self.take()
    }

    fn take(&mut self) -> Option<Vec<f32>> {
        let buf = std::mem::take(&mut self.pending);
        self.pending_started = None;
        if buf.is_empty() || is_mostly_silence(&buf) {
            None
        } else {
            Some(buf)
        }
    }
}

impl Default for ChunkOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

fn is_mostly_silence(samples: &[f32]) -> bool {
    if samples.is_empty() {
        return true;
    }
    let mut sum_sq = 0.0f32;
    for &s in samples {
        sum_sq += s * s;
    }
    (sum_sq / samples.len() as f32).sqrt() < 0.005
}
