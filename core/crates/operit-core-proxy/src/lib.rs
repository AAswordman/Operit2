#![allow(non_snake_case)]

extern crate self as operit_core_proxy;

use async_trait::async_trait;
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::HostManager::HostManager;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_host_api::{FileSystemHost, RuntimeStorageHost};
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient,
    CoreLinkError, CoreLinkPushSession, CoreLinkSharedClient, CoreRequestId,
    CoreStreamSource, CoreValue, CoreWatchRequest,
};
use operit_tools::runtime_support::{CoreNodeToolRuntime, ToolRuntimeSupport};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::chat::ChatRuntimeHolder::ChatRuntimeHolder;
use operit_util::stream::ReverseStream::ReverseStreamSender;
use operit_util::stream::Stream::Stream;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tokio::sync::oneshot;
use tokio::sync::Mutex;

include!(concat!(env!("OUT_DIR"), "/generated_core_dispatch.rs"));

#[derive(Clone)]
pub struct LocalCoreProxy {
    application: Arc<Mutex<OperitApplication>>,
    hostManager: HostManager,
    chatRuntimeHolder: Arc<Mutex<ChatRuntimeHolder>>,
    coreStreamPool: Arc<CoreStreamPool>,
}

/// Owns the in-process sources referenced by serialized `CoreStream` handles.
pub struct CoreStreamPool {
    sources: StdMutex<HashMap<String, Arc<CoreStreamSource>>>,
}

impl CoreStreamPool {
    /// Creates an empty local stream source pool.
    fn new() -> Self {
        Self {
            sources: StdMutex::new(HashMap::new()),
        }
    }

    /// Adopts one source captured while a Core value was serialized.
    fn adopt(&self, attachment: operit_link::CoreStreamAttachment) {
        let mut sources = self
            .sources
            .lock()
            .expect("core stream pool mutex poisoned");
        if let Some(existing) = sources.get(&attachment.streamId) {
            if !Arc::ptr_eq(existing, &attachment.source) {
                existing.attachNextSegment(attachment.source);
            }
        } else {
            sources.insert(attachment.streamId, attachment.source);
        }
    }

    /// Removes one source after its client-facing stream has completed.
    fn remove(&self, streamId: &str) {
        self.sources
            .lock()
            .expect("core stream pool mutex poisoned")
            .remove(streamId);
    }
}

/// Owns the runtime-side endpoints for one generated reverse stream invocation.
pub struct CoreReverseStreamSession {
    sender: Box<dyn CoreReverseStreamSender>,
    completion: Option<oneshot::Receiver<Result<(), CoreLinkError>>>,
}

/// Accepts Link values for one typed reverse stream item channel.
#[async_trait]
trait CoreReverseStreamSender: Send {
    /// Decodes and delivers one Link item to the typed stream consumer.
    async fn send(&self, value: CoreValue) -> Result<(), CoreLinkError>;

    /// Completes the typed stream consumer input.
    fn close(&mut self);
}

/// Bridges one typed reverse stream producer to Link values.
struct TypedCoreReverseStreamSender<T> {
    sender: ReverseStreamSender<T>,
}

#[async_trait]
impl<T> CoreReverseStreamSender for TypedCoreReverseStreamSender<T>
where
    T: DeserializeOwned + Send + 'static,
{
    /// Decodes one Link item and forwards it to the typed stream.
    async fn send(&self, value: CoreValue) -> Result<(), CoreLinkError> {
        let value = operit_link::fromCoreValue(value).map_err(|error| {
            CoreLinkError::new("INVALID_REVERSE_STREAM_ITEM", error.to_string())
        })?;
        self.sender
            .send(value)
            .await
            .map_err(CoreLinkError::internal)
    }

    /// Closes the typed sender after the Link input completes.
    fn close(&mut self) {
        self.sender.close();
    }
}

