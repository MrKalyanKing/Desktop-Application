//! Streaming speech worker (no Whisper):
//! capture → RNNoise → AGC → VAD → optimized chunk → Gemini multimodal audio → intent → text Gemini.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::modules::audio::capture_engine::{AudioSource, CaptureEngine};
use crate::modules::audio::chunk_optimizer::ChunkOptimizer;
use crate::modules::audio::finalize;
use crate::modules::audio::perf_metrics;
use crate::modules::audio::rnnoise_processor::RnnoiseProcessor;
use crate::modules::audio::speaker_tracker::SpeakerTracker;
use crate::modules::audio::streaming_agc::StreamingAgc;
use crate::modules::audio::vad_engine::{VadProfile, VadState, VADEngine};
use crate::modules::cognition::question_parser::QuestionParser;
use crate::modules::cognition::state_manager::ConversationStateManager;
use crate::modules::transcription::gemini_service::GeminiTranscriptionService;

const CLEAN_CAP_SAMPLES: usize = 60 * 16_000;
/// Merge clips shorter than ~0.7s within 900ms to cut Gemini request count.
const MIN_SEND_SAMPLES: usize = 11_200;
const MERGE_WINDOW_MS: u64 = 900;
const MIN_REQUEST_GAP_MS: u64 = 400;

pub fn spawn_streaming_worker(
    app: AppHandle,
    capture_engine: Arc<CaptureEngine>,
    state_manager: Arc<ConversationStateManager>,
    speaker_tracker: Arc<SpeakerTracker>,
    transcription_service: Arc<GeminiTranscriptionService>,
    question_parser: Arc<QuestionParser>,
    inflight_stt: Arc<AtomicUsize>,
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

        let max_final_inflight = if source == AudioSource::SystemLoopback {
            2
        } else {
            1
        };
        let mut interval = tokio::time::interval(Duration::from_millis(20));

        loop {
            tokio::select! {
                _ = &mut stop_rx => {
                    // Flush any merged short clips on stop
                    if let Some(samples) = optimizer.flush_pending() {
                        submit_audio_job(
                            app.clone(),
                            Arc::clone(&capture_engine),
                            Arc::clone(&state_manager),
                            Arc::clone(&speaker_tracker),
                            Arc::clone(&transcription_service),
                            Arc::clone(&question_parser),
                            Arc::clone(&inflight_stt),
                            samples,
                            source,
                            source_label,
                            max_final_inflight,
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
                        if let Some(samples) = optimizer.flush_if_expired(Duration::from_millis(MERGE_WINDOW_MS)) {
                            submit_audio_job(
                                app.clone(),
                                Arc::clone(&capture_engine),
                                Arc::clone(&state_manager),
                                Arc::clone(&speaker_tracker),
                                Arc::clone(&transcription_service),
                                Arc::clone(&question_parser),
                                Arc::clone(&inflight_stt),
                                samples,
                                source,
                                source_label,
                                max_final_inflight,
                            );
                        }
                        continue;
                    }

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

                    let t_vad = perf_metrics::now();
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
                    perf_metrics::log_stage("vad", source_key, t_vad);

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

                        if utterance.len() >= 1600 {
                            if let Some(packed) = optimizer.push_utterance(
                                &utterance,
                                MIN_SEND_SAMPLES,
                                Duration::from_millis(MERGE_WINDOW_MS),
                                Duration::from_millis(MIN_REQUEST_GAP_MS),
                            ) {
                                submit_audio_job(
                                    app.clone(),
                                    Arc::clone(&capture_engine),
                                    Arc::clone(&state_manager),
                                    Arc::clone(&speaker_tracker),
                                    Arc::clone(&transcription_service),
                                    Arc::clone(&question_parser),
                                    Arc::clone(&inflight_stt),
                                    packed,
                                    source,
                                    source_label,
                                    max_final_inflight,
                                );
                            }
                        }
                    }
                }
            }
        }
    });
}

fn submit_audio_job(
    app: AppHandle,
    capture_engine: Arc<CaptureEngine>,
    state_manager: Arc<ConversationStateManager>,
    speaker_tracker: Arc<SpeakerTracker>,
    transcription_service: Arc<GeminiTranscriptionService>,
    question_parser: Arc<QuestionParser>,
    inflight_stt: Arc<AtomicUsize>,
    samples: Vec<f32>,
    source: AudioSource,
    source_label: &str,
    max_final_inflight: usize,
) {
    if samples.len() < 1600 {
        return;
    }
    if inflight_stt.load(Ordering::Relaxed) >= max_final_inflight {
        println!("[CHUNK OPT] Dropping clip — inflight STT saturated");
        return;
    }

    inflight_stt.fetch_add(1, Ordering::Relaxed);
    let app2 = app;
    let capture_engine2 = capture_engine;
    let state_manager2 = state_manager;
    let speaker_tracker2 = speaker_tracker;
    let transcription_service2 = transcription_service;
    let question_parser2 = question_parser;
    let inflight2 = inflight_stt;
    let source_label = source_label.to_string();

    tokio::spawn(async move {
        if source == AudioSource::SystemLoopback && state_manager2.is_ai_generating() {
            inflight2.fetch_sub(1, Ordering::Relaxed);
            return;
        }

        let speaker_id = if source == AudioSource::Microphone {
            "You".to_string()
        } else {
            speaker_tracker2.identify_speaker(&samples, 16000)
        };
        let prev = state_manager2.get_history(2);

        let t0 = perf_metrics::now();
        let result = transcription_service2
            .transcribe(&samples, &source_label, &speaker_id, &prev)
            .await;
        perf_metrics::log_stage("gemini_audio_pipeline", &source_label, t0);

        match result {
            Ok((text, confidence)) => {
                if text.trim().is_empty() {
                    inflight2.fetch_sub(1, Ordering::Relaxed);
                    return;
                }

                if let Some(last) = state_manager2.get_history(1).last() {
                    let a = last.text.trim().to_lowercase();
                    let b = text.trim().to_lowercase();
                    if a == b {
                        println!("[CHUNK OPT] Skipping duplicate transcript");
                        inflight2.fetch_sub(1, Ordering::Relaxed);
                        return;
                    }
                }

                finalize::finalize_transcript(
                    &app2,
                    &capture_engine2,
                    &state_manager2,
                    &speaker_tracker2,
                    &question_parser2,
                    &text,
                    confidence,
                    source,
                    &source_label,
                    &samples,
                )
                .await;
            }
            Err(e) => eprintln!("[Gemini Audio] {}", e),
        }
        inflight2.fetch_sub(1, Ordering::Relaxed);
    });
}
