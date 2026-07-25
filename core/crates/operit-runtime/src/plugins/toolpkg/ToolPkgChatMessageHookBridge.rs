use std::sync::{Mutex, OnceLock};

use operit_model::ChatMessage::ChatMessage;
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_CHAT_MESSAGE;
use operit_plugin_sdk::toolpkg::ToolPkgHooks::ToolPkgChatMessageHookRegistration;
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;
use operit_util::ChainLogger::{self, PLUGIN_CHAIN};
use serde_json::Value;

use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;

static CHAT_MESSAGE_HOOKS: OnceLock<Mutex<Vec<ToolPkgChatMessageHookRegistration>>> =
    OnceLock::new();
static CHAT_MESSAGE_RUNTIME: OnceLock<ToolPkgBridgeRuntime> = OnceLock::new();

pub const CHAT_MESSAGE_EVENT_PERSISTED: &str = "message_persisted";

pub struct ToolPkgChatMessageHookBridge;

impl ToolPkgChatMessageHookBridge {
    /// Registers chat message persistence hooks for one application runtime.
    pub fn register(runtime: ToolPkgBridgeRuntime) {
        CHAT_MESSAGE_RUNTIME.get_or_init(|| runtime.clone());
        let manager = runtime.package_manager();
        manager.addToolPkgRuntimeChangeListener(std::sync::Arc::new(|activeContainers| {
            ToolPkgChatMessageHookBridge::syncToolPkgRegistrations(activeContainers);
        }));
    }

    /// Synchronizes active chat message hook registrations from enabled ToolPkg containers.
    #[allow(non_snake_case)]
    pub fn syncToolPkgRegistrations(activeContainers: Vec<ToolPkgContainerRuntime>) {
        let mut hooks = activeContainers
            .iter()
            .flat_map(|runtime| {
                runtime.chatMessageHooks.iter().map(|hook| {
                    ToolPkgChatMessageHookRegistration {
                        containerPackageName: runtime.packageName.clone(),
                        hookId: hook.id.clone(),
                        functionName: hook.function.clone(),
                        functionSource: hook.functionSource.clone(),
                    }
                })
            })
            .collect::<Vec<_>>();
        hooks.sort_by(|left, right| {
            left.containerPackageName
                .cmp(&right.containerPackageName)
                .then(left.hookId.cmp(&right.hookId))
        });
        *CHAT_MESSAGE_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg chat message hook mutex poisoned") = hooks;
    }

    /// Dispatches a notification after a chat message has been persisted.
    #[allow(non_snake_case)]
    pub fn dispatchMessagePersisted(chatId: &str, message: &ChatMessage) {
        let Some(runtime) = CHAT_MESSAGE_RUNTIME.get() else {
            return;
        };
        let activeHooks = CHAT_MESSAGE_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg chat message hook mutex poisoned")
            .clone();
        if activeHooks.is_empty() {
            return;
        }

        ChainLogger::info(
            PLUGIN_CHAIN,
            "plugin.toolpkg.chat_message.scan",
            &[
                ("event", CHAT_MESSAGE_EVENT_PERSISTED.to_string()),
                ("chatId", chatId.to_string()),
                ("sender", message.sender.clone()),
                ("timestamp", message.timestamp.to_string()),
                ("hookCount", activeHooks.len().to_string()),
            ],
        );

        let eventPayload = buildChatMessagePayload(chatId, message);
        let manager = runtime.package_manager();
        for hook in activeHooks {
            ChainLogger::info(
                PLUGIN_CHAIN,
                "plugin.toolpkg.chat_message.run.start",
                &[
                    ("event", CHAT_MESSAGE_EVENT_PERSISTED.to_string()),
                    ("package", hook.containerPackageName.clone()),
                    ("hookId", hook.hookId.clone()),
                    ("function", hook.functionName.clone()),
                ],
            );
            match manager.runToolPkgMainHook(
                &hook.containerPackageName,
                &hook.functionName,
                TOOLPKG_EVENT_CHAT_MESSAGE,
                Some(CHAT_MESSAGE_EVENT_PERSISTED),
                Some(&hook.hookId),
                hook.functionSource.as_deref(),
                eventPayload.clone(),
                None,
                None,
                None,
            ) {
                Ok(_) => ChainLogger::info(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.chat_message.run.done",
                    &[
                        ("event", CHAT_MESSAGE_EVENT_PERSISTED.to_string()),
                        ("package", hook.containerPackageName.clone()),
                        ("hookId", hook.hookId.clone()),
                    ],
                ),
                Err(error) => ChainLogger::error(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.chat_message.run.error",
                    &[
                        ("event", CHAT_MESSAGE_EVENT_PERSISTED.to_string()),
                        ("package", hook.containerPackageName.clone()),
                        ("hookId", hook.hookId.clone()),
                        ("function", hook.functionName.clone()),
                        ("error", error),
                    ],
                ),
            }
        }
    }
}

/// Builds the stable payload delivered to chat message persistence hooks.
#[allow(non_snake_case)]
fn buildChatMessagePayload(chatId: &str, message: &ChatMessage) -> Value {
    serde_json::json!({
        "chatId": chatId,
        "timestamp": message.timestamp,
        "sender": message.sender,
        "roleName": message.roleName,
        "content": message.content,
        "completedAt": message.completedAt,
        "provider": message.provider,
        "modelName": message.modelName,
        "inputTokens": message.inputTokens,
        "outputTokens": message.outputTokens,
        "cachedInputTokens": message.cachedInputTokens,
        "sentAt": message.sentAt,
        "outputDurationMs": message.outputDurationMs,
        "waitDurationMs": message.waitDurationMs,
        "displayMode": format!("{:?}", message.displayMode),
        "selectedVariantIndex": message.selectedVariantIndex,
        "isFavorite": message.isFavorite,
    })
}
