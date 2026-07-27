use std::collections::HashMap;
use std::sync::Mutex;
use crate::modules::audio::fft::compute_spectral_centroid;
use crate::modules::audio::vad_engine::estimate_pitch;
use crate::modules::audio::echo_reference::EchoReferenceBuffer;

pub type SpeakerId = String;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VoiceProfile {
    pub mean_pitch: f32,
    pub spectral_centroid: f32,
    pub zero_crossing_rate: f32,
}

pub struct SpeakerTracker {
    voice_profiles: Mutex<HashMap<SpeakerId, VoiceProfile>>,
    current_speaker: Mutex<Option<SpeakerId>>,
    speaker_counter: Mutex<usize>,
}

impl SpeakerTracker {
    pub fn new() -> Self {
        Self {
            voice_profiles: Mutex::new(HashMap::new()),
            current_speaker: Mutex::new(None),
            speaker_counter: Mutex::new(0),
        }
    }

    pub fn reset(&self) {
        if let Ok(mut profiles) = self.voice_profiles.lock() {
            profiles.clear();
        }
        if let Ok(mut current) = self.current_speaker.lock() {
            *current = None;
        }
        if let Ok(mut counter) = self.speaker_counter.lock() {
            *counter = 0;
        }
    }

    pub fn extract_features(&self, samples: &[f32], sample_rate: u32) -> (f32, f32, f32) {
        if samples.is_empty() {
            return (0.0, 0.0, 0.0);
        }
        
        // 1. Zero-Crossing Rate
        let mut zero_crossings = 0;
        for i in 1..samples.len() {
            if (samples[i] >= 0.0 && samples[i - 1] < 0.0) || (samples[i] < 0.0 && samples[i - 1] >= 0.0) {
                zero_crossings += 1;
            }
        }
        let zcr = zero_crossings as f32 / samples.len() as f32;
        
        // 2. Mean Pitch (F0) & 3. Spectral Centroid
        let chunk_size = 800; // 50ms chunks at 16kHz
        let mut total_pitch = 0.0;
        let mut pitch_count = 0;
        
        let mut total_centroid = 0.0;
        let mut centroid_count = 0;
        
        for chunk in samples.chunks(chunk_size) {
            if chunk.len() >= 320 {
                if let Some(p) = estimate_pitch(chunk, sample_rate) {
                    total_pitch += p;
                    pitch_count += 1;
                }
            }
            if chunk.len() >= 64 {
                let c = compute_spectral_centroid(chunk, sample_rate);
                if c > 0.0 {
                    total_centroid += c;
                    centroid_count += 1;
                }
            }
        }
        
        let mean_pitch = if pitch_count > 0 { total_pitch / pitch_count as f32 } else { 120.0 }; // Default standard pitch
        let mean_centroid = if centroid_count > 0 { total_centroid / centroid_count as f32 } else { 1500.0 };
        
        (mean_pitch, mean_centroid, zcr)
    }

    pub fn identify_speaker(&self, samples: &[f32], sample_rate: u32) -> SpeakerId {
        let (pitch, centroid, zcr) = self.extract_features(samples, sample_rate);
        
        let mut profiles = self.voice_profiles.lock().unwrap();
        
        // Normalized features:
        // pitch ~ 50-400Hz (norm divisor 200)
        // centroid ~ 100-5000Hz (norm divisor 2000)
        // zcr ~ 0.0-1.0 (norm divisor 0.2)
        let norm_pitch = pitch / 200.0;
        let norm_centroid = centroid / 2000.0;
        let norm_zcr = zcr / 0.2;
        
        let mut best_speaker = None;
        let mut best_dist = f32::MAX;
        
        for (id, profile) in profiles.iter() {
            let d_pitch = norm_pitch - (profile.mean_pitch / 200.0);
            let d_centroid = norm_centroid - (profile.spectral_centroid / 2000.0);
            let d_zcr = norm_zcr - (profile.zero_crossing_rate / 0.2);
            let dist = (d_pitch * d_pitch + d_centroid * d_centroid + d_zcr * d_zcr).sqrt();
            
            if dist < best_dist {
                best_dist = dist;
                best_speaker = Some(id.clone());
            }
        }
        
        // Threshold for speaker match: 0.8
        if best_dist < 0.8 && best_speaker.is_some() {
            let id = best_speaker.unwrap();
            *self.current_speaker.lock().unwrap() = Some(id.clone());
            id
        } else {
            let mut counter = self.speaker_counter.lock().unwrap();
            *counter += 1;
            let new_id = format!("Speaker {}", counter);
            
            profiles.insert(new_id.clone(), VoiceProfile {
                mean_pitch: pitch,
                spectral_centroid: centroid,
                zero_crossing_rate: zcr,
            });
            
            *self.current_speaker.lock().unwrap() = Some(new_id.clone());
            new_id
        }
    }

    pub fn is_ai_echo(&self, samples: &[f32], echo_ref: &EchoReferenceBuffer) -> bool {
        let ref_data = {
            let buf = echo_ref.downsampled_buffer.lock().unwrap();
            let mut v = Vec::with_capacity(buf.len());
            for &x in buf.iter() {
                v.push(x);
            }
            v
        };
        
        if ref_data.is_empty() {
            return false;
        }
        
        let captured_downsampled = downsample_16k_to_1k(samples);
        if captured_downsampled.is_empty() {
            return false;
        }
        
        let l_c = captured_downsampled.len();
        let l_r = ref_data.len();
        
        if l_r < l_c {
            return false;
        }
        
        let mut max_corr = 0.0;
        let max_lag = l_r - l_c;
        
        // Limit lag checks to speed up correlation
        let search_step = if max_lag > 3000 { (max_lag / 1000).max(1) } else { 1 };
        
        let mut i = 0;
        while i <= max_lag {
            let mut dot = 0.0;
            let mut e_c = 0.0;
            let mut e_r = 0.0;
            
            for j in 0..l_c {
                let xc = captured_downsampled[j];
                let xr = ref_data[i + j];
                dot += xc * xr;
                e_c += xc * xc;
                e_r += xr * xr;
            }
            
            let norm = (e_c * e_r).sqrt();
            if norm > 1e-6 {
                let r = dot / norm;
                if r > max_corr {
                    max_corr = r;
                }
            }
            
            i += search_step;
        }
        
        max_corr > 0.7
    }
}

fn downsample_16k_to_1k(samples: &[f32]) -> Vec<f32> {
    samples.chunks(16)
        .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
        .collect()
}
