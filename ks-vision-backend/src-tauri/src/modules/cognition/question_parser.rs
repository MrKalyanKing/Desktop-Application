use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubQuestion {
    pub text: String,
    pub is_question: bool,
    pub dependencies: Vec<usize>, // 1-based indices in the array
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QuestionParseResponse {
    pub questions: Vec<SubQuestion>,
}

pub struct QuestionParser;

#[derive(Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
}

#[derive(Deserialize)]
struct GeminiContent {
    parts: Option<Vec<GeminiPart>>,
}

#[derive(Deserialize)]
struct GeminiPart {
    text: Option<String>,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

impl QuestionParser {
    pub fn new() -> Self {
        Self
    }

    fn load_api_key(&self) -> String {
        if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            if !key.is_empty() && key != "YOUR_GEMINI_API_KEY_HERE" {
                return key;
            }
        }
        if let Ok(key) = std::env::var("VITE_GEMINI_API_KEY") {
            if !key.is_empty() && key != "YOUR_GEMINI_API_KEY_HERE" {
                return key;
            }
        }
        
        let paths = vec![".env", "../.env", "src-tauri/.env", "../src-tauri/.env"];
        for path in paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some(pos) = line.find('=') {
                        let key = line[..pos].trim();
                        let value = line[pos + 1..].trim();
                        let value = value.trim_matches('"').trim_matches('\'');
                        if key == "GEMINI_API_KEY" || key == "VITE_GEMINI_API_KEY" {
                            if !value.is_empty() && value != "YOUR_GEMINI_API_KEY_HERE" {
                                return value.to_string();
                            }
                        }
                    }
                }
            }
        }
        String::new()
    }

    pub async fn parse_questions(
        &self,
        text: &str,
        from_system_audio: bool,
    ) -> Result<Vec<SubQuestion>, String> {
        let api_key = self.load_api_key();
        if api_key.is_empty() {
            return Err("Gemini API key is not configured.".to_string());
        }

        let system_instruction = if from_system_audio {
            "You are a meeting intelligence copilot listening to REMOTE callers (Meet/Teams/Zoom/YouTube). \
Analyze the transcript (may contain STT errors / noise) and extract ONLY actionable questions or \
explicit requests directed at the local user / interview candidate / assistant.

RULES:
1. Repair grammar and obvious STT errors into clean questions.
2. Set is_question=true ONLY for real questions or actionable tasks (not greetings, filler, or statements).
3. Ignore chit-chat, music lyrics, and UI announcements.
4. If nothing actionable, return {\"questions\": []}.
5. Output raw JSON only:
{\"questions\":[{\"text\":\"...\",\"is_question\":true,\"dependencies\":[]}]}"
        } else {
            "You are an advanced meeting intelligence copilot. Analyze spoken transcripts \
(may contain STT errors) and extract discrete actionable questions or tasks.

1. Semantic repair: fix grammar, remove repetitions/fillers.
2. is_question=true for actionable questions/tasks only.
3. If none, return {\"questions\": []}.
4. Output raw JSON:
{\"questions\":[{\"text\":\"...\",\"is_question\":true,\"dependencies\":[]}]}"
        };

        let user_prompt = format!("Input: \"{}\"\n\nParse into the JSON schema.", text);

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "systemInstruction": {
                "parts": [{ "text": system_instruction }]
            },
            "contents": [{
                "parts": [{ "text": user_prompt }]
            }],
            "generationConfig": {
                "responseMimeType": "application/json",
                "temperature": 0.2
            }
        });

        let manager = crate::modules::ai::model_manager::GeminiModelManager::global();
        let mut attempts = 0;

        loop {
            let active_model = manager.select_model();
            println!("[QUESTION PARSER] Using: {}", active_model);

            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                active_model, api_key
            );

            let resp = match client.post(&url).json(&payload).send().await {
                Ok(r) => r,
                Err(e) => return Err(format!("Failed to send question parsing request: {}", e)),
            };

            let status_code = resp.status();
            if !status_code.is_success() {
                let err_text = resp.text().await.unwrap_or_default();
                eprintln!(
                    "[QUESTION PARSER ERROR] Gemini API returned status {}: {}",
                    status_code, err_text
                );

                if manager.is_quota_error(status_code, &err_text) {
                    let delay = manager.parse_retry_delay(&err_text);
                    manager.blacklist_model(&active_model, &err_text, delay);

                    attempts += 1;
                    if attempts < manager.models_len() {
                        let next_model = manager.select_model();
                        println!("[MODEL MANAGER] 429 received -> Switching to: {}", next_model);
                        continue;
                    }
                }

                return Err(format!(
                    "Gemini API returned error status {}: {}",
                    status_code, err_text
                ));
            }

            let gemini_resp: GeminiResponse = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

            let json_text = gemini_resp
                .candidates
                .and_then(|c| c.into_iter().next())
                .and_then(|c| c.content)
                .and_then(|c| c.parts)
                .and_then(|p| p.into_iter().next())
                .and_then(|p| p.text)
                .unwrap_or_default();

            let parsed: QuestionParseResponse = serde_json::from_str(&json_text).map_err(|e| {
                format!(
                    "Failed to parse structured JSON questions: {}. Raw: {}",
                    e, json_text
                )
            })?;

            return Ok(parsed.questions);
        }
    }
}
