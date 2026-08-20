#![allow(non_snake_case)]

extern crate self as operit_core_proxy;

use async_trait::async_trait;
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::HostManager::HostManager;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_host_api::{FileSystemHost, RuntimeStorageHost};
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient,
    CoreLinkError, CoreLinkPushSession, CoreLinkSharedClient, CoreObjectPath, CoreRequestId,
    CoreValue, CoreWatchRequest, CoreWatchSourceResume,
};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::chat::ChatRuntimeHolder::ChatRuntimeHolder;
use operit_tools::runtime_support::{CoreNodeToolRuntime, ToolRuntimeSupport};
use operit_util::stream::ReverseStream::ReverseStreamSender;
use operit_util::stream::RevisableTextStream::{
    RenderableTextStream, ResponseStreamItem, RevisableTextStream, TextStreamEventType,
};
use operit_util::stream::Stream::Stream;
use operit_util::stream::TextStreamRevisionTracker::TextStreamRevisionState;
use operit_util::stream::TextStreamRevisionTracker::TextStreamRevisionTracker;
use operit_util::MarkdownRenderStream::{MarkdownRenderEventStream, MarkdownStreamEvent};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;

pub mod CoreNodeRouter;
#[cfg(not(target_arch = "wasm32"))]
pub mod RuntimeRemoteLinkDiscovery;
pub mod RuntimeRemoteLinkService;
pub mod SpacePersistenceSyncService;

include!(concat!(env!("OUT_DIR"), "/generated_core_dispatch.rs"));

pub(crate) const CORE_ROUTE_CURSOR_ARGUMENT: &str = "__operit_route_cursor";
pub(crate) const CORE_ROUTE_CURSOR_PROPERTY: &str = "__operit_route_cursor";
pub(crate) const CORE_ROUTE_SOURCE_TRANSITION_PROPERTY: &str = "__operit_route_source_transition";

/// Stores the adapter-owned state needed to continue one text watch.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct TextWatchCursor {
    streamKey: String,
    revisions: TextStreamRevisionState,
}

/// Carries one opaque physical-source request from a text stream to its router.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct CoreRouteSourceTransition {
    pub(crate) targetNodeId: String,
    pub(crate) payload: Vec<u8>,
}

#[derive(Clone)]
pub struct LocalCoreProxy {
    application: Arc<Mutex<OperitApplication>>,
    hostManager: HostManager,
    chatRuntimeHolder: Arc<Mutex<ChatRuntimeHolder>>,
    toolRuntimeSupport: Arc<dyn ToolRuntimeSupport>,
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
        let toolRuntimeSupport = application.toolHandler.runtimeSupport();
        Self {
            hostManager: application.hostManager.clone(),
            chatRuntimeHolder: application.chatRuntimeHolder.clone(),
            toolRuntimeSupport,
            application: Arc::new(Mutex::new(application)),
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
impl CoreNodeRouter::CoreRouteLocalRuntime for LocalCoreProxy {
    /// Installs the live Core routing capability exposed to built-in tools.
    #[allow(non_snake_case)]
    fn bindCoreNodeToolRuntime(
        &self,
        runtime: Arc<dyn CoreNodeToolRuntime>,
    ) -> Result<(), CoreLinkError> {
        self.toolRuntimeSupport
            .bindCoreNodeToolRuntime(runtime)
            .map_err(CoreLinkError::internal)
    }

    /// Returns the runtime storage capability owned by this local proxy.
    #[allow(non_snake_case)]
    fn runtimeStorageHost(&self) -> Arc<dyn RuntimeStorageHost> {
        LocalCoreProxy::runtimeStorageHost(self)
    }

    /// Executes one local Core call through the generated dispatch surface.
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        CoreLinkSharedClient::call(self, request).await
    }

    /// Reads one local Core watch snapshot through the generated dispatch surface.
    #[allow(non_snake_case)]
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        CoreLinkSharedClient::watchSnapshot(self, request).await
    }

    /// Opens one local Core watch and lets the selected inner source consume its opaque resume.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        CoreLinkSharedClient::watch(self, request).await
    }

    /// Activates one generated watch source before its Binding is published.
    #[allow(non_snake_case)]
    async fn activateWatchSource(
        &self,
        request: CoreWatchRequest,
        generation: i64,
        payload: Vec<u8>,
    ) -> Result<(), CoreLinkError> {
        generated_dispatch_core_proxy_watch_transition_async(
            self,
            &request,
            CoreWatchSourceResume {
                generation,
                payload,
            },
        )
        .await
    }

    /// Opens one generated local Core push stream.
    #[allow(non_snake_case)]
    fn openPush(
        &self,
        request: operit_link::CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        Ok(Box::new(self.openReverseStream(request)?))
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
        Ok(Box::new(self.openReverseStream(request)?))
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
        generated_dispatch_core_proxy_watch_snapshot_async(self, request).await
    }

    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        generated_dispatch_core_proxy_watch_async(self, request).await
    }
}

