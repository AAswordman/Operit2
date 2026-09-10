use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use operit_model::InputProcessingState::InputProcessingState;
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_CHAT_RUNTIME;
use operit_plugin_sdk::toolpkg::ToolPkgHooks::ToolPkgChatRuntimeHookRegistration;
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;
use operit_util::ChainLogger::{self, PLUGIN_CHAIN};

use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;

static CHAT_RUNTIME_HOOKS: OnceLock<Mutex<Vec<ToolPkgChatRuntimeHookRegistration>>> =
    OnceLock::new();
static CHAT_RUNTIME: OnceLock<ToolPkgBridgeRuntime> = OnceLock::new();

/// Names the chat runtime state-change event delivered to ToolPkg hooks.
pub const CHAT_RUNTIME_EVENT_STATE_CHANGED: &str = "state_changed";

/// Connects ToolPkg chat runtime hooks to application chat state changes.
pub struct ToolPkgChatRuntimeHookBridge;

impl ToolPkgChatRuntimeHookBridge {
    /// Registers chat runtime hooks for one application runtime.
    pub fn register(runtime: ToolPkgBridgeRuntime) {
        CHAT_RUNTIME.get_or_init(|| runtime.clone());
        let manager = runtime.package_manager();
        manager.addToolPkgRuntimeChangeListener(std::sync::Arc::new(|activeContainers| {
            ToolPkgChatRuntimeHookBridge::syncToolPkgRegistrations(activeContainers);
        }));
    }

    /// Synchronizes active chat runtime hook registrations from enabled ToolPkg containers.
    #[allow(non_snake_case)]
    pub fn syncToolPkgRegistrations(activeContainers: Vec<ToolPkgContainerRuntime>) {
        let mut hooks = activeContainers
            .iter()
            .flat_map(|runtime| {
                runtime
                    .chatRuntimeHooks
                    .iter()
                    .map(|hook| ToolPkgChatRuntimeHookRegistration {
                        containerPackageName: runtime.packageName.clone(),
                        hookId: hook.id.clone(),
                        functionName: hook.function.clone(),
                        functionSource: hook.functionSource.clone(),
                    })
            })
            .collect::<Vec<_>>();
        hooks.sort_by(|left, right| {
            left.containerPackageName
                .cmp(&right.containerPackageName)
                .then(left.hookId.cmp(&right.hookId))
        });
        *CHAT_RUNTIME_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg chat runtime hook mutex poisoned") = hooks;
    }

