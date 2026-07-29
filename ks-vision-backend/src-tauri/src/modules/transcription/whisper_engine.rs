//! Local Whisper STT — Gemini must NEVER receive audio from this module.
//!
//! Resolution order:
//! 1. LOCAL_WHISPER_URL  (OpenAI-compatible HTTP, e.g. whisper.cpp server)
//! 2. WHISPER_CLI + WHISPER_MODEL (spawn whisper-cli / main.exe)
//! 3. Error — no Gemini multimodal fallback

use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

use super::gemini_service::write_wav_to_bytes;

#[derive(Debug, Clone)]
pub struct WhisperResult {
    pub text: String,
    pub confidence: f32,
    pub engine: String,
}

pub struct WhisperEngine;

impl WhisperEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn transcribe(&self, samples: &[f32], sample_rate: u32) -> Result<WhisperResult, String> {
        if samples.len() < 1600 {
            return Ok(WhisperResult {
                text: String::new(),
                confidence: 0.0,
                engine: "none".into(),
            });
        }

        println!("[Audio] Captured {} samples @ {}Hz", samples.len(), sample_rate);
        println!("[Whisper] Transcribing locally (audio will NOT be sent to Gemini)...");

        let wav = write_wav_to_bytes(samples, sample_rate);

        // 1) HTTP local server
        if let Ok(url) = std::env::var("LOCAL_WHISPER_URL") {
            if !url.trim().is_empty() {
                match Self::http_transcribe(url.trim(), wav.clone()).await {
                    Ok(mut r) => {
                        println!("[Whisper] Engine=http Confidence={:.2}", r.confidence);
                        println!("[Whisper] Text: {}", r.text);
                        r.engine = "whisper-http".into();
                        return Ok(r);
                    }
                    Err(e) => {
                        eprintln!("[Whisper] HTTP failed: {} — trying CLI...", e);
                    }
                }
            }
        } else {
            // Default local OpenAI-compatible endpoint
            match Self::http_transcribe("http://127.0.0.1:8080/v1/audio/transcriptions", wav.clone())
                .await
            {
                Ok(mut r) => {
                    println!("[Whisper] Engine=http-default Confidence={:.2}", r.confidence);
                    println!("[Whisper] Text: {}", r.text);
                    r.engine = "whisper-http".into();
                    return Ok(r);
                }
                Err(e) => {
                    eprintln!("[Whisper] Default HTTP unavailable: {}", e);
                }
            }
        }

        // 2) CLI binary
        match Self::cli_transcribe(&wav).await {
            Ok(mut r) => {
                println!("[Whisper] Engine=cli Confidence={:.2}", r.confidence);
                println!("[Whisper] Text: {}", r.text);
                r.engine = "whisper-cli".into();
                return Ok(r);
            }
            Err(e) => {
                eprintln!("[Whisper] CLI failed: {}", e);
            }
        }

