//! Pure speech segmenter: 16 kHz samples in, complete utterances out.
//! Runs on a sample clock (not wall time) so the live worker and the offline
//! listening tests exercise exactly the same logic.

use crate::modules::audio::rnnoise_processor::RnnoiseProcessor;
use crate::modules::audio::vad_engine::{frame_rms, VadProfile, VadState, VADEngine, FRAME_SAMPLES};

pub const SAMPLE_RATE: usize = 16_000;
/// Audio kept before the detected onset so the first word is never clipped.
const LEAD_PAD: usize = 6_400; // 400 ms
/// Retained processed audio; must exceed MAX_SPEECH_MS + pad + hangover.
const CLEAN_CAP: usize = 18 * SAMPLE_RATE;
/// A new utterance starting this soon after the previous one is the same thought
/// (mid-question pause): both are sent together so the model hears the whole question.
pub const CONTINUATION_GAP: usize = 1_400 * SAMPLE_RATE / 1000;
/// Upper bound on what is sent per request (merged continuations included).
pub const MAX_UTTERANCE: usize = 20 * SAMPLE_RATE;
/// Default for what the model receives; chosen from the Gemini listening test.
pub const SEND_DENOISED: bool = true;

#[derive(Debug, Clone)]
pub struct Utterance {
    pub samples: Vec<f32>,
    /// True when this includes a previous utterance it continues.
    pub merged: bool,
    /// Sample clock of the earliest included audio.
    pub start_clock: u64,
    /// Sample clock at the end of the utterance.
    pub end_clock: u64,
}

#[derive(Debug, Clone)]
pub enum SegEvent {
    State(VadState),
    Utterance(Utterance),
    /// A burst that was loud but not speech (click, cough, door) — dropped locally.
    Rejected,
}

/// Rolling signal statistics for the UI health indicator.
#[derive(Debug, Clone, Copy, Default)]
pub struct SignalStats {
    /// Raw input level (before denoise/AGC), RMS.
    pub raw_rms: f32,
    pub raw_peak: f32,
    /// Level after processing, RMS.
    pub level: f32,
    pub noise_floor: f32,
    /// Smoothed level of speech frames; 0 until speech has been heard.
    pub speech_level: f32,
}

pub struct Segmenter {
    rnnoise: RnnoiseProcessor,
    send_denoised: bool,
    vad: VADEngine,
    /// Audio that is sent to the model (denoised or natural, see `with_send_denoised`).
    clean_buf: Vec<f32>,
    /// Clock value of clean_buf[0].
    buf_start: u64,
    /// Processed samples not yet framed for the VAD.
    pending: Vec<f32>,
    clock: u64,
    speech_start: Option<u64>,
    last_state: VadState,
    last_utterance: Option<Utterance>,
    stats: SignalStats,
}

impl Segmenter {
    pub fn new(profile: VadProfile) -> Self {
        Self::with_send_denoised(profile, SEND_DENOISED)
    }

    /// The VAD always listens to denoised audio (robust in noise). `send_denoised`
    /// picks whether the model receives that, or the natural audio (no denoiser
    /// artifacts; strong speech models are trained on noisy speech).
    pub fn with_send_denoised(profile: VadProfile, send_denoised: bool) -> Self {
        let vad = VADEngine::new();
        vad.reset(profile);
        Self {
            rnnoise: RnnoiseProcessor::new(),
            send_denoised,
            vad,
            clean_buf: Vec::with_capacity(CLEAN_CAP),
            buf_start: 0,
            pending: Vec::with_capacity(FRAME_SAMPLES * 4),
            clock: 0,
            speech_start: None,
            last_state: VadState::Silent,
            last_utterance: None,
            stats: SignalStats::default(),
        }
    }

    pub fn stats(&self) -> SignalStats {
        self.stats
    }

    pub fn state(&self) -> VadState {
        self.last_state
    }

    pub fn in_speech(&self) -> bool {
        matches!(self.last_state, VadState::Speech | VadState::Holding)
    }

    pub fn clock(&self) -> u64 {
        self.clock
    }

