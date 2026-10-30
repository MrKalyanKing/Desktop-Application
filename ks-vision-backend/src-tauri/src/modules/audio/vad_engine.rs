//! Energy + voicing VAD for 30 ms frames at 16 kHz.
//!
//! Robustness in noise comes from four pieces working together:
//! - adaptive noise floor (falls fast, rises slowly; rolling-minimum tracking during speech),
//! - hysteresis (higher threshold to start speech than to stay in it),
//! - onset confirmation (several loud frames, at least one periodic/voiced),
//! - segment validation (too little voiced speech → dropped, never sent to the API).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

pub struct AtomicF32 {
    bits: AtomicU32,
}

impl AtomicF32 {
    pub fn new(val: f32) -> Self {
        Self {
            bits: AtomicU32::new(val.to_bits()),
        }
    }

    pub fn load(&self, order: Ordering) -> f32 {
        f32::from_bits(self.bits.load(order))
    }

    pub fn store(&self, val: f32, order: Ordering) {
        self.bits.store(val.to_bits(), order);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VadState {
    Silent = 0,
    Speech = 1,
    Holding = 2,
    Trailing = 3,
}

impl From<u8> for VadState {
    fn from(val: u8) -> Self {
        match val {
            0 => VadState::Silent,
            1 => VadState::Speech,
            2 => VadState::Holding,
            3 => VadState::Trailing,
            _ => VadState::Silent,
        }
    }
}

/// Mic vs system-loopback need different silence/energy profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadProfile {
    Microphone,
    SystemLoopback,
}

pub const FRAME_SAMPLES: usize = 480;
const FRAME_MS: u32 = 30;

/// Silence needed to close an utterance. Long enough to bridge a mid-sentence pause.
pub const MIC_HANGOVER_MS: u32 = 570;
pub const SYSTEM_HANGOVER_MS: u32 = 660;
/// Hard cap per utterance; the pipeline merges continuations.
pub const MAX_SPEECH_MS: u32 = 15_000;

const ONSET_FRAMES: u32 = 3;
const ONSET_VOICED: u32 = 2;
const MIN_SEGMENT_SPEECH_FRAMES: u32 = 8;
const MIN_SEGMENT_VOICED_FRAMES: u32 = 3;
/// Speech loudness swings with syllables; hum, fans and drones barely move.
const MIN_SEGMENT_MODULATION_DB: f32 = 3.0;
const VOICING_THRESHOLD: f32 = 0.45;
const MIN_WINDOW_FRAMES: usize = 50; // 1.5 s rolling minimum
/// A segment that is this steady after 1 s is a drone (hum, fan, music bed): cut it now.
const STEADY_CHECK_FRAMES: u32 = 33;
const STEADY_MODULATION_DB: f32 = 2.0;
/// Within an utterance, frames this far below the talker's loudness count as silence.
const CONTINUE_REL: f32 = 0.1; // -20 dB
/// The talker has stopped when the last 1 s average sits 12 dB under their
/// loudness, even if background chatter or music keeps frames "voiced".
const TALKER_GONE_FRAMES: usize = 33;
const TALKER_GONE_REL: f32 = 0.251; // -12 dB
/// Foreground-talker memory fades ~1 dB per 10 s.
const FG_DECAY: f32 = 0.999_654;

struct Params {
    enter_mult: f32,
    stay_mult: f32,
    abs_min: f32,
    floor_min: f32,
    hangover_ms: u32,
    /// Segments quieter than the remembered foreground talker by more than this
    /// are background speech (TV, nearby people) and are not sent.
    fg_margin: f32,
}

impl Params {
    fn for_profile(profile: VadProfile) -> Self {
        match profile {
            VadProfile::Microphone => Self {
                enter_mult: 2.6,
                stay_mult: 1.6,
                abs_min: 0.0025,
                floor_min: 0.0002,
                hangover_ms: MIC_HANGOVER_MS,
                fg_margin: 0.178, // -15 dB: the user is close to the mic
            },
            // Meeting audio carries less room noise; voicing + modulation checks
            // reject music beds, so the energy gate can stay sensitive.
            VadProfile::SystemLoopback => Self {
                enter_mult: 2.4,
                stay_mult: 1.6,
                abs_min: 0.003,
                floor_min: 0.0003,
                hangover_ms: SYSTEM_HANGOVER_MS,
                fg_margin: 0.1, // -20 dB: meeting participants vary in level
            },
        }
    }
}

struct Inner {
    profile: VadProfile,
    params: Params,
    noise_floor: f32,
    state: VadState,
    silence_ms: u32,
    speech_ms: u32,
    onset_frames: u32,
    onset_voiced: u32,
    seg_speech_frames: u32,
    seg_voiced_frames: u32,
    /// Sum and sum of squares of frame dB over the segment's speech frames.
    seg_db_sum: f32,
    seg_db_sq: f32,
    /// Smoothed frame level and its maximum over the segment (talker loudness).
    seg_smooth: f32,
    seg_loud: f32,
    /// Loudness of recent accepted speech, decaying slowly.
    fg_level: f32,
    recent_rms: VecDeque<f32>,
    /// Frame RMS of the onset candidate, folded into the segment once confirmed.
    onset_db: Vec<(f32, bool)>,
    /// Consecutive loud frames seen while holding.
    holding_loud_run: u32,
    last_rejected: bool,
}

impl Inner {
    fn new(profile: VadProfile) -> Self {
        let floor = 0.002;
        Self {
            profile,
            params: Params::for_profile(profile),
            noise_floor: floor,
            state: VadState::Silent,
            silence_ms: 0,
            speech_ms: 0,
            onset_frames: 0,
            onset_voiced: 0,
            seg_speech_frames: 0,
            seg_voiced_frames: 0,
            seg_db_sum: 0.0,
            seg_db_sq: 0.0,
            seg_smooth: 0.0,
            seg_loud: 0.0,
            fg_level: 0.0,
            recent_rms: VecDeque::with_capacity(MIN_WINDOW_FRAMES + 1),
            onset_db: Vec::with_capacity(ONSET_FRAMES as usize * 4),
            holding_loud_run: 0,
            last_rejected: false,
        }
    }

