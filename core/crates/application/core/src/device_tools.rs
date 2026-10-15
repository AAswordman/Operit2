//! Core-owned device tool routing, deadlines and cancellation.
use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_link::{
    fromCoreValue, toCoreValue, CoreCallRequest, CoreEventKind, CoreValue, CoreWatchRequest,
};
use operit_node_runtime::CoreNodeRouter::CoreNodeRouter;
use operit_plugin_sdk::javascript::{
    JsDeviceToolAction, JsDeviceToolCall, JsExecutionCompletion, JsToolCallRequest,
    JsToolCallResult, JsToolCallResultData,
};
use operit_tools::tools::DeviceTools::{
    failure, DeviceToolPreflight, DeviceToolRuntime, DEVICE_TOOL_TARGET,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use tokio::sync::oneshot;

type Pending = Arc<Mutex<HashMap<(String, String), Option<oneshot::Sender<()>>>>>;

#[derive(Clone)]
pub(super) struct CoreDeviceToolRuntime {
    pub(super) router: Weak<CoreNodeRouter>,
    pub(super) scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    pending: Pending,
    pub(super) channels: super::device_transport::Channels,
}
impl CoreDeviceToolRuntime {
    pub fn new(
        router: Weak<CoreNodeRouter>,
        scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    ) -> Self {
        Self {
            router,
            scheduler,
            pending: Arc::new(Mutex::new(HashMap::new())),
            channels: Arc::default(),
        }
    }
}
struct PendingGuard {
    pending: Pending,
    key: (String, String),
}
impl Drop for PendingGuard {
    fn drop(&mut self) {
        self.pending.lock().unwrap().remove(&self.key);
    }
}

impl CoreDeviceToolRuntime {
    async fn execute_port(
        &self,
        node_id: String,
        request: CoreCallRequest,
    ) -> Result<serde_json::Value, String> {
        let router = self.router.clone();
        let (send, receive) = tokio::sync::oneshot::channel();
        let scheduled = self.scheduler.scheduleHostRuntimeAsyncTask(
            "core-plugin-edge-port",
            Box::new(move || {
                Box::pin(async move {
                    let result = match router.upgrade() {
                        Some(router) if node_id != router.localNodeId() => {
                            // Target routing retains pairing direction, Space membership and
                            // NetworkControl policy. No direct peer call, forced address, or adapter retry.
                            match router.callNode(node_id, request).await.result {
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
        scheduled.map_err(|error| format!("Edge task scheduling failed: {error}"))?;
        receive
            .await
            .map_err(|_| "Edge execution stopped without a result".to_string())?
    }

    async fn execute_device(
        &self,
        request: JsToolCallRequest,
        device: JsDeviceToolCall,
    ) -> Result<JsToolCallResult, String> {
        if device.action == JsDeviceToolAction::Transport {
            return self.execute_transport(request, device).await;
        }
        if device.action == JsDeviceToolAction::Port {
            let (call, _) =
                operit_tools::tools::EdgePortTool::prepare_device_port(&device.node_id, &request)?;
            let data = self.execute_port(device.node_id.clone(), call).await?;
            let result = operit_tools::tools::EdgePortTool::port_result(device.node_id, data)?;
            return Ok(JsToolCallResult {
                success: true,
                data: JsToolCallResultData::Value(
                    serde_json::to_value(result).map_err(|error| error.to_string())?,
                ),
                error: None,
            });
        }
        let router = self
            .router
            .upgrade()
            .ok_or("Owning Core runtime is no longer available")?;
        let catalog = device.action == JsDeviceToolAction::Catalog;
        let watch = CoreWatchRequest {
            requestId: operit_link::CoreRequestId(operit_link::nextCoreRouteRequestId(
                "deviceTool",
            )),
            target: DEVICE_TOOL_TARGET.into(),
            propertyName: if catalog { "catalog" } else { "execute" }.into(),
            args: if catalog {
                CoreValue::Null
            } else {
                toCoreValue(request).map_err(|error| error.to_string())?
            },
        };
        let mut stream = if device.node_id == router.localNodeId() {
            router.openDeviceToolWatch(
                watch,
                router.localNodeId(),
                operit_link::RoutedCoreRequestKind::Target,
            )
        } else {
            router.watchNode(device.node_id, watch).await
        }
        .map_err(|error| error.to_string())?;
        while let Some(event) = stream.recv().await {
            if event.kind == CoreEventKind::Completed {
                return fromCoreValue(event.value).map_err(|error| error.to_string());
            }
        }
        Err("Device tool connection closed before the final result".into())
    }
}

impl DeviceToolRuntime for CoreDeviceToolRuntime {
    fn close_transports(&self, owner: &str) {self.close_transport_owner(owner);}

    fn dispatch_with_preflight(
        &self,
        mut request: JsToolCallRequest,
        preflight: Option<DeviceToolPreflight>,
    ) -> JsExecutionCompletion<JsToolCallResult> {
        let Some(device) = request.device.take() else {
            return Box::pin(async { failure("Device metadata is required") });
        };
        if let Err(error) = device.validate() {
            return Box::pin(async move { failure(error) });
        }
        let key = (device.node_id.clone(), device.invocation_id.clone());
        if device.action == JsDeviceToolAction::Cancel {
            if let Some(cancel) = self
                .pending
                .lock()
                .unwrap()
                .get_mut(&key)
                .and_then(Option::take)
            {
                let _ = cancel.send(());
            }
            return Box::pin(async {
                JsToolCallResult {
                    success: true,
                    ..Default::default()
                }
            });
        }
        let (cancel, cancelled) = oneshot::channel();
        {
            let mut pending = self.pending.lock().unwrap();
            if pending.contains_key(&key) {
                return Box::pin(async { failure("Duplicate device invocation ID") });
            }
            pending.insert(key.clone(), Some(cancel));
        }
        let deadline =
            tokio::time::Instant::now() + std::time::Duration::from_millis(device.timeout_ms);
        let guard = PendingGuard {
            pending: self.pending.clone(),
            key,
        };
        let runtime = self.clone();
        let (mut send, receive) = oneshot::channel();
        let scheduled = self.scheduler.scheduleHostRuntimeAsyncTask(
            "core-device-tool",
            Box::new(move || {
                Box::pin(async move {
                    let transport_open=device.action==JsDeviceToolAction::Transport && request.parameters.get("op").and_then(serde_json::Value::as_str)==Some("open");
                    let transport_receive=device.action==JsDeviceToolAction::Transport && request.parameters.get("op").and_then(serde_json::Value::as_str)==Some("next");
                    let execution = async {
                        if let Some(preflight) = preflight {
                            preflight.await?;
                        }
                        runtime.execute_device(request, device).await
                    };
                    let result = tokio::select! {
                        biased;
                        _ = cancelled => failure("Device tool call cancelled"),
                        _ = send.closed() => failure("Device tool caller was dropped"),
                        result = async {
                            // Watching an idle endpoint is not a failed request. Ownership,
                            // cancellation and endpoint EOF still end this pending receive.
                            if transport_receive {
                                execution.await.unwrap_or_else(failure)
                            } else {
                                match tokio::time::timeout_at(deadline, execution).await {
                                    Ok(Ok(result)) => result,
                                    Ok(Err(error)) => failure(error),
                                    Err(_) => failure("Device tool call timed out"),
                                }
                            }
                        } => result
                    };
                    drop(guard);
                    if let Err(result)=send.send(result) {
                        if transport_open {
                            if let JsToolCallResultData::Value(value)=result.data {
                                if let Some(handle)=value["handle"].as_str() {runtime.close_transport_handle(handle);}
                            }
                        }
                    }
                })
            }),
        );
        Box::pin(async move {
            if let Err(error) = scheduled {
                return failure(format!("Device task scheduling failed: {error}"));
            }
            receive
                .await
                .unwrap_or_else(|_| failure("Device tool task stopped without a result"))
        })
    }
}

#[cfg(test)]
#[path = "device_tools_tests.rs"]
mod tests;
