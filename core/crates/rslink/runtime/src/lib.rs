#![allow(non_snake_case)]

use async_trait::async_trait;
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkError,
    CoreLinkPushSession, CoreRouteRuntime, CoreStreamAttachment, CoreStreamSource, CoreValue,
    CoreWatchRequest,
};
use operit_store::PreferencesDataStore::{Flow, FlowCancellation, StateFlow};
use operit_util::stream::ReverseStream::{ReverseStream, ReverseStreamSender};
use operit_util::stream::Stream::Stream;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tokio::sync::oneshot;

/// Receives stream attachments captured while generated Link values are encoded.
pub type CoreStreamAttachmentAdopter = Arc<dyn Fn(Vec<CoreStreamAttachment>) + Send + Sync>;

/// Owns the in-process sources referenced by serialized `CoreStream` handles.
pub struct CoreStreamPool {
    sources: StdMutex<HashMap<String, Arc<CoreStreamSource>>>,
}

impl CoreStreamPool {
    /// Creates an empty local stream source pool.
    pub fn new() -> Self {
        Self {
            sources: StdMutex::new(HashMap::new()),
        }
    }

    /// Adopts every source captured while Core values were serialized.
    pub fn adoptAll(&self, attachments: Vec<operit_link::CoreStreamAttachment>) {
        for attachment in attachments {
            self.adopt(attachment);
        }
    }

