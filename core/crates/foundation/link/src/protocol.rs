use std::collections::BTreeMap;

use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tokio::sync::mpsc;


#[derive(Clone, Debug, PartialEq)]
pub enum CoreValue {
    Null,
    Bool(bool),
    Signed(i64),
    Unsigned(u64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    List(Vec<CoreValue>),
    Map(BTreeMap<String, CoreValue>),
}

impl CoreValue {
    /// Returns an empty structured argument map.
    pub fn emptyMap() -> Self {
        Self::Map(BTreeMap::new())
    }

    /// Produces the smallest safe event representation for one ordered value update.
    pub fn incrementalEvent(
        previous: &mut Option<CoreValue>,
        current: CoreValue,
    ) -> (CoreEventKind, CoreValue) {
        let Some(previousValue) = previous.as_ref() else {
            *previous = Some(current.clone());
            return (CoreEventKind::Snapshot, current);
        };
        let delta = buildCoreValueDelta(previousValue, &current);
        let event = if coreValueSize(&delta) < coreValueSize(&current) {
            (CoreEventKind::Delta, delta)
        } else {
            (CoreEventKind::Changed, current.clone())
        };
        *previous = Some(current);
        event
    }

    /// Applies one generic incremental value payload to a complete base value.
    pub fn applyIncrementalDelta(&self, delta: &CoreValue) -> Result<CoreValue, String> {
        self.clone().intoIncrementalDelta(delta)
    }

