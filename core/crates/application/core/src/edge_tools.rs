//! Owns plugin hardware dispatch at Core assembly, not in tools or ESP32 UI.
use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_link::{CoreCallRequest, fromCoreValue};
use operit_node_runtime::CoreNodeRouter::CoreNodeRouter;
use operit_tools::runtime_support::EdgeToolRuntime;
use std::sync::{Arc, Weak};

pub(super) struct CoreEdgeToolRuntime {
    router: Weak<CoreNodeRouter>,
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
}
impl CoreEdgeToolRuntime {
    pub fn new(
        router: Weak<CoreNodeRouter>,
        scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    ) -> Self {
        Self { router, scheduler }
    }
}
impl EdgeToolRuntime for CoreEdgeToolRuntime {
    fn execute(
        &self,
        nodeId: String,
        request: CoreCallRequest,
    ) -> operit_plugin_sdk::javascript::JsExecutionCompletion<Result<serde_json::Value, String>>
    {
        let allowed = matches!(
            (request.target.as_str(), request.methodName.as_str()),
            ("edge.plugins", "invoke") | ("edge.deviceIo", "getDigitalOutput" | "setDigitalOutput")
        );
        if !allowed {
            return Box::pin(async {
                Err("Only declared Edge hardware entrypoints are allowed".into())
            });
        }
        let router = self.router.clone();
        let (send, receive) = tokio::sync::oneshot::channel();
        let scheduled = self.scheduler.scheduleHostRuntimeAsyncTask(
            "core-plugin-edge-port",
            Box::new(move || {
                Box::pin(async move {
                    let result = match router.upgrade() {
                        Some(router) if nodeId != router.localNodeId() => {
                            // Target routing retains pairing direction, Space membership and
                            // NetworkControl policy. No direct peer call, forced address, or adapter retry.
                            match router.callNode(nodeId, request).await.result {
                                Ok(value) => fromCoreValue(value)
                                    .map_err(|error| format!("Invalid Edge reply: {error}")),
                                Err(error) => Err(error.to_string()),
                            }
                        }
                        Some(_) => Err("An explicit remote Edge node is required".into()),
                        None => Err("Owning Core runtime is no longer available".into()),
                    };
                    // A timed-out/cancelled JS caller must not drop the in-flight Link ACK.
                    let _ = send.send(result);
                })
            }),
        );
        Box::pin(async move {
            scheduled.map_err(|error| format!("Edge task scheduling failed: {error}"))?;
            receive
                .await
                .map_err(|_| "Edge execution stopped without a result".to_string())?
        })
    }
}
