//! Listening tests: real spoken questions (Windows TTS fixtures, see
//! tests/fixtures/gen_speech.ps1) mixed with noise at several SNRs and levels,
//! pushed through the production chain 48 kHz capture → resampler → segmenter.
//!
//!   cargo test --lib listening -- --nocapture            (offline matrix)
//!   cargo test --lib gemini_hears -- --ignored --nocapture  (live Gemini, needs API key)

use std::path::PathBuf;
use std::time::Instant;

use super::resampler::StreamResampler;
use super::segmenter::{SegEvent, Segmenter, Utterance, SAMPLE_RATE, SEND_DENOISED};
use super::vad_engine::{VadProfile, MIC_HANGOVER_MS, SYSTEM_HANGOVER_MS};

const QUESTIONS: [&str; 6] = [
    "q_selenium_playwright",
    "q_explicit_waits",
    "q_about_yourself",
    "q_postman_api",
    "q_page_object",
    "q_flaky_tests",
];

fn fixture(name: &str) -> Vec<f32> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/speech")
        .join(format!("{name}.wav"));
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut i = 12;
    let mut data = None;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let len = u32::from_le_bytes(bytes[i + 4..i + 8].try_into().unwrap()) as usize;
        if id == b"fmt " {
            let channels = u16::from_le_bytes([bytes[i + 10], bytes[i + 11]]);
            let rate = u32::from_le_bytes(bytes[i + 12..i + 16].try_into().unwrap());
            assert_eq!((channels, rate), (1, 16_000), "{name}: expected 16 kHz mono");
        }
        if id == b"data" {
            let end = (i + 8 + len).min(bytes.len());
            data = Some(&bytes[i + 8..end]);
            break;
        }
        i += 8 + len + (len & 1);
    }
    data.expect("no data chunk")
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
        .collect()
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32 * 2.0 - 1.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Noise {
    White,
    Pink,
    Fan,
    Hum,
    Clicks,
    Babble,
    Music,
}

impl Noise {
    const ALL: [Noise; 7] = [
        Noise::White,
        Noise::Pink,
        Noise::Fan,
        Noise::Hum,
        Noise::Clicks,
        Noise::Babble,
        Noise::Music,
    ];
    /// Non-speech, non-tonal noise: must never trigger a request on its own.
    fn is_stationary_non_speech(self) -> bool {
        matches!(self, Noise::White | Noise::Pink | Noise::Fan | Noise::Hum | Noise::Clicks)
    }
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|s| s * s).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

fn normalize(mut x: Vec<f32>) -> Vec<f32> {
    let r = rms(&x).max(1e-9);
    x.iter_mut().for_each(|s| *s /= r);
    x
}

/// Unit-RMS noise of the given kind.
fn make_noise(kind: Noise, len: usize, seed: u64) -> Vec<f32> {
    let mut rng = Rng(seed | 1);
    let sr = SAMPLE_RATE as f32;
    let tau = std::f32::consts::TAU;
    let x: Vec<f32> = match kind {
        Noise::White => (0..len).map(|_| rng.next()).collect(),
        Noise::Pink => {
            let (mut b0, mut b1, mut b2) = (0.0f32, 0.0f32, 0.0f32);
            (0..len)
                .map(|_| {
                    let w = rng.next();
                    b0 = 0.99765 * b0 + w * 0.0990460;
                    b1 = 0.96300 * b1 + w * 0.2965164;
                    b2 = 0.57000 * b2 + w * 1.0526913;
                    b0 + b1 + b2 + w * 0.1848
                })
                .collect()
        }
        Noise::Fan => {
            let mut lp = 0.0f32;
            (0..len)
                .map(|i| {
                    lp = 0.97 * lp + 0.03 * rng.next();
                    lp * 8.0 + 0.15 * (tau * 120.0 * i as f32 / sr).sin()
                })
                .collect()
        }
        Noise::Hum => (0..len)
            .map(|i| {
                let t = i as f32 / sr;
                [(50.0, 1.0), (100.0, 0.5), (150.0, 0.3), (250.0, 0.2)]
                    .iter()
                    .map(|(f, a)| a * (tau * f * t).sin())
                    .sum()
            })
            .collect(),
        Noise::Clicks => {
            let mut v = vec![0.0f32; len];
            let mut pos = 0usize;
            while pos < len {
                let burst = 80; // 5 ms keystroke
                for k in 0..burst.min(len - pos) {
                    v[pos + k] = rng.next() * (-(k as f32) / 15.0).exp();
                }
                pos += 1_600 + ((rng.next() + 1.0) * 1_400.0) as usize; // 100–275 ms apart
            }
            v
        }
        Noise::Babble => {
            let mut v = vec![0.0f32; len];
            for (k, name) in QUESTIONS.iter().enumerate() {
                let talker = fixture(name);
                let offset = k * 5_300;
                for i in 0..len {
                    v[i] += talker[(i + offset) % talker.len()];
                }
            }
            v
        }
        Noise::Music => {
            let chords = [[220.0, 277.2, 329.6], [196.0, 246.9, 293.7], [174.6, 220.0, 261.6]];
            (0..len)
                .map(|i| {
                    let t = i as f32 / sr;
                    let chord = chords[(t / 0.5) as usize % chords.len()];
                    let trem = 0.75 + 0.25 * (tau * 4.0 * t).sin();
                    chord
                        .iter()
                        .map(|f| (tau * f * t).sin() + 0.3 * (tau * 2.0 * f * t).sin())
                        .sum::<f32>()
                        * trem
                })
                .collect()
        }
    };
    normalize(x)
}

