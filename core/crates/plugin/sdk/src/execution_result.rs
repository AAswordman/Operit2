use std::error::Error;
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Classifies failures produced by the JavaScript execution boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JsExecutionErrorKind {
    Initialization,
    InvalidRequest,
    Protocol,
    Runtime,
    Serialization,
    Timeout,
    WorkerUnavailable,
}

/// Describes one typed JavaScript execution failure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsExecutionError {
    pub kind: JsExecutionErrorKind,
    pub message: String,
}

impl JsExecutionError {
    /// Creates one JavaScript execution failure with an explicit category.
    pub fn new(kind: JsExecutionErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Creates an initialization failure.
    pub fn initialization(message: impl Into<String>) -> Self {
        Self::new(JsExecutionErrorKind::Initialization, message)
    }

    /// Creates an invalid-request failure.
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(JsExecutionErrorKind::InvalidRequest, message)
    }

    /// Creates a protocol decoding failure.
    pub fn protocol(message: impl Into<String>) -> Self {
        Self::new(JsExecutionErrorKind::Protocol, message)
    }

    /// Creates a JavaScript runtime failure.
    pub fn runtime(message: impl Into<String>) -> Self {
        Self::new(JsExecutionErrorKind::Runtime, message)
    }

    /// Creates a serialization failure.
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::new(JsExecutionErrorKind::Serialization, message)
    }

    /// Creates an execution timeout failure.
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(JsExecutionErrorKind::Timeout, message)
    }

    /// Creates a worker communication failure.
    pub fn worker_unavailable(message: impl Into<String>) -> Self {
        Self::new(JsExecutionErrorKind::WorkerUnavailable, message)
    }
}

impl Display for JsExecutionError {
    /// Formats the execution failure message.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for JsExecutionError {}

/// Represents a typed result produced by JavaScript execution.
pub type JsExecutionResult<T> = Result<T, JsExecutionError>;

/// Structured failure reported by the JavaScript execution contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsExecutionFailure {
    pub message: String,
    pub data_text: String,
}

/// Builds the JSON payload used to report JavaScript execution failure.
pub fn build_js_execution_error_payload(message: &str) -> String {
    serde_json::json!({
        "success": false,
        "message": message.trim()
    })
    .to_string()
}

/// Extracts a structured JavaScript failure from a raw execution payload.
pub fn extract_js_execution_failure(raw: Option<&str>) -> Option<JsExecutionFailure> {
    let text = raw.unwrap_or_default().trim();
    if text.is_empty() {
        return None;
    }
    let parsed = serde_json::from_str::<Value>(text).ok()?;
    let object = parsed.as_object()?;
    if !object.contains_key("success")
        || object
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    {
        return None;
    }
    let message = object
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let data_text = match object.get("data") {
        Some(Value::Null) | None => String::new(),
        Some(Value::String(value)) => value.clone(),
        Some(value) => value.to_string(),
    };
    Some(JsExecutionFailure { message, data_text })
}

/// Extracts only the JavaScript execution error message from a raw payload.
pub fn extract_js_execution_error_message(raw: Option<&str>) -> Option<String> {
    extract_js_execution_failure(raw).and_then(|failure| {
        if failure.message.is_empty() {
            None
        } else {
            Some(failure.message)
        }
    })
}

/// Decodes a JavaScript execution result as strict JSON.
pub fn decode_js_execution_result_value(raw: Option<&str>) -> JsExecutionResult<Value> {
    let Some(raw) = raw else {
        return Ok(Value::Null);
    };
    let text = raw.trim();
    if text.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str::<Value>(text).map_err(|error| {
        JsExecutionError::protocol(format!("invalid JavaScript execution result JSON: {error}"))
    })
}
