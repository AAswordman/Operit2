use crate::AppLogger::{AppLogger, VERBOSE_LEVEL_5};

pub const SEND_CHAIN: &str = "SendChain";
pub const MESSAGE_STORE_CHAIN: &str = "MessageStoreChain";
pub const RECEIVE_CHAIN: &str = "ReceiveChain";
pub const TOOL_CHAIN: &str = "ToolChain";
pub const PLUGIN_CHAIN: &str = "PluginChain";

#[allow(non_snake_case)]
/// Writes a useful business outcome or durable state change at Info.
pub fn info(tag: &str, event: &str, fields: &[(&str, String)]) {
    AppLogger::i(tag, &format_event(event, fields));
}

/// Writes routing, scheduling, persistence, or state propagation details at V5.
#[allow(non_snake_case)]
pub fn verbose(tag: &str, event: &str, fields: &[(&str, String)]) {
    verbose_with_level(tag, event, fields, VERBOSE_LEVEL_5);
}

/// Writes a structured diagnostic at the application's explicit Verbose depth.
#[allow(non_snake_case)]
pub fn verbose_with_level(tag: &str, event: &str, fields: &[(&str, String)], verbose_level: i32) {
    AppLogger::v_with_level(tag, &format_event(event, fields), verbose_level);
}

#[allow(non_snake_case)]
/// Writes an actionable warning without reducing its severity.
pub fn warn(tag: &str, event: &str, fields: &[(&str, String)]) {
    AppLogger::w(tag, &format_event(event, fields));
}

#[allow(non_snake_case)]
/// Writes a failed operation without reducing its severity.
pub fn error(tag: &str, event: &str, fields: &[(&str, String)]) {
    AppLogger::e(tag, &format_event(event, fields));
}

#[allow(non_snake_case)]
/// Formats one boolean field for a structured chain event.
pub fn boolField(value: bool) -> String {
    value.to_string()
}

#[allow(non_snake_case)]
/// Counts Unicode characters for a structured size field.
pub fn lenField(value: &str) -> String {
    value.chars().count().to_string()
}

/// Formats one event with escaped, line-safe fields.
fn format_event(event: &str, fields: &[(&str, String)]) -> String {
    let mut text = event.to_string();
    for (name, value) in fields {
        text.push(' ');
        text.push_str(name);
        text.push('=');
        text.push_str(&sanitize_value(value));
    }
    text
}

/// Escapes control characters without changing the field value.
fn sanitize_value(value: &str) -> String {
    value
        .replace('\r', "\\r")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}
