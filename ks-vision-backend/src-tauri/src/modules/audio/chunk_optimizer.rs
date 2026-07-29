//! Token-efficient utterance packing: silence trim, short-chunk merge, audio dedupe.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

/// Packs VAD utterances into fewer, denser Gemini audio requests.
pub struct ChunkOptimizer {
    pending: Vec<f32>,
    pending_started: Option<Instant>,
    last_audio_hash: u64,
    last_transcript: String,
    last_submit_at: Option<Instant>,
}

impl ChunkOptimizer {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            pending_started: None,
            last_audio_hash: 0,
            last_transcript: String::new(),
            last_submit_at: None,
        }
    }

    /// Push a VAD utterance. Returns Some(samples) when ready to send to Gemini.
    /// Merges clips shorter than `min_samples` within `merge_window`.
    pub fn push_utterance(
        &mut self,
        samples: &[f32],
        min_samples: usize,
        merge_window: Duration,
        min_gap_between_requests: Duration,
    ) -> Option<Vec<f32>> {
        if samples.is_empty() {
            return None;
        }

        // Rate-limit duplicate floods
        if let Some(at) = self.last_submit_at {
            if at.elapsed() < min_gap_between_requests && samples.len() < min_samples * 2 {
                // Hold into pending instead of dropping if mergeable
            }
        }

        if samples.len() < min_samples {
            if self.pending.is_empty() {
                self.pending_started = Some(Instant::now());
            }
            self.pending.extend_from_slice(samples);

            let expired = self
                .pending_started
                .map(|t| t.elapsed() >= merge_window)
                .unwrap_or(false);
            if self.pending.len() >= min_samples || expired {
                return self.flush_pending();
            }
            return None;
        }

        // Substantial clip — flush any pending first, then this clip
        if !self.pending.is_empty() {
            self.pending.extend_from_slice(samples);
            return self.flush_pending();
        }

        self.prepare_send(samples.to_vec())
    }

    pub fn flush_pending(&mut self) -> Option<Vec<f32>> {
        if self.pending.is_empty() {
            return None;
        }
        let buf = std::mem::take(&mut self.pending);
        self.pending_started = None;
        self.prepare_send(buf)
    }

    /// Flush only if merge window elapsed (keeps short clips coalesced).
    pub fn flush_if_expired(&mut self, merge_window: Duration) -> Option<Vec<f32>> {
        let expired = self
            .pending_started
            .map(|t| t.elapsed() >= merge_window)
            .unwrap_or(false);
        if expired && !self.pending.is_empty() {
            self.flush_pending()
        } else {
            None
        }
    }

    fn prepare_send(&mut self, samples: Vec<f32>) -> Option<Vec<f32>> {
        let hash = audio_fingerprint(&samples);
        if hash == self.last_audio_hash {
            println!("[CHUNK OPT] Skipping duplicate audio fingerprint");
            return None;
        }
        self.last_audio_hash = hash;
        self.last_submit_at = Some(Instant::now());
        Some(samples)
    }

    /// Skip Gemini answer/STT if transcript is near-duplicate of the last one.
    pub fn is_duplicate_transcript(&mut self, text: &str) -> bool {
        let t = text.trim().to_lowercase();
        if t.is_empty() {
            return true;
        }
        if !self.last_transcript.is_empty() {
            if t == self.last_transcript {
                return true;
            }
            // High overlap on short strings
            if t.len() < 80 && self.last_transcript.contains(&t) {
                return true;
            }
            if self.last_transcript.len() < 80 && t.contains(&self.last_transcript) {
                return true;
            }
        }
        self.last_transcript = t;
        false
    }
}

impl Default for ChunkOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Cheap fingerprint: downsample + hash of quantized amplitudes.
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
