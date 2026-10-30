//! Streaming speech worker.
//! The segmenter cuts complete utterances; each becomes one Gemini request.
//! A continuation (question resumed after a pause) cancels the in-flight request
//! and re-asks with the whole question, so the model never answers half of it.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;

use crate::modules::audio::capture_engine::{AudioSource, CaptureEngine};
use crate::modules::audio::lock_free_ring::SpscAudioRing;
use crate::modules::audio::segmenter::{SegEvent, Segmenter, SignalStats, Utterance, SAMPLE_RATE};
use crate::modules::audio::vad_engine::{VadProfile, VadState};
use crate::modules::cognition::state_manager::ConversationStateManager;
use crate::modules::shortcuts::ops::{OpKind, OpRegistry};
use crate::modules::transcription::gemini_voice;

const JOB_WATCHDOG: Duration = Duration::from_secs(25);
const TICK: Duration = Duration::from_millis(20);
/// No packets for this long → feed silence so open utterances still end.
const STARVED_AFTER: Duration = Duration::from_millis(120);
const NO_SIGNAL_AFTER: Duration = Duration::from_secs(2);
const HEALTH_EVERY: Duration = Duration::from_millis(250);
const PREWARM_EVERY: Duration = Duration::from_secs(60);
/// 5 s of per-tick level history for health classification.
const HEALTH_WINDOW: usize = 250;

/// Last time (unix ms) the system-audio worker was inside speech. Used by the mic
/// worker to drop speaker echo when both sources are on without headphones.
static SYSTEM_SPEECH_AT_MS: AtomicU64 = AtomicU64::new(0);
static SYSTEM_ACTIVE: AtomicBool = AtomicBool::new(false);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn db(x: f32) -> f32 {
    20.0 * x.max(1e-6).log10()
}