    /// Applies a delta by transferring ownership of the base, without cloning
    /// unchanged subtrees. On error the consumed base is dropped; callers that
    /// need rollback must use applyIncrementalDelta instead.
    pub fn intoIncrementalDelta(mut self, delta: &CoreValue) -> Result<CoreValue, String> {
        applyCoreValueDelta(&mut self, delta)?;
        Ok(self)
    }
}

const CORE_DELTA_MARKER: &str = "$coreDelta";

/// Builds a generic map/list delta between two serialized Core values.
fn buildCoreValueDelta(previous: &CoreValue, current: &CoreValue) -> CoreValue {
    let mut operations = Vec::new();
    collectCoreValueDelta(previous, current, &mut Vec::new(), &mut operations);
    let mut delta = BTreeMap::new();
    delta.insert(CORE_DELTA_MARKER.to_string(), CoreValue::List(operations));
    CoreValue::Map(delta)
}

/// Collects recursive set and remove operations for one value pair.
fn collectCoreValueDelta(
    previous: &CoreValue,
    current: &CoreValue,
    path: &mut Vec<CoreValue>,
    operations: &mut Vec<CoreValue>,
) {
    if previous == current {
        return;
    }
    match (previous, current) {
        (CoreValue::Map(previousValues), CoreValue::Map(currentValues)) => {
            for key in previousValues.keys() {
                if !currentValues.contains_key(key) {
                    path.push(CoreValue::String(key.clone()));
                    appendCoreDeltaOperation(operations, "remove", path, None);
                    path.pop();
                }
            }
            for (key, currentValue) in currentValues {
                path.push(CoreValue::String(key.clone()));
                match previousValues.get(key) {
                    Some(previousValue) => {
                        collectCoreValueDelta(previousValue, currentValue, path, operations)
                    }
                    None => appendCoreDeltaOperation(operations, "set", path, Some(currentValue)),
                }
                path.pop();
            }
        }
        (CoreValue::List(previousValues), CoreValue::List(currentValues)) => {
            let commonLength = previousValues.len().min(currentValues.len());
            for index in 0..commonLength {
                path.push(CoreValue::Unsigned(index as u64));
                collectCoreValueDelta(
                    &previousValues[index],
                    &currentValues[index],
                    path,
                    operations,
                );
                path.pop();
            }
            for index in (currentValues.len()..previousValues.len()).rev() {
                path.push(CoreValue::Unsigned(index as u64));
                appendCoreDeltaOperation(operations, "remove", path, None);
                path.pop();
            }
            for index in previousValues.len()..currentValues.len() {
                path.push(CoreValue::Unsigned(index as u64));
                appendCoreDeltaOperation(operations, "set", path, Some(&currentValues[index]));
                path.pop();
            }
        }
        _ => appendCoreDeltaOperation(operations, "set", path, Some(current)),
    }
}

/// Appends one encoded delta operation without interpreting business fields.
fn appendCoreDeltaOperation(
    operations: &mut Vec<CoreValue>,
    operation: &str,
    path: &[CoreValue],
    value: Option<&CoreValue>,
) {
    let mut fields = BTreeMap::new();
    fields.insert("op".to_string(), CoreValue::String(operation.to_string()));
    fields.insert("path".to_string(), CoreValue::List(path.to_vec()));
    if let Some(value) = value {
        fields.insert("value".to_string(), value.clone());
    }
    operations.push(CoreValue::Map(fields));
}

/// Applies all operations contained in one generic Core value delta.
fn applyCoreValueDelta(result: &mut CoreValue, delta: &CoreValue) -> Result<(), String> {
    let CoreValue::Map(deltaFields) = delta else {
        return Err("incremental delta must be a map".to_string());
    };
    let Some(CoreValue::List(operations)) = deltaFields.get(CORE_DELTA_MARKER) else {
        return Err("incremental delta marker is missing".to_string());
    };
    for operation in operations {
        applyCoreDeltaOperation(result, operation)?;
    }
    Ok(())
}

/// Applies one set or remove operation to a mutable Core value tree.
fn applyCoreDeltaOperation(target: &mut CoreValue, operation: &CoreValue) -> Result<(), String> {
    let CoreValue::Map(fields) = operation else {
        return Err("incremental operation must be a map".to_string());
    };
    let Some(CoreValue::String(operationName)) = fields.get("op") else {
        return Err("incremental operation name is missing".to_string());
    };
    let Some(CoreValue::List(path)) = fields.get("path") else {
        return Err("incremental operation path is missing".to_string());
    };
    match operationName.as_str() {
        "set" => {
            let Some(value) = fields.get("value") else {
                return Err("incremental set operation value is missing".to_string());
            };
            setCoreValueAtPath(target, path, value.clone())
        }
        "remove" => removeCoreValueAtPath(target, path),
        _ => Err(format!(
            "unsupported incremental operation: {operationName}"
        )),
    }
}

/// Replaces or appends one value at a typed map/list path.
fn setCoreValueAtPath(
    target: &mut CoreValue,
    path: &[CoreValue],
    value: CoreValue,
) -> Result<(), String> {
    if path.is_empty() {
        *target = value;
        return Ok(());
    }
    let segment = path
        .first()
        .expect("non-empty path must have a first segment");
    match target {
        CoreValue::Map(fields) => {
            let CoreValue::String(key) = segment else {
                return Err("map delta path segment must be a string".to_string());
            };
            if path.len() == 1 {
                fields.insert(key.clone(), value);
                return Ok(());
            }
            let child = fields
                .get_mut(key)
                .ok_or_else(|| format!("map delta path does not exist: {key}"))?;
            setCoreValueAtPath(child, &path[1..], value)
        }
        CoreValue::List(values) => {
            let CoreValue::Unsigned(index) = segment else {
                return Err("list delta path segment must be an unsigned index".to_string());
            };
            let index = usize::try_from(*index)
                .map_err(|_| format!("list delta index exceeds platform limits: {index}"))?;
            if path.len() == 1 {
                if index == values.len() {
                    values.push(value);
                } else if index < values.len() {
                    values[index] = value;
                } else {
                    return Err(format!("list delta append index is invalid: {index}"));
                }
                return Ok(());
            }
            let child = values
                .get_mut(index)
                .ok_or_else(|| format!("list delta path does not exist: {index}"))?;
            setCoreValueAtPath(child, &path[1..], value)
        }
        _ => Err("delta path traverses a scalar value".to_string()),
    }
}

/// Removes one value at a typed map/list path.
fn removeCoreValueAtPath(target: &mut CoreValue, path: &[CoreValue]) -> Result<(), String> {
    if path.is_empty() {
        return Err("cannot remove the root Core value".to_string());
    }
    let segment = path
        .first()
        .expect("non-empty path must have a first segment");
    match target {
        CoreValue::Map(fields) => {
            let CoreValue::String(key) = segment else {
                return Err("map delta path segment must be a string".to_string());
            };
            if path.len() == 1 {
                fields
                    .remove(key)
                    .map(|_| ())
                    .ok_or_else(|| format!("map delta removal path does not exist: {key}"))
            } else {
                let child = fields
                    .get_mut(key)
                    .ok_or_else(|| format!("map delta path does not exist: {key}"))?;
                removeCoreValueAtPath(child, &path[1..])
            }
        }
        CoreValue::List(values) => {
            let CoreValue::Unsigned(index) = segment else {
                return Err("list delta path segment must be an unsigned index".to_string());
            };
            let index = usize::try_from(*index)
                .map_err(|_| format!("list delta index exceeds platform limits: {index}"))?;
            if path.len() == 1 {
                if index < values.len() {
                    values.remove(index);
                    Ok(())
                } else {
                    Err(format!("list delta removal index is invalid: {index}"))
                }
            } else {
                let child = values
                    .get_mut(index)
                    .ok_or_else(|| format!("list delta path does not exist: {index}"))?;
                removeCoreValueAtPath(child, &path[1..])
            }
        }
        _ => Err("delta path traverses a scalar value".to_string()),
    }
}

/// Estimates the encoded size of one Core value for automatic delta selection.
fn coreValueSize(value: &CoreValue) -> usize {
    match value {
        CoreValue::Null => 1,
        CoreValue::Bool(_) => 2,
        CoreValue::Signed(_) | CoreValue::Unsigned(_) | CoreValue::Float(_) => 9,
        CoreValue::String(value) => value.len() + 1,
        CoreValue::Bytes(value) => value.len() + 1,
        CoreValue::List(values) => 2 + values.iter().map(coreValueSize).sum::<usize>(),
        CoreValue::Map(values) => {
            2 + values
                .iter()
                .map(|(key, value)| key.len() + coreValueSize(value))
                .sum::<usize>()
        }
    }
}

impl Serialize for CoreValue {
    /// Serializes a core value directly into the serializer's native data model.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Signed(value) => serializer.serialize_i64(*value),
            Self::Unsigned(value) => serializer.serialize_u64(*value),
            Self::Float(value) => serializer.serialize_f64(*value),
            Self::String(value) => serializer.serialize_str(value),
            Self::Bytes(value) => serializer.serialize_bytes(value),
            Self::List(values) => {
                let mut sequence = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    sequence.serialize_element(value)?;
                }
                sequence.end()
            }
            Self::Map(values) => {
                let mut map = serializer.serialize_map(Some(values.len()))?;
                for (key, value) in values {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}

struct CoreValueVisitor;

impl<'de> Visitor<'de> for CoreValueVisitor {
    type Value = CoreValue;

    /// Describes the native value forms accepted by CoreValue.
    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a Link core value")
    }

    /// Decodes a null value.
    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(CoreValue::Null)
    }

