use std::collections::HashSet;

use reqwest::Client;
use crate::modules::ai::errors::AiError;
use crate::modules::ai::models::{ModelsListResponse, GenerateOptions, ModelInfo};
use crate::modules::ai::health::HealthStatus;
use crate::modules::ai::model_manager::{GeminiModelManager, ModelCapability};
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;
use tauri::ipc::Channel;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct GeminiPayload {
    contents: Vec<GeminiContentPayload>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstructionPayload>,
}

#[derive(Serialize)]
struct GeminiContentPayload {
    parts: Vec<GeminiPartPayload>,
}

#[derive(Serialize)]
struct GeminiSystemInstructionPayload {
    parts: Vec<GeminiPartPayload>,
}

#[derive(Serialize)]
struct GeminiPartPayload {
    text: String,
}

#[derive(Deserialize, Clone)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize, Clone)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
}

#[derive(Deserialize, Clone)]
struct GeminiContent {
    parts: Option<Vec<GeminiPart>>,
}

#[derive(Deserialize, Clone)]
struct GeminiPart {
    text: Option<String>,
}

fn load_env_file() {
    let paths = vec![".env", "../.env", "src-tauri/.env", "../src-tauri/.env"];
    for path in paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some(pos) = line.find('=') {
                    let key = line[..pos].trim();
                    let value = line[pos + 1..].trim();
                    let value = value.trim_matches('"').trim_matches('\'');
                    std::env::set_var(key, value);
                }
            }
            break;
        }
    }
}

/// Gemini text API client (typed chat / reasoning).
/// Voice path sends audio directly via gemini_voice (no STT).
pub struct GeminiClient {
    client: Client,
}

