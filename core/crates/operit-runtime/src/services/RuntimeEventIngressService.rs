#![allow(non_snake_case)]

use std::sync::Arc;

use operit_host_api::HostRuntimeEventRegistration;
use serde_json::Value;

use operit_context::OperitApplicationContext::OperitApplicationContext;
use crate::core::events::RuntimeEvent::RuntimeEvent;
use crate::plugins::toolpkg::ToolPkgHostEventHookBridge::ToolPkgHostEventHookBridge;

pub struct RuntimeEventIngressService;

impl RuntimeEventIngressService {
    pub fn getInstance(_context: &OperitApplicationContext) -> Self {
        Self
    }

    pub(crate) fn startHostRuntimeEventSupport(
        context: OperitApplicationContext,
    ) -> Result<Option<Box<dyn HostRuntimeEventRegistration>>, String> {
        let Some(host) = context.hostRuntimeEventHost.clone() else {
            return Ok(None);
        };
        let handlerContext = context.clone();
        let registration = host
            .startHostRuntimeEventStream(Arc::new(move |eventValue| {
                match serde_json::from_value::<RuntimeEvent>(eventValue) {
                    Ok(event) => {
                        let service = RuntimeEventIngressService::getInstance(&handlerContext);
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

    pub fn ingestEvent(&self, event: RuntimeEvent) -> Value {
        ToolPkgHostEventHookBridge::dispatchHostEvent("broadcast", event.hostEventPayload());
        serde_json::json!({
            "ok": true,
        })
    }
}
