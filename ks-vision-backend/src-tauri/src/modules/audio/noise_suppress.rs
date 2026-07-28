use nnnoiseless::DenoiseState;

pub struct NoiseSuppressor {
    denoiser: Box<DenoiseState<'static>>,
    hpf_prev_x: f32,
    hpf_prev_y: f32,
    agc_energy: f32,
}

impl NoiseSuppressor {
    pub fn new() -> Self {
        Self {
            denoiser: DenoiseState::new(),
            hpf_prev_x: 0.0,
            hpf_prev_y: 0.0,
            agc_energy: 0.01,
        }
    }

    /// Process mono samples in-place.
    /// Supports cleaning high frequency clicks, AC/fan rumble, and normalizing volume.
    pub fn process(&mut self, samples: &mut [f32], sample_rate: u32) {
        if samples.is_empty() {
            return;
        }

        // 1. High-Pass Filter (80Hz cutoff)
        let dt = 1.0 / sample_rate as f32;
        let rc = 1.0 / (2.0 * std::f32::consts::PI * 80.0);
        let alpha = rc / (rc + dt);

        for sample in samples.iter_mut() {
            let x = *sample;
            let y = alpha * (self.hpf_prev_y + x - self.hpf_prev_x);
            self.hpf_prev_x = x;
            self.hpf_prev_y = y;
            *sample = y;
        }

        // 2. RNNNoise Suppression
        if sample_rate == 16000 {
            let samples_48k = resample_16k_to_48k(samples);
            let mut denoised_48k = vec![0.0; samples_48k.len()];

            // Process in 480-sample frames (10ms at 48kHz)
            for (chunk_in, chunk_out) in samples_48k.chunks_exact(480).zip(denoised_48k.chunks_exact_mut(480)) {
                // process_frame expects slice arguments
                self.denoiser.process_frame(chunk_out, chunk_in);
            }

            let resampled_16k = resample_48k_to_16k(&denoised_48k);
            let len = samples.len().min(resampled_16k.len());
            samples[..len].copy_from_slice(&resampled_16k[..len]);
        }

        // 3. Automatic Gain Control (AGC) & Volume Normalization
        let mut sum_sq = 0.0;
        for &s in samples.iter() {
            sum_sq += s * s;
        }
        let rms = (sum_sq / samples.len() as f32).sqrt().max(0.0001);

        // Track envelope energy with smoothing
        let beta = 0.05;
        self.agc_energy = (1.0 - beta) * self.agc_energy + beta * rms;

        // Target amplitude is 0.1 (~ -20dBFS)
        let target_rms = 0.1;
        let current_energy = self.agc_energy.max(0.0001);
        let mut gain = target_rms / current_energy;

        // Cap amplification to avoid blowing up static noise
        if gain > 5.0 {
            gain = 5.0;
        } else if gain < 0.2 {
            gain = 0.2;
        }

        for s in samples.iter_mut() {
            *s = (*s * gain).clamp(-1.0, 1.0);
        }
    }
}

fn resample_16k_to_48k(input: &[f32]) -> Vec<f32> {
    let mut output = Vec::with_capacity(input.len() * 3);
    if input.is_empty() {
        return output;
    }
    for i in 0..input.len() {
        let curr = input[i];
        let next = if i + 1 < input.len() { input[i + 1] } else { curr };
        output.push(curr);
        output.push(curr + (next - curr) * 0.3333);
        output.push(curr + (next - curr) * 0.6667);
    }
    output
}

fn resample_48k_to_16k(input: &[f32]) -> Vec<f32> {
    let mut output = Vec::with_capacity(input.len() / 3);
    for chunk in input.chunks_exact(3) {
        let avg = (chunk[0] + chunk[1] + chunk[2]) / 3.0;
        output.push(avg);
    }
    output
}
