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
    /// Priority order (highest first) — keep Google-doc models as implemented.
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
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<GeminiModelManager> = OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            // Strategic priority (same set as product / Google doc implementation)
            models: vec![
                "gemini-3.5-flash-lite".to_string(),
                "gemini-2.5-flash-lite".to_string(),
                "gemini-2.5-flash".to_string(),
                "gemini-3.1-flash-lite".to_string(),
            ],
            blacklist: RwLock::new(HashMap::new()),
        })
    }

    pub fn known_models(&self) -> &[String] {
        &self.models
    }

    pub fn select_model(&self) -> String {
        self.select_model_preferring(None)
    }

    /// Prefer UI selection when valid & available; else first non-blacklisted in priority order.
    pub fn select_model_preferring(&self, preferred: Option<&str>) -> String {
        let now = SystemTime::now();

        {
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
                for model in restored {
                    println!("[MODEL MANAGER] ✅ Restored (cooldown expired): {}", model);
                }
            }
        }

        if let Ok(read_guard) = self.blacklist.read() {
            if let Some(pref) = preferred {
                let pref = pref.trim();
                if !pref.is_empty() {
                    if self.models.iter().any(|m| m == pref) && !read_guard.contains_key(pref) {
                        println!("[MODEL MANAGER] Selected preferred model: {}", pref);
                        return pref.to_string();
                    }
                    if read_guard.contains_key(pref) {
                        println!(
                            "[MODEL MANAGER] Preferred '{}' is blacklisted — picking next available",
                            pref
                        );
                    } else if !self.models.iter().any(|m| m == pref) {
                        println!(
                            "[MODEL MANAGER] Preferred '{}' not in priority list — picking next available",
                            pref
                        );
                    }
                }
            }

            for model in &self.models {
                if !read_guard.contains_key(model) {
                    return model.clone();
                }
            }
        }

        // All blacklisted — still return highest priority so caller can retry/report
        let fallback = self
            .models
            .first()
            .cloned()
            .unwrap_or_else(|| "gemini-3.5-flash-lite".to_string());
        println!(
            "[MODEL MANAGER] ⚠️ All models blacklisted — falling back to: {}",
            fallback
        );
        fallback
    }

    /// Log a strategic switch: FROM → TO with reason.
    pub fn log_switch(&self, from: &str, to: &str, reason: &str) {
        println!("[MODEL MANAGER] ========================================");
        println!("[MODEL MANAGER] 🔄 SWITCHING MODEL");
        println!("[MODEL MANAGER]    FROM : {}", from);
        println!("[MODEL MANAGER]    TO   : {}", to);
        println!("[MODEL MANAGER]    WHY  : {}", reason);
        println!("[MODEL MANAGER] ========================================");
    }

    pub fn blacklist_model(&self, model: &str, reason: &str, retry_after: Option<Duration>) {
        let now = SystemTime::now();
        let is_hard = reason.to_lowercase().contains("not_found")
            || reason.to_lowercase().contains("not found")
            || reason.to_lowercase().contains("is not found")
            || reason.contains("404");
        let duration = retry_after.unwrap_or_else(|| {
            if is_hard {
                Duration::from_secs(3600)
            } else {
                Duration::from_secs(60)
            }
        });
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

        let short_reason = if reason.len() > 160 {
            format!("{}…", &reason[..160])
        } else {
            reason.to_string()
        };
        println!(
            "[MODEL MANAGER] ⛔ Blacklisted '{}' for {}s — {}",
            model,
            duration.as_secs(),
            short_reason
        );
    }

    pub fn models_len(&self) -> usize {
        self.models.len()
    }

    pub fn is_quota_error(&self, status: reqwest::StatusCode, body: &str) -> bool {
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return true;
        }
        let lower = body.to_lowercase();
        lower.contains("resource_exhausted")
            || lower.contains("quota exceeded")
            || lower.contains("rate limit exceeded")
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
    }

    pub fn switch_reason(status: reqwest::StatusCode, body: &str) -> String {
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS
            || body.to_lowercase().contains("resource_exhausted")
            || body.to_lowercase().contains("quota")
        {
            return format!("quota/rate-limit (HTTP {})", status.as_u16());
        }
        if status == reqwest::StatusCode::NOT_FOUND || body.to_lowercase().contains("not_found") {
            return format!("model unavailable / NOT_FOUND (HTTP {})", status.as_u16());
        }
        format!("API error HTTP {} — trying next priority model", status.as_u16())
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
