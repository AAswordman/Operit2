use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use serde_json::Value;

use crate::core::tools::packTool::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_CHAT_VIEW;
use crate::core::tools::packTool::ToolPkgParser::ToolPkgContainerRuntime;
use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::{
    toolPkgPackageManager, ToolPkgChatViewHookRegistration,
};
use crate::util::ChainLogger::{self, PLUGIN_CHAIN};

static CHAT_VIEW_HOOKS: OnceLock<Mutex<Vec<ToolPkgChatViewHookRegistration>>> = OnceLock::new();
static REPLAYABLE_OPEN_VIEW_PARAMS: OnceLock<Mutex<Vec<ChatViewHookParams>>> = OnceLock::new();

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatViewEvent {
    ViewOpened,
    ViewClosed,
    ViewFocused,
    ViewBlurred,
}

impl ChatViewEvent {
    #[allow(non_snake_case)]
    pub fn wireName(&self) -> &'static str {
        match self {
            ChatViewEvent::ViewOpened => "view_opened",
            ChatViewEvent::ViewClosed => "view_closed",
            ChatViewEvent::ViewFocused => "view_focused",
            ChatViewEvent::ViewBlurred => "view_blurred",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChatViewHookParams {
    pub viewId: String,
    pub chatId: String,
    pub workspacePath: Option<String>,
    pub workspaceEnv: Value,
    pub runtime: String,
    pub title: Option<String>,
}

pub struct ToolPkgChatViewHookBridge;

impl ToolPkgChatViewHookBridge {
    pub fn register() {
        static INSTALLED: AtomicBool = AtomicBool::new(false);
        if INSTALLED.swap(true, Ordering::SeqCst) {
            return;
        }
        let manager = toolPkgPackageManager();
        manager.addToolPkgRuntimeChangeListener(std::sync::Arc::new(|activeContainers| {
            ToolPkgChatViewHookBridge::syncAndReplayToolPkgRegistrations(activeContainers);
        }));
    }

    #[allow(non_snake_case)]
    pub fn onEvent(event: ChatViewEvent, params: ChatViewHookParams) {
        updateReplayableOpenViewParams(&event, &params);
        let activeHooks = CHAT_VIEW_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg chat view hook mutex poisoned")
            .clone();
        if activeHooks.is_empty() {
            return;
        }

        ChainLogger::info(
            PLUGIN_CHAIN,
            "plugin.toolpkg.chat_view.scan",
            &[
                ("event", event.wireName().to_string()),
                ("chatId", params.chatId.clone()),
                ("viewId", params.viewId.clone()),
                ("hookCount", activeHooks.len().to_string()),
            ],
        );
        let eventPayload = buildChatViewEventPayload(&params);
        for hook in activeHooks {
            runChatViewHook(&hook, event.wireName(), eventPayload.clone());
        }
    }

    #[allow(non_snake_case)]
    pub fn syncAndReplayToolPkgRegistrations(activeContainers: Vec<ToolPkgContainerRuntime>) {
        let previousHooks = CHAT_VIEW_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg chat view hook mutex poisoned")
            .clone();
        let mut nextHooks = activeContainers
            .iter()
            .flat_map(|runtime| {
                runtime
                    .chatViewHooks
                    .iter()
                    .map(|hook| ToolPkgChatViewHookRegistration {
                        containerPackageName: runtime.packageName.clone(),
                        hookId: hook.id.clone(),
                        functionName: hook.function.clone(),
                        functionSource: hook.functionSource.clone(),
                    })
            })
            .collect::<Vec<_>>();
        nextHooks.sort_by(|left, right| {
            left.containerPackageName
                .cmp(&right.containerPackageName)
                .then(left.hookId.cmp(&right.hookId))
        });
        *CHAT_VIEW_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg chat view hook mutex poisoned") = nextHooks.clone();

        let hooksToReplay = nextHooks
            .into_iter()
            .filter(|hook| {
                !previousHooks
                    .iter()
                    .any(|previous| sameHook(previous, hook))
            })
            .collect::<Vec<_>>();
        if hooksToReplay.is_empty() {
            return;
        }
        let replayParams = REPLAYABLE_OPEN_VIEW_PARAMS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg chat view replay mutex poisoned")
            .clone();
        if replayParams.is_empty() {
            return;
        }
        replayOpenViews(hooksToReplay, replayParams);
    }
}

#[allow(non_snake_case)]
fn updateReplayableOpenViewParams(event: &ChatViewEvent, params: &ChatViewHookParams) {
    let mut replayParams = REPLAYABLE_OPEN_VIEW_PARAMS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .expect("toolpkg chat view replay mutex poisoned");
    match event {
        ChatViewEvent::ViewOpened | ChatViewEvent::ViewFocused => {
            replayParams.retain(|item| item.viewId != params.viewId);
            replayParams.push(params.clone());
        }
        ChatViewEvent::ViewClosed => {
            replayParams.retain(|item| item.viewId != params.viewId);
        }
        ChatViewEvent::ViewBlurred => {}
    }
}

#[allow(non_snake_case)]
fn replayOpenViews(
    hooksToReplay: Vec<ToolPkgChatViewHookRegistration>,
    replayParams: Vec<ChatViewHookParams>,
) {
    for params in replayParams {
        let eventPayload = buildChatViewEventPayload(&params);
        for hook in &hooksToReplay {
            runChatViewHook(
                hook,
                ChatViewEvent::ViewOpened.wireName(),
                eventPayload.clone(),
            );
        }
    }
}

#[allow(non_snake_case)]
fn runChatViewHook(hook: &ToolPkgChatViewHookRegistration, eventName: &str, eventPayload: Value) {
    let manager = toolPkgPackageManager();
    ChainLogger::info(
        PLUGIN_CHAIN,
        "plugin.toolpkg.chat_view.run.start",
        &[
            ("event", eventName.to_string()),
            ("package", hook.containerPackageName.clone()),
            ("hookId", hook.hookId.clone()),
            ("function", hook.functionName.clone()),
        ],
    );
    match manager.runToolPkgMainHook(
        &hook.containerPackageName,
        &hook.functionName,
        TOOLPKG_EVENT_CHAT_VIEW,
        Some(eventName),
        Some(&hook.hookId),
        hook.functionSource.as_deref(),
        eventPayload,
        None,
        None,
        None,
    ) {
        Ok(_) => ChainLogger::info(
            PLUGIN_CHAIN,
            "plugin.toolpkg.chat_view.run.done",
            &[
                ("event", eventName.to_string()),
                ("package", hook.containerPackageName.clone()),
                ("hookId", hook.hookId.clone()),
            ],
        ),
        Err(error) => ChainLogger::error(
            PLUGIN_CHAIN,
            "plugin.toolpkg.chat_view.run.error",
            &[
                ("event", eventName.to_string()),
                ("package", hook.containerPackageName.clone()),
                ("hookId", hook.hookId.clone()),
                ("function", hook.functionName.clone()),
                ("error", error),
            ],
        ),
    }
}

#[allow(non_snake_case)]
fn buildChatViewEventPayload(params: &ChatViewHookParams) -> Value {
    serde_json::json!({
        "viewId": params.viewId,
        "chatId": params.chatId,
        "workspacePath": params.workspacePath,
        "workspaceEnv": params.workspaceEnv,
        "runtime": params.runtime,
        "title": params.title,
    })
}

#[allow(non_snake_case)]
fn sameHook(
    left: &ToolPkgChatViewHookRegistration,
    right: &ToolPkgChatViewHookRegistration,
) -> bool {
    left.containerPackageName == right.containerPackageName
        && left.hookId == right.hookId
        && left.functionName == right.functionName
        && left.functionSource == right.functionSource
}