impl CoreReverseStreamSession {
    /// Creates one Link session over a typed reverse stream sender and completion receiver.
    pub fn new<T>(
        sender: ReverseStreamSender<T>,
        completion: oneshot::Receiver<Result<(), CoreLinkError>>,
    ) -> Self
    where
        T: DeserializeOwned + Send + 'static,
    {
        Self {
            sender: Box::new(TypedCoreReverseStreamSender { sender }),
            completion: Some(completion),
        }
    }

    /// Delivers one ordered Link item into the reverse stream.
    pub async fn pushItem(&mut self, value: CoreValue) -> Result<(), CoreLinkError> {
        self.sender.send(value).await
    }

    /// Completes the reverse stream and waits for its runtime consumer.
    pub async fn close(&mut self) -> Result<(), CoreLinkError> {
        self.sender.close();
        self.completion
            .take()
            .ok_or_else(|| {
                CoreLinkError::new("REVERSE_STREAM_CLOSED", "reverse stream is already closed")
            })?
            .await
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
    }
}

#[async_trait]
impl CoreLinkPushSession for CoreReverseStreamSession {
    /// Delivers one Link value into the generated typed reverse stream.
    async fn send(&mut self, value: CoreValue) -> Result<(), CoreLinkError> {
        self.pushItem(value).await
    }

    /// Closes the generated typed reverse stream and awaits its service result.
    async fn close(mut self: Box<Self>) -> Result<(), CoreLinkError> {
        CoreReverseStreamSession::close(&mut self).await
    }
}

impl LocalCoreProxy {
    /// Resolves one generated schema key to its process-local numeric object id.
    #[allow(non_snake_case)]
    pub fn generatedObjectIdForSchema(schema: &str) -> Option<u32> {
        generated_object_id_for_schema(schema)
    }

    /// Returns the generated local object ID for one concrete runtime type.
    pub fn generatedObjectIdForType(typeName: &str) -> Option<u32> {
        generated_object_id_for_type(typeName)
    }

    /// Installs the server-side CoreNode tool capability into this local runtime.
    #[allow(non_snake_case)]
    pub fn bindCoreNodeToolRuntime(
        &self,
        runtime: Arc<dyn CoreNodeToolRuntime>,
    ) -> Result<(), CoreLinkError> {
        let application = self
            .application
            .try_lock()
            .map_err(|_| CoreLinkError::internal("Local Core application is busy"))?;
        application
            .toolHandler
            .runtimeSupport()
            .bindCoreNodeToolRuntime(runtime)
            .map_err(CoreLinkError::internal)
    }

    /// Opens one caller-owned input stream directly on this local Core proxy.
    #[allow(non_snake_case)]
    pub fn openPushLocal(
        &self,
        request: operit_link::CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        Ok(Box::new(self.openReverseStream(request)?))
    }

    /// Reports whether a push request is a generated reverse-stream method.
    #[allow(non_snake_case)]
    pub fn isReverseStreamRequest(&self, request: &operit_link::CorePushRequest) -> bool {
        generated_is_reverse_stream_request(request)
    }
    /// Opens one generated reverse stream selected by its proxy schema declaration.
    #[allow(non_snake_case)]
    pub fn openReverseStream(
        &self,
        request: operit_link::CorePushRequest,
    ) -> Result<CoreReverseStreamSession, CoreLinkError> {
        generated_open_reverse_stream(self, request)
    }
    /// Creates a local link client backed by an in-process application.
    pub fn new(application: OperitApplication) -> Self {
        Self {
            hostManager: application.hostManager.clone(),
            chatRuntimeHolder: application.chatRuntimeHolder.clone(),
            application: Arc::new(Mutex::new(application)),
            coreStreamPool: Arc::new(CoreStreamPool::new()),
        }
    }

    /// Returns mutable access to the hosted local application.
    #[allow(non_snake_case)]
    pub fn localApplicationMut(&mut self) -> &mut OperitApplication {
        Arc::get_mut(&mut self.application)
            .expect("LocalCoreProxy application must not be shared while mutating setup")
            .get_mut()
    }

