//! Fast local intent / question detection — runs before Gemini when possible.

/// Returns true when the transcript looks like a complete, actionable question/request.
pub fn looks_like_complete_question(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }

    let words: Vec<&str> = t
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .collect();
    if words.len() < 3 {
        return false;
    }

    let lower = t.to_lowercase();
    if lower.ends_with('?') {
        return true;
    }

    let starters = [
        "what", "why", "how", "when", "where", "who", "which", "whose", "whom",
        "can", "could", "would", "should", "is", "are", "do", "does", "did",
        "will", "may", "might", "explain", "describe", "tell", "give", "show",
        "please", "help",
    ];
    let first = words[0]
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase();
    if starters.iter().any(|s| *s == first) {
        // Prefer endpoint-ready: not ending mid-phrase with "and"/"or"/"the"
        let last = words[words.len() - 1]
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        let incomplete_tails = ["and", "or", "the", "a", "an", "to", "of", "in", "for", "with"];
        return !incomplete_tails.iter().any(|s| *s == last);
    }

    // Imperative / task style: "write a function...", "fix this bug"
    let verbs = ["write", "fix", "create", "implement", "refactor", "debug", "optimize", "generate"];
    if verbs.iter().any(|s| *s == first.as_str()) && words.len() >= 4 {
        return true;
    }

    false
}

/// True if text is too weak to bother Gemini with.
pub fn is_trivial_utterance(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    if t.is_empty() {
        return true;
    }
    let fillers = [
        "um", "uh", "hmm", "okay", "ok", "yeah", "yep", "right", "thanks", "thank you",
        "bye", "hello", "hi",
    ];
    let words: Vec<&str> = t
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    words.len() <= 2 && words.iter().all(|w| fillers.contains(w))
}
