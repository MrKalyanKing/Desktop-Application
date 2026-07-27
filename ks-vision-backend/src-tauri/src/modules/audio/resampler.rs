pub struct LinearResampler;

impl LinearResampler {
    pub fn new() -> Self {
        Self
    }

    pub fn process(&self, samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
        if from_rate == to_rate {
            return samples.to_vec();
        }
        let ratio = from_rate as f64 / to_rate as f64;
        let num_samples = (samples.len() as f64 / ratio).floor() as usize;
        let mut output = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let pos = i as f64 * ratio;
            let idx = pos as usize;
            let frac = (pos - idx as f64) as f32;
            if idx + 1 < samples.len() {
                let val = samples[idx] * (1.0 - frac) + samples[idx + 1] * frac;
                output.push(val);
            } else if idx < samples.len() {
                output.push(samples[idx]);
            }
        }
        output
    }
}
