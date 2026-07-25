use std::sync::{Mutex, OnceLock};

use operit_plugin_sdk::toolpkg::ToolPkgHooks::ToolPkgAppLifecycleHookRegistration;
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;
use operit_util::ChainLogger::{self, PLUGIN_CHAIN};
use serde_json::Value;

use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;

static APP_LIFECYCLE_HOOKS: OnceLock<Mutex<Vec<ToolPkgAppLifecycleHookRegistration>>> =
    OnceLock::new();
static REPLAYABLE_APP_EVENTS: OnceLock<Mutex<Vec<AppLifecycleReplayEvent>>> = OnceLock::new();

#[derive(Clone, Debug)]
struct AppLifecycleReplayEvent {
    eventName: String,
    payload: Value,
}

pub struct ToolPkgAppLifecycleHookBridge;

impl ToolPkgAppLifecycleHookBridge {
    /// Registers app lifecycle hooks for one application runtime.
    pub fn register(runtime: ToolPkgBridgeRuntime) {
        let manager = runtime.package_manager();
        manager.addToolPkgRuntimeChangeListener(std::sync::Arc::new(move |activeContainers| {
            ToolPkgAppLifecycleHookBridge::syncAndReplayToolPkgRegistrations(
                &runtime,
                activeContainers,
            );
        }));
    }

    /// Synchronizes app lifecycle hooks and replays application events to newly added hooks.
    #[allow(non_snake_case)]
    pub fn syncAndReplayToolPkgRegistrations(
        runtime: &ToolPkgBridgeRuntime,
        activeContainers: Vec<ToolPkgContainerRuntime>,
    ) {
        let previousHooks = APP_LIFECYCLE_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg app lifecycle hook mutex poisoned")
            .clone();
        let mut nextHooks = activeContainers
            .iter()
            .flat_map(|container| {
                container.appLifecycleHooks.iter().map(|hook| {
                    ToolPkgAppLifecycleHookRegistration {
                        containerPackageName: container.packageName.clone(),
                        hookId: hook.id.clone(),
                        event: hook.event.clone(),
                        functionName: hook.function.clone(),
                        functionSource: hook.functionSource.clone(),
                    }
                })
            })
            .collect::<Vec<_>>();
        nextHooks.sort_by(|left, right| {
            left.event
                .cmp(&right.event)
                .then(left.containerPackageName.cmp(&right.containerPackageName))
                .then(left.hookId.cmp(&right.hookId))
        });
        *APP_LIFECYCLE_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg app lifecycle hook mutex poisoned") = nextHooks.clone();

        let hooksToReplay = nextHooks
            .into_iter()
            .filter(|hook| {
                !previousHooks
                    .iter()
                    .any(|previous| sameLifecycleHook(previous, hook))
            })
            .collect::<Vec<_>>();
        if hooksToReplay.is_empty() {
            return;
        }
        let replayEvents = REPLAYABLE_APP_EVENTS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg app lifecycle replay mutex poisoned")
            .clone();
        for replayEvent in replayEvents {
            for hook in hooksToReplay
                .iter()
                .filter(|hook| hook.event == replayEvent.eventName)
            {
                runAppLifecycleHook(runtime, hook, &replayEvent.eventName, replayEvent.payload.clone());
            }
        }
    }

    /// Dispatches an app lifecycle event to matching ToolPkg hooks.
    #[allow(non_snake_case)]
    pub fn dispatchEvent(runtime: &ToolPkgBridgeRuntime, eventName: &str, eventPayload: Value) {
        rememberReplayableEvent(eventName, eventPayload.clone());
        let activeHooks = APP_LIFECYCLE_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg app lifecycle hook mutex poisoned")
            .clone();
        let matchingHooks = activeHooks
            .into_iter()
            .filter(|hook| hook.event == eventName)
            .collect::<Vec<_>>();
        if matchingHooks.is_empty() {
            return;
        }
        ChainLogger::info(
            PLUGIN_CHAIN,
            "plugin.toolpkg.app_lifecycle.scan",
            &[
                ("event", eventName.to_string()),
                ("hookCount", matchingHooks.len().to_string()),
            ],
        );
        for hook in matchingHooks {
            runAppLifecycleHook(runtime, &hook, eventName, eventPayload.clone());
        }
    }
}

/// Stores application-level lifecycle events that newly registered hooks should receive.
#[allow(non_snake_case)]
fn rememberReplayableEvent(eventName: &str, eventPayload: Value) {
    match eventName {
        operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_APPLICATION_ON_CREATE
        | operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_APPLICATION_ON_FOREGROUND => {
            let mut events = REPLAYABLE_APP_EVENTS
                .get_or_init(|| Mutex::new(Vec::new()))
                .lock()
                .expect("toolpkg app lifecycle replay mutex poisoned");
            events.retain(|event| event.eventName != eventName);
            events.push(AppLifecycleReplayEvent {
                eventName: eventName.to_string(),
                payload: eventPayload,
            });
        }
        _ => {}
    }
}

/// Invokes one app lifecycle hook and records plugin execution status.
#[allow(non_snake_case)]
fn runAppLifecycleHook(
    runtime: &ToolPkgBridgeRuntime,
    hook: &ToolPkgAppLifecycleHookRegistration,
    eventName: &str,
    eventPayload: Value,
) {
    let manager = runtime.package_manager();
    ChainLogger::info(
        PLUGIN_CHAIN,
        "plugin.toolpkg.app_lifecycle.run.start",
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
        eventName,
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
            "plugin.toolpkg.app_lifecycle.run.done",
            &[
                ("event", eventName.to_string()),
                ("package", hook.containerPackageName.clone()),
                ("hookId", hook.hookId.clone()),
            ],
        ),
        Err(error) => ChainLogger::error(
            PLUGIN_CHAIN,
            "plugin.toolpkg.app_lifecycle.run.error",
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

/// Compares two app lifecycle hook registrations by their stable dispatch identity.
#[allow(non_snake_case)]
fn sameLifecycleHook(
    left: &ToolPkgAppLifecycleHookRegistration,
    right: &ToolPkgAppLifecycleHookRegistration,
) -> bool {
    left.containerPackageName == right.containerPackageName
        && left.hookId == right.hookId
        && left.event == right.event
        && left.functionName == right.functionName
        && left.functionSource == right.functionSource
}
