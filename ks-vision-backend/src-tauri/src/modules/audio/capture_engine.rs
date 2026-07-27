use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crate::modules::audio::segmented_ring_buffer::SegmentedRingBuffer;
use crate::modules::audio::echo_reference::EchoReferenceBuffer;
use crate::modules::audio::resampler::LinearResampler;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSource {
    Microphone,
    SystemLoopback,
}

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("Session already in progress for this source")]
    SessionInProgress,
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    #[error("Failed to build CPAL stream: {0}")]
    StreamBuildError(String),
    #[error("Failed to play CPAL stream: {0}")]
    StreamPlayError(String),
    #[error("Failed to get device config: {0}")]
    ConfigError(String),
}

pub struct SendStream(pub cpal::Stream);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

pub struct CaptureEngine {
    pub mic_stream: RwLock<Option<SendStream>>,
    pub system_stream: RwLock<Option<SendStream>>,
    pub mic_buffer: Arc<Mutex<SegmentedRingBuffer>>,
    pub system_buffer: Arc<Mutex<SegmentedRingBuffer>>,
    pub ai_output_monitor: Arc<EchoReferenceBuffer>,
    
    // Stop signal senders for background tasks
    pub mic_stop_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    pub system_stop_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
}

impl CaptureEngine {
    pub fn new() -> Self {
        // 30 seconds of rolling buffer at 16kHz mono = 30 * 16000 = 480,000 samples
        let capacity = 30 * 16000;
        Self {
            mic_stream: RwLock::new(None),
            system_stream: RwLock::new(None),
            mic_buffer: Arc::new(Mutex::new(SegmentedRingBuffer::new(capacity))),
            system_buffer: Arc::new(Mutex::new(SegmentedRingBuffer::new(capacity))),
            ai_output_monitor: Arc::new(EchoReferenceBuffer::new()),
            mic_stop_tx: Mutex::new(None),
            system_stop_tx: Mutex::new(None),
        }
    }

    pub fn is_recording(&self, source: AudioSource) -> bool {
        match source {
            AudioSource::Microphone => self.mic_stream.read().unwrap().is_some(),
            AudioSource::SystemLoopback => self.system_stream.read().unwrap().is_some(),
        }
    }

    pub fn start_capture<F>(&self, source: AudioSource, mut frame_handler: F) -> Result<Option<SendStream>, CaptureError>
    where
        F: FnMut(&[f32], AudioSource) + Send + 'static,
    {
        // 1. Check if session already in progress
        {
            let stream_guard = match source {
                AudioSource::Microphone => self.mic_stream.read().unwrap(),
                AudioSource::SystemLoopback => self.system_stream.read().unwrap(),
            };
            if stream_guard.is_some() {
                return Err(CaptureError::SessionInProgress);
            }
        }

        // 2. Select host and device
        let host = cpal::default_host();
        let device = match source {
            AudioSource::Microphone => host.default_input_device()
                .ok_or_else(|| CaptureError::DeviceNotFound("Default microphone not found".to_string()))?,
            AudioSource::SystemLoopback => {
                #[cfg(target_os = "windows")]
                {
                    host.default_output_device()
                        .ok_or_else(|| CaptureError::DeviceNotFound("Default output device not found for WASAPI loopback".to_string()))?
                }
                #[cfg(not(target_os = "windows"))]
                {
                    return Err(CaptureError::DeviceNotFound("System audio capture is only supported on Windows".to_string()));
                }
            }
        };

        // 3. Get configuration
        let config = device.default_input_config()
            .map_err(|e| CaptureError::ConfigError(e.to_string()))?;
        
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        let sample_format = config.sample_format();

        // 4. Set up buffer and resampler
        let buffer = match source {
            AudioSource::Microphone => Arc::clone(&self.mic_buffer),
            AudioSource::SystemLoopback => Arc::clone(&self.system_buffer),
        };
        buffer.lock().unwrap().clear();

        let resampler = LinearResampler::new();
        let err_fn = |err| eprintln!("An error occurred on cpal stream: {}", err);
        
        // Define callback logic
        let mut on_data = move |raw_data: &[f32]| {
            // Downmix to mono
            let mono_samples = if channels > 1 {
                let mut mono = Vec::with_capacity(raw_data.len() / channels as usize);
                for chunk in raw_data.chunks_exact(channels as usize) {
                    let sum: f32 = chunk.iter().sum();
                    mono.push(sum / channels as f32);
                }
                mono
            } else {
                raw_data.to_vec()
            };

            // Resample to 16kHz
            let resampled = resampler.process(&mono_samples, sample_rate, 16000);
            
            // Push to rolling buffer
            buffer.lock().unwrap().push_slice(&resampled);
            
            // Call external frame handler
            frame_handler(&resampled, source);
        };

        // 5. Build cpal stream based on sample format
        let stream = match sample_format {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    on_data(data);
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let f32_data: Vec<f32> = data.iter().map(|&x| x as f32 / 32768.0).collect();
                    on_data(&f32_data);
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    let f32_data: Vec<f32> = data.iter().map(|&x| (x as f32 - 32768.0) / 32768.0).collect();
                    on_data(&f32_data);
                },
                err_fn,
                None,
            ),
            _ => return Err(CaptureError::StreamBuildError("Unsupported sample format".to_string())),
        }.map_err(|e| CaptureError::StreamBuildError(e.to_string()))?;

        // 6. Play stream
        stream.play().map_err(|e| CaptureError::StreamPlayError(e.to_string()))?;

        // 7. Store stream under lock and return any old stream
        let old_stream = match source {
            AudioSource::Microphone => {
                let mut mic_guard = self.mic_stream.write().unwrap();
                std::mem::replace(&mut *mic_guard, Some(SendStream(stream)))
            }
            AudioSource::SystemLoopback => {
                let mut system_guard = self.system_stream.write().unwrap();
                std::mem::replace(&mut *system_guard, Some(SendStream(stream)))
            }
        };

        Ok(old_stream)
    }

    /// Stops audio capture for a given source, returning the accumulated audio samples after a 200ms grace period.
    pub async fn stop_capture(&self, source: AudioSource) -> (Vec<f32>, Option<SendStream>) {
        // Graceful drain: wait 200ms to capture trailing speech before pausing stream
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Pause/remove stream
        let stream_opt = match source {
            AudioSource::Microphone => {
                let mut mic_guard = self.mic_stream.write().unwrap();
                mic_guard.take()
            }
            AudioSource::SystemLoopback => {
                let mut system_guard = self.system_stream.write().unwrap();
                system_guard.take()
            }
        };

        // Return the captured samples
        let buffer = match source {
            AudioSource::Microphone => &self.mic_buffer,
            AudioSource::SystemLoopback => &self.system_buffer,
        };
        let samples = buffer.lock().unwrap().get_samples();
        
        (samples, stream_opt)
    }
}
