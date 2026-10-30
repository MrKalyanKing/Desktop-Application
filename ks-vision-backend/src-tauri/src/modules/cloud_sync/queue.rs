use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Manager};

static EVENT_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    pub enabled: bool,
    pub api_base_url: String,
    pub access_token: String,
    pub refresh_token: String,
    pub device_id: String,
    pub active_session_id: String,
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_base_url: "http://localhost:3001/api/v1".into(),
            access_token: String::new(),
            refresh_token: String::new(),
            device_id: String::new(),
            active_session_id: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingEvent {
    pub event_id: String,
    pub session_id: String,
    pub speaker_type: String,
    pub text: String,
    pub source: String,
    pub occurred_at: String,
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("cloud_sync.json"))
}

fn queue_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir: {e}"))?;
    Ok(dir.join("cloud_sync_queue.json"))
}

pub fn load_config(app: &AppHandle) -> CloudConfig {
    let Ok(path) = config_path(app) else {
        return CloudConfig::default();
    };
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_config(app: &AppHandle, config: &CloudConfig) -> Result<(), String> {
    let path = config_path(app)?;
    std::fs::write(path, serde_json::to_string_pretty(config).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

fn load_queue(app: &AppHandle) -> Vec<PendingEvent> {
    let Ok(path) = queue_path(app) else {
        return Vec::new();
    };
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_queue(app: &AppHandle, queue: &[PendingEvent]) -> Result<(), String> {
    let path = queue_path(app)?;
    std::fs::write(path, serde_json::to_string_pretty(queue).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

pub fn enqueue_voice_event(app: &AppHandle, speaker_type: &str, text: &str, source: &str) {
    let config = load_config(app);
    if !config.enabled || config.access_token.is_empty() || config.active_session_id.is_empty() {
        return;
    }
    let mut queue = load_queue(app);
    queue.push(PendingEvent {
        event_id: format!(
            "evt-{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            EVENT_SEQ.fetch_add(1, Ordering::Relaxed)
        ),
        session_id: config.active_session_id.clone(),
        speaker_type: speaker_type.to_string(),
        text: text.to_string(),
        source: source.to_string(),
        occurred_at: chrono_now(),
    });
    let _ = save_queue(app, &queue);
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = flush(&handle).await;
    });
}

pub async fn flush(app: &AppHandle) -> Result<usize, String> {
    let config = load_config(app);
    if !config.enabled || config.access_token.is_empty() {
        return Ok(0);
    }
    let queue = load_queue(app);
    if queue.is_empty() {
        return Ok(0);
    }

    let mut remaining = Vec::new();
    let mut sent = 0usize;
    let client = reqwest::Client::new();

    for event in queue {
        let url = format!(
            "{}/desktop/sync/{}",
            config.api_base_url.trim_end_matches('/'),
            event.session_id
        );
        let body = serde_json::json!({
            "segments": [{
                "eventId": event.event_id,
                "speakerType": event.speaker_type,
                "text": event.text,
                "source": event.source
            }],
            "events": [{
                "eventId": event.event_id,
                "eventType": "transcript_segment",
                "occurredAt": event.occurred_at,
                "payload": { "source": event.source }
            }]
        });
        match client
            .post(url)
            .bearer_auth(&config.access_token)
            .json(&body)
            .send()
            .await
        {
            Ok(res) if res.status().is_success() => sent += 1,
            _ => remaining.push(event),
        }
    }

    save_queue(app, &remaining)?;
    Ok(sent)
}

fn chrono_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

pub fn pending_count(app: &AppHandle) -> usize {
    load_queue(app).len()
}
