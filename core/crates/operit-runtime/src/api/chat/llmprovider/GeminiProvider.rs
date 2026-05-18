use super::AIService::AIService;

pub struct GeminiProvider {
    pub api_endpoint: String,
    pub model_name: String,
    pub provider_type: String,
    pub enable_google_search: bool,
    pub enable_tool_call: bool,
}

impl GeminiProvider {
    pub fn create_request_body(&self) {}
}

impl AIService for GeminiProvider {
    fn provider_model(&self) -> String {
        format!("{}:{}", self.provider_type, self.model_name)
    }
}
