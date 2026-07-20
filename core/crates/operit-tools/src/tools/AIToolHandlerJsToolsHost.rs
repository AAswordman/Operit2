use std::collections::BTreeMap;

use operit_plugin_sdk::javascript::{JsExecutionHost, JsToolCallRequest, JsToolCallResultData};
use operit_plugin_sdk::js_sdk::chat::*;
use operit_plugin_sdk::js_sdk::files::*;
use operit_plugin_sdk::js_sdk::memory::*;
use operit_plugin_sdk::js_sdk::network::*;
use operit_plugin_sdk::js_sdk::results::*;
use operit_plugin_sdk::js_sdk::software_settings::SoftwareSettingsHost;
use operit_plugin_sdk::js_sdk::system::*;
use operit_plugin_sdk::js_sdk::tool_types::BuiltinToolName;
use operit_plugin_sdk::js_sdk::{JsFuture, JsHostError};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Map, Value};

use super::AIToolHandler::AIToolHandler;
use super::ToolResultDataClasses::ToolResultData;
use crate::ToolExecutionManager::{AITool, ToolParameter};

type GeneratedArgument = Result<(String, Value), JsHostError>;

/// Serializes one generated host method argument without erasing its Rust type contract.
fn generated_argument<T: Serialize>(name: &str, value: T) -> GeneratedArgument {
    serde_json::to_value(value)
        .map(|value| (name.to_string(), value))
        .map_err(|error| {
            JsHostError::new(format!(
                "Tools argument `{name}` cannot be serialized: {error}"
            ))
        })
}

/// Represents an uninhabited `Record<string, never>` argument as an empty object.
fn generated_empty_argument(name: &str) -> GeneratedArgument {
    Ok((name.to_string(), Value::Object(Map::new())))
}

/// Creates the explicit empty argument list used by generated zero-argument methods.
fn generated_no_arguments() -> Vec<GeneratedArgument> {
    Vec::new()
}

/// Converts one camel-case SDK field name into the executor's snake-case wire name.
fn snake_case_name(name: &str) -> String {
    let mut output = String::new();
    for character in name.chars() {
        if character.is_ascii_uppercase() {
            output.push('_');
            output.push(character.to_ascii_lowercase());
        } else {
            output.push(character);
        }
    }
    output
}

/// Reports whether one generated argument contributes fields directly to the tool parameter map.
fn flattens_argument(name: &str) -> bool {
    matches!(name, "options" | "params" | "updates")
        || name.ends_with("OrOptions")
        || name.ends_with("OrParams")
}

/// Resolves a scalar union argument's field name after removing its options suffix.
fn scalar_union_field(name: &str) -> String {
    for suffix in ["OrOptions", "OrParams"] {
        if let Some(name) = name.strip_suffix(suffix) {
            return snake_case_name(name);
        }
    }
    snake_case_name(name)
}

/// Resolves the legacy wire key for one positional Tools method argument.
fn positional_wire_name(namespace: &str, method: &str, name: &str) -> String {
    match (namespace, method, name) {
        ("Files", "writeBinary", "base64Content") => "base64Content".to_string(),
        ("Files", "apply" | "create" | "edit", "newContent") => "new".to_string(),
        ("Files", "edit", "oldContent") => "old".to_string(),
        ("System", "sleep", "milliseconds") => "duration_ms".to_string(),
        ("System", "listApps", "includeSystem") => "include_system_apps".to_string(),
        _ => snake_case_name(name.trim_start_matches("r#")),
    }
}

/// Resolves the legacy wire key for one flattened options field.
fn flattened_wire_name(namespace: &str, name: &str) -> String {
    if namespace == "Net" {
        name.to_string()
    } else {
        snake_case_name(name)
    }
}

