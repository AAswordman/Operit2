#![allow(non_snake_case)]

use async_trait::async_trait;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient,
    CoreLinkError, CoreObjectPath, CoreRequestId, CoreWatchRequest,
};
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::util::stream::RevisableTextStream::{
    RevisableTextStream, TextStreamEventCarrier, TextStreamEventType,
};
use operit_runtime::util::stream::Stream::Stream;
use operit_runtime::util::MarkdownRenderStream::{MarkdownRenderEventStream, MarkdownStreamEvent};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

include!(concat!(env!("OUT_DIR"), "/generated_core_dispatch.rs"));

pub struct LocalCoreProxy {
    application: OperitApplication,
}

impl LocalCoreProxy {
    pub fn new(application: OperitApplication) -> Self {
        Self { application }
    }

    #[allow(non_snake_case)]
    pub fn localApplicationMut(&mut self) -> &mut OperitApplication {
        &mut self.application
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl CoreLinkClient for LocalCoreProxy {
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        match self.dispatchCall(request).await {
            Ok(value) => CoreCallResponse::ok(requestId, value),
            Err(error) => CoreCallResponse::err(requestId, error),
        }
    }

    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        self.dispatchWatchSnapshot(request)
    }

    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        self.dispatchWatch(request)
    }
}

impl LocalCoreProxy {
    #[allow(non_snake_case)]
    async fn dispatchCall(&mut self, request: CoreCallRequest) -> Result<Value, CoreLinkError> {
        generated_dispatch_core_proxy_call(self, request).await
    }

    #[allow(non_snake_case)]
    pub fn watchSnapshotSync(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        self.dispatchWatchSnapshot(request)
    }

    #[allow(non_snake_case)]
    pub fn watchSync(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        self.dispatchWatch(request)
    }

    #[allow(non_snake_case)]
    fn dispatchWatchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        generated_dispatch_core_proxy_watch_snapshot(self, request)
    }

    #[allow(non_snake_case)]
    fn dispatchWatch(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        generated_dispatch_core_proxy_watch(self, request)
    }
}

fn chat_runtime_slot(path: &CoreObjectPath) -> Option<ChatRuntimeSlot> {
    match path.key().as_str() {
        "chatRuntimeHolder.MAIN" | "chatRuntimeHolder.main" => Some(ChatRuntimeSlot::MAIN),
        "chatRuntimeHolder.FLOATING" | "chatRuntimeHolder.floating" => {
            Some(ChatRuntimeSlot::FLOATING)
        }
        "application.chatRuntimeHolder.MAIN" | "application.chatRuntimeHolder.main" => {
            Some(ChatRuntimeSlot::MAIN)
        }
        "application.chatRuntimeHolder.FLOATING" | "application.chatRuntimeHolder.floating" => {
            Some(ChatRuntimeSlot::FLOATING)
        }
        _ => None,
    }
}

fn object_args(args: Value) -> Result<Map<String, Value>, CoreLinkError> {
    match args {
        Value::Object(value) => Ok(value),
        Value::Null => Ok(Map::new()),
        _ => Err(CoreLinkError::new(
            "INVALID_ARGS",
            "core call args must be a JSON object",
        )),
    }
}

fn decode_core_arg<T: DeserializeOwned>(
    args: &mut Map<String, Value>,
    name: &str,
) -> Result<T, CoreLinkError> {
    let value = args.remove(name).unwrap_or(Value::Null);
    serde_json::from_value(value)
        .map_err(|error| CoreLinkError::new("INVALID_ARGS", format!("{name}: {error}")))
}

fn to_core_value(value: impl serde::Serialize) -> Result<Value, CoreLinkError> {
    serde_json::to_value(value).map_err(|error| CoreLinkError::internal(error.to_string()))
}

fn core_event_stream_channel() -> (
    tokio::sync::mpsc::UnboundedSender<CoreEvent>,
    CoreEventStream,
) {
    tokio::sync::mpsc::unbounded_channel()
}

fn core_text_event_stream<S>(
    stream_chat_id: String,
    stream: S,
    request: CoreWatchRequest,
) -> CoreEventStream
where
    S: RevisableTextStream + Clone + Send + 'static,
{
    let mut text_stream = stream.clone();
    let mut event_stream = TextStreamEventCarrier::event_channel(&stream).clone();
    let (sender, receiver) = core_event_stream_channel();
    let event_sender = sender.clone();
    let event_request_id = request.requestId.clone();
    let event_target_path = request.targetPath.clone();
    let event_property_name = request.propertyName.clone();
    let event_chat_id = stream_chat_id.clone();

    spawn_core_task(move || {
        event_stream.collect(&mut |event| {
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
        });
    });

    spawn_core_task(move || {
        let mut markdown_stream = MarkdownRenderEventStream::new(stream_chat_id);
        text_stream.collect(&mut |chunk| {
            for event in markdown_stream.pushChunk(&chunk) {
                send_text_event(
                    &sender,
                    &request.requestId,
                    &request.targetPath,
                    &request.propertyName,
                    CoreEventKind::Changed,
                    to_core_value(event).expect("MarkdownStreamEvent must serialize"),
                );
            }
        });
        send_text_event(
            &sender,
            &request.requestId,
            &request.targetPath,
            &request.propertyName,
            CoreEventKind::Completed,
            to_core_value(markdown_stream.completed()).expect("MarkdownStreamEvent must serialize"),
        );
    });

    receiver
}

fn core_string_event_stream<S>(mut stream: S, request: CoreWatchRequest) -> CoreEventStream
where
    S: Stream<Item = String> + Send + 'static,
{
    let (sender, receiver) = core_event_stream_channel();
    spawn_core_task(move || {
        stream.collect(&mut |value| {
            let _ = sender.send(CoreEvent {
                requestId: Some(request.requestId.clone()),
                targetPath: request.targetPath.clone(),
                propertyName: request.propertyName.clone(),
                kind: CoreEventKind::Changed,
                value: Value::String(value),
            });
        });
        let _ = sender.send(CoreEvent {
            requestId: Some(request.requestId),
            targetPath: request.targetPath,
            propertyName: request.propertyName,
            kind: CoreEventKind::Completed,
            value: Value::Null,
        });
    });
    receiver
}

fn send_text_event(
    sender: &tokio::sync::mpsc::UnboundedSender<CoreEvent>,
    request_id: &CoreRequestId,
    target_path: &CoreObjectPath,
    property_name: &str,
    kind: CoreEventKind,
    value: Value,
) {
    let _ = sender.send(CoreEvent {
        requestId: Some(request_id.clone()),
        targetPath: target_path.clone(),
        propertyName: property_name.to_string(),
        kind,
        value,
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_core_task<F>(task: F)
where
    F: FnOnce() + Send + 'static,
{
    std::thread::spawn(task);
}

#[cfg(target_arch = "wasm32")]
fn spawn_core_task<F>(task: F)
where
    F: FnOnce() + 'static,
{
    wasm_bindgen_futures::spawn_local(async move {
        task();
    });
}

fn generated_proxy_request_id() -> String {
    let millis = currentTimeMillis();
    format!("core-proxy-{millis}")
}
