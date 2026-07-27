use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct ContentClassification {
    pub content_type: String,     // "Code" | "Error" | "Document" | "UI" | "Unknown"
    pub language: String,         // "Rust" | "TypeScript" | "Python" | "Java" | "C#" | "Go" | "N/A"
    pub confidence: u32,
}

pub fn detect_content_type(text: &str) -> ContentClassification {
    let lower = text.to_lowercase();
    
    let mut code_score = 0u32;
    let mut error_score = 0u32;
    let mut doc_score = 0u32;
    let mut ui_score = 0u32;

    let code_keywords = vec![
        "let ", "fn ", "mut ", "const ", "import ", "export ", "class ", "pub ", "impl ", 
        "struct ", "interface ", "def ", "func ", "package ", "namespace ", "using ", "async ", 
        "await ", "return ", "for ", "while ", "if ", "else ", "match ", "switch ", "case ",
        "void ", "private ", "protected ", "public "
    ];
    for kw in code_keywords {
        if lower.contains(kw) {
            code_score += 10;
        }
    }
    if lower.contains("=>") || lower.contains("<-") || lower.contains("console.log") {
        code_score += 15;
    }

    let error_keywords = vec![
        "exception", "stacktrace", "traceback", "caused by:", "at ", "panic!", "error:",
        "build failed", "exit code:", "nullpointer", "undefined", "not found", "cannot find",
        "thread 'main'", "fatal:", "compilation error"
    ];
    for kw in error_keywords {
        if lower.contains(kw) {
            error_score += 12;
        }
    }

    let doc_keywords = vec![
        "introduction", "summary", "chapter", "section", "documentation", "table of contents",
        "guide", "reference", "license", "author", "readme", "revision", "overview"
    ];
    for kw in doc_keywords {
        if lower.contains(kw) {
            doc_score += 10;
        }
    }
    if lower.contains("# ") || lower.contains("## ") || lower.contains("### ") {
        doc_score += 20;
    }

    let ui_keywords = vec![
        "submit", "cancel", "save", "delete", "edit", "dashboard", "settings", "username",
        "password", "login", "register", "profile", "home", "search", "filter", "apply",
        "button", "input", "form", "checkbox", "radio"
    ];
    for kw in ui_keywords {
        if lower.contains(kw) {
            ui_score += 8;
        }
    }

    let scores = vec![
        ("Code", code_score),
        ("Error", error_score),
        ("Document", doc_score),
        ("UI", ui_score)
    ];

    let max = scores.iter().max_by_key(|&&(_, val)| val).unwrap();
    
    if max.1 < 15 {
        return ContentClassification {
            content_type: "Unknown".to_string(),
            language: "N/A".to_string(),
            confidence: 50,
        };
    }

    let c_type = max.0.to_string();
    let mut confidence = (max.1.min(100) as f32 / 100.0 * 100.0) as u32;
    confidence = confidence.max(65).min(98);

    let mut language = "N/A".to_string();
    if c_type == "Code" {
        let mut rust = 0;
        let mut ts = 0;
        let mut py = 0;
        let mut java = 0;
        let mut cs = 0;
        let mut go = 0;

        if lower.contains("fn ") || lower.contains("pub ") || lower.contains("match ") || lower.contains("impl ") || lower.contains("mut ") { rust += 25; }
        if lower.contains("let ") { rust += 5; ts += 5; }
        if lower.contains("const ") || lower.contains("import ") || lower.contains("interface ") || lower.contains("console.log") || lower.contains("=>") { ts += 25; }
        if lower.contains("def ") || lower.contains("self.") || lower.contains("elif ") || lower.contains("print(") { py += 20; }
        if lower.contains("public class ") || lower.contains("public static void main") || lower.contains("@override") || lower.contains("system.out.println") { java += 30; }
        if lower.contains("using system") || lower.contains("Console.WriteLine") { cs += 25; }
        if lower.contains("package ") || lower.contains("func ") || lower.contains("chan ") { go += 30; }

        let lang_scores = vec![
            ("Rust", rust),
            ("TypeScript", ts),
            ("Python", py),
            ("Java", java),
            ("C#", cs),
            ("Go", go)
        ];
        let max_lang = lang_scores.iter().max_by_key(|&&(_, val)| val).unwrap();
        if max_lang.1 > 5 {
            language = max_lang.0.to_string();
        }
    }

    ContentClassification {
        content_type: c_type,
        language,
        confidence,
    }
}
