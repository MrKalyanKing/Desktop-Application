use reqwest::Client;
use crate::modules::ai::errors::AiError;
use crate::modules::ai::models::{ModelsListResponse, GenerateOptions, ModelInfo};
use crate::modules::ai::health::HealthStatus;
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

/// Gemini text API client (chat / reasoning). Speech STT uses gemini_audio multimodal separately.
pub struct GeminiClient {
    client: Client,
}

impl GeminiClient {
    pub fn new(_base_url: Option<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
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
        let models = vec![
            ModelInfo {
                name: "Gemini 3.5 Flash Lite".to_string(),
                model: "gemini-3.5-flash-lite".to_string(),
                size: 0,
                digest: "".to_string(),
            },
            ModelInfo {
                name: "Gemini 2.5 Flash Lite".to_string(),
                model: "gemini-2.5-flash-lite".to_string(),
                size: 0,
                digest: "".to_string(),
            },
            ModelInfo {
                name: "Gemini 2.5 Flash".to_string(),
                model: "gemini-2.5-flash".to_string(),
                size: 0,
                digest: "".to_string(),
            },
            ModelInfo {
                name: "Gemini 3.1 Flash Lite".to_string(),
                model: "gemini-3.1-flash-lite".to_string(),
                size: 0,
                digest: "".to_string(),
            },
        ];

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
        println!("[Gemini] Sending text only ({} chars)", prompt.len());
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

        let manager = crate::modules::ai::model_manager::GeminiModelManager::global();
        let mut attempts = 0;

        loop {
            let active_model = manager.select_model_preferring(Some(preferred_model));
            println!("[AI SYSTEM] Using: {}", active_model);

            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                active_model, api_key
            );

            let request = self.client.post(&url).json(&payload).send();

            let resp = tokio::select! {
                _ = cancellation_token.cancelled() => {
                    return Err(AiError::RequestCancelled);
                }
                resp_res = request => {
                    match resp_res {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("[AI SYSTEM ERROR] Http network request failed: {}", e);
                            return Err(AiError::NetworkError {
                                message: e.to_string(),
                            });
                        }
                    }
                }
            };

            let status_code = resp.status();
            if !status_code.is_success() {
                let err_text = resp.text().await.unwrap_or_default();
                eprintln!(
                    "[AI SYSTEM ERROR] Gemini API returned status {}: {}",
                    status_code, err_text
                );

                let should_switch = manager.is_quota_error(status_code, &err_text)
                    || manager.is_model_unavailable(status_code, &err_text);
                if should_switch {
                    let delay = manager.parse_retry_delay(&err_text);
                    let reason = crate::modules::ai::model_manager::GeminiModelManager::switch_reason(
                        status_code,
                        &err_text,
                    );
                    manager.blacklist_model(&active_model, &err_text, delay);
                    attempts += 1;
                    if attempts < manager.models_len() {
                        let next_model = manager.select_model_preferring(None);
                        manager.log_switch(&active_model, &next_model, &reason);
                        continue;
                    }
                    return Err(AiError::GeminiError {
                        message: format!(
                            "Failed to switch model — all candidates exhausted. Last error on {}: {}",
                            active_model, err_text
                        ),
                    });
                }

                return Err(AiError::GeminiError {
                    message: format!("Gemini API returned status {}: {}", status_code, err_text),
                });
            }

            let gemini_resp = resp.json::<GeminiResponse>().await.map_err(|e| {
                eprintln!(
                    "[AI SYSTEM ERROR] Failed to parse Gemini response payload: {}",
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

            println!(
                "[Gemini] Response received from {} ({} chars)",
                active_model,
                text.len()
            );
            return Ok(text);
        }
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

        let manager = crate::modules::ai::model_manager::GeminiModelManager::global();
        let mut attempts = 0;

        let response = loop {
            let active_model = manager.select_model_preferring(Some(preferred_model));
            println!("[AI SYSTEM STREAM] Using: {}", active_model);

            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}",
                active_model, api_key
            );

            let response_res = self.client.post(&url).json(&payload).send().await;
            let resp = match response_res {
                Ok(r) => r,
                Err(e) => {
                    eprintln!(
                        "[AI SYSTEM STREAM ERROR] Http network stream request failed: {}",
                        e
                    );
                    return Err(AiError::NetworkError {
                        message: e.to_string(),
                    });
                }
            };

            let status_code = resp.status();
            if !status_code.is_success() {
                let err_text = resp.text().await.unwrap_or_default();
                eprintln!(
                    "[AI SYSTEM STREAM ERROR] Gemini stream returned status {}: {}",
                    status_code, err_text
                );

                let should_switch = manager.is_quota_error(status_code, &err_text)
                    || manager.is_model_unavailable(status_code, &err_text);
                if should_switch {
                    let delay = manager.parse_retry_delay(&err_text);
                    let reason = crate::modules::ai::model_manager::GeminiModelManager::switch_reason(
                        status_code,
                        &err_text,
                    );
                    manager.blacklist_model(&active_model, &err_text, delay);
                    attempts += 1;
                    if attempts < manager.models_len() {
                        let next_model = manager.select_model_preferring(None);
                        manager.log_switch(&active_model, &next_model, &reason);
                        continue;
                    }
                    return Err(AiError::GeminiError {
                        message: format!(
                            "Failed to switch model — all candidates exhausted. Last stream error on {}: {}",
                            active_model, err_text
                        ),
                    });
                }

                return Err(AiError::GeminiError {
                    message: format!("Gemini stream returned error: {}", err_text),
                });
            }

            break resp;
        };

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
