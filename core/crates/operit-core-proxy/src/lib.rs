#![allow(non_snake_case)]

extern crate self as operit_core_proxy;

use async_trait::async_trait;
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::HostManager::HostManager;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_host_api::{FileSystemHost, RuntimeStorageHost};
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient,
    CoreLinkError, CoreLinkSharedClient, CoreObjectPath, CoreRequestId, CoreValue,
    CoreWatchRequest,
};
use operit_model::ChatMessage::SharedAiResponseStream;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::chat::ChatRuntimeHolder::ChatRuntimeHolder;
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_util::stream::RevisableTextStream::{TextStreamEvent, TextStreamEventType};
use operit_util::stream::Stream::Stream;
use operit_util::MarkdownRenderStream::{MarkdownRenderEventStream, MarkdownStreamEvent};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod RuntimeCoreRouter;
#[cfg(not(target_arch = "wasm32"))]
pub mod RuntimeRemoteLinkDiscovery;
pub mod RuntimeRemoteLinkService;

include!(concat!(env!("OUT_DIR"), "/generated_core_dispatch.rs"));

#[derive(Clone)]
pub struct LocalCoreProxy {
    application: Arc<Mutex<OperitApplication>>,
    hostManager: HostManager,
    chatRuntimeHolder: Arc<Mutex<ChatRuntimeHolder>>,
}

impl LocalCoreProxy {
    /// Creates a local link client backed by an in-process application.
    pub fn new(application: OperitApplication) -> Self {
        Self {
            hostManager: application.hostManager.clone(),
            chatRuntimeHolder: application.chatRuntimeHolder.clone(),
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

fn chat_runtime_slot(path: &CoreObjectPath) -> Option<ChatRuntimeSlot> {
    let segments = path.segments.as_slice();
    match segments {
        [root, slot] if root == "chatRuntimeHolder" => chat_runtime_slot_from_segments(slot, None),
        [root, holder, slot] if root == "application" && holder == "chatRuntimeHolder" => {
            chat_runtime_slot_from_segments(slot, None)
        }
        [root, slot, id] if root == "chatRuntimeHolder" => {
            chat_runtime_slot_from_segments(slot, Some(id))
        }
        [root, holder, slot, id] if root == "application" && holder == "chatRuntimeHolder" => {
            chat_runtime_slot_from_segments(slot, Some(id))
        }
        _ => None,
    }
}

fn chat_runtime_slot_from_segments(slot: &str, id: Option<&String>) -> Option<ChatRuntimeSlot> {
    match (slot, id) {
        ("MAIN" | "main", None) => Some(ChatRuntimeSlot::MAIN),
        ("FLOATING" | "floating", None) => Some(ChatRuntimeSlot::FLOATING),
        ("DETACHED" | "detached", Some(id)) => Some(ChatRuntimeSlot::DETACHED(id.clone())),
        _ => None,
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

fn core_text_event_stream(
    stream_chat_id: String,
    stream: SharedAiResponseStream,
    request: CoreWatchRequest,
) -> CoreEventStream {
    let (sender, receiver) = core_event_stream_channel();
    let (eventCancelSender, mut eventCancelReceiver) = tokio::sync::oneshot::channel::<()>();
    let (textCancelSender, mut textCancelReceiver) = tokio::sync::oneshot::channel::<()>();
    let event_sender = sender.clone();
    let event_request_id = request.requestId.clone();
    let event_target_path = request.targetPath.clone();
    let event_property_name = request.propertyName.clone();
    let event_chat_id = stream_chat_id.clone();
    let mut eventStream = stream.event_channel.clone();
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "core-proxy-text-events",
            Box::new(move || {
                Box::pin(async move {
                    let mut eventCollector = move |event: TextStreamEvent| {
                        let value = to_core_value(match event.event_type {
                            TextStreamEventType::Savepoint => {
                                MarkdownStreamEvent::savepoint(event_chat_id.clone(), event.id)
                            }
                            TextStreamEventType::Rollback => {
                                MarkdownStreamEvent::rollback(event_chat_id.clone(), event.id)
                            }
                        })
                        .expect("MarkdownStreamEvent must serialize");
                        send_text_event(
                            &event_sender,
                            &event_request_id,
                            &event_target_path,
                            &event_property_name,
                            CoreEventKind::Changed,
                            value,
                        );
                    };
                    let eventCollection = eventStream.collect(&mut eventCollector);
                    tokio::select! {
                        _ = eventCollection => {},
                        _ = &mut eventCancelReceiver => {},
                    }
                })
            }),
        )
        .expect("Core text event task must be scheduled");
    let (textInputSender, textInputReceiver) = tokio::sync::mpsc::unbounded_channel();
    spawn_text_markdown_processor(
        stream_chat_id,
        textInputReceiver,
        sender.clone(),
        request.requestId.clone(),
        request.targetPath.clone(),
        request.propertyName.clone(),
    );
    let mut textStream = stream.upstream.clone();
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "core-proxy-text-chunks",
            Box::new(move || {
                Box::pin(async move {
                    let mut textCollector = move |chunk| {
                        let _ = textInputSender.send(chunk);
                    };
                    let textCollection = textStream.collect(&mut textCollector);
                    tokio::select! {
                        _ = textCollection => {},
                        _ = &mut textCancelReceiver => {},
                    }
                })
            }),
        )
        .expect("Core text chunk task must be scheduled");
    receiver.withOnClose(move || {
        let _ = eventCancelSender.send(());
        let _ = textCancelSender.send(());
    })
}

/// Starts one host-scheduled markdown renderer that forwards text chunks as Link events.
fn spawn_text_markdown_processor(
    streamChatId: String,
    mut input: tokio::sync::mpsc::UnboundedReceiver<String>,
    sender: tokio::sync::mpsc::UnboundedSender<CoreEvent>,
    requestId: CoreRequestId,
    targetPath: CoreObjectPath,
    propertyName: String,
) {
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "core-proxy-text-markdown",
            Box::new(move || {
                Box::pin(async move {
                    let mut markdownStream = MarkdownRenderEventStream::new(streamChatId);
                    while let Some(chunk) = input.recv().await {
                        for event in markdownStream.pushChunk(&chunk) {
                            send_text_event(
                                &sender,
                                &requestId,
                                &targetPath,
                                &propertyName,
                                CoreEventKind::Changed,
                                to_core_value(event).expect("MarkdownStreamEvent must serialize"),
                            );
                        }
                    }
                    send_text_event(
                        &sender,
                        &requestId,
                        &targetPath,
                        &propertyName,
                        CoreEventKind::Completed,
                        to_core_value(markdownStream.completed())
                            .expect("MarkdownStreamEvent must serialize"),
                    );
                })
            }),
        )
        .expect("Markdown processor task must be scheduled");
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
