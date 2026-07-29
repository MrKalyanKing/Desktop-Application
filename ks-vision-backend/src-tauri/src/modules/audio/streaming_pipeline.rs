//! Ultra-low-latency streaming speech worker:
//! capture → RNNoise → AGC → VAD → overlapping Whisper → incremental transcript → intent → Gemini text.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::modules::audio::capture_engine::{AudioSource, CaptureEngine};
use crate::modules::audio::finalize;
use crate::modules::audio::perf_metrics;
use crate::modules::audio::preprocess;
use crate::modules::audio::rnnoise_processor::RnnoiseProcessor;
use crate::modules::audio::speaker_tracker::SpeakerTracker;
use crate::modules::audio::streaming_agc::StreamingAgc;
use crate::modules::audio::vad_engine::{VadProfile, VadState, VADEngine};
use crate::modules::cognition::question_parser::QuestionParser;
use crate::modules::cognition::state_manager::ConversationStateManager;
use crate::modules::transcription::gemini_service::GeminiTranscriptionService;
use crate::modules::transcription::intent_detector;
use crate::modules::transcription::postprocess;
use crate::modules::transcription::transcript_builder::IncrementalTranscriptBuilder;
use crate::modules::transcription::whisper_engine::WhisperEngine;

const WINDOW_SAMPLES: usize = 32_000; // ~2.0s
const HOP_SAMPLES: usize = 11_200; // ~0.7s
const MIN_PARTIAL_SAMPLES: usize = 8_000; // ~0.5s
const CLEAN_CAP_SAMPLES: usize = 60 * 16_000;