pub fn spawn_streaming_worker(
    app: AppHandle,
    capture_engine: Arc<CaptureEngine>,
    state_manager: Arc<ConversationStateManager>,
    _shared_inflight: Arc<AtomicUsize>,
    source: AudioSource,
) {
    let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
    match source {
        AudioSource::Microphone => {
            *capture_engine.mic_stop_tx.lock().unwrap() = Some(stop_tx);
        }
        AudioSource::SystemLoopback => {
            *capture_engine.system_stop_tx.lock().unwrap() = Some(stop_tx);
        }
    }

    let from_system = source == AudioSource::SystemLoopback;
    let profile = if from_system {
        VadProfile::SystemLoopback
    } else {
        VadProfile::Microphone
    };
    let source_key = if from_system { "system" } else { "mic" };
    let dispatcher = Dispatcher {
        app: app.clone(),
        state_manager,
        inflight: Arc::new(AtomicUsize::new(0)),
        pending: Arc::new(Mutex::new(None)),
        active_cancel: Arc::new(Mutex::new(None)),
        answered: Arc::new(AtomicBool::new(false)),
        from_system,
        source_label: if from_system { "system" } else { "voice" },
    };

    tokio::spawn(async move {
        let raw_buffer: Arc<SpscAudioRing> = match source {
            AudioSource::Microphone => Arc::clone(&capture_engine.mic_buffer),
            AudioSource::SystemLoopback => Arc::clone(&capture_engine.system_buffer),
        };
        let device_error = Arc::clone(capture_engine.device_error(source));
        if from_system {
            SYSTEM_ACTIVE.store(true, Ordering::Relaxed);
        }
        gemini_voice::prewarm(&app);

        let mut segmenter = Segmenter::new(profile);
        let mut health = HealthTracker::new(from_system);
        let mut scratch = Vec::with_capacity(4096);
        let mut events = Vec::new();
        let mut last_audio = Instant::now();
        let mut last_fed = Instant::now();
        let mut last_health = Instant::now();
        let mut last_prewarm = Instant::now();
        // Echo accounting for the current mic segment.
        let mut seg_ticks = 0u32;
        let mut seg_echo_ticks = 0u32;
        let mut interval = tokio::time::interval(TICK);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                _ = interval.tick() => {}
            }
            dispatcher.drain();
            // Once a request has been answered, the next utterance is a new question,
            // never a continuation of the old one.
            if dispatcher.answered.swap(false, Ordering::Relaxed) {
                segmenter.forget_previous();
            }
            let now = Instant::now();

            raw_buffer.pop_into(&mut scratch);
            if scratch.is_empty() {
                let gap = now.duration_since(last_fed);
                if gap >= STARVED_AFTER {
                    let n = (gap.as_millis() as usize).min(500) * SAMPLE_RATE / 1000;
                    segmenter.push_silence(n, &mut events);
                    last_fed = now;
                }
            } else {
                segmenter.push(&scratch, &mut events);
                last_audio = now;
                last_fed = now;
                let level = segmenter.stats().level;
                let _ = app.emit(
                    "audio-waveform-data",
                    serde_json::json!({ "volume": level, "pitch": 0.0, "source": source_key }),
                );
            }
            let starved = now.duration_since(last_audio) >= NO_SIGNAL_AFTER;
            health.observe(segmenter.stats(), scratch.is_empty());

            if segmenter.in_speech() {
                if from_system {
                    SYSTEM_SPEECH_AT_MS.store(now_ms(), Ordering::Relaxed);
                } else {
                    seg_ticks += 1;
                    let sys_at = SYSTEM_SPEECH_AT_MS.load(Ordering::Relaxed);
                    if SYSTEM_ACTIVE.load(Ordering::Relaxed) && now_ms().saturating_sub(sys_at) < 300 {
                        seg_echo_ticks += 1;
                    }
                }
            }

            for ev in events.drain(..) {
                match ev {
                    SegEvent::State(state) => {
                        let state_str = match state {
                            VadState::Speech => "listening",
                            VadState::Holding => "holding",
                            VadState::Silent | VadState::Trailing => "idle",
                        };
                        let _ = app.emit(
                            "audio-state-changed",
                            serde_json::json!({ "state": state_str, "source": source_key }),
                        );
                    }
                    SegEvent::Rejected => {
                        seg_ticks = 0;
                        seg_echo_ticks = 0;
                        let _ = app.emit(
                            "audio-noise-ignored",
                            serde_json::json!({ "source": source_key }),
                        );
                    }
                    SegEvent::Utterance(utt) => {
                        // Mic heard mostly what the speakers were playing: echo, not the user.
                        let echo = !from_system && seg_ticks > 0 && seg_echo_ticks * 10 >= seg_ticks * 6;
                        seg_ticks = 0;
                        seg_echo_ticks = 0;
                        if echo {
                            segmenter.forget_previous();
                            let _ = app.emit(
                                "audio-noise-ignored",
                                serde_json::json!({ "source": source_key, "reason": "echo" }),
                            );
                            continue;
                        }
                        dispatcher.send(utt);
                    }
                }
            }

            if now.duration_since(last_health) >= HEALTH_EVERY {
                last_health = now;
                let err = device_error.lock().ok().and_then(|e| e.clone());
                let report = health.report(segmenter.stats(), starved, err);
                let _ = app.emit(
                    "audio-health",
                    serde_json::json!({
                        "source": source_key,
                        "signal": report.signal,
                        "detail": report.detail,
                        "levelDb": report.level_db,
                        "noiseDb": report.noise_db,
                        "snrDb": report.snr_db,
                    }),
                );
            }

            if now.duration_since(last_prewarm) >= PREWARM_EVERY
                && dispatcher.inflight.load(Ordering::Relaxed) == 0
            {
                last_prewarm = now;
                gemini_voice::prewarm(&app);
            }
        }

        // Send speech that was still open when capture stopped.
        segmenter.push_silence(SAMPLE_RATE, &mut events);
        for ev in events.drain(..) {
            if let SegEvent::Utterance(utt) = ev {
                dispatcher.send(utt);
            }
        }
        if from_system {
            SYSTEM_ACTIVE.store(false, Ordering::Relaxed);
        }
        let _ = app.emit(
            "audio-health",
            serde_json::json!({ "source": source_key, "signal": "off" }),
        );
    });
}

