use super::super::hooks::PromptTurn::PromptTurn;

pub struct MessageProcessingHookParams {
    pub chat_id: Option<String>,
    pub message_content: String,
    pub chat_history: Vec<PromptTurn>,
    pub workspace_path: Option<String>,
    pub max_tokens: i32,
    pub token_usage_threshold: f64,
}

pub trait MessageProcessingController {
    fn cancel(&self);
}

pub struct MessageProcessingExecution<TController> {
    pub controller: TController,
}

pub trait MessageProcessingPlugin {
    fn id(&self) -> &str;
}

pub struct MessageProcessingPluginRegistry;
