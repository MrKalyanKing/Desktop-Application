use std::collections::VecDeque;
use std::sync::Mutex;
use crate::modules::audio::speaker_tracker::SpeakerId;
use crate::modules::cognition::question_parser::SubQuestion;
use crate::modules::transcription::types::TranscriptChunk;

pub struct ConversationStateManager {
    pub transcript_history: Mutex<Vec<TranscriptChunk>>,
    pub pending_queue: Mutex<VecDeque<SubQuestion>>,
    pub active_speaker: Mutex<Option<SpeakerId>>,
    pub was_interrupted: Mutex<bool>,
    pub ai_is_generating: Mutex<bool>,
    pub last_ai_output: Mutex<Option<String>>,
}

#[allow(dead_code)]
impl ConversationStateManager {
    pub fn new() -> Self {
        Self {
            transcript_history: Mutex::new(Vec::new()),
            pending_queue: Mutex::new(VecDeque::new()),
            active_speaker: Mutex::new(None),
            was_interrupted: Mutex::new(false),
            ai_is_generating: Mutex::new(false),
            last_ai_output: Mutex::new(None),
        }
    }

    pub fn reset(&self) {
        self.transcript_history.lock().unwrap().clear();
        self.pending_queue.lock().unwrap().clear();
        *self.active_speaker.lock().unwrap() = None;
        *self.was_interrupted.lock().unwrap() = false;
        *self.ai_is_generating.lock().unwrap() = false;
        *self.last_ai_output.lock().unwrap() = None;
    }

    pub fn add_to_history(&self, chunk: TranscriptChunk) {
        let mut history = self.transcript_history.lock().unwrap();
        history.push(chunk);
        // Limit history to 50 items
        if history.len() > 50 {
            history.remove(0);
        }
    }

    pub fn get_history(&self, limit: usize) -> Vec<TranscriptChunk> {
        let history = self.transcript_history.lock().unwrap();
        let start = if history.len() > limit {
            history.len() - limit
        } else {
            0
        };
        history[start..].to_vec()
    }

    pub fn enqueue_questions(&self, questions: Vec<SubQuestion>) {
        let mut queue = self.pending_queue.lock().unwrap();
        for q in questions {
            queue.push_back(q);
        }
    }

    pub fn pop_question(&self) -> Option<SubQuestion> {
        self.pending_queue.lock().unwrap().pop_front()
    }

    pub fn get_pending_count(&self) -> usize {
        self.pending_queue.lock().unwrap().len()
    }

    pub fn clear_queue(&self) {
        self.pending_queue.lock().unwrap().clear();
    }

    pub fn set_active_speaker(&self, speaker: Option<SpeakerId>) {
        *self.active_speaker.lock().unwrap() = speaker;
    }

    pub fn get_active_speaker(&self) -> Option<SpeakerId> {
        self.active_speaker.lock().unwrap().clone()
    }

    pub fn set_ai_generating(&self, generating: bool) {
        *self.ai_is_generating.lock().unwrap() = generating;
    }

    pub fn is_ai_generating(&self) -> bool {
        *self.ai_is_generating.lock().unwrap()
    }

    pub fn set_interrupted(&self, interrupted: bool) {
        *self.was_interrupted.lock().unwrap() = interrupted;
    }

    pub fn was_interrupted(&self) -> bool {
        *self.was_interrupted.lock().unwrap()
    }

    pub fn set_last_ai_output(&self, text: String) {
        *self.last_ai_output.lock().unwrap() = Some(text);
    }

    pub fn get_last_ai_output(&self) -> Option<String> {
        self.last_ai_output.lock().unwrap().clone()
    }
}
