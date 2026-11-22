#![allow(non_snake_case)]

use std::sync::{Arc, OnceLock};

use operit_host_api::HostRuntimeEventRegistration;
use serde_json::Value;

use crate::core::application::ApplicationUpdate::ApplicationUpdateNetwork;
use crate::core::events::RuntimeEvent::{RuntimeEvent, RuntimeEventTopic};
use crate::plugins::toolpkg::ToolPkgAppLifecycleHookBridge::ToolPkgAppLifecycleHookBridge;
use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use crate::plugins::toolpkg::ToolPkgHostEventHookBridge::ToolPkgHostEventHookBridge;
use crate::plugins::toolpkg::ToolPkgToolPermissionBridge::ToolPkgToolPermissionBridge;
use operit_host_api::HostManager::HostManager;
use operit_plugin_sdk::js_sdk::toolpkg::{
    ToolPkgBroadcastNetworkChangedData, ToolPkgBroadcastNetworkType,
};
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::{
    TOOLPKG_EVENT_ACTIVITY_ON_CREATE, TOOLPKG_EVENT_ACTIVITY_ON_DESTROY,
    TOOLPKG_EVENT_ACTIVITY_ON_PAUSE, TOOLPKG_EVENT_ACTIVITY_ON_RESUME,
    TOOLPKG_EVENT_ACTIVITY_ON_START, TOOLPKG_EVENT_ACTIVITY_ON_STOP,
    TOOLPKG_EVENT_APPLICATION_ON_BACKGROUND, TOOLPKG_EVENT_APPLICATION_ON_FOREGROUND,
    TOOLPKG_EVENT_APPLICATION_ON_LOW_MEMORY, TOOLPKG_EVENT_APPLICATION_ON_TRIM_MEMORY,
};
use operit_store::PreferencesDataStore::StateFlow;

pub struct RuntimeEventIngressService {
    toolpkg_runtime: ToolPkgBridgeRuntime,
}

impl RuntimeEventIngressService {
    /// Observes local connectivity and explicit unmetered Wi-Fi or Ethernet eligibility.
    pub fn updateNetworkFlow() -> StateFlow<ApplicationUpdateNetwork> {
        static NETWORK: OnceLock<StateFlow<ApplicationUpdateNetwork>> = OnceLock::new();
        NETWORK
            .get_or_init(|| StateFlow::new(ApplicationUpdateNetwork::default()))
            .clone()
    }

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
        if event.topic == RuntimeEventTopic::SystemNetworkChanged {
            if let Ok(network) =
                serde_json::from_value::<ToolPkgBroadcastNetworkChangedData>(event.payload.clone())
            {
                Self::updateNetworkFlow().set_value(updateNetwork(&network));
            }
        }
        // Raw platform/Flutter lifecycle is diagnostic data. Only the owner's
        // product visibility changes foreground/background, never blur or detach.
        let lifecycleEvent = appLifecycleEventName(&event.topic, &event.payload);
        match lifecycleEvent {
            Some(TOOLPKG_EVENT_APPLICATION_ON_FOREGROUND) => self
                .toolpkg_runtime
                .tool_handler()
                .runtimeSupport()
                .setScheduledChatForeground(true),
            Some(TOOLPKG_EVENT_APPLICATION_ON_BACKGROUND) => self
                .toolpkg_runtime
                .tool_handler()
                .runtimeSupport()
                .setScheduledChatForeground(false),
            _ => {}
        }
        if let Some(eventName) = lifecycleEvent {
            ToolPkgAppLifecycleHookBridge::dispatchEvent(
                &self.toolpkg_runtime,
                eventName,
                serde_json::json!({ "extras": payload.clone() }),
            );
        }
        if lifecycleEvent == Some(TOOLPKG_EVENT_APPLICATION_ON_FOREGROUND) {
            ToolPkgToolPermissionBridge::notify();
        }
        // Main UI state belongs to registerAppLifecycleHook; do not create a
        // second public subscription for the host's internal normalization.
        if event.topic != RuntimeEventTopic::AppVisibilityChanged {
            ToolPkgHostEventHookBridge::dispatchHostEvent(
                &self.toolpkg_runtime,
                "broadcast",
                payload,
            );
        }
        serde_json::json!({
            "ok": true,
        })
    }
}

