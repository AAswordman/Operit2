use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use serde_json::Value;

use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use operit_plugin_sdk::javascript::JsExecutionEngine;
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_CHAT_VIEW;
use operit_plugin_sdk::toolpkg::ToolPkgHooks::ToolPkgChatViewHookRegistration;
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;
use operit_util::ChainLogger::{self, PLUGIN_CHAIN};

static CHAT_VIEW_HOOKS: OnceLock<Mutex<Vec<ToolPkgChatViewHookRegistration>>> = OnceLock::new();
static REPLAYABLE_OPEN_VIEW_PARAMS: OnceLock<Mutex<Vec<ChatViewHookParams>>> = OnceLock::new();
static CHAT_VIEW_RUNTIME: OnceLock<ToolPkgBridgeRuntime> = OnceLock::new();
// Track the execution instance that received each hook, not just its registration.
// Weak references neither keep destroyed JS runtimes alive nor confuse replacements
// with an old allocation whose address may later be reused.
type HookExecutionEngines = BTreeMap<(String, String), Weak<dyn JsExecutionEngine>>;
static CHAT_VIEW_HOOK_ENGINES: OnceLock<Mutex<HookExecutionEngines>> = OnceLock::new();

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatViewEvent {
    ViewOpened,
    ViewUpdated,
    ViewClosed,
}