/// [first, last) sample of audible speech in a clean clip.
fn speech_region(clean: &[f32]) -> (usize, usize) {
    let win = 160;
    let energies: Vec<f32> = clean.chunks(win).map(rms).collect();
    let peak = energies.iter().cloned().fold(0.0f32, f32::max);
    let th = peak * 0.03;
    let first = energies.iter().position(|&e| e > th).unwrap_or(0);
    let last = energies.iter().rposition(|&e| e > th).unwrap_or(energies.len() - 1);
    (first * win, ((last + 1) * win).min(clean.len()))
}

struct Scene {
    audio: Vec<f32>,
    /// Speech regions [on, off) in samples.
    regions: Vec<(usize, usize)>,
}

/// Clips placed with gaps, speech scaled to `speech_dbfs`, noise at `snr_db` below it.
fn build_scene(
    clips: &[(&[f32], f32)], // (clip, silence before it in seconds)
    tail_s: f32,
    speech_dbfs: f32,
    noise: Option<(Noise, f32)>,
    seed: u64,
) -> Scene {
    let target = 10f32.powf(speech_dbfs / 20.0);
    let mut audio = Vec::new();
    let mut regions = Vec::new();
    for (clip, gap) in clips {
        audio.extend(std::iter::repeat(0.0).take((gap * SAMPLE_RATE as f32) as usize));
        let (on, off) = speech_region(clip);
        let gain = target / rms(&clip[on..off]).max(1e-6);
        let base = audio.len();
        audio.extend(clip.iter().map(|s| s * gain));
        regions.push((base + on, base + off));
    }
    audio.extend(std::iter::repeat(0.0).take((tail_s * SAMPLE_RATE as f32) as usize));

    if let Some((kind, snr_db)) = noise {
        let level = target / 10f32.powf(snr_db / 20.0);
        let n = make_noise(kind, audio.len(), seed);
        for (a, b) in audio.iter_mut().zip(n) {
            *a += b * level;
        }
    }
    for s in audio.iter_mut() {
        *s = s.clamp(-1.0, 1.0);
    }
    Scene { audio, regions }
}

struct RunResult {
    /// (utterance, clock when it was emitted)
    utterances: Vec<(Utterance, u64)>,
    rejected: usize,
    elapsed_s: f64,
}

/// 16 kHz scene → 48 kHz "device" → production resampler → segmenter, in 10 ms callbacks.
fn run(scene: &[f32], profile: VadProfile) -> RunResult {
    run_with(scene, profile, SEND_DENOISED)
}

