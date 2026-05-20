use super::super::hooks::PromptTurn::PromptTurn;
use crate::util::stream::HotStream::MutableSharedStreamImpl;

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
    pub stream: MutableSharedStreamImpl<String>,
}

pub trait MessageProcessingPlugin {
    fn id(&self) -> &str;
}

pub struct MessageProcessingPluginRegistry;

impl MessageProcessingPluginRegistry {
    #[allow(non_snake_case)]
    pub fn createExecutionIfMatched(
        _params: MessageProcessingHookParams,
    ) -> Option<MessageProcessingExecution<Box<dyn MessageProcessingController + Send + Sync>>> {
        None
    }
}