    /// Opens one anonymous stream source from the rslinkrs-owned stream pool.
    pub fn openCoreStreamWatch(
        self: &Arc<Self>,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        if request.propertyName != "openCoreStream" {
            return Err(CoreLinkError::watchNotFound(&request.registryKey()));
        }
        let mut args = object_args(request.args.clone())?;
        let streamId: String = decode_core_arg(&mut args, "streamId")?;
        let source = self
            .sources
            .lock()
            .expect("core stream pool mutex poisoned")
            .get(&streamId)
            .cloned()
            .ok_or_else(|| CoreLinkError::watchNotFound(&request.registryKey()))?;
        let pool = self.clone();
        let cleanupStreamId = streamId.clone();
        source.open(request).map(|stream| {
            stream.withOnClose(move || {
                pool.remove(&cleanupStreamId);
            })
        })
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

    /// Removes one source after its logical stream has completed.
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
        let value = operit_link::fromCoreValue(value)
            .map_err(|error| CoreLinkError::new("INVALID_REVERSE_STREAM_ITEM", error.to_string()))?;
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

/// Extracts a string-keyed argument map from a CoreValue request payload.
pub fn object_args(args: CoreValue) -> Result<BTreeMap<String, CoreValue>, CoreLinkError> {
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
pub fn decode_core_arg<T: DeserializeOwned>(
    args: &mut BTreeMap<String, CoreValue>,
    name: &str,
) -> Result<T, CoreLinkError> {
    let value = args.remove(name).unwrap_or(CoreValue::Null);
    operit_link::fromCoreValue(value)
        .map_err(|error| CoreLinkError::new("INVALID_ARGS", format!("{name}: {error}")))
}

/// Converts a serializable runtime value into the native Link value model.
pub fn to_core_value(value: impl serde::Serialize) -> Result<CoreValue, CoreLinkError> {
    operit_link::toCoreValue(value).map_err(|error| CoreLinkError::internal(error.to_string()))
}

/// Converts a serializable caller argument into a Link request value.
pub fn to_core_arg_value(value: impl serde::Serialize) -> Result<CoreValue, CoreLinkError> {
    operit_link::toCoreValue(value)
        .map_err(|error| CoreLinkError::new("INVALID_ARGS", error.to_string()))
}

/// Converts one named caller argument into a Link request value.
pub fn to_named_core_arg_value(
    name: &str,
    value: impl serde::Serialize,
) -> Result<CoreValue, CoreLinkError> {
    operit_link::toCoreValue(value)
        .map_err(|error| CoreLinkError::new("INVALID_ARGS", format!("{name}: {error}")))
}

/// Converts one named caller argument into a map entry for a generated Link request.
pub fn core_arg_entry(
    name: &str,
    value: impl Serialize,
) -> Result<(String, CoreValue), CoreLinkError> {
    Ok((name.to_string(), to_named_core_arg_value(name, value)?))
}

/// Decodes a Link response value into the generated caller return type.
pub fn from_core_response_value<T: DeserializeOwned>(
    value: CoreValue,
) -> Result<T, CoreLinkError> {
    operit_link::fromCoreValue(value)
        .map_err(|error| CoreLinkError::new("INVALID_RESPONSE", error.to_string()))
}

/// Creates a command error with native Link details.
pub fn core_call_error(message: String, details: CoreValue) -> CoreLinkError {
    CoreLinkError::withDetails("COMMAND_ERROR", message, details)
}

/// Builds a string-keyed CoreValue map for generated Link payloads.
pub fn core_value_map(fields: impl IntoIterator<Item = (String, CoreValue)>) -> CoreValue {
    CoreValue::Map(fields.into_iter().collect())
}

/// Builds route arguments with the incremental-watch capability marker enabled.
pub fn core_route_args(fields: impl IntoIterator<Item = (String, CoreValue)>) -> CoreValue {
    let mut args = fields.into_iter().collect::<BTreeMap<_, _>>();
    args.insert(
        operit_link::CORE_INCREMENTAL_VALUES_ARGUMENT.to_string(),
        CoreValue::Bool(true),
    );
    CoreValue::Map(args)
}

/// Builds the internal Link call request used by annotation-generated routes.
pub fn core_route_call_request(method_name: &str, args: CoreValue) -> CoreCallRequest {
    CoreCallRequest::new(
        operit_link::nextCoreRouteRequestId(method_name),
        operit_link::CORE_INTERNAL_ROUTE_OBJECT_ID,
        method_name,
        args,
    )
}

/// Builds the internal Link watch request used by annotation-generated routes.
pub fn core_route_watch_request(method_name: &str, args: CoreValue) -> CoreWatchRequest {
    CoreWatchRequest::new(
        operit_link::nextCoreRouteRequestId(method_name),
        operit_link::CORE_INTERNAL_ROUTE_OBJECT_ID,
        method_name,
        args,
    )
}

/// Routes one annotation-generated call through the installed Core route runtime.
pub async fn core_route_call_response(
    runtime: &dyn CoreRouteRuntime,
    method_name: &str,
    args: CoreValue,
) -> CoreCallResponse {
    runtime.call(core_route_call_request(method_name, args)).await
}

/// Decodes one Core call response into the generated caller return type.
pub fn decode_core_call_response<T: DeserializeOwned>(
    response: CoreCallResponse,
) -> Result<T, CoreLinkError> {
    from_core_response_value(response.result?)
}

/// Decodes one Core call response whose generated caller return type is unit.
pub fn decode_core_call_unit_response(response: CoreCallResponse) -> Result<(), CoreLinkError> {
    response.result.map(|_| ())
}

/// Creates one unbounded Core event stream channel pair.
pub fn core_event_stream_channel() -> (
    tokio::sync::mpsc::UnboundedSender<CoreEvent>,
    CoreEventStream,
) {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    (sender, CoreEventStream::new(receiver))
}

/// Adopts only empty attachment batches for route-owned values.
pub fn require_empty_core_stream_attachments() -> CoreStreamAttachmentAdopter {
    Arc::new(|attachments| {
        assert!(
            attachments.is_empty(),
            "route values must not capture anonymous Core streams"
        );
    })
}

/// Captures stream attachments while encoding one ordered watch update.
fn send_core_watch_value_with_attachments<T: Serialize>(
    sender: &tokio::sync::mpsc::UnboundedSender<CoreEvent>,
    previous_value: &StdMutex<Option<CoreValue>>,
    request_id: &operit_link::CoreRequestId,
    target_object_id: u32,
    property_name: &str,
    incremental: bool,
    attachment_adopter: &CoreStreamAttachmentAdopter,
    value: T,
) {
    let (encoded_value, attachments) = operit_link::withCoreStreamCaptureSync(|| {
        to_core_value(value).expect("Core watch value must serialize")
    });
    attachment_adopter(attachments);
    let (kind, value) = CoreValue::incrementalEvent(
        &mut *previous_value
            .lock()
            .expect("Core watch previous value mutex must not be poisoned"),
        encoded_value,
        incremental,
    );
    let _ = sender.send(CoreEvent {
        requestId: Some(request_id.clone()),
        targetObjectId: target_object_id,
        propertyName: property_name.to_string(),
        kind,
        value,
    });
}

/// Converts a preferences Flow into a Link Core event stream.
pub fn core_flow_event_stream<T>(
    flow: Flow<T>,
    request: CoreWatchRequest,
    attachment_adopter: CoreStreamAttachmentAdopter,
) -> Result<CoreEventStream, CoreLinkError>
where
    T: Serialize + Clone + Send + 'static,
{
    let (sender, receiver) = core_event_stream_channel();
    let incremental = request.acceptsIncrementalValues();
    let request_id = request.requestId;
    let target_object_id = request.targetObjectId;
    let property_name = request.propertyName;
    let previous_value = Arc::new(StdMutex::new(None::<CoreValue>));
    let previous_for_subscriber = previous_value.clone();
    let subscription = flow
        .subscribeWithCancellation(FlowCancellation::new(), move |value| {
            send_core_watch_value_with_attachments(
                &sender,
                previous_for_subscriber.as_ref(),
                &request_id,
                target_object_id,
                &property_name,
                incremental,
                &attachment_adopter,
                value,
            );
        })
        .map_err(|error| CoreLinkError::internal(error.to_string()))?;
    Ok(receiver.withOnClose(move || subscription.cancel()))
}

/// Converts a preferences StateFlow into a Link Core event stream.
pub fn core_state_flow_event_stream<T>(
    state_flow: StateFlow<T>,
    request: CoreWatchRequest,
    attachment_adopter: CoreStreamAttachmentAdopter,
) -> Result<CoreEventStream, CoreLinkError>
where
    T: Serialize + Clone + PartialEq + Send + 'static,
{
    let (sender, receiver) = core_event_stream_channel();
    let incremental = request.acceptsIncrementalValues();
    let request_id = request.requestId;
    let target_object_id = request.targetObjectId;
    let property_name = request.propertyName;
    let previous_value = Arc::new(StdMutex::new(None::<CoreValue>));
    let previous_for_subscriber = previous_value.clone();
    let subscription_state_flow = state_flow.clone();
    let subscription_id = state_flow.subscribe(move |value| {
        send_core_watch_value_with_attachments(
            &sender,
            previous_for_subscriber.as_ref(),
            &request_id,
            target_object_id,
            &property_name,
            incremental,
            &attachment_adopter,
            value,
        );
    });
    Ok(receiver.withOnClose(move || {
        subscription_state_flow.unsubscribe(subscription_id)
    }))
}

/// Converts a local route StateFlow into a Link Core event stream.
pub fn core_route_state_flow_event_stream<T>(
    state_flow: StateFlow<T>,
    request: CoreWatchRequest,
) -> Result<CoreEventStream, CoreLinkError>
where
    T: Serialize + Clone + PartialEq + Send + 'static,
{
    core_state_flow_event_stream(
        state_flow,
        request,
        require_empty_core_stream_attachments(),
    )
}

/// Reconstructs a local StateFlow from a remote Link Core event stream.
pub async fn core_state_flow_from_stream<T>(
    mut stream: CoreEventStream,
) -> Result<StateFlow<T>, CoreLinkError>
where
    T: DeserializeOwned + Clone + PartialEq + Send + 'static,
{
    let first = stream.recv().await.ok_or_else(|| {
        CoreLinkError::new(
            "WATCH_STREAM_EMPTY",
            "Core watch stream completed before its snapshot",
        )
    })?;
    if first.kind == CoreEventKind::Completed {
        return Err(CoreLinkError::new(
            "WATCH_STREAM_COMPLETED",
            "Core watch stream completed before its snapshot",
        ));
    }
    if first.kind == CoreEventKind::Delta {
        return Err(CoreLinkError::new(
            "WATCH_STREAM_DELTA_FIRST",
            "Core watch stream delivered a delta before its snapshot",
        ));
    }
    let initial_value = first.value.clone();
    let initial = from_core_response_value::<T>(initial_value.clone())?;
    let state_flow = operit_store::PreferencesDataStore::mutableStateFlow(initial);
    let state_for_task = state_flow.clone();
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "core-rslinkrs-state-flow",
            Box::new(move || {
                Box::pin(async move {
                    let mut previous_value = Some(initial_value);
                    while let Some(event) = stream.recv().await {
                        if event.kind == CoreEventKind::Completed {
                            break;
                        }
                        let value = if event.kind == CoreEventKind::Delta {
                            previous_value
                                .as_ref()
                                .expect("Core watch delta requires a previous value")
                                .applyIncrementalDelta(&event.value)
                                .expect("Core watch delta must apply to previous value")
                        } else {
                            event.value.clone()
                        };
                        previous_value = Some(value.clone());
                        let decoded = from_core_response_value::<T>(value)
                            .expect("Core watch value must decode into StateFlow item");
                        state_for_task.set_value(decoded);
                    }
                })
            }),
        )
        .map_err(|error| CoreLinkError::internal(error.to_string()))?;
    Ok(state_flow.asStateFlow())
}

/// Routes one annotation-generated StateFlow watch through the Core route runtime.
pub async fn core_route_state_flow<T>(
    runtime: &dyn CoreRouteRuntime,
    method_name: &str,
    args: CoreValue,
) -> Result<StateFlow<T>, CoreLinkError>
where
    T: DeserializeOwned + Clone + PartialEq + Send + 'static,
{
    let stream = runtime.watch(core_route_watch_request(method_name, args)).await?;
    core_state_flow_from_stream(stream).await
}

/// Forwards one Core event stream through an unbounded sender on the host scheduler.
pub fn forward_core_event_stream(
    mut stream: CoreEventStream,
    sender: tokio::sync::mpsc::UnboundedSender<CoreEvent>,
    task_name: &'static str,
) -> Result<(), CoreLinkError> {
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            task_name,
            Box::new(move || {
                Box::pin(async move {
                    while let Some(event) = stream.recv().await {
                        let _ = sender.send(event);
                    }
                })
            }),
        )
        .map_err(|error| CoreLinkError::internal(error.to_string()))
}

/// Converts a string stream into a Link Core event stream.
pub fn core_string_event_stream<S>(mut stream: S, request: CoreWatchRequest) -> CoreEventStream
where
    S: Stream<Item = String> + Send + 'static,
{
    let (sender, receiver) = core_event_stream_channel();
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "core-rslinkrs-string-events",
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

/// Converts a serializable stream into a Link Core event stream.
pub fn core_json_event_stream<S>(mut stream: S, request: CoreWatchRequest) -> CoreEventStream
where
    S: Stream + Send + 'static,
    S::Item: serde::Serialize,
{
    let (sender, receiver) = core_event_stream_channel();
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "core-rslinkrs-json-events",
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

/// Creates one typed reverse stream channel for generated Rust-to-Link-to-Rust calls.
pub fn core_reverse_stream_channel<T>() -> (ReverseStreamSender<T>, ReverseStream<T>) {
    ReverseStream::<T>::channel()
}

/// Generates one Core proxy request id using the host clock.
pub fn generated_proxy_request_id() -> String {
    let millis = operit_host_api::TimeUtils::currentTimeMillis();
    format!("core-proxy-{millis}")
}
