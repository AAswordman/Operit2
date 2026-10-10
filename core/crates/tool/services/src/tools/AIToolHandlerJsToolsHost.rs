use std::collections::BTreeMap;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_tools::files::PathMapper::PathMapper;

use operit_plugin_sdk::javascript::{JsExecutionHost, JsToolCallRequest, JsToolCallResultData};
use operit_plugin_sdk::js_sdk::chat::*;
use operit_plugin_sdk::js_sdk::core::JsonObject;
use operit_plugin_sdk::js_sdk::files::*;
use operit_plugin_sdk::js_sdk::edge::*;
use operit_plugin_sdk::js_sdk::network::*;
use operit_plugin_sdk::js_sdk::results::*;
use operit_plugin_sdk::js_sdk::software_settings::*;
use operit_plugin_sdk::js_sdk::system::*;
use operit_plugin_sdk::js_sdk::storage::*;
use operit_plugin_sdk::js_sdk::tool_types::BuiltinToolName;
use operit_plugin_sdk::js_sdk::{JsAny, JsAsyncIterable, JsFuture, JsHostError, rejected_js_async_iterable};
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
        let response = JsExecutionHost::execute_tool_call(&host, request?).await;
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
        host.notifyToolCallRequested(&tool);
        let interception = host.checkToolInterception(&tool).await.map_err(JsHostError::new)?;
        if let operit_tools::tools::AIToolHook::AIToolHookDecision::Block(_) = interception {
            let result = AIToolHandler::toolInterceptionResult(&tool, interception);
            host.notifyToolExecutionResult(&tool, &result);
            host.notifyToolExecutionFinished(&tool);
            let message = result
                .error
                .expect("intercepted terminal streaming result must include an error");
            return Err(JsHostError::new(message));
        }
        let results = host
            .executeToolDirectlyWithResolvedExecutor(&tool)
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

/// Requires the immutable engine-bound owner and its still-enabled actual registration.
fn authenticated_chat_extension_owner(host: &AIToolHandler) -> Result<String, JsHostError> {
    let owner = host.authenticatedExtensionOwner.as_deref().ok_or_else(|| {
        JsHostError::new("Chat extensions require a real ToolPkg execution owner")
    })?;
    if owner.trim().is_empty() {
        return Err(JsHostError::new("Chat extension owner is empty"));
    }
    let manager = host.getOrCreatePackageManager();
    let registry = manager.lock().expect("package manager mutex poisoned");
    registry
        .packageRegistryReadiness()
        .require_ready()
        .map_err(JsHostError::new)?;
    registry.getToolPkgContainerRuntime(owner).ok_or_else(|| {
        JsHostError::new(format!("Chat extension owner is not registered: {owner}"))
    })?;
    if !registry.getEnabledToolPkgContainerRuntimes().iter()
        .any(|runtime| runtime.packageName == owner)
    {
        return Err(JsHostError::new(format!(
            "Chat extension owner is disabled: {owner}"
        )));
    }
    Ok(owner.to_string())
}

/// Reads the exact caller namespace directly through the typed record support boundary.
fn invoke_chat_extension_read(
    host: &AIToolHandler,
    target: ChatExtensionTarget,
) -> JsFuture<JsNullable<JsonObject>> {
    let owner = authenticated_chat_extension_owner(host);
    let support = host.runtimeSupport();
    Box::pin(async move {
        let owner = owner?;
        target.validate().map_err(JsHostError::new)?;
        support
            .readChatExtension(&owner, &target)
            .map(JsNullable::from_option)
            .map_err(JsHostError::new)
    })
}

/// Replaces only the exact caller's namespace without invoking an AI tool or a chat runtime lock.
fn invoke_chat_extension_write(
    host: &AIToolHandler,
    target: ChatExtensionTarget,
    value: JsonObject,
) -> JsFuture<JsonObject> {
    let owner = authenticated_chat_extension_owner(host);
    let support = host.runtimeSupport();
    Box::pin(async move {
        let owner = owner?;
        target.validate().map_err(JsHostError::new)?;
        support
            .writeChatExtension(&owner, &target, value)
            .map_err(JsHostError::new)
    })
}

