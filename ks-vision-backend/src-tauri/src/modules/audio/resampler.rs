/// Resampler with a cheap low-pass before downsampling (reduces aliasing from 48k→16k loopback).
pub struct LinearResampler;

impl LinearResampler {
    pub fn new() -> Self {
        Self
    }

    pub fn process(&self, samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
        if from_rate == to_rate {
            return samples.to_vec();
        }

        let input = if from_rate > to_rate {
            // Moving-average low-pass ≈ cutoff near Nyquist of target rate
            let window = ((from_rate as f32 / to_rate as f32).round() as usize).clamp(2, 16);
            moving_average(samples, window)
        } else {
            samples.to_vec()
        };

        let ratio = from_rate as f64 / to_rate as f64;
        let num_samples = (input.len() as f64 / ratio).floor() as usize;
        let mut output = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let pos = i as f64 * ratio;
            let idx = pos as usize;
            let frac = (pos - idx as f64) as f32;
            if idx + 1 < input.len() {
                // Hermite-ish cubic blend is overkill; linear on pre-filtered signal is fine
                let val = input[idx] * (1.0 - frac) + input[idx + 1] * frac;
                output.push(val);
            } else if idx < input.len() {
                output.push(input[idx]);
            }
        }
        output
    }
}

fn moving_average(samples: &[f32], window: usize) -> Vec<f32> {
    if window <= 1 || samples.is_empty() {
        return samples.to_vec();
    }
    let mut out = Vec::with_capacity(samples.len());
    let mut acc = 0.0f32;
    for i in 0..samples.len() {
        acc += samples[i];
        if i >= window {
            acc -= samples[i - window];
            out.push(acc / window as f32);
        } else {
            out.push(acc / (i + 1) as f32);
        }
    }
    out
}
