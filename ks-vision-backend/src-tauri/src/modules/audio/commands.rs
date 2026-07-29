use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{State, Manager, AppHandle, Emitter};
use cpal::traits::StreamTrait;
use crate::modules::audio::capture_engine::{CaptureEngine, AudioSource, SendStream};
use crate::modules::audio::speaker_tracker::SpeakerTracker;
use crate::modules::cognition::state_manager::ConversationStateManager;
use crate::modules::audio::segmentation_engine::SegmentationEngine;
use crate::modules::audio::vad_engine::VadProfile;
use crate::modules::transcription::gemini_service::{GeminiTranscriptionService, TranscriptChunk};
use crate::modules::cognition::question_parser::{QuestionParser, SubQuestion};
use crate::modules::audio::echo_reference::{EchoReferenceBuffer, AIAudioOutputSink};
use crate::modules::audio::preprocess;

pub struct AudioState {
    pub capture_engine: Arc<CaptureEngine>,
    pub speaker_tracker: Arc<SpeakerTracker>,
    pub state_manager: Arc<ConversationStateManager>,
    pub segmentation_engine: Arc<SegmentationEngine>,
    pub transcription_service: Arc<GeminiTranscriptionService>,
    pub question_parser: Arc<QuestionParser>,
    pub inflight_stt: Arc<AtomicUsize>,
}