    /// Returns the runtime storage capability owned by this local core.
    #[allow(non_snake_case)]
    pub fn runtimeStorageHost(&self) -> Arc<dyn RuntimeStorageHost> {
        self.hostManager
            .runtimeStorageHost
            .clone()
            .expect("LocalCoreProxy requires a RuntimeStorageHost")
    }

    /// Returns the file-system capability owned by this local core.
    #[allow(non_snake_case)]
    pub fn fileSystemHost(&self) -> Arc<dyn FileSystemHost> {
        self.hostManager
            .fileSystemHost
            .clone()
            .expect("LocalCoreProxy requires a FileSystemHost")
    }
}

#[async_trait(?Send)]
impl CoreLinkClient for LocalCoreProxy {
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        CoreLinkSharedClient::call(self, request).await
    }

    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        CoreLinkSharedClient::watchSnapshot(self, request).await
    }

    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        CoreLinkSharedClient::watch(self, request).await
    }

    #[allow(non_snake_case)]
    async fn openPush(
        &mut self,
        request: operit_link::CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        self.openPushLocal(request)
    }
}

#[async_trait(?Send)]
impl CoreLinkSharedClient for LocalCoreProxy {
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        match self.dispatchCall(request).await {
            Ok(value) => CoreCallResponse::ok(requestId, value),
            Err(error) => CoreCallResponse::err(requestId, error),
        }
    }

    #[allow(non_snake_case)]
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        let (result, attachments) = operit_link::withCoreStreamCapture(
            generated_dispatch_core_proxy_watch_snapshot_async(self, request),
        )
        .await;
        self.adoptCoreStreamAttachments(attachments);
        result
    }

    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        if request.targetObjectId == operit_link::CORE_STREAM_POOL_OBJECT_ID {
            return self.openCoreStreamWatch(request);
        }
        generated_dispatch_core_proxy_watch_async(self, request).await
    }
}

impl LocalCoreProxy {
    #[allow(non_snake_case)]
    async fn dispatchCall(&self, request: CoreCallRequest) -> Result<CoreValue, CoreLinkError> {
        let (result, attachments) = operit_link::withCoreStreamCapture(
            generated_dispatch_core_proxy_call(self, request),
        )
        .await;
        self.adoptCoreStreamAttachments(attachments);
        result
    }

    /// Executes a watch snapshot through the generated synchronous dispatcher.
    #[allow(non_snake_case)]
    pub fn watchSnapshotSync(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        self.dispatchWatchSnapshot(request)
    }

    /// Opens a watch stream through the generated synchronous dispatcher.
    #[allow(non_snake_case)]
    pub fn watchSync(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        self.dispatchWatch(request)
    }

    #[allow(non_snake_case)]
    fn dispatchWatchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        let (result, attachments) = operit_link::withCoreStreamCaptureSync(|| {
            generated_dispatch_core_proxy_watch_snapshot(self, request)
        });
        self.adoptCoreStreamAttachments(attachments);
        result
    }

    #[allow(non_snake_case)]
    fn dispatchWatch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        if request.targetObjectId == operit_link::CORE_STREAM_POOL_OBJECT_ID {
            return self.openCoreStreamWatch(request);
        }
        generated_dispatch_core_proxy_watch(self, request)
    }

    /// Transfers serialized stream sources into this proxy's owned pool.
    fn adoptCoreStreamAttachments(&self, attachments: Vec<operit_link::CoreStreamAttachment>) {
        for attachment in attachments {
            self.coreStreamPool.adopt(attachment);
        }
    }

    /// Opens one anonymous stream source from the proxy-owned stream pool.
    fn openCoreStreamWatch(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        if request.propertyName != "openCoreStream" {
            return Err(CoreLinkError::watchNotFound(&request.registryKey()));
        }
        let mut args = object_args(request.args.clone())?;
        let streamId: String = decode_core_arg(&mut args, "streamId")?;
        let source = self
            .coreStreamPool
            .sources
            .lock()
            .expect("core stream pool mutex poisoned")
            .get(&streamId)
            .cloned()
            .ok_or_else(|| CoreLinkError::watchNotFound(&request.registryKey()))?;
        source.open(request)
    }
}

