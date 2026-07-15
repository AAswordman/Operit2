#![allow(non_snake_case)]

use std::sync::Arc;

use operit_host_api::HostRuntimeEventRegistration;
use serde_json::Value;

use crate::core::events::RuntimeEvent::RuntimeEvent;
use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use crate::plugins::toolpkg::ToolPkgHostEventHookBridge::ToolPkgHostEventHookBridge;
use operit_host_api::HostManager::HostManager;

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
        ToolPkgHostEventHookBridge::dispatchHostEvent(&self.toolpkg_runtime, "broadcast", payload);
        serde_json::json!({
            "ok": true,
        })
    }
}
