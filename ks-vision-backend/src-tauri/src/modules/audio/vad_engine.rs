use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

pub struct AtomicF32 {
    bits: AtomicU32,
}

impl AtomicF32 {
    pub fn new(val: f32) -> Self {
        Self {
            bits: AtomicU32::new(val.to_bits()),
        }
    }

    pub fn load(&self, order: Ordering) -> f32 {
        f32::from_bits(self.bits.load(order))
    }

    pub fn store(&self, val: f32, order: Ordering) {
        self.bits.store(val.to_bits(), order);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VadState {
    Silent = 0,
    Speech = 1,
    Holding = 2,
    Trailing = 3,
}

impl From<u8> for VadState {
    fn from(val: u8) -> Self {
        match val {
            0 => VadState::Silent,
            1 => VadState::Speech,
            2 => VadState::Holding,
            3 => VadState::Trailing,
            _ => VadState::Silent,
        }
    }
}

/// Mic vs system-loopback need different silence/energy profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadProfile {
    Microphone,
    SystemLoopback,
}

pub struct VADEngine {
    noise_floor: AtomicF32,
    state: AtomicU8,
    silence_duration_ms: AtomicU32,
    speech_duration_ms: AtomicU32,
    speech_onset_frames: AtomicU32,
    transient_rejected_count: AtomicU32,
    speech_frames_count: AtomicU32,
    profile: AtomicU8, // 0 = mic, 1 = system
}

impl VADEngine {
    pub fn new() -> Self {
        Self {
            noise_floor: AtomicF32::new(0.005),
            state: AtomicU8::new(VadState::Silent as u8),
            silence_duration_ms: AtomicU32::new(0),
            speech_duration_ms: AtomicU32::new(0),
            speech_onset_frames: AtomicU32::new(0),
            transient_rejected_count: AtomicU32::new(0),
            speech_frames_count: AtomicU32::new(0),
            profile: AtomicU8::new(0),
        }
    }

    pub fn reset(&self, profile: VadProfile) {
        let (floor, profile_id) = match profile {
            VadProfile::Microphone => (0.004f32, 0u8),
            // System/Meet often has music beds & higher ambient — higher floor, don't latch forever
            VadProfile::SystemLoopback => (0.012f32, 1u8),
        };
        self.noise_floor.store(floor, Ordering::Relaxed);
        self.state.store(VadState::Silent as u8, Ordering::Relaxed);
        self.silence_duration_ms.store(0, Ordering::Relaxed);
        self.speech_duration_ms.store(0, Ordering::Relaxed);
        self.speech_onset_frames.store(0, Ordering::Relaxed);
        self.profile.store(profile_id, Ordering::Relaxed);
    }

    pub fn get_state(&self) -> VadState {
        VadState::from(self.state.load(Ordering::Relaxed))
    }

    pub fn set_state(&self, new_state: VadState) {
        self.state.store(new_state as u8, Ordering::Relaxed);
    }

    pub fn get_noise_floor(&self) -> f32 {
        self.noise_floor.load(Ordering::Relaxed)
    }

    pub fn get_transient_rejected_count(&self) -> u32 {
        self.transient_rejected_count.load(Ordering::Relaxed)
    }

    pub fn get_speech_frames_count(&self) -> u32 {
        self.speech_frames_count.load(Ordering::Relaxed)
    }

    pub fn reset_metrics(&self) {
        self.transient_rejected_count.store(0, Ordering::Relaxed);
        self.speech_frames_count.store(0, Ordering::Relaxed);
    }

    fn is_system(&self) -> bool {
        self.profile.load(Ordering::Relaxed) == 1
    }

