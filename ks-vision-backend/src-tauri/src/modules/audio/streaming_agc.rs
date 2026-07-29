//! Continuous streaming AGC — applied before VAD (primary gain control).

/// Soft automatic gain control for 16 kHz mono float PCM in [-1, 1].
pub struct StreamingAgc {
    target_rms: f32,
    max_gain: f32,
    min_gain: f32,
    attack: f32,
    release: f32,
    current_gain: f32,
    noise_floor: f32,
}

impl StreamingAgc {
    pub fn new() -> Self {
        Self {
            target_rms: 0.12,
            max_gain: 12.0,
            min_gain: 0.25,
            attack: 0.35,
            release: 0.08,
            current_gain: 1.0,
            noise_floor: 0.004,
        }
    }

    /// Gentler AGC for meeting/system audio — preserve remote speaker dynamics.
    pub fn for_meeting() -> Self {
        Self {
            target_rms: 0.10,
            max_gain: 4.0,
            min_gain: 0.5,
            attack: 0.25,
            release: 0.06,
            current_gain: 1.0,
            noise_floor: 0.005,
        }
    }

    /// Process samples in-place. Returns frame RMS after gain.
    pub fn process(&mut self, samples: &mut [f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let mut sum_sq = 0.0f32;
        for &s in samples.iter() {
            sum_sq += s * s;
        }
        let rms = (sum_sq / samples.len() as f32).sqrt();

        // Adapt noise floor on quiet frames
        if rms < self.noise_floor * 2.0 {
            self.noise_floor = self.noise_floor * 0.95 + rms * 0.05;
        }

        // Don't boost silence / pure noise
        let desired = if rms < self.noise_floor * 1.5 {
            1.0
        } else {
            (self.target_rms / rms.max(1e-6)).clamp(self.min_gain, self.max_gain)
        };

        let coeff = if desired < self.current_gain {
            self.attack
        } else {
            self.release
        };
        self.current_gain += (desired - self.current_gain) * coeff;

        for s in samples.iter_mut() {
            *s = (*s * self.current_gain).clamp(-0.98, 0.98);
        }

        let mut out_sq = 0.0f32;
        for &s in samples.iter() {
            out_sq += s * s;
        }
        (out_sq / samples.len() as f32).sqrt()
    }
}

impl Default for StreamingAgc {
    fn default() -> Self {
        Self::new()
    }
}
