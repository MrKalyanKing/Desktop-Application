//! Lightweight transcript history types (optional logging — not used for STT).

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TranscriptChunk {
    pub text: String,
    pub speaker: String,
    pub timestamp: u64,
}
