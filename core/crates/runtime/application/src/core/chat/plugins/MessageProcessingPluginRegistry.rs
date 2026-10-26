use operit_model::PromptTurn::PromptTurn;
use operit_util::stream::HotStream::MutableSharedStreamImpl;
use operit_util::ChainLogger::{self, PLUGIN_CHAIN};
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

pub trait MessageProcessingPlugin: Send + Sync {
    fn id(&self) -> &str;

    #[allow(non_snake_case)]
    fn createExecutionIfMatched<'a>(
        &'a self,
        params: &'a MessageProcessingHookParams,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Option<
                        MessageProcessingExecution<
                            Box<dyn MessageProcessingController + Send + Sync>,
                        >,
                    >,
                > + Send
                + 'a,
        >,
    >;
}

pub struct MessageProcessingPluginRegistry;

impl MessageProcessingPluginRegistry {
    #[allow(non_snake_case)]
    pub fn register(plugin: Arc<dyn MessageProcessingPlugin + Send + Sync>) {
        let pluginId = plugin.id().to_string();
        let mut plugins = plugins()
            .lock()
            .expect("message plugin registry mutex poisoned");
        plugins.retain(|item| item.id() != plugin.id());
        plugins.push(plugin);
        ChainLogger::verbose_with_level(
            PLUGIN_CHAIN,
            "plugin.message_processing.register",
            &[("pluginId", pluginId)],
            operit_util::AppLogger::VERBOSE_LEVEL_4,
        );
    }

    #[allow(non_snake_case)]
    pub fn unregister(pluginId: &str) {
        let mut plugins = plugins()
            .lock()
            .expect("message plugin registry mutex poisoned");
        plugins.retain(|item| item.id() != pluginId);
        ChainLogger::verbose_with_level(
            PLUGIN_CHAIN,
            "plugin.message_processing.unregister",
            &[("pluginId", pluginId.to_string())],
            operit_util::AppLogger::VERBOSE_LEVEL_4,
        );
    }

    #[allow(non_snake_case)]
    pub async fn createExecutionIfMatched(
        params: MessageProcessingHookParams,
    ) -> Option<MessageProcessingExecution<Box<dyn MessageProcessingController + Send + Sync>>>
    {
        let plugins = plugins()
            .lock()
            .expect("message plugin registry mutex poisoned")
            .clone();
        ChainLogger::verbose_with_level(
            PLUGIN_CHAIN,
            "plugin.message_processing.scan",
            &[
                ("pluginCount", plugins.len().to_string()),
                (
                    "messageChars",
                    ChainLogger::lenField(&params.message_content),
                ),
                ("historyCount", params.chat_history.len().to_string()),
            ],
            operit_util::AppLogger::VERBOSE_LEVEL_3,
        );
        for plugin in plugins {
            let pluginId = plugin.id().to_string();
            let execution = plugin.createExecutionIfMatched(&params).await;
            if execution.is_some() {
                ChainLogger::verbose_with_level(
                    PLUGIN_CHAIN,
                    "plugin.message_processing.execution",
                    &[("pluginId", pluginId)],
                    operit_util::AppLogger::VERBOSE_LEVEL_3,
                );
                return execution;
            }
        }
        ChainLogger::verbose_with_level(
            PLUGIN_CHAIN,
            "plugin.message_processing.no_match",
            &[(
                "messageChars",
                ChainLogger::lenField(&params.message_content),
            )],
            operit_util::AppLogger::VERBOSE_LEVEL_3,
        );
        None
    }
}

fn plugins() -> &'static Mutex<Vec<Arc<dyn MessageProcessingPlugin + Send + Sync>>> {
    static PLUGINS: OnceLock<Mutex<Vec<Arc<dyn MessageProcessingPlugin + Send + Sync>>>> =
        OnceLock::new();
    PLUGINS.get_or_init(|| Mutex::new(Vec::new()))
}