    /// Decodes a null optional value.
    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(CoreValue::Null)
    }

    /// Decodes a present optional value.
    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
    }

    /// Decodes a boolean value.
    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(CoreValue::Bool(value))
    }

    /// Decodes a signed integer value.
    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(CoreValue::Signed(value))
    }

    /// Decodes an unsigned integer value.
    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(CoreValue::Unsigned(value))
    }

    /// Decodes a floating-point value.
    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
        Ok(CoreValue::Float(value))
    }

    /// Decodes an owned string value.
    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(CoreValue::String(value))
    }

    /// Decodes a borrowed string value.
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(CoreValue::String(value.to_string()))
    }

    /// Decodes an owned binary value.
    fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E> {
        Ok(CoreValue::Bytes(value))
    }

    /// Decodes a borrowed binary value.
    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E> {
        Ok(CoreValue::Bytes(value.to_vec()))
    }

    /// Decodes a sequence value.
    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0));
        while let Some(value) = sequence.next_element()? {
            values.push(value);
        }
        Ok(CoreValue::List(values))
    }

    /// Decodes a string-keyed map value.
    fn visit_map<A>(self, mut entries: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = BTreeMap::new();
        while let Some((key, value)) = entries.next_entry::<String, CoreValue>()? {
            values.insert(key, value);
        }
        Ok(CoreValue::Map(values))
    }
}

impl<'de> Deserialize<'de> for CoreValue {
    /// Deserializes a core value directly from the serializer's native data model.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(CoreValueVisitor)
    }
}

/// Converts a serializable Rust value into the Link value model.
#[allow(non_snake_case)]
pub fn toCoreValue(value: impl Serialize) -> Result<CoreValue, crate::codec::CoreLinkCodecError> {
    crate::value_codec::to_value(value)
        .map_err(|e| crate::codec::CoreLinkCodecError::Encode(e.to_string()))
}