/// Retains all connections while identifying those allowing large unattended transfers.
fn updateNetwork(network: &ToolPkgBroadcastNetworkChangedData) -> ApplicationUpdateNetwork {
    ApplicationUpdateNetwork {
        connected: network.connected,
        unmetered: network.connected
            && network.metered == Some(false)
            && matches!(
                network.networkType,
                ToolPkgBroadcastNetworkType::Wifi | ToolPkgBroadcastNetworkType::Ethernet
            ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Focus changes must not open background surfaces or change scheduled delivery eligibility.
    #[test]
    fn inactive_preserves_application_visibility() {
        assert_eq!(
            appLifecycleEventName(&RuntimeEventTopic::AppLifecycleInactive, &Value::Null),
            None
        );
    }

    /// Raw Flutter events do not determine product visibility or process termination.
    #[test]
    fn application_visibility_maps_only_visible_lifecycle_transitions() {
        for topic in [
            RuntimeEventTopic::AppLifecycleResumed,
            RuntimeEventTopic::AppLifecycleInactive,
            RuntimeEventTopic::AppLifecycleHidden,
            RuntimeEventTopic::AppLifecyclePaused,
            RuntimeEventTopic::AppLifecycleDetached,
        ] {
            assert_eq!(appLifecycleEventName(&topic, &Value::Null), None);
        }
        for (visibility, expected) in [
            ("visible", TOOLPKG_EVENT_APPLICATION_ON_FOREGROUND),
            ("hidden", TOOLPKG_EVENT_APPLICATION_ON_BACKGROUND),
        ] {
            assert_eq!(
                appLifecycleEventName(
                    &RuntimeEventTopic::AppVisibilityChanged,
                    &serde_json::json!({"visibility": visibility}),
                ),
                Some(expected)
            );
        }
    }

    /// Cellular and unknown costs stay distinguishable from a disconnected network.
    #[test]
    fn update_network_preserves_connectivity_and_unmetered_eligibility() {
        for transport in ["wifi", "ethernet", "cellular", "vpn", "other", "none"] {
            for metered in [Some(false), Some(true), None] {
                for connected in [true, false] {
                    let network = serde_json::from_value(serde_json::json!({
                        "connected": connected, "networkType": transport, "metered": metered,
                    }))
                    .unwrap();
                    let state = updateNetwork(&network);
                    assert_eq!(state.connected, connected);
                    assert_eq!(
                        state.unmetered,
                        connected
                            && metered == Some(false)
                            && matches!(transport, "wifi" | "ethernet")
                    );
                }
            }
        }
    }
}

/// Maps application visibility and activity events without treating transient focus loss as background.
#[allow(non_snake_case)]
fn appLifecycleEventName(topic: &RuntimeEventTopic, payload: &Value) -> Option<&'static str> {
    Some(match topic {
        RuntimeEventTopic::AppVisibilityChanged => match payload["visibility"].as_str() {
            Some("visible") => TOOLPKG_EVENT_APPLICATION_ON_FOREGROUND,
            Some("hidden") => TOOLPKG_EVENT_APPLICATION_ON_BACKGROUND,
            _ => return None,
        },
        RuntimeEventTopic::AppLifecycleLowMemory => TOOLPKG_EVENT_APPLICATION_ON_LOW_MEMORY,
        RuntimeEventTopic::AppLifecycleTrimMemory => TOOLPKG_EVENT_APPLICATION_ON_TRIM_MEMORY,
        RuntimeEventTopic::ActivityLifecycleCreate => TOOLPKG_EVENT_ACTIVITY_ON_CREATE,
        RuntimeEventTopic::ActivityLifecycleStart => TOOLPKG_EVENT_ACTIVITY_ON_START,
        RuntimeEventTopic::ActivityLifecycleResume => TOOLPKG_EVENT_ACTIVITY_ON_RESUME,
        RuntimeEventTopic::ActivityLifecyclePause => TOOLPKG_EVENT_ACTIVITY_ON_PAUSE,
        RuntimeEventTopic::ActivityLifecycleStop => TOOLPKG_EVENT_ACTIVITY_ON_STOP,
        RuntimeEventTopic::ActivityLifecycleDestroy => TOOLPKG_EVENT_ACTIVITY_ON_DESTROY,
        _ => return None,
    })
}
