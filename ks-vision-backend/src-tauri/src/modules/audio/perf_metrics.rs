//! Stage latency metrics for the streaming speech pipeline.
use std::time::Instant;

#[inline]
pub fn now() -> Instant {
    Instant::now()
}

pub fn log_stage(stage: &str, source: &str, started: Instant) {
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    // Avoid flooding logs on the 20ms audio tick; always keep STT/Gemini timings.
    let hot = matches!(
        stage,
        "rnnoise" | "agc" | "vad" | "audio_capture_slice"
    );
    if hot && ms < 5.0 {
        return;
    }
    println!(
        "[PERF] stage={} source={} latency_ms={:.2}",
        stage, source, ms
    );
}

pub fn log_stage_ms(stage: &str, source: &str, ms: f64) {
    println!("[PERF] stage={} source={} latency_ms={:.2}", stage, source, ms);
}
