use operit_link::{CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkError, CoreStreamAttachment, CoreStreamSource, CoreValue, CoreWatchRequest, CORE_STREAM_TARGET};
use operit_runtime::core::chat::ChatRuntimeHolder::ChatRuntimeHolder;
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

/// Owns the Space-side runtime object registry and embedded stream pool.
#[derive(Clone)]
pub struct SpaceRuntime {
    chatRuntimeHolder: Arc<AsyncMutex<ChatRuntimeHolder>>,
    streamPool: Arc<SpaceStreamPool>,
}

/// Stores sources referenced by embedded stream descriptors returned over Link.
struct SpaceStreamPool {
    sources: Mutex<BTreeMap<String, Arc<CoreStreamSource>>>,
}

impl SpaceStreamPool {
    /// Creates an empty Space-owned stream pool.
    fn new() -> Self {
        Self {
            sources: Mutex::new(BTreeMap::new()),
        }
    }

    /// Adopts one captured embedded stream source.
    fn adopt(&self, attachment: CoreStreamAttachment) {
        let mut sources = self
            .sources
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !sources.contains_key(&attachment.streamId) {
            sources.insert(attachment.streamId, attachment.source);
        }
    }
}

impl SpaceRuntime {
    /// Creates a Space runtime over the process ChatRuntimeHolder.
    pub fn new(chatRuntimeHolder: Arc<AsyncMutex<ChatRuntimeHolder>>) -> Self {
        Self {
            chatRuntimeHolder,
            streamPool: Arc::new(SpaceStreamPool::new()),
        }
    }

    /// Executes one annotation-addressed Space call on the main runtime slot.
    pub async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let Some(_route) = crate::generated_space_call_route(&request) else {
            return CoreCallResponse::err(
                requestId,
                CoreLinkError::new(
                    "SPACE_ROUTE_NOT_FOUND",
                    "Space call route is not registered",
                ),
            );
        };
        let (result, attachments) = operit_link::withCoreStreamCapture({
            let holder = self.chatRuntimeHolder.clone();
            async move {
                if matches!(request.methodName.as_str(), "callToolPkgRoute" | "watchToolPkgRoute" | "pushToolPkgRoute") {
                    let handler = {
                        let mut holder = holder.lock().await;
                        holder.getCore(ChatRuntimeSlot::MAIN).toolPkgRouteHost()
                    };
                    return operit_tools::tools::packTool::ToolPkgRouteRuntime::dispatch(handler, &request.methodName, request.args).await;
                }
                let mut holder = holder.lock().await;
                let core = holder.getCore(ChatRuntimeSlot::MAIN);
                crate::generated_space_call_on_chat_core(core, request).await
            }
        })
        .await;
        self.adoptAttachments(attachments);
        match result {
            Ok(value) => CoreCallResponse::ok(requestId, value),
            Err(error) => CoreCallResponse::err(requestId, error),
        }
    }

    /// Reads one annotation-addressed Space watch snapshot on the main slot.
    pub async fn watchSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        let Some(_route) = crate::generated_space_watch_route(&request) else {
            return Err(CoreLinkError::new(
                "SPACE_ROUTE_NOT_FOUND",
                "Space watch route is not registered",
            ));
        };
        let requestId = request.requestId.clone();
        let target = request.target.clone();
        let propertyName = request.propertyName.clone();
        let (result, attachments) = operit_link::withCoreStreamCapture({
            let holder = self.chatRuntimeHolder.clone();
            async move {
                let mut holder = holder.lock().await;
                let core = holder.getCore(ChatRuntimeSlot::MAIN);
                crate::generated_space_watch_snapshot_on_chat_core(core, &request).await
            }
        })
        .await;
        self.adoptAttachments(attachments);
        Ok(CoreEvent {
            requestId: Some(requestId),
            target,
            propertyName,
            kind: CoreEventKind::Snapshot,
            value: result?,
        })
    }

    /// Opens one annotation-addressed Space watch on the main slot.
    pub async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        if request.target == CORE_STREAM_TARGET {
            return self.openEmbeddedStream(request);
        }
        let Some(_route) = crate::generated_space_watch_route(&request) else {
            return Err(CoreLinkError::new(
                "SPACE_ROUTE_NOT_FOUND",
                "Space watch route is not registered",
            ));
        };
        let mut holder = self.chatRuntimeHolder.lock().await;
        let core = holder.getCore(ChatRuntimeSlot::MAIN);
        crate::generated_space_watch_on_chat_core(core, request, self.streamAttachmentAdopter())
            .await
    }

    /// Adopts captured stream sources into the Space-owned pool.
    fn adoptAttachments(&self, attachments: Vec<CoreStreamAttachment>) {
        for attachment in attachments {
            self.streamPool.adopt(attachment);
        }
    }

    /// Creates an adopter for stream sources emitted by live Space watch values.
    #[allow(non_snake_case)]
    fn streamAttachmentAdopter(&self) -> Arc<dyn Fn(Vec<CoreStreamAttachment>) + Send + Sync> {
        let streamPool = self.streamPool.clone();
        Arc::new(move |attachments| {
            for attachment in attachments {
                streamPool.adopt(attachment);
            }
        })
    }

    /// Opens an embedded response stream referenced by the fixed Link pool object id.
    fn openEmbeddedStream(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        if request.propertyName != "openCoreStream" {
            return Err(CoreLinkError::watchNotFound(&request.registryKey()));
        }
        let mut args = match request.args.clone() {
            CoreValue::Map(value) => value,
            CoreValue::Null => BTreeMap::new(),
            _ => {
                return Err(CoreLinkError::new(
                    "INVALID_ARGS",
                    "stream pool arguments must be a map",
                ))
            }
        };
        let streamId: String = decodeArgument(&mut args, "streamId")?;
        let source = self
            .streamPool
            .sources
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&streamId)
            .cloned()
            .ok_or_else(|| CoreLinkError::watchNotFound(&request.registryKey()))?;
        source.open(request)
    }
}