struct HealthReport {
    signal: &'static str,
    detail: Option<String>,
    level_db: f32,
    noise_db: f32,
    snr_db: Option<f32>,
}

/// Classifies what the user should know about the input right now.
struct HealthTracker {
    from_system: bool,
    raw: VecDeque<(f32, f32)>, // (rms, peak) per tick with audio
}

impl HealthTracker {
    fn new(from_system: bool) -> Self {
        Self {
            from_system,
            raw: VecDeque::with_capacity(HEALTH_WINDOW + 1),
        }
    }

    fn observe(&mut self, stats: SignalStats, empty: bool) {
        if empty {
            return;
        }
        self.raw.push_back((stats.raw_rms, stats.raw_peak));
        if self.raw.len() > HEALTH_WINDOW {
            self.raw.pop_front();
        }
    }

    fn report(&self, stats: SignalStats, starved: bool, device_error: Option<String>) -> HealthReport {
        let level_db = db(stats.level);
        let noise_db = db(stats.noise_floor);
        let snr_db = (stats.speech_level > 0.0).then(|| db(stats.speech_level) - noise_db);
        let mk = |signal: &'static str, detail: Option<&str>| HealthReport {
            signal,
            detail: detail.map(str::to_string),
            level_db,
            noise_db,
            snr_db,
        };

        if let Some(e) = device_error {
            return HealthReport {
                detail: Some(format!("Audio device error: {e}")),
                ..mk("device-error", None)
            };
        }
        if starved {
            return if self.from_system {
                mk("idle", Some("Nothing is playing on this PC"))
            } else {
                mk(
                    "no-signal",
                    Some("Mic sends no audio — check it is not muted or blocked in Windows privacy settings"),
                )
            };
        }
        if self.raw.len() < 25 {
            return mk("ok", None);
        }
        let loudest = self.raw.iter().map(|r| r.0).fold(0.0f32, f32::max);
        let quietest = self.raw.iter().map(|r| r.0).fold(f32::MAX, f32::min);
        let recent_clip = self.raw.iter().rev().take(50).any(|r| r.1 >= 0.99);

        if !self.from_system && loudest < 0.00005 {
            return mk("muted", Some("Mic is muted (digital silence) — unmute it in Windows or on the headset"));
        }
        if recent_clip {
            return mk("clipping", Some("Input is distorting — lower the volume or move the mic away"));
        }
        // Minimum statistics: even the quietest moments are loud → noisy room / music.
        if quietest > 0.03 {
            return mk("noisy", Some("Loud background noise — accuracy may drop; move closer to the mic"));
        }
        if !self.from_system && loudest < 0.004 {
            return mk("quiet", Some("Very low mic level — raise input volume or speak closer"));
        }
        mk("ok", None)
    }
}

/// Owns the in-flight request for one source. New speech cancels the old request
/// (barge-in) and only the newest utterance waits.
#[derive(Clone)]
struct Dispatcher {
    app: AppHandle,
    state_manager: Arc<ConversationStateManager>,
    inflight: Arc<AtomicUsize>,
    pending: Arc<Mutex<Option<Utterance>>>,
    active_cancel: Arc<Mutex<Option<CancellationToken>>>,
    /// Set when a request finished on its own (not superseded by barge-in).
    answered: Arc<AtomicBool>,
    from_system: bool,
    source_label: &'static str,
}

impl Dispatcher {
    fn send(&self, utt: Utterance) {
        let _ = self.app.emit(
            "voice-utterance",
            serde_json::json!({
                "source": self.source_label,
                "durationMs": utt.samples.len() * 1000 / SAMPLE_RATE,
                "merged": utt.merged,
            }),
        );
        if self.inflight.load(Ordering::Relaxed) > 0 {
            if let Some(token) = self.active_cancel.lock().unwrap().take() {
                token.cancel();
            }
            *self.pending.lock().unwrap() = Some(utt);
            return;
        }
        self.spawn_job(utt);
    }