    /// Feed raw 16 kHz mono samples; events are appended to `out`.
    pub fn push(&mut self, raw: &[f32], out: &mut Vec<SegEvent>) {
        if raw.is_empty() {
            return;
        }

        let (raw_rms, raw_peak) = rms_peak(raw);
        self.stats.raw_rms = raw_rms;
        self.stats.raw_peak = raw_peak;

        // No streaming AGC: it pumps background noise up when speech stops, which
        // delays endpoints. The VAD is level-relative; loudness is fixed per utterance.
        let processed = self.rnnoise.process_16k(raw);
        self.stats.level = rms_peak(&processed).0;

        if self.send_denoised {
            self.clean_buf.extend_from_slice(&processed);
        } else {
            self.clean_buf.extend_from_slice(raw);
        }
        if self.clean_buf.len() > CLEAN_CAP {
            let excess = self.clean_buf.len() - CLEAN_CAP;
            self.clean_buf.drain(..excess);
            self.buf_start += excess as u64;
        }
        self.pending.extend_from_slice(&processed);

        let mut offset = 0;
        while self.pending.len() - offset >= FRAME_SAMPLES {
            let frame = &self.pending[offset..offset + FRAME_SAMPLES];
            offset += FRAME_SAMPLES;
            self.clock += FRAME_SAMPLES as u64;
            let rms = frame_rms(frame);
            let (state, boundary) = self.vad.process_frame(frame, &[]);

            if state == VadState::Speech {
                self.stats.speech_level = if self.stats.speech_level == 0.0 {
                    rms
                } else {
                    self.stats.speech_level * 0.95 + rms * 0.05
                };
            }

            if self.speech_start.is_none() && matches!(state, VadState::Speech | VadState::Holding) {
                // Onset frames precede the state change.
                self.speech_start = Some(self.clock.saturating_sub(4 * FRAME_SAMPLES as u64));
            }

            if state != self.last_state {
                out.push(SegEvent::State(state));
                self.last_state = state;
            }

            if boundary {
                if let Some(u) = self.cut_utterance() {
                    out.push(SegEvent::Utterance(u));
                }
            } else if state == VadState::Silent && self.speech_start.is_some() {
                // Segment closed without a valid boundary: noise burst.
                self.speech_start = None;
                if self.vad.last_segment_rejected() {
                    out.push(SegEvent::Rejected);
                }
            }
        }
        self.pending.drain(..offset);
        self.stats.noise_floor = self.vad.get_noise_floor();
    }

    /// Feed `samples` of digital silence (used when a loopback device stops
    /// delivering packets, so open utterances still reach their endpoint).
    pub fn push_silence(&mut self, samples: usize, out: &mut Vec<SegEvent>) {
        if samples > 0 {
            self.push(&vec![0.0; samples], out);
        }
    }

    fn cut_utterance(&mut self) -> Option<Utterance> {
        let speech_start = self.speech_start.take()?;
        let end = self.clock.min(self.buf_start + self.clean_buf.len() as u64);
        let mut start = speech_start.saturating_sub(LEAD_PAD as u64).max(self.buf_start);

        // Pause = silence between the previous utterance's last speech and this onset.
        let hangover = self.vad.hangover_ms() as u64 * (SAMPLE_RATE as u64 / 1000);
        let prev = self.last_utterance.take().filter(|p| {
            speech_start.saturating_sub(p.end_clock.saturating_sub(hangover)) <= CONTINUATION_GAP as u64
        });
        if let Some(p) = &prev {
            start = start.max(p.end_clock); // no duplicated audio
        }
        if start >= end {
            return None;
        }
        let s = (start - self.buf_start) as usize;
        let e = (end - self.buf_start) as usize;
        let mut samples = self.clean_buf[s..e].to_vec();

        normalize_loudness(&mut samples);

        let merged = prev.is_some();
        let mut start_clock = start;
        if let Some(p) = prev {
            start_clock = p.start_clock;
            let mut joined = p.samples;
            joined.extend_from_slice(&samples);
            samples = joined;
        }
        if samples.len() > MAX_UTTERANCE {
            samples.drain(..samples.len() - MAX_UTTERANCE);
            start_clock = start_clock.max(end - MAX_UTTERANCE as u64);
        }

        let utt = Utterance {
            samples,
            merged,
            start_clock,
            end_clock: end,
        };
        self.last_utterance = Some(utt.clone());
        Some(utt)
    }

    /// Forget the previous utterance (e.g. it was answered long ago or was echo).
    pub fn forget_previous(&mut self) {
        self.last_utterance = None;
    }
}

/// Bring the speech in an utterance to a consistent level (about -20 dBFS on its
/// loud frames) with a soft limiter, so quiet and loud speakers sound alike.
pub fn normalize_loudness(samples: &mut [f32]) {
    let mut frames: Vec<f32> = samples.chunks(FRAME_SAMPLES).map(frame_rms).collect();
    if frames.is_empty() {
        return;
    }
    frames.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let loud = frames[(frames.len() * 9 / 10).min(frames.len() - 1)];
    if loud < 1e-5 {
        return;
    }
    let gain = (0.1 / loud).clamp(0.3, 40.0);
    for s in samples.iter_mut() {
        let x = *s * gain;
        *s = if x.abs() <= 0.8 {
            x
        } else {
            x.signum() * (0.8 + 0.2 * ((x.abs() - 0.8) / 0.2).tanh())
        };
    }
}

pub fn rms_peak(samples: &[f32]) -> (f32, f32) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    let mut sum = 0.0f32;
    let mut peak = 0.0f32;
    for &s in samples {
        sum += s * s;
        peak = peak.max(s.abs());
    }
    ((sum / samples.len() as f32).sqrt(), peak)
}
