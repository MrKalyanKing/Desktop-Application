//! Capability-based Gemini model selection with runtime health scoring.
//! Route: Capability → Healthy → Lowest latency → Lowest cost → Select.

use std::collections::{HashMap, HashSet};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, SystemTime};

use serde::Deserialize;

/// What the request needs. Selection never sends audio to a text-only model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelCapability {
    Text,
    Audio,
    Image,
    Multimodal,
}

impl ModelCapability {
    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Audio => "Audio",
            Self::Image => "Image",
            Self::Multimodal => "Multimodal",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Caps {
    text: bool,
    audio: bool,
    image: bool,
}

impl Caps {
    const fn text_only() -> Self {
        Self {
            text: true,
            audio: false,
            image: false,
        }
    }

    const fn multimodal() -> Self {
        Self {
            text: true,
            audio: true,
            image: true,
        }
    }

    fn supports(self, need: ModelCapability) -> bool {
        match need {
            ModelCapability::Text => self.text,
            ModelCapability::Audio => self.audio,
            ModelCapability::Image => self.image,
            ModelCapability::Multimodal => self.audio && self.image && self.text,
        }
    }

    fn demote(mut self, failed: ModelCapability) -> Self {
        match failed {
            ModelCapability::Text => self.text = false,
            ModelCapability::Audio => self.audio = false,
            ModelCapability::Image => self.image = false,
            ModelCapability::Multimodal => {
                self.audio = false;
                self.image = false;
            }
        }
        self
    }
}

#[derive(Debug, Clone)]
struct ModelEntry {
    id: &'static str,
    caps: Caps,
    /// Catalog cost hint (lower = cheaper). Tie-breaker / cold-start prior.
    cost_rank: u8,
    /// Assumed latency (ms) before live samples exist.
    cold_latency_ms: u32,
}

#[derive(Debug, Clone, Default)]
struct ModelStats {
    successes: u64,
    failures: u64,
    latency_sum_ms: u64,
    last_failure: Option<SystemTime>,
}

impl ModelStats {
    fn samples(&self) -> u64 {
        self.successes + self.failures
    }

    fn success_rate(&self) -> f64 {
        let n = self.samples();
        if n == 0 {
            1.0
        } else {
            self.successes as f64 / n as f64
        }
    }

