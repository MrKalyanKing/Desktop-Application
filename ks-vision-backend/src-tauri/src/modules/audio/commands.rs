use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{State, Manager, AppHandle, Emitter};
use cpal::traits::StreamTrait;
use crate::modules::audio::capture_engine::{CaptureEngine, AudioSource, SendStream};
use crate::modules::audio::speaker_tracker::{SpeakerTracker, SpeakerId};
use crate::modules::cognition::state_manager::ConversationStateManager;
use crate::modules::audio::segmentation_engine::SegmentationEngine;
use crate::modules::transcription::gemini_service::{GeminiTranscriptionService, TranscriptChunk};
use crate::modules::cognition::question_parser::QuestionParser;
use crate::modules::audio::echo_reference::{EchoReferenceBuffer, AIAudioOutputSink};

pub struct AudioState {
    pub capture_engine: Arc<CaptureEngine>,
    pub speaker_tracker: Arc<SpeakerTracker>,
    pub state_manager: Arc<ConversationStateManager>,
    pub segmentation_engine: Arc<SegmentationEngine>,
    pub transcription_service: Arc<GeminiTranscriptionService>,
    pub question_parser: Arc<QuestionParser>,
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
    let segmentation_engine = Arc::clone(&state.segmentation_engine);
    let transcription_service = Arc::clone(&state.transcription_service);
    let question_parser = Arc::clone(&state.question_parser);
    
    let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
    
    match source {
        AudioSource::Microphone => {
            let mut guard = capture_engine.mic_stop_tx.lock().unwrap();
            *guard = Some(stop_tx);
        }
        AudioSource::SystemLoopback => {
            let mut guard = capture_engine.system_stop_tx.lock().unwrap();
            *guard = Some(stop_tx);
        }
    }
    
    tokio::spawn(async move {
        let buffer = match source {
            AudioSource::Microphone => Arc::clone(&capture_engine.mic_buffer),
            AudioSource::SystemLoopback => Arc::clone(&capture_engine.system_buffer),
        };
        
        let source_label = match source {
            AudioSource::Microphone => "Voice",
            AudioSource::SystemLoopback => "System",
        };
        
        let mut processed_samples_len = 0;
        let mut recent_speech_samples = Vec::new();
        let vad = segmentation_engine.get_vad();
        let mut last_state = vad.get_state();
        let mut interval = tokio::time::interval(Duration::from_millis(30));
        
        loop {
            tokio::select! {
                _ = &mut stop_rx => {
                    break;
                }
                _ = interval.tick() => {
                    let all_samples = buffer.lock().unwrap().get_samples();
                    if all_samples.len() > processed_samples_len {
                        let new_samples = &all_samples[processed_samples_len..];
                        
                        // Emit waveform visualization data from Tokio worker thread to prevent COM crashes
                        let mut sum_sq = 0.0;
                        for &x in new_samples {
                            sum_sq += x * x;
                        }
                        let rms = (sum_sq / new_samples.len().max(1) as f32).sqrt();
                        let pitch = crate::modules::audio::vad_engine::estimate_pitch(new_samples, 16000).unwrap_or(0.0);
                        let _ = app.emit("audio-waveform-data", serde_json::json!({
                            "volume": rms,
                            "pitch": pitch,
                            "source": if source == AudioSource::Microphone { "mic" } else { "system" }
                        }));

                        let frame_size = 480; // 30ms at 16kHz
                        
                        let mut reset_occurred = false;
                        for chunk in new_samples.chunks(frame_size) {
                            if chunk.len() == frame_size {
                                let current_vad_state = vad.get_state();
                                if current_vad_state == crate::modules::audio::vad_engine::VadState::Speech {
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
                                    let _ = app.emit("audio-state-changed", serde_json::json!({ "state": state_str }));
                                    last_state = current_state;
                                }
                                
                                if boundary_triggered {
                                    let utterance_samples = &all_samples[0..processed_samples_len + chunk.len()];
                                    
                                    process_utterance(
                                        &app,
                                        &state_manager,
                                        &speaker_tracker,
                                        &transcription_service,
                                        &question_parser,
                                        utterance_samples,
                                        source,
                                        source_label,
                                    ).await;
                                    
                                    buffer.lock().unwrap().clear();
                                    processed_samples_len = 0;
                                    recent_speech_samples.clear();
                                    reset_occurred = true;
                                    break;
                                }
                            }
                        }
                        
                        if !reset_occurred {
                            if processed_samples_len > 0 || !new_samples.is_empty() {
                                processed_samples_len = all_samples.len();
                            }
                        }
                    }
                }
            }
        }
    });
}

async fn process_utterance(
    app: &AppHandle,
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
    
    // 1. AI Echo Cancellation
    if source == AudioSource::SystemLoopback {
        if speaker_tracker.is_ai_echo(samples, &get_echo_reference(app)) {
            return;
        }
    }
    
    // 2. Speaker Diarization
    let speaker_id = speaker_tracker.identify_speaker(samples, 16000);
    state_manager.set_active_speaker(Some(speaker_id.clone()));
    
    // 3. Get history context
    let prev_context = state_manager.get_history(3);
    
    // 4. Gemini Transcription
    match transcription_service.transcribe(samples, source_label, &speaker_id, &prev_context).await {
        Ok((text, confidence)) => {
            if text.is_empty() {
                return;
            }
            
            // Low confidence check
            if confidence < 0.7 {
                let _ = app.emit("audio-clarification-needed", serde_json::json!({
                    "text": text,
                    "speaker": speaker_id,
                    "source": source_label.to_lowercase(),
                }));
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
                if source == AudioSource::Microphone { "user" } else { "system_participant" },
                &text,
                source_label,
            );
            
            let _ = app.emit("audio-transcription", serde_json::json!({
                "text": text,
                "speaker": speaker_id.clone(),
                "source": source_label.to_lowercase(),
                "status": "final",
            }));
            
            // 5. Question extraction (Only for user mic)
            if source == AudioSource::Microphone {
                match question_parser.parse_questions(&text).await {
                    Ok(sub_questions) => {
                        let was_generating = state_manager.is_ai_generating();
                        if was_generating {
                            state_manager.set_interrupted(true);
                            let _ = app.emit("ai-interrupted", ());
                        }
                        
                        state_manager.enqueue_questions(sub_questions.clone());
                        
                        let _ = app.emit("questions-parsed", serde_json::json!({
                            "questions": sub_questions,
                        }));
                        
                        process_next_question(app, state_manager).await;
                    }
                    Err(e) => eprintln!("Failed to parse questions: {}", e),
                }
            }
        }
        Err(e) => eprintln!("Transcription error: {}", e),
    }
}