/// Converts a Link value into a typed Rust value.
#[allow(non_snake_case)]
pub fn fromCoreValue<T>(value: CoreValue) -> Result<T, crate::codec::CoreLinkCodecError>
where
    T: serde::de::DeserializeOwned,
{
    crate::value_codec::from_value(value)
        .map_err(|e| crate::codec::CoreLinkCodecError::Decode(e.to_string()))
}

pub struct CoreEventStream {
    receiver: EventReceiver,
    onClose: Option<Box<dyn FnOnce() + Send + 'static>>,
}

enum EventReceiver { Unbounded(mpsc::UnboundedReceiver<CoreEvent>), Bounded(mpsc::Receiver<CoreEvent>) }

impl CoreEventStream {
    /// Creates a stream together with its sender for an in-process Link source.
    pub fn channel() -> (mpsc::UnboundedSender<CoreEvent>, Self) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (sender, Self::new(receiver))
    }

    /// Wraps an event receiver as a link event stream.
    pub fn new(receiver: mpsc::UnboundedReceiver<CoreEvent>) -> Self {
        Self {
            receiver: EventReceiver::Unbounded(receiver),
            onClose: None,
        }
    }

    /// Bounded transport stream. A stalled consumer must not exhaust Edge RAM.
    pub fn boundedChannel(capacity: usize) -> (mpsc::Sender<CoreEvent>, Self) {
        let (sender, receiver) = mpsc::channel(capacity);
        (sender, Self { receiver: EventReceiver::Bounded(receiver), onClose: None })
    }

    /// Registers a callback that runs when the stream is dropped.
    #[allow(non_snake_case)]
    pub fn withOnClose(mut self, onClose: impl FnOnce() + Send + 'static) -> Self {
        let previous = self.onClose.take();
        self.onClose = Some(Box::new(move || {
            if let Some(previous) = previous {
                previous();
            }
            onClose();
        }));
        self
    }

    /// Waits for the next event from the stream.
    pub async fn recv(&mut self) -> Option<CoreEvent> {
        match &mut self.receiver { EventReceiver::Unbounded(r) => r.recv().await, EventReceiver::Bounded(r) => r.recv().await }
    }

    /// Polls with the caller's waker, without adding a task or a timer.
    pub(crate) fn poll_recv(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<CoreEvent>> {
        match &mut self.receiver { EventReceiver::Unbounded(r) => r.poll_recv(cx), EventReceiver::Bounded(r) => r.poll_recv(cx) }
    }

    /// Polls the stream for an already available event.
    pub fn try_recv(&mut self) -> Result<CoreEvent, mpsc::error::TryRecvError> {
        match &mut self.receiver { EventReceiver::Unbounded(r) => r.try_recv(), EventReceiver::Bounded(r) => r.try_recv() }
    }
}

