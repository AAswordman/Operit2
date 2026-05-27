use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MCPToolParameter {
    pub name: String,
    pub parameter_type: String,
    pub description: String,
    pub required: bool,
    pub defaultValue: Option<String>,
}

impl MCPToolParameter {
    #[allow(non_snake_case)]
    pub fn convertParameterValue(&self, value: Value) -> Value {
        match value {
            Value::String(text) => {
                Self::smartConvert(Value::String(text), Some(&self.parameter_type))
            }
            other => other,
        }
    }

    #[allow(non_snake_case)]
    pub fn smartConvert(value: Value, typeName: Option<&str>) -> Value {
        match value {
            Value::Array(items) => Value::Array(
                items
                    .into_iter()
                    .map(|item| Self::smartConvert(item, None))
                    .collect(),
            ),
            Value::String(text) => match typeName.map(|value| value.to_ascii_lowercase()) {
                Some(value) if value == "number" => parseNumberValue(&text),
                Some(value) if value == "boolean" => {
                    Value::Bool(text.to_ascii_lowercase() == "true")
                }
                Some(value) if value == "integer" => text
                    .parse::<i64>()
                    .map(|number| serde_json::json!(number))
                    .unwrap_or(Value::String(text)),
                Some(value) if value == "float" || value == "double" => text
                    .parse::<f64>()
                    .map(|number| serde_json::json!(number))
                    .unwrap_or(Value::String(text)),
                Some(value) if value == "array" => parseArrayValue(&text),
                Some(value) if value == "object" => parseObjectValue(&text),
                _ => guessValue(&text),
            },
            other => other,
        }
    }
}

#[allow(non_snake_case)]
fn parseNumberValue(text: &str) -> Value {
    if text.contains('.') {
        text.parse::<f64>()
            .map(|number| serde_json::json!(number))
            .unwrap_or_else(|_| Value::String(text.to_string()))
    } else {
        text.parse::<i64>()
            .map(|number| serde_json::json!(number))
            .unwrap_or_else(|_| Value::String(text.to_string()))
    }
}

#[allow(non_snake_case)]
fn parseArrayValue(text: &str) -> Value {
    let trimmed = text.trim();
    if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(trimmed) {
        return Value::Array(
            items
                .into_iter()
                .map(|item| MCPToolParameter::smartConvert(item, None))
                .collect(),
        );
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let content = trimmed[1..trimmed.len() - 1].trim();
        if content
            .chars()
            .all(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '-' | ',' | ' '))
        {
            return Value::Array(
                content
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(|item| {
                        MCPToolParameter::smartConvert(Value::String(item.to_string()), None)
                    })
                    .collect(),
            );
        }
    }
    Value::String(text.to_string())
}

#[allow(non_snake_case)]
fn parseObjectValue(text: &str) -> Value {
    let trimmed = text.trim();
    if let Ok(Value::Object(object)) = serde_json::from_str::<Value>(trimmed) {
        let converted = object
            .into_iter()
            .map(|(key, value)| (key, MCPToolParameter::smartConvert(value, None)))
            .collect();
        return Value::Object(converted);
    }
    Value::String(text.to_string())
}

#[allow(non_snake_case)]
fn guessValue(text: &str) -> Value {
    let trimmed = text.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return parseObjectValue(text);
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return parseArrayValue(text);
    }
    if trimmed
        .chars()
        .all(|ch| ch.is_ascii_digit() || matches!(ch, '-' | '.'))
        && trimmed.chars().any(|ch| ch.is_ascii_digit())
    {
        return parseNumberValue(trimmed);
    }
    if matches!(trimmed.to_ascii_lowercase().as_str(), "true" | "false") {
        return Value::Bool(trimmed.eq_ignore_ascii_case("true"));
    }
    Value::String(text.to_string())
}
