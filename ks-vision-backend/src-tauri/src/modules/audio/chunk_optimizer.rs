//! Pack VAD fragments into ONE complete utterance per Gemini request.
//! Resets the merge timer on every new fragment so mid-sentence pauses
//! don't fire a separate API call (which Gemini then SKIP's).

use std::time::{Duration, Instant};

/// ~1.2s @ 16 kHz — don't bother Gemini with shorter crumbs
pub const MIN_UTTERANCE_SAMPLES: usize = 19_200;
/// Wait this long after the *last* fragment before sending (end of sentence).
pub const MERGE_GAP: Duration = Duration::from_millis(1_800);
/// Hard cap — force send so we never hold forever (~10s)
const MAX_PENDING_SAMPLES: usize = 160_000;
/// Absolute floor after waiting (~0.9s)
const FLUSH_MIN_SAMPLES: usize = 14_400;

pub struct ChunkOptimizer {
    pending: Vec<f32>,
    last_fragment_at: Option<Instant>,
}

impl ChunkOptimizer {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            last_fragment_at: None,
        }
    }

    /// Always buffer. Only emit when a real pause after the last fragment
    /// (or we hit the max length). Never fire on the first short cut alone.
    pub fn push_utterance(&mut self, samples: &[f32]) -> Option<Vec<f32>> {
        if samples.is_empty() || is_mostly_silence(samples) {
            return None;
        }

        self.pending.extend_from_slice(samples);
        self.last_fragment_at = Some(Instant::now());

        if self.pending.len() >= MAX_PENDING_SAMPLES {
            return self.take();
        }
        None
    }

    pub fn flush_pending(&mut self) -> Option<Vec<f32>> {
        if self.pending.len() < FLUSH_MIN_SAMPLES || is_mostly_silence(&self.pending) {
            self.clear();
            return None;
        }
        self.take()
    }

    pub fn discard_expired_short(&mut self) {
        let gap_ok = self
            .last_fragment_at
            .map(|t| t.elapsed() >= MERGE_GAP)
            .unwrap_or(false);
        if gap_ok && self.pending.len() < FLUSH_MIN_SAMPLES {
            self.clear();
        }
    }

    pub fn flush_if_ready(&mut self) -> Option<Vec<f32>> {
        let gap_ok = self
            .last_fragment_at
            .map(|t| t.elapsed() >= MERGE_GAP)
            .unwrap_or(false);
        if !gap_ok {
            return None;
        }
        if self.pending.len() >= FLUSH_MIN_SAMPLES && !is_mostly_silence(&self.pending) {
            return self.take();
        }
        self.clear();
        None
    }

    fn take(&mut self) -> Option<Vec<f32>> {
        let buf = std::mem::take(&mut self.pending);
        self.last_fragment_at = None;
        if buf.is_empty() || is_mostly_silence(&buf) {
            None
        } else {
            Some(buf)
        }
    }

    fn clear(&mut self) {
        self.pending.clear();
        self.last_fragment_at = None;
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
