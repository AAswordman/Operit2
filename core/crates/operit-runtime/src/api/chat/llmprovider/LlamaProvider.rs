use super::AIService::AIService;

pub struct LlamaProvider;

impl LlamaProvider {
    pub fn create_session_config(&self) {}
}

impl AIService for LlamaProvider {}
