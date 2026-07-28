use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use crate::modules::audio::fft::{Complex, fft, apply_hann_window};

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
            noise_floor: AtomicF32::new(0.005),
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
        let mut max_val = 0.0f32;
        for &sample in frame {
            let abs_val = sample.abs();
            if abs_val > max_val {
                max_val = abs_val;
            }
            sum_sq += sample * sample;
        }
        let rms = (sum_sq / frame.len() as f32).sqrt().max(0.0001);

        // 2. Click Filtering (Peak-to-Average Power Ratio transient protection)
        let peak_to_rms = max_val / rms;
        let is_transient_click = peak_to_rms > 6.0;

        // 3. Multi-Band Voice Frequency Analysis
        let voice_band_ratio = compute_voice_band_ratio(frame, 16000);
        let is_voice_range = voice_band_ratio > 0.45;

        // 4. Adaptive Threshold based on ambient noise
        let current_noise_floor = self.noise_floor.load(Ordering::Relaxed);
        let threshold_factor = if current_noise_floor > 0.02 {
            6.0 // High noise environment: require high SNR to prevent false triggers
        } else if current_noise_floor < 0.002 {
            2.5 // Quiet environment: capture soft voices
        } else {
            // Linear scale between 2.5 and 6.0
            2.5 + (current_noise_floor - 0.002) * (3.5 / 0.018)
        };
        let speech_threshold = current_noise_floor * threshold_factor;
        
        // Frame is considered speech if it exceeds the SNR threshold, is in the human voice spectrum,
        // and is not a transient click (mouse/keyboard tap).
        let is_frame_speech = rms > speech_threshold && is_voice_range && !is_transient_click;

        let current_state = self.get_state();
        let mut next_state = current_state;
        let mut trigger_boundary = false;

        match current_state {
            VadState::Silent => {
                if is_frame_speech {
                    next_state = VadState::Speech;
                    self.silence_duration_ms.store(0, Ordering::Relaxed);
                    println!("[VAD] Speech detected! (RMS: {:.5}, Threshold: {:.5}, PAPR: {:.2})", rms, speech_threshold, peak_to_rms);
                } else if !is_transient_click {
                    // Update noise floor during actual silence (don't let short transient clicks bias noise tracking)
                    let alpha = 0.05;
                    let updated_noise_floor = (1.0 - alpha) * current_noise_floor + alpha * rms;
                    let min_noise_floor = 0.0001;
                    self.noise_floor.store(updated_noise_floor.max(min_noise_floor), Ordering::Relaxed);
                }
            }
            VadState::Speech => {
                if is_frame_speech {
                    self.silence_duration_ms.store(0, Ordering::Relaxed);
                } else {
                    next_state = VadState::Holding;
                    self.silence_duration_ms.store(30, Ordering::Relaxed);
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

                    // Check F0 Pitch contour to extend VAD completion timer
                    let is_question_incomplete = is_pitch_rising(recent_speech_samples, 16000);
                    let timeout_ms = if is_question_incomplete {
                        2500 // Incomplete question: extend to 2.5s
                    } else {
                        1500 // Statement: standard 1.5s hangover
                    };

                    if new_silence >= timeout_ms {
                        next_state = VadState::Silent;
                        trigger_boundary = true;
                        self.silence_duration_ms.store(0, Ordering::Relaxed);
                        println!("[VAD] Silence timeout reached. Processing boundary.");
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

fn compute_voice_band_ratio(samples: &[f32], sample_rate: u32) -> f32 {
    let n = samples.len();
    if n == 0 {
        return 0.0;
    }

    let fft_size = 512;
    let mut padded = samples.to_vec();
    padded.resize(fft_size, 0.0);

    let windowed = apply_hann_window(&padded);
    let complex_in: Vec<Complex> = windowed.iter().map(|&x| Complex::new(x, 0.0)).collect();
    let complex_out = fft(&complex_in);

    let bin_resolution = sample_rate as f32 / fft_size as f32;
    let mut voice_energy = 0.0;
    let mut total_energy = 0.0;

    for k in 0..(fft_size / 2) {
        let freq = k as f32 * bin_resolution;
        let mag = complex_out[k].norm();
        let energy = mag * mag;

        if freq >= 300.0 && freq <= 3400.0 {
            voice_energy += energy;
        }
        total_energy += energy;
    }

    if total_energy < 1e-6 {
        return 0.0;
    }
    voice_energy / total_energy
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