fn run_with(scene: &[f32], profile: VadProfile, send_denoised: bool) -> RunResult {
    let mut up = StreamResampler::new(16_000, 48_000);
    let device = up.process(scene);
    let mut down = StreamResampler::new(48_000, 16_000);
    let mut seg = Segmenter::with_send_denoised(profile, send_denoised);
    let mut events = Vec::new();
    let mut utterances = Vec::new();
    let mut rejected = 0;
    let t0 = Instant::now();
    for cb in device.chunks(480) {
        let samples = down.process(cb);
        seg.push(&samples, &mut events);
        for ev in events.drain(..) {
            match ev {
                SegEvent::Utterance(u) => utterances.push((u, seg.clock())),
                SegEvent::Rejected => rejected += 1,
                SegEvent::State(_) => {}
            }
        }
    }
    RunResult {
        utterances,
        rejected,
        elapsed_s: t0.elapsed().as_secs_f64(),
    }
}

struct Detection {
    detected: bool,
    /// Fraction of the speech region inside the final (possibly merged) utterance.
    coverage: f32,
    /// Speech end → utterance emitted.
    latency_ms: f32,
    pieces: usize,
}

fn detect(result: &RunResult, (on, off): (usize, usize)) -> Detection {
    let overlapping: Vec<&(Utterance, u64)> = result
        .utterances
        .iter()
        .filter(|(u, _)| (u.start_clock as usize) < off && (u.end_clock as usize) > on)
        .collect();
    let Some((last, emitted)) = overlapping.last() else {
        return Detection {
            detected: false,
            coverage: 0.0,
            latency_ms: f32::NAN,
            pieces: 0,
        };
    };
    let a = (last.start_clock as usize).max(on);
    let b = (last.end_clock as usize).min(off);
    Detection {
        detected: true,
        coverage: b.saturating_sub(a) as f32 / (off - on) as f32,
        latency_ms: (*emitted as f32 - off as f32) * 1000.0 / SAMPLE_RATE as f32,
        pieces: overlapping.len(),
    }
}

fn profile_name(p: VadProfile) -> &'static str {
    match p {
        VadProfile::Microphone => "mic",
        VadProfile::SystemLoopback => "system",
    }
}

fn hangover(p: VadProfile) -> f32 {
    match p {
        VadProfile::Microphone => MIC_HANGOVER_MS as f32,
        VadProfile::SystemLoopback => SYSTEM_HANGOVER_MS as f32,
    }
}

