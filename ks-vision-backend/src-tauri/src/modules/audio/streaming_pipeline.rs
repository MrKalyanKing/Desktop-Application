//! Streaming speech worker.
//! One spoken sentence → exactly one Gemini API request.
//! Next sentences queue FIFO until the current request finishes.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::modules::audio::capture_engine::{AudioSource, CaptureEngine};
use crate::modules::audio::chunk_optimizer::ChunkOptimizer;
use crate::modules::audio::rnnoise_processor::RnnoiseProcessor;
use crate::modules::audio::streaming_agc::StreamingAgc;
use crate::modules::audio::vad_engine::{VadProfile, VadState, VADEngine};
use crate::modules::cognition::state_manager::ConversationStateManager;
use crate::modules::transcription::gemini_voice;

const CLEAN_CAP_SAMPLES: usize = 60 * 16_000;
const SPEECH_LEAD_PAD: usize = 6_400;
/// One sentence minimum (~0.8s) — avoids tiny partial API calls.
const HARD_MIN_SAMPLES: usize = 12_800;

pub fn spawn_streaming_worker(
    app: AppHandle,
    capture_engine: Arc<CaptureEngine>,
    state_manager: Arc<ConversationStateManager>,
    inflight_voice: Arc<AtomicUsize>,
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

    let profile = match source {
        AudioSource::Microphone => VadProfile::Microphone,
        AudioSource::SystemLoopback => VadProfile::SystemLoopback,
    };
    let vad = VADEngine::new();
    vad.reset(profile);

    tokio::spawn(async move {
        let raw_buffer = match source {
            AudioSource::Microphone => Arc::clone(&capture_engine.mic_buffer),
            AudioSource::SystemLoopback => Arc::clone(&capture_engine.system_buffer),
        };
        let source_label = match source {
            AudioSource::Microphone => "Voice",
            AudioSource::SystemLoopback => "System",
        };
        let source_key = if source == AudioSource::Microphone {
            "mic"
        } else {
            "system"
        };
        let from_system = source == AudioSource::SystemLoopback;

        let mut rnnoise = if from_system {
            None
        } else {
            Some(RnnoiseProcessor::new())
        };
        let mut agc = if from_system {
            StreamingAgc::for_meeting()
        } else {
            StreamingAgc::new()
        };
        let mut optimizer = ChunkOptimizer::new();

        let mut raw_cursor = 0usize;
        let mut scratch = Vec::with_capacity(4096);
        let mut clean_buf: Vec<f32> = Vec::with_capacity(CLEAN_CAP_SAMPLES);
        let mut vad_cursor = 0usize;
        let mut speech_start: Option<usize> = None;
        let mut recent_speech = Vec::new();
        let mut last_state = vad.get_state();
        let mut remnant = Vec::new();

        // Exactly ONE Gemini request at a time (one sentence → one API call).
        // Next sentences wait in FIFO queue — never parallel fragment spam.
        let max_inflight = 1usize;
        let pending_queue: Arc<Mutex<VecDeque<Vec<f32>>>> =
            Arc::new(Mutex::new(VecDeque::new()));
        let mut interval = tokio::time::interval(Duration::from_millis(20));

        loop {
            tokio::select! {
                _ = &mut stop_rx => {
                    if let Some(samples) = optimizer.flush_pending() {
                        enqueue_or_run(
                            &app,
                            &state_manager,
                            &inflight_voice,
                            &pending_queue,
                            samples,
                            from_system,
                            source_label,
                            max_inflight,
                        );
                    }
                    break;
                }
                _ = interval.tick() => {
                    drain_pending(
                        &app,
                        &state_manager,
                        &inflight_voice,
                        &pending_queue,
                        from_system,
                        source_label,
                        max_inflight,
                    );

                    {
                        let guard = raw_buffer.lock().unwrap();
                        if guard.len() < raw_cursor {
                            raw_cursor = 0;
                            clean_buf.clear();
                            vad_cursor = 0;
                            speech_start = None;
                            recent_speech.clear();
                            remnant.clear();
                        }
                        guard.copy_from(raw_cursor, &mut scratch);
                        raw_cursor = guard.len();
                    }
                    if scratch.is_empty() {
                        if let Some(samples) = optimizer.flush_if_ready() {
                            enqueue_or_run(
                                &app,
                                &state_manager,
                                &inflight_voice,
                                &pending_queue,
                                samples,
                                from_system,
                                source_label,
                                max_inflight,
                            );
                        }
                        continue;
                    }

                    let mut processed = if let Some(ref mut rn) = rnnoise {
                        rn.process_16k(&scratch)
                    } else {
                        scratch.clone()
                    };
                    agc.process(&mut processed);
                    clean_buf.extend_from_slice(&processed);
                    if clean_buf.len() > CLEAN_CAP_SAMPLES {
                        let excess = clean_buf.len() - CLEAN_CAP_SAMPLES;
                        clean_buf.drain(0..excess);
                        vad_cursor = vad_cursor.saturating_sub(excess);
                        if let Some(s) = speech_start.as_mut() {
                            *s = s.saturating_sub(excess);
                        }
                    }

                    remnant.extend_from_slice(&clean_buf[vad_cursor.min(clean_buf.len())..]);
                    vad_cursor = clean_buf.len();
                    let frame_size = 480;
                    let mut boundary = false;
                    let mut boundary_end = speech_start.unwrap_or(0);

                    while remnant.len() >= frame_size {
                        let chunk: Vec<f32> = remnant.drain(..frame_size).collect();
                        let abs_pos = clean_buf.len().saturating_sub(remnant.len() + frame_size);

                        let (current_state, triggered) = vad.process_frame(&chunk, &recent_speech);

                        if matches!(current_state, VadState::Speech | VadState::Holding) {
                            if speech_start.is_none() {
                                speech_start = Some(abs_pos);
                            }
                            recent_speech.extend_from_slice(&chunk);
                            if recent_speech.len() > 4800 {
                                let excess = recent_speech.len() - 4800;
                                recent_speech.drain(0..excess);
                            }
                        }

                        if current_state != last_state {
                            let state_str = match current_state {
                                VadState::Silent => "idle",
                                VadState::Speech => "listening",
                                VadState::Holding => "holding",
                                VadState::Trailing => "idle",
                            };
                            let _ = app.emit(
                                "audio-state-changed",
                                serde_json::json!({
                                    "state": state_str,
                                    "source": source_key,
                                }),
                            );
                            last_state = current_state;
                        }

                        if triggered {
                            boundary = true;
                            boundary_end = clean_buf.len().saturating_sub(remnant.len());
                            break;
                        }
                    }

                    let mut sum_sq = 0.0f32;
                    for &x in &processed {
                        sum_sq += x * x;
                    }
                    let rms = (sum_sq / processed.len().max(1) as f32).sqrt();
                    let _ = app.emit(
                        "audio-waveform-data",
                        serde_json::json!({
                            "volume": rms,
                            "pitch": 0.0,
                            "source": source_key,
                        }),
                    );

                    if boundary {
                        let start = speech_start
                            .unwrap_or(0)
                            .saturating_sub(SPEECH_LEAD_PAD)
                            .min(boundary_end);
                        let end = boundary_end.min(clean_buf.len());
                        let utterance = if start < end {
                            clean_buf[start..end].to_vec()
                        } else {
                            Vec::new()
                        };

                        {
                            let mut guard = raw_buffer.lock().unwrap();
                            let consumed = raw_cursor.min(guard.len());
                            if consumed > 0 {
                                guard.discard_front_samples(consumed);
                                raw_cursor = raw_cursor.saturating_sub(consumed);
                            }
                        }
                        if end > 0 && end <= clean_buf.len() {
                            clean_buf.drain(0..end);
                        }
                        vad_cursor = 0;
                        remnant.clear();
                        speech_start = None;
                        recent_speech.clear();

                        if !utterance.is_empty() {
                            if let Some(packed) = optimizer.push_utterance(&utterance) {
                                enqueue_or_run(
                                    &app,
                                    &state_manager,
                                    &inflight_voice,
                                    &pending_queue,
                                    packed,
                                    from_system,
                                    source_label,
                                    max_inflight,
                                );
                            }
                        }
                    } else if let Some(samples) = optimizer.flush_if_ready() {
                        enqueue_or_run(
                            &app,
                            &state_manager,
                            &inflight_voice,
                            &pending_queue,
                            samples,
                            from_system,
                            source_label,
                            max_inflight,
                        );
                    }
                }
            }
        }
    });
}

