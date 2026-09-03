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
    profile: AtomicU8, // 0 = mic, 1 = system
}

impl VADEngine {
    pub fn new() -> Self {
        Self {
            noise_floor: AtomicF32::new(0.005),
            state: AtomicU8::new(VadState::Silent as u8),
            silence_duration_ms: AtomicU32::new(0),
            speech_duration_ms: AtomicU32::new(0),
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

    fn is_system(&self) -> bool {
        self.profile.load(Ordering::Relaxed) == 1
    }

    /// Process a 30ms frame of mono samples at 16kHz (480 samples).
    /// Returns true if a boundary (end of utterance) is detected.
    pub fn process_frame(&self, frame: &[f32], _recent_speech_samples: &[f32]) -> (VadState, bool) {
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

        // System: slightly higher than mic, but not so high that remote speakers are missed.
        let speech_mult = if system { 3.8 } else { 3.2 };
        let speech_threshold = current_noise_floor * speech_mult;
        let is_frame_speech = rms > speech_threshold;

        let current_state = self.get_state();
        let mut next_state = current_state;
        let mut trigger_boundary = false;

        // Hard max so clips stay small (~12s) for fast Gemini uploads.
        let max_speech_ms: u32 = if system { 12_000 } else { 12_000 };

        match current_state {
            VadState::Silent => {
                if is_frame_speech {
                    next_state = VadState::Speech;
                    self.silence_duration_ms.store(0, Ordering::Relaxed);
                    self.speech_duration_ms.store(30, Ordering::Relaxed);
                } else {
                    let alpha = if system { 0.06 } else { 0.05 };
                    let updated = (1.0 - alpha) * current_noise_floor + alpha * rms;
                    let min_floor = if system { 0.0015 } else { 0.0001 };
                    self.noise_floor
                        .store(updated.max(min_floor), Ordering::Relaxed);
                }
            }
            VadState::Speech => {
                let speech_ms = self.speech_duration_ms.fetch_add(30, Ordering::Relaxed) + 30;

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
                    // Same utterance continues — do not fire API.
                    next_state = VadState::Speech;
                    self.silence_duration_ms.store(0, Ordering::Relaxed);
                } else {
                    let prev_silence = self.silence_duration_ms.load(Ordering::Relaxed);
                    let new_silence = prev_silence + 30;
                    self.silence_duration_ms.store(new_silence, Ordering::Relaxed);

                    // Short hangover for low speech-end latency (energy VAD only).
                    let timeout_ms: u32 = if system { 400 } else { 280 };

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

    if best_corr > 0.4 && best_lag > 0 {
        Some(sample_rate as f32 / best_lag as f32)
    } else {
        None
    }
}
