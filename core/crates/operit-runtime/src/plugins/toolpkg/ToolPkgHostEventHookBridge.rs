use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex, OnceLock,
};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::core::tools::packTool::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_HOST_EVENT;
use crate::core::tools::packTool::ToolPkgParser::ToolPkgContainerRuntime;
use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::{
    toolPkgPackageManager, ToolPkgHostEventRegistration,
};
use crate::util::ChainLogger::{self, PLUGIN_CHAIN};

static HOST_EVENT_HOOKS: OnceLock<Mutex<Vec<ToolPkgHostEventRegistration>>> = OnceLock::new();
static HOST_EVENT_SCHEDULE_GENERATION: OnceLock<Arc<AtomicU64>> = OnceLock::new();

pub struct ToolPkgHostEventHookBridge;

impl ToolPkgHostEventHookBridge {
    pub fn register() {
    }

    #[allow(non_snake_case)]
    pub fn syncToolPkgRegistrations(activeContainers: Vec<ToolPkgContainerRuntime>) {
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
        syncHostEventSchedules();
    }

    /// Dispatch a host event to all matching ToolPkg hooks.
    ///
    /// `source` identifies the event origin, e.g. `"broadcast"`, `"timer"`, `"interval"`.
    ///
    /// `eventPayload` is the JSON payload delivered to the handler function.
    #[allow(non_snake_case)]
    pub fn dispatchHostEvent(source: &str, eventPayload: Value) {
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

        let manager = toolPkgPackageManager();
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

#[allow(non_snake_case)]
fn hostEventHookMatchesPayload(hook: &ToolPkgHostEventRegistration, payload: &Value) -> bool {
    if let Some(targetHookId) = payload.get("hookId").and_then(Value::as_str) {
        return targetHookId == hook.hookId;
    }
    if let Some(topic) = payload.get("topic").and_then(Value::as_str) {
        return hostEventTriggerMatchesString(&hook.trigger, topic, "topic", "topics");
    }
    true
}

#[allow(non_snake_case)]
fn syncHostEventSchedules() {
    let generation = hostEventScheduleGeneration();
    let currentGeneration = generation.fetch_add(1, Ordering::SeqCst) + 1;
    let hooks = HOST_EVENT_HOOKS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .expect("toolpkg host event hook mutex poisoned")
        .clone();

    for hook in hooks {
        if !hook.enabled {
            continue;
        }
        match hook.source.as_str() {
            "timer" => scheduleTimerHostEvent(hook, currentGeneration, Arc::clone(&generation)),
            "interval" => scheduleIntervalHostEvent(hook, currentGeneration, Arc::clone(&generation)),
            _ => {}
        }
    }
}

#[allow(non_snake_case)]
fn scheduleTimerHostEvent(
    hook: ToolPkgHostEventRegistration,
    generation: u64,
    generationState: Arc<AtomicU64>,
) {
    let Some(delayMs) = hostEventTriggerDelayMs(&hook, "timer", "delayMs") else {
        return;
    };
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(delayMs));
        if generationState.load(Ordering::SeqCst) != generation {
            return;
        }
        ToolPkgHostEventHookBridge::dispatchHostEvent(
            "timer",
            timerHostEventPayload(&hook, delayMs),
        );
    });
}

#[allow(non_snake_case)]
fn scheduleIntervalHostEvent(
    hook: ToolPkgHostEventRegistration,
    generation: u64,
    generationState: Arc<AtomicU64>,
) {
    let Some(intervalMs) = hostEventTriggerDelayMs(&hook, "interval", "intervalMs") else {
        return;
    };
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(intervalMs));
            if generationState.load(Ordering::SeqCst) != generation {
                return;
            }
            ToolPkgHostEventHookBridge::dispatchHostEvent(
                "interval",
                intervalHostEventPayload(&hook, intervalMs),
            );
        }
    });
}

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
                ("error", format!("trigger.{fieldName} must be a positive integer")),
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
                ("error", format!("trigger.{fieldName} must be a positive integer")),
            ],
        );
        return None;
    }
    Some(value)
}

#[allow(non_snake_case)]
fn timerHostEventPayload(hook: &ToolPkgHostEventRegistration, delayMs: u64) -> Value {
    let mut payload = serde_json::json!({
        "hookId": hook.hookId,
        "source": "timer",
        "trigger": hook.trigger,
        "scheduledAtMillis": currentTimeMillis(),
        "delayMs": delayMs,
    });
    insertTriggerPayload(&mut payload, &hook.trigger);
    payload
}

#[allow(non_snake_case)]
fn intervalHostEventPayload(hook: &ToolPkgHostEventRegistration, intervalMs: u64) -> Value {
    let mut payload = serde_json::json!({
        "hookId": hook.hookId,
        "source": "interval",
        "trigger": hook.trigger,
        "scheduledAtMillis": currentTimeMillis(),
        "intervalMs": intervalMs,
    });
    insertTriggerPayload(&mut payload, &hook.trigger);
    payload
}

#[allow(non_snake_case)]
fn insertTriggerPayload(payload: &mut Value, trigger: &Value) {
    if let Some(value) = trigger.get("payload") {
        payload["payload"] = value.clone();
    }
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before unix epoch")
        .as_millis() as u64
}

#[allow(non_snake_case)]
fn hostEventScheduleGeneration() -> Arc<AtomicU64> {
    HOST_EVENT_SCHEDULE_GENERATION
        .get_or_init(|| Arc::new(AtomicU64::new(0)))
        .clone()
}

#[allow(non_snake_case)]
fn hostEventTriggerMatchesString(
    trigger: &Value,
    value: &str,
    singleField: &str,
    arrayField: &str,
) -> bool {
    let Some(object) = trigger.as_object() else {
        return true;
    };
    if let Some(expected) = object.get(singleField).and_then(Value::as_str) {
        return expected == value;
    }
    if let Some(values) = object.get(arrayField).and_then(Value::as_array) {
        return values
            .iter()
            .filter_map(Value::as_str)
            .any(|expected| expected == value);
    }
    true
}