impl Drop for CoreEventStream {
    fn drop(&mut self) {
        if let Some(onClose) = self.onClose.take() {
            onClose();
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CoreRequestId(pub String);

impl CoreRequestId {
    /// Creates a request identifier from a caller-provided value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// Fixed protocol address of the process-local embedded stream pool.
pub const CORE_STREAM_TARGET: &str = "$core.stream.open";
/// Carries the routed method or property that produced an embedded stream.
pub const CORE_ROUTE_STREAM_SOURCE_METHOD_ARGUMENT: &str = "$coreRouteStreamSourceMethod";
/// Carries the routed request mode that produced an embedded stream.
pub const CORE_ROUTE_STREAM_SOURCE_MODE_ARGUMENT: &str = "$coreRouteStreamSourceMode";
/// Carries the routed request arguments that produced an embedded stream.
pub const CORE_ROUTE_STREAM_SOURCE_ARGS_ARGUMENT: &str = "$coreRouteStreamSourceArgs";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoreMethodMode {
    Call,
    Watch,
    Push,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorePayloadKind {
    Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoreWatchInitial {
    None,
    Snapshot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreMethodProtocol {
    pub mode: CoreMethodMode,
    pub payload: CorePayloadKind,
    pub initial: CoreWatchInitial,
}

impl CoreMethodProtocol {
    /// Describes a structured value request/response method call.
    pub fn callValue() -> Self {
        Self {
            mode: CoreMethodMode::Call,
            payload: CorePayloadKind::Value,
            initial: CoreWatchInitial::None,
        }
    }

    /// Describes a structured value watch whose initial event behavior is explicit.
    pub fn watchValue(initial: CoreWatchInitial) -> Self {
        Self {
            mode: CoreMethodMode::Watch,
            payload: CorePayloadKind::Value,
            initial,
        }
    }

    /// Describes a client-owned structured value input stream.
    pub fn pushValue() -> Self {
        Self {
            mode: CoreMethodMode::Push,
            payload: CorePayloadKind::Value,
            initial: CoreWatchInitial::None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreCallRequest {
    pub requestId: CoreRequestId,
    pub target: String,
    pub methodName: String,
    pub args: CoreValue,
}

impl CoreCallRequest {
    /// Creates a serialized core call request.
    pub fn new(
        requestId: impl Into<String>,
        target: impl Into<String>,
        methodName: impl Into<String>,
        args: CoreValue,
    ) -> Self {
        Self {
            requestId: CoreRequestId::new(requestId),
            target: target.into(),
            methodName: methodName.into(),
            args,
        }
    }

    /// Returns the generated dispatch registry key for this call.
    pub fn registryKey(&self) -> String {
        format!("{}::{}", self.target, self.methodName)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreCallResponse {
    pub requestId: CoreRequestId,
    pub result: Result<CoreValue, CoreLinkError>,
}

/// Minimal device information shared by Link pairing without depending on a
/// full runtime's Access types.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkDeviceInfo {
    pub platform: String,
    pub model: String,
}

impl LinkDeviceInfo {
    /// Describes the local CLI device with its runtime role.
    #[allow(non_snake_case)]
    pub fn nativeCli(role: &str) -> Self {
        let mut deviceInfo = Self::native();
        deviceInfo.model = format!("{}(cli)-{}", role, deviceInfo.model);
        deviceInfo
    }

    pub fn native() -> Self {
        Self {
            platform: std::env::consts::OS.to_string(),
            model: std::env::consts::ARCH.to_string(),
        }
    }

    pub fn displayName(&self) -> String {
        [self.platform.trim(), self.model.trim()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }
}

impl CoreCallResponse {
    /// Creates a successful call response.
    pub fn ok(requestId: CoreRequestId, value: CoreValue) -> Self {
        Self {
            requestId,
            result: Ok(value),
        }
    }

    /// Creates a failed call response.
    pub fn err(requestId: CoreRequestId, error: CoreLinkError) -> Self {
        Self {
            requestId,
            result: Err(error),
        }
    }
}

/// Marks a watch request that accepts generic incremental value updates.
pub const CORE_INCREMENTAL_VALUES_ARGUMENT: &str = "$coreIncremental";

/// Identifies a request emitted by an annotation wrapper rather than a Dart proxy.
pub const CORE_INTERNAL_TARGET: &str = "$core.internal";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreWatchRequest {
    pub requestId: CoreRequestId,
    pub target: String,
    pub propertyName: String,
    pub args: CoreValue,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorePushRequest {
    pub requestId: CoreRequestId,
    pub target: String,
    pub methodName: String,
    #[serde(default = "CoreValue::emptyMap")]
    pub args: CoreValue,
}

impl CorePushRequest {
    /// Creates a client-owned input stream targeting one core method.
    pub fn new(
        requestId: impl Into<String>,
        target: impl Into<String>,
        methodName: impl Into<String>,
    ) -> Self {
        Self {
            requestId: CoreRequestId::new(requestId),
            target: target.into(),
            methodName: methodName.into(),
            args: CoreValue::emptyMap(),
        }
    }

    /// Attaches one-time non-stream arguments to this reverse stream open request.
    #[allow(non_snake_case)]
    pub fn withArgs(mut self, args: CoreValue) -> Self {
        self.args = args;
        self
    }
}

/// One standard application Link request. Transports must expose only these
/// three operation families; pairing/session control is not a Link operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum CoreLinkRequest {
    Call(CoreCallRequest),
    Watch(CoreLinkWatchRequest),
    Push(CoreLinkPushRequestMessage),
}

/// Watch lifecycle messages carried by the standard Link ingress.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum CoreLinkWatchRequest {
    Snapshot(CoreWatchRequest),
    Open(CoreWatchRequest),
    Close { requestId: CoreRequestId },
}

/// Push lifecycle messages carried by the standard Link ingress.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum CoreLinkPushRequestMessage {
    Open(CorePushRequest),
    Item(CorePushItem),
    Close { pushId: String },
}

/// Responses preserve the operation family and correlation even when an operation fails.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum CoreLinkResponse {
    Call(CoreCallResponse),
    Watch {
        requestId: CoreRequestId,
        result: Result<CoreLinkWatchResponse, CoreLinkError>,
    },
    Push {
        pushId: String,
        result: Result<CoreLinkPushResponse, CoreLinkError>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum CoreLinkWatchResponse {
    Snapshot(CoreEvent),
    Opened,
    Event(CoreEvent),
    Closed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum CoreLinkPushResponse {
    Opened,
    ItemAccepted { sequence: u64 },
    Closed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorePushItem {
    pub pushId: String,
    pub sequence: u64,
    pub args: CoreValue,
}

/// Selects the object namespace used by a routed Core request.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoutedCoreRequestKind {
    /// Addresses the destination through its local Core object namespace.
    #[default]
    Target,
    /// Addresses the destination through the annotation-generated Space route namespace.
    SpaceRoute,
    /// Resolves the generated Binding at the addressed adjacent Space router.
    /// The resolved request continues using the ordinary SpaceRoute namespace.
    SpaceBinding,
}

/// Carries one Core operation across the authenticated device-space topology.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoutedCoreRequest<T> {
    pub spaceId: String,
    /// Identifies the original CoreNode that authorized and initiated this request.
    pub originNodeId: String,
    pub targetNodeId: String,
    pub ttl: u32,
    #[serde(default)]
    pub routeKind: RoutedCoreRequestKind,
    pub payload: T,
}

/// Carries one asynchronous standard PeerLink message across any authenticated carrier.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeerFrame {
    pub messageId: String,
    pub payload: PeerFramePayload,
}

/// Carries an ordered batch of PeerLink frames on batch-oriented carriers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeerFrameBatch {
    pub frames: Vec<PeerFrame>,
}

/// Defines every message exchanged by a bidirectional Space PeerLink.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum PeerFramePayload {
    Request(PeerRequest),
    Response(PeerResponse),
    WatchEvent(PeerWatchEvent),
    WatchClosed(PeerWatchClosed),
    Heartbeat(PeerHeartbeat),
}

/// Carries a sequence-numbered heartbeat probe or its exact acknowledgement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum PeerHeartbeat {
    Probe { sequence: u64, sentAt: i64 },
    Ack { sequence: u64, sentAt: i64 },
}

/// Defines one routed Core operation requested by an adjacent Space member.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum PeerRequest {
    Call(RoutedCoreRequest<CoreCallRequest>),
    WatchSnapshot(RoutedCoreRequest<CoreWatchRequest>),
    WatchOpen(PeerWatchOpenRequest),
    WatchClose(PeerWatchCloseRequest),
    PushOpen(PeerPushOpenRequest),
    PushItem(CorePushItem),
    PushClose(PeerPushCloseRequest),
}

/// Defines the typed response to one PeerLink request.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum PeerResponse {
    Call(CoreCallResponse),
    WatchSnapshot(Result<CoreEvent, CoreLinkError>),
    Operation(Result<(), CoreLinkError>),
}

/// Opens one routed watch under a stable subscription identifier.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeerWatchOpenRequest {
    pub subscriptionId: String,
    pub request: RoutedCoreRequest<CoreWatchRequest>,
}

/// Closes one routed watch opened on the same PeerLink.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeerWatchCloseRequest {
    pub subscriptionId: String,
}

/// Delivers one routed watch event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeerWatchEvent {
    pub subscriptionId: String,
    pub event: CoreEvent,
}

/// Reports that a routed watch ended before a completion event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeerWatchClosed {
    pub subscriptionId: String,
    pub error: CoreLinkError,
}

/// Opens one routed push under a stable push identifier.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeerPushOpenRequest {
    pub pushId: String,
    pub request: RoutedCoreRequest<CorePushRequest>,
}

