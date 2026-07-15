use std::sync::{Arc, Mutex, OnceLock};

use operit_host_api::{
    HostRuntimeEventSchedule, HostRuntimeEventScheduleFire, HostRuntimeEventScheduleKind,
};
use serde_json::Value;

use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_HOST_EVENT;
use operit_plugin_sdk::toolpkg::ToolPkgHooks::ToolPkgHostEventRegistration;
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;
use operit_util::ChainLogger::{self, PLUGIN_CHAIN};

static HOST_EVENT_HOOKS: OnceLock<Mutex<Vec<ToolPkgHostEventRegistration>>> = OnceLock::new();

pub struct ToolPkgHostEventHookBridge;

impl ToolPkgHostEventHookBridge {
    /// Registers host event hook support for one application runtime.
    pub fn register(_runtime: ToolPkgBridgeRuntime) {}

    #[allow(non_snake_case)]
    pub fn syncToolPkgRegistrations(
        runtime: &ToolPkgBridgeRuntime,
        activeContainers: Vec<ToolPkgContainerRuntime>,
    ) {
        let hooks = activeContainers
            .iter()
            .flat_map(|container| {
                container
                    .hostEventHooks
                    .iter()
                    .map(|hook| ToolPkgHostEventRegistration {
                        containerPackageName: container.packageName.clone(),
                        hookId: hook.id.clone(),
                        source: hook.source.clone(),
                        trigger: hook.trigger.clone(),
                        functionName: hook.function.clone(),
                        functionSource: hook.functionSource.clone(),
                        enabled: hook.enabled,
                    })
            })
            .collect::<Vec<_>>();
        let hookCount = hooks.len();
        *HOST_EVENT_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg host event hook mutex poisoned") = hooks;
        ChainLogger::info(
            PLUGIN_CHAIN,
            "plugin.toolpkg.host_event.sync",
            &[("hookCount", hookCount.to_string())],
        );
        syncHostEventSchedules(runtime);
    }

