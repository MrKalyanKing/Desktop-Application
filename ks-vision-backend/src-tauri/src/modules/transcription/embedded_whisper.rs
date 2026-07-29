//! In-process Whisper STT via whisper-rs (no HTTP, no CLI, no localhost).
//! Model is loaded once at startup and reused for the app lifetime.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::modules::audio::perf_metrics;

struct EngineInner {
    ctx: WhisperContext,
    model_path: PathBuf,
}

static ENGINE: OnceLock<Mutex<EngineInner>> = OnceLock::new();

const BUNDLED_MODEL_NAME: &str = "ggml-tiny.en.bin";

/// Resolve bundled model path (dev + packaged).
pub fn resolve_model_path(resource_dir: Option<PathBuf>) -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(res) = resource_dir {
        candidates.push(res.join("resources").join("models").join(BUNDLED_MODEL_NAME));
        candidates.push(res.join("models").join(BUNDLED_MODEL_NAME));
        candidates.push(res.join(BUNDLED_MODEL_NAME));
    }

    // Dev / source tree next to src-tauri
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        candidates.push(
            PathBuf::from(manifest)
                .join("resources")
                .join("models")
                .join(BUNDLED_MODEL_NAME),
        );
    }

    // Relative to current exe (installed layout)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("resources").join("models").join(BUNDLED_MODEL_NAME));
            candidates.push(dir.join("models").join(BUNDLED_MODEL_NAME));
        }
    }

    // CWD fallback
    candidates.push(PathBuf::from("resources/models").join(BUNDLED_MODEL_NAME));

    for p in &candidates {
        if p.exists() {
            return p.clone();
        }
    }

    // Return preferred install path for error message
    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from(BUNDLED_MODEL_NAME))
}

/// Load the bundled model once. Safe to call multiple times.
pub fn init_embedded_whisper(resource_dir: Option<PathBuf>) -> Result<(), String> {
    if ENGINE.get().is_some() {
        println!("[Embedded Whisper] Already initialized");
        return Ok(());
    }

    let model_path = resolve_model_path(resource_dir);
    if !model_path.exists() {
        return Err(format!(
            "Bundled speech model not found at {}. Reinstall the application.",
            model_path.display()
        ));
    }

    println!(
        "[Embedded Whisper] Loading model once: {}",
        model_path.display()
    );
    let t0 = perf_metrics::now();

    let params = WhisperContextParameters::default();
    let ctx = WhisperContext::new_with_params(
        model_path.to_string_lossy().as_ref(),
        params,
    )
    .map_err(|e| format!("Failed to load embedded Whisper model: {}", e))?;

    ENGINE
        .set(Mutex::new(EngineInner {
            ctx,
            model_path: model_path.clone(),
        }))
        .map_err(|_| "Embedded Whisper already set".to_string())?;

    perf_metrics::log_stage("whisper_model_load", "startup", t0);
    println!("[Embedded Whisper] Ready (in-process, no external service)");
    Ok(())
}

pub fn is_ready() -> bool {
    ENGINE.get().is_some()
}

pub fn model_path_loaded() -> Option<PathBuf> {
    ENGINE
        .get()
        .and_then(|m| m.lock().ok().map(|g| g.model_path.clone()))
}

/// Transcribe 16 kHz mono f32 PCM entirely in-process.
pub fn transcribe_pcm(samples: &[f32], sample_rate: u32) -> Result<(String, f32), String> {
    if samples.len() < 1600 {
        return Ok((String::new(), 0.0));
    }

    let engine = ENGINE
        .get()
        .ok_or_else(|| "Embedded Whisper not initialized".to_string())?;

    // Resample if needed (pipeline is already 16 kHz)
    let audio: Vec<f32> = if sample_rate == 16000 {
        samples.to_vec()
    } else {
        crate::modules::audio::resampler::LinearResampler::new().process(
            samples,
            sample_rate,
            16000,
        )
    };

    // Cap ~30s to bound CPU for long meetings
    let max = 30 * 16000;
    let audio = if audio.len() > max {
        audio[audio.len() - max..].to_vec()
    } else {
        audio
    };

    let t0 = perf_metrics::now();
    let mut guard = engine
        .lock()
        .map_err(|_| "Embedded Whisper lock poisoned".to_string())?;

    let mut state = guard
        .ctx
        .create_state()
        .map_err(|e| format!("Whisper create_state failed: {}", e))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_translate(false);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_suppress_blank(true);
    params.set_no_context(true);
    params.set_single_segment(false);
    // Lower threads a bit for UI responsiveness during long meetings
    let threads = std::thread::available_parallelism()
        .map(|n| (n.get() as i32).clamp(1, 4))
        .unwrap_or(2);
    params.set_n_threads(threads);

    state
        .full(params, &audio)
        .map_err(|e| format!("Whisper inference failed: {}", e))?;

    let n = state.full_n_segments();
    let mut text = String::new();
    for i in 0..n {
        if let Some(seg) = state.get_segment(i) {
            if let Ok(s) = seg.to_str_lossy() {
                let t = s.trim();
                if !t.is_empty() {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(t);
                }
            }
        }
    }

    perf_metrics::log_stage("embedded_whisper", "stt", t0);

    let text = text.trim().to_string();
    if text.is_empty() {
        return Ok((String::new(), 0.0));
    }

    let confidence = 0.85f32;
    println!("[Embedded Whisper] {}", text);
    Ok((text, confidence))
}

/// Async wrapper — runs blocking inference off the Tokio worker threads.
pub async fn transcribe_pcm_async(
    samples: Vec<f32>,
    sample_rate: u32,
) -> Result<(String, f32), String> {
    tokio::task::spawn_blocking(move || transcribe_pcm(&samples, sample_rate))
        .await
        .map_err(|e| format!("Whisper task join error: {}", e))?
}

#[allow(dead_code)]
pub fn ensure_initialized_from_cwd() -> Result<(), String> {
    init_embedded_whisper(None)
}

pub fn bundled_model_name() -> &'static str {
    BUNDLED_MODEL_NAME
}

pub fn assert_model_file(path: &Path) -> Result<(), String> {
    if path.exists() {
        Ok(())
    } else {
        Err(format!("Model missing: {}", path.display()))
    }
}
