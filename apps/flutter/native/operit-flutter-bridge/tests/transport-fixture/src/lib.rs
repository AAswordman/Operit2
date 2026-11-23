//! Runs the production transport against a scripted Core without starting the application.
#![allow(non_snake_case, dead_code)]

use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_link::{CoreCallRequest, CoreCallResponse, CoreLinkSharedClient};
use std::sync::Arc;

#[path = "../../../src/BridgeCodec.rs"]
mod BridgeCodec;
#[path = "../../../src/BridgeTransport.rs"]
mod BridgeTransport;
#[path = "../../../src/FfiTransport.rs"]
mod FfiTransport;

/// Supplies scripted business operations to the unchanged production transport.
pub struct OperitFlutterBridge {
    localCore: Arc<dyn CoreLinkSharedClient + Send + Sync>,
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
}

struct CoreCallTask {
    core: Arc<dyn CoreLinkSharedClient + Send + Sync>,
    request: CoreCallRequest,
}

impl CoreCallTask {
    /// Executes the fixture's business call using the same shared Core interface.
    async fn execute(self) -> CoreCallResponse {
        self.core.call(self.request).await
    }
}

impl OperitFlutterBridge {
    /// Prepares an owned call while leaving scheduling and delivery to production code.
    fn prepareCall(&self, request: CoreCallRequest) -> CoreCallTask {
        CoreCallTask {
            core: self.localCore.clone(),
            request,
        }
    }

    /// Returns the real native Host scheduler installed by the fixture.
    fn runtimeTaskScheduler(&self) -> Arc<dyn HostRuntimeTaskSchedulerHost> {
        self.scheduler.clone()
    }
}

mod BridgeExports {
    /// Extracts panic text for the production transport's error response.
    pub(crate) fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
        if let Some(message) = payload.downcast_ref::<&str>() {
            return (*message).into();
        }
        if let Some(message) = payload.downcast_ref::<String>() {
            return message.clone();
        }
        "non-string panic payload".into()
    }
}

#[cfg(test)]
mod tests;