fn enqueue_or_run(
    app: &AppHandle,
    state_manager: &Arc<ConversationStateManager>,
    inflight_voice: &Arc<AtomicUsize>,
    pending_queue: &Arc<Mutex<VecDeque<Vec<f32>>>>,
    samples: Vec<f32>,
    from_system: bool,
    source_label: &str,
    max_inflight: usize,
) {
    if samples.len() < HARD_MIN_SAMPLES {
        return;
    }

    if inflight_voice.load(Ordering::Relaxed) >= max_inflight {
        // Queue — never drop (paid continuous answering).
        let mut q = pending_queue.lock().unwrap();
        if q.len() >= 6 {
            q.pop_front(); // keep newest sentences if extremely backed up
        }
        q.push_back(samples);
        return;
    }

    spawn_voice_job(
        app.clone(),
        Arc::clone(state_manager),
        Arc::clone(inflight_voice),
        Arc::clone(pending_queue),
        samples,
        from_system,
        source_label,
        max_inflight,
    );
}

fn drain_pending(
    app: &AppHandle,
    state_manager: &Arc<ConversationStateManager>,
    inflight_voice: &Arc<AtomicUsize>,
    pending_queue: &Arc<Mutex<VecDeque<Vec<f32>>>>,
    from_system: bool,
    source_label: &str,
    max_inflight: usize,
) {
    loop {
        if inflight_voice.load(Ordering::Relaxed) >= max_inflight {
            break;
        }
        let next = {
            let mut q = pending_queue.lock().unwrap();
            q.pop_front()
        };
        let Some(samples) = next else { break };
        spawn_voice_job(
            app.clone(),
            Arc::clone(state_manager),
            Arc::clone(inflight_voice),
            Arc::clone(pending_queue),
            samples,
            from_system,
            source_label,
            max_inflight,
        );
    }
}