    fn adapt_floor(&mut self, rms: f32, alpha_down: f32, alpha_up: f32) {
        let a = if rms < self.noise_floor { alpha_down } else { alpha_up };
        self.noise_floor = ((1.0 - a) * self.noise_floor + a * rms).max(self.params.floor_min);
    }

    fn add_speech_frame(&mut self, rms: f32) {
        let d = 20.0 * rms.max(1e-6).log10();
        self.seg_speech_frames += 1;
        self.seg_db_sum += d;
        self.seg_db_sq += d * d;
        self.seg_smooth = 0.7 * self.seg_smooth + 0.3 * rms;
        self.seg_loud = self.seg_loud.max(self.seg_smooth);
    }

    /// RMS over the last `frames` frames.
    fn recent_level(&self, frames: usize) -> f32 {
        let n = frames.min(self.recent_rms.len()).max(1);
        let sum_sq: f32 = self.recent_rms.iter().rev().take(n).map(|r| r * r).sum();
        (sum_sq / n as f32).sqrt()
    }

    fn segment_stats(&self) -> (f32, f32) {
        let n = self.seg_speech_frames.max(1) as f32;
        let mean = self.seg_db_sum / n;
        let modulation_db = (self.seg_db_sq / n - mean * mean).max(0.0).sqrt();
        (mean, modulation_db)
    }

    fn end_segment(&mut self) -> bool {
        let (mean_db, modulation_db) = self.segment_stats();
        let steady = modulation_db < MIN_SEGMENT_MODULATION_DB;
        let background = self.seg_loud < self.fg_level * self.params.fg_margin;
        let valid = self.seg_speech_frames >= MIN_SEGMENT_SPEECH_FRAMES
            && self.seg_voiced_frames >= MIN_SEGMENT_VOICED_FRAMES
            && !steady
            && !background;
        if valid {
            self.fg_level = self.fg_level.max(self.seg_loud);
        } else if steady && self.seg_speech_frames >= MIN_SEGMENT_SPEECH_FRAMES {
            // A drone that looked like speech: it is the new background level.
            self.noise_floor = self.noise_floor.max(10f32.powf(mean_db / 20.0));
        }
        self.seg_smooth = 0.0;
        self.seg_loud = 0.0;
        self.state = VadState::Silent;
        self.silence_ms = 0;
        self.speech_ms = 0;
        self.seg_speech_frames = 0;
        self.seg_voiced_frames = 0;
        self.seg_db_sum = 0.0;
        self.seg_db_sq = 0.0;
        self.onset_frames = 0;
        self.onset_voiced = 0;
        self.holding_loud_run = 0;
        self.last_rejected = !valid;
        valid
    }