    /// Process a 30ms frame of mono samples at 16kHz (480 samples).
    /// Returns true if a boundary (end of utterance) is detected.
    pub fn process_frame(&self, frame: &[f32], recent_speech_samples: &[f32]) -> (VadState, bool) {
        if frame.is_empty() {
            return (self.get_state(), false);
        }

        let mut sum_sq = 0.0;
        for &sample in frame {
            sum_sq += sample * sample;
        }
        let rms = (sum_sq / frame.len() as f32).sqrt();

        let current_noise_floor = self.noise_floor.load(Ordering::Relaxed);
        let system = self.is_system();

        // System: slightly higher threshold multiplier for system audio to reject music background.
        let speech_mult = if system { 3.6 } else { 3.0 };
        let speech_threshold = current_noise_floor * speech_mult;
        let is_frame_speech = rms > speech_threshold;

        let current_state = self.get_state();
        let mut next_state = current_state;
        let mut trigger_boundary = false;

        // Safety cap only — prefer natural silence endpointing.
        let max_speech_ms: u32 = if system { 22_000 } else { 25_000 };

        match current_state {
            VadState::Silent => {
                if is_frame_speech {
                    // Speech Onset Verification: require 5 consecutive 30ms frames (~150ms)
                    // before transitioning to confirmed Speech to ignore short clicks/dings (<220ms).
                    let onset = self.speech_onset_frames.fetch_add(1, Ordering::Relaxed) + 1;
                    if onset >= 5 {
                        next_state = VadState::Speech;
                        self.silence_duration_ms.store(0, Ordering::Relaxed);
                        self.speech_duration_ms.store(onset * 30, Ordering::Relaxed);
                        self.speech_onset_frames.store(0, Ordering::Relaxed);
                        self.speech_frames_count.fetch_add(onset, Ordering::Relaxed);
                    }
                } else {
                    let onset = self.speech_onset_frames.load(Ordering::Relaxed);
                    if onset > 0 {
                        // Transient noise burst (<150ms) rejected!
                        self.speech_onset_frames.store(0, Ordering::Relaxed);
                        self.transient_rejected_count.fetch_add(1, Ordering::Relaxed);
                    }
                    let alpha = if system { 0.06 } else { 0.05 };
                    let updated = (1.0 - alpha) * current_noise_floor + alpha * rms;
                    let min_floor = if system { 0.0015 } else { 0.0001 };
                    self.noise_floor
                        .store(updated.max(min_floor), Ordering::Relaxed);
                }
            }
            VadState::Speech => {
                let speech_ms = self.speech_duration_ms.fetch_add(30, Ordering::Relaxed) + 30;
                self.speech_frames_count.fetch_add(1, Ordering::Relaxed);

                if is_frame_speech {
                    self.silence_duration_ms.store(0, Ordering::Relaxed);
                    if system {
                        let alpha = 0.008;
                        let updated = (1.0 - alpha) * current_noise_floor + alpha * (rms * 0.25);
                        self.noise_floor
                            .store(updated.max(0.0015), Ordering::Relaxed);
                    }
                    if speech_ms >= max_speech_ms {
                        next_state = VadState::Silent;
                        trigger_boundary = true;
                        self.silence_duration_ms.store(0, Ordering::Relaxed);
                        self.speech_duration_ms.store(0, Ordering::Relaxed);
                    }
                } else {
                    next_state = VadState::Holding;
                    self.silence_duration_ms.store(30, Ordering::Relaxed);
                }
            }
            VadState::Holding => {
                if is_frame_speech {
                    // Same utterance continues — return to Speech state
                    next_state = VadState::Speech;
                    self.silence_duration_ms.store(0, Ordering::Relaxed);
                    self.speech_frames_count.fetch_add(1, Ordering::Relaxed);
                } else {
                    let prev_silence = self.silence_duration_ms.load(Ordering::Relaxed);
                    let new_silence = prev_silence + 30;
                    self.silence_duration_ms.store(new_silence, Ordering::Relaxed);

                    // Adaptive speech-continuation confidence estimation
                    let continuation_confidence = estimate_speech_continuation_confidence(recent_speech_samples, 16000);
                    
                    // Base silence timeout is 750ms for normal speech (fast response!).
                    // Dynamically extended up to 1500ms only when continuation confidence is high.
                    let base_timeout_ms: u32 = if system { 750 } else { 700 };
                    let extra_timeout_ms: u32 = (continuation_confidence * 750.0) as u32;
                    let timeout_ms = base_timeout_ms + extra_timeout_ms;

                    if new_silence >= timeout_ms {
                        next_state = VadState::Silent;
                        trigger_boundary = true;
                        self.silence_duration_ms.store(0, Ordering::Relaxed);
                        self.speech_duration_ms.store(0, Ordering::Relaxed);
                    }
                }
            }
            VadState::Trailing => {
                next_state = VadState::Silent;
            }
        }

        if next_state != current_state {
            self.set_state(next_state);
        }

        (next_state, trigger_boundary)
    }
}

/// Calculate multi-factor speech-continuation confidence (0.0 to 1.0).
/// Evaluates tail energy trajectory, pitch presence/trend, and speech frame density.
pub fn estimate_speech_continuation_confidence(samples: &[f32], sample_rate: u32) -> f32 {
    let n = samples.len();
    if n < 1600 {
        return 0.0;
    }

    let mut confidence = 0.0f32;

    // 1. Tail Energy Trajectory: check if last 100ms has non-zero speech energy (speaker paused vs ended sentence)
    let tail_len = 1600.min(n);
    let tail = &samples[n - tail_len..];
    let tail_rms = frame_rms(tail);
    let overall_rms = frame_rms(samples);

    if overall_rms > 1e-5 {
        let energy_ratio = (tail_rms / overall_rms).clamp(0.0, 2.0);
        if energy_ratio > 0.4 {
            confidence += 0.35;
        } else if energy_ratio > 0.2 {
            confidence += 0.15;
        }
    }

    // 2. Pitch / Formant structure check
    if let Some(_pitch) = estimate_pitch(tail, sample_rate) {
        confidence += 0.35;
    }

    // 3. Pitch rising check (interrogative intonation)
    if is_pitch_rising(samples, sample_rate) {
        confidence += 0.30;
    }

    confidence.clamp(0.0, 1.0)
}

fn frame_rms(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0f32;
    for &s in frame {
        sum += s * s;
    }
    (sum / frame.len() as f32).sqrt()
}

pub fn estimate_pitch(samples: &[f32], sample_rate: u32) -> Option<f32> {
    let n = samples.len();
    if n < 320 {
        return None;
    }

    let min_freq = 50.0;
    let max_freq = 400.0;
    let max_lag = (sample_rate as f32 / min_freq) as usize;
    let min_lag = (sample_rate as f32 / max_freq) as usize;

    let mut best_lag = 0;
    let mut best_corr = -1.0;

    for lag in min_lag..=max_lag {
        let mut corr = 0.0;
        let mut energy_x = 0.0;
        let mut energy_y = 0.0;

        let limit = n - lag;
        for i in 0..limit {
            let x = samples[i];
            let y = samples[i + lag];
            corr += x * y;
            energy_x += x * x;
            energy_y += y * y;
        }

        let norm = (energy_x * energy_y).sqrt();
        if norm > 1e-6 {
            let r = corr / norm;
            if r > best_corr {
                best_corr = r;
                best_lag = lag;
            }
        }
    }

    if best_corr > 0.35 && best_lag > 0 {
        Some(sample_rate as f32 / best_lag as f32)
    } else {
        None
    }
}

pub fn is_pitch_rising(samples: &[f32], sample_rate: u32) -> bool {
    let n = samples.len();
    if n < 3200 {
        return false;
    }

    let chunk_size = 800;
    let mut pitches = Vec::new();
    for i in 0..4 {
        let start = i * chunk_size;
        let end = start + chunk_size;
        if let Some(pitch) = estimate_pitch(&samples[start..end], sample_rate) {
            pitches.push(pitch);
        }
    }

    if pitches.len() >= 2 {
        let first = pitches[0];
        let last = pitches[pitches.len() - 1];
        last > first * 1.05
    } else {
        false
    }
}