/// Extracts a string-keyed argument map from a CoreValue request payload.
fn object_args(args: CoreValue) -> Result<BTreeMap<String, CoreValue>, CoreLinkError> {
    match args {
        CoreValue::Map(value) => Ok(value),
        CoreValue::Null => Ok(BTreeMap::new()),
        _ => Err(CoreLinkError::new(
            "INVALID_ARGS",
            "core call args must be a map",
        )),
    }
}

/// Decodes and removes one named argument from a CoreValue argument map.
fn decode_core_arg<T: DeserializeOwned>(
    args: &mut BTreeMap<String, CoreValue>,
    name: &str,
) -> Result<T, CoreLinkError> {
    let value = args.remove(name).unwrap_or(CoreValue::Null);
    operit_link::fromCoreValue(value)
        .map_err(|error| CoreLinkError::new("INVALID_ARGS", format!("{name}: {error}")))
}

/// Converts a serializable runtime value into the native Link value model.
fn to_core_value(value: impl serde::Serialize) -> Result<CoreValue, CoreLinkError> {
    operit_link::toCoreValue(value).map_err(|error| CoreLinkError::internal(error.to_string()))
}

/// Creates a command error with native Link details.
fn core_call_error(message: String, details: CoreValue) -> CoreLinkError {
    CoreLinkError::withDetails("COMMAND_ERROR", message, details)
}

/// Builds a string-keyed CoreValue map for generated Link payloads.
fn core_value_map(fields: impl IntoIterator<Item = (String, CoreValue)>) -> CoreValue {
    CoreValue::Map(fields.into_iter().collect())
}

fn core_event_stream_channel() -> (
    tokio::sync::mpsc::UnboundedSender<CoreEvent>,
    CoreEventStream,
) {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    (sender, CoreEventStream::new(receiver))
}

fn core_string_event_stream<S>(mut stream: S, request: CoreWatchRequest) -> CoreEventStream
where
    S: Stream<Item = String> + Send + 'static,
{
    let (sender, receiver) = core_event_stream_channel();
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "core-proxy-string-events",
            Box::new(move || {
                Box::pin(async move {
                    stream
                        .collect(&mut |value| {
                            let _ = sender.send(CoreEvent {
                                requestId: Some(request.requestId.clone()),
                                targetObjectId: request.targetObjectId,
                                propertyName: request.propertyName.clone(),
                                kind: CoreEventKind::Changed,
                                value: CoreValue::String(value),
                            });
                        })
                        .await;
                    let _ = sender.send(CoreEvent {
                        requestId: Some(request.requestId),
                        targetObjectId: request.targetObjectId,
                        propertyName: request.propertyName,
                        kind: CoreEventKind::Completed,
                        value: CoreValue::Null,
                    });
                })
            }),
        )
        .expect("Core string event task must be scheduled");
    receiver
}

fn core_json_event_stream<S>(mut stream: S, request: CoreWatchRequest) -> CoreEventStream
where
    S: Stream + Send + 'static,
    S::Item: serde::Serialize,
{
    let (sender, receiver) = core_event_stream_channel();
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "core-proxy-json-events",
            Box::new(move || {
                Box::pin(async move {
                    stream
                        .collect(&mut |item| {
                            let value = to_core_value(item).expect("stream item must serialize");
                            let _ = sender.send(CoreEvent {
                                requestId: Some(request.requestId.clone()),
                                targetObjectId: request.targetObjectId,
                                propertyName: request.propertyName.clone(),
                                kind: CoreEventKind::Changed,
                                value,
                            });
                        })
                        .await;
                    let _ = sender.send(CoreEvent {
                        requestId: Some(request.requestId),
                        targetObjectId: request.targetObjectId,
                        propertyName: request.propertyName,
                        kind: CoreEventKind::Completed,
                        value: CoreValue::Null,
                    });
                })
            }),
        )
        .expect("Core JSON event task must be scheduled");
    receiver
}

fn generated_proxy_request_id() -> String {
    let millis = currentTimeMillis();
    format!("core-proxy-{millis}")
}
