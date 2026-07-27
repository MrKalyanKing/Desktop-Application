#[allow(dead_code)]
pub struct PromptBuilder;

impl PromptBuilder {
    #[allow(dead_code)]
    pub fn build(system: Option<&str>, prompt: &str, context: Option<&str>) -> String {
        let mut full_prompt = String::new();
        if let Some(sys) = system {
            full_prompt.push_str("System Instruction:\n");
            full_prompt.push_str(sys);
            full_prompt.push_str("\n\n");
        }
        if let Some(ctx) = context {
            full_prompt.push_str("Context Details:\n");
            full_prompt.push_str(ctx);
            full_prompt.push_str("\n\n");
        }
        full_prompt.push_str("User Request:\n");
        full_prompt.push_str(prompt);
        full_prompt
    }
}