impl LocalCoreProxy {
    #[allow(non_snake_case)]
    async fn dispatchCall(&self, request: CoreCallRequest) -> Result<CoreValue, CoreLinkError> {
        generated_dispatch_core_proxy_call(self, request).await
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
        generated_dispatch_core_proxy_watch_snapshot(self, request)
    }

    #[allow(non_snake_case)]
    fn dispatchWatch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        generated_dispatch_core_proxy_watch(self, request)
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

/// Converts one renderable text response into streamed Markdown events.
fn core_text_event_stream<S>(
    stream_key: String,
    stream: S,
    request: CoreWatchRequest,
) -> Result<CoreEventStream, CoreLinkError>
where
    S: RenderableTextStream + Send + 'static,
{
    let resumeCursor = core_route_cursor(&request.args)?
        .map(|value| {
            operit_link::fromCoreValue::<TextWatchCursor>(value)
                .map_err(|error| CoreLinkError::new("CORE_ROUTE_CURSOR_CODEC", error.to_string()))
        })
        .transpose()?;
    let (sender, receiver) = core_event_stream_channel();
    let (cancelSender, mut cancelReceiver) = tokio::sync::oneshot::channel::<()>();
    let initialContent = stream.initial_render_content();
    if let Some(cursor) = resumeCursor.as_ref() {
        if cursor.streamKey != stream_key {
            return Err(CoreLinkError::new(
                "CORE_ROUTE_CURSOR_STREAM_MISMATCH",
                "Route cursor belongs to a different text stream",
            ));
        }
        if !initialContent.starts_with(&cursor.revisions.content) {
            return Err(CoreLinkError::new(
                "CORE_ROUTE_CURSOR_PREFIX_MISMATCH",
                "Reopened text stream does not preserve the routed cursor prefix",
            ));
        }
    }
    let mut orderedStream = stream;
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "core-proxy-text-markdown",
            Box::new(move || {
                Box::pin(async move {
                    let mut markdownStream = MarkdownRenderEventStream::new(stream_key.clone());
                    let mut textRevisions = match resumeCursor {
                        Some(cursor) => {
                            markdownStream.restoreContent(&cursor.revisions.content);
                            let suffix = &initialContent[cursor.revisions.content.len()..];
                            let mut revisions =
                                TextStreamRevisionTracker::from_state(cursor.revisions);
                            if !suffix.is_empty() {
                                let _ = revisions.append(suffix);
                                for event in markdownStream.pushChunk(suffix) {
                                    send_text_event(
                                        &sender,
                                        &request.requestId,
                                        &request.targetPath,
                                        &request.propertyName,
                                        CoreEventKind::Changed,
                                        to_core_value(event)
                                            .expect("MarkdownStreamEvent must serialize"),
                                    );
                                }
                            }
                            revisions
                        }
                        None => {
                            for event in markdownStream.beginSnapshot(&initialContent) {
                                send_text_event(
                                    &sender,
                                    &request.requestId,
                                    &request.targetPath,
                                    &request.propertyName,
                                    CoreEventKind::Changed,
                                    to_core_value(event)
                                        .expect("MarkdownStreamEvent must serialize"),
                                );
                            }
                            TextStreamRevisionTracker::new(&initialContent)
                        }
                    };
                    send_text_cursor(&sender, &request, &stream_key, &textRevisions);
                    let mut itemCollector = |item: ResponseStreamItem| match item {
                        ResponseStreamItem::Chunk(chunk) => {
                            let _ = textRevisions.append(&chunk);
                            for event in markdownStream.pushChunk(&chunk) {
                                send_text_event(
                                    &sender,
                                    &request.requestId,
                                    &request.targetPath,
                                    &request.propertyName,
                                    CoreEventKind::Changed,
                                    to_core_value(event)
                                        .expect("MarkdownStreamEvent must serialize"),
                                );
                            }
                            send_text_cursor(&sender, &request, &stream_key, &textRevisions);
                        }
                        ResponseStreamItem::Revision(event) => {
                            let markdownEvent = match event.event_type {
                                TextStreamEventType::Savepoint => {
                                    textRevisions.savepoint(&event.id);
                                    MarkdownStreamEvent::savepoint(stream_key.clone(), event.id)
                                }
                                TextStreamEventType::Rollback => {
                                    let content = textRevisions.rollback(&event.id).expect(
                                        "markdown rollback must reference an active savepoint",
                                    );
                                    markdownStream.restoreContent(content);
                                    MarkdownStreamEvent::rollback(stream_key.clone(), event.id)
                                }
                            };
                            send_text_event(
                                &sender,
                                &request.requestId,
                                &request.targetPath,
                                &request.propertyName,
                                CoreEventKind::Changed,
                                to_core_value(markdownEvent)
                                    .expect("MarkdownStreamEvent must serialize"),
                            );
                            send_text_cursor(&sender, &request, &stream_key, &textRevisions);
                        }
                    };
                    let streamCollection = orderedStream.collect_ordered(&mut itemCollector);
                    tokio::select! {
                        _ = streamCollection => {},
                        _ = &mut cancelReceiver => {},
                    }
                    if orderedStream.terminal_source_target().is_some() {
                        match orderedStream.wait_terminal_source_transition().await {
                            Ok(transition) => {
                                send_text_event(
                                    &sender,
                                    &request.requestId,
                                    &request.targetPath,
                                    CORE_ROUTE_SOURCE_TRANSITION_PROPERTY,
                                    CoreEventKind::Changed,
                                    to_core_value(CoreRouteSourceTransition {
                                        targetNodeId: transition.target,
                                        payload: transition.payload,
                                    })
                                    .expect("CoreRouteSourceTransition must serialize"),
                                );
                            }
                            Err(error) => {
                                operit_util::AppLogger::AppLogger::e(
                                    "CoreStreamRoute",
                                    &format!("text source transition failed: {error}"),
                                );
                            }
                        }
                        return;
                    }
                    send_text_event(
                        &sender,
                        &request.requestId,
                        &request.targetPath,
                        &request.propertyName,
                        CoreEventKind::Completed,
                        to_core_value(markdownStream.completed())
                            .expect("MarkdownStreamEvent must serialize"),
                    );
                })
            }),
        )
        .expect("Core text markdown task must be scheduled");
    Ok(receiver.withOnClose(move || {
        let _ = cancelSender.send(());
    }))
}