    /// Dispatch a host event to all matching ToolPkg hooks.
    ///
    /// `source` identifies the event origin, e.g. `"broadcast"`, `"timer"`, `"interval"`.
    ///
    /// `eventPayload` is the JSON payload delivered to the handler function.
    #[allow(non_snake_case)]
    pub fn dispatchHostEvent(runtime: &ToolPkgBridgeRuntime, source: &str, eventPayload: Value) {
        let hooks = HOST_EVENT_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg host event hook mutex poisoned")
            .clone();

        ChainLogger::info(
            PLUGIN_CHAIN,
            "plugin.toolpkg.host_event.dispatch",
            &[
                ("source", source.to_string()),
                ("hooks", hooks.len().to_string()),
            ],
        );

        let manager = runtime.package_manager();
        for hook in hooks {
            if !hook.enabled {
                continue;
            }
            if hook.source != source {
                continue;
            }
            if !hostEventHookMatchesPayload(&hook, &eventPayload) {
                continue;
            }
            ChainLogger::info(
                PLUGIN_CHAIN,
                "plugin.toolpkg.host_event.run.start",
                &[
                    ("source", source.to_string()),
                    ("package", hook.containerPackageName.clone()),
                    ("hookId", hook.hookId.clone()),
                    ("function", hook.functionName.clone()),
                ],
            );
            let payload = serde_json::json!({
                "eventSource": source,
                "hookId": hook.hookId,
                "trigger": hook.trigger,
                "payload": eventPayload,
            });
            match manager.runToolPkgMainHook(
                &hook.containerPackageName,
                &hook.functionName,
                TOOLPKG_EVENT_HOST_EVENT,
                Some("host_event"),
                Some(&hook.hookId),
                hook.functionSource.as_deref(),
                payload,
                None,
                None,
                None,
            ) {
                Ok(_) => ChainLogger::info(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.host_event.run.done",
                    &[
                        ("source", source.to_string()),
                        ("package", hook.containerPackageName.clone()),
                        ("hookId", hook.hookId.clone()),
                    ],
                ),
                Err(error) => ChainLogger::error(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.host_event.run.error",
                    &[
                        ("source", source.to_string()),
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

/// Tests whether one hook accepts the supplied target, topic, and platform.
#[allow(non_snake_case)]
fn hostEventHookMatchesPayload(hook: &ToolPkgHostEventRegistration, payload: &Value) -> bool {
    if let Some(targetPackageName) = payload.get("containerPackageName").and_then(Value::as_str) {
        if targetPackageName != hook.containerPackageName {
            return false;
        }
    }
    if let Some(targetHookId) = payload.get("hookId").and_then(Value::as_str) {
        return targetHookId == hook.hookId;
    }
    if !hostEventTriggerMatchesString(
        &hook.trigger,
        payload.get("topic").and_then(Value::as_str),
        "topic",
        "topics",
    ) {
        return false;
    }
    hostEventTriggerMatchesString(
        &hook.trigger,
        payload.get("platform").and_then(Value::as_str),
        "platform",
        "platforms",
    )
}

/// Reconciles all active timer and interval hooks with the platform scheduler.
#[allow(non_snake_case)]
fn syncHostEventSchedules(runtime: &ToolPkgBridgeRuntime) {
    let hooks = HOST_EVENT_HOOKS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .expect("toolpkg host event hook mutex poisoned")
        .clone();

    let schedules = hooks
        .iter()
        .filter(|hook| hook.enabled)
        .filter_map(hostEventSchedule)
        .collect::<Vec<_>>();
    let hostManager = runtime.host_manager();
    let Some(scheduler) = hostManager.hostRuntimeEventSchedulerHost else {
        if !schedules.is_empty() {
            ChainLogger::error(
                PLUGIN_CHAIN,
                "plugin.toolpkg.host_event.schedule.error",
                &[(
                    "error",
                    "host runtime event scheduler is not installed".to_string(),
                )],
            );
        }
        return;
    };
    let callbackRuntime = runtime.clone();
    if let Err(error) = scheduler.replaceHostRuntimeEventSchedules(
        schedules,
        Arc::new(move |fire| dispatchScheduledHostEvent(&callbackRuntime, fire)),
    ) {
        ChainLogger::error(
            PLUGIN_CHAIN,
            "plugin.toolpkg.host_event.schedule.error",
            &[("error", error.to_string())],
        );
    }
}

/// Converts one enabled timer or interval hook into a host-owned schedule.
#[allow(non_snake_case)]
fn hostEventSchedule(hook: &ToolPkgHostEventRegistration) -> Option<HostRuntimeEventSchedule> {
    let (kind, delayMs, intervalMs) = match hook.source.as_str() {
        "timer" => (
            HostRuntimeEventScheduleKind::Timer,
            hostEventTriggerDelayMs(hook, "timer", "delayMs")?,
            None,
        ),
        "interval" => {
            let intervalMs = hostEventTriggerDelayMs(hook, "interval", "intervalMs")?;
            (
                HostRuntimeEventScheduleKind::Interval,
                intervalMs,
                Some(intervalMs),
            )
        }
        _ => return None,
    };
    Some(HostRuntimeEventSchedule {
        scheduleId: hostEventScheduleId(hook),
        containerPackageName: hook.containerPackageName.clone(),
        hookId: hook.hookId.clone(),
        kind,
        delayMs,
        intervalMs,
    })
}

/// Builds the stable opaque identity shared with platform schedulers.
#[allow(non_snake_case)]
fn hostEventScheduleId(hook: &ToolPkgHostEventRegistration) -> String {
    serde_json::to_string(&[
        hook.containerPackageName.as_str(),
        hook.source.as_str(),
        hook.hookId.as_str(),
    ])
    .expect("host event schedule identity must serialize")
}

/// Dispatches one platform scheduler firing to its exact ToolPkg hook.
#[allow(non_snake_case)]
fn dispatchScheduledHostEvent(runtime: &ToolPkgBridgeRuntime, fire: HostRuntimeEventScheduleFire) {
    let hook = HOST_EVENT_HOOKS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .expect("toolpkg host event hook mutex poisoned")
        .iter()
        .find(|hook| hook.enabled && hostEventScheduleId(hook) == fire.scheduleId)
        .cloned();
    let Some(hook) = hook else {
        ChainLogger::error(
            PLUGIN_CHAIN,
            "plugin.toolpkg.host_event.schedule.fire.error",
            &[("scheduleId", fire.scheduleId)],
        );
        return;
    };
    let payload = scheduledHostEventPayload(&hook, &fire);
    ToolPkgHostEventHookBridge::dispatchHostEvent(runtime, &hook.source, payload);
}

/// Reads and validates one positive scheduling duration from a hook trigger.
#[allow(non_snake_case)]
fn hostEventTriggerDelayMs(
    hook: &ToolPkgHostEventRegistration,
    expectedKind: &str,
    fieldName: &str,
) -> Option<u64> {
    let Some(trigger) = hook.trigger.as_object() else {
        ChainLogger::error(
            PLUGIN_CHAIN,
            "plugin.toolpkg.host_event.schedule.error",
            &[
                ("hookId", hook.hookId.clone()),
                ("error", "trigger must be an object".to_string()),
            ],
        );
        return None;
    };
    let kind = trigger.get("kind").and_then(Value::as_str);
    if kind != Some(expectedKind) {
        ChainLogger::error(
            PLUGIN_CHAIN,
            "plugin.toolpkg.host_event.schedule.error",
            &[
                ("hookId", hook.hookId.clone()),
                ("error", format!("trigger.kind must be {expectedKind}")),
            ],
        );
        return None;
    }
    let Some(value) = trigger.get(fieldName).and_then(Value::as_u64) else {
        ChainLogger::error(
            PLUGIN_CHAIN,
            "plugin.toolpkg.host_event.schedule.error",
            &[
                ("hookId", hook.hookId.clone()),
                (
                    "error",
                    format!("trigger.{fieldName} must be a positive integer"),
                ),
            ],
        );
        return None;
    };
    if value == 0 {
        ChainLogger::error(
            PLUGIN_CHAIN,
            "plugin.toolpkg.host_event.schedule.error",
            &[
                ("hookId", hook.hookId.clone()),
                (
                    "error",
                    format!("trigger.{fieldName} must be a positive integer"),
                ),
            ],
        );
        return None;
    }
    Some(value)
}

/// Builds the callback payload for one platform-owned timer or interval firing.
#[allow(non_snake_case)]
fn scheduledHostEventPayload(
    hook: &ToolPkgHostEventRegistration,
    fire: &HostRuntimeEventScheduleFire,
) -> Value {
    let mut payload = serde_json::json!({
        "containerPackageName": hook.containerPackageName,
        "hookId": hook.hookId,
        "source": hook.source,
        "trigger": hook.trigger,
        "scheduledAtMillis": fire.scheduledAtMillis,
        "firedAtMillis": fire.firedAtMillis,
    });
    if hook.source == "timer" {
        payload["delayMs"] = hook.trigger["delayMs"].clone();
    }
    if hook.source == "interval" {
        payload["intervalMs"] = hook.trigger["intervalMs"].clone();
    }
    insertTriggerPayload(&mut payload, &hook.trigger);
    payload
}

/// Copies the plugin-defined trigger payload into a scheduled event payload.
#[allow(non_snake_case)]
fn insertTriggerPayload(payload: &mut Value, trigger: &Value) {
    if let Some(value) = trigger.get("payload") {
        payload["payload"] = value.clone();
    }
}

/// Matches a scalar or array trigger selector against an optional event value.
#[allow(non_snake_case)]
fn hostEventTriggerMatchesString(
    trigger: &Value,
    value: Option<&str>,
    singleField: &str,
    arrayField: &str,
) -> bool {
    let Some(object) = trigger.as_object() else {
        return true;
    };
    if let Some(expected) = object.get(singleField).and_then(Value::as_str) {
        return value == Some(expected);
    }
    if let Some(values) = object.get(arrayField).and_then(Value::as_array) {
        return value.is_some_and(|value| {
            values
                .iter()
                .filter_map(Value::as_str)
                .any(|expected| expected == value)
        });
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies broadcast hooks require both a selected topic and selected host platform.
    #[test]
    fn matches_broadcast_topic_and_platform_filters() {
        let hook = ToolPkgHostEventRegistration {
            containerPackageName: "example".to_string(),
            hookId: "network".to_string(),
            source: "broadcast".to_string(),
            trigger: serde_json::json!({
                "kind": "broadcast",
                "topic": "system.network.changed",
                "platforms": ["ios", "macos", "web"],
            }),
            functionName: "onNetworkChanged".to_string(),
            functionSource: None,
            enabled: true,
        };
        assert!(hostEventHookMatchesPayload(
            &hook,
            &serde_json::json!({
                "topic": "system.network.changed",
                "platform": "ios",
            }),
        ));
        assert!(!hostEventHookMatchesPayload(
            &hook,
            &serde_json::json!({
                "topic": "system.network.changed",
                "platform": "android",
            }),
        ));
        assert!(!hostEventHookMatchesPayload(
            &hook,
            &serde_json::json!({
                "topic": "system.power.connected",
                "platform": "ios",
            }),
        ));
    }

    /// Verifies platform schedules retain exact package identity and timing semantics.
    #[test]
    fn builds_exact_platform_schedule_and_payload() {
        let hook = ToolPkgHostEventRegistration {
            containerPackageName: "package.example".to_string(),
            hookId: "refresh".to_string(),
            source: "interval".to_string(),
            trigger: serde_json::json!({
                "kind": "interval",
                "intervalMs": 60000,
                "payload": {"scope": "inbox"},
            }),
            functionName: "onRefresh".to_string(),
            functionSource: None,
            enabled: true,
        };
        let schedule = hostEventSchedule(&hook).expect("valid interval must schedule");
        assert_eq!(schedule.containerPackageName, "package.example");
        assert_eq!(schedule.hookId, "refresh");
        assert_eq!(schedule.delayMs, 60000);
        assert_eq!(schedule.intervalMs, Some(60000));

        let payload = scheduledHostEventPayload(
            &hook,
            &HostRuntimeEventScheduleFire {
                scheduleId: schedule.scheduleId,
                scheduledAtMillis: 100,
                firedAtMillis: 125,
            },
        );
        assert_eq!(payload["containerPackageName"], "package.example");
        assert_eq!(payload["hookId"], "refresh");
        assert_eq!(payload["scheduledAtMillis"], 100);
        assert_eq!(payload["firedAtMillis"], 125);
        assert_eq!(payload["payload"], serde_json::json!({"scope": "inbox"}));
    }

    /// Verifies targeted schedule events cannot cross package boundaries.
    #[test]
    fn rejects_targeted_schedule_for_another_package() {
        let hook = ToolPkgHostEventRegistration {
            containerPackageName: "package.one".to_string(),
            hookId: "same-id".to_string(),
            source: "timer".to_string(),
            trigger: serde_json::json!({"kind": "timer", "delayMs": 1000}),
            functionName: "run".to_string(),
            functionSource: None,
            enabled: true,
        };
        assert!(!hostEventHookMatchesPayload(
            &hook,
            &serde_json::json!({
                "containerPackageName": "package.two",
                "hookId": "same-id",
            }),
        ));
    }
}
