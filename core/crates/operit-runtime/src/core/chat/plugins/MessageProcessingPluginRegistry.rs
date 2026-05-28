use super::super::hooks::PromptTurn::PromptTurn;
use crate::util::stream::HotStream::MutableSharedStreamImpl;
use std::sync::{Arc, Mutex, OnceLock};

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

    #[allow(non_snake_case)]
    fn createExecutionIfMatched(
        &self,
        params: &MessageProcessingHookParams,
    ) -> Option<MessageProcessingExecution<Box<dyn MessageProcessingController + Send + Sync>>>;
}

pub struct MessageProcessingPluginRegistry;

impl MessageProcessingPluginRegistry {
    #[allow(non_snake_case)]
    pub fn register(plugin: Arc<dyn MessageProcessingPlugin + Send + Sync>) {
        let mut plugins = plugins()
            .lock()
            .expect("message plugin registry mutex poisoned");
        plugins.retain(|item| item.id() != plugin.id());
        plugins.push(plugin);
    }

    #[allow(non_snake_case)]
    pub fn unregister(pluginId: &str) {
        let mut plugins = plugins()
            .lock()
            .expect("message plugin registry mutex poisoned");
        plugins.retain(|item| item.id() != pluginId);
    }

    #[allow(non_snake_case)]
    pub fn createExecutionIfMatched(
        params: MessageProcessingHookParams,
    ) -> Option<MessageProcessingExecution<Box<dyn MessageProcessingController + Send + Sync>>>
    {
        let plugins = plugins()
            .lock()
            .expect("message plugin registry mutex poisoned")
            .clone();
        for plugin in plugins {
            let execution = plugin.createExecutionIfMatched(&params);
            if execution.is_some() {
                return execution;
            }
        }
        None
    }
}

fn plugins() -> &'static Mutex<Vec<Arc<dyn MessageProcessingPlugin + Send + Sync>>> {
    static PLUGINS: OnceLock<Mutex<Vec<Arc<dyn MessageProcessingPlugin + Send + Sync>>>> =
        OnceLock::new();
    PLUGINS.get_or_init(|| Mutex::new(Vec::new()))
}