/// Reads one opaque route cursor from the reserved watch argument.
fn core_route_cursor(args: &CoreValue) -> Result<Option<CoreValue>, CoreLinkError> {
    match args {
        CoreValue::Map(values) => Ok(values.get(CORE_ROUTE_CURSOR_ARGUMENT).cloned()),
        CoreValue::Null => Ok(None),
        _ => Err(CoreLinkError::new(
            "INVALID_ARGS",
            "core watch args must be a map",
        )),
    }
}

/// Emits one adapter-owned cursor that routers retain without decoding.
fn send_text_cursor(
    sender: &tokio::sync::mpsc::UnboundedSender<CoreEvent>,
    request: &CoreWatchRequest,
    streamKey: &str,
    revisions: &TextStreamRevisionTracker,
) {
    send_text_event(
        sender,
        &request.requestId,
        &request.targetPath,
        CORE_ROUTE_CURSOR_PROPERTY,
        CoreEventKind::Changed,
        to_core_value(TextWatchCursor {
            streamKey: streamKey.to_string(),
            revisions: revisions.state(),
        })
        .expect("TextWatchCursor must serialize"),
    );
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
                                targetPath: request.targetPath.clone(),
                                propertyName: request.propertyName.clone(),
                                kind: CoreEventKind::Changed,
                                value: CoreValue::String(value),
                            });
                        })
                        .await;
                    let _ = sender.send(CoreEvent {
                        requestId: Some(request.requestId),
                        targetPath: request.targetPath,
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
                                targetPath: request.targetPath.clone(),
                                propertyName: request.propertyName.clone(),
                                kind: CoreEventKind::Changed,
                                value,
                            });
                        })
                        .await;
                    let _ = sender.send(CoreEvent {
                        requestId: Some(request.requestId),
                        targetPath: request.targetPath,
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

fn send_text_event(
    sender: &tokio::sync::mpsc::UnboundedSender<CoreEvent>,
    request_id: &CoreRequestId,
    target_path: &CoreObjectPath,
    property_name: &str,
    kind: CoreEventKind,
    value: CoreValue,
) {
    let _ = sender.send(CoreEvent {
        requestId: Some(request_id.clone()),
        targetPath: target_path.clone(),
        propertyName: property_name.to_string(),
        kind,
        value,
    });
}

fn generated_proxy_request_id() -> String {
    let millis = currentTimeMillis();
    format!("core-proxy-{millis}")
}