/// Deletes only the exact caller's namespace and propagates record or execution-guard failures.
fn invoke_chat_extension_delete(
    host: &AIToolHandler,
    target: ChatExtensionTarget,
) -> JsFuture<bool> {
    let owner = authenticated_chat_extension_owner(host);
    let support = host.runtimeSupport();
    Box::pin(async move {
        let owner = owner?;
        target.validate().map_err(JsHostError::new)?;
        support
            .deleteChatExtension(&owner, &target)
            .map_err(JsHostError::new)
    })
}

/// Opens the shared real send pipeline using only immutable engine-authenticated plugin ownership.
fn open_chat_send(host: &AIToolHandler, request: ChatSendRequest, observeParts: bool) -> JsAsyncIterable<ChatSendEvent> {
    let opening = authenticated_chat_extension_owner(host).and_then(|owner| {
        host.runtimeSupport().openPluginChatMessage(&owner, request, observeParts).map_err(JsHostError::new)
    });
    match opening { Ok(stream) => stream, Err(error) => rejected_js_async_iterable(error) }
}

/// Opens semantic observation on the sole real native send pipeline.
fn invoke_chat_stream(host: &AIToolHandler, request: ChatSendRequest) -> JsAsyncIterable<ChatSendEvent> {
    open_chat_send(host, request, true)
}

