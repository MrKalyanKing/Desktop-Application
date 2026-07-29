//! One Gemini request per complete spoken utterance.
//! Short VAD fragments merge; do not silently drop the second half of a question.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

/// Hard floor @ 16 kHz mono (~0.75s) — short tech questions still get through.
pub const MIN_UTTERANCE_SAMPLES: usize = 12_000;
/// Merge window for consecutive short VAD fragments of the same utterance.
pub const MERGE_WINDOW: Duration = Duration::from_millis(2_500);
/// Gap between sends — short enough to keep answering sequential questions.
pub const MIN_REQUEST_GAP: Duration = Duration::from_millis(450);

pub struct ChunkOptimizer {
    pending: Vec<f32>,
    pending_started: Option<Instant>,
    last_audio_hash: u64,
    last_submit_at: Option<Instant>,
    /// Held while rate-limited so the next utterance is not discarded.
    deferred: Option<Vec<f32>>,
}

impl ChunkOptimizer {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            pending_started: None,
            last_audio_hash: 0,
            last_submit_at: None,
            deferred: None,
        }
    }

    pub fn push_utterance(&mut self, samples: &[f32]) -> Option<Vec<f32>> {
        if samples.is_empty() || is_mostly_silence(samples) {
            return None;
        }

        if samples.len() < MIN_UTTERANCE_SAMPLES {
            if self.pending.is_empty() {
                self.pending_started = Some(Instant::now());
            }
            self.pending.extend_from_slice(samples);

            if self.pending.len() >= MIN_UTTERANCE_SAMPLES {
                return self.take_ready();
            }
            return None;
        }

        if !self.pending.is_empty() {
            self.pending.extend_from_slice(samples);
            return self.take_ready();
        }

        self.finalize(samples.to_vec())
    }

    pub fn flush_pending(&mut self) -> Option<Vec<f32>> {
        if let Some(d) = self.deferred.take() {
            if d.len() >= MIN_UTTERANCE_SAMPLES {
                return self.finalize(d);
            }
        }
        if self.pending.len() < MIN_UTTERANCE_SAMPLES {
            // Still send reasonably long shorts on stop (≥0.6s).
            if self.pending.len() >= 9_600 && !is_mostly_silence(&self.pending) {
                return self.take_ready_force();
            }
            self.pending.clear();
            self.pending_started = None;
            return None;
        }
        self.take_ready()
    }

    pub fn discard_expired_short(&mut self) {
        let expired = self
            .pending_started
            .map(|t| t.elapsed() >= MERGE_WINDOW)
            .unwrap_or(false);
        if !expired {
            return;
        }
        // Prefer sending ≥0.6s speech over discarding it.
        if self.pending.len() >= 9_600 && !is_mostly_silence(&self.pending) {
            return;
        }
        if self.pending.len() < MIN_UTTERANCE_SAMPLES {
            self.pending.clear();
            self.pending_started = None;
        }
    }

    pub fn flush_if_ready(&mut self) -> Option<Vec<f32>> {
        if let Some(d) = self.deferred.take() {
            if let Some(out) = self.finalize(d) {
                return Some(out);
            }
        }

        let expired = self
            .pending_started
            .map(|t| t.elapsed() >= MERGE_WINDOW)
            .unwrap_or(false);
        if !expired {
            return None;
        }
        if self.pending.len() >= 9_600 && !is_mostly_silence(&self.pending) {
            return self.take_ready_force();
        }
        if self.pending.len() < MIN_UTTERANCE_SAMPLES {
            self.pending.clear();
            self.pending_started = None;
            return None;
        }
        self.take_ready()
    }

    fn take_ready(&mut self) -> Option<Vec<f32>> {
        if self.pending.is_empty() {
            return None;
        }
        let buf = std::mem::take(&mut self.pending);
        self.pending_started = None;
        self.finalize(buf)
    }

    fn take_ready_force(&mut self) -> Option<Vec<f32>> {
        if self.pending.is_empty() {
            return None;
        }
        let buf = std::mem::take(&mut self.pending);
        self.pending_started = None;
        self.finalize(buf)
    }

    fn finalize(&mut self, samples: Vec<f32>) -> Option<Vec<f32>> {
        if samples.len() < 9_600 || is_mostly_silence(&samples) {
            return None;
        }

        if let Some(at) = self.last_submit_at {
            if at.elapsed() < MIN_REQUEST_GAP {
                // Keep for next tick — do not drop the other person's next sentence.
                self.deferred = Some(samples);
                return None;
            }
        }

        let hash = audio_fingerprint(&samples);
        if hash == self.last_audio_hash {
            return None;
        }

        self.last_audio_hash = hash;
        self.last_submit_at = Some(Instant::now());
        Some(samples)
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
    let rms = (sum_sq / samples.len() as f32).sqrt();
    rms < 0.006
}

fn audio_fingerprint(samples: &[f32]) -> u64 {
    let mut hasher = DefaultHasher::new();
    let step = (samples.len() / 256).max(1);
    for (i, &s) in samples.iter().step_by(step).enumerate() {
        if i > 256 {
            break;
        }
        let q = (s * 64.0).round() as i16;
        q.hash(&mut hasher);
    }
    samples.len().hash(&mut hasher);
    hasher.finish()
}
