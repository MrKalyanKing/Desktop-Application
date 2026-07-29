//! Light post-endpoint trim helpers for Gemini audio clips.
//! Primary gain/denoise happen in streaming AGC + RNNoise BEFORE VAD.

/// Soft noise gate: attenuate frames below adaptive floor (keeps speech, reduces hiss/music bed).
pub fn soft_noise_gate(samples: &mut [f32], frame_size: usize) {
    if samples.is_empty() || frame_size == 0 {
        return;
    }

    let mut noise_floor = 0.008f32;
    for frame in samples.chunks_mut(frame_size) {
        let rms = frame_rms(frame);
        let is_speech = rms > noise_floor * 2.5;
        if !is_speech {
            noise_floor = noise_floor * 0.95 + rms * 0.05;
            let gain = (rms / (noise_floor * 2.0 + 1e-6)).clamp(0.05, 1.0);
            for s in frame.iter_mut() {
                *s *= gain * gain;
            }
        } else {
            // Slow decay of floor during speech so quiet talkers aren't gated later
            noise_floor = (noise_floor * 0.999).max(0.001);
        }
    }
}

/// First-order high-pass (~80 Hz @ 16 kHz) to remove rumble / AC hum.
pub fn highpass_rumble(samples: &mut [f32]) {
    // y[n] = α * (y[n-1] + x[n] - x[n-1]), α ≈ 0.97 for ~80Hz @ 16k
    let alpha = 0.97f32;
    let mut prev_x = 0.0f32;
    let mut prev_y = 0.0f32;
    for x in samples.iter_mut() {
        let y = alpha * (prev_y + *x - prev_x);
        prev_x = *x;
        prev_y = y;
        *x = y;
    }
}

/// Peak-normalize helper (legacy). Primary gain is StreamingAgc before VAD.
#[allow(dead_code)]
pub fn peak_normalize(samples: &mut [f32], target_peak: f32) {
    let mut peak = 0.0f32;
    for &s in samples.iter() {
        peak = peak.max(s.abs());
    }
    if peak < 1e-5 {
        return;
    }
    // Only boost quiet clips; don't smash already-loud meeting audio
    if peak < target_peak {
        let gain = (target_peak / peak).min(8.0);
        for s in samples.iter_mut() {
            *s = (*s * gain).clamp(-1.0, 1.0);
        }
    } else if peak > 0.99 {
        let gain = target_peak / peak;
        for s in samples.iter_mut() {
            *s *= gain;
        }
    }
}

/// Trim leading/trailing silence using RMS frames; keep `pad_samples` of context.
pub fn trim_silence(samples: &[f32], frame_size: usize, pad_samples: usize) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    let frame_size = frame_size.max(160);
    let mut energies = Vec::new();
    for frame in samples.chunks(frame_size) {
        energies.push(frame_rms(frame));
    }
    if energies.is_empty() {
        return samples.to_vec();
    }

    // Adaptive threshold from quietest 20% of frames
    let mut sorted = energies.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let quiet_n = (sorted.len() / 5).max(1);
    let quiet_avg: f32 = sorted[..quiet_n].iter().sum::<f32>() / quiet_n as f32;
    let threshold = (quiet_avg * 3.5).max(0.004);

    let mut first = None;
    let mut last = None;
    for (i, &e) in energies.iter().enumerate() {
        if e >= threshold {
            if first.is_none() {
                first = Some(i);
            }
            last = Some(i);
        }
    }

    let (Some(f), Some(l)) = (first, last) else {
        // Entire clip quiet — return empty so STT is skipped
        return Vec::new();
    };

    let start = f.saturating_mul(frame_size).saturating_sub(pad_samples);
    let end = ((l + 1) * frame_size + pad_samples).min(samples.len());
    if start >= end {
        return Vec::new();
    }
    samples[start..end].to_vec()
}

/// Light trim before Gemini audio STT. Gain/denoise already applied continuously upstream.
/// Peak normalize is intentionally NOT used as primary AGC anymore.
pub fn prepare_for_stt(samples: &[f32]) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    let mut buf = samples.to_vec();
    highpass_rumble(&mut buf);
    // Soft residual gate only — streaming AGC already set level
    soft_noise_gate(&mut buf, 480);
    trim_silence(&buf, 480, 1600) // ~100ms pad @ 16k — lower latency
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