        Err(
            "Local Whisper unavailable. Start whisper.cpp server on :8080 or set LOCAL_WHISPER_URL / WHISPER_CLI+WHISPER_MODEL. \
Gemini will NOT receive audio — fix local STT to continue."
                .into(),
        )
    }

    async fn http_transcribe(url: &str, wav_bytes: Vec<u8>) -> Result<WhisperResult, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| e.to_string())?;

        let part = reqwest::multipart::Part::bytes(wav_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| e.to_string())?;

        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("response_format", "verbose_json")
            .text("language", "en");

        let response = client
            .post(url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !response.status().is_success() {
            // Retry as plain json {text}
            return Err(format!("HTTP {}", response.status()));
        }

        let body = response.text().await.map_err(|e| e.to_string())?;

        // Some servers return plain {text}; others verbose_json
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
            let text = v
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if text.is_empty() && body.trim().starts_with('"') {
                // raw JSON string
                if let Ok(s) = serde_json::from_str::<String>(&body) {
                    return Ok(WhisperResult {
                        text: s.trim().to_string(),
                        confidence: 0.8,
                        engine: "whisper-http".into(),
                    });
                }
            }
            let confidence = confidence_from_verbose(&v);
            return Ok(WhisperResult {
                text,
                confidence,
                engine: "whisper-http".into(),
            });
        }

        // Plain text body
        let plain = body.trim().to_string();
        if !plain.is_empty() {
            return Ok(WhisperResult {
                text: plain,
                confidence: 0.8,
                engine: "whisper-http".into(),
            });
        }

        Err("Invalid Whisper JSON".into())
    }

    async fn cli_transcribe(wav_bytes: &[u8]) -> Result<WhisperResult, String> {
        let cli = resolve_whisper_cli().ok_or_else(|| {
            "WHISPER_CLI not set and whisper-cli/main not found on PATH".to_string()
        })?;
        let model = resolve_whisper_model().ok_or_else(|| {
            "WHISPER_MODEL not set (path to ggml-*.bin)".to_string()
        })?;

        let tmp_dir = std::env::temp_dir();
        let wav_path = tmp_dir.join(format!("ksvision_whisper_{}.wav", std::process::id()));
        let out_base = tmp_dir.join(format!("ksvision_whisper_out_{}", std::process::id()));
        std::fs::write(&wav_path, wav_bytes).map_err(|e| e.to_string())?;

        // whisper.cpp CLI: whisper-cli -m model -f file -otxt -of outbase -np -nt
        let output = Command::new(&cli)
            .arg("-m")
            .arg(&model)
            .arg("-f")
            .arg(&wav_path)
            .arg("-otxt")
            .arg("-of")
            .arg(&out_base)
            .arg("-np")
            .arg("-nt")
            .arg("-l")
            .arg("en")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to spawn {}: {}", cli.display(), e))?;

        let txt_path = PathBuf::from(format!("{}.txt", out_base.display()));
        let text = if txt_path.exists() {
            std::fs::read_to_string(&txt_path).unwrap_or_default()
        } else {
            String::from_utf8_lossy(&output.stdout).to_string()
        };

        let _ = std::fs::remove_file(&wav_path);
        let _ = std::fs::remove_file(&txt_path);

        if !output.status.success() && text.trim().is_empty() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("whisper-cli failed: {}", err));
        }

        Ok(WhisperResult {
            text: text.trim().to_string(),
            confidence: 0.85, // CLI without segments — assume mid-high
            engine: "whisper-cli".into(),
        })
    }
}

fn confidence_from_verbose(v: &serde_json::Value) -> f32 {
    let Some(segs) = v.get("segments").and_then(|s| s.as_array()) else {
        return 0.8;
    };
    if segs.is_empty() {
        return 0.8;
    }
    // avg_logprob typically -0.2 .. -1.0 ; map roughly to 0..1
    let mut sum = 0.0f32;
    let mut n = 0usize;
    for seg in segs {
        if let Some(lp) = seg.get("avg_logprob").and_then(|x| x.as_f64()) {
            // exp(avg_logprob) is a soft confidence proxy
            let c = (lp.exp() as f32).clamp(0.05, 1.0);
            sum += c;
            n += 1;
        } else if let Some(nllp) = seg.get("no_speech_prob").and_then(|x| x.as_f64()) {
            sum += (1.0 - nllp as f32).clamp(0.0, 1.0);
            n += 1;
        }
    }
    if n == 0 {
        0.8
    } else {
        (sum / n as f32).clamp(0.0, 1.0)
    }
}

fn resolve_whisper_cli() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("WHISPER_CLI") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    for name in ["whisper-cli", "whisper-cli.exe", "main", "main.exe", "whisper"] {
        if let Ok(out) = std::process::Command::new("where").arg(name).output() {
            if out.status.success() {
                let path = String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }
    }
    None
}

fn resolve_whisper_model() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("WHISPER_MODEL") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    // Common local locations
    let candidates = [
        "models/ggml-base.en.bin",
        "models/ggml-tiny.en.bin",
        "ggml-base.en.bin",
        "ggml-tiny.en.bin",
        "../models/ggml-base.en.bin",
        "src-tauri/models/ggml-base.en.bin",
    ];
    for c in candidates {
        let pb = PathBuf::from(c);
        if pb.exists() {
            return Some(pb);
        }
    }
    None
}
