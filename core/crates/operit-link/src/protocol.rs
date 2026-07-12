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
        self.onClose = Some(Box::new(onClose));
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CoreObjectPath {
    pub segments: Vec<String>,
}

impl CoreObjectPath {
    /// Returns the root object path.
    pub fn root() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Parses a dot-delimited object path into path segments.
    pub fn parse(path: &str) -> Self {
        Self {
            segments: path
                .split('.')
                .map(str::trim)
                .filter(|segment| !segment.is_empty())
                .map(ToString::to_string)
                .collect(),
        }
    }

    /// Joins path segments into the canonical registry key.
    pub fn key(&self) -> String {
        self.segments.join(".")
    }
}

impl From<&str> for CoreObjectPath {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for CoreObjectPath {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoreMethodMode {
    Call,
    Watch,
    Push,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorePayloadKind {
    Value,
    TextStreamEvent,
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

    /// Describes a watch stream that emits rendered text stream events.
    pub fn watchTextStreamEvent() -> Self {
        Self {
            mode: CoreMethodMode::Watch,
            payload: CorePayloadKind::TextStreamEvent,
            initial: CoreWatchInitial::None,
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
    pub targetPath: CoreObjectPath,
    pub methodName: String,
    pub args: CoreValue,
}

impl CoreCallRequest {
    /// Creates a serialized core call request.
    pub fn new(
        requestId: impl Into<String>,
        targetPath: impl Into<CoreObjectPath>,
        methodName: impl Into<String>,
        args: CoreValue,
    ) -> Self {
        Self {
            requestId: CoreRequestId::new(requestId),
            targetPath: targetPath.into(),
            methodName: methodName.into(),
            args,
        }
    }

    /// Returns the generated dispatch registry key for this call.
    pub fn registryKey(&self) -> String {
        format!("{}::{}", self.targetPath.key(), self.methodName)
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreWatchRequest {
    pub requestId: CoreRequestId,
    pub targetPath: CoreObjectPath,
    pub propertyName: String,
    pub args: CoreValue,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorePushRequest {
    pub requestId: CoreRequestId,
    pub targetPath: CoreObjectPath,
    pub methodName: String,
}

impl CorePushRequest {
    /// Creates a client-owned input stream targeting one core method.
    pub fn new(
        requestId: impl Into<String>,
        targetPath: impl Into<CoreObjectPath>,
        methodName: impl Into<String>,
    ) -> Self {
        Self {
            requestId: CoreRequestId::new(requestId),
            targetPath: targetPath.into(),
            methodName: methodName.into(),
        }
    }

    /// Builds the one-shot call represented by one ordered push item.
    pub fn itemCall(&self, sequence: u64, args: CoreValue) -> CoreCallRequest {
        CoreCallRequest {
            requestId: CoreRequestId::new(format!("{}:{sequence}", self.requestId.0)),
            targetPath: self.targetPath.clone(),
            methodName: self.methodName.clone(),
            args,
        }
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
        targetPath: impl Into<CoreObjectPath>,
        propertyName: impl Into<String>,
        args: CoreValue,
    ) -> Self {
        Self {
            requestId: CoreRequestId::new(requestId),
            targetPath: targetPath.into(),
            propertyName: propertyName.into(),
            args,
        }
    }

    /// Returns the generated dispatch registry key for this watch.
    pub fn registryKey(&self) -> String {
        format!("{}::{}", self.targetPath.key(), self.propertyName)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreEvent {
    pub requestId: Option<CoreRequestId>,
    pub targetPath: CoreObjectPath,
    pub propertyName: String,
    pub kind: CoreEventKind,
    pub value: CoreValue,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CoreEventKind {
    Snapshot,
    Changed,
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
