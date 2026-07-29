//! Local transcript cleanup — runs BEFORE any Gemini text call.

/// Apply filler removal, dedupe, capitalization, punctuation, tech vocab.
pub fn postprocess_transcript(raw: &str, previous_context: &str) -> String {
    let mut text = raw.trim().to_string();
    if text.is_empty() {
        return text;
    }

    // Strip STT artifacts
    for junk in ["[BLANK_AUDIO]", "[Silence]", "(silence)", "[silence]", "♪", "♫"] {
        text = text.replace(junk, "");
    }
    text = text.trim().to_string();
    if text.is_empty() {
        return text;
    }

    text = remove_fillers(&text);
    text = collapse_repeated_words(&text);
    text = collapse_repeated_phrases(&text);
    text = apply_tech_vocabulary(&text);
    text = normalize_punctuation(&text);
    text = restore_capitalization(&text);

    // Light context-aware: if previous ended mid-sentence, lowercase join-friendly start
    if !previous_context.trim().is_empty() {
        let prev = previous_context.trim();
        let ends_open = !prev.ends_with('.') && !prev.ends_with('?') && !prev.ends_with('!');
        if ends_open && text.len() > 1 {
            // Prefer the longer overlapping hypothesis style: keep our cleaned text
            // but drop leading filler that duplicates previous tail words.
            let prev_tail: Vec<&str> = prev.split_whitespace().rev().take(4).collect();
            let mut words: Vec<&str> = text.split_whitespace().collect();
            for tail in prev_tail.iter().rev() {
                if words
                    .first()
                    .map(|w| w.eq_ignore_ascii_case(tail))
                    .unwrap_or(false)
                {
                    words.remove(0);
                }
            }
            text = words.join(" ");
            text = restore_capitalization(&text);
        }
    }

    println!("[Correction] Technical vocabulary / cleanup applied");
    text
}

fn remove_fillers(text: &str) -> String {
    let fillers = [
        " um ", " uh ", " uh,", " um,", " like ", " you know ", " sort of ", " kind of ",
    ];
    let mut out = format!(" {} ", text);
    for f in fillers {
        while out.to_lowercase().contains(f) {
            // case-insensitive replace is awkward; do simple passes
            let lower = out.to_lowercase();
            if let Some(idx) = lower.find(f) {
                out.replace_range(idx..idx + f.len(), " ");
            } else {
                break;
            }
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collapse_repeated_words(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut out = Vec::new();
    for w in words {
        if out
            .last()
            .map(|prev: &&str| prev.eq_ignore_ascii_case(w))
            .unwrap_or(false)
        {
            continue;
        }
        out.push(w);
    }
    out.join(" ")
}

fn collapse_repeated_phrases(text: &str) -> String {
    // Collapse immediate duplicated bigrams/trigrams: "runs offline runs offline" → "runs offline"
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 4 {
        return text.to_string();
    }
    for n in [3usize, 2] {
        let mut i = 0;
        let mut out = Vec::new();
        while i < words.len() {
            if i + 2 * n <= words.len() {
                let a = &words[i..i + n];
                let b = &words[i + n..i + 2 * n];
                let same = a
                    .iter()
                    .zip(b.iter())
                    .all(|(x, y)| x.eq_ignore_ascii_case(y));
                if same {
                    out.extend_from_slice(a);
                    i += 2 * n;
                    continue;
                }
            }
            out.push(words[i]);
            i += 1;
        }
        if out.len() < words.len() {
            return out.join(" ");
        }
    }
    text.to_string()
}

fn apply_tech_vocabulary(text: &str) -> String {
    // Longer phrases first
    let pairs: &[(&str, &str)] = &[
        ("node js", "Node.js"),
        ("nodejs", "Node.js"),
        ("next js", "Next.js"),
        ("nextjs", "Next.js"),
        ("nest js", "NestJS"),
        ("nestjs", "NestJS"),
        ("type script", "TypeScript"),
        ("typescript", "TypeScript"),
        ("java script", "JavaScript"),
        ("javascript", "JavaScript"),
        ("postgress", "PostgreSQL"),
        ("postgres", "PostgreSQL"),
        ("postgresql", "PostgreSQL"),
        ("mongo db", "MongoDB"),
        ("mongodb", "MongoDB"),
        ("react js", "React"),
        ("reactjs", "React"),
        ("redis cache", "Redis"),
        ("api gateway", "API Gateway"),
        ("rest api", "REST API"),
        ("graphql", "GraphQL"),
        ("oauth", "OAuth"),
        ("jwt", "JWT"),
        ("ci cd", "CI/CD"),
        ("kubernettes", "Kubernetes"),
        ("kubernetes", "Kubernetes"),
        ("docker compose", "Docker Compose"),
        ("vs code", "VS Code"),
        ("github actions", "GitHub Actions"),
        ("tailwind css", "Tailwind CSS"),
        ("web assembly", "WebAssembly"),
        ("wasm", "Wasm"),
    ];

    let mut out = text.to_string();
    for (from, to) in pairs {
        out = replace_ci_wholeish(&out, from, to);
    }
    out
}

fn replace_ci_wholeish(hay: &str, needle: &str, repl: &str) -> String {
    let lower = hay.to_lowercase();
    let n = needle.to_lowercase();
    let mut result = String::new();
    let mut i = 0;
    let bytes = hay.as_bytes();
    while i < hay.len() {
        if lower[i..].starts_with(&n) {
            let end = i + n.len();
            let before_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            let after_ok = end >= hay.len() || !bytes.get(end).map(|b| b.is_ascii_alphanumeric()).unwrap_or(false);
            if before_ok && after_ok {
                result.push_str(repl);
                i = end;
                continue;
            }
        }
        result.push(hay[i..].chars().next().unwrap());
        i += hay[i..].chars().next().unwrap().len_utf8();
    }
    result
}

fn normalize_punctuation(text: &str) -> String {
    let mut t = text.to_string();
    t = t.replace(" ,", ",").replace(" .", ".").replace(" ?", "?").replace(" !", "!");
    t = t.replace("  ", " ");
    t.trim().to_string()
}

fn restore_capitalization(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return text.to_string();
    }
    // Capitalize first letter
    if let Some(c) = chars.first_mut() {
        *c = c.to_ascii_uppercase();
    }
    // Capitalize after .?!
    let mut cap_next = false;
    for c in chars.iter_mut() {
        if cap_next && c.is_alphabetic() {
            *c = c.to_ascii_uppercase();
            cap_next = false;
        }
        if matches!(*c, '.' | '?' | '!') {
            cap_next = true;
        }
    }
    chars.into_iter().collect()
}
