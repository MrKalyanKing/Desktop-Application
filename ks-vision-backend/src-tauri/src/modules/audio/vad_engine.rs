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

pub struct VADEngine {
    noise_floor: AtomicF32,
    state: AtomicU8,
    silence_duration_ms: AtomicU32,
}

impl VADEngine {
    pub fn new() -> Self {
        Self {
            noise_floor: AtomicF32::new(0.005), // Reasonable initial noise floor
            state: AtomicU8::new(VadState::Silent as u8),
            silence_duration_ms: AtomicU32::new(0),
        }
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

    /// Process a 30ms frame of mono samples at 16kHz (480 samples).
    /// Returns true if a boundary (transition back to SILENT after speech) is detected.
    pub fn process_frame(&self, frame: &[f32], recent_speech_samples: &[f32]) -> (VadState, bool) {
        if frame.is_empty() {
            return (self.get_state(), false);
        }

        // 1. Calculate RMS energy of the frame
        let mut sum_sq = 0.0;
        for &sample in frame {
            sum_sq += sample * sample;
        }
        let rms = (sum_sq / frame.len() as f32).sqrt();

        let current_noise_floor = self.noise_floor.load(Ordering::Relaxed);
        
        // 12dB threshold is ~3.981 in amplitude/RMS ratio
        let speech_threshold = current_noise_floor * 3.981;
        let is_frame_speech = rms > speech_threshold;

        let current_state = self.get_state();
        let mut next_state = current_state;
        let mut trigger_boundary = false;

        match current_state {
            VadState::Silent => {
                if is_frame_speech {
                    next_state = VadState::Speech;
                    self.silence_duration_ms.store(0, Ordering::Relaxed);
                } else {
                    // Update adaptive noise floor during silence
                    let alpha = 0.05;
                    let updated_noise_floor = (1.0 - alpha) * current_noise_floor + alpha * rms;
                    // Prevent noise floor from going below a threshold
                    let min_noise_floor = 0.0001;
                    self.noise_floor.store(updated_noise_floor.max(min_noise_floor), Ordering::Relaxed);
                }
            }
            VadState::Speech => {
                if is_frame_speech {
                    self.silence_duration_ms.store(0, Ordering::Relaxed);
                } else {
                    next_state = VadState::Holding;
                    self.silence_duration_ms.store(30, Ordering::Relaxed); // Started silence (30ms frame)
                }
            }
            VadState::Holding => {
                if is_frame_speech {
                    next_state = VadState::Speech;
                    self.silence_duration_ms.store(0, Ordering::Relaxed);
                } else {
                    let prev_silence = self.silence_duration_ms.load(Ordering::Relaxed);
                    let new_silence = prev_silence + 30;
                    self.silence_duration_ms.store(new_silence, Ordering::Relaxed);

                    // Check F0 Pitch contour of the last 200ms of active speech to determine the timeout
                    let is_question_incomplete = is_pitch_rising(recent_speech_samples, 16000);
                    let timeout_ms = if is_question_incomplete {
                        2500 // Incomplete question: extend timeout to 2.5s
                    } else {
                        1500 // Statement completion or falling pitch: standard 1.5s
                    };

                    if new_silence >= timeout_ms {
                        next_state = VadState::Silent;
                        trigger_boundary = true;
                        self.silence_duration_ms.store(0, Ordering::Relaxed);
                    }
                }
            }
            VadState::Trailing => {
                // Unused in standard state transitions, fallback to Silent
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

pub fn is_pitch_rising(samples: &[f32], sample_rate: u32) -> bool {
    let n = samples.len();
    if n < 3200 {
        return false;
    }
    
    // Split into 4 chunks of 800 samples (50ms each)
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
        last > first * 1.05 // Pitch rose by > 5%
    } else {
        false
    }
}
