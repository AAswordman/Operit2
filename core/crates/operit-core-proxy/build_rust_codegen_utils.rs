use super::*;

pub(crate) fn object_uses_arc_mutex_instance(access: &ObjectAccess) -> bool {
    matches!(
        access,
        ObjectAccess::ContextGetInstanceArcMutexConstruct
            | ObjectAccess::ContextRefGetInstanceArcMutexConstruct
    )
}

pub(crate) fn json_string(value: &str) -> String {
    serde_json_escape(value)
}

pub(crate) fn option_json_string(value: Option<&str>) -> String {
    match value {
        Some(value) => serde_json_escape(value),
        None => "null".to_string(),
    }
}

fn serde_json_escape(value: &str) -> String {
    let mut result = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            other => result.push(other),
        }
    }
    result.push('"');
    result
}