    fn process(&mut self, frame: &[f32]) -> (VadState, bool) {
        self.last_rejected = false;
        let rms = frame_rms(frame);

        self.recent_rms.push_back(rms);
        if self.recent_rms.len() > MIN_WINDOW_FRAMES {
            self.recent_rms.pop_front();
        }

        self.fg_level *= FG_DECAY;
        let enter = (self.noise_floor * self.params.enter_mult).max(self.params.abs_min);
        let stay = (self.noise_floor * self.params.stay_mult).max(self.params.abs_min * 0.6);
        let mut boundary = false;

        match self.state {
            VadState::Silent | VadState::Trailing => {
                if rms > enter {
                    let voiced = periodicity(frame) > VOICING_THRESHOLD;
                    self.onset_frames += 1;
                    if voiced {
                        self.onset_voiced += 1;
                    } else {
                        // Loud but aperiodic (fan switching on, hiss): learn it as noise.
                        self.adapt_floor(rms, 0.15, 0.03);
                    }
                    // Sliding window over the latest onset frames.
                    self.onset_db.push((rms, voiced));
                    if self.onset_db.len() > ONSET_FRAMES as usize + 1 {
                        let (_, old_voiced) = self.onset_db.remove(0);
                        self.onset_frames -= 1;
                        if old_voiced {
                            self.onset_voiced -= 1;
                        }
                    }
                    if self.onset_frames >= ONSET_FRAMES && self.onset_voiced >= ONSET_VOICED {
                        self.state = VadState::Speech;
                        self.speech_ms = self.onset_frames * FRAME_MS;
                        self.silence_ms = 0;
                        for (r, _) in std::mem::take(&mut self.onset_db) {
                            self.add_speech_frame(r);
                        }
                        self.seg_voiced_frames = self.onset_voiced;
                        self.onset_frames = 0;
                        self.onset_voiced = 0;
                    }
                } else {
                    self.onset_frames = 0;
                    self.onset_voiced = 0;
                    self.onset_db.clear();
                    self.adapt_floor(rms, 0.15, 0.02);
                }
            }
            VadState::Speech | VadState::Holding => {
                self.speech_ms += FRAME_MS;

                // Rolling minimum ≈ noise between words; lets the floor follow a
                // noise change mid-speech instead of latching until the hard cap.
                if self.recent_rms.len() >= MIN_WINDOW_FRAMES {
                    let window_min = self.recent_rms.iter().cloned().fold(f32::MAX, f32::min);
                    if window_min > self.noise_floor {
                        self.noise_floor += 0.03 * (window_min - self.noise_floor);
                    }
                }
                if rms < self.noise_floor {
                    self.adapt_floor(rms, 0.05, 0.0);
                }

                let above_floor = rms > stay;
                let periodic = above_floor && periodicity(frame) > VOICING_THRESHOLD;
                // Voiced sound far below this talker is someone else (chatter, TV, music).
                // Unvoiced tails ("s", "t") of the talker's own words stay in.
                let background = periodic && rms < self.seg_loud * CONTINUE_REL;
                let loud = above_floor && !background;
                let voiced = periodic && !background;
                self.holding_loud_run = if loud { self.holding_loud_run + 1 } else { 0 };
                // While holding, a keystroke or tap (1–2 loud aperiodic frames) must not
                // restart the hangover, or typing would keep the utterance open forever.
                let continues = match self.state {
                    VadState::Holding => voiced || self.holding_loud_run >= 3,
                    _ => loud,
                };

                if continues {
                    self.state = VadState::Speech;
                    self.silence_ms = 0;
                    self.add_speech_frame(rms);
                    if voiced && rms > enter {
                        self.seg_voiced_frames += 1;
                    }
                } else {
                    self.state = VadState::Holding;
                    self.silence_ms += FRAME_MS;
                    if self.silence_ms >= self.params.hangover_ms {
                        boundary = self.end_segment();
                    }
                }

                if self.state != VadState::Silent && self.speech_ms >= MAX_SPEECH_MS {
                    boundary = self.end_segment();
                } else if self.state != VadState::Silent
                    && self.seg_speech_frames as usize >= TALKER_GONE_FRAMES
                    && self.recent_level(TALKER_GONE_FRAMES) < self.seg_loud * TALKER_GONE_REL
                {
                    boundary = self.end_segment();
                } else if self.state != VadState::Silent
                    && self.seg_speech_frames == STEADY_CHECK_FRAMES
                    && self.segment_stats().1 < STEADY_MODULATION_DB
                {
                    boundary = self.end_segment();
                }
            }
        }

        (self.state, boundary)
    }
}

pub struct VADEngine {
    inner: Mutex<Inner>,
}

impl VADEngine {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner::new(VadProfile::Microphone)),
        }
    }

    pub fn reset(&self, profile: VadProfile) {
        *self.inner.lock().unwrap() = Inner::new(profile);
    }

    pub fn profile(&self) -> VadProfile {
        self.inner.lock().unwrap().profile
    }

    pub fn get_state(&self) -> VadState {
        self.inner.lock().unwrap().state
    }

    pub fn get_noise_floor(&self) -> f32 {
        self.inner.lock().unwrap().noise_floor
    }

    pub fn hangover_ms(&self) -> u32 {
        self.inner.lock().unwrap().params.hangover_ms
    }

    /// True when the last frame closed a segment that was too short / unvoiced to send.
    pub fn last_segment_rejected(&self) -> bool {
        self.inner.lock().unwrap().last_rejected
    }

    /// Process a 30ms frame of mono samples at 16kHz (480 samples).
    /// Returns true if a valid boundary (end of utterance) is detected.
    pub fn process_frame(&self, frame: &[f32], _recent_speech_samples: &[f32]) -> (VadState, bool) {
        if frame.is_empty() {
            return (self.get_state(), false);
        }
        self.inner.lock().unwrap().process(frame)
    }
}

