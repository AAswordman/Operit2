use super::AIService::AIService;

pub struct OllamaProvider;

impl OllamaProvider {
    pub fn create_request_body(&self) {}
}

impl AIService for OllamaProvider {}
