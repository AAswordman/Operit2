use super::AIService::AIService;

pub struct ToolPkgJsAiProviderService;

impl ToolPkgJsAiProviderService {
    pub fn invoke_provider_function(&self, _event_name: &str) {}

    pub fn build_base_payload(&self) {}

    pub fn serialize_prompt_turn(&self) {}
}

impl AIService for ToolPkgJsAiProviderService {}
