use super::AIService::AIService;

pub struct MistralProvider;

impl MistralProvider {
    pub fn create_request_body(&self) {}
}

impl AIService for MistralProvider {}
