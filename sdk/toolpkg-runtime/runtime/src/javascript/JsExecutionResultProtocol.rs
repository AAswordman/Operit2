use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsExecutionFailure {
    pub message: String,
    pub dataText: String,
}

#[allow(non_snake_case)]
pub fn buildJsExecutionErrorPayload(message: &str) -> String {
    serde_json::json!({
        "success": false,
        "message": message.trim()
    })
    .to_string()
}

#[allow(non_snake_case)]
pub fn extractJsExecutionFailure(raw: Option<&str>) -> Option<JsExecutionFailure> {
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
    let dataText = match object.get("data") {
        Some(Value::Null) | None => String::new(),
        Some(Value::String(value)) => value.clone(),
        Some(value) => value.to_string(),
    };
    Some(JsExecutionFailure { message, dataText })
}

#[allow(non_snake_case)]
pub fn extractJsExecutionErrorMessage(raw: Option<&str>) -> Option<String> {
    extractJsExecutionFailure(raw).and_then(|failure| {
        if failure.message.is_empty() {
            None
        } else {
            Some(failure.message)
        }
    })
}

#[allow(non_snake_case)]
pub fn decodeJsExecutionResultValue(raw: Option<&str>) -> Value {
    let Some(raw) = raw else {
        return Value::Null;
    };
    let text = raw.trim();
    if text.is_empty() {
        return Value::Null;
    }
    serde_json::from_str::<Value>(text).unwrap_or_else(|_| Value::String(raw.to_string()))
}
