pub struct TranscriptCorrector;

impl TranscriptCorrector {
    /// Perform technical terminology boosting and basic transcript cleanup.
    pub fn correct(text: &str) -> String {
        let mut corrected = text.to_string();

        let replacements = vec![
            ("sqlite", "SQLite"),
            ("react", "React"),
            ("tauri", "Tauri"),
            ("rust", "Rust"),
            ("vite", "Vite"),
            ("copiolot", "Copilot"),
            ("copilot", "Copilot"),
            ("gemini", "Gemini"),
            ("github", "GitHub"),
            ("npm", "npm"),
            ("npx", "npx"),
            ("cargo", "cargo"),
            ("index css", "index.css"),
            ("index.css", "index.css"),
            ("main rs", "main.rs"),
            ("main.rs", "main.rs"),
            ("package json", "package.json"),
            ("package.json", "package.json"),
            ("typescript", "TypeScript"),
            ("javascript", "JavaScript"),
            ("groq", "Groq"),
            ("onnx", "ONNX"),
            ("wasapi", "WASAPI"),
            ("cpal", "CPAL"),
            ("too many request", "Too Many Requests"),
            ("too many requests", "Too Many Requests"),
            ("stack buffer overrun", "STATUS_STACK_BUFFER_OVERRUN"),
            ("status_stack_buffer_overrun", "STATUS_STACK_BUFFER_OVERRUN"),
            ("status stack buffer overrun", "STATUS_STACK_BUFFER_OVERRUN"),
        ];

        for &(from, to) in &replacements {
            let mut start_idx = 0;
            // Loop with lowercase search matching
            while let Some(pos) = corrected.to_lowercase()[start_idx..].find(from) {
                let absolute_pos = start_idx + pos;
                corrected.replace_range(absolute_pos..absolute_pos + from.len(), to);
                start_idx = absolute_pos + to.len();
            }
        }

        // Clean up common speech-to-text formatting spacing errors
        let mut result = corrected
            .replace(" ,", ",")
            .replace(" .", ".")
            .replace(" ?", "?")
            .replace(" !", "!")
            .replace(" :", ":")
            .replace(" ;", ";");

        // Capitalize the first letter if needed
        if let Some(first_char) = result.chars().next() {
            if first_char.is_ascii_lowercase() {
                result.replace_range(0..1, &first_char.to_uppercase().to_string());
            }
        }

        result
    }
}
