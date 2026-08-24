use std::fs;

pub fn parse_bool_arg(value: Option<&String>, usage: &str) -> Result<bool, String> {
    match value.ok_or_else(|| usage.to_string())?.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("invalid bool: {other}; expected true | false")),
    }
}

pub fn parse_i32_arg(value: Option<&String>, usage: &str) -> Result<i32, String> {
    value
        .ok_or_else(|| usage.to_string())?
        .parse::<i32>()
        .map_err(|error| error.to_string())
}

pub fn parse_f32_arg(value: Option<&String>, usage: &str) -> Result<f32, String> {
    value
        .ok_or_else(|| usage.to_string())?
        .parse::<f32>()
        .map_err(|error| error.to_string())
}

pub fn parse_i64_arg(value: Option<&String>, usage: &str) -> Result<i64, String> {
    value
        .ok_or_else(|| usage.to_string())?
        .parse::<i64>()
        .map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
pub fn parseCsvList(value: &str) -> Vec<String> {
    let mut result = Vec::new();
    for item in value.split(',') {
        let trimmed = item.trim();
        if !trimmed.is_empty() && !result.iter().any(|entry| entry == trimmed) {
            result.push(trimmed.to_string());
        }
    }
    result
}

pub fn parse_on_off_arg(value: Option<&String>, usage: &str) -> Result<bool, String> {
    match value.ok_or_else(|| usage.to_string())?.as_str() {
        "on" => Ok(true),
        "off" => Ok(false),
        other => Err(format!("invalid switch: {other}; expected on | off")),
    }
}

pub fn read_content_arg(value: &str) -> Result<String, String> {
    if let Some(path) = value.strip_prefix('@') {
        return fs::read_to_string(path).map_err(|error| error.to_string());
    }
    Ok(value.to_string())
}
