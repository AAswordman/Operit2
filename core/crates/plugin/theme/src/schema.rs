//! Small evaluator for the exact keyword subset in the checked-in v1 schemas.
//! These schemas are trusted compile-time inputs, not supplied by theme authors.
use crate::ThemeValidationError;
use regex::Regex;
use serde_json::Value;

pub(crate) fn resolve<'a>(root: &'a Value, node: &'a Value) -> &'a Value {
    match node.get("$ref") {
        Some(reference) => {
            let pointer = reference.as_str().expect("schema reference must be a string")
                .strip_prefix('#').expect("only local schema references are supported");
            root.pointer(pointer).expect("schema reference must exist")
        }
        None => node,
    }
}

pub(crate) fn validate(
    root: &Value, node: &Value, value: &Value, path: &str, require_fields: bool,
) -> Result<(), ThemeValidationError> {
    let node = resolve(root, node);
    let type_matches = |kind: &str| match kind {
        "null" => value.is_null(),
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        // Version and byte counts intentionally reject 1.0 as well as 1.5.
        "integer" => value.is_u64() || value.is_i64(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        _ => panic!("unsupported trusted schema type"),
    };
    if let Some(kind) = node.get("type") {
        let matches = match kind {
            Value::String(kind) => type_matches(kind),
            Value::Array(kinds) => kinds.iter().any(|k| type_matches(k.as_str().unwrap())),
            _ => panic!("invalid trusted schema type"),
        };
        if !matches { return Err(ThemeValidationError::contract(path, "incorrect JSON type")); }
    }
    if let Some(expected) = node.get("const") {
        if value != expected { return Err(ThemeValidationError::contract(path, "unsupported contract value")); }
    }
    if let Some(choices) = node.get("enum") {
        if !choices.as_array().unwrap().contains(value) {
            return Err(ThemeValidationError::contract(path, "unregistered enum value"));
        }
    }
    if let Some(object) = value.as_object() {
        let properties = node["properties"].as_object().expect("object schema must be closed");
        for (key, child) in object {
            let child_path = format!("{path}/{}", key.replace('~', "~0").replace('/', "~1"));
            match properties.get(key) {
                Some(child_schema) => validate(root, child_schema, child, &child_path, require_fields)?,
                None if node["additionalProperties"] == Value::Bool(true) => {},
                None => return Err(ThemeValidationError::contract(&child_path, "unknown field")),
            }
        }
        if require_fields {
            if let Some(required) = node.get("required") {
                for key in required.as_array().unwrap() {
                    let key = key.as_str().unwrap();
                    if !object.contains_key(key) {
                        return Err(ThemeValidationError::contract(&format!("{path}/{key}"), "required field is missing"));
                    }
                }
            }
        }
    }
    if let Some(array) = value.as_array() {
        check_bound(node, "minItems", array.len() as f64, path, true)?;
        check_bound(node, "maxItems", array.len() as f64, path, false)?;
        if node.get("uniqueItems") == Some(&Value::Bool(true)) {
            for (index, item) in array.iter().enumerate() {
                if array[..index].contains(item) {
                    return Err(ThemeValidationError::contract(path, "duplicate array item"));
                }
            }
        }
        for (index, item) in array.iter().enumerate() {
            validate(root, &node["items"], item, &format!("{path}/{index}"), require_fields)?;
        }
    }
    if let Some(string) = value.as_str() {
        check_bound(node, "minLength", string.chars().count() as f64, path, true)?;
        check_bound(node, "maxLength", string.chars().count() as f64, path, false)?;
        if let Some(pattern) = node.get("pattern") {
            let regex = Regex::new(pattern.as_str().unwrap()).expect("invalid trusted schema regex");
            if !regex.is_match(string) {
                return Err(ThemeValidationError::contract(path, "string is outside the registered format"));
            }
        }
    }
    if let Some(number) = value.as_f64() {
        check_bound(node, "minimum", number, path, true)?;
        check_bound(node, "maximum", number, path, false)?;
    }
    Ok(())
}

fn check_bound(node: &Value, key: &str, value: f64, path: &str, minimum: bool) -> Result<(), ThemeValidationError> {
    if let Some(bound) = node.get(key) {
        let bound = bound.as_f64().unwrap();
        if (minimum && value < bound) || (!minimum && value > bound) {
            return Err(ThemeValidationError::contract(path, format!("violates {key}={bound}")));
        }
    }
    Ok(())
}

/// Expands only schema-defined defaults. Invalid authored values are never replaced.
pub(crate) fn normalize(root: &Value, node: &Value, value: &mut Value) {
    let node = resolve(root, node);
    if let Some(object) = value.as_object_mut() {
        for (key, child_schema) in node["properties"].as_object().unwrap() {
            let child_schema = resolve(root, child_schema);
            if !object.contains_key(key) {
                if let Some(default) = child_schema.get("default") {
                    object.insert(key.clone(), default.clone());
                }
            }
            if let Some(child) = object.get_mut(key) { normalize(root, child_schema, child); }
        }
    } else if let Some(array) = value.as_array_mut() {
        for child in array { normalize(root, &node["items"], child); }
    } else if node.get("type") == Some(&Value::String("number".into())) {
        let number = value.as_f64().unwrap();
        if number.fract() == 0.0 {
            *value = Value::from(number as i64);
        }
    }
}