impl Default for VADEngine {
    fn default() -> Self {
        Self::new()
    }
}

pub fn frame_rms(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = frame.iter().map(|s| s * s).sum();
    (sum_sq / frame.len() as f32).sqrt()
}

/// Pitch-peak strength of the normalized autocorrelation (60–400 Hz lags at 16 kHz).
/// Voiced speech is strongly periodic; fans, hiss and clicks are not.
/// Pre-emphasis whitens rumble/pink noise, and only peaks after the first dip
/// count — low-passed noise is merely "smooth", which would otherwise read as periodic.
pub fn periodicity(frame: &[f32]) -> f32 {
    let n = frame.len();
    let min_lag = 40; // 400 Hz
    let max_lag = 266.min(n.saturating_sub(120)); // 60 Hz, keep ≥120 overlapping samples
    if max_lag <= min_lag {
        return 0.0;
    }
    let mut x = Vec::with_capacity(n);
    let mut prev = frame[0];
    for &s in frame {
        x.push(s - 0.9 * prev);
        prev = s;
    }
    let mean = x.iter().sum::<f32>() / n as f32;
    x.iter_mut().for_each(|v| *v -= mean);

    let corr_at = |lag: usize| {
        let (mut c, mut ex, mut ey) = (0.0f32, 0.0f32, 0.0f32);
        for i in 0..n - lag {
            c += x[i] * x[i + lag];
            ex += x[i] * x[i];
            ey += x[i + lag] * x[i + lag];
        }
        let norm = (ex * ey).sqrt();
        if norm > 1e-9 {
            c / norm
        } else {
            0.0
        }
    };

    let mut dipped = false;
    let mut best = 0.0f32;
    for lag in (2..=max_lag).step_by(2) {
        let r = corr_at(lag);
        if !dipped {
            dipped = r < 0.1;
            continue;
        }
        if lag >= min_lag {
            best = best.max(r);
        }
    }
    best
}

pub fn estimate_pitch(samples: &[f32], sample_rate: u32) -> Option<f32> {
    let n = samples.len();
    if n < 320 {
        return None;
    }

    let min_freq = 50.0;
    let max_freq = 400.0;
    let max_lag = (sample_rate as f32 / min_freq) as usize;
    let min_lag = (sample_rate as f32 / max_freq) as usize;

    let mut best_lag = 0;
    let mut best_corr = -1.0;

    for lag in min_lag..=max_lag {
        let mut corr = 0.0;
        let mut energy_x = 0.0;
        let mut energy_y = 0.0;

        let limit = n - lag;
        for i in 0..limit {
            let x = samples[i];
            let y = samples[i + lag];
            corr += x * y;
            energy_x += x * x;
            energy_y += y * y;
        }

        let norm = (energy_x * energy_y).sqrt();
        if norm > 1e-6 {
            let r = corr / norm;
            if r > best_corr {
                best_corr = r;
                best_lag = lag;
            }
        }
    }

    if best_corr > 0.4 && best_lag > 0 {
        Some(sample_rate as f32 / best_lag as f32)
    } else {
        None
    }
}
