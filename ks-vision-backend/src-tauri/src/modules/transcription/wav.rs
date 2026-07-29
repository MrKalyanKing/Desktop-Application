//! Minimal WAV encoder for Gemini multimodal audio parts.

pub fn write_wav_to_bytes(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let mut spec = Vec::new();

    spec.extend_from_slice(b"RIFF");
    let num_samples = samples.len();
    let data_size = num_samples * 2;
    let file_size = 36 + data_size;
    spec.extend_from_slice(&(file_size as u32).to_le_bytes());
    spec.extend_from_slice(b"WAVE");

    spec.extend_from_slice(b"fmt ");
    spec.extend_from_slice(&(16u32).to_le_bytes());
    spec.extend_from_slice(&(1u16).to_le_bytes());
    spec.extend_from_slice(&(1u16).to_le_bytes());
    spec.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate: u32 = sample_rate * 1 * 2;
    spec.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align: u16 = 1 * 2;
    spec.extend_from_slice(&block_align.to_le_bytes());
    spec.extend_from_slice(&(16u16).to_le_bytes());

    spec.extend_from_slice(b"data");
    spec.extend_from_slice(&(data_size as u32).to_le_bytes());

    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let scaled = (clamped * 32767.0) as i16;
        spec.extend_from_slice(&scaled.to_le_bytes());
    }

    spec
}
