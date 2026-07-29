//! One Gemini request per complete spoken utterance.
//! Short VAD fragments are merged; sub-1s / silence / duplicates never hit the API.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

/// Hard floor: never send audio shorter than this (16 kHz mono).
pub const MIN_UTTERANCE_SAMPLES: usize = 16_000; // 1.0 s
/// Max merge wait for consecutive short VAD fragments of the same utterance.
pub const MERGE_WINDOW: Duration = Duration::from_millis(2_000);
/// Minimum gap between accepted API submissions (anti-spam / anti-dupe).
pub const MIN_REQUEST_GAP: Duration = Duration::from_millis(1_200);

pub struct ChunkOptimizer {
    pending: Vec<f32>,
    pending_started: Option<Instant>,
    last_audio_hash: u64,
    last_submit_at: Option<Instant>,
}

impl ChunkOptimizer {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            pending_started: None,
            last_audio_hash: 0,
            last_submit_at: None,
        }
    }

    /// Push a VAD-complete utterance fragment.
    /// Returns `Some(samples)` only when ready for **one** Gemini call.
    pub fn push_utterance(&mut self, samples: &[f32]) -> Option<Vec<f32>> {
        if samples.is_empty() || is_mostly_silence(samples) {
            return None;
        }

        // Coalesce short fragments into one sentence buffer.
        if samples.len() < MIN_UTTERANCE_SAMPLES {
            if self.pending.is_empty() {
                self.pending_started = Some(Instant::now());
            }
            self.pending.extend_from_slice(samples);

            if self.pending.len() >= MIN_UTTERANCE_SAMPLES {
                return self.take_ready();
            }

            // Still too short — wait for more speech or merge-window discard.
            return None;
        }

        // Long enough fragment: fold any pending shorts into it first.
        if !self.pending.is_empty() {
            self.pending.extend_from_slice(samples);
            return self.take_ready();
        }

        self.finalize(samples.to_vec())
    }

    /// On capture stop: send only if we already have a full utterance.
    pub fn flush_pending(&mut self) -> Option<Vec<f32>> {
        if self.pending.len() < MIN_UTTERANCE_SAMPLES {
            self.pending.clear();
            self.pending_started = None;
            return None;
        }
        self.take_ready()
    }

    /// Drop stale short fragments — never send them to Gemini.
    pub fn discard_expired_short(&mut self) {
        let expired = self
            .pending_started
            .map(|t| t.elapsed() >= MERGE_WINDOW)
            .unwrap_or(false);
        if expired && self.pending.len() < MIN_UTTERANCE_SAMPLES {
            self.pending.clear();
            self.pending_started = None;
        } else if expired && self.pending.len() >= MIN_UTTERANCE_SAMPLES {
            // Should have been flushed already; keep until take_ready.
        }
    }

    /// If merge window elapsed and buffer is long enough, release once.
    pub fn flush_if_ready(&mut self) -> Option<Vec<f32>> {
        let expired = self
            .pending_started
            .map(|t| t.elapsed() >= MERGE_WINDOW)
            .unwrap_or(false);
        if !expired {
            return None;
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

    fn finalize(&mut self, samples: Vec<f32>) -> Option<Vec<f32>> {
        if samples.len() < MIN_UTTERANCE_SAMPLES || is_mostly_silence(&samples) {
            return None;
        }

        // Rate-limit: never double-fire the same utterance window.
        if let Some(at) = self.last_submit_at {
            if at.elapsed() < MIN_REQUEST_GAP {
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
    rms < 0.008
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
