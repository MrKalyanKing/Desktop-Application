//! Finalize refined transcript text and dispatch to Gemini reasoning (text-only answers).

use tauri::{AppHandle, Emitter};
use crate::modules::audio::capture_engine::{AudioSource, CaptureEngine};
use crate::modules::audio::perf_metrics;
use crate::modules::audio::speaker_tracker::SpeakerTracker;
use crate::modules::cognition::question_parser::{QuestionParser, SubQuestion};
use crate::modules::cognition::state_manager::ConversationStateManager;
use crate::modules::transcription::gemini_service::TranscriptChunk;
use crate::modules::transcription::intent_detector;

pub async fn enqueue_and_trigger_text(
    app: &AppHandle,
    state_manager: &ConversationStateManager,
    text: &str,
) {
    let text = text.trim();
    if text.is_empty() || intent_detector::is_trivial_utterance(text) {
        return;
    }

    let was_generating = state_manager.is_ai_generating();
    if was_generating {
        state_manager.set_interrupted(true);
        let _ = app.emit("ai-interrupted", ());
    }

    let prompt = SubQuestion {
        text: text.to_string(),
        is_question: true,
        dependencies: vec![],
    };
    state_manager.enqueue_questions(vec![prompt.clone()]);
    let _ = app.emit(
        "questions-parsed",
        serde_json::json!({ "questions": [prompt] }),
    );
    process_next_question(app, state_manager).await;
}

pub async fn process_next_question(app: &AppHandle, state_manager: &ConversationStateManager) {
    while let Some(q) = state_manager.pop_question() {
        if !q.is_question || q.text.trim().is_empty() {
            continue;
        }
        let t0 = perf_metrics::now();
        let _ = app.emit(
            "processing-question",
            serde_json::json!({
                "text": q.text.clone(),
                "pending_count": state_manager.get_pending_count(),
            }),
        );
        let _ = app.emit(
            "trigger-ai-response",
            serde_json::json!({
                "prompt": q.text.clone(),
            }),
        );
        perf_metrics::log_stage("gemini_request_dispatch", "text", t0);
        break;
    }
}

/// Finalize a refined transcript. Answer path is text-only Gemini.
pub async fn finalize_transcript(
    app: &AppHandle,
    capture_engine: &CaptureEngine,
    state_manager: &ConversationStateManager,
    speaker_tracker: &SpeakerTracker,
    question_parser: &QuestionParser,
    text: &str,
    confidence: f32,
    source: AudioSource,
    source_label: &str,
    samples_for_speaker: &[f32],
) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }

    if source == AudioSource::SystemLoopback && state_manager.is_ai_generating() {
        return;
    }

    let exclusive_mic = source == AudioSource::Microphone
        && !capture_engine.is_recording(AudioSource::SystemLoopback);
    let min_confidence = if exclusive_mic { 0.35 } else { 0.50 };
    if confidence < min_confidence && confidence > 0.0 {
        let _ = app.emit(
            "audio-clarification-needed",
            serde_json::json!({
                "text": text,
                "source": source_label.to_lowercase(),
            }),
        );
        return;
    }

    let speaker_id = if source == AudioSource::Microphone {
        "You".to_string()
    } else if !samples_for_speaker.is_empty() {
        speaker_tracker.identify_speaker(samples_for_speaker, 16000)
    } else {
        "Speaker".to_string()
    };
    state_manager.set_active_speaker(Some(speaker_id.clone()));

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    state_manager.add_to_history(TranscriptChunk {
        text: text.to_string(),
        speaker: speaker_id.clone(),
        timestamp: now,
    });

    let _ = crate::modules::database::history::save_message(
        app,
        if source == AudioSource::Microphone {
            "user"
        } else {
            "system_participant"
        },
        text,
        source_label,
    );

    let _ = app.emit(
        "audio-transcription",
        serde_json::json!({
            "text": text,
            "speaker": speaker_id,
            "source": source_label.to_lowercase(),
            "status": "final",
        }),
    );

    let t_q = perf_metrics::now();
    if exclusive_mic {
        if intent_detector::is_trivial_utterance(text) {
            return;
        }
        println!("[MIC] STT → Gemini (text only): {}", text);
        enqueue_and_trigger_text(app, state_manager, text).await;
        perf_metrics::log_stage("question_detection", "mic", t_q);
        return;
    }

    // Fast local intent first — avoid Gemini parse RTT when obvious
    if intent_detector::looks_like_complete_question(text) {
        println!("[INTENT] Local question detect: {}", text);
        enqueue_and_trigger_text(app, state_manager, text).await;
        perf_metrics::log_stage("question_detection", "local", t_q);
        return;
    }

    match question_parser
        .parse_questions(text, source == AudioSource::SystemLoopback)
        .await
    {
        Ok(sub_questions) => {
            let actionable: Vec<_> = sub_questions
                .into_iter()
                .filter(|q| q.is_question && !q.text.trim().is_empty())
                .collect();
            if actionable.is_empty() {
                perf_metrics::log_stage("question_detection", "none", t_q);
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
                serde_json::json!({ "questions": actionable }),
            );
            process_next_question(app, state_manager).await;
            perf_metrics::log_stage("question_detection", "gemini_parse", t_q);
        }
        Err(e) => eprintln!("Failed to parse questions: {}", e),
    }
}