    fn avg_latency_ms(&self) -> Option<f64> {
        if self.successes == 0 {
            None
        } else {
            Some(self.latency_sum_ms as f64 / self.successes as f64)
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelHealthSnapshot {
    pub model: String,
    pub success_rate: f64,
    pub avg_latency_ms: Option<f64>,
    pub last_failure_ago_secs: Option<u64>,
    pub samples: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BlacklistItem {
    pub blocked_at: SystemTime,
    pub available_at: SystemTime,
    pub reason: String,
}

pub struct GeminiModelManager {
    catalog: Vec<ModelEntry>,
    runtime_caps: RwLock<HashMap<String, Caps>>,
    blacklist: RwLock<HashMap<String, BlacklistItem>>,
    stats: RwLock<HashMap<String, ModelStats>>,
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

const PRIMARY: &str = "gemini-3.5-flash-lite";
const MAX_BLACKLIST_SECS: u64 = 300;
const HARD_UNAVAILABLE_SECS: u64 = 3600;
const CAPABILITY_DEMOTE_SECS: u64 = 1800;
const DEFAULT_QUOTA_SECS: u64 = 60;
const COST_WEIGHT_MS: f64 = 40.0;
const FAILURE_PENALTY_MS: f64 = 2_500.0;

impl GeminiModelManager {
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<GeminiModelManager> = OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            // cost_rank + cold_latency are priors only.
            // Live latency/success stats re-rank every request.
            catalog: vec![
                ModelEntry {
                    id: "gemini-3.5-flash-lite",
                    caps: Caps::multimodal(),
                    cost_rank: 0,
                    cold_latency_ms: 850,
                },
                ModelEntry {
                    id: "gemini-2.5-flash-lite",
                    caps: Caps::multimodal(),
                    cost_rank: 1,
                    cold_latency_ms: 1_100,
                },
                ModelEntry {
                    id: "gemini-2.5-flash",
                    caps: Caps::multimodal(),
                    cost_rank: 2,
                    cold_latency_ms: 1_400,
                },
                ModelEntry {
                    id: "gemini-3.1-flash-lite",
                    caps: Caps::multimodal(),
                    cost_rank: 3,
                    cold_latency_ms: 1_000,
                },
                ModelEntry {
                    id: "gemini-2.0-flash-lite",
                    caps: Caps::text_only(),
                    cost_rank: 10,
                    cold_latency_ms: 900,
                },
            ],
            runtime_caps: RwLock::new(HashMap::new()),
            blacklist: RwLock::new(HashMap::new()),
            stats: RwLock::new(HashMap::new()),
        })
    }

    pub fn primary_model(&self) -> &'static str {
        PRIMARY
    }

    pub fn known_models(&self) -> Vec<String> {
        self.catalog
            .iter()
            .filter(|e| e.caps.text)
            .map(|e| e.id.to_string())
            .collect()
    }

    pub fn models_len(&self) -> usize {
        self.catalog.len()
    }

    pub fn capable_count(&self, capability: ModelCapability) -> usize {
        self.catalog
            .iter()
            .filter(|e| self.effective_caps(e).supports(capability))
            .count()
    }

    fn effective_caps(&self, entry: &ModelEntry) -> Caps {
        if let Ok(guard) = self.runtime_caps.read() {
            if let Some(over) = guard.get(entry.id) {
                return *over;
            }
        }
        entry.caps
    }

    fn purge_expired(&self) {
        let now = SystemTime::now();
        let mut recovered_primary = false;
        if let Ok(mut write_guard) = self.blacklist.write() {
            write_guard.retain(|model, item| {
                if item.available_at <= now {
                    if model == PRIMARY {
                        recovered_primary = true;
                    }
                    false
                } else {
                    true
                }
            });
        }
        if recovered_primary {
            println!("[MODEL]");
            println!("Recovered");
            println!("Primary model available again");
        }
    }

    /// Lower score wins: latency + failure penalty + cost.
    fn score_entry(
        &self,
        entry: &ModelEntry,
        preferred: Option<&str>,
        stats_guard: &HashMap<String, ModelStats>,
    ) -> f64 {
        let st = stats_guard.get(entry.id);
        let samples = st.map(|s| s.samples()).unwrap_or(0);
        let success_rate = st.map(|s| s.success_rate()).unwrap_or(1.0);
        let latency = st
            .and_then(|s| s.avg_latency_ms())
            .unwrap_or(entry.cold_latency_ms as f64);
        let failure_penalty = (1.0 - success_rate) * FAILURE_PENALTY_MS;
        let cost = entry.cost_rank as f64 * COST_WEIGHT_MS;
        let prefer_bonus = if preferred == Some(entry.id) && samples < 5 {
            -30.0
        } else {
            0.0
        };
        latency + failure_penalty + cost + prefer_bonus
    }

    /// Capability → healthy → lowest latency → lowest cost → select.
    pub fn select_for(
        &self,
        capability: ModelCapability,
        preferred: Option<&str>,
        tried: &HashSet<String>,
    ) -> Option<String> {
        self.purge_expired();
        let blacklisted = self.blacklist.read().ok()?;
        let stats_guard = self.stats.read().ok()?;
        let pref = preferred.map(str::trim).filter(|s| !s.is_empty());

        let capable_untried = |entry: &ModelEntry| -> bool {
            !tried.contains(entry.id) && self.effective_caps(entry).supports(capability)
        };

        let healthy: Vec<&ModelEntry> = self
            .catalog
            .iter()
            .filter(|e| capable_untried(e) && !blacklisted.contains_key(e.id))
            .collect();

        let pool: Vec<&ModelEntry> = if !healthy.is_empty() {
            healthy
        } else {
            self.catalog
                .iter()
                .filter(|e| capable_untried(e))
                .collect()
        };

        if pool.is_empty() {
            return None;
        }

        let mut best: Option<(&ModelEntry, f64)> = None;
        for entry in pool {
            let score = self.score_entry(entry, pref, &stats_guard);
            match best {
                None => best = Some((entry, score)),
                Some((_, best_score)) if score < best_score => best = Some((entry, score)),
                _ => {}
            }
        }
        best.map(|(e, _)| e.id.to_string())
    }

    pub fn next_model(
        &self,
        preferred: Option<&str>,
        tried: &HashSet<String>,
        voice: bool,
    ) -> Option<String> {
        let cap = if voice {
            ModelCapability::Audio
        } else {
            ModelCapability::Text
        };
        self.select_for(cap, preferred, tried)
    }

    pub fn select_model(&self) -> String {
        self.select_for(ModelCapability::Text, None, &HashSet::new())
            .unwrap_or_else(|| PRIMARY.to_string())
    }

    pub fn select_model_preferring(&self, preferred: Option<&str>) -> String {
        self.select_for(ModelCapability::Text, preferred, &HashSet::new())
            .unwrap_or_else(|| PRIMARY.to_string())
    }

    pub fn record_success(&self, model: &str, latency_ms: u64) {
        if let Ok(mut guard) = self.stats.write() {
            let st = guard.entry(model.to_string()).or_default();
            st.successes = st.successes.saturating_add(1);
            st.latency_sum_ms = st.latency_sum_ms.saturating_add(latency_ms);
        }
    }

    pub fn record_failure_stat(&self, model: &str) {
        if let Ok(mut guard) = self.stats.write() {
            let st = guard.entry(model.to_string()).or_default();
            st.failures = st.failures.saturating_add(1);
            st.last_failure = Some(SystemTime::now());
        }
    }

    pub fn health_snapshots(&self) -> Vec<ModelHealthSnapshot> {
        let stats = self.stats.read().ok();
        let now = SystemTime::now();
        self.catalog
            .iter()
            .map(|e| {
                let st = stats.as_ref().and_then(|g| g.get(e.id));
                let last_failure_ago_secs = st.and_then(|s| {
                    s.last_failure
                        .and_then(|t| now.duration_since(t).ok())
                        .map(|d| d.as_secs())
                });
                ModelHealthSnapshot {
                    model: e.id.to_string(),
                    success_rate: st.map(|s| s.success_rate()).unwrap_or(1.0),
                    avg_latency_ms: st.and_then(|s| s.avg_latency_ms()),
                    last_failure_ago_secs,
                    samples: st.map(|s| s.samples()).unwrap_or(0),
                }
            })
            .collect()
    }

    pub fn log_selected(&self, model: &str, capability: ModelCapability) {
        let (rate, latency) = self
            .stats
            .read()
            .ok()
            .and_then(|g| g.get(model).cloned())
            .map(|s| (s.success_rate(), s.avg_latency_ms()))
            .unwrap_or((1.0, None));

        println!("[MODEL]");
        println!("Selected: {}", model);
        println!("Capability: {}", capability.label());
        if let Some(ms) = latency {
            println!("Avg Latency: {:.0} ms", ms);
            println!("Success Rate: {:.1}%", rate * 100.0);
        }
    }

    pub fn log_fallback(&self, from: &str, to: &str, reason: &str) {
        println!("[MODEL]");
        println!("Fallback");
        println!("Reason: {}", reason);
        println!("From: {}", from);
        println!("To: {}", to);
    }

    pub fn log_switch(&self, from: &str, to: &str, reason: &str) {
        self.log_fallback(from, to, reason);
    }

    pub fn blacklist_model(&self, model: &str, reason: &str, retry_after: Option<Duration>) {
        let now = SystemTime::now();
        let lower = reason.to_lowercase();
        let is_capability = Self::is_capability_error_body(&lower)
            || Self::is_modality_unsupported_body(&lower);
        let is_hard = lower.contains("not_found")
            || lower.contains("not found")
            || lower.contains("is not found")
            || lower.contains("unknown model")
            || lower.contains("invalid model")
            || reason.contains("404")
            || is_capability;

        let mut secs = retry_after
            .map(|d| d.as_secs().max(1))
            .unwrap_or(if is_capability {
                CAPABILITY_DEMOTE_SECS
            } else if is_hard {
                HARD_UNAVAILABLE_SECS
            } else {
                DEFAULT_QUOTA_SECS
            });

        if !is_hard && !is_capability {
            secs = secs.min(MAX_BLACKLIST_SECS);
        }

        let duration = Duration::from_secs(secs);
        if let Ok(mut write_guard) = self.blacklist.write() {
            write_guard.insert(
                model.to_string(),
                BlacklistItem {
                    blocked_at: now,
                    available_at: now + duration,
                    reason: reason.to_string(),
                },
            );
        }
        self.record_failure_stat(model);
    }

    pub fn demote_capability(&self, model: &str, capability: ModelCapability) {
        let entry = self.catalog.iter().find(|e| e.id == model);
        let base = entry
            .map(|e| self.effective_caps(e))
            .unwrap_or_else(Caps::text_only);
        let next = base.demote(capability);
        if let Ok(mut guard) = self.runtime_caps.write() {
            guard.insert(model.to_string(), next);
        }
    }

    pub fn register_failure(
        &self,
        model: &str,
        capability: ModelCapability,
        status: reqwest::StatusCode,
        body: &str,
    ) -> String {
        let reason = Self::switch_reason(status, body);
        let delay = self.parse_retry_delay(body);

        if Self::is_capability_error(status, body) || Self::is_modality_unsupported(status, body)
        {
            self.demote_capability(model, capability);
            if matches!(capability, ModelCapability::Audio | ModelCapability::Image) {
                self.demote_capability(model, ModelCapability::Multimodal);
            }
            self.blacklist_model(
                model,
                body,
                delay.or(Some(Duration::from_secs(CAPABILITY_DEMOTE_SECS))),
            );
            return if matches!(capability, ModelCapability::Audio) {
                "Audio not supported".to_string()
            } else if matches!(capability, ModelCapability::Image) {
                "Image not supported".to_string()
            } else {
                reason
            };
        }

        self.blacklist_model(model, body, delay);
        reason
    }

    pub fn should_fallback(&self, status: reqwest::StatusCode, body: &str) -> bool {
        if self.is_quota_error(status, body)
            || self.is_model_unavailable(status, body)
            || Self::is_capability_error(status, body)
            || Self::is_modality_unsupported(status, body)
        {
            return true;
        }
        matches!(status.as_u16(), 408 | 429 | 500 | 502 | 503 | 504) || {
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
            || lower.contains("invalid model")
            || lower.contains("unknown model")
    }

    pub fn is_capability_error(status: reqwest::StatusCode, body: &str) -> bool {
        let lower = body.to_lowercase();
        Self::is_capability_error_body(&lower)
            || (status.as_u16() == 400 && Self::is_modality_unsupported_body(&lower))
    }

    pub fn is_modality_unsupported(status: reqwest::StatusCode, body: &str) -> bool {
        let lower = body.to_lowercase();
        Self::is_modality_unsupported_body(&lower)
            || (status.as_u16() == 400 && lower.contains("invalid_argument"))
    }

    fn is_capability_error_body(lower: &str) -> bool {
        lower.contains("invalid_argument")
            || lower.contains("not supported")
            || lower.contains("unsupported")
            || lower.contains("does not support")
            || lower.contains("capability")
            || lower.contains("modality")
    }

    fn is_modality_unsupported_body(lower: &str) -> bool {
        (lower.contains("audio")
            && (lower.contains("support")
                || lower.contains("invalid")
                || lower.contains("unable")))
            || lower.contains("inline_data")
            || lower.contains("inlinedata")
            || lower.contains("mime_type")
            || lower.contains("mimetype")
            || lower.contains("media type")
            || lower.contains("unable to process input")
            || (lower.contains("image") && lower.contains("support"))
    }

    pub fn switch_reason(status: reqwest::StatusCode, body: &str) -> String {
        let lower = body.to_lowercase();
        if Self::is_modality_unsupported_body(&lower) || Self::is_capability_error_body(&lower) {
            if lower.contains("audio") {
                return "Audio not supported".into();
            }
            if lower.contains("image") {
                return "Image not supported".into();
            }
            return format!("capability error (HTTP {})", status.as_u16());
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS
            || lower.contains("resource_exhausted")
            || lower.contains("quota")
            || lower.contains("rate limit")
        {
            return format!("quota/rate-limit (HTTP {})", status.as_u16());
        }
        if status == reqwest::StatusCode::NOT_FOUND || lower.contains("not_found") {
            return format!("model unavailable (HTTP {})", status.as_u16());
        }
        if matches!(status.as_u16(), 500 | 502 | 503 | 504)
            || lower.contains("overloaded")
            || lower.contains("unavailable")
        {
            return format!("transient server error (HTTP {})", status.as_u16());
        }
        format!("API error HTTP {}", status.as_u16())
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
