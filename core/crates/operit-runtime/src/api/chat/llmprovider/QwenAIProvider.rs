use super::AIService::AIService;

pub struct QwenAIProvider {
    pub api_endpoint: String,
    pub model_name: String,
    pub provider_type: String,
    pub supports_vision: bool,
    pub supports_audio: bool,
    pub supports_video: bool,
    pub enable_tool_call: bool,
}

impl QwenAIProvider {
    pub fn create_request_body(&self) {}

    pub fn apply_qwen_reasoning_settings(&self) {}

    pub fn resolve_silicon_flow_thinking_budget(&self) {}
}

impl AIService for QwenAIProvider {
    fn provider_model(&self) -> String {
        format!("{}:{}", self.provider_type, self.model_name)
    }
}
