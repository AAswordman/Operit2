#![allow(non_snake_case)]

use std::sync::Arc;

use operit_host_api::HostRuntimeEventRegistration;
use serde_json::Value;

use crate::core::events::RuntimeEvent::{RuntimeEvent, RuntimeEventTopic};
use crate::plugins::toolpkg::ToolPkgAppLifecycleHookBridge::ToolPkgAppLifecycleHookBridge;
use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use crate::plugins::toolpkg::ToolPkgHostEventHookBridge::ToolPkgHostEventHookBridge;
use operit_host_api::HostManager::HostManager;
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::{
    TOOLPKG_EVENT_ACTIVITY_ON_CREATE, TOOLPKG_EVENT_ACTIVITY_ON_DESTROY,
    TOOLPKG_EVENT_ACTIVITY_ON_PAUSE, TOOLPKG_EVENT_ACTIVITY_ON_RESUME,
    TOOLPKG_EVENT_ACTIVITY_ON_START, TOOLPKG_EVENT_ACTIVITY_ON_STOP,
    TOOLPKG_EVENT_APPLICATION_ON_BACKGROUND, TOOLPKG_EVENT_APPLICATION_ON_FOREGROUND,
    TOOLPKG_EVENT_APPLICATION_ON_LOW_MEMORY, TOOLPKG_EVENT_APPLICATION_ON_TERMINATE,
    TOOLPKG_EVENT_APPLICATION_ON_TRIM_MEMORY,
};

pub struct RuntimeEventIngressService {
    toolpkg_runtime: ToolPkgBridgeRuntime,
}

impl RuntimeEventIngressService {
    /// Creates the runtime event ingress service for the supplied host context.
    pub fn getInstance(_context: &HostManager, toolpkg_runtime: ToolPkgBridgeRuntime) -> Self {
        Self { toolpkg_runtime }
    }

    /// Starts host runtime event forwarding for one application runtime.
    pub(crate) fn startHostRuntimeEventSupport(
        context: HostManager,
        toolpkg_runtime: ToolPkgBridgeRuntime,
    ) -> Result<Option<Box<dyn HostRuntimeEventRegistration>>, String> {
        let Some(host) = context.hostRuntimeEventHost.clone() else {
            return Ok(None);
        };
        let handlerContext = context.clone();
        let handlerToolPkgRuntime = toolpkg_runtime.clone();
        let registration = host
            .startHostRuntimeEventStream(Arc::new(move |eventValue| {
                match serde_json::from_value::<RuntimeEvent>(eventValue) {
                    Ok(event) => {
                        let service = RuntimeEventIngressService::getInstance(
                            &handlerContext,
                            handlerToolPkgRuntime.clone(),
                        );
                        let _ = service.ingestEvent(event);
                    }
                    Err(error) => {
                        operit_util::AppLogger::AppLogger::e(
                            "RuntimeEventIngress",
                            &format!("invalid host runtime event: {error}"),
                        );
                    }
                }
            }))
            .map_err(|error| error.to_string())?;
        Ok(Some(registration))
    }

    /// Dispatches one runtime event into registered tool package host-event hooks.
    pub fn ingestEvent(&self, event: RuntimeEvent) -> Value {
        let payload = match event.hostEventPayload() {
            Ok(payload) => payload,
            Err(error) => {
                operit_util::AppLogger::AppLogger::e("RuntimeEventIngress", &error);
                return serde_json::json!({
                    "ok": false,
                    "error": error,
                });
            }
        };
        dispatchAppLifecycleEvent(&self.toolpkg_runtime, &event, payload.clone());
        ToolPkgHostEventHookBridge::dispatchHostEvent(&self.toolpkg_runtime, "broadcast", payload);
        serde_json::json!({
            "ok": true,
        })
    }
}

/// Dispatches normalized application lifecycle runtime events to ToolPkg app lifecycle hooks.
#[allow(non_snake_case)]
fn dispatchAppLifecycleEvent(
    runtime: &ToolPkgBridgeRuntime,
    event: &RuntimeEvent,
    hostEventPayload: Value,
) {
    let eventName = match event.topic.clone() {
        RuntimeEventTopic::AppLifecycleResumed => TOOLPKG_EVENT_APPLICATION_ON_FOREGROUND,
        RuntimeEventTopic::AppLifecycleInactive
        | RuntimeEventTopic::AppLifecyclePaused
        | RuntimeEventTopic::AppLifecycleHidden => TOOLPKG_EVENT_APPLICATION_ON_BACKGROUND,
        RuntimeEventTopic::AppLifecycleDetached => TOOLPKG_EVENT_APPLICATION_ON_TERMINATE,
        RuntimeEventTopic::AppLifecycleLowMemory => TOOLPKG_EVENT_APPLICATION_ON_LOW_MEMORY,
        RuntimeEventTopic::AppLifecycleTrimMemory => TOOLPKG_EVENT_APPLICATION_ON_TRIM_MEMORY,
        RuntimeEventTopic::ActivityLifecycleCreate => TOOLPKG_EVENT_ACTIVITY_ON_CREATE,
        RuntimeEventTopic::ActivityLifecycleStart => TOOLPKG_EVENT_ACTIVITY_ON_START,
        RuntimeEventTopic::ActivityLifecycleResume => TOOLPKG_EVENT_ACTIVITY_ON_RESUME,
        RuntimeEventTopic::ActivityLifecyclePause => TOOLPKG_EVENT_ACTIVITY_ON_PAUSE,
        RuntimeEventTopic::ActivityLifecycleStop => TOOLPKG_EVENT_ACTIVITY_ON_STOP,
        RuntimeEventTopic::ActivityLifecycleDestroy => TOOLPKG_EVENT_ACTIVITY_ON_DESTROY,
        _ => return,
    };
    ToolPkgAppLifecycleHookBridge::dispatchEvent(
        runtime,
        eventName,
        serde_json::json!({
            "extras": hostEventPayload,
        }),
    );
}
