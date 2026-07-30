//! Light post-endpoint trim helpers for Gemini audio clips.
//! Keep processing gentle — aggressive gating damages consonants / tech vocabulary.

pub struct AudioQualityMetrics {
    pub avg_rms_db: f32,
    pub peak_db: f32,
    pub speech_ratio_pct: u32,
}

/// Sanitize float samples against NaN/Inf values and clamp to [-1.0, 1.0].
pub fn sanitize_samples(samples: &mut [f32]) {
    for s in samples.iter_mut() {
        if s.is_nan() || s.is_infinite() {
            *s = 0.0;
        } else {
            *s = s.clamp(-1.0, 1.0);
        }
    }
}

/// Calculate Audio Quality metrics (Average RMS in dB, Peak Level in dB, Speech Ratio %).
pub fn calculate_audio_quality_metrics(samples: &[f32], frame_size: usize) -> AudioQualityMetrics {
    if samples.is_empty() || frame_size == 0 {
        return AudioQualityMetrics {
            avg_rms_db: -96.0,
            peak_db: -96.0,
            speech_ratio_pct: 0,
        };
    }

    let mut sum_sq = 0.0f32;
    let mut peak = 0.0f32;
    let mut speech_frames = 0usize;
    let mut total_frames = 0usize;
    let floor = 0.005f32;

    for chunk in samples.chunks(frame_size) {
        total_frames += 1;
        let mut chunk_sq = 0.0f32;
        for &s in chunk {
            chunk_sq += s * s;
            peak = peak.max(s.abs());
        }
        sum_sq += chunk_sq;
        let chunk_rms = (chunk_sq / chunk.len() as f32).sqrt();
        if chunk_rms > floor * 2.0 {
            speech_frames += 1;
        }
    }

    let rms = (sum_sq / samples.len() as f32).sqrt();
    let avg_rms_db = if rms > 1e-5 { 20.0 * rms.log10() } else { -96.0 };
    let peak_db = if peak > 1e-5 { 20.0 * peak.log10() } else { -96.0 };
    let speech_ratio_pct = if total_frames > 0 {
        ((speech_frames as f32 / total_frames as f32) * 100.0).round() as u32
    } else {
        0
    };

    AudioQualityMetrics {
        avg_rms_db,
        peak_db,
        speech_ratio_pct,
    }
}

/// Mild noise attenuator — never square-crush soft speech frames.
pub fn soft_noise_gate(samples: &mut [f32], frame_size: usize) {
    if samples.is_empty() || frame_size == 0 {
        return;
    }

    let mut noise_floor = 0.006f32;
    for frame in samples.chunks_mut(frame_size) {
        let rms = frame_rms(frame);
        // Higher speech threshold margin so soft consonants stay intact.
        let is_speech = rms > noise_floor * 1.8;
        if !is_speech {
            noise_floor = noise_floor * 0.97 + rms * 0.03;
            // Gentle linear attenuation only (was gain² — that destroyed consonants).
            let gain = (rms / (noise_floor * 1.5 + 1e-6)).clamp(0.35, 1.0);
            for s in frame.iter_mut() {
                *s *= gain;
            }
        } else {
            noise_floor = (noise_floor * 0.999).max(0.001);
        }
    }
}

/// First-order high-pass (~80 Hz @ 16 kHz) to remove rumble / AC hum.
pub fn highpass_rumble(samples: &mut [f32]) {
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

#[allow(dead_code)]
pub fn peak_normalize(samples: &mut [f32], target_peak: f32) {
    let mut peak = 0.0f32;
    for &s in samples.iter() {
        peak = peak.max(s.abs());
    }
    if peak < 1e-5 {
        return;
    }
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

/// Trim leading/trailing silence; keep generous pad so first/last words survive.
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

    let mut sorted = energies.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let quiet_n = (sorted.len() / 5).max(1);
    let quiet_avg: f32 = sorted[..quiet_n].iter().sum::<f32>() / quiet_n as f32;
    // Milder threshold — don't treat soft onsets as silence.
    let threshold = (quiet_avg * 2.5).max(0.003);

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
        return Vec::new();
    };

    let start = f.saturating_mul(frame_size).saturating_sub(pad_samples);
    let end = ((l + 1) * frame_size + pad_samples).min(samples.len());
    if start >= end {
        return Vec::new();
    }
    samples[start..end].to_vec()
}

/// Prepare speech for Gemini — preserve intelligibility over aggressive cleanup.
pub fn prepare_for_voice(samples: &[f32]) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    let mut buf = samples.to_vec();
    sanitize_samples(&mut buf);
    highpass_rumble(&mut buf);
    soft_noise_gate(&mut buf, 480);
    // ~500ms pad @ 16k (8000 samples) — keep first/last consonants (NestJS, Kubernetes, …)
    let trimmed = trim_silence(&buf, 480, 8_000);
    let mut result = if trimmed.is_empty() { buf } else { trimmed };
    sanitize_samples(&mut result);
    result
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