async fn process_next_question(app: &AppHandle, state_manager: &ConversationStateManager) {
    if let Some(q) = state_manager.pop_question() {
        let _ = app.emit("processing-question", serde_json::json!({
            "text": q.text.clone(),
            "pending_count": state_manager.get_pending_count(),
        }));
        
        let _ = app.emit("trigger-ai-response", serde_json::json!({
            "prompt": q.text.clone(),
        }));
    }
}

#[tauri::command]
pub async fn start_audio_capture(app: AppHandle, state: State<'_, AudioState>, mode: String) -> Result<(), String> {
    let source = if mode == "system" {
        AudioSource::SystemLoopback
    } else {
        AudioSource::Microphone
    };

    let state_clone = Arc::new(AudioState {
        capture_engine: Arc::clone(&state.capture_engine),
        speaker_tracker: Arc::clone(&state.speaker_tracker),
        state_manager: Arc::clone(&state.state_manager),
        segmentation_engine: Arc::clone(&state.segmentation_engine),
        transcription_service: Arc::clone(&state.transcription_service),
        question_parser: Arc::clone(&state.question_parser),
    });

    let old_stream = state.capture_engine.start_capture(source, move |_samples, _src| {
        // Empty callback to prevent COM / thread-marshaling crashes on Windows audio callback threads
    }).map_err(|e| e.to_string())?;

    if let Some(send_stream) = old_stream {
        let _ = app.run_on_main_thread(move || {
            drop(send_stream);
        });
    }

    spawn_background_worker(app, state_clone, source);

    Ok(())
}

#[tauri::command]
pub async fn stop_audio_capture(
    app: AppHandle,
    state: State<'_, AudioState>,
) -> Result<String, String> {
    let mic_active = state.capture_engine.is_recording(AudioSource::Microphone);
    let system_active = state.capture_engine.is_recording(AudioSource::SystemLoopback);
    
    let source = if mic_active {
        AudioSource::Microphone
    } else if system_active {
        AudioSource::SystemLoopback
    } else {
        return Ok("".to_string());
    };

    let stop_tx = match source {
        AudioSource::Microphone => {
            let mut guard = state.capture_engine.mic_stop_tx.lock().unwrap();
            guard.take()
        }
        AudioSource::SystemLoopback => {
            let mut guard = state.capture_engine.system_stop_tx.lock().unwrap();
            guard.take()
        }
    };

    if let Some(tx) = stop_tx {
        let _ = tx.send(());
    }

    let (samples, stream_opt) = state.capture_engine.stop_capture(source).await;
    
    if let Some(send_stream) = stream_opt {
        let SendStream(ref stream) = send_stream;
        let _ = stream.pause();
        let _ = app.run_on_main_thread(move || {
            drop(send_stream);
        });
    }
    
    if samples.is_empty() {
        return Ok("".to_string());
    }

    let prev_context = state.state_manager.get_history(3);
    let source_label = if source == AudioSource::Microphone { "Voice" } else { "System" };
    let speaker_id = state.speaker_tracker.identify_speaker(&samples, 16000);
    
    match state.transcription_service.transcribe(&samples, source_label, &speaker_id, &prev_context).await {
        Ok((text, _)) => {
            if !text.is_empty() {
                let chunk = TranscriptChunk {
                    text: text.clone(),
                    speaker: speaker_id,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };
                state.state_manager.add_to_history(chunk);
                
                let _ = crate::modules::database::history::save_message(
                    &app,
                    if source == AudioSource::Microphone { "user" } else { "system_participant" },
                    &text,
                    source_label,
                );
            }
            Ok(text)
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub fn get_audio_capture_state(state: State<'_, AudioState>) -> CaptureStateResponse {
    let mic_active = state.capture_engine.is_recording(AudioSource::Microphone);
    let system_active = state.capture_engine.is_recording(AudioSource::SystemLoopback);
    
    CaptureStateResponse {
        is_recording: mic_active || system_active,
        mode: if system_active { "system".to_string() } else { "mic".to_string() },
    }
}

#[tauri::command]
pub fn register_ai_audio_output(state: State<'_, AudioState>, samples: Vec<f32>, sample_rate: u32) {
    state.capture_engine.ai_output_monitor.register_ai_output(&samples, sample_rate);
}

#[tauri::command]
pub fn set_ai_generating_state(state: State<'_, AudioState>, generating: bool) {
    state.state_manager.set_ai_generating(generating);
    if generating {
        state.state_manager.set_interrupted(false);
    }
}

#[tauri::command]
pub async fn pop_next_pending_question(app: AppHandle, state: State<'_, AudioState>) -> Result<Option<String>, String> {
    if let Some(q) = state.state_manager.pop_question() {
        if q.is_question {
            Ok(Some(q.text))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}
