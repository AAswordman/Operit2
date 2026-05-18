use super::AIService::AIService;

pub struct ClaudeProvider {
    pub api_endpoint: String,
    pub model_name: String,
    pub provider_type: String,
    pub enable_tool_call: bool,
}

impl ClaudeProvider {
    pub fn create_request_body(&self) {}

    pub fn build_messages_and_count_tokens(&self) {}

    pub fn apply_stable_cache_breakpoints(&self) {}
}

impl AIService for ClaudeProvider {
    fn provider_model(&self) -> String {
        format!("{}:{}", self.provider_type, self.model_name)
    }
}