pub fn spawn_streaming_worker(
    app: AppHandle,
    capture_engine: Arc<CaptureEngine>,
    state_manager: Arc<ConversationStateManager>,
    speaker_tracker: Arc<SpeakerTracker>,
    _transcription_service: Arc<GeminiTranscriptionService>,
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
        let builder = Arc::new(Mutex::new(IncrementalTranscriptBuilder::new()));
        let early_dispatched = Arc::new(AtomicBool::new(false));
        let partial_inflight = Arc::new(AtomicUsize::new(0));

        let mut raw_cursor = 0usize;
        let mut scratch = Vec::with_capacity(4096);
        let mut clean_buf: Vec<f32> = Vec::with_capacity(CLEAN_CAP_SAMPLES);
        let mut vad_cursor = 0usize;
        let mut speech_start: Option<usize> = None;
        let mut recent_speech = Vec::new();
        let mut last_state = vad.get_state();
        let mut last_partial_at = 0usize;
        let mut remnant = Vec::new();

        let max_final_inflight = if source == AudioSource::SystemLoopback {
            2
        } else {
            1
        };
        let mut interval = tokio::time::interval(Duration::from_millis(20));

        loop {
            tokio::select! {
                _ = &mut stop_rx => { break; }
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
                            builder.lock().unwrap().reset();
                            last_partial_at = 0;
                            early_dispatched.store(false, Ordering::Relaxed);
                        }
                        guard.copy_from(raw_cursor, &mut scratch);
                        raw_cursor = guard.len();
                    }
                    if scratch.is_empty() {
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
                        last_partial_at = last_partial_at.saturating_sub(excess);
                    }

                    // Frame-align VAD input
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
                                builder.lock().unwrap().reset();
                                last_partial_at = abs_pos;
                                early_dispatched.store(false, Ordering::Relaxed);
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

                    // Waveform from latest denoised chunk
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

                    // Overlapping partial Whisper while speaking
                    if matches!(vad.get_state(), VadState::Speech | VadState::Holding) {
                        if let Some(start) = speech_start {
                            let end = clean_buf.len();
                            if end.saturating_sub(last_partial_at) >= HOP_SAMPLES
                                && end.saturating_sub(start) >= MIN_PARTIAL_SAMPLES
                                && partial_inflight.load(Ordering::Relaxed) == 0
                                && !early_dispatched.load(Ordering::Relaxed)
                            {
                                let win_start = end.saturating_sub(WINDOW_SAMPLES).max(start);
                                let window = clean_buf[win_start..end].to_vec();
                                last_partial_at = end;
                                partial_inflight.fetch_add(1, Ordering::Relaxed);

                                let app2 = app.clone();
                                let builder2 = Arc::clone(&builder);
                                let early2 = Arc::clone(&early_dispatched);
                                let inflight = Arc::clone(&partial_inflight);
                                let capture_engine2 = Arc::clone(&capture_engine);
                                let state_manager2 = Arc::clone(&state_manager);
                                let source_label2 = source_label.to_string();
                                let source_key2 = source_key.to_string();
                                let prompt = builder.lock().unwrap().current().to_string();

                                tokio::spawn(async move {
                                    let t0 = perf_metrics::now();
                                    let prepared = preprocess::prepare_for_stt(&window);
                                    let whisper = WhisperEngine::new();
                                    let result = whisper
                                        .transcribe_with_prompt(
                                            &prepared,
                                            16000,
                                            if prompt.is_empty() {
                                                None
                                            } else {
                                                Some(prompt.as_str())
                                            },
                                        )
                                        .await;
                                    perf_metrics::log_stage("whisper_partial", &source_key2, t0);
                                    inflight.fetch_sub(1, Ordering::Relaxed);

                                    let Ok(r) = result else { return };
                                    if r.text.trim().is_empty() {
                                        return;
                                    }

                                    let t_ref = perf_metrics::now();
                                    let cleaned =
                                        postprocess::postprocess_transcript(&r.text, &prompt);
                                    perf_metrics::log_stage(
                                        "transcript_refinement",
                                        &source_key2,
                                        t_ref,
                                    );

                                    let updated = {
                                        let mut b = builder2.lock().unwrap();
                                        b.ingest_partial(&cleaned)
                                    };

                                    if let Some(text) = updated {
                                        let _ = app2.emit(
                                            "audio-transcription",
                                            serde_json::json!({
                                                "text": text,
                                                "speaker": if source == AudioSource::Microphone { "You" } else { "Speaker" },
                                                "source": source_label2.to_lowercase(),
                                                "status": "partial",
                                            }),
                                        );

                                        // Early exclusive-mic dispatch when question looks complete
                                        let exclusive_mic = source == AudioSource::Microphone
                                            && !capture_engine2
                                                .is_recording(AudioSource::SystemLoopback);
                                        if exclusive_mic
                                            && intent_detector::looks_like_complete_question(&text)
                                            && !intent_detector::is_trivial_utterance(&text)
                                            && !early2.load(Ordering::Relaxed)
                                            && !state_manager2.is_ai_generating()
                                        {
                                            early2.store(true, Ordering::Relaxed);
                                            let t_q = perf_metrics::now();
                                            println!("[INTENT] Early question: {}", text);
                                            finalize::enqueue_and_trigger_text(
                                                &app2,
                                                &state_manager2,
                                                &text,
                                            )
                                            .await;
                                            perf_metrics::log_stage(
                                                "question_detection",
                                                "early",
                                                t_q,
                                            );
                                        }
                                    }
                                });
                            }
                        }
                    }

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
                        last_partial_at = 0;

                        let prior = builder.lock().unwrap().current().to_string();
                        builder.lock().unwrap().reset();
                        let already = early_dispatched.swap(false, Ordering::Relaxed);

                        if utterance.len() >= 1600
                            && inflight_stt.load(Ordering::Relaxed) < max_final_inflight
                        {
                            inflight_stt.fetch_add(1, Ordering::Relaxed);
                            let app2 = app.clone();
                            let capture_engine2 = Arc::clone(&capture_engine);
                            let state_manager2 = Arc::clone(&state_manager);
                            let speaker_tracker2 = Arc::clone(&speaker_tracker);
                            let question_parser2 = Arc::clone(&question_parser);
                            let inflight2 = Arc::clone(&inflight_stt);

                            tokio::spawn(async move {
                                let t0 = perf_metrics::now();
                                let prepared = preprocess::prepare_for_stt(&utterance);
                                let whisper = WhisperEngine::new();
                                let final_res = whisper
                                    .transcribe_with_prompt(
                                        &prepared,
                                        16000,
                                        if prior.is_empty() {
                                            None
                                        } else {
                                            Some(prior.as_str())
                                        },
                                    )
                                    .await;
                                perf_metrics::log_stage("whisper_final", source_label, t0);

                                let (text, confidence) = match final_res {
                                    Ok(r) => {
                                        let t_ref = perf_metrics::now();
                                        let cleaned =
                                            postprocess::postprocess_transcript(&r.text, &prior);
                                        perf_metrics::log_stage(
                                            "transcript_refinement",
                                            source_label,
                                            t_ref,
                                        );
                                        let best = if cleaned.len() >= prior.len() {
                                            cleaned
                                        } else if !prior.is_empty() {
                                            prior
                                        } else {
                                            cleaned
                                        };
                                        (best, r.confidence)
                                    }
                                    Err(e) => {
                                        eprintln!("[Whisper] final failed: {}", e);
                                        (prior, 0.7)
                                    }
                                };

                                if already {
                                    let _ = app2.emit(
                                        "audio-transcription",
                                        serde_json::json!({
                                            "text": text,
                                            "speaker": "You",
                                            "source": source_label.to_lowercase(),
                                            "status": "final",
                                        }),
                                    );
                                    inflight2.fetch_sub(1, Ordering::Relaxed);
                                    return;
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
                                    source_label,
                                    &utterance,
                                )
                                .await;
                                inflight2.fetch_sub(1, Ordering::Relaxed);
                            });
                        }
                    }
                }
            }
        }
    });
}