fn spawn_voice_job(
    app: AppHandle,
    state_manager: Arc<ConversationStateManager>,
    inflight_voice: Arc<AtomicUsize>,
    pending_queue: Arc<Mutex<VecDeque<Vec<f32>>>>,
    samples: Vec<f32>,
    from_system: bool,
    source_label: &str,
    max_inflight: usize,
) {
    inflight_voice.fetch_add(1, Ordering::Relaxed);
    let source_label = source_label.to_string();

    tokio::spawn(async move {
        let result = gemini_voice::answer_from_audio(&samples, 16000, from_system).await;

        match result {
            Ok(Some(answer)) => {
                if state_manager.is_ai_generating() {
                    state_manager.set_interrupted(true);
                    let _ = app.emit("ai-interrupted", ());
                }
                state_manager.set_ai_generating(true);
                let _ = app.emit(
                    "voice-gemini-answer",
                    serde_json::json!({
                        "answer": answer,
                        "source": source_label.to_lowercase(),
                    }),
                );
                state_manager.set_ai_generating(false);
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!("[API ERROR] {}", e);
                let _ = app.emit(
                    "voice-gemini-error",
                    serde_json::json!({
                        "message": e,
                        "source": source_label.to_lowercase(),
                    }),
                );
            }
        }

        inflight_voice.fetch_sub(1, Ordering::Relaxed);

        // Immediately start next queued utterance.
        loop {
            if inflight_voice.load(Ordering::Relaxed) >= max_inflight {
                break;
            }
            let next = {
                let mut q = pending_queue.lock().unwrap();
                q.pop_front()
            };
            let Some(samples) = next else { break };
            spawn_voice_job(
                app.clone(),
                Arc::clone(&state_manager),
                Arc::clone(&inflight_voice),
                Arc::clone(&pending_queue),
                samples,
                from_system,
                &source_label,
                max_inflight,
            );
        }
    });
}
