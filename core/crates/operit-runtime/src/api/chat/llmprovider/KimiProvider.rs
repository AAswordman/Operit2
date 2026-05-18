use super::AIService::AIService;

pub struct KimiProvider {
    pub api_endpoint: String,
    pub model_name: String,
    pub provider_type: String,
    pub supports_vision: bool,
    pub supports_audio: bool,
    pub supports_video: bool,
    pub enable_tool_call: bool,
}

impl KimiProvider {
    pub fn create_request_body(&self) {}
}

impl AIService for KimiProvider {
    fn provider_model(&self) -> String {
        format!("{}:{}", self.provider_type, self.model_name)
    }
}
