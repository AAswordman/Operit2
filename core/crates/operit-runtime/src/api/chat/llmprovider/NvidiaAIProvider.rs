use super::AIService::AIService;

pub struct NvidiaAIProvider;

impl NvidiaAIProvider {
    pub fn create_request_body(&self) {}
}

impl AIService for NvidiaAIProvider {}
