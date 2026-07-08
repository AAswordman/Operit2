use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

pub type CoreValue = Value;

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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorePayloadKind {
    Json,
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
    /// Describes a JSON request/response method call.
    pub fn callJson() -> Self {
        Self {
            mode: CoreMethodMode::Call,
            payload: CorePayloadKind::Json,
            initial: CoreWatchInitial::None,
        }
    }

    /// Describes a JSON watch whose initial event behavior is explicit.
    pub fn watchJson(initial: CoreWatchInitial) -> Self {
        Self {
            mode: CoreMethodMode::Watch,
            payload: CorePayloadKind::Json,
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
    pub details: Option<Value>,
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
        details: Value,
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
