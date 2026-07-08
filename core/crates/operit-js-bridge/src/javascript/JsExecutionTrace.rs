use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

/// Listener for observable events emitted by JavaScript execution.
pub trait JsExecutionListener {
    /// Records a log message for a running JavaScript call.
    fn onCallLog(&mut self, _callId: &str, _level: &str, _message: &str) {}

    /// Records an intermediate result emitted by a running JavaScript call.
    fn onIntermediateResult(&mut self, _callId: &str, _value: &Value) {}

    /// Records successful completion for a JavaScript call.
    fn onCompleted(&mut self, _callId: &str, _result: &str) {}

    /// Records failure for a JavaScript call.
    fn onFailed(&mut self, _callId: &str, _error: &str) {}
}

#[derive(Clone, Debug)]
/// Captures a compact execution trace for one JavaScript function call.
pub struct JsExecutionTraceRecorder {
    scriptPath: String,
    functionName: String,
    paramsJson: String,
    envFilePath: Option<String>,
    startedAtMs: u128,
    events: Vec<String>,
}

impl JsExecutionTraceRecorder {
    /// Creates a trace recorder for one JavaScript execution request.
    pub fn new(
        scriptPath: String,
        functionName: String,
        paramsJson: String,
        envFilePath: Option<String>,
    ) -> Self {
        Self {
            scriptPath,
            functionName,
            paramsJson,
            envFilePath,
            startedAtMs: nowMillis(),
            events: Vec::new(),
        }
    }

    /// Builds the standard trace payload for a completed execution.
    pub fn buildPayload(&self, success: bool, result: Value, error: Option<String>) -> Value {
        self.buildResultData(success, result, error, None, None, None)
    }

    /// Builds a detailed trace payload with execution mode metadata.
    pub fn buildResultData(
        &self,
        success: bool,
        result: Value,
        error: Option<String>,
        executionMode: Option<String>,
        scriptLabel: Option<String>,
        requestedWaitMs: Option<u64>,
    ) -> Value {
        let finishedAtMs = nowMillis();
        json!({
            "success": success,
            "scriptPath": self.scriptPath,
            "functionName": self.functionName,
            "params": parseJsonText(&self.paramsJson),
            "envFilePath": self.envFilePath,
            "startedAtMs": self.startedAtMs,
            "finishedAtMs": finishedAtMs,
            "durationMs": finishedAtMs.saturating_sub(self.startedAtMs),
            "result": result,
            "error": error.filter(|error| !error.trim().is_empty()),
            "events": self.events,
            "executionMode": executionMode,
            "scriptLabel": scriptLabel,
            "requestedWaitMs": requestedWaitMs
        })
    }
}

impl JsExecutionListener for JsExecutionTraceRecorder {
    fn onCallLog(&mut self, _callId: &str, level: &str, message: &str) {
        let normalizedLevel = level.trim().to_ascii_uppercase();
        let normalizedMessage = collapseWhitespace(message);
        if normalizedMessage.is_empty() {
            return;
        }
        if normalizedLevel.is_empty() {
            self.events.push(normalizedMessage);
        } else {
            self.events
                .push(format!("{normalizedLevel}: {normalizedMessage}"));
        }
    }

    fn onIntermediateResult(&mut self, _callId: &str, value: &Value) {
        let summary = summarizeValue(value, 240, 0);
        if !summary.is_empty() {
            self.events.push(format!("intermediate: {summary}"));
        }
    }

    fn onCompleted(&mut self, _callId: &str, result: &str) {
        let summary = collapseWhitespace(result);
        if !summary.is_empty() {
            self.events
                .push(format!("completed: {}", truncateText(&summary, 240)));
        }
    }

    fn onFailed(&mut self, _callId: &str, error: &str) {
        let normalizedError = collapseWhitespace(error);
        if !normalizedError.is_empty() {
            self.events.push(format!("failed: {normalizedError}"));
        }
    }
}

fn parseJsonText(text: &str) -> Value {
    let normalized = text.trim();
    if normalized.is_empty() {
        return json!({});
    }
    serde_json::from_str(normalized).unwrap_or_else(|_| Value::String(normalized.to_string()))
}

fn summarizeValue(value: &Value, maxLength: usize, depth: usize) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => truncateText(&collapseWhitespace(text), maxLength),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => summarizeArray(values, maxLength, depth),
        Value::Object(values) => summarizeObject(values, maxLength, depth),
    }
}

fn summarizeArray(values: &[Value], maxLength: usize, depth: usize) -> String {
    if depth >= 2 {
        return format!("[{} items]", values.len());
    }
    let mut items = values
        .iter()
        .take(3)
        .map(|value| summarizeValue(value, 80, depth + 1))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if values.len() > 3 {
        items.push(format!("+{} more", values.len() - 3));
    }
    truncateText(&format!("[{}]", items.join(", ")), maxLength)
}

fn summarizeObject(
    values: &serde_json::Map<String, Value>,
    maxLength: usize,
    depth: usize,
) -> String {
    if values.is_empty() {
        return "{}".to_string();
    }
    if depth >= 2 {
        let preview = values
            .keys()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let suffix = if values.len() > 3 { ", ..." } else { "" };
        return format!("{{{preview}{suffix}}}");
    }
    let mut parts = values
        .iter()
        .take(4)
        .filter_map(|(key, value)| {
            let summary = summarizeValue(value, 120, depth + 1);
            if summary.is_empty() {
                None
            } else {
                Some(format!("{key}={summary}"))
            }
        })
        .collect::<Vec<_>>();
    if values.len() > 4 {
        parts.push(format!("+{} more", values.len() - 4));
    }
    truncateText(&parts.join("; "), maxLength)
}

fn collapseWhitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncateText(text: &str, maxLength: usize) -> String {
    if text.chars().count() <= maxLength {
        return text.to_string();
    }
    text.chars()
        .take(maxLength.saturating_sub(3))
        .collect::<String>()
        + "..."
}

fn nowMillis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after unix epoch")
        .as_millis()
}
