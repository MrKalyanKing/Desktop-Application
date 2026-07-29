use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use cpal::traits::StreamTrait;
use crate::modules::audio::capture_engine::{CaptureEngine, AudioSource, SendStream};
use crate::modules::audio::speaker_tracker::SpeakerTracker;
use crate::modules::cognition::state_manager::ConversationStateManager;
use crate::modules::audio::segmentation_engine::SegmentationEngine;
use crate::modules::transcription::gemini_service::{GeminiTranscriptionService, TranscriptChunk};
use crate::modules::cognition::question_parser::QuestionParser;
use crate::modules::audio::echo_reference::EchoReferenceBuffer;
use crate::modules::audio::streaming_pipeline;

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

fn start_one_source(app: &AppHandle, state: &AudioState, source: AudioSource) -> Result<(), String> {
    if state.capture_engine.is_recording(source) {
        return Ok(());
    }
    let old_stream = state
        .capture_engine
        .start_capture(source, move |_s, _src| {})
        .map_err(|e| e.to_string())?;
    if let Some(send_stream) = old_stream {
        let _ = app.run_on_main_thread(move || {
            drop(send_stream);
        });
    }

    streaming_pipeline::spawn_streaming_worker(
        app.clone(),
        Arc::clone(&state.capture_engine),
        Arc::clone(&state.state_manager),
        Arc::clone(&state.speaker_tracker),
        Arc::clone(&state.transcription_service),
        Arc::clone(&state.question_parser),
        Arc::clone(&state.inflight_stt),
        source,
    );
    Ok(())
}

async fn stop_one_source(app: &AppHandle, state: &AudioState, source: AudioSource) -> Result<String, String> {
    if !state.capture_engine.is_recording(source) {
        return Ok(String::new());
    }
    let stop_tx = match source {
        AudioSource::Microphone => state.capture_engine.mic_stop_tx.lock().unwrap().take(),
        AudioSource::SystemLoopback => state.capture_engine.system_stop_tx.lock().unwrap().take(),
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
    if samples.len() < 1600 {
        return Ok(String::new());
    }
    let source_label = if source == AudioSource::Microphone {
        "Voice"
    } else {
        "System"
    };
    let speaker_id = if source == AudioSource::Microphone {
        "You".to_string()
    } else {
        state.speaker_tracker.identify_speaker(&samples, 16000)
    };
    let prev_context = state.state_manager.get_history(3);
    match state
        .transcription_service
        .transcribe(&samples, source_label, &speaker_id, &prev_context)
        .await
    {
        Ok((text, _)) => {
            if !text.is_empty() {
                state.state_manager.add_to_history(TranscriptChunk {
                    text: text.clone(),
                    speaker: speaker_id,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                });
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
            }
            Ok(text)
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn start_audio_capture(
    app: AppHandle,
    state: State<'_, AudioState>,
    mode: String,
) -> Result<(), String> {
    let mode = mode.to_lowercase();
    match mode.as_str() {
        "both" | "all" | "continuous" => {
            let mic_res = start_one_source(&app, &state, AudioSource::Microphone);
            let sys_res = start_one_source(&app, &state, AudioSource::SystemLoopback);
            match (mic_res, sys_res) {
                (Ok(()), Ok(())) => Ok(()),
                (Ok(()), Err(e)) => {
                    eprintln!("[AUDIO] System failed (mic OK): {}", e);
                    Ok(())
                }
                (Err(e), Ok(())) => {
                    eprintln!("[AUDIO] Mic failed (system OK): {}", e);
                    Ok(())
                }
                (Err(e1), Err(e2)) => Err(format!("Mic: {}; System: {}", e1, e2)),
            }
        }
        "system" => {
            if state.capture_engine.is_recording(AudioSource::Microphone) {
                let _ = stop_one_source(&app, &state, AudioSource::Microphone).await;
            }
            start_one_source(&app, &state, AudioSource::SystemLoopback)
        }
        _ => {
            if state.capture_engine.is_recording(AudioSource::SystemLoopback) {
                let _ = stop_one_source(&app, &state, AudioSource::SystemLoopback).await;
            }
            start_one_source(&app, &state, AudioSource::Microphone)
        }
    }
}

#[tauri::command]
pub async fn stop_audio_capture(
    app: AppHandle,
    state: State<'_, AudioState>,
) -> Result<String, String> {
    let mic_active = state.capture_engine.is_recording(AudioSource::Microphone);
    let system_active = state.capture_engine.is_recording(AudioSource::SystemLoopback);
    if !mic_active && !system_active {
        return Ok(String::new());
    }
    let mut texts = Vec::new();
    if mic_active {
        let t = stop_one_source(&app, &state, AudioSource::Microphone).await?;
        if !t.is_empty() {
            texts.push(t);
        }
    }
    if system_active {
        let t = stop_one_source(&app, &state, AudioSource::SystemLoopback).await?;
        if !t.is_empty() {
            texts.push(t);
        }
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
    CaptureStateResponse {
        is_recording: mic_active || system_active,
        mode: mode.to_string(),
    }
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
