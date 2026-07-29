//! Continuous RNNoise (nnnoiseless) at 48 kHz frames, bridged from/to 16 kHz pipeline.

use nnnoiseless::DenoiseState;
use crate::modules::audio::resampler::LinearResampler;

pub struct RnnoiseProcessor {
    state: Box<DenoiseState<'static>>,
    resampler: LinearResampler,
    /// Leftover 48 kHz samples waiting for a full FRAME_SIZE.
    pending_48k: Vec<f32>,
    first_frame: bool,
    out_scratch: Vec<f32>,
}

impl RnnoiseProcessor {
    pub fn new() -> Self {
        Self {
            state: DenoiseState::new(),
            resampler: LinearResampler::new(),
            pending_48k: Vec::with_capacity(DenoiseState::FRAME_SIZE * 2),
            first_frame: true,
            out_scratch: vec![0.0; DenoiseState::FRAME_SIZE],
        }
    }

    /// Denoise a chunk of 16 kHz mono float [-1,1]. Returns denoised 16 kHz audio.
    pub fn process_16k(&mut self, samples_16k: &[f32]) -> Vec<f32> {
        if samples_16k.is_empty() {
            return Vec::new();
        }

        // Upsample 16k → 48k
        let up = self.resampler.process(samples_16k, 16000, 48000);
        // Scale to RNNoise's expected i16-ish range
        self.pending_48k
            .extend(up.iter().map(|s| (s * 32768.0).clamp(-32768.0, 32767.0)));

        let mut denoised_48k = Vec::with_capacity(self.pending_48k.len());
        while self.pending_48k.len() >= DenoiseState::FRAME_SIZE {
            let frame: Vec<f32> = self.pending_48k.drain(..DenoiseState::FRAME_SIZE).collect();
            self.state
                .process_frame(&mut self.out_scratch, &frame);
            if self.first_frame {
                // Discard fade-in artifact frame
                self.first_frame = false;
            } else {
                denoised_48k.extend_from_slice(&self.out_scratch);
            }
        }

        // Scale back to [-1,1] and downsample 48k → 16k
        for s in denoised_48k.iter_mut() {
            *s /= 32768.0;
        }
        self.resampler.process(&denoised_48k, 48000, 16000)
    }
}

impl Default for RnnoiseProcessor {
    fn default() -> Self {
        Self::new()
    }
}
