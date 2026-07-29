//! Stage latency helpers — silent by default (voice path logs only API request/response).

use std::time::Instant;

#[inline]
pub fn now() -> Instant {
    Instant::now()
}

/// No-op: processing logs were flooding the console every frame.
#[inline]
pub fn log_stage(_stage: &str, _source: &str, _started: Instant) {}

#[inline]
pub fn log_stage_ms(_stage: &str, _source: &str, _ms: f64) {}
