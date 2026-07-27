use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use crate::modules::audio::resampler::LinearResampler;

pub trait AIAudioOutputSink: Send + Sync {
    /// Feed AI-generated audio into the echo reference buffer.
    /// Samples can be any sample rate; the sink handles resampling.
    fn register_ai_output(&self, samples: &[f32], sample_rate: u32);
}

pub struct EchoReferenceBuffer {
    pub downsampled_buffer: Arc<Mutex<VecDeque<f32>>>,
    resampler: LinearResampler,
}

impl EchoReferenceBuffer {
    pub fn new() -> Self {
        Self {
            downsampled_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(5000))),
            resampler: LinearResampler::new(),
        }
    }
    
    pub fn clear(&self) {
        if let Ok(mut buf) = self.downsampled_buffer.lock() {
            buf.clear();
        }
    }
}

impl AIAudioOutputSink for EchoReferenceBuffer {
    fn register_ai_output(&self, samples: &[f32], sample_rate: u32) {
        let downsampled = self.resampler.process(samples, sample_rate, 1000);
        if let Ok(mut buf) = self.downsampled_buffer.lock() {
            for sample in downsampled {
                buf.push_back(sample);
                if buf.len() > 5000 { // 5 seconds at 1kHz = 5000 samples
                    buf.pop_front();
                }
            }
        }
    }
}