impl GeminiClient {
    pub fn new(_base_url: Option<String>) -> Self {
        Self {
            client: Client::builder()
                // Text generate can take longer than a health ping.
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    pub async fn check_health(&self) -> HealthStatus {
        load_env_file();
        let api_key = std::env::var("GEMINI_API_KEY")
            .or_else(|_| std::env::var("VITE_GEMINI_API_KEY"))
            .unwrap_or_default();

        if api_key.is_empty() || api_key == "YOUR_GEMINI_API_KEY_HERE" {
            HealthStatus {
                available: false,
                url: "https://generativelanguage.googleapis.com".to_string(),
                message: "Gemini API key is missing. Please add GEMINI_API_KEY to your .env file."
                    .to_string(),
            }
        } else {
            HealthStatus {
                available: true,
                url: "https://generativelanguage.googleapis.com".to_string(),
                message: "Gemini API connection configured and active".to_string(),
            }
        }
    }

    pub async fn list_models(&self) -> Result<ModelsListResponse, AiError> {
        let manager = GeminiModelManager::global();
        let models = manager
            .known_models()
            .into_iter()
            // Keep UI on primary cost/speed tier (exclude text-only safety net).
            .filter(|id| id != "gemini-2.0-flash-lite")
            .map(|id| {
                let name = id
                    .replace("gemini-", "Gemini ")
                    .replace('-', " ")
                    .split_whitespace()
                    .map(|w| {
                        let mut c = w.chars();
                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                ModelInfo {
                    name,
                    model: id,
                    size: 0,
                    digest: String::new(),
                }
            })
            .collect();

        Ok(ModelsListResponse { models })
    }

    pub async fn generate(
        &self,
        preferred_model: &str,
        prompt: &str,
        system: Option<String>,
        _options: Option<GenerateOptions>,
        cancellation_token: CancellationToken,
    ) -> Result<String, AiError> {
        println!("[Gemini] Sending text ({} chars)", prompt.len());
        load_env_file();
        let api_key = std::env::var("GEMINI_API_KEY")
            .or_else(|_| std::env::var("VITE_GEMINI_API_KEY"))
            .unwrap_or_default();

        if api_key.is_empty() || api_key == "YOUR_GEMINI_API_KEY_HERE" {
            eprintln!("[AI SYSTEM ERROR] Gemini API Key is missing.");
            return Err(AiError::GeminiError {
                message: "Gemini API Key is not set. Add GEMINI_API_KEY to your .env file."
                    .to_string(),
            });
        }

        let system_instruction = system.map(|sys| GeminiSystemInstructionPayload {
            parts: vec![GeminiPartPayload { text: sys }],
        });

        let payload = GeminiPayload {
            contents: vec![GeminiContentPayload {
                parts: vec![GeminiPartPayload {
                    text: prompt.to_string(),
                }],
            }],
            system_instruction,
        };

        let manager = GeminiModelManager::global();
        let capability = ModelCapability::Text;
        let mut tried: HashSet<String> = HashSet::new();
        let mut last_err = String::from("No Gemini model available");
        let mut logged_selection = false;

        while let Some(active_model) =
            manager.select_for(capability, Some(preferred_model), &tried)
        {
            tried.insert(active_model.clone());
            if !logged_selection {
                manager.log_selected(&active_model, capability);
                logged_selection = true;
            }

            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                active_model, api_key
            );

            let request_started = std::time::Instant::now();
            let request = self.client.post(&url).json(&payload).send();

            let resp = tokio::select! {
                _ = cancellation_token.cancelled() => {
                    return Err(AiError::RequestCancelled);
                }
                resp_res = request => {
                    match resp_res {
                        Ok(r) => r,
                        Err(e) => {
                            last_err = format!("Http network request failed on {}: {}", active_model, e);
                            eprintln!("[API ERROR] {}", last_err);
                            manager.blacklist_model(
                                &active_model,
                                &last_err,
                                Some(std::time::Duration::from_secs(15)),
                            );
                            if let Some(next) = manager.select_for(capability, None, &tried) {
                                manager.log_fallback(&active_model, &next, "network error");
                                continue;
                            }
                            return Err(AiError::NetworkError {
                                message: last_err.clone(),
                            });
                        }
                    }
                }
            };

            let status_code = resp.status();
            if !status_code.is_success() {
                let err_text = resp.text().await.unwrap_or_default();
                last_err = format!(
                    "Gemini API returned status {} on {}: {}",
                    status_code, active_model, err_text
                );
                eprintln!("[API ERROR] {}", last_err);

                if manager.should_fallback(status_code, &err_text) {
                    let reason =
                        manager.register_failure(&active_model, capability, status_code, &err_text);
                    if let Some(next_model) = manager.select_for(capability, None, &tried) {
                        manager.log_fallback(&active_model, &next_model, &reason);
                        continue;
                    }
                    return Err(AiError::GeminiError {
                        message: format!(
                            "All Gemini models exhausted. Last error on {}: {}",
                            active_model, err_text
                        ),
                    });
                }

                return Err(AiError::GeminiError {
                    message: last_err,
                });
            }

            let gemini_resp = resp.json::<GeminiResponse>().await.map_err(|e| {
                eprintln!(
                    "[API ERROR] Failed to parse Gemini response payload: {}",
                    e
                );
                AiError::NetworkError {
                    message: format!("Failed to parse Gemini response: {}", e),
                }
            })?;

            let text = gemini_resp
                .candidates
                .and_then(|c| c.first().cloned())
                .and_then(|c| c.content)
                .and_then(|c| c.parts)
                .and_then(|p| p.first().cloned())
                .and_then(|p| p.text)
                .ok_or_else(|| AiError::EmptyResponse)?;

            manager.record_success(
                &active_model,
                request_started.elapsed().as_millis() as u64,
            );
            return Ok(text);
        }

        Err(AiError::GeminiError {
            message: format!("All Gemini models exhausted. Last error: {}", last_err),
        })
    }

    pub async fn generate_stream(
        &self,
        preferred_model: &str,
        prompt: &str,
        system: Option<String>,
        _options: Option<GenerateOptions>,
        channel: Channel<String>,
        cancellation_token: CancellationToken,
    ) -> Result<(), AiError> {
        println!("[Gemini] Streaming text only ({} chars)", prompt.len());
        load_env_file();
        let api_key = std::env::var("GEMINI_API_KEY")
            .or_else(|_| std::env::var("VITE_GEMINI_API_KEY"))
            .unwrap_or_default();

        if api_key.is_empty() || api_key == "YOUR_GEMINI_API_KEY_HERE" {
            eprintln!("[AI SYSTEM STREAM ERROR] Gemini API Key is missing.");
            return Err(AiError::GeminiError {
                message: "Gemini API Key is not set. Add GEMINI_API_KEY to your .env file."
                    .to_string(),
            });
        }

        let system_instruction = system.map(|sys| GeminiSystemInstructionPayload {
            parts: vec![GeminiPartPayload { text: sys }],
        });

        let payload = GeminiPayload {
            contents: vec![GeminiContentPayload {
                parts: vec![GeminiPartPayload {
                    text: prompt.to_string(),
                }],
            }],
            system_instruction,
        };

        let manager = GeminiModelManager::global();
        let capability = ModelCapability::Text;
        let mut tried: HashSet<String> = HashSet::new();
        let mut last_err = String::from("No Gemini model available");
        let mut logged_selection = false;
        let mut selected_model = String::new();
        let mut stream_started = std::time::Instant::now();

        let response = loop {
            let Some(active_model) =
                manager.select_for(capability, Some(preferred_model), &tried)
            else {
                return Err(AiError::GeminiError {
                    message: format!(
                        "All Gemini models exhausted. Last stream error: {}",
                        last_err
                    ),
                });
            };
            tried.insert(active_model.clone());
            if !logged_selection {
                manager.log_selected(&active_model, capability);
                logged_selection = true;
            }

            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}",
                active_model, api_key
            );

            stream_started = std::time::Instant::now();
            let response_res = self.client.post(&url).json(&payload).send().await;
            let resp = match response_res {
                Ok(r) => r,
                Err(e) => {
                    last_err = format!("Http network stream failed on {}: {}", active_model, e);
                    eprintln!("[API ERROR] {}", last_err);
                    manager.blacklist_model(
                        &active_model,
                        &last_err,
                        Some(std::time::Duration::from_secs(15)),
                    );
                    if let Some(next) = manager.select_for(capability, None, &tried) {
                        manager.log_fallback(&active_model, &next, "network error");
                        continue;
                    }
                    return Err(AiError::NetworkError {
                        message: last_err.clone(),
                    });
                }
            };

            let status_code = resp.status();
            if !status_code.is_success() {
                let err_text = resp.text().await.unwrap_or_default();
                last_err = format!(
                    "Gemini stream status {} on {}: {}",
                    status_code, active_model, err_text
                );
                eprintln!("[API ERROR] {}", last_err);

                if manager.should_fallback(status_code, &err_text) {
                    let reason =
                        manager.register_failure(&active_model, capability, status_code, &err_text);
                    if let Some(next_model) = manager.select_for(capability, None, &tried) {
                        manager.log_fallback(&active_model, &next_model, &reason);
                        continue;
                    }
                    return Err(AiError::GeminiError {
                        message: format!(
                            "All Gemini models exhausted. Last stream error on {}: {}",
                            active_model, err_text
                        ),
                    });
                }

                return Err(AiError::GeminiError {
                    message: format!("Gemini stream returned error: {}", err_text),
                });
            }

            selected_model = active_model;
            break resp;
        };

        manager.record_success(
            &selected_model,
            stream_started.elapsed().as_millis() as u64,
        );

        let mut stream = response.bytes_stream();

        while let Some(chunk_res) = stream.next().await {
            if cancellation_token.is_cancelled() {
                return Err(AiError::RequestCancelled);
            }

            let chunk = chunk_res.map_err(|e| AiError::NetworkError {
                message: format!("Error reading stream chunk: {}", e),
            })?;

            let chunk_str = String::from_utf8_lossy(&chunk);
            let mut clean_chunk = chunk_str.trim();

            if clean_chunk.starts_with('[') {
                clean_chunk = clean_chunk[1..].trim();
            }
            if clean_chunk.ends_with(']') {
                clean_chunk = clean_chunk[..clean_chunk.len() - 1].trim();
            }
            if clean_chunk.starts_with(',') {
                clean_chunk = clean_chunk[1..].trim();
            }
            if clean_chunk.ends_with(',') {
                clean_chunk = clean_chunk[..clean_chunk.len() - 1].trim();
            }

            if !clean_chunk.is_empty() && clean_chunk != "[" && clean_chunk != "]" {
                if let Ok(gemini_resp) = serde_json::from_str::<GeminiResponse>(clean_chunk) {
                    if let Some(candidates) = gemini_resp.candidates {
                        if let Some(first) = candidates.first() {
                            if let Some(content) = &first.content {
                                if let Some(parts) = &content.parts {
                                    if let Some(part) = parts.first() {
                                        if let Some(text) = &part.text {
                                            if !text.is_empty() {
                                                let _ = channel.send(text.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    let mut search_str = clean_chunk;
                    while let Some(pos) = search_str.find("\"text\":") {
                        let start = pos + 7;
                        if let Some(sub) = search_str.get(start..) {
                            let sub = sub.trim();
                            if sub.starts_with('"') {
                                if let Some(end) = sub[1..].find('"') {
                                    let text_val = &sub[1..end + 1];
                                    if !text_val.is_empty() {
                                        let clean_text = text_val
                                            .replace("\\n", "\n")
                                            .replace("\\\"", "\"")
                                            .replace("\\t", "\t")
                                            .replace("\\\\", "\\");
                                        let _ = channel.send(clean_text);
                                    }
                                    search_str = &sub[end + 2..];
                                    continue;
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}
