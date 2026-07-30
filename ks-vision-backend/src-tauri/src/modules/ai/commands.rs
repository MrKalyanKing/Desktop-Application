use std::sync::Mutex;
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;
use tauri::State;
use crate::modules::ai::health::HealthStatus;
use crate::modules::ai::models::{ModelsListResponse, GenerateOptions};
use crate::modules::ai::errors::AiError;
use crate::modules::ai::client::GeminiClient;

pub struct AiState {
    pub active_generations: Mutex<HashMap<String, CancellationToken>>,
}

impl AiState {
    pub fn new() -> Self {
        Self {
            active_generations: Mutex::new(HashMap::new()),
        }
    }
}

#[tauri::command]
pub async fn ai_health_check(app: tauri::AppHandle, base_url: Option<String>) -> HealthStatus {
    let client = GeminiClient::new(app, base_url);
    client.check_health().await
}

#[tauri::command]
pub async fn ai_get_models(app: tauri::AppHandle, base_url: Option<String>) -> Result<ModelsListResponse, AiError> {
    let client = GeminiClient::new(app, base_url);
    client.list_models().await
}

#[tauri::command]
pub async fn ask_ai(
    app: tauri::AppHandle,
    request_id: String,
    model: String,
    prompt: String,
    system: Option<String>,
    options: Option<GenerateOptions>,
    base_url: Option<String>,
    state: State<'_, AiState>,
) -> Result<String, AiError> {
    let token = CancellationToken::new();

    {
        let mut active = state.active_generations.lock().unwrap();
        active.insert(request_id.clone(), token.clone());
    }

    let client = GeminiClient::new(app, base_url);
    let result = client.generate(&model, &prompt, system, options, token).await;

    {
        let mut active = state.active_generations.lock().unwrap();
        active.remove(&request_id);
    }

    result
}

#[tauri::command]
pub async fn stream_ai(
    app: tauri::AppHandle,
    request_id: String,
    model: String,
    prompt: String,
    system: Option<String>,
    options: Option<GenerateOptions>,
    channel: tauri::ipc::Channel<String>,
    base_url: Option<String>,
    state: State<'_, AiState>,
) -> Result<(), AiError> {
    let token = CancellationToken::new();

    {
        let mut active = state.active_generations.lock().unwrap();
        active.insert(request_id.clone(), token.clone());
    }

    let client = GeminiClient::new(app, base_url);
    let result = client
        .generate_stream(&model, &prompt, system, options, channel, token)
        .await;

    {
        let mut active = state.active_generations.lock().unwrap();
        active.remove(&request_id);
    }

    result
}

#[tauri::command]
pub fn cancel_ai(request_id: String, state: State<'_, AiState>) -> Result<(), String> {
    let active = state.active_generations.lock().unwrap();
    if let Some(token) = active.get(&request_id) {
        token.cancel();
        Ok(())
    } else {
        Err("Request ID not active or already finished".to_string())
    }
}

#[tauri::command]
pub fn db_save_message_cmd(app: tauri::AppHandle, role: String, content: String, source: String) -> Result<(), String> {
    crate::modules::database::history::save_message(&app, &role, &content, &source)
}

#[tauri::command]
pub fn db_load_history_cmd(app: tauri::AppHandle, limit: u32) -> Result<Vec<crate::modules::database::history::MessageRecord>, String> {
    crate::modules::database::history::load_history(&app, limit)
}

#[tauri::command]
pub fn db_clear_history_cmd(app: tauri::AppHandle) -> Result<(), String> {
    crate::modules::database::history::clear_history(&app)
}

#[tauri::command]
pub fn db_delete_message_cmd(app: tauri::AppHandle, id: i64) -> Result<(), String> {
    crate::modules::database::history::delete_message(&app, id)
}

#[tauri::command]
pub fn db_search_history_cmd(app: tauri::AppHandle, query: String) -> Result<Vec<crate::modules::database::history::MessageRecord>, String> {
    crate::modules::database::history::search_history(&app, &query)
}

#[tauri::command]
pub fn db_export_markdown_cmd(app: tauri::AppHandle) -> Result<String, String> {
    crate::modules::database::history::export_markdown(&app)
}

#[tauri::command]
pub fn db_export_json_cmd(app: tauri::AppHandle) -> Result<String, String> {
    crate::modules::database::history::export_json(&app)
}

