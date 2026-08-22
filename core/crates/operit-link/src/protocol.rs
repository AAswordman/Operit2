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
        incremental: bool,
    ) -> (CoreEventKind, CoreValue) {
        let Some(previousValue) = previous.as_ref() else {
            *previous = Some(current.clone());
            return (CoreEventKind::Snapshot, current);
        };
        if !incremental {
            *previous = Some(current.clone());
            return (CoreEventKind::Changed, current);
        }
        let Some(delta) = buildCoreValueDelta(previousValue, &current) else {
            *previous = Some(current.clone());
            return (CoreEventKind::Changed, current);
        };
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
        applyCoreValueDelta(self, delta)
    }
}

const CORE_DELTA_MARKER: &str = "$coreDelta";

/// Builds a generic map/list delta between two serialized Core values.
fn buildCoreValueDelta(previous: &CoreValue, current: &CoreValue) -> Option<CoreValue> {
    if previous == current {
        return None;
    }
    let mut operations = Vec::new();
    collectCoreValueDelta(previous, current, &mut Vec::new(), &mut operations);
    let mut delta = BTreeMap::new();
    delta.insert(CORE_DELTA_MARKER.to_string(), CoreValue::List(operations));
    Some(CoreValue::Map(delta))
}

/// Collects recursive set and remove operations for one value pair.
fn collectCoreValueDelta(
    previous: &CoreValue,
    current: &CoreValue,
    path: &mut Vec<CoreValue>,
    operations: &mut Vec<CoreValue>,
) {
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
fn applyCoreValueDelta(base: &CoreValue, delta: &CoreValue) -> Result<CoreValue, String> {
    let CoreValue::Map(deltaFields) = delta else {
        return Err("incremental delta must be a map".to_string());
    };
    let Some(CoreValue::List(operations)) = deltaFields.get(CORE_DELTA_MARKER) else {
        return Err("incremental delta marker is missing".to_string());
    };
    let mut result = base.clone();
    for operation in operations {
        applyCoreDeltaOperation(&mut result, operation)?;
    }
    Ok(result)
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
    crate::codec::decodeLink(&crate::codec::encodeLink(value)?)
}

/// Converts a Link value into a typed Rust value.
#[allow(non_snake_case)]
pub fn fromCoreValue<T>(value: CoreValue) -> Result<T, crate::codec::CoreLinkCodecError>
where
    T: serde::de::DeserializeOwned,
{
    crate::codec::decodeLink(&crate::codec::encodeLink(value)?)
}

pub struct CoreEventStream {
    receiver: mpsc::UnboundedReceiver<CoreEvent>,
    onClose: Option<Box<dyn FnOnce() + Send + 'static>>,
}

impl CoreEventStream {
    /// Creates a stream together with its sender for an in-process Link source.
    pub fn channel() -> (
        mpsc::UnboundedSender<CoreEvent>,
        Self,
    ) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (sender, Self::new(receiver))
    }

    /// Wraps an event receiver as a link event stream.
    pub fn new(receiver: mpsc::UnboundedReceiver<CoreEvent>) -> Self {
        Self {
            receiver,
            onClose: None,
        }
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
        self.receiver.recv().await
    }

    /// Polls the stream for an already available event.
    pub fn try_recv(&mut self) -> Result<CoreEvent, mpsc::error::TryRecvError> {
        self.receiver.try_recv()
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
pub const CORE_STREAM_POOL_OBJECT_ID: u32 = u32::MAX;

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
    pub targetObjectId: u32,
    pub methodName: String,
    pub args: CoreValue,
}

impl CoreCallRequest {
    /// Creates a serialized core call request.
    pub fn new(
        requestId: impl Into<String>,
        targetObjectId: u32,
        methodName: impl Into<String>,
        args: CoreValue,
    ) -> Self {
        Self {
            requestId: CoreRequestId::new(requestId),
            targetObjectId,
            methodName: methodName.into(),
            args,
        }
    }

    /// Returns the generated dispatch registry key for this call.
    pub fn registryKey(&self) -> String {
        format!("{}::{}", self.targetObjectId, self.methodName)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreCallResponse {
    pub requestId: CoreRequestId,
    pub result: Result<CoreValue, CoreLinkError>,
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

pub const CORE_INCREMENTAL_VALUES_ARGUMENT: &str = "$coreIncremental";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreWatchRequest {
    pub requestId: CoreRequestId,
    pub targetObjectId: u32,
    pub propertyName: String,
    pub args: CoreValue,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorePushRequest {
    pub requestId: CoreRequestId,
    pub targetObjectId: u32,
    pub methodName: String,
    #[serde(default = "CoreValue::emptyMap")]
    pub args: CoreValue,
}

impl CorePushRequest {
    /// Creates a client-owned input stream targeting one core method.
    pub fn new(
        requestId: impl Into<String>,
        targetObjectId: u32,
        methodName: impl Into<String>,
    ) -> Self {
        Self {
            requestId: CoreRequestId::new(requestId),
            targetObjectId,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorePushItem {
    pub pushId: String,
    pub sequence: u64,
    pub args: CoreValue,
}

impl CoreWatchRequest {
    /// Creates a serialized watch request.
    pub fn new(
        requestId: impl Into<String>,
        targetObjectId: u32,
        propertyName: impl Into<String>,
        args: CoreValue,
    ) -> Self {
        Self {
            requestId: CoreRequestId::new(requestId),
            targetObjectId,
            propertyName: propertyName.into(),
            args,
        }
    }

    /// Returns the generated dispatch registry key for this watch.
    pub fn registryKey(&self) -> String {
        format!("{}::{}", self.targetObjectId, self.propertyName)
    }

    /// Reports whether this subscriber accepts generic incremental values.
    pub fn acceptsIncrementalValues(&self) -> bool {
        matches!(
            &self.args,
            CoreValue::Map(arguments)
                if arguments.get(CORE_INCREMENTAL_VALUES_ARGUMENT) == Some(&CoreValue::Bool(true))
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreEvent {
    pub requestId: Option<CoreRequestId>,
    pub targetObjectId: u32,
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

    #[track_caller]
    /// Creates an internal link error annotated with caller location and backtrace.
    pub fn internal(message: impl Into<String>) -> Self {
        let caller = std::panic::Location::caller();
        let backtrace = std::backtrace::Backtrace::force_capture();
        Self {
            code: "INTERNAL_ERROR".to_string(),
            message: message.into(),
            details: None,
            location: Some(CoreLinkErrorLocation {
                file: caller.file().to_string(),
                line: caller.line(),
                column: caller.column(),
            }),
            backtrace: Some(backtrace.to_string()),
        }
    }
}

impl std::fmt::Display for CoreLinkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)?;
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
