//! Optional NestJS cloud sync for the existing desktop assistant.
//! Capture, Gemini, and local SQLite keep working if this module is disabled or the API is down.

pub mod commands;
mod queue;

pub use queue::enqueue_voice_event;
