//! Shared Gemini model priority + request-scoped fallback.
//! One primary model, automatic switch on quota / unavailable / transient errors.

use std::collections::{HashMap, HashSet};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, SystemTime};

use serde::Deserialize;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BlacklistItem {
    pub blocked_at: SystemTime,
    pub available_at: SystemTime,
    pub reason: String,
}

pub struct GeminiModelManager {
    /// Priority order (highest first).
    models: Vec<String>,
    /// Voice/audio multimodal priority (models that handle audio reliably first).
    voice_models: Vec<String>,
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

/// Soft ceiling so one bad RetryInfo cannot block a model for hours mid-session.
const MAX_BLACKLIST_SECS: u64 = 300;
const HARD_UNAVAILABLE_SECS: u64 = 3600;
const DEFAULT_QUOTA_SECS: u64 = 60;

impl GeminiModelManager {
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<GeminiModelManager> = OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            // Text / chat priority — fast lite first, then stronger flash.
            models: vec![
                "gemini-3.5-flash-lite".to_string(),
                "gemini-2.5-flash-lite".to_string(),
                "gemini-2.5-flash".to_string(),
                "gemini-3.1-flash-lite".to_string(),
            ],
            // Voice multimodal — prefer flash (audio-capable) before lite.
            voice_models: vec![
                "gemini-2.5-flash".to_string(),
                "gemini-3.5-flash-lite".to_string(),
                "gemini-2.5-flash-lite".to_string(),
                "gemini-3.1-flash-lite".to_string(),
            ],
            blacklist: RwLock::new(HashMap::new()),
        })
    }

    pub fn known_models(&self) -> &[String] {
        &self.models
    }

    pub fn models_len(&self) -> usize {
        self.models.len()
    }

    pub fn voice_models_len(&self) -> usize {
        self.voice_models.len()
    }

    fn purge_expired(&self) {
        let now = SystemTime::now();
        if let Ok(mut write_guard) = self.blacklist.write() {
            let mut restored = Vec::new();
            write_guard.retain(|model, item| {
                if item.available_at <= now {
                    restored.push(model.clone());
                    false
                } else {
                    true
                }
            });
            for _model in restored {
                // quiet — cooldown restore is not an API event
            }
        }
    }

    pub fn select_model(&self) -> String {
        self.next_model(None, &HashSet::new(), false)
            .unwrap_or_else(|| "gemini-3.5-flash-lite".to_string())
    }

    /// Prefer UI selection when valid & available; else first non-blacklisted in priority order.
    pub fn select_model_preferring(&self, preferred: Option<&str>) -> String {
        self.next_model(preferred, &HashSet::new(), false)
            .unwrap_or_else(|| "gemini-3.5-flash-lite".to_string())
    }

    /// Next unused model for this request. `voice=true` uses audio-optimized priority.
    /// Returns `None` when every candidate in the list was already tried.
    pub fn next_model(
        &self,
        preferred: Option<&str>,
        tried: &HashSet<String>,
        voice: bool,
    ) -> Option<String> {
        self.purge_expired();
        let list = if voice {
            &self.voice_models
        } else {
            &self.models
        };

        let blacklisted = self.blacklist.read().ok()?;

        if let Some(pref) = preferred {
            let pref = pref.trim();
            if !pref.is_empty()
                && !tried.contains(pref)
                && !blacklisted.contains_key(pref)
                && list.iter().any(|m| m == pref)
            {
                return Some(pref.to_string());
            }
        }

        for model in list {
            if tried.contains(model) {
                continue;
            }
            if !blacklisted.contains_key(model) {
                return Some(model.clone());
            }
        }

        // All remaining candidates blacklisted — try any untried anyway (last resort).
        for model in list {
            if !tried.contains(model) {
                return Some(model.clone());
            }
        }

        None
    }

    pub fn log_switch(&self, from: &str, to: &str, reason: &str) {
        println!("[API FALLBACK] {} → {} ({})", from, to, reason);
    }

    pub fn blacklist_model(&self, model: &str, reason: &str, retry_after: Option<Duration>) {
        let now = SystemTime::now();
        let lower = reason.to_lowercase();
        let is_hard = lower.contains("not_found")
            || lower.contains("not found")
            || lower.contains("is not found")
            || lower.contains("unknown model")
            || lower.contains("invalid model")
            || reason.contains("404");

        let mut secs = retry_after
            .map(|d| d.as_secs().max(1))
            .unwrap_or(if is_hard {
                HARD_UNAVAILABLE_SECS
            } else {
                DEFAULT_QUOTA_SECS
            });

        if !is_hard {
            secs = secs.min(MAX_BLACKLIST_SECS);
        }

        let duration = Duration::from_secs(secs);
        let available_at = now + duration;

        if let Ok(mut write_guard) = self.blacklist.write() {
            write_guard.insert(
                model.to_string(),
                BlacklistItem {
                    blocked_at: now,
                    available_at,
                    reason: reason.to_string(),
                },
            );
        }

        println!(
            "[API FALLBACK] blacklisted '{}' for {}s",
            model,
            duration.as_secs()
        );
    }

    /// True when caller should blacklist + try the next priority model.
    pub fn should_fallback(&self, status: reqwest::StatusCode, body: &str) -> bool {
        if self.is_quota_error(status, body) || self.is_model_unavailable(status, body) {
            return true;
        }
        // Transient server / capacity failures
        matches!(
            status.as_u16(),
            408 | 429 | 500 | 502 | 503 | 504
        ) || {
            let lower = body.to_lowercase();
            lower.contains("unavailable")
                || lower.contains("internal")
                || lower.contains("overloaded")
                || lower.contains("deadline exceeded")
                || lower.contains("temporarily")
        }
    }

    pub fn is_quota_error(&self, status: reqwest::StatusCode, body: &str) -> bool {
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return true;
        }
        let lower = body.to_lowercase();
        lower.contains("resource_exhausted")
            || lower.contains("quota exceeded")
            || lower.contains("rate limit exceeded")
            || lower.contains("ratelimit")
    }

    pub fn is_model_unavailable(&self, status: reqwest::StatusCode, body: &str) -> bool {
        if status == reqwest::StatusCode::NOT_FOUND {
            return true;
        }
        let lower = body.to_lowercase();
        lower.contains("not_found")
            || lower.contains("is not found")
            || lower.contains("not supported")
            || lower.contains("unsupported")
            || lower.contains("invalid model")
            || lower.contains("unknown model")
            || lower.contains("does not support")
    }

    pub fn switch_reason(status: reqwest::StatusCode, body: &str) -> String {
        let lower = body.to_lowercase();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS
            || lower.contains("resource_exhausted")
            || lower.contains("quota")
            || lower.contains("rate limit")
        {
            return format!("quota/rate-limit (HTTP {})", status.as_u16());
        }
        if status == reqwest::StatusCode::NOT_FOUND
            || lower.contains("not_found")
            || lower.contains("unknown model")
        {
            return format!("model unavailable / NOT_FOUND (HTTP {})", status.as_u16());
        }
        if matches!(status.as_u16(), 500 | 502 | 503 | 504)
            || lower.contains("overloaded")
            || lower.contains("unavailable")
        {
            return format!("transient server error (HTTP {})", status.as_u16());
        }
        format!(
            "API error HTTP {} — trying next priority model",
            status.as_u16()
        )
    }

    pub fn parse_retry_delay(&self, body: &str) -> Option<Duration> {
        let err_resp: GeminiErrorResponse = serde_json::from_str(body).ok()?;
        let details = err_resp.error?.details?;
        for val in details {
            if val.get("@type").and_then(|v| v.as_str())
                == Some("type.googleapis.com/google.rpc.RetryInfo")
            {
                if let Some(delay_val) = val.get("retryDelay") {
                    if let Some(delay_str) = delay_val.as_str() {
                        let stripped = delay_str.trim_end_matches('s');
                        if let Ok(secs) = stripped.parse::<f64>() {
                            return Some(Duration::from_secs_f64(secs.max(1.0)));
                        }
                    } else if let Some(delay_num) = delay_val.as_f64() {
                        return Some(Duration::from_secs_f64(delay_num.max(1.0)));
                    }
                }
            }
        }
        None
    }
}
