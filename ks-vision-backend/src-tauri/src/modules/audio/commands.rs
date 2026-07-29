use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use cpal::traits::StreamTrait;
use crate::modules::audio::capture_engine::{CaptureEngine, AudioSource, SendStream};
use crate::modules::audio::speaker_tracker::SpeakerTracker;
use crate::modules::cognition::state_manager::ConversationStateManager;
use crate::modules::audio::segmentation_engine::SegmentationEngine;
use crate::modules::audio::echo_reference::EchoReferenceBuffer;
use crate::modules::audio::streaming_pipeline;

pub struct AudioState {
    pub capture_engine: Arc<CaptureEngine>,
    pub speaker_tracker: Arc<SpeakerTracker>,
    pub state_manager: Arc<ConversationStateManager>,
    pub segmentation_engine: Arc<SegmentationEngine>,
    pub inflight_voice: Arc<AtomicUsize>,
}

impl AudioState {
    pub fn new() -> Self {
        Self {
            capture_engine: Arc::new(CaptureEngine::new()),
            speaker_tracker: Arc::new(SpeakerTracker::new()),
            state_manager: Arc::new(ConversationStateManager::new()),
            segmentation_engine: Arc::new(SegmentationEngine::new()),
            inflight_voice: Arc::new(AtomicUsize::new(0)),
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
        Arc::clone(&state.inflight_voice),
        source,
    );
    Ok(())
}

async fn stop_one_source(app: &AppHandle, state: &AudioState, source: AudioSource) -> Result<(), String> {
    if !state.capture_engine.is_recording(source) {
        return Ok(());
    }
    let stop_tx = match source {
        AudioSource::Microphone => state.capture_engine.mic_stop_tx.lock().unwrap().take(),
        AudioSource::SystemLoopback => state.capture_engine.system_stop_tx.lock().unwrap().take(),
    };
    if let Some(tx) = stop_tx {
        let _ = tx.send(());
    }
    let (_samples, stream_opt) = state.capture_engine.stop_capture(source).await;
    if let Some(send_stream) = stream_opt {
        let SendStream(ref stream) = send_stream;
        let _ = stream.pause();
        let _ = app.run_on_main_thread(move || {
            drop(send_stream);
        });
    }
    // No STT on stop — voice answers are produced live via Gemini multimodal.
    Ok(())
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
    if mic_active {
        stop_one_source(&app, &state, AudioSource::Microphone).await?;
    }
    if system_active {
        stop_one_source(&app, &state, AudioSource::SystemLoopback).await?;
    }
    Ok(String::new())
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