/// Drains the same pipeline and returns only its actual finalized terminal receipt.
fn invoke_chat_send(host: &AIToolHandler, request: ChatSendRequest) -> JsFuture<MessageSendResultData> {
    let stream = open_chat_send(host, request, false);
    Box::pin(async move {
        let result = loop {
            match stream.next().await {
                Ok(Some(ChatSendEvent::Part { .. })) => {},
                Ok(Some(ChatSendEvent::Completed { result })) => break Ok(result),
                Ok(None) => break Err(JsHostError::new("Chat send ended without its terminal receipt")),
                Err(error) => break Err(error),
            }
        };
        let closed = stream.close().await;
        match (result, closed) {
            (Ok(result), Ok(())) => Ok(result),
            (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
            (Err(error), Err(close)) => Err(JsHostError::new(format!("{error}; chat observation disposal failed: {close}"))),
        }
    })
}

/// Requests cancellation after capturing only this enabled authenticated owner's active execution.
fn invoke_chat_cancel(host: &AIToolHandler, chatId: String) -> JsFuture<ChatCancelResult> {
    let result = authenticated_chat_extension_owner(host).and_then(|owner| {
        host.runtimeSupport().requestPluginChatCancellation(&owner, &chatId).map_err(JsHostError::new)
    });
    Box::pin(async move { result })
}

include!(concat!(env!("OUT_DIR"), "/js_tools_host_impl.rs"));

/// Converts supported public VFS, runtime-relative and Host paths to one canonical runtime storage identity.
fn resolve_plugin_storage_path(path: &str, roots: &RuntimeStorePaths) -> Result<String, JsHostError> {
    let normalized = path.replace('\\', "/");
    if normalized.starts_with("runtime/") {
        return Ok(normalized);
    }
    if normalized == "/app/data" || normalized.starts_with("/app/data/") {
        let relative = PathMapper::relativePath("/app/data", &normalized)
            .map_err(JsHostError::new)?
            .ok_or_else(|| JsHostError::new("Storage VFS path is not inside the runtime root"))?;
        return Ok(format!("runtime/{relative}"));
    }
    let root = roots.runtime_dir().to_string_lossy().replace('\\', "/");
    let relative = normalized.strip_prefix(&format!("{root}/"))
        .ok_or_else(|| JsHostError::new("Storage path is not inside the runtime Host root"))?;
    Ok(format!("runtime/{relative}"))
}

/// Captures real enabled ownership, normalizes the selected Host path, and uses the engine's own handles.
fn invoke_storage_request(host: &AIToolHandler, mut request: StorageRequest) -> JsFuture<Value> {
    let owner = authenticated_chat_extension_owner(host);
    let context = host.getContext();
    let session = host.storageSession.clone();
    Box::pin(async move {
        owner?;
        let sqlite = context.runtimeSqliteHost.ok_or_else(|| JsHostError::new("RuntimeSqliteHost is not registered"))?;
        let storage = context.runtimeStorageHost.ok_or_else(|| JsHostError::new("RuntimeStorageHost is not registered"))?;
        if let StorageRequest::Open { path, .. } = &mut request {
            *path = resolve_plugin_storage_path(path, &RuntimeStorePaths::default())?;
        }
        let deviceId = operit_store::SyncOperationStore::SyncOperationStore::new(storage.clone(), operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH).localDeviceId().map_err(|e| JsHostError::new(e.to_string()))?;
        session.request(sqlite, storage, request, &deviceId).map_err(|e| JsHostError::new(e.to_string()))
    })
}

/// Executes a narrow typed configuration-directory read without registering or invoking an AI tool.
fn invoke_software_settings_directory<TResult>(
    host: &AIToolHandler,
    method: &str,
) -> JsFuture<TResult>
where
    TResult: DeserializeOwned + Send + 'static,
{
    let support = host.runtimeSupport();
    let method = method.to_owned();
    Box::pin(async move {
        let value = match method.as_str() {
            "listModelSummaries" => {
                serde_json::to_value(support.listModelSummaries().map_err(JsHostError::new)?)
            }
            "listTtsConfigs" => {
                serde_json::to_value(support.listTtsConfigs().map_err(JsHostError::new)?)
            }
            "listThemeConfigs" => serde_json::to_value(support.listThemeConfigs().map_err(JsHostError::new)?),
            "getCurrentTtsConfigId" => serde_json::to_value(support.getCurrentTtsConfigId().map_err(JsHostError::new)?),
            "readToolSourceCatalog" => serde_json::to_value(
                support
                    .readToolSourceCatalog()
                    .await
                    .map_err(JsHostError::new)?,
            ),
            _ => {
                return Err(JsHostError::new(format!(
                    "Unknown SoftwareSettings directory method: {method}"
                )))
            }
        }
        .map_err(|error| JsHostError::new(error.to_string()))?;
        serde_json::from_value(value).map_err(|error| JsHostError::new(error.to_string()))
    })
}

/// Applies one narrowly declared ordinary configuration reference through its canonical manager.
fn invoke_software_settings_config<TResult>(host: &AIToolHandler, method: &str, id: String) -> JsFuture<TResult>
where
    TResult: DeserializeOwned + Send + 'static,
{
    let support = host.runtimeSupport();
    let method = method.to_owned();
    Box::pin(async move {
        if id.trim().is_empty() || id.trim() != id {
            return Err(JsHostError::new("Configuration ID must be exact nonblank text"));
        }
        let value = match method.as_str() {
            "applyThemeConfig" => serde_json::to_value(support.applyThemeConfig(id).map_err(JsHostError::new)?),
            "setCurrentTtsConfigId" => serde_json::to_value(support.setCurrentTtsConfigId(id).map_err(JsHostError::new)?),
            _ => return Err(JsHostError::new(format!("Unknown SoftwareSettings configuration method: {method}"))),
        }.map_err(|error| JsHostError::new(error.to_string()))?;
        serde_json::from_value(value).map_err(|error| JsHostError::new(error.to_string()))
    })
}

#[cfg(test)]
mod plugin_storage_paths {
    use super::*;

    /// Resolves public local and shared directories to the same identity as runtime and Host paths.
    #[test]
    fn directory_paths_resolve_to_canonical_owned_storage() {
        let roots = RuntimeStorePaths::new(std::path::PathBuf::from("storage-root/runtime"), std::path::PathBuf::from("storage-root/workspaces"));
        for scope_path in ["device/example/cache.sqlite", "space/example/memory.sqlite"] {
            let relative = format!("plugin_data/{scope_path}");
            let expected = format!("runtime/{relative}");
            for path in [format!("/app/data/{relative}"), expected.clone(), format!("storage-root/runtime/{relative}")] {
                assert_eq!(resolve_plugin_storage_path(&path, &roots).unwrap(), expected);
            }
        }
        for path in ["/app/workspaces/example/cache.sqlite", "/app/data/../../escape.sqlite", "storage-root/other/cache.sqlite", "/app/database/cache.sqlite"] {
            assert!(resolve_plugin_storage_path(path, &roots).is_err(), "{path}");
        }
    }
}
