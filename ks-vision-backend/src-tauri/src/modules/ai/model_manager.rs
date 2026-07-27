use std::sync::{RwLock, OnceLock};
use std::time::{SystemTime, Duration};
use std::collections::HashMap;
use serde::Deserialize;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BlacklistItem {
    pub blocked_at: SystemTime,
    pub available_at: SystemTime,
    pub reason: String,
}

pub struct GeminiModelManager {
    models: Vec<String>,
    blacklist: RwLock<HashMap<String, BlacklistItem>>,
}

#[derive(Deserialize)]
struct GeminiErrorResponse {
    error: Option<GeminiErrorDetails>,
}

#[derive(Deserialize)]
struct GeminiErrorDetails {
    #[allow(dead_code)]
    code: Option<u16>,
    #[allow(dead_code)]
    message: Option<String>,
    #[allow(dead_code)]
    status: Option<String>,
    details: Option<Vec<serde_json::Value>>,
}

impl GeminiModelManager {
    /// Retrieve the global thread-safe instance of the model manager
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<GeminiModelManager> = OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            models: vec![
                "gemini-3.5-flash-lite".to_string(),
                "gemini-2.5-flash-lite".to_string(),
                "gemini-2.5-flash".to_string(),
            ],
            blacklist: RwLock::new(HashMap::new()),
        })
    }

    /// Selects the highest priority model that is currently available.
    /// Performs automatic recovery for any blacklisted models whose timer has expired.
    pub fn select_model(&self) -> String {
        let now = SystemTime::now();

        // 1. Recover expired blacklisted models
        {
            if let Ok(mut write_guard) = self.blacklist.write() {
                let mut restored = Vec::new();
                write_guard.retain(|model, item| {
                    if item.available_at <= now {
                        restored.push(model.clone());
                        false // remove from blacklist
                    } else {
                        true // keep
                    }
                });
                for model in restored {
                    println!("[MODEL MANAGER] Model restored: {}", model);
                }
            }
        }

        // 2. Return the first model in the priority list that is not blacklisted
        if let Ok(read_guard) = self.blacklist.read() {
            for model in &self.models {
                if !read_guard.contains_key(model) {
                    return model.clone();
                }
            }
        }

        // Fallback: If all models are currently blacklisted, use the first one (highest priority)
        self.models.first().cloned().unwrap_or_else(|| "gemini-3.5-flash-lite".to_string())
    }

    /// Blacklists a model due to a quota error, optionally specifying a retry delay
    pub fn blacklist_model(&self, model: &str, reason: &str, retry_after: Option<Duration>) {
        let now = SystemTime::now();
        let duration = retry_after.unwrap_or(Duration::from_secs(60));
        let available_at = now + duration;

        if let Ok(mut write_guard) = self.blacklist.write() {
            write_guard.insert(model.to_string(), BlacklistItem {
                blocked_at: now,
                available_at,
                reason: reason.to_string(),
            });
        }

        println!("[MODEL MANAGER] 429 received for: {}", model);
        println!("[MODEL MANAGER] Blacklisting model: {} (Retry available in {} seconds)", model, duration.as_secs());
    }

    /// Returns the length of the configured models list
    pub fn models_len(&self) -> usize {
        self.models.len()
    }

    /// Helper to verify if an HTTP error is quota/rate-limit related
    pub fn is_quota_error(&self, status: reqwest::StatusCode, body: &str) -> bool {
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return true;
        }
        let lower = body.to_lowercase();
        lower.contains("resource_exhausted")
            || lower.contains("quota exceeded")
            || lower.contains("rate limit exceeded")
    }

    /// Parses the retry-after duration from Gemini's standard error response
    pub fn parse_retry_delay(&self, body: &str) -> Option<Duration> {
        let err_resp: GeminiErrorResponse = serde_json::from_str(body).ok()?;
        let details = err_resp.error?.details?;
        for val in details {
            if val.get("@type").and_then(|v| v.as_str()) == Some("type.googleapis.com/google.rpc.RetryInfo") {
                if let Some(delay_val) = val.get("retryDelay") {
                    if let Some(delay_str) = delay_val.as_str() {
                        // String format: "52s" or "52.123s"
                        let stripped = delay_str.trim_end_matches('s');
                        if let Ok(secs) = stripped.parse::<f64>() {
                            return Some(Duration::from_secs_f64(secs));
                        }
                    } else if let Some(delay_num) = delay_val.as_f64() {
                        return Some(Duration::from_secs_f64(delay_num));
                    }
                }
            }
        }
        None
    }
}