/// Closes one routed push opened on the same PeerLink.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeerPushCloseRequest {
    pub pushId: String,
}

impl CoreWatchRequest {
    /// Creates a serialized watch request.
    pub fn new(
        requestId: impl Into<String>,
        target: impl Into<String>,
        propertyName: impl Into<String>,
        args: CoreValue,
    ) -> Self {
        Self {
            requestId: CoreRequestId::new(requestId),
            target: target.into(),
            propertyName: propertyName.into(),
            args,
        }
    }

    /// Returns the generated dispatch registry key for this watch.
    pub fn registryKey(&self) -> String {
        format!("{}::{}", self.target, self.propertyName)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreEvent {
    pub requestId: Option<CoreRequestId>,
    pub target: String,
    pub propertyName: String,
    pub kind: CoreEventKind,
    pub value: CoreValue,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CoreEventKind {
    Snapshot,
    Changed,
    Delta,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreLinkError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<CoreValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<CoreLinkErrorLocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backtrace: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreLinkErrorLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

impl CoreLinkError {
    /// Creates a link error with a code and message.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            location: None,
            backtrace: None,
        }
    }

    /// Creates a link error with structured details.
    #[allow(non_snake_case)]
    pub fn withDetails(
        code: impl Into<String>,
        message: impl Into<String>,
        details: CoreValue,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: Some(details),
            location: None,
            backtrace: None,
        }
    }

    /// Creates the standard error for an unknown method registry key.
    pub fn methodNotFound(key: &str) -> Self {
        Self::new("METHOD_NOT_FOUND", format!("core method not found: {key}"))
    }

    /// Creates the standard error for an unknown watch registry key.
    pub fn watchNotFound(key: &str) -> Self {
        Self::new(
            "WATCH_NOT_FOUND",
            format!("core watch target not found: {key}"),
        )
    }

    /// Creates an error produced by command execution.
    pub fn command(message: impl Into<String>) -> Self {
        Self::new("COMMAND_ERROR", message)
    }

    /// Returns whether this error came from command execution.
    pub fn isCommandError(&self) -> bool {
        self.code == "COMMAND_ERROR"
    }

    /// Returns whether this error represents a route capability denial.
    #[allow(non_snake_case)]
    pub fn isRoutePermissionDenied(&self) -> bool {
        self.code == "ROUTE_PERMISSION_DENIED"
    }

    #[track_caller]
    /// Creates an internal link error annotated with caller location and backtrace.
    pub fn internal(message: impl Into<String>) -> Self {
        let caller = std::panic::Location::caller();
        // ESP-IDF's Xtensa unwind wrapper aborts instead of returning a
        // backtrace. Reporting a recoverable error must never reset the board.
        #[cfg(not(target_os = "espidf"))]
        let backtrace = Some(format!("{:#}", std::backtrace::Backtrace::force_capture()));
        #[cfg(target_os = "espidf")]
        let backtrace = None;
        Self {
            code: "INTERNAL_ERROR".to_string(),
            message: message.into(),
            details: None,
            location: Some(CoreLinkErrorLocation {
                file: caller.file().to_string(),
                line: caller.line(),
                column: caller.column(),
            }),
            // Full formatting retains instruction addresses even without debug symbols.
            backtrace,
        }
    }
}

impl std::fmt::Display for CoreLinkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)?;
        if self.code == "ROUTE_PERMISSION_DENIED" {
            if let Some(details) = &self.details {
                write!(formatter, "\nRoute permission details: {details:?}")?;
            }
        }
        if let Some(location) = &self.location {
            write!(
                formatter,
                "\nRust error location: {}:{}:{}",
                location.file, location.line, location.column
            )?;
        }
        if let Some(backtrace) = &self.backtrace {
            write!(formatter, "\nRust backtrace:\n{backtrace}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CoreLinkError {}

#[cfg(test)]
mod internal_error_tests {
    use super::CoreLinkError;

    #[test]
    fn internal_error_retains_message_and_caller_location() {
        let line = line!() + 1;
        let error = CoreLinkError::internal("NVS capacity exhausted");
        assert_eq!(error.code, "INTERNAL_ERROR");
        assert_eq!(error.message, "NVS capacity exhausted");
        let location = error.location.as_ref().unwrap();
        assert_eq!(location.line, line);
        assert_eq!(location.file, file!());
        #[cfg(not(target_os = "espidf"))]
        assert!(error.backtrace.is_some());
        #[cfg(target_os = "espidf")]
        assert!(error.backtrace.is_none());
    }
}