    /// Dispatches a state-change notification through registered ToolPkg chat runtime hooks.
    #[allow(non_snake_case)]
    pub fn dispatchStateChanged(
        chatId: &str,
        state: &InputProcessingState,
        activeChatIds: Vec<String>,
        currentTurnToolInvocationCount: i32,
        activeConversationCount: i32,
        currentSessionToolCount: i32,
    ) {
        let Some(runtime) = CHAT_RUNTIME.get() else {
            return;
        };
        let hooks = CHAT_RUNTIME_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg chat runtime hook mutex poisoned")
            .clone();
        if hooks.is_empty() {
            return;
        }
        let payload = buildPayload(
            chatId,
            state,
            activeChatIds,
            currentTurnToolInvocationCount,
            activeConversationCount,
            currentSessionToolCount,
        );
        let manager = runtime.package_manager();
        for hook in hooks {
            ChainLogger::info(
                PLUGIN_CHAIN,
                "plugin.toolpkg.chat_runtime.run.start",
                &[
                    ("event", CHAT_RUNTIME_EVENT_STATE_CHANGED.to_string()),
                    ("package", hook.containerPackageName.clone()),
                    ("hookId", hook.hookId.clone()),
                    ("function", hook.functionName.clone()),
                ],
            );
            match manager.runToolPkgMainHook(
                &hook.containerPackageName,
                &hook.functionName,
                TOOLPKG_EVENT_CHAT_RUNTIME,
                Some(CHAT_RUNTIME_EVENT_STATE_CHANGED),
                Some(&hook.hookId),
                hook.functionSource.as_deref(),
                payload.clone(),
                None,
                None,
                None,
            ) {
                Ok(_) => ChainLogger::info(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.chat_runtime.run.done",
                    &[
                        ("event", CHAT_RUNTIME_EVENT_STATE_CHANGED.to_string()),
                        ("package", hook.containerPackageName.clone()),
                        ("hookId", hook.hookId.clone()),
                    ],
                ),
                Err(error) => ChainLogger::error(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.chat_runtime.run.error",
                    &[
                        ("event", CHAT_RUNTIME_EVENT_STATE_CHANGED.to_string()),
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

/// Builds the stable payload delivered to ToolPkg chat runtime hooks.
#[allow(non_snake_case)]
fn buildPayload(
    chatId: &str,
    state: &InputProcessingState,
    mut activeChatIds: Vec<String>,
    currentTurnToolInvocationCount: i32,
    activeConversationCount: i32,
    currentSessionToolCount: i32,
) -> serde_json::Value {
    activeChatIds.sort();
    serde_json::json!({
        "chatId": chatId,
        "slot": "main",
        "state": stateName(state),
        "message": stateMessage(state),
        "toolName": stateToolName(state),
        "progress": stateProgress(state),
        "isActive": stateIsActive(state),
        "activeChatIds": activeChatIds,
        "currentTurnToolInvocationCount": currentTurnToolInvocationCount,
        "activeConversationCount": activeConversationCount,
        "currentSessionToolCount": currentSessionToolCount,
        "timestamp": operit_host_api::TimeUtils::currentTimeMillis(),
    })
}

/// Returns the wire name associated with one input processing state.
#[allow(non_snake_case)]
fn stateName(state: &InputProcessingState) -> &'static str {
    match state {
        InputProcessingState::Idle => "idle",
        InputProcessingState::Processing { .. } => "processing",
        InputProcessingState::Connecting { .. } => "connecting",
        InputProcessingState::Receiving { .. } => "receiving",
        InputProcessingState::ExecutingTool { .. } => "executing_tool",
        InputProcessingState::ToolProgress { .. } => "tool_progress",
        InputProcessingState::ProcessingToolResult { .. } => "processing_tool_result",
        InputProcessingState::Summarizing { .. } => "summarizing",
        InputProcessingState::ExecutingPlan { .. } => "executing_plan",
        InputProcessingState::Completed => "completed",
        InputProcessingState::Error { .. } => "error",
    }
}

/// Returns optional display text associated with one input processing state.
#[allow(non_snake_case)]
fn stateMessage(state: &InputProcessingState) -> Option<&str> {
    match state {
        InputProcessingState::Processing { message }
        | InputProcessingState::Connecting { message }
        | InputProcessingState::Receiving { message }
        | InputProcessingState::Summarizing { message }
        | InputProcessingState::ExecutingPlan { message }
        | InputProcessingState::Error { message }
        | InputProcessingState::ToolProgress { message, .. } => Some(message),
        _ => None,
    }
}

/// Returns the tool name associated with one input processing state.
#[allow(non_snake_case)]
fn stateToolName(state: &InputProcessingState) -> Option<&str> {
    match state {
        InputProcessingState::ExecutingTool { toolName }
        | InputProcessingState::ToolProgress { toolName, .. }
        | InputProcessingState::ProcessingToolResult { toolName } => Some(toolName),
        _ => None,
    }
}

/// Returns tool progress associated with one input processing state.
#[allow(non_snake_case)]
fn stateProgress(state: &InputProcessingState) -> Option<f32> {
    match state {
        InputProcessingState::ToolProgress { progress, .. } => Some(*progress),
        _ => None,
    }
}

/// Returns whether one input processing state represents active work.
#[allow(non_snake_case)]
fn stateIsActive(state: &InputProcessingState) -> bool {
    !matches!(
        state,
        InputProcessingState::Idle
            | InputProcessingState::Completed
            | InputProcessingState::Error { .. }
    )
}
