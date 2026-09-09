use regex::escape;
use serde_json::{json, Map, Value};

/// Converts one Operit1 thinking configuration document to the current rule schema.
pub(crate) fn convert_thinking_configurations(
    input: &str,
    provider_type_id: &str,
) -> Result<String, String> {
    let source: Value = serde_json::from_str(input).map_err(|error| error.to_string())?;
    let rules = match source {
        Value::Array(rules) => rules,
        Value::Object(mut object) => object
            .remove("rules")
            .and_then(|value| value.as_array().cloned())
            .ok_or_else(|| "Operit1 thinking configuration must contain rules".to_string())?,
        _ => return Err("Operit1 thinking configuration must be an array or object".to_string()),
    };
    let mut converted = Vec::new();
    for rule in rules {
        let object = rule
            .as_object()
            .ok_or_else(|| "Operit1 thinking rule must be an object".to_string())?;
        if object.get("enabled").and_then(Value::as_bool) == Some(false) {
            continue;
        }
        converted.push(convert_rule(object, provider_type_id)?);
    }
    serde_json::to_string(&converted).map_err(|error| error.to_string())
}

/// Converts one legacy rule while preserving matching, actions, labels, and option IDs.
fn convert_rule(source: &Map<String, Value>, provider_type_id: &str) -> Result<Value, String> {
    let matcher = source.get("match").and_then(Value::as_object);
    let parameter_label = optional_string(source, "parameterLabel")?
        .or(optional_string(source, "label")?)
        .unwrap_or_default();
    let mut providers = string_list(source, "providers")?;
    providers.extend(string_list(source, "providerTypeIds")?);
    bind_provider_scope(&mut providers, provider_type_id);
    let model_prefix = matcher_values(source, matcher, "modelPrefix")?;
    let mut model_regex = matcher_values(source, matcher, "modelRegex")?;
    for value in matcher_values(source, matcher, "modelContains")? {
        model_regex.push(format!("(?i){}", escape(&value)));
    }
    for value in matcher_values(source, matcher, "modelSuffix")? {
        model_regex.push(format!("(?i){}$", escape(&value)));
    }
    for value in matcher_values(source, matcher, "firstSegment")? {
        model_regex.push(format!("(?i)^{}(?:/|$)", escape(&value)));
    }
    for value in matcher_values(source, matcher, "lastSegmentPrefix")? {
        model_regex.push(format!("(?i)(?:^|/){}[^/]*$", escape(&value)));
    }
    for value in matcher_values(source, matcher, "lastSegmentContains")? {
        model_regex.push(format!("(?i)(?:^|/)[^/]*{}[^/]*$", escape(&value)));
    }
    for value in matcher_values(source, matcher, "lastSegmentRegex")? {
        model_regex.push(format!("(?i)(?:^|/)[^/]*(?:{value})[^/]*$"));
    }
    let mut endpoint_suffix = string_list(source, "endpointSuffix")?;
    if let Some(matcher) = matcher {
        endpoint_suffix.extend(string_list(matcher, "endpointSuffix")?);
    }
    let control = match optional_string(source, "control")?.as_deref() {
        Some("levels") => "levels",
        Some("toggle_only") | Some("toggle") => "toggle_only",
        Some("unsupported") | None => "unsupported",
        Some(value) => return Err(format!("unknown Operit1 thinking control: {value}")),
    };
    let required = optional_bool(source, "required")?
        .or(optional_bool(source, "reasoningRequired")?)
        .unwrap_or(false);
    Ok(json!({
        "providers": providers,
        "model_prefix": model_prefix,
        "model_regex": model_regex,
        "endpoint_suffix": endpoint_suffix,
        "control": control,
        "required": required,
        "enable": action_list(source, &["enable", "enabledActions", "on"])? ,
        "disable": action_list(source, &["disable", "disabledActions", "off"])? ,
        "options": options(source, &parameter_label)?,
    }))
}

/// Binds provider-scoped legacy rules to the model config that owns them.
fn bind_provider_scope(providers: &mut Vec<String>, provider_type_id: &str) {
    let provider_type_id = provider_type_id.trim();
    if providers.is_empty() && !provider_type_id.is_empty() {
        providers.push(provider_type_id.to_string());
    }
}

