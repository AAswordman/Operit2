use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CoreRequestId(pub String);

impl CoreRequestId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CoreObjectPath {
    pub segments: Vec<String>,
}

impl CoreObjectPath {
    pub fn root() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    pub fn parse(path: &str) -> Self {
        let segments = path
            .split('.')
            .map(str::trim)
            .filter(|segment| !segment.is_empty())
            .map(ToString::to_string)
            .collect();
        Self { segments }
    }

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

pub type CoreValue = Value;
pub type CoreEventStream = mpsc::UnboundedReceiver<CoreEvent>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreCallRequest {
    pub requestId: CoreRequestId,
    pub targetPath: CoreObjectPath,
    pub methodName: String,
    pub args: CoreValue,
}

impl CoreCallRequest {
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
    pub fn ok(requestId: CoreRequestId, value: CoreValue) -> Self {
        Self {
            requestId,
            result: Ok(value),
        }
    }

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreLinkError {
    pub code: String,
    pub message: String,
}

impl CoreLinkError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn methodNotFound(key: &str) -> Self {
        Self::new("METHOD_NOT_FOUND", format!("core method not found: {key}"))
    }

    pub fn watchNotFound(key: &str) -> Self {
        Self::new("WATCH_NOT_FOUND", format!("core watch target not found: {key}"))
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("INTERNAL_ERROR", message)
    }
}

impl std::fmt::Display for CoreLinkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CoreLinkError {}
