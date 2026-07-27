use serde::Serialize;
use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Error)]
pub enum AiError {
    #[error("Ollama server is unavailable at {url}: {message}")]
    ServerUnavailable { url: String, message: String },
    #[error("Model '{model}' not found in installed models")]
    ModelNotFound { model: String },
    #[error("Network request failed: {message}")]
    NetworkError { message: String },
    #[error("Request was cancelled by user")]
    RequestCancelled,
    #[error("Ollama returned an empty response")]
    EmptyResponse,
    #[error("Response timeout")]
    Timeout,
    #[error("Ollama error: {message}")]
    OllamaError { message: String },
}