/// Questions at normal / quiet / loud levels, clean and in 7 noise types at
/// 20/10/5/0 dB SNR, for mic and system profiles.
#[test]
fn listening_matrix_in_noise() {
    let levels = [("normal", -22.0f32), ("quiet", -40.0), ("loud", -8.0)];
    let mut jobs = Vec::new();
    for profile in [VadProfile::Microphone, VadProfile::SystemLoopback] {
        for level in levels {
            jobs.push((profile, level));
        }
    }
    let results: Vec<(Vec<String>, Vec<String>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = jobs
            .iter()
            .map(|&(profile, level)| scope.spawn(move || matrix_slice(profile, level)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    println!("
=== Listening matrix (6 spoken questions per row) ===");
    let mut failures = Vec::new();
    for (rows, fails) in results {
        rows.iter().for_each(|r| println!("{r}"));
        failures.extend(fails);
    }
    assert!(failures.is_empty(), "{} failures:
{}", failures.len(), failures.join("
"));
}

/// One profile × level: clean + 7 noise types × 4 SNRs. Returns (report rows, failures).
fn matrix_slice(profile: VadProfile, (level_name, dbfs): (&str, f32)) -> (Vec<String>, Vec<String>) {
    let snrs = [20.0f32, 10.0, 5.0, 0.0];
    let mut failures = Vec::new();
    let mut rows = Vec::new();
    let mut conditions: Vec<Option<(Noise, f32)>> = vec![None];
    for n in Noise::ALL {
        for snr in snrs {
            conditions.push(Some((n, snr)));
        }
    }
    for cond in conditions {
        let (mut hit, mut cov_sum, mut lat_max, mut split) = (0, 0.0f32, 0.0f32, 0);
        for (k, q) in QUESTIONS.iter().enumerate() {
            let clip = fixture(q);
            let scene = build_scene(&[(&clip, 1.5)], 2.0, dbfs, cond, 7 + k as u64);
            let res = run(&scene.audio, profile);
            let d = detect(&res, scene.regions[0]);
            if d.detected {
                hit += 1;
                cov_sum += d.coverage;
                lat_max = lat_max.max(d.latency_ms);
                if d.pieces > 1 {
                    split += 1;
                }
            }
            // Requirements: speech at >=5 dB SNR in stationary noise (or clean)
            // is always heard, whole, and closed promptly.
            let must = match cond {
                None => true,
                Some((n, snr)) => n.is_stationary_non_speech() && snr >= 5.0,
            };
            if must {
                let ok = d.detected && d.coverage >= 0.95 && d.latency_ms <= hangover(profile) + 300.0;
                if !ok {
                    failures.push(format!(
                        "{} {} {:?} {}: detected={} coverage={:.2} latency={:.0}ms",
                        profile_name(profile), level_name, cond, q, d.detected, d.coverage, d.latency_ms
                    ));
                }
            }
        }
        rows.push(format!(
            "{:<6} {:<6} {:<16} heard {}/{}  coverage {:>5.1}%  worst-endpoint {:>4.0} ms  split {}",
            profile_name(profile),
            level_name,
            cond.map(|(k, s)| format!("{:?} {:>2}dB", k, s)).unwrap_or_else(|| "clean".into()),
            hit,
            QUESTIONS.len(),
            if hit > 0 { cov_sum / hit as f32 * 100.0 } else { 0.0 },
            lat_max,
            split
        ));
    }
    (rows, failures)
}

/// 15 s of each noise with no speech must not produce a single request.
#[test]
fn listening_ignores_noise_only() {
    let mut failures = Vec::new();
    println!("\n=== Noise only, 15 s (false requests) ===");
    for profile in [VadProfile::Microphone, VadProfile::SystemLoopback] {
        for kind in Noise::ALL {
            for dbfs in [-45.0f32, -30.0, -18.0] {
                let len = 15 * SAMPLE_RATE;
                let level = 10f32.powf(dbfs / 20.0);
                let audio: Vec<f32> = make_noise(kind, len, 99)
                    .into_iter()
                    .map(|s| (s * level).clamp(-1.0, 1.0))
                    .collect();
                let res = run(&audio, profile);
                println!(
                    "{:<6} {:<7?} {:>4} dBFS  requests {}  ignored-bursts {}",
                    profile_name(profile),
                    kind,
                    dbfs,
                    res.utterances.len(),
                    res.rejected
                );
                if kind.is_stationary_non_speech() && !res.utterances.is_empty() {
                    failures.push(format!("{} {:?} {} dBFS → {} requests", profile_name(profile), kind, dbfs, res.utterances.len()));
                }
            }
        }
    }
    assert!(failures.is_empty(), "false triggers:\n{}", failures.join("\n"));
}

/// Mid-sentence pauses must not split the question that is sent.
#[test]
fn listening_keeps_paused_questions_whole() {
    for profile in [VadProfile::Microphone, VadProfile::SystemLoopback] {
        for (name, noise) in [
            ("q_pause_short", None),
            ("q_pause_long", None),
            ("q_pause_short", Some((Noise::Pink, 10.0))),
            ("q_pause_long", Some((Noise::Fan, 10.0))),
        ] {
            let clip = fixture(name);
            let scene = build_scene(&[(&clip, 1.0)], 2.0, -22.0, noise, 3);
            let res = run(&scene.audio, profile);
            let d = detect(&res, scene.regions[0]);
            println!(
                "{:<6} {:<14} {:?}: pieces {} final coverage {:.2}",
                profile_name(profile), name, noise, d.pieces, d.coverage
            );
            assert!(d.detected, "{name} not heard");
            // A split is fine only if the final request carries the whole question.
            assert!(d.coverage >= 0.95, "{name}: final request covers {:.2}", d.coverage);
        }
    }
}

/// Two different questions 2.5 s apart are two requests, the second not merged.
#[test]
fn listening_separates_distinct_questions() {
    let a = fixture("q_postman_api");
    let b = fixture("q_page_object");
    let scene = build_scene(&[(&a, 1.0), (&b, 2.5)], 2.0, -22.0, Some((Noise::Pink, 15.0)), 5);
    let res = run(&scene.audio, VadProfile::Microphone);
    assert_eq!(res.utterances.len(), 2, "expected 2 requests");
    assert!(!res.utterances[1].0.merged);
    let d = detect(&res, scene.regions[1]);
    assert!(d.coverage >= 0.95);
}

/// A fan switching on mid-session must not latch the VAD open.
#[test]
fn listening_adapts_to_noise_change() {
    let q = fixture("q_explicit_waits");
    let mut scene = build_scene(&[(&q, 6.0)], 4.0, -22.0, None, 1);
    let fan = make_noise(Noise::Fan, scene.audio.len(), 4);
    let level = 10f32.powf(-34.0 / 20.0);
    let switch_on = 2 * SAMPLE_RATE;
    for i in switch_on..scene.audio.len() {
        scene.audio[i] += fan[i] * level;
    }
    for profile in [VadProfile::Microphone, VadProfile::SystemLoopback] {
        let res = run(&scene.audio, profile);
        let d = detect(&res, scene.regions[0]);
        let longest = res.utterances.iter().map(|(u, _)| u.samples.len()).max().unwrap_or(0);
        println!(
            "{}: requests {} coverage {:.2} longest {:.1}s",
            profile_name(profile),
            res.utterances.len(),
            d.coverage,
            longest as f32 / SAMPLE_RATE as f32
        );
        assert!(d.detected && d.coverage >= 0.95);
        assert!(longest < 8 * SAMPLE_RATE, "VAD latched on fan noise");
    }
}

/// Filler words alone are short; they may be sent, but never merged into a later question.
#[test]
fn listening_filler_then_question() {
    let filler = fixture("filler_hmm");
    let q = fixture("q_flaky_tests");
    let scene = build_scene(&[(&filler, 1.0), (&q, 3.0)], 2.0, -22.0, None, 2);
    let res = run(&scene.audio, VadProfile::Microphone);
    let last = &res.utterances.last().expect("question not heard").0;
    assert!(!last.merged, "filler 3 s earlier was merged into the question");
}

/// Processing cost per second of audio (both profiles). Must be far below real time.
#[test]
fn listening_realtime_factor() {
    let q = fixture("q_selenium_playwright");
    let clips: Vec<(&[f32], f32)> = (0..8).map(|_| (q.as_slice(), 2.0)).collect();
    let scene = build_scene(&clips, 1.0, -22.0, Some((Noise::Babble, 10.0)), 8);
    let secs = scene.audio.len() as f64 / SAMPLE_RATE as f64;
    for profile in [VadProfile::Microphone, VadProfile::SystemLoopback] {
        let res = run(&scene.audio, profile);
        let rtf = res.elapsed_s / secs;
        println!(
            "{}: {:.1}s audio processed in {:.0} ms (real-time factor {:.3}, {} build)",
            profile_name(profile),
            secs,
            res.elapsed_s * 1000.0,
            rtf,
            if cfg!(debug_assertions) { "debug" } else { "release" }
        );
        assert!(rtf < if cfg!(debug_assertions) { 1.0 } else { 0.1 });
    }
}

/// End-to-end: segmenter output sent to the real Gemini model with the production
/// prompt. Checks the model's `Q:` line names the right topic, in noise.
#[tokio::test]
#[ignore]
async fn gemini_hears_questions_in_noise() {
    use crate::modules::ai::model_manager::GeminiModelManager;
    use crate::modules::transcription::gemini_voice;

    let key = gemini_voice::load_api_key_from_disk();
    assert!(!key.is_empty(), "GEMINI_API_KEY not found in env or .env");
    let model: &str = GeminiModelManager::global().primary_model();
    let client = reqwest::Client::new();

    let expect: [(&str, &[&str]); 7] = [
        ("q_selenium_playwright", &["selenium", "playwright"]),
        ("q_explicit_waits", &["explicit", "wait"]),
        ("q_about_yourself", &["yourself"]),
        ("q_postman_api", &["postman"]),
        ("q_page_object", &["page object"]),
        ("q_flaky_tests", &["flaky"]),
        ("q_pause_long", &["testng", "junit"]),
    ];
    let conditions: [(&str, f32, Option<(Noise, f32)>); 6] = [
        ("clean", -22.0, None),
        ("pink 10dB", -22.0, Some((Noise::Pink, 10.0))),
        ("babble 10dB", -22.0, Some((Noise::Babble, 10.0))),
        ("fan 5dB", -22.0, Some((Noise::Fan, 5.0))),
        ("clicks 5dB", -22.0, Some((Noise::Clicks, 5.0))),
        ("quiet+white 15dB", -40.0, Some((Noise::White, 15.0))),
    ];

    let mut pass = 0;
    let mut total = 0;
    let mut by_mode = [(0, 0), (0, 0)]; // (pass, total) for [denoised, natural]
    let mut latencies = Vec::new();
    println!("\n=== Gemini end-to-end ({model}) ===");
    for (cond_name, dbfs, noise) in conditions {
        for send_denoised in [true, false] {
            let mode = if send_denoised { "denoised" } else { "natural" };
            let m = !send_denoised as usize;
            for (k, (name, words)) in expect.iter().enumerate() {
                let clip = fixture(name);
                let scene = build_scene(&[(&clip, 1.0)], 1.5, dbfs, noise, 11 + k as u64);
                let res = run_with(&scene.audio, VadProfile::Microphone, send_denoised);
                total += 1;
                by_mode[m].1 += 1;
                let Some((utt, _)) = res.utterances.last() else {
                    println!("{cond_name:<17} {mode:<8} {name:<22} NOT DETECTED");
                    continue;
                };
                let ok = ask_gemini(&client, &key, model, &utt.samples, words, &mut latencies, cond_name, mode, name).await;
                if ok {
                    pass += 1;
                    by_mode[m].0 += 1;
                }
            }
        }
    }
    println!(
        "\ndenoised audio: {}/{}   natural audio: {}/{}",
        by_mode[0].0, by_mode[0].1, by_mode[1].0, by_mode[1].1
    );
    latencies.sort();
    println!(
        "heard correctly: {pass}/{total}   request latency median {} ms, p90 {} ms",
        latencies[latencies.len() / 2],
        latencies[latencies.len() * 9 / 10]
    );
    assert!(pass * 100 >= total * 85, "only {pass}/{total} heard correctly");
}

/// One non-streaming request with the production prompt; true if the `Q:` line
/// contains every expected word.
#[allow(clippy::too_many_arguments)]
async fn ask_gemini(
    client: &reqwest::Client,
    key: &str,
    model: &str,
    samples: &[f32],
    words: &[&str],
    latencies: &mut Vec<u128>,
    cond_name: &str,
    mode: &str,
    name: &str,
) -> bool {
    use crate::modules::audio::preprocess;
    use crate::modules::cognition::scenario_engine::ScenarioEngine;
    use crate::modules::transcription::gemini_voice::{self, HeardHeader};
    use crate::modules::transcription::wav::write_wav_to_bytes;
    use base64::{engine::general_purpose::STANDARD as B64, Engine};

    let prepared = preprocess::prepare_for_voice(samples);
    let wav = write_wav_to_bytes(&prepared, SAMPLE_RATE as u32);
    let mut variant = 0u8;
    let mut retries = 0u64;
    let t0 = Instant::now();
    let text = loop {
        let payload = serde_json::json!({
            "systemInstruction": { "parts": [{ "text": ScenarioEngine::live_system_prompt(false) }] },
            "contents": [{ "role": "user", "parts": [
                { "text": "Audio clip follows. Apply the live listening rules." },
                { "inlineData": { "mimeType": "audio/wav", "data": B64.encode(&wav) } }
            ]}],
            "generationConfig": gemini_voice::generation_config(160, variant)
        });
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={key}"
        );
        let resp = client.post(&url).json(&payload).send().await.expect("network");
        let status = resp.status();
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        if status.as_u16() == 400 && variant < 2 && body.to_string().to_lowercase().contains("thinking") {
            variant += 1;
            continue;
        }
        // Transient overload: retry like the app's model fallback would.
        if matches!(status.as_u16(), 429 | 500 | 503) && retries < 4 {
            retries += 1;
            tokio::time::sleep(std::time::Duration::from_secs(3 * retries)).await;
            continue;
        }
        assert!(status.is_success(), "{status}: {body}");
        break body["candidates"][0]["content"]["parts"]
            .as_array()
            .map(|p| p.iter().filter_map(|x| x["text"].as_str()).collect::<String>())
            .unwrap_or_default();
    };
    latencies.push(t0.elapsed().as_millis());
    let heard = match gemini_voice::parse_heard_header(&text) {
        HeardHeader::Heard { question, .. } => question.to_string(),
        _ => text.lines().next().unwrap_or("").to_string(),
    };
    let lower = heard.to_lowercase();
    let ok = words.iter().all(|w| lower.contains(w));
    println!(
        "{cond_name:<17} {mode:<8} {name:<22} {} heard: {heard}",
        if ok { "PASS" } else { "MISS" }
    );
    ok
}

/// Time to first streamed token with the production request shape, per thinking setting.
///   cargo test --lib gemini_latency -- --ignored --nocapture
#[tokio::test]
#[ignore]
async fn gemini_latency_probe() {
    use crate::modules::ai::model_manager::GeminiModelManager;
    use crate::modules::audio::preprocess;
    use crate::modules::cognition::scenario_engine::ScenarioEngine;
    use crate::modules::transcription::gemini_voice;
    use crate::modules::transcription::wav::write_wav_to_bytes;
    use base64::{engine::general_purpose::STANDARD as B64, Engine};
    use futures_util::StreamExt;

    let key = gemini_voice::load_api_key_from_disk();
    assert!(!key.is_empty(), "GEMINI_API_KEY not found");
    let client = reqwest::Client::builder().tcp_nodelay(true).build().unwrap();
    let clip = fixture("q_selenium_playwright");
    let scene = build_scene(&[(&clip, 0.5)], 1.0, -22.0, None, 1);
    let utt = run(&scene.audio, VadProfile::Microphone).utterances.pop().unwrap().0;
    let wav = write_wav_to_bytes(&preprocess::prepare_for_voice(&utt.samples), SAMPLE_RATE as u32);
    let audio = B64.encode(&wav);

    for model in GeminiModelManager::global().known_models() {
        for variant in 0..3u8 {
            let mut ttfts = Vec::new();
            let mut status_note = String::new();
            for _ in 0..4 {
                let payload = serde_json::json!({
                    "systemInstruction": { "parts": [{ "text": ScenarioEngine::live_system_prompt(false) }] },
                    "contents": [{ "role": "user", "parts": [
                        { "text": "Audio clip follows. Apply the live listening rules." },
                        { "inlineData": { "mimeType": "audio/wav", "data": audio } }
                    ]}],
                    "generationConfig": gemini_voice::generation_config(512, variant)
                });
                let url = format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/{model}:streamGenerateContent?alt=sse&key={key}"
                );
                let t0 = Instant::now();
                let resp = client.post(&url).json(&payload).send().await.expect("network");
                if !resp.status().is_success() {
                    let st = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    status_note = format!("{st}: {}", body.chars().take(120).collect::<String>());
                    break;
                }
                let mut stream = resp.bytes_stream();
                let mut first = None;
                let mut total = 0u128;
                while let Some(chunk) = stream.next().await {
                    let chunk = chunk.expect("stream");
                    if first.is_none() && String::from_utf8_lossy(&chunk).contains("\"text\"") {
                        first = Some(t0.elapsed().as_millis());
                    }
                    total = t0.elapsed().as_millis();
                }
                ttfts.push((first.unwrap_or(total), total));
            }
            let label = ["thinkingLevel=minimal", "thinkingBudget=0", "default thinking"][variant as usize];
            if ttfts.is_empty() {
                println!("{model:<24} {label:<22} rejected: {status_note}");
            } else {
                ttfts.sort();
                println!(
                    "{model:<24} {label:<22} first token median {} ms (min {} ms)   full answer median {} ms",
                    ttfts[ttfts.len() / 2].0,
                    ttfts[0].0,
                    ttfts[ttfts.len() / 2].1
                );
            }
        }
    }
}

/// Live: (1) answers arrive as many streamed pieces over time, not one block;
/// (2) a new question after an answered one is answered on its own topic.
///   cargo test --lib gemini_streams -- --ignored --nocapture
#[tokio::test]
#[ignore]
async fn gemini_streams_and_switches_topic() {
    use crate::modules::ai::model_manager::GeminiModelManager;
    use crate::modules::audio::preprocess;
    use crate::modules::cognition::scenario_engine::ScenarioEngine;
    use crate::modules::transcription::gemini_voice::{self, HeardHeader, SseTextParser};
    use crate::modules::transcription::wav::write_wav_to_bytes;
    use base64::{engine::general_purpose::STANDARD as B64, Engine};
    use futures_util::StreamExt;

    let key = gemini_voice::load_api_key_from_disk();
    assert!(!key.is_empty(), "GEMINI_API_KEY not found");
    let model = GeminiModelManager::global().primary_model();
    let client = reqwest::Client::new();
    gemini_voice::clear_turns();

    // Ask a sequence of unrelated questions, carrying context like the app does.
    let sequence: [(&str, &[&str], &[&str]); 4] = [
        ("q_selenium_playwright", &["selenium", "playwright"], &[]),
        ("q_postman_api", &["postman"], &["playwright"]),
        ("q_flaky_tests", &["flaky"], &["postman"]),
        ("q_page_object", &["page object"], &["flaky"]),
    ];
    let mut failures = Vec::new();
    for (name, must_hear, must_not_mention) in sequence {
        let clip = fixture(name);
        let scene = build_scene(&[(&clip, 0.5)], 1.0, -22.0, Some((Noise::Pink, 15.0)), 3);
        let utt = run(&scene.audio, VadProfile::Microphone).utterances.pop().unwrap().0;
        let wav = write_wav_to_bytes(&preprocess::prepare_for_voice(&utt.samples), SAMPLE_RATE as u32);
        let payload = serde_json::json!({
            "systemInstruction": { "parts": [{ "text": ScenarioEngine::live_system_prompt(false) }] },
            "contents": [{ "role": "user", "parts": [
                { "text": gemini_voice::listen_text(false) },
                { "inlineData": { "mimeType": "audio/wav", "data": B64.encode(&wav) } }
            ]}],
            "generationConfig": gemini_voice::generation_config(512, 0)
        });
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:streamGenerateContent?alt=sse&key={key}"
        );
        let mut attempt = 0u64;
        let resp = loop {
            let r = client.post(&url).json(&payload).send().await.expect("network");
            if matches!(r.status().as_u16(), 429 | 500 | 503) && attempt < 4 {
                attempt += 1;
                tokio::time::sleep(std::time::Duration::from_secs(3 * attempt)).await;
                continue;
            }
            assert!(r.status().is_success(), "{}", r.status());
            break r;
        };
        let t0 = Instant::now();
        let mut sse = SseTextParser::default();
        let mut text = String::new();
        let mut pieces: Vec<(u128, usize)> = Vec::new();
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let delta = sse.push(&chunk.expect("stream"));
            if !delta.is_empty() {
                text.push_str(&delta);
                pieces.push((t0.elapsed().as_millis(), text.len()));
            }
        }
        text.push_str(&sse.finish());

        let (heard, answer) = match gemini_voice::parse_heard_header(&text) {
            HeardHeader::Heard { question, body } => (question.to_string(), body.to_string()),
            _ => (String::new(), text.clone()),
        };
        let lower_heard = heard.to_lowercase();
        let lower_answer = answer.to_lowercase();
        let heard_ok = must_hear.iter().all(|w| lower_heard.contains(w));
        let leaked: Vec<&&str> = must_not_mention.iter().filter(|w| lower_answer.contains(*w)).collect();
        println!(
            "{name:<22} pieces {:>2} (first at {} ms, last at {} ms)  heard: {heard}{}",
            pieces.len(),
            pieces.first().map(|p| p.0).unwrap_or(0),
            pieces.last().map(|p| p.0).unwrap_or(0),
            if leaked.is_empty() { String::new() } else { format!("  ⚠ mentions old topic {leaked:?}") }
        );
        if pieces.len() < 2 {
            failures.push(format!("{name}: answer arrived in {} piece(s)", pieces.len()));
        }
        if !heard_ok {
            failures.push(format!("{name}: heard '{heard}'"));
        }
        if !leaked.is_empty() {
            failures.push(format!("{name}: answer drifted to previous topic {leaked:?}"));
        }
        gemini_voice::remember_turn(Some(&heard), &answer);
    }
    gemini_voice::clear_turns();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