    fn drain(&self) {
        if self.inflight.load(Ordering::Relaxed) > 0 {
            return;
        }
        let next = self.pending.lock().unwrap().take();
        if let Some(utt) = next {
            self.spawn_job(utt);
        }
    }

    fn spawn_job(&self, utt: Utterance) {
        if self.inflight.fetch_add(1, Ordering::Relaxed) > 0 {
            self.inflight.fetch_sub(1, Ordering::Relaxed);
            *self.pending.lock().unwrap() = Some(utt);
            return;
        }

        // Registered before spawning so F3 can cancel it by id from the first moment.
        let (op_id, cancel) = self.app.state::<OpRegistry>().begin(OpKind::Voice);
        *self.active_cancel.lock().unwrap() = Some(cancel.clone());
        let this = self.clone();

        tokio::spawn(async move {
            let app = this.app.clone();
            let started = Instant::now();
            let result = tokio::time::timeout(
                JOB_WATCHDOG,
                gemini_voice::answer_from_audio(
                    app.clone(),
                    &utt.samples,
                    SAMPLE_RATE as u32,
                    this.from_system,
                    cancel.clone(),
                    op_id,
                ),
            )
            .await;

            // F3 may land after the stream finished but before we emit: drop the answer.
            let cancelled = cancel.is_cancelled() || app.state::<OpRegistry>().is_cancelled(op_id);
            match result {
                Ok(Ok(Some(_))) if cancelled => {}
                Ok(Ok(Some(va))) => this.publish(op_id, utt.merged, va),
                Ok(Ok(None)) => {}
                Ok(Err(e)) => {
                    if e != "cancelled" && !cancelled {
                        eprintln!("[API ERROR] {}", e);
                        this.emit_error(op_id, &e);
                    }
                }
                Err(_) => {
                    let msg = format!(
                        "Voice request timed out after {}s — check your connection",
                        started.elapsed().as_secs()
                    );
                    eprintln!("[API ERROR] {}", msg);
                    this.emit_error(op_id, &msg);
                }
            }

            app.state::<OpRegistry>().end(op_id);
            this.inflight.fetch_sub(1, Ordering::Relaxed);
            if !cancel.is_cancelled() {
                this.answered.store(true, Ordering::Relaxed);
                *this.active_cancel.lock().unwrap() = None;
            }
            this.drain();
        });
    }

    fn publish(&self, op_id: u64, merged: bool, va: gemini_voice::VoiceAnswer) {
        let app = &self.app;
        if self.state_manager.is_ai_generating() {
            self.state_manager.set_interrupted(true);
            let _ = app.emit("ai-interrupted", ());
        }
        self.state_manager.set_ai_generating(true);

        gemini_voice::remember_turn(va.question.as_deref(), &va.answer);

        if let Some(question) = va.question.as_deref() {
            if self.from_system {
                let _ = app.emit("voice-system-question", serde_json::json!({ "question": question }));
                crate::modules::cloud_sync::enqueue_voice_event(app, "interviewer", question, "desktop_system");
            }
        }

        let _ = app.emit(
            "voice-gemini-answer",
            serde_json::json!({
                "answer": va.answer,
                "question": va.question,
                "opId": op_id,
                "source": self.source_label,
                "merged": merged,
                "firstTokenMs": va.first_token_ms,
                "totalMs": va.total_ms,
            }),
        );
        crate::modules::cloud_sync::enqueue_voice_event(app, "copilot", &va.answer, "desktop_gemini");
        self.state_manager.set_ai_generating(false);
    }

    fn emit_error(&self, op_id: u64, message: &str) {
        let _ = self.app.emit(
            "voice-gemini-error",
            serde_json::json!({
                "message": message,
                "opId": op_id,
                "source": self.source_label,
            }),
        );
    }
}