/// Decodes one named argument from a Link argument map.
fn decodeArgument<T: DeserializeOwned>(
    args: &mut BTreeMap<String, CoreValue>,
    name: &str,
) -> Result<T, CoreLinkError> {
    operit_link::fromCoreValue(args.remove(name).unwrap_or(CoreValue::Null))
        .map_err(|error| CoreLinkError::new("INVALID_ARGS", format!("{name}: {error}")))
}

/// Full Core supplies chat execution through the existing Link service boundary.
/// The shared router does not need to know or construct ChatRuntimeHolder.
#[async_trait::async_trait(?Send)]
impl operit_link::CoreLinkSharedClient for SpaceRuntime {
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        SpaceRuntime::call(self, request).await
    }

    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        SpaceRuntime::watchSnapshot(self, request).await
    }

    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        SpaceRuntime::watch(self, request).await
    }

    /// Opens an annotated plugin consumer after the router has authorized its Binding.
    async fn openPush(
        &self,
        request: operit_link::CorePushRequest,
    ) -> Result<Box<dyn operit_link::CoreLinkPushSession>, operit_link::CoreLinkError> {
        let route = crate::generated_space_push_route(&request)
            .ok_or_else(|| CoreLinkError::methodNotFound(&format!("{}::{}", request.target, request.methodName)))?;
        if route.methodName != "pushToolPkgRoute" {
            return crate::generated_space_push_on_chat_core(self.chatRuntimeHolder.clone(), request);
        }
        let args = operit_tools::tools::packTool::ToolPkgRouteRuntime::decode(request.args)?;
        operit_tools::tools::packTool::ToolPkgRouteRuntime::requireSpacePackage(&args.package_name).map_err(CoreLinkError::internal)?;
        let input = args.input_index.ok_or_else(|| CoreLinkError::new("INVALID_ARGS", "Push input index is required"))?;
        let handler = {
            let mut holder = self.chatRuntimeHolder.lock().await;
            holder.getCore(ChatRuntimeSlot::MAIN).toolPkgRouteHost()
        };
        let (sender, values) = operit_rslink_runtime::core_reverse_stream_channel::<serde_json::Value>();
        let (mut completion, result) = tokio::sync::oneshot::channel();
        let payload = serde_json::json!({ "op":"pushOpen", "bindingKey":args.binding_key, "methodName":args.method_name, "arguments":args.arguments, "inputIndex":input });
        operit_host_api::HostRuntimeTaskSchedulerHost::scheduleHostRuntimeAsyncTask(
            operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost().as_ref(),
            "operit-plugin-reverse-stream", Box::new(move || Box::pin(async move {
                let outcome = tokio::select! {
                    biased;
                    _ = completion.closed() => return,
                    outcome = operit_tools::tools::packTool::ToolPkgRouteRuntime::consume(handler, args.package_name, payload, values) => outcome.map_err(CoreLinkError::internal),
                };
                let _ = completion.send(outcome);
            })),
        ).map_err(|error| CoreLinkError::internal(error.to_string()))?;
        Ok(Box::new(operit_rslink_runtime::CoreReverseStreamSession::new(sender, result)))
    }
}