impl ChatViewEvent {
    #[allow(non_snake_case)]
    pub fn wireName(&self) -> &'static str {
        match self {
            ChatViewEvent::ViewOpened => "view_opened",
            ChatViewEvent::ViewUpdated => "view_updated",
            ChatViewEvent::ViewClosed => "view_closed",
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
    /// Registers chat view hooks for one application runtime.
    pub fn register(runtime: ToolPkgBridgeRuntime) {
        CHAT_VIEW_RUNTIME.get_or_init(|| runtime.clone());
    }

    #[allow(non_snake_case)]
    pub fn onEvent(
        runtime: &ToolPkgBridgeRuntime,
        event: ChatViewEvent,
        params: ChatViewHookParams,
    ) {
        updateReplayableOpenViewParams(&event, &params);
        let activeHooks = CHAT_VIEW_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg chat view hook mutex poisoned")
            .clone();
        if activeHooks.is_empty() {
            return;
        }

        let eventPayload = buildChatViewEventPayload(&params);
        for hook in activeHooks {
            runChatViewHook(runtime, &hook, event.wireName(), eventPayload.clone());
        }
    }

    /// Dispatches chat view hooks through the runtime registered by the common bridge.
    #[allow(non_snake_case)]
    pub fn dispatchRegisteredChatViewEvent(event: ChatViewEvent, params: ChatViewHookParams) {
        updateReplayableOpenViewParams(&event, &params);
        let Some(runtime) = CHAT_VIEW_RUNTIME.get() else {
            return;
        };
        Self::onEvent(runtime, event, params);
    }

    /// Synchronizes this bridge from the committed active package catalog.
    #[allow(non_snake_case)]
    pub fn syncAndReplayToolPkgRegistrations(
        runtime: &ToolPkgBridgeRuntime,
        activeContainers: &[ToolPkgContainerRuntime],
    ) {
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

        let previousEngines = {
            let mut engines = CHAT_VIEW_HOOK_ENGINES
                .get_or_init(|| Mutex::new(BTreeMap::new()))
                .lock()
                .expect("toolpkg chat view engine mutex poisoned");
            engines.retain(|key, _| nextHooks.iter().any(|hook| hookKey(hook) == *key));
            engines.clone()
        };
        let manager = runtime.package_manager();
        let hooksToReplay = nextHooks
            .into_iter()
            .filter(|hook| {
                let contextKey = format!("toolpkg_main:{}", hook.containerPackageName);
                let currentEngine =
                    manager.findToolPkgExecutionEngine(&contextKey, &hook.containerPackageName);
                shouldReplayChatViewHook(
                    hook,
                    &previousHooks,
                    previousEngines.get(&hookKey(hook)),
                    currentEngine.as_ref(),
                )
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
        replayOpenViews(runtime, hooksToReplay, replayParams);
    }
}

#[allow(non_snake_case)]
fn updateReplayableOpenViewParams(event: &ChatViewEvent, params: &ChatViewHookParams) {
    let mut replayParams = REPLAYABLE_OPEN_VIEW_PARAMS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .expect("toolpkg chat view replay mutex poisoned");
    match event {
        ChatViewEvent::ViewOpened => {
            replayParams.retain(|item| item.viewId != params.viewId);
            replayParams.push(params.clone());
        }
        ChatViewEvent::ViewUpdated => {
            if replayParams.iter().any(|item| item.viewId == params.viewId) {
                replayParams.retain(|item| item.viewId != params.viewId);
                replayParams.push(params.clone());
            }
        }
        ChatViewEvent::ViewClosed => {
            replayParams.retain(|item| item.viewId != params.viewId);
        }
    }
}

#[allow(non_snake_case)]
fn replayOpenViews(
    runtime: &ToolPkgBridgeRuntime,
    hooksToReplay: Vec<ToolPkgChatViewHookRegistration>,
    replayParams: Vec<ChatViewHookParams>,
) {
    for params in replayParams {
        let eventPayload = buildChatViewEventPayload(&params);
        for hook in &hooksToReplay {
            runChatViewHook(
                runtime,
                hook,
                ChatViewEvent::ViewOpened.wireName(),
                eventPayload.clone(),
            );
        }
    }
}

#[allow(non_snake_case)]
fn runChatViewHook(
    runtime: &ToolPkgBridgeRuntime,
    hook: &ToolPkgChatViewHookRegistration,
    eventName: &str,
    eventPayload: Value,
) {
    let runtime = runtime.to_owned();
    let hook = hook.to_owned();
    let eventName = eventName.to_owned();
    super::ToolPkgHookBridgeSupport::scheduleToolPkgNotification(
        "operit-toolpkg-notification",
        move || {
            Box::pin(async move {
                let runtime = &runtime;
                let hook = &hook;
                let eventName = &eventName;
                let manager = runtime.package_manager();
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
                match manager
                    .runToolPkgMainHook(
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
                    )
                    .await
                {
                    Ok(_) => {
                        let contextKey = format!("toolpkg_main:{}", hook.containerPackageName);
                        if let Some(engine) = manager
                            .findToolPkgExecutionEngine(&contextKey, &hook.containerPackageName)
                        {
                            CHAT_VIEW_HOOK_ENGINES
                                .get_or_init(|| Mutex::new(BTreeMap::new()))
                                .lock()
                                .expect("toolpkg chat view engine mutex poisoned")
                                .insert(hookKey(hook), Arc::downgrade(&engine));
                        }
                        ChainLogger::info(
                            PLUGIN_CHAIN,
                            "plugin.toolpkg.chat_view.run.done",
                            &[
                                ("event", eventName.to_string()),
                                ("package", hook.containerPackageName.clone()),
                                ("hookId", hook.hookId.clone()),
                            ],
                        );
                    }
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
            })
        },
    );
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

#[allow(non_snake_case)]
fn hookKey(hook: &ToolPkgChatViewHookRegistration) -> (String, String) {
    (hook.containerPackageName.clone(), hook.hookId.clone())
}

/// A recreated JS context needs view replay even when its hook declaration is unchanged.
#[allow(non_snake_case)]
fn shouldReplayChatViewHook<T: ?Sized>(
    hook: &ToolPkgChatViewHookRegistration,
    previousHooks: &[ToolPkgChatViewHookRegistration],
    previousEngine: Option<&Weak<T>>,
    currentEngine: Option<&Arc<T>>,
) -> bool {
    if !previousHooks
        .iter()
        .any(|previous| sameHook(previous, hook))
    {
        return true;
    }
    match (previousEngine.and_then(Weak::upgrade), currentEngine) {
        (Some(previous), Some(current)) => !Arc::ptr_eq(&previous, current),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hook() -> ToolPkgChatViewHookRegistration {
        ToolPkgChatViewHookRegistration {
            containerPackageName: "com.operit.plan_mode_bundle".to_string(),
            hookId: "plan_mode_chat_view".to_string(),
            functionName: "onChatViewEvent".to_string(),
            functionSource: None,
        }
    }

    #[test]
    fn unchanged_hook_in_same_execution_instance_does_not_replay() {
        let hook = hook();
        let engine = Arc::new(());
        let weak = Arc::downgrade(&engine);
        assert!(!shouldReplayChatViewHook(
            &hook,
            std::slice::from_ref(&hook),
            Some(&weak),
            Some(&engine.clone()),
        ));
    }

    #[test]
    fn unchanged_hook_replays_when_execution_instance_is_destroyed_or_replaced() {
        let hook = hook();
        let old = Arc::new(());
        let weak = Arc::downgrade(&old);
        assert!(shouldReplayChatViewHook(
            &hook,
            std::slice::from_ref(&hook),
            Some(&weak),
            None,
        ));
        let replacement = Arc::new(());
        assert!(shouldReplayChatViewHook(
            &hook,
            std::slice::from_ref(&hook),
            Some(&weak),
            Some(&replacement),
        ));
        drop(old);
        assert!(shouldReplayChatViewHook(
            &hook,
            std::slice::from_ref(&hook),
            Some(&weak),
            Some(&replacement),
        ));
    }

    #[test]
    fn newly_registered_or_changed_hook_replays_in_existing_instance() {
        let hook = hook();
        let engine = Arc::new(());
        let weak = Arc::downgrade(&engine);
        assert!(shouldReplayChatViewHook(
            &hook,
            &[],
            Some(&weak),
            Some(&engine)
        ));
        let mut previous = hook.clone();
        previous.functionSource = Some("old implementation".to_string());
        assert!(shouldReplayChatViewHook(
            &hook,
            &[previous],
            Some(&weak),
            Some(&engine),
        ));
        assert!(shouldReplayChatViewHook(
            &hook,
            std::slice::from_ref(&hook),
            None,
            Some(&engine),
        ));
    }
}
