//! Pack VAD fragments into ONE sentence per Gemini request.
//! Always flush after the merge window — never leave speech stuck in pending.

use std::time::{Duration, Instant};

/// ~0.7s @ 16 kHz
pub const MIN_UTTERANCE_SAMPLES: usize = 11_200;
pub const MERGE_WINDOW: Duration = Duration::from_millis(1_500);
/// Absolute floor to send after waiting (~0.5s)
const FLUSH_MIN_SAMPLES: usize = 8_000;

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

    pub fn push_utterance(&mut self, samples: &[f32]) -> Option<Vec<f32>> {
        if samples.is_empty() || is_mostly_silence(samples) {
            return None;
        }

        // Complete sentence already — send as one request immediately.
        if samples.len() >= MIN_UTTERANCE_SAMPLES && self.pending.is_empty() {
            return Some(samples.to_vec());
        }

        if self.pending.is_empty() {
            self.pending_started = Some(Instant::now());
        }
        self.pending.extend_from_slice(samples);

        if self.pending.len() >= MIN_UTTERANCE_SAMPLES {
            return self.take();
        }

        // Timed flush of shorts so we never stall after a few questions.
        let expired = self
            .pending_started
            .map(|t| t.elapsed() >= MERGE_WINDOW)
            .unwrap_or(false);
        if expired && self.pending.len() >= FLUSH_MIN_SAMPLES {
            return self.take();
        }

        None
    }

    pub fn flush_pending(&mut self) -> Option<Vec<f32>> {
        if self.pending.len() < FLUSH_MIN_SAMPLES || is_mostly_silence(&self.pending) {
            self.pending.clear();
            self.pending_started = None;
            return None;
        }
        self.take()
    }

    pub fn discard_expired_short(&mut self) {
        let expired = self
            .pending_started
            .map(|t| t.elapsed() >= MERGE_WINDOW)
            .unwrap_or(false);
        if expired && self.pending.len() < FLUSH_MIN_SAMPLES {
            self.pending.clear();
            self.pending_started = None;
        }
    }

    pub fn flush_if_ready(&mut self) -> Option<Vec<f32>> {
        let expired = self
            .pending_started
            .map(|t| t.elapsed() >= MERGE_WINDOW)
            .unwrap_or(false);
        if !expired {
            return None;
        }
        if self.pending.len() >= FLUSH_MIN_SAMPLES && !is_mostly_silence(&self.pending) {
            return self.take();
        }
        // Clear stuck crumbs so future sentences aren't blocked.
        self.pending.clear();
        self.pending_started = None;
        None
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