/// Reads an optional string property from a legacy JSON object.
fn optional_string(source: &Map<String, Value>, key: &str) -> Result<Option<String>, String> {
    match source.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.trim().to_string())),
        Some(_) => Err(format!("Operit1 thinking field {key} must be a string")),
    }
}

/// Reads an optional boolean property from a legacy JSON object.
fn optional_bool(source: &Map<String, Value>, key: &str) -> Result<Option<bool>, String> {
    match source.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(format!("Operit1 thinking field {key} must be boolean")),
    }
}

/// Reads a legacy string list represented as a scalar or array.
fn string_list(source: &Map<String, Value>, key: &str) -> Result<Vec<String>, String> {
    match source.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::String(value)) => Ok(vec![value.trim().to_string()]),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| match value {
                Value::String(value) => Ok(value.trim().to_string()),
                _ => Err(format!("Operit1 thinking field {key} must contain strings")),
            })
            .collect(),
        Some(_) => Err(format!("Operit1 thinking field {key} must be a string list")),
    }
}

/// Combines matcher values declared on a rule and its nested matcher object.
fn matcher_values(
    source: &Map<String, Value>,
    matcher: Option<&Map<String, Value>>,
    key: &str,
) -> Result<Vec<String>, String> {
    let mut values = string_list(source, key)?;
    if let Some(matcher) = matcher {
        values.extend(string_list(matcher, key)?);
    }
    Ok(values)
}

/// Converts legacy enable and disable action declarations.
fn action_list(source: &Map<String, Value>, keys: &[&str]) -> Result<Vec<Value>, String> {
    let mut actions = Vec::new();
    for key in keys {
        let Some(value) = source.get(*key) else { continue };
        let entries: Vec<&Value> = match value {
            Value::Array(values) => values.iter().collect(),
            Value::Object(_) => vec![value],
            _ => return Err(format!("Operit1 thinking field {key} must be an action list")),
        };
        for entry in entries {
            let object = entry
                .as_object()
                .ok_or_else(|| format!("Operit1 thinking field {key} has invalid action"))?;
            let path = optional_string(object, "path")?
                .filter(|path| !path.is_empty())
                .ok_or_else(|| format!("Operit1 thinking action {key} has no path"))?;
            let value = object
                .get("value")
                .cloned()
                .ok_or_else(|| format!("Operit1 thinking action {key} has no value"))?;
            actions.push(json!({"path": path, "value": value}));
        }
    }
    Ok(actions)
}

/// Converts legacy options and preserves direct actions.
fn options(source: &Map<String, Value>, parameter_label: &str) -> Result<Vec<Value>, String> {
    let Some(value) = source.get("options") else { return Ok(Vec::new()) };
    let values = value
        .as_array()
        .ok_or_else(|| "Operit1 thinking options must be an array".to_string())?;
    values
        .iter()
        .map(|entry| {
            let object = entry
                .as_object()
                .ok_or_else(|| "Operit1 thinking option must be an object".to_string())?;
            let option_value = object.get("value").cloned();
            let id = optional_string(object, "id")?
                .filter(|id| !id.is_empty())
                .or_else(|| option_value.as_ref().map(option_id))
                .ok_or_else(|| "Operit1 thinking option has no id".to_string())?;
            let label = optional_string(object, "label")?
                .filter(|label| !label.is_empty())
                .unwrap_or_else(|| id.clone());
            let path = optional_string(object, "path")?.unwrap_or_else(|| parameter_label.to_string());
            Ok(json!({
                "id": id,
                "label": label,
                "path": path,
                "value": option_value,
                "actions": action_list(object, &["actions"])?,
            }))
        })
        .collect()
}

/// Creates a stable option identifier from a legacy JSON value.
fn option_id(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        value => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::convert_thinking_configurations;
    use serde_json::Value;

    /// Verifies provider-scoped legacy rules are bound to the owning provider profile.
    #[test]
    fn binds_provider_scoped_rules_to_owner_provider() {
        let source = r#"
        [
          {
            "control": "levels",
            "parameterLabel": "reasoning.effort",
            "options": [
              {"id": "low", "label": "low", "value": "low"}
            ]
          }
        ]
        "#;
        let converted = convert_thinking_configurations(source, "DEEPSEEK")
            .expect("thinking rules must convert");
        let rules: Vec<Value> =
            serde_json::from_str(&converted).expect("converted rules must be JSON");
        assert_eq!(rules[0]["providers"][0], "DEEPSEEK");
    }
}
