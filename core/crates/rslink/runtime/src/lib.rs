#![allow(non_snake_case)]

use async_trait::async_trait;
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkError,
    CoreLinkPushSession, CoreRouteRuntime, CoreStreamAttachment, CoreStreamDescriptor,
    CoreStreamSource, CoreValue, CoreWatchRequest, CORE_ROUTE_STREAM_SOURCE_ARGS_ARGUMENT,
    CORE_ROUTE_STREAM_SOURCE_METHOD_ARGUMENT, CORE_ROUTE_STREAM_SOURCE_MODE_ARGUMENT,
    CORE_STREAM_POOL_OBJECT_ID,
};
use operit_store::PreferencesDataStore::{Flow, FlowCancellation, StateFlow};
use operit_util::stream::ReverseStream::{ReverseStream, ReverseStreamSender};
use operit_util::stream::Stream::Stream;
use operit_util::AppLogger::AppLogger;
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
        if !attachments.is_empty() {
            AppLogger::i(
                "CoreRouteStream",
                &format!("pool.adopt_all count={}", attachments.len()),
            );
        }
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
        AppLogger::i(
            "CoreRouteStream",
            &format!(
                "pool.open requestId={} property={} streamId={}",
                request.requestId.0, request.propertyName, streamId
            ),
        );
        let source = self
            .sources
            .lock()
            .expect("core stream pool mutex poisoned")
            .get(&streamId)
            .cloned()
            .ok_or_else(|| {
                AppLogger::e(
                    "CoreStreamTrace",
                    &format!(
                        "pool.open.missing requestId={} property={} streamId={}",
                        request.requestId.0, request.propertyName, streamId
                    ),
                );
                CoreLinkError::watchNotFound(&request.registryKey())
            })?;
        AppLogger::i(
            "CoreRouteStream",
            &format!(
                "pool.open.ready requestId={} property={} streamId={}",
                request.requestId.0, request.propertyName, streamId
            ),
        );
        source.open(request)
    }

    /// Adopts one source captured while a Core value was serialized.
    fn adopt(&self, attachment: operit_link::CoreStreamAttachment) {
        let mut sources = self
            .sources
            .lock()
            .expect("core stream pool mutex poisoned");
        if let Some(existing) = sources.get(&attachment.streamId) {
            let relation = if Arc::ptr_eq(existing, &attachment.source) {
                "same"
            } else {
                "duplicate"
            };
            AppLogger::i(
                "CoreRouteStream",
                &format!("pool.adopt.{relation} streamId={}", attachment.streamId),
            );
        } else {
            AppLogger::i(
                "CoreRouteStream",
                &format!("pool.adopt.insert streamId={}", attachment.streamId),
            );
            sources.insert(attachment.streamId, attachment.source);
        }
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
pub fn from_core_response_value<T: DeserializeOwned>(value: CoreValue) -> Result<T, CoreLinkError> {
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
    let request = core_route_call_request(method_name, args);
    let requestId = request.requestId.0.clone();
    AppLogger::i(
        "CoreRouteTrace",
        &format!(
            "wrapper.call requestId={} method={}",
            requestId, method_name
        ),
    );
    let response = runtime.call(request).await;
    AppLogger::i(
        "CoreRouteTrace",
        &format!(
            "wrapper.call.result requestId={} method={} success={}",
            requestId,
            method_name,
            response.result.is_ok()
        ),
    );
    response
}

/// Decodes one Core call response into the generated caller return type.
pub fn decode_core_call_response<T: DeserializeOwned>(
    response: CoreCallResponse,
) -> Result<T, CoreLinkError> {
    from_core_response_value(response.result?)
}

/// Reports whether a route open error should leave execution on the local source.
pub fn core_route_error_uses_local_source(error: &CoreLinkError) -> bool {
    matches!(
        error.code.as_str(),
        "CORE_NODE_UNREACHABLE"
            | "PEER_LINK_CLOSED"
            | "PEER_RESPONSE_CLOSED"
            | "PEER_SEND_FAILED"
            | "PEER_WATCH_SOURCE_CLOSED"
    )
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
    Ok(receiver.withOnClose(move || subscription_state_flow.unsubscribe(subscription_id)))
}

/// Converts a local route StateFlow into a Link Core event stream.
pub fn core_route_state_flow_event_stream<T>(
    state_flow: StateFlow<T>,
    request: CoreWatchRequest,
) -> Result<CoreEventStream, CoreLinkError>
where
    T: Serialize + Clone + PartialEq + Send + 'static,
{
    core_state_flow_event_stream(state_flow, request, require_empty_core_stream_attachments())
}

/// Reconstructs a local StateFlow from a remote Link Core event stream.
pub async fn core_state_flow_from_stream<T>(
    stream: CoreEventStream,
) -> Result<StateFlow<T>, CoreLinkError>
where
    T: DeserializeOwned + Clone + PartialEq + Send + 'static,
{
    core_state_flow_from_stream_with_decoder(stream, Arc::new(from_core_response_value::<T>)).await
}

/// Reconstructs a local StateFlow using a caller-provided value decoder.
async fn core_state_flow_from_stream_with_decoder<T>(
    mut stream: CoreEventStream,
    decoder: Arc<dyn Fn(CoreValue) -> Result<T, CoreLinkError> + Send + Sync>,
) -> Result<StateFlow<T>, CoreLinkError>
where
    T: Clone + PartialEq + Send + 'static,
{
    let first = stream.recv().await.ok_or_else(|| {
        CoreLinkError::new(
            "WATCH_STREAM_EMPTY",
            "Core watch stream completed before its snapshot",
        )
    })?;
    AppLogger::i(
        "CoreRouteStream",
        &format!(
            "state_flow.first requestId={} targetObjectId={} property={} kind={:?} value={}",
            first
                .requestId
                .as_ref()
                .map(|requestId| requestId.0.as_str())
                .unwrap_or("<none>"),
            first.targetObjectId,
            first.propertyName,
            first.kind,
            core_value_shape(&first.value)
        ),
    );
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
    let initial = decoder(initial_value.clone())?;
    let state_flow = operit_store::PreferencesDataStore::mutableStateFlow(initial);
    let state_for_task = state_flow.clone();
    let decoder_for_task = decoder.clone();
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "core-rslinkrs-state-flow",
            Box::new(move || {
                Box::pin(async move {
                    let mut previous_value = Some(initial_value);
                    let mut event_count = 0_u64;
                    while let Some(event) = stream.recv().await {
                        event_count += 1;
                        let completed = event.kind == CoreEventKind::Completed;
                        if completed || should_log_core_route_event_count(event_count) {
                            AppLogger::i(
                                "CoreRouteStream",
                                &format!(
                                    "state_flow.event requestId={} targetObjectId={} property={} kind={:?} value={} count={} summary={}",
                                    event
                                        .requestId
                                        .as_ref()
                                        .map(|requestId| requestId.0.as_str())
                                        .unwrap_or("<none>"),
                                    event.targetObjectId,
                                    event.propertyName,
                                    event.kind,
                                    core_value_shape(&event.value),
                                    event_count,
                                    core_value_trace_summary(&event.value)
                                ),
                            );
                        }
                        if completed {
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
                        let decoded = decoder_for_task(value)
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
    runtime: Arc<dyn CoreRouteRuntime>,
    method_name: &str,
    args: CoreValue,
) -> Result<StateFlow<T>, CoreLinkError>
where
    T: DeserializeOwned + Clone + PartialEq + Send + 'static,
{
    AppLogger::i(
        "CoreRouteStream",
        &format!("wrapper.watch method={}", method_name),
    );
    let decoder = core_route_state_flow_value_decoder::<T>(
        runtime.clone(),
        method_name.to_string(),
        args.clone(),
    );
    let request = core_route_watch_request(method_name, args);
    AppLogger::i(
        "CoreRouteStream",
        &format!(
            "wrapper.watch.open requestId={} method={}",
            request.requestId.0, method_name
        ),
    );
    let stream = runtime.watch(request).await?;
    core_state_flow_from_stream_with_decoder(stream, decoder).await
}

/// Routes one annotation-generated StateFlow watch with an already opened local source stream.
pub async fn core_route_state_flow_with_local_source<T>(
    runtime: Arc<dyn CoreRouteRuntime>,
    method_name: &str,
    args: CoreValue,
    local_state_flow: StateFlow<T>,
) -> Result<StateFlow<T>, CoreLinkError>
where
    T: Serialize + DeserializeOwned + Clone + PartialEq + Send + 'static,
{
    AppLogger::i(
        "CoreRouteStream",
        &format!("wrapper.watch.local_source method={}", method_name),
    );
    let request = core_route_watch_request(method_name, args.clone());
    let sources = Arc::new(StdMutex::new(
        HashMap::<String, Arc<CoreStreamSource>>::new(),
    ));
    let sources_for_local_stream = sources.clone();
    let adopter: CoreStreamAttachmentAdopter = Arc::new(move |attachments| {
        adopt_core_stream_attachments(sources_for_local_stream.clone(), attachments);
    });
    let local_stream = core_state_flow_event_stream(local_state_flow, request.clone(), adopter)?;
    let decoder = core_route_state_flow_value_decoder_with_sources::<T>(
        runtime.clone(),
        method_name.to_string(),
        args,
        sources,
    );
    AppLogger::i(
        "CoreRouteStream",
        &format!(
            "wrapper.watch.local_source.open requestId={} method={}",
            request.requestId.0, method_name
        ),
    );
    let stream = runtime.watchWithLocalSource(request, local_stream).await?;
    core_state_flow_from_stream_with_decoder(stream, decoder).await
}

/// Creates a decoder that attaches routed stream sources to decoded CoreStream descriptors.
fn core_route_state_flow_value_decoder<T>(
    runtime: Arc<dyn CoreRouteRuntime>,
    source_method: String,
    source_args: CoreValue,
) -> Arc<dyn Fn(CoreValue) -> Result<T, CoreLinkError> + Send + Sync>
where
    T: DeserializeOwned + Send + 'static,
{
    let sources = Arc::new(StdMutex::new(
        HashMap::<String, Arc<CoreStreamSource>>::new(),
    ));
    core_route_state_flow_value_decoder_with_sources(runtime, source_method, source_args, sources)
}

/// Creates a decoder over a caller-owned stream source registry.
fn core_route_state_flow_value_decoder_with_sources<T>(
    runtime: Arc<dyn CoreRouteRuntime>,
    source_method: String,
    source_args: CoreValue,
    sources: Arc<StdMutex<HashMap<String, Arc<CoreStreamSource>>>>,
) -> Arc<dyn Fn(CoreValue) -> Result<T, CoreLinkError> + Send + Sync>
where
    T: DeserializeOwned + Send + 'static,
{
    Arc::new(move |value| {
        let runtime_for_resolver = runtime.clone();
        let source_method_for_resolver = source_method.clone();
        let source_args_for_resolver = source_args.clone();
        let sources_for_resolver = sources.clone();
        let resolver = Arc::new(move |descriptor: &CoreStreamDescriptor| {
            routed_core_stream_source_for_descriptor(
                sources_for_resolver.clone(),
                runtime_for_resolver.clone(),
                &source_method_for_resolver,
                source_args_for_resolver.clone(),
                descriptor,
            )
        });
        operit_link::withCoreStreamSourceResolverSync(resolver, || {
            from_core_response_value::<T>(value)
        })
    })
}

/// Adopts captured stream sources into one routed stream source registry.
fn adopt_core_stream_attachments(
    sources: Arc<StdMutex<HashMap<String, Arc<CoreStreamSource>>>>,
    attachments: Vec<CoreStreamAttachment>,
) {
    if !attachments.is_empty() {
        AppLogger::i(
            "CoreRouteStream",
            &format!("route.attachments.adopt count={}", attachments.len()),
        );
    }
    let mut sources = sources
        .lock()
        .expect("routed core stream source registry mutex poisoned");
    for attachment in attachments {
        if let Some(existing) = sources.get(&attachment.streamId) {
            let relation = if Arc::ptr_eq(existing, &attachment.source) {
                "same"
            } else {
                "duplicate"
            };
            AppLogger::i(
                "CoreRouteStream",
                &format!(
                    "route.attachments.{relation} streamId={}",
                    attachment.streamId
                ),
            );
        } else {
            AppLogger::i(
                "CoreRouteStream",
                &format!("route.attachments.insert streamId={}", attachment.streamId),
            );
            sources.insert(attachment.streamId, attachment.source);
        }
    }
}

/// Resolves one routed embedded stream descriptor into a stable local source.
fn routed_core_stream_source_for_descriptor(
    sources: Arc<StdMutex<HashMap<String, Arc<CoreStreamSource>>>>,
    runtime: Arc<dyn CoreRouteRuntime>,
    source_method: &str,
    source_args: CoreValue,
    descriptor: &CoreStreamDescriptor,
) -> Option<Arc<CoreStreamSource>> {
    if descriptor.targetObjectId != CORE_STREAM_POOL_OBJECT_ID
        || descriptor.propertyName != "openCoreStream"
    {
        AppLogger::i(
            "CoreRouteStream",
            &format!(
                "route.descriptor.skip streamId={} targetObjectId={} property={}",
                descriptor.streamId, descriptor.targetObjectId, descriptor.propertyName
            ),
        );
        return None;
    }
    let mut sources = sources
        .lock()
        .expect("routed core stream source registry mutex poisoned");
    if let Some(source) = sources.get(&descriptor.streamId) {
        AppLogger::i(
            "CoreRouteStream",
            &format!(
                "route.descriptor.existing streamId={} sourceMethod={}",
                descriptor.streamId, source_method
            ),
        );
        return Some(source.clone());
    }
    AppLogger::i(
        "CoreRouteStream",
        &format!(
            "route.descriptor.create streamId={} sourceMethod={}",
            descriptor.streamId, source_method
        ),
    );
    let source = Arc::new(core_route_embedded_stream_source(
        runtime,
        "watch",
        source_method.to_string(),
        source_args,
    ));
    sources.insert(descriptor.streamId.clone(), source.clone());
    Some(source)
}

/// Creates a local stream source that opens an embedded stream through route watch.
fn core_route_embedded_stream_source(
    runtime: Arc<dyn CoreRouteRuntime>,
    source_mode: &'static str,
    source_method: String,
    source_args: CoreValue,
) -> CoreStreamSource {
    CoreStreamSource::new(move |open_request| {
        let mut routed_open_request = open_request;
        let streamId = core_stream_id_argument(&routed_open_request.args);
        AppLogger::i(
            "CoreRouteStream",
            &format!(
                "embedded.open.prepare requestId={} streamId={} sourceMode={} sourceMethod={} property={}",
                routed_open_request.requestId.0,
                streamId.as_deref().unwrap_or("<missing>"),
                source_mode,
                source_method,
                routed_open_request.propertyName
            ),
        );
        routed_open_request.args = core_route_embedded_stream_open_args(
            routed_open_request.args,
            source_mode,
            source_method.clone(),
            source_args.clone(),
        )?;
        let runtime = runtime.clone();
        let (sender, receiver) = core_event_stream_channel();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "core-rslinkrs-routed-embedded-stream",
                Box::new(move || {
                    Box::pin(async move {
                        let request_id = routed_open_request.requestId.clone();
                        let target_object_id = routed_open_request.targetObjectId;
                        let property_name = routed_open_request.propertyName.clone();
                        let stream_id = core_stream_id_argument(&routed_open_request.args)
                            .unwrap_or_else(|| "<missing>".to_string());
                        AppLogger::i(
                            "CoreRouteStream",
                            &format!(
                                "embedded.watch.start requestId={} streamId={} property={}",
                                request_id.0, stream_id, property_name
                            ),
                        );
                        match runtime.watch(routed_open_request).await {
                            Ok(mut stream) => {
                                let mut event_count = 0_u64;
                                AppLogger::i(
                                    "CoreRouteStream",
                                    &format!(
                                        "embedded.watch.opened requestId={} streamId={} property={}",
                                        request_id.0, stream_id, property_name
                                    ),
                                );
                                while let Some(event) = stream.recv().await {
                                    event_count += 1;
                                    let completed = event.kind == CoreEventKind::Completed;
                                    if completed || should_log_core_route_event_count(event_count) {
                                        AppLogger::i(
                                            "CoreRouteStream",
                                            &format!(
                                                "embedded.watch.event requestId={} streamId={} eventProperty={} kind={:?} value={} count={} summary={}",
                                                request_id.0,
                                                stream_id,
                                                event.propertyName,
                                                event.kind,
                                                core_value_shape(&event.value),
                                                event_count,
                                                core_value_trace_summary(&event.value)
                                            ),
                                        );
                                    }
                                    if sender.send(event).is_err() {
                                        AppLogger::i(
                                            "CoreRouteStream",
                                            &format!(
                                                "embedded.watch.receiver_closed requestId={} streamId={}",
                                                request_id.0, stream_id
                                            ),
                                        );
                                        break;
                                    }
                                    if completed {
                                        AppLogger::i(
                                            "CoreRouteStream",
                                            &format!(
                                                "embedded.watch.completed requestId={} streamId={} events={}",
                                                request_id.0, stream_id, event_count
                                            ),
                                        );
                                        break;
                                    }
                                }
                            }
                            Err(error) => {
                                operit_util::AppLogger::AppLogger::e(
                                    "CoreRouteStream",
                                    &format!(
                                        "embedded_stream_open_failed requestId={} streamId={} property={} code={} error={}",
                                        request_id.0, stream_id, property_name, error.code, error
                                    ),
                                );
                                let _ = sender.send(CoreEvent {
                                    requestId: Some(request_id),
                                    targetObjectId: target_object_id,
                                    propertyName: property_name,
                                    kind: CoreEventKind::Completed,
                                    value: CoreValue::Null,
                                });
                            }
                        }
                    })
                }),
            )
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        Ok(receiver)
    })
}

/// Adds the route origin metadata required to reopen one embedded stream.
fn core_route_embedded_stream_open_args(
    open_args: CoreValue,
    source_mode: &'static str,
    source_method: String,
    source_args: CoreValue,
) -> Result<CoreValue, CoreLinkError> {
    let CoreValue::Map(mut arguments) = open_args else {
        return Err(CoreLinkError::new(
            "INVALID_ARGS",
            "embedded stream open arguments must be a map",
        ));
    };
    arguments.insert(
        CORE_ROUTE_STREAM_SOURCE_METHOD_ARGUMENT.to_string(),
        CoreValue::String(source_method),
    );
    arguments.insert(
        CORE_ROUTE_STREAM_SOURCE_MODE_ARGUMENT.to_string(),
        CoreValue::String(source_mode.to_string()),
    );
    arguments.insert(
        CORE_ROUTE_STREAM_SOURCE_ARGS_ARGUMENT.to_string(),
        source_args,
    );
    Ok(CoreValue::Map(arguments))
}

/// Returns the coarse shape of one Core value for protocol trace logs.
fn core_value_shape(value: &CoreValue) -> &'static str {
    match value {
        CoreValue::Null => "null",
        CoreValue::Bool(_) => "bool",
        CoreValue::Signed(_) => "signed",
        CoreValue::Unsigned(_) => "unsigned",
        CoreValue::Float(_) => "float",
        CoreValue::String(_) => "string",
        CoreValue::Bytes(_) => "bytes",
        CoreValue::List(_) => "list",
        CoreValue::Map(_) => "map",
    }
}

/// Builds one compact Core value summary for route diagnostics.
fn core_value_trace_summary(value: &CoreValue) -> String {
    match value {
        CoreValue::Null => "null".to_string(),
        CoreValue::Bool(value) => format!("bool={value}"),
        CoreValue::Signed(value) => format!("signed={value}"),
        CoreValue::Unsigned(value) => format!("unsigned={value}"),
        CoreValue::Float(value) => format!("float={value}"),
        CoreValue::String(value) => format!("stringLen={}", value.chars().count()),
        CoreValue::Bytes(value) => format!("bytesLen={}", value.len()),
        CoreValue::List(values) => format!(
            "listLen={} last={}",
            values.len(),
            values
                .last()
                .map(core_value_trace_summary)
                .unwrap_or_else(|| "none".to_string())
        ),
        CoreValue::Map(values) => {
            if let Some(event_type) = values.get("eventType") {
                let value_len = values.get("value").map(core_value_shape).unwrap_or("none");
                let id = values
                    .get("id")
                    .map(core_value_trace_field)
                    .unwrap_or_else(|| "none".to_string());
                let block_id = values
                    .get("blockId")
                    .map(core_value_trace_field)
                    .unwrap_or_else(|| "none".to_string());
                let inline_id = values
                    .get("inlineId")
                    .map(core_value_trace_field)
                    .unwrap_or_else(|| "none".to_string());
                let parent_block_id = values
                    .get("parentBlockId")
                    .map(core_value_trace_field)
                    .unwrap_or_else(|| "none".to_string());
                let node_type = values
                    .get("nodeType")
                    .map(core_value_trace_field)
                    .unwrap_or_else(|| "none".to_string());
                let header_level = values
                    .get("headerLevel")
                    .map(core_value_trace_field)
                    .unwrap_or_else(|| "none".to_string());
                return format!(
                    "markdown eventType={} value={} id={} blockId={} inlineId={} parentBlockId={} nodeType={} headerLevel={}",
                    core_value_trace_field(event_type),
                    value_len,
                    id,
                    block_id,
                    inline_id,
                    parent_block_id,
                    node_type,
                    header_level
                );
            }
            format!("mapKeys={}", values.len())
        }
    }
}

/// Reads one Core value as a compact trace field string.
fn core_value_trace_field(value: &CoreValue) -> String {
    match value {
        CoreValue::Null => "null".to_string(),
        CoreValue::Bool(value) => value.to_string(),
        CoreValue::Signed(value) => value.to_string(),
        CoreValue::Unsigned(value) => value.to_string(),
        CoreValue::Float(value) => value.to_string(),
        CoreValue::String(value) => value.clone(),
        CoreValue::Bytes(value) => format!("bytesLen={}", value.len()),
        CoreValue::List(values) => format!("listLen={}", values.len()),
        CoreValue::Map(values) => format!("mapKeys={}", values.len()),
    }
}

/// Returns whether one routed stream event count should be logged.
fn should_log_core_route_event_count(count: u64) -> bool {
    count <= 5 || count % 256 == 0
}

/// Reads the embedded stream identifier from a Link argument map for logs.
fn core_stream_id_argument(args: &CoreValue) -> Option<String> {
    match args {
        CoreValue::Map(arguments) => match arguments.get("streamId") {
            Some(CoreValue::String(value)) => Some(value.clone()),
            _ => None,
        },
        _ => None,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost;
    use operit_host_api::{
        HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    };
    use operit_link::{
        CoreEventKind, CoreRequestId, CoreRouteRuntime, CoreStream, CORE_INTERNAL_ROUTE_OBJECT_ID,
    };
    use serde::{Deserialize, Serialize};
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Once;
    use std::time::Duration;

    /// Schedules test runtime tasks on the current Tokio runtime.
    #[derive(Clone, Copy, Debug, Default)]
    struct TestHostRuntimeTaskScheduler;

    impl HostRuntimeTaskSchedulerHost for TestHostRuntimeTaskScheduler {
        /// Starts one synchronous test task on a native thread.
        fn scheduleHostRuntimeTask(
            &self,
            _taskName: &str,
            task: HostRuntimeTask,
        ) -> HostResult<()> {
            std::thread::spawn(task);
            Ok(())
        }

        /// Starts one asynchronous test task on the active Tokio runtime.
        fn scheduleHostRuntimeAsyncTask(
            &self,
            _taskName: &str,
            task: HostRuntimeAsyncTask,
        ) -> HostResult<()> {
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("create test route task runtime failed");
                runtime.block_on(task());
            });
            Ok(())
        }

        /// Starts one delayed test task on a native thread.
        fn scheduleDelayedHostRuntimeTask(
            &self,
            _taskName: &str,
            delayMs: u64,
            task: HostRuntimeTask,
        ) -> HostResult<()> {
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(delayMs));
                task();
            });
            Ok(())
        }

        /// Completes one test runtime turn immediately.
        fn waitForHostRuntimeTaskTurn(&self) -> operit_host_api::HostRuntimeTurnFuture {
            Box::pin(async { Ok(()) })
        }

        /// Completes one test runtime delay immediately.
        fn waitForHostRuntimeDelay(&self, _delayMs: u64) -> operit_host_api::HostRuntimeTurnFuture {
            Box::pin(async { Ok(()) })
        }
    }

    /// Installs the shared test scheduler once for this process.
    fn installTestRuntimeScheduler() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            setDefaultHostRuntimeTaskSchedulerHost(Arc::new(TestHostRuntimeTaskScheduler));
        });
    }

    /// Carries the embedded stream shape used by chat message flow values.
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct RoutedFlowValue {
        contentStream: Option<CoreStream<String>>,
    }

    /// Provides a route runtime that returns one Flow value with one embedded stream descriptor.
    #[derive(Clone)]
    struct EmbeddedStreamRouteRuntime {
        openedEmbeddedStream: Arc<AtomicBool>,
    }

    impl EmbeddedStreamRouteRuntime {
        /// Creates a route runtime that records embedded stream opens.
        fn new() -> Self {
            Self {
                openedEmbeddedStream: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    impl CoreRouteRuntime for EmbeddedStreamRouteRuntime {
        /// Sends every test invocation through the route runtime.
        fn shouldRoute(&self, _methodName: &str, _args: &CoreValue) -> Result<bool, CoreLinkError> {
            Ok(true)
        }

        /// Reports that this test route runtime does not implement calls.
        fn call(
            &self,
            request: CoreCallRequest,
        ) -> Pin<Box<dyn std::future::Future<Output = CoreCallResponse>>> {
            Box::pin(async move {
                CoreCallResponse::err(
                    request.requestId,
                    CoreLinkError::new("TEST_CALL_NOT_REGISTERED", "test call not registered"),
                )
            })
        }

        /// Opens either the source StateFlow stream or its embedded CoreStream.
        fn watch(
            &self,
            request: CoreWatchRequest,
        ) -> Pin<Box<dyn std::future::Future<Output = Result<CoreEventStream, CoreLinkError>>>>
        {
            let openedEmbeddedStream = self.openedEmbeddedStream.clone();
            Box::pin(async move {
                if request.targetObjectId == CORE_INTERNAL_ROUTE_OBJECT_ID
                    && request.propertyName == "chatMessagesFlow"
                {
                    let source = Arc::new(CoreStreamSource::new(|request| {
                        let (sender, receiver) = core_event_stream_channel();
                        sender
                            .send(CoreEvent {
                                requestId: Some(request.requestId.clone()),
                                targetObjectId: request.targetObjectId,
                                propertyName: request.propertyName.clone(),
                                kind: CoreEventKind::Changed,
                                value: CoreValue::String("unused-source".to_string()),
                            })
                            .expect("test stream receiver must be open");
                        Ok(receiver)
                    }));
                    let value = to_core_value(RoutedFlowValue {
                        contentStream: Some(CoreStream::fromSourceWithId(
                            "route-stream-1".to_string(),
                            source,
                        )),
                    })?;
                    let (sender, receiver) = core_event_stream_channel();
                    sender
                        .send(CoreEvent {
                            requestId: Some(request.requestId),
                            targetObjectId: request.targetObjectId,
                            propertyName: request.propertyName,
                            kind: CoreEventKind::Snapshot,
                            value,
                        })
                        .expect("test flow receiver must be open");
                    return Ok(receiver);
                }

                if request.targetObjectId == CORE_STREAM_POOL_OBJECT_ID
                    && request.propertyName == "openCoreStream"
                {
                    openedEmbeddedStream.store(true, Ordering::Release);
                    let arguments = object_args(request.args.clone())?;
                    assert_eq!(
                        arguments.get(CORE_ROUTE_STREAM_SOURCE_METHOD_ARGUMENT),
                        Some(&CoreValue::String("chatMessagesFlow".to_string()))
                    );
                    assert_eq!(
                        arguments.get(CORE_ROUTE_STREAM_SOURCE_MODE_ARGUMENT),
                        Some(&CoreValue::String("watch".to_string()))
                    );
                    assert!(arguments
                        .get(CORE_ROUTE_STREAM_SOURCE_ARGS_ARGUMENT)
                        .is_some());
                    let (sender, receiver) = core_event_stream_channel();
                    sender
                        .send(CoreEvent {
                            requestId: Some(request.requestId.clone()),
                            targetObjectId: request.targetObjectId,
                            propertyName: request.propertyName.clone(),
                            kind: CoreEventKind::Changed,
                            value: CoreValue::String("route-chunk".to_string()),
                        })
                        .expect("test embedded receiver must be open");
                    sender
                        .send(CoreEvent {
                            requestId: Some(request.requestId),
                            targetObjectId: request.targetObjectId,
                            propertyName: request.propertyName,
                            kind: CoreEventKind::Completed,
                            value: CoreValue::Null,
                        })
                        .expect("test embedded receiver must be open");
                    return Ok(receiver);
                }

                Err(CoreLinkError::new(
                    "TEST_WATCH_NOT_REGISTERED",
                    request.registryKey(),
                ))
            })
        }
    }

    /// Verifies routed StateFlow values preserve embedded stream sources when re-serialized.
    #[tokio::test(flavor = "current_thread")]
    async fn routed_state_flow_embedded_stream_opens_through_route_runtime() {
        installTestRuntimeScheduler();
        let runtime = Arc::new(EmbeddedStreamRouteRuntime::new());
        let stateFlow = core_route_state_flow::<RoutedFlowValue>(
            runtime.clone(),
            "chatMessagesFlow",
            CoreValue::Map(BTreeMap::from([(
                "chatId".to_string(),
                CoreValue::String("chat-1".to_string()),
            )])),
        )
        .await
        .expect("test route flow must open");

        let value = stateFlow.value();
        let streamDescriptor = value
            .contentStream
            .as_ref()
            .expect("test flow value must carry a stream")
            .descriptor
            .clone();
        let (_encodedValue, attachments) =
            operit_link::withCoreStreamCaptureSync(|| to_core_value(value).unwrap());
        assert_eq!(attachments.len(), 1);

        let streamPool = Arc::new(CoreStreamPool::new());
        streamPool.adoptAll(attachments);
        let mut stream = streamPool
            .openCoreStreamWatch(CoreWatchRequest::new(
                CoreRequestId::new("embedded-open").0,
                streamDescriptor.targetObjectId,
                streamDescriptor.propertyName,
                streamDescriptor.args,
            ))
            .expect("test embedded stream must open");

        let changed = tokio::time::timeout(Duration::from_secs(1), stream.recv())
            .await
            .expect("test embedded stream must emit")
            .expect("test embedded stream must stay open");
        assert_eq!(changed.kind, CoreEventKind::Changed);
        assert_eq!(changed.value, CoreValue::String("route-chunk".to_string()));
        assert!(runtime.openedEmbeddedStream.load(Ordering::Acquire));
    }

    /// Verifies duplicate attachments do not reopen one logical stream.
    #[tokio::test(flavor = "current_thread")]
    async fn stream_pool_keeps_one_source_for_duplicate_logical_stream_id() {
        let streamId = "duplicate-logical-stream".to_string();
        let firstSource = Arc::new(CoreStreamSource::new(|request| {
            let (sender, receiver) = core_event_stream_channel();
            sender
                .send(CoreEvent {
                    requestId: Some(request.requestId.clone()),
                    targetObjectId: request.targetObjectId,
                    propertyName: request.propertyName.clone(),
                    kind: CoreEventKind::Changed,
                    value: CoreValue::String("first-source".to_string()),
                })
                .expect("test stream receiver must be open");
            sender
                .send(CoreEvent {
                    requestId: Some(request.requestId),
                    targetObjectId: request.targetObjectId,
                    propertyName: request.propertyName,
                    kind: CoreEventKind::Completed,
                    value: CoreValue::Null,
                })
                .expect("test stream receiver must be open");
            Ok(receiver)
        }));
        let secondSource = Arc::new(CoreStreamSource::new(|request| {
            let (sender, receiver) = core_event_stream_channel();
            sender
                .send(CoreEvent {
                    requestId: Some(request.requestId.clone()),
                    targetObjectId: request.targetObjectId,
                    propertyName: request.propertyName.clone(),
                    kind: CoreEventKind::Changed,
                    value: CoreValue::String("second-source".to_string()),
                })
                .expect("test stream receiver must be open");
            sender
                .send(CoreEvent {
                    requestId: Some(request.requestId),
                    targetObjectId: request.targetObjectId,
                    propertyName: request.propertyName,
                    kind: CoreEventKind::Completed,
                    value: CoreValue::Null,
                })
                .expect("test stream receiver must be open");
            Ok(receiver)
        }));
        let streamPool = Arc::new(CoreStreamPool::new());
        streamPool.adoptAll(vec![
            CoreStreamAttachment {
                streamId: streamId.clone(),
                source: firstSource,
            },
            CoreStreamAttachment {
                streamId: streamId.clone(),
                source: secondSource,
            },
        ]);

        let mut stream = streamPool
            .openCoreStreamWatch(CoreWatchRequest::new(
                "duplicate-logical-open",
                CORE_STREAM_POOL_OBJECT_ID,
                "openCoreStream",
                CoreValue::Map(BTreeMap::from([(
                    "streamId".to_string(),
                    CoreValue::String(streamId),
                )])),
            ))
            .expect("duplicate logical stream must open");

        let first = stream
            .recv()
            .await
            .expect("duplicate logical stream must emit first event");
        assert_eq!(first.value, CoreValue::String("first-source".to_string()));
        let completed = stream
            .recv()
            .await
            .expect("duplicate logical stream must complete");
        assert_eq!(completed.kind, CoreEventKind::Completed);
        assert!(stream.recv().await.is_none());
    }
}
