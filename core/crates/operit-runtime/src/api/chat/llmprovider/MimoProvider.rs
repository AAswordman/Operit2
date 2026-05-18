use super::AIService::AIService;

pub struct MimoProvider {
    pub api_endpoint: String,
    pub model_name: String,
    pub provider_type: String,
    pub supports_vision: bool,
    pub supports_audio: bool,
    pub supports_video: bool,
    pub enable_tool_call: bool,
}

impl MimoProvider {
    pub fn create_request_body(&self) {}
}

impl AIService for MimoProvider {
    fn provider_model(&self) -> String {
        format!("{}:{}", self.provider_type, self.model_name)
    }
}
