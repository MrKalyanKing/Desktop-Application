use std::f32::consts::PI;

#[derive(Clone, Copy, Debug)]
pub struct Complex {
    pub re: f32,
    pub im: f32,
}

impl Complex {
    pub fn new(re: f32, im: f32) -> Self {
        Self { re, im }
    }

    pub fn add(self, other: Self) -> Self {
        Self::new(self.re + other.re, self.im + other.im)
    }

    pub fn sub(self, other: Self) -> Self {
        Self::new(self.re - other.re, self.im - other.im)
    }

    pub fn mul(self, other: Self) -> Self {
        Self::new(
            self.re * other.re - self.im * other.im,
            self.re * other.im + self.im * other.re,
        )
    }

    pub fn norm(self) -> f32 {
        (self.re * self.re + self.im * self.im).sqrt()
    }
}

pub fn fft(input: &[Complex]) -> Vec<Complex> {
    let n = input.len();
    if n <= 1 {
        return input.to_vec();
    }
    
    if !n.is_power_of_two() {
        let next_power = n.next_power_of_two();
        let mut padded = input.to_vec();
        padded.resize(next_power, Complex::new(0.0, 0.0));
        return fft(&padded);
    }

    let mut even = Vec::with_capacity(n / 2);
    let mut odd = Vec::with_capacity(n / 2);
    for i in 0..n {
        if i % 2 == 0 {
            even.push(input[i]);
        } else {
            odd.push(input[i]);
        }
    }

    let fft_even = fft(&even);
    let fft_odd = fft(&odd);

    let mut result = vec![Complex::new(0.0, 0.0); n];
    for k in 0..(n / 2) {
        let angle = -2.0 * PI * (k as f32) / (n as f32);
        let twiddle = Complex::new(angle.cos(), angle.sin());
        let t = fft_odd[k].mul(twiddle);
        result[k] = fft_even[k].add(t);
        result[k + n / 2] = fft_even[k].sub(t);
    }
    result
}

pub fn apply_hann_window(samples: &[f32]) -> Vec<f32> {
    let n = samples.len();
    if n <= 1 {
        return samples.to_vec();
    }
    let mut windowed = Vec::with_capacity(n);
    for (i, &x) in samples.iter().enumerate() {
        let w = 0.5 * (1.0 - (2.0 * PI * i as f32 / (n - 1) as f32).cos());
        windowed.push(x * w);
    }
    windowed
}

pub fn compute_spectral_centroid(samples: &[f32], sample_rate: u32) -> f32 {
    let mut n = samples.len();
    if n == 0 {
        return 0.0;
    }
    
    if !n.is_power_of_two() {
        let mut p = 1;
        while p * 2 <= n {
            p *= 2;
        }
        n = p;
    }
    
    if n < 64 {
        return 0.0;
    }

    let windowed = apply_hann_window(&samples[0..n]);
    let complex_in: Vec<Complex> = windowed.iter().map(|&x| Complex::new(x, 0.0)).collect();
    let complex_out = fft(&complex_in);

    let mut num = 0.0;
    let mut den = 0.0;

    let half_n = n / 2;
    let bin_resolution = sample_rate as f32 / n as f32;

    for k in 0..half_n {
        let freq = k as f32 * bin_resolution;
        let mag = complex_out[k].norm();
        num += freq * mag;
        den += mag;
    }

    if den < 1e-6 {
        return 0.0;
    }
    num / den
}
