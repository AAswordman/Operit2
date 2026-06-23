use operit_util::AppLogger::AppLogger;

pub const SEND_CHAIN: &str = "SendChain";
pub const MESSAGE_STORE_CHAIN: &str = "MessageStoreChain";
pub const RECEIVE_CHAIN: &str = "ReceiveChain";
pub const TOOL_CHAIN: &str = "ToolChain";
pub const PLUGIN_CHAIN: &str = "PluginChain";

#[allow(non_snake_case)]
pub fn info(tag: &str, event: &str, fields: &[(&str, String)]) {
    AppLogger::i(tag, &format_event(event, fields));
}

#[allow(non_snake_case)]
pub fn warn(tag: &str, event: &str, fields: &[(&str, String)]) {
    AppLogger::w(tag, &format_event(event, fields));
}

#[allow(non_snake_case)]
pub fn error(tag: &str, event: &str, fields: &[(&str, String)]) {
    AppLogger::e(tag, &format_event(event, fields));
}

#[allow(non_snake_case)]
pub fn boolField(value: bool) -> String {
    value.to_string()
}

#[allow(non_snake_case)]
pub fn lenField(value: &str) -> String {
    value.chars().count().to_string()
}

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

fn sanitize_value(value: &str) -> String {
    value
        .replace('\r', "\\r")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}