/// Normalizes the one legacy memory field whose executor uses comma-separated text.
fn normalize_memory_titles(
    namespace: &str,
    parameters: &mut Map<String, Value>,
) -> Result<(), JsHostError> {
    if namespace != "Memory" {
        return Ok(());
    }
    let Some(Value::Array(titles)) = parameters.get("titles") else {
        return Ok(());
    };
    let titles = titles
        .iter()
        .map(|title| match title {
            Value::String(title) => Ok(title.as_str()),
            _ => Err(JsHostError::new("Memory titles must contain strings")),
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(",");
    parameters.insert("titles".to_string(), Value::String(titles));
    Ok(())
}

/// Adds fixed protocol fields that are part of a method binding rather than a Rust argument.
fn add_binding_fields(namespace: &str, method: &str, parameters: &mut Map<String, Value>) {
    match (namespace, method) {
        ("Net", "httpGet") => {
            parameters.insert("method".to_string(), Value::String("GET".to_string()));
        }
        ("Net", "httpPost") => {
            parameters.insert("method".to_string(), Value::String("POST".to_string()));
        }
        ("Net.cookies", "get") => {
            parameters.insert("action".to_string(), Value::String("get".to_string()));
        }
        ("Net.cookies", "set") => {
            parameters.insert("action".to_string(), Value::String("set".to_string()));
        }
        ("Net.cookies", "clear") => {
            parameters.insert("action".to_string(), Value::String("clear".to_string()));
        }
        _ => {}
    }
}

/// Builds the exact legacy executor parameter object from typed generated arguments.
fn build_generated_parameters(
    namespace: &str,
    method: &str,
    arguments: Vec<GeneratedArgument>,
) -> Result<BTreeMap<String, Value>, JsHostError> {
    let mut parameters = Map::new();
    for argument in arguments {
        let (name, value) = argument?;
        if flattens_argument(&name) {
            match value {
                Value::Null => {}
                Value::Object(fields) => {
                    for (field, value) in fields {
                        parameters.insert(flattened_wire_name(namespace, &field), value);
                    }
                }
                value => {
                    parameters.insert(scalar_union_field(&name), value);
                }
            }
        } else {
            parameters.insert(positional_wire_name(namespace, method, &name), value);
        }
    }
    add_binding_fields(namespace, method, &mut parameters);
    normalize_memory_titles(namespace, &mut parameters)?;
    Ok(parameters.into_iter().collect())
}

/// Executes one generated typed host method through its canonical built-in binding.
fn invoke_generated<TResult>(
    host: &AIToolHandler,
    name: BuiltinToolName,
    namespace: &str,
    method: &str,
    arguments: Vec<GeneratedArgument>,
) -> JsFuture<TResult>
where
    TResult: DeserializeOwned + Send + 'static,
{
    let parameters = build_generated_parameters(namespace, method, arguments);
    invoke_builtin(host, name, parameters)
}

/// Executes one typed built-in request through the registered executor chain.
fn invoke_builtin<TResult>(
    host: &AIToolHandler,
    name: BuiltinToolName,
    parameters: Result<BTreeMap<String, Value>, JsHostError>,
) -> JsFuture<TResult>
where
    TResult: DeserializeOwned + Send + 'static,
{
    let request = parameters.map(|parameters| JsToolCallRequest {
        tool_type: "default".to_string(),
        tool_name: name.as_str().to_string(),
        parameters,
    });
    let host = host.clone();
    Box::pin(async move {
        let response = JsExecutionHost::execute_tool_call(&host, request?);
        if !response.success {
            return Err(JsHostError::new(
                response
                    .error
                    .expect("failed built-in tool result must include an error"),
            ));
        }
        let value = match response.data {
            JsToolCallResultData::Value(value) => value,
            JsToolCallResultData::Binary(value) => {
                serde_json::to_value(value).map_err(|error| {
                    JsHostError::new(format!("Binary tool result cannot be serialized: {error}"))
                })?
            }
        };
        serde_json::from_value(value).map_err(|error| {
            JsHostError::new(format!(
                "Tool `{name}` returned an incompatible result: {error}"
            ))
        })
    })
}

/// Converts one JSON boundary value into the executor's stable text representation.
fn executor_parameter_value(value: Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value,
        value => value.to_string(),
    }
}

/// Executes terminal streaming and invokes the typed event callback for every stream event.
fn invoke_terminal_streaming(
    host: &AIToolHandler,
    name: BuiltinToolName,
    session_id: String,
    command: String,
    options: Option<SystemTerminalHostExecStreamingOptions>,
) -> JsFuture<TerminalCommandResultData> {
    let (timeout_ms, callback) = match options {
        Some(options) => (options.timeout_ms, options.on_intermediate_result),
        None => (None, None),
    };
    let mut parameters = vec![
        ToolParameter {
            name: "session_id".to_string(),
            value: session_id,
        },
        ToolParameter {
            name: "command".to_string(),
            value: command,
        },
    ];
    if let Some(timeout_ms) = timeout_ms {
        let value = serde_json::to_value(timeout_ms)
            .expect("terminal timeout boundary type must serialize");
        parameters.push(ToolParameter {
            name: "timeout_ms".to_string(),
            value: executor_parameter_value(value),
        });
    }
    let tool = AITool {
        name: name.as_str().to_string(),
        parameters,
    };
    let mut host = host.clone();
    Box::pin(async move {
        let results = host
            .executeToolSafelyWithResolvedExecutor(&tool)
            .await
            .ok_or_else(|| JsHostError::new("Terminal streaming tool is not registered"))?;
        let mut final_result = None;
        for result in results {
            if !result.success {
                return Err(JsHostError::new(
                    result
                        .error
                        .expect("failed terminal streaming result must include an error"),
                ));
            }
            match result.result {
                ToolResultData::TerminalStreamEventData(event) => {
                    if let Some(callback) = &callback {
                        callback(event);
                    }
                }
                ToolResultData::TerminalCommandResultData(result) => final_result = Some(result),
                _ => {
                    return Err(JsHostError::new(
                        "Terminal streaming executor returned an incompatible result",
                    ))
                }
            }
        }
        final_result.ok_or_else(|| {
            JsHostError::new("Terminal streaming executor did not return a final result")
        })
    })
}

/// Executes the current chat streaming binding without pretending to synthesize stream events.
fn invoke_chat_streaming(
    host: &AIToolHandler,
    name: BuiltinToolName,
    message: String,
    chat_id: Option<String>,
    role_card_id: Option<String>,
    sender_name: Option<String>,
    options: Option<ChatSendMessageStreamingOptions>,
) -> JsFuture<MessageSendResultData> {
    let (base_options, waifu, callback) = match options {
        Some(options) => (
            Some(options.base_send_message_options),
            options.waifu,
            options.onIntermediateResult,
        ),
        None => (None, None, None),
    };
    if callback.is_some() {
        return Box::pin(async {
            Err(JsHostError::new(
                "Chat streaming callbacks are not implemented by the current executor",
            ))
        });
    }
    invoke_generated(
        host,
        name,
        "Chat",
        "sendMessageStreaming",
        vec![
            generated_argument("message", message),
            generated_argument("chatId", chat_id),
            generated_argument("roleCardId", role_card_id),
            generated_argument("senderName", sender_name),
            generated_argument("options", base_options),
            generated_argument("waifu", waifu),
        ],
    )
}

include!(concat!(env!("OUT_DIR"), "/js_tools_host_impl.rs"));
