//! Streaming speech worker — NO speech-to-text.
//! capture → RNNoise → AGC → VAD endpoint → one complete utterance → one Gemini call.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::modules::audio::capture_engine::{AudioSource, CaptureEngine};
use crate::modules::audio::chunk_optimizer::{
    ChunkOptimizer, MIN_UTTERANCE_SAMPLES,
};
use crate::modules::audio::rnnoise_processor::RnnoiseProcessor;
use crate::modules::audio::streaming_agc::StreamingAgc;
use crate::modules::audio::vad_engine::{VadProfile, VadState, VADEngine};
use crate::modules::cognition::state_manager::ConversationStateManager;
use crate::modules::transcription::gemini_voice;

const CLEAN_CAP_SAMPLES: usize = 60 * 16_000;

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

        let mut rnnoise = RnnoiseProcessor::new();
        let mut agc = StreamingAgc::new();
        let mut optimizer = ChunkOptimizer::new();

        let mut raw_cursor = 0usize;
        let mut scratch = Vec::with_capacity(4096);
        let mut clean_buf: Vec<f32> = Vec::with_capacity(CLEAN_CAP_SAMPLES);
        let mut vad_cursor = 0usize;
        let mut speech_start: Option<usize> = None;
        let mut recent_speech = Vec::new();
        let mut last_state = vad.get_state();
        let mut remnant = Vec::new();

        // Exactly one Gemini call at a time — prevents overlapping billing.
        let max_inflight = 1usize;
        let mut interval = tokio::time::interval(Duration::from_millis(20));

        loop {
            tokio::select! {
                _ = &mut stop_rx => {
                    if let Some(samples) = optimizer.flush_pending() {
                        submit_voice_job(
                            app.clone(),
                            Arc::clone(&state_manager),
                            Arc::clone(&inflight_voice),
                            samples,
                            from_system,
                            source_label,
                            max_inflight,
                        );
                    }
                    break;
                }
                _ = interval.tick() => {
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
                        optimizer.discard_expired_short();
                        if let Some(samples) = optimizer.flush_if_ready() {
                            submit_voice_job(
                                app.clone(),
                                Arc::clone(&state_manager),
                                Arc::clone(&inflight_voice),
                                samples,
                                from_system,
                                source_label,
                                max_inflight,
                            );
                        }
                        continue;
                    }

                    // Never cut / send while user is mid-speech — only buffer.
                    let mut denoised = rnnoise.process_16k(&scratch);
                    agc.process(&mut denoised);
                    clean_buf.extend_from_slice(&denoised);
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

                        let st = vad.get_state();
                        if matches!(st, VadState::Speech | VadState::Holding) {
                            if speech_start.is_none() {
                                speech_start = Some(abs_pos.saturating_sub(frame_size));
                            }
                            recent_speech.extend_from_slice(&chunk);
                            if recent_speech.len() > 3200 {
                                let excess = recent_speech.len() - 3200;
                                recent_speech.drain(0..excess);
                            }
                        }

                        let (current_state, triggered) = vad.process_frame(&chunk, &recent_speech);
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
                    for &x in &denoised {
                        sum_sq += x * x;
                    }
                    let rms = (sum_sq / denoised.len().max(1) as f32).sqrt();
                    let _ = app.emit(
                        "audio-waveform-data",
                        serde_json::json!({
                            "volume": rms,
                            "pitch": 0.0,
                            "source": source_key,
                        }),
                    );

                    // Endpoint only — never while still speaking.
                    if boundary {
                        let start = speech_start
                            .unwrap_or(0)
                            .saturating_sub(1600)
                            .min(boundary_end);
                        let end = boundary_end.min(clean_buf.len());
                        let utterance = if start < end {
                            clean_buf[start..end].to_vec()
                        } else {
                            Vec::new()
                        };

                        {
                            let mut guard = raw_buffer.lock().unwrap();
                            let drop_n = (guard.len() / 2).min(guard.len());
                            guard.discard_front_samples(drop_n);
                            raw_cursor = guard.len().min(raw_cursor);
                        }
                        if end > 0 && end <= clean_buf.len() {
                            clean_buf.drain(0..end);
                        }
                        vad_cursor = 0;
                        remnant.clear();
                        speech_start = None;
                        recent_speech.clear();

                        // Gate: fragment may be short; optimizer merges or drops (<1s).
                        if !utterance.is_empty() {
                            if let Some(packed) = optimizer.push_utterance(&utterance) {
                                submit_voice_job(
                                    app.clone(),
                                    Arc::clone(&state_manager),
                                    Arc::clone(&inflight_voice),
                                    packed,
                                    from_system,
                                    source_label,
                                    max_inflight,
                                );
                            }
                        }
                    } else {
                        optimizer.discard_expired_short();
                    }
                }
            }
        }
    });
}

fn submit_voice_job(
    app: AppHandle,
    state_manager: Arc<ConversationStateManager>,
    inflight_voice: Arc<AtomicUsize>,
    samples: Vec<f32>,
    from_system: bool,
    source_label: &str,
    max_inflight: usize,
) {
    // Final hard gate — never bill Gemini for tiny clips.
    if samples.len() < MIN_UTTERANCE_SAMPLES {
        return;
    }
    // One in-flight request only (no duplicate concurrent calls).
    if inflight_voice.load(Ordering::Relaxed) >= max_inflight {
        return;
    }

    inflight_voice.fetch_add(1, Ordering::Relaxed);
    let source_label = source_label.to_string();

    tokio::spawn(async move {
        if from_system && state_manager.is_ai_generating() {
            inflight_voice.fetch_sub(1, Ordering::Relaxed);
            return;
        }

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
    });
}
