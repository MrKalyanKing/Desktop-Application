/// Resampler with a cheap low-pass before downsampling (reduces aliasing from 48k→16k loopback).
/// Stateless: fine for one-shot buffers. Use `StreamResampler` for continuous capture.
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

/// Continuous resampler for audio callbacks: windowed-sinc anti-alias FIR plus
/// fractional read position carried across chunks, so chunk boundaries are seamless.
pub struct StreamResampler {
    from_rate: u32,
    to_rate: u32,
    step: f64,
    taps: Vec<f32>,
    /// Last `taps.len() - 1` raw input samples (FIR history).
    fir_hist: Vec<f32>,
    /// Filtered samples not yet fully consumed by interpolation.
    filtered: Vec<f32>,
    /// Read position into `filtered`.
    pos: f64,
}

impl StreamResampler {
    pub fn new(from_rate: u32, to_rate: u32) -> Self {
        let step = from_rate as f64 / to_rate.max(1) as f64;
        let taps = if from_rate > to_rate {
            design_lowpass(from_rate, to_rate)
        } else {
            Vec::new()
        };
        let hist_len = taps.len().saturating_sub(1);
        Self {
            from_rate,
            to_rate,
            step,
            taps,
            fir_hist: vec![0.0; hist_len],
            filtered: Vec::with_capacity(4096),
            pos: 0.0,
        }
    }

    pub fn rates(&self) -> (u32, u32) {
        (self.from_rate, self.to_rate)
    }

    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        if self.from_rate == self.to_rate {
            return input.to_vec();
        }
        if input.is_empty() {
            return Vec::new();
        }

        if self.taps.is_empty() {
            self.filtered.extend_from_slice(input);
        } else {
            let n = self.taps.len();
            let hist = n - 1;
            self.fir_hist.extend_from_slice(input);
            for i in 0..input.len() {
                let window = &self.fir_hist[i..i + n];
                let mut acc = 0.0f32;
                for (x, h) in window.iter().zip(self.taps.iter()) {
                    acc += x * h;
                }
                self.filtered.push(acc);
            }
            let drop = self.fir_hist.len() - hist;
            self.fir_hist.drain(..drop);
        }

        let mut out = Vec::with_capacity((input.len() as f64 / self.step) as usize + 2);
        while self.pos + 1.0 < self.filtered.len() as f64 {
            let idx = self.pos as usize;
            let frac = (self.pos - idx as f64) as f32;
            out.push(self.filtered[idx] * (1.0 - frac) + self.filtered[idx + 1] * frac);
            self.pos += self.step;
        }

        let consumed = (self.pos as usize).min(self.filtered.len());
        self.filtered.drain(..consumed);
        self.pos -= consumed as f64;
        out
    }
}

/// Hamming-windowed sinc low-pass with cutoff just under the target Nyquist.
fn design_lowpass(from_rate: u32, to_rate: u32) -> Vec<f32> {
    let ratio = from_rate as f64 / to_rate as f64;
    let mut n = ((16.0 * ratio).ceil() as usize).clamp(15, 97);
    if n % 2 == 0 {
        n += 1;
    }
    let fc = 0.44 * to_rate as f64 / from_rate as f64; // cycles per input sample
    let mid = (n - 1) as f64 / 2.0;
    let mut taps: Vec<f64> = (0..n)
        .map(|i| {
            let x = i as f64 - mid;
            let sinc = if x == 0.0 {
                2.0 * fc
            } else {
                (2.0 * std::f64::consts::PI * fc * x).sin() / (std::f64::consts::PI * x)
            };
            let w = 0.54 - 0.46 * (2.0 * std::f64::consts::PI * i as f64 / (n - 1) as f64).cos();
            sinc * w
        })
        .collect();
    let sum: f64 = taps.iter().sum();
    for t in taps.iter_mut() {
        *t /= sum;
    }
    taps.into_iter().map(|t| t as f32).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, rate: u32, secs: f32) -> Vec<f32> {
        let n = (rate as f32 * secs) as usize;
        (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin() * 0.5)
            .collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|s| s * s).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    #[test]
    fn passes_speech_band() {
        for freq in [200.0, 1000.0, 3000.0, 5000.0] {
            let mut r = StreamResampler::new(48_000, 16_000);
            let out = r.process(&sine(freq, 48_000, 0.5));
            let level = rms(&out[200..]) / (0.5 / 2f32.sqrt());
            assert!(level > 0.85, "{freq} Hz attenuated to {level}");
        }
    }

    #[test]
    fn rejects_aliasing_content() {
        for freq in [10_000.0, 12_000.0, 18_000.0] {
            let mut r = StreamResampler::new(48_000, 16_000);
            let out = r.process(&sine(freq, 48_000, 0.5));
            let level = rms(&out[200..]) / (0.5 / 2f32.sqrt());
            assert!(level < 0.03, "{freq} Hz leaked at {level}");
        }
    }

    #[test]
    fn chunked_matches_one_shot() {
        for from in [44_100u32, 48_000, 96_000] {
            let input = sine(440.0, from, 1.0);
            let mut whole = StreamResampler::new(from, 16_000);
            let a = whole.process(&input);
            let mut chunked = StreamResampler::new(from, 16_000);
            let mut b = Vec::new();
            for c in input.chunks(441) {
                b.extend(chunked.process(c));
            }
            assert!((a.len() as i64 - b.len() as i64).abs() <= 1);
            let max_diff = a
                .iter()
                .zip(b.iter())
                .map(|(x, y)| (x - y).abs())
                .fold(0.0f32, f32::max);
            assert!(max_diff < 1e-4, "{from}: chunk seam error {max_diff}");
            let expect = input.len() as f64 * 16_000.0 / from as f64;
            assert!((b.len() as f64 - expect).abs() < 3.0, "{from}: drift {} vs {expect}", b.len());
        }
    }
}