impl AudioState {
    pub fn new() -> Self {
        Self {
            capture_engine: Arc::new(CaptureEngine::new()),
            speaker_tracker: Arc::new(SpeakerTracker::new()),
            state_manager: Arc::new(ConversationStateManager::new()),
            segmentation_engine: Arc::new(SegmentationEngine::new()),
            transcription_service: Arc::new(GeminiTranscriptionService::new()),
            question_parser: Arc::new(QuestionParser::new()),
            inflight_stt: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[derive(serde::Serialize)]
pub struct CaptureStateResponse {
    pub is_recording: bool,
    pub mode: String,
}

pub fn get_echo_reference(app: &AppHandle) -> Arc<EchoReferenceBuffer> {
    let state: State<'_, AudioState> = app.state::<AudioState>();
    Arc::clone(&state.capture_engine.ai_output_monitor)
}

fn spawn_background_worker(app: AppHandle, state: Arc<AudioState>, source: AudioSource) {
    let capture_engine = Arc::clone(&state.capture_engine);
    let state_manager = Arc::clone(&state.state_manager);
    let speaker_tracker = Arc::clone(&state.speaker_tracker);
    let transcription_service = Arc::clone(&state.transcription_service);
    let question_parser = Arc::clone(&state.question_parser);
    let inflight_stt = Arc::clone(&state.inflight_stt);

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
    let vad = crate::modules::audio::vad_engine::VADEngine::new();
    vad.reset(profile);

    tokio::spawn(async move {
        let buffer = match source {
            AudioSource::Microphone => Arc::clone(&capture_engine.mic_buffer),
            AudioSource::SystemLoopback => Arc::clone(&capture_engine.system_buffer),
        };
        let source_label = match source {
            AudioSource::Microphone => "Voice",
            AudioSource::SystemLoopback => "System",
        };

        let mut processed_samples_len = 0usize;
        let mut speech_start: Option<usize> = None;
        let mut recent_speech_samples = Vec::new();
        let mut last_state = vad.get_state();
        let mut interval = tokio::time::interval(Duration::from_millis(30));
        let max_inflight = if source == AudioSource::SystemLoopback { 2 } else { 1 };

        loop {
            tokio::select! {
                _ = &mut stop_rx => { break; }
                _ = interval.tick() => {
                    let all_samples = buffer.lock().unwrap().get_samples();
                    if all_samples.len() < processed_samples_len {
                        processed_samples_len = 0;
                        speech_start = None;
                        recent_speech_samples.clear();
                    }
                    if all_samples.len() <= processed_samples_len { continue; }

                    let new_samples = &all_samples[processed_samples_len..];
                    let mut sum_sq = 0.0f32;
                    for &x in new_samples { sum_sq += x * x; }
                    let rms = (sum_sq / new_samples.len().max(1) as f32).sqrt();
                    let pitch = crate::modules::audio::vad_engine::estimate_pitch(new_samples, 16000).unwrap_or(0.0);
                    let _ = app.emit("audio-waveform-data", serde_json::json!({
                        "volume": rms, "pitch": pitch,
                        "source": if source == AudioSource::Microphone { "mic" } else { "system" }
                    }));

                    let frame_size = 480;
                    let mut reset_occurred = false;
                    for chunk in new_samples.chunks(frame_size) {
                        if chunk.len() != frame_size { continue; }
                        let current_vad_state = vad.get_state();
                        if matches!(current_vad_state,
                            crate::modules::audio::vad_engine::VadState::Speech
                            | crate::modules::audio::vad_engine::VadState::Holding)
                        {
                            if speech_start.is_none() { speech_start = Some(processed_samples_len); }
                            recent_speech_samples.extend_from_slice(chunk);
                            if recent_speech_samples.len() > 3200 {
                                let excess = recent_speech_samples.len() - 3200;
                                recent_speech_samples.drain(0..excess);
                            }
                        }
                        let (current_state, boundary_triggered) = vad.process_frame(chunk, &recent_speech_samples);
                        if current_state != last_state {
                            let state_str = match current_state {
                                crate::modules::audio::vad_engine::VadState::Silent => "idle",
                                crate::modules::audio::vad_engine::VadState::Speech => "listening",
                                crate::modules::audio::vad_engine::VadState::Holding => "holding",
                                crate::modules::audio::vad_engine::VadState::Trailing => "idle",
                            };
                            let _ = app.emit("audio-state-changed", serde_json::json!({
                                "state": state_str,
                                "source": if source == AudioSource::Microphone { "mic" } else { "system" }
                            }));
                            last_state = current_state;
                        }
                        if boundary_triggered {
                            let end = (processed_samples_len + chunk.len()).min(all_samples.len());
                            let start = speech_start.unwrap_or(0).saturating_sub(3200).min(end);
                            let utterance = all_samples[start..end].to_vec();
                            buffer.lock().unwrap().discard_front_samples(end);
                            processed_samples_len = 0;
                            speech_start = None;
                            recent_speech_samples.clear();
                            reset_occurred = true;
                            if utterance.len() >= 1600 {
                                let current_inflight = inflight_stt.load(Ordering::Relaxed);
                                if current_inflight < max_inflight {
                                    inflight_stt.fetch_add(1, Ordering::Relaxed);
                                    let app2 = app.clone();
                                    let capture_engine2 = Arc::clone(&capture_engine);
                                    let state_manager2 = Arc::clone(&state_manager);
                                    let speaker_tracker2 = Arc::clone(&speaker_tracker);
                                    let transcription_service2 = Arc::clone(&transcription_service);
                                    let question_parser2 = Arc::clone(&question_parser);
                                    let inflight2 = Arc::clone(&inflight_stt);
                                    tokio::spawn(async move {
                                        process_utterance(
                                            &app2,
                                            &capture_engine2,
                                            &state_manager2,
                                            &speaker_tracker2,
                                            &transcription_service2,
                                            &question_parser2,
                                            &utterance,
                                            source,
                                            source_label,
                                        )
                                        .await;
                                        inflight2.fetch_sub(1, Ordering::Relaxed);
                                    });
                                }
                            }
                            break;
                        }
                    }
                    if !reset_occurred { processed_samples_len = all_samples.len(); }
                }
            }
        }
    });
}

fn start_one_source(app: &AppHandle, state: &AudioState, source: AudioSource) -> Result<(), String> {
    if state.capture_engine.is_recording(source) { return Ok(()); }
    let state_clone = Arc::new(AudioState {
        capture_engine: Arc::clone(&state.capture_engine),
        speaker_tracker: Arc::clone(&state.speaker_tracker),
        state_manager: Arc::clone(&state.state_manager),
        segmentation_engine: Arc::clone(&state.segmentation_engine),
        transcription_service: Arc::clone(&state.transcription_service),
        question_parser: Arc::clone(&state.question_parser),
        inflight_stt: Arc::clone(&state.inflight_stt),
    });
    let old_stream = state.capture_engine.start_capture(source, move |_s, _src| {}).map_err(|e| e.to_string())?;
    if let Some(send_stream) = old_stream {
        let _ = app.run_on_main_thread(move || { drop(send_stream); });
    }
    spawn_background_worker(app.clone(), state_clone, source);
    Ok(())
}

async fn stop_one_source(app: &AppHandle, state: &AudioState, source: AudioSource) -> Result<String, String> {
    if !state.capture_engine.is_recording(source) { return Ok(String::new()); }
    let stop_tx = match source {
        AudioSource::Microphone => state.capture_engine.mic_stop_tx.lock().unwrap().take(),
        AudioSource::SystemLoopback => state.capture_engine.system_stop_tx.lock().unwrap().take(),
    };
    if let Some(tx) = stop_tx { let _ = tx.send(()); }
    let (samples, stream_opt) = state.capture_engine.stop_capture(source).await;
    if let Some(send_stream) = stream_opt {
        let SendStream(ref stream) = send_stream;
        let _ = stream.pause();
        let _ = app.run_on_main_thread(move || { drop(send_stream); });
    }
    if samples.len() < 1600 { return Ok(String::new()); }
    let source_label = if source == AudioSource::Microphone { "Voice" } else { "System" };
    let speaker_id = if source == AudioSource::Microphone { "You".to_string() } else { state.speaker_tracker.identify_speaker(&samples, 16000) };
    let prev_context = state.state_manager.get_history(3);
    match state.transcription_service.transcribe(&samples, source_label, &speaker_id, &prev_context).await {
        Ok((text, _)) => {
            if !text.is_empty() {
                state.state_manager.add_to_history(TranscriptChunk {
                    text: text.clone(), speaker: speaker_id,
                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                });
                let _ = crate::modules::database::history::save_message(app, if source == AudioSource::Microphone { "user" } else { "system_participant" }, &text, source_label);
            }
            Ok(text)
        }
        Err(e) => Err(e),
    }
}

async fn process_utterance(
    app: &AppHandle,
    capture_engine: &CaptureEngine,
    state_manager: &ConversationStateManager,
    speaker_tracker: &SpeakerTracker,
    transcription_service: &GeminiTranscriptionService,
    question_parser: &QuestionParser,
    samples: &[f32],
    source: AudioSource,
    source_label: &str,
) {
    if samples.is_empty() {
        return;
    }

    // While AI is answering, skip system loopback to avoid feeding TTS/UI sounds back in
    if source == AudioSource::SystemLoopback && state_manager.is_ai_generating() {
        return;
    }

    // AI echo cancellation (when register_ai_audio_output is wired)
    if source == AudioSource::SystemLoopback {
        if speaker_tracker.is_ai_echo(samples, &get_echo_reference(app)) {
            return;
        }
    }

    // Quick energy check before spending an API call
    let prepared_preview = preprocess::prepare_for_stt(samples);
    if prepared_preview.len() < 1600 {
        return;
    }

    let speaker_id = if source == AudioSource::Microphone {
        "You".to_string()
    } else {
        speaker_tracker.identify_speaker(samples, 16000)
    };
    state_manager.set_active_speaker(Some(speaker_id.clone()));

    let prev_context = state_manager.get_history(3);

    // STT: audio → text only (never send raw audio to the answer model)
    match transcription_service
        .transcribe(samples, source_label, &speaker_id, &prev_context)
        .await
    {
        Ok((text, confidence)) => {
            if text.is_empty() {
                return;
            }

            // Mic (exclusive My Voice) is a direct voice assistant — allow slightly lower confidence.
            let exclusive_mic = source == AudioSource::Microphone
                && !capture_engine.is_recording(AudioSource::SystemLoopback);
            let min_confidence = if exclusive_mic { 0.40 } else { 0.55 };
            if confidence < min_confidence {
                let _ = app.emit(
                    "audio-clarification-needed",
                    serde_json::json!({
                        "text": text,
                        "speaker": speaker_id,
                        "source": source_label.to_lowercase(),
                    }),
                );
                return;
            }

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let chunk = TranscriptChunk {
                text: text.clone(),
                speaker: speaker_id.clone(),
                timestamp: now,
            };
            state_manager.add_to_history(chunk);

            let _ = crate::modules::database::history::save_message(
                app,
                if source == AudioSource::Microphone {
                    "user"
                } else {
                    "system_participant"
                },
                &text,
                source_label,
            );

            let _ = app.emit(
                "audio-transcription",
                serde_json::json!({
                    "text": text,
                    "speaker": speaker_id.clone(),
                    "source": source_label.to_lowercase(),
                    "status": "final",
                }),
            );

            // Exclusive My Voice: Whisper text → Gemini answer directly (voice assistant).
            // System / Both: extract meeting questions; dual-mode mic stays meeting-filtered.
            let exclusive_mic = source == AudioSource::Microphone
                && !capture_engine.is_recording(AudioSource::SystemLoopback);

            if exclusive_mic {
                if is_mic_filler_only(&text) {
                    println!("[MIC] Skipping filler-only transcript: {}", text);
                    return;
                }

                println!("[MIC] STT → Gemini (text only): {}", text);

                let was_generating = state_manager.is_ai_generating();
                if was_generating {
                    state_manager.set_interrupted(true);
                    let _ = app.emit("ai-interrupted", ());
                }

                let prompt = SubQuestion {
                    text: text.clone(),
                    is_question: true,
                    dependencies: vec![],
                };
                state_manager.enqueue_questions(vec![prompt.clone()]);

                let _ = app.emit(
                    "questions-parsed",
                    serde_json::json!({ "questions": [prompt] }),
                );

                process_next_question(app, state_manager).await;
                return;
            }

            // System audio (and mic while in Both): parse actionable questions from TEXT
            match question_parser
                .parse_questions(&text, source == AudioSource::SystemLoopback)
                .await
            {
                Ok(sub_questions) => {
                    let actionable: Vec<_> = sub_questions
                        .into_iter()
                        .filter(|q| q.is_question && !q.text.trim().is_empty())
                        .collect();

                    if actionable.is_empty() {
                        return;
                    }

                    let was_generating = state_manager.is_ai_generating();
                    if was_generating {
                        state_manager.set_interrupted(true);
                        let _ = app.emit("ai-interrupted", ());
                    }

                    state_manager.enqueue_questions(actionable.clone());

                    let _ = app.emit(
                        "questions-parsed",
                        serde_json::json!({
                            "questions": actionable,
                        }),
                    );

                    process_next_question(app, state_manager).await;
                }
                Err(e) => eprintln!("Failed to parse questions: {}", e),
            }
        }
        Err(e) => eprintln!("Transcription error: {}", e),
    }
}

fn is_mic_filler_only(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    if t.is_empty() {
        return true;
    }
    let fillers = [
        "um", "uh", "uhm", "hmm", "mm", "mhm", "ah", "oh", "okay", "ok", "yeah", "yep", "yup",
        "right", "alright", "huh", "bye",
    ];
    let words: Vec<&str> = t
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    if words.is_empty() {
        return true;
    }
    words.len() <= 2 && words.iter().all(|w| fillers.contains(w))
}

async fn process_next_question(app: &AppHandle, state_manager: &ConversationStateManager) {
    // Skip non-questions in the queue
    while let Some(q) = state_manager.pop_question() {
        if !q.is_question || q.text.trim().is_empty() {
            continue;
        }
        let _ = app.emit(
            "processing-question",
            serde_json::json!({
                "text": q.text.clone(),
                "pending_count": state_manager.get_pending_count(),
            }),
        );

        // TEXT ONLY to the frontend AI stream — never audio
        let _ = app.emit(
            "trigger-ai-response",
            serde_json::json!({
                "prompt": q.text.clone(),
            }),
        );
        break;
    }
}


#[tauri::command]
pub async fn start_audio_capture(app: AppHandle, state: State<'_, AudioState>, mode: String) -> Result<(), String> {
    let mode = mode.to_lowercase();
    match mode.as_str() {
        "both" | "all" | "continuous" => {
            let mic_res = start_one_source(&app, &state, AudioSource::Microphone);
            let sys_res = start_one_source(&app, &state, AudioSource::SystemLoopback);
            match (mic_res, sys_res) {
                (Ok(()), Ok(())) => Ok(()),
                (Ok(()), Err(e)) => { eprintln!("[AUDIO] System failed (mic OK): {}", e); Ok(()) }
                (Err(e), Ok(())) => { eprintln!("[AUDIO] Mic failed (system OK): {}", e); Ok(()) }
                (Err(e1), Err(e2)) => Err(format!("Mic: {}; System: {}", e1, e2)),
            }
        }
        "system" => {
            // Exclusive: stop mic if it was running in dual mode
            if state.capture_engine.is_recording(AudioSource::Microphone) {
                let _ = stop_one_source(&app, &state, AudioSource::Microphone).await;
            }
            start_one_source(&app, &state, AudioSource::SystemLoopback)
        }
        _ => {
            // Exclusive mic: stop system if it was running in dual mode
            if state.capture_engine.is_recording(AudioSource::SystemLoopback) {
                let _ = stop_one_source(&app, &state, AudioSource::SystemLoopback).await;
            }
            start_one_source(&app, &state, AudioSource::Microphone)
        }
    }
}

#[tauri::command]
pub async fn stop_audio_capture(app: AppHandle, state: State<'_, AudioState>) -> Result<String, String> {
    let mic_active = state.capture_engine.is_recording(AudioSource::Microphone);
    let system_active = state.capture_engine.is_recording(AudioSource::SystemLoopback);
    if !mic_active && !system_active { return Ok(String::new()); }
    let mut texts = Vec::new();
    if mic_active {
        let t = stop_one_source(&app, &state, AudioSource::Microphone).await?;
        if !t.is_empty() { texts.push(t); }
    }
    if system_active {
        let t = stop_one_source(&app, &state, AudioSource::SystemLoopback).await?;
        if !t.is_empty() { texts.push(t); }
    }
    Ok(texts.join(" "))
}

#[tauri::command]
pub fn get_audio_capture_state(state: State<'_, AudioState>) -> CaptureStateResponse {
    let mic_active = state.capture_engine.is_recording(AudioSource::Microphone);
    let system_active = state.capture_engine.is_recording(AudioSource::SystemLoopback);
    let mode = match (mic_active, system_active) {
        (true, true) => "both",
        (false, true) => "system",
        _ => "mic",
    };
    CaptureStateResponse { is_recording: mic_active || system_active, mode: mode.to_string() }
}

#[tauri::command]
pub fn register_ai_audio_output(state: State<'_, AudioState>, samples: Vec<f32>, sample_rate: u32) {
    state
        .capture_engine
        .ai_output_monitor
        .register_ai_output(&samples, sample_rate);
}

#[tauri::command]
pub fn set_ai_generating_state(state: State<'_, AudioState>, generating: bool) {
    state.state_manager.set_ai_generating(generating);
    if generating {
        state.state_manager.set_interrupted(false);
    }
}

#[tauri::command]
pub async fn pop_next_pending_question(
    _app: AppHandle,
    state: State<'_, AudioState>,
) -> Result<Option<String>, String> {
    while let Some(q) = state.state_manager.pop_question() {
        if q.is_question && !q.text.trim().is_empty() {
            return Ok(Some(q.text));
        }
    }
    Ok(None)
}
