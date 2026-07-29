//! Incremental transcript builder — evolves text while the speaker is still talking.

#[derive(Debug, Clone, Default)]
pub struct IncrementalTranscriptBuilder {
    current: String,
    finalized: String,
    last_partial: String,
}

impl IncrementalTranscriptBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current(&self) -> &str {
        &self.current
    }

    pub fn reset(&mut self) {
        self.current.clear();
        self.finalized.clear();
        self.last_partial.clear();
    }

    /// Merge a new Whisper partial into the growing hypothesis.
    /// Returns the updated transcript if it changed.
    pub fn ingest_partial(&mut self, raw: &str) -> Option<String> {
        let text = raw.trim();
        if text.is_empty() {
            return None;
        }

        // Skip identical repeats from overlapping windows
        if text.eq_ignore_ascii_case(self.last_partial.trim()) {
            return None;
        }
        self.last_partial = text.to_string();

        let merged = merge_hypotheses(&self.current, text);
        if merged == self.current {
            return None;
        }
        self.current = merged;
        Some(self.current.clone())
    }

    /// Finalize after endpoint — prefer longer of current vs final Whisper pass.
    pub fn finalize_with(&mut self, final_text: &str) -> String {
        let final_text = final_text.trim();
        let best = if final_text.len() >= self.current.len() {
            final_text.to_string()
        } else if !self.current.is_empty() {
            self.current.clone()
        } else {
            final_text.to_string()
        };
        self.finalized = best.clone();
        self.current = best.clone();
        best
    }
}

fn merge_hypotheses(prev: &str, next: &str) -> String {
    let prev = prev.trim();
    let next = next.trim();
    if prev.is_empty() {
        return next.to_string();
    }
    if next.is_empty() {
        return prev.to_string();
    }

    let prev_l = prev.to_lowercase();
    let next_l = next.to_lowercase();

    // Next supersedes previous (common Whisper rewrite of full window)
    if next_l.starts_with(&prev_l) {
        return next.to_string();
    }
    // Previous already contains next (stale shorter window)
    if prev_l.starts_with(&next_l) {
        return prev.to_string();
    }
    // Next is continuation suffix / overlapping rewrite
    if let Some(overlap) = longest_overlap_words(&prev_l, &next_l) {
        if overlap >= 2 {
            let prev_words: Vec<&str> = prev.split_whitespace().collect();
            let next_words: Vec<&str> = next.split_whitespace().collect();
            let keep = prev_words.len().saturating_sub(overlap);
            let mut out: Vec<&str> = prev_words[..keep].to_vec();
            out.extend_from_slice(&next_words);
            return out.join(" ");
        }
    }

    // Fallback: append if clearly longer / different
    if next_l.contains(&prev_l) {
        return next.to_string();
    }
    format!("{} {}", prev, next)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn longest_overlap_words(a: &str, b: &str) -> Option<usize> {
    let aw: Vec<&str> = a.split_whitespace().collect();
    let bw: Vec<&str> = b.split_whitespace().collect();
    if aw.is_empty() || bw.is_empty() {
        return None;
    }
    let max = aw.len().min(bw.len()).min(12);
    for n in (1..=max).rev() {
        if aw[aw.len() - n..] == bw[..n] {
            return Some(n);
        }
    }
    None
}
