use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::AITool;
use crate::core::tools::packTool::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_TOOL_LIFECYCLE;
use crate::core::tools::packTool::ToolPkgParser::ToolPkgContainerRuntime;
use crate::core::tools::AIToolHook::AIToolHook;
use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::{
    toolPkgPackageManager, toolPkgToolHandler, ToolPkgToolLifecycleHookRegistration,
};
use crate::util::ChainLogger::{self, PLUGIN_CHAIN};

static TOOL_LIFECYCLE_HOOKS: OnceLock<Mutex<Vec<ToolPkgToolLifecycleHookRegistration>>> =
    OnceLock::new();

pub struct ToolPkgToolLifecycleBridge;

impl ToolPkgToolLifecycleBridge {
    pub fn register() {
        let mut handler = toolPkgToolHandler();
        handler.addToolHook(Arc::new(ToolLifecycleBridge));
    }

    #[allow(non_snake_case)]
    pub fn syncToolPkgRegistrations(activeContainers: Vec<ToolPkgContainerRuntime>) {
        let hooks = activeContainers
            .iter()
            .flat_map(|container| {
                container.toolLifecycleHooks.iter().map(|hook| {
                    ToolPkgToolLifecycleHookRegistration {
                        containerPackageName: container.packageName.clone(),
                        hookId: hook.id.clone(),
                        functionName: hook.function.clone(),
                        functionSource: hook.functionSource.clone(),
                    }
                })
            })
            .collect::<Vec<_>>();
        *TOOL_LIFECYCLE_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg tool lifecycle hook mutex poisoned") = hooks;
    }
}

struct ToolLifecycleBridge;

impl AIToolHook for ToolLifecycleBridge {
    fn id(&self) -> &str {
        "builtin.toolpkg.tool-lifecycle-bridge"
    }

    fn onToolCallRequested(&self, tool: &AITool) {
        deliver("tool_call_requested", build_base_payload(tool));
    }

    fn onToolPermissionChecked(&self, tool: &AITool, granted: bool, reason: Option<&str>) {
        let mut payload = build_base_payload(tool);
        payload["granted"] = Value::Bool(granted);
        payload["reason"] = reason
            .map(|value| Value::String(value.to_string()))
            .unwrap_or(Value::Null);
        deliver("tool_permission_checked", payload);
    }

    fn onToolExecutionStarted(&self, tool: &AITool) {
        deliver("tool_execution_started", build_base_payload(tool));
    }

    fn onToolExecutionResult(&self, tool: &AITool, result: &ToolResult) {
        let mut payload = build_base_payload(tool);
        payload["success"] = Value::Bool(result.success);
        payload["errorMessage"] = result
            .error
            .as_ref()
            .map(|value| Value::String(value.clone()))
            .unwrap_or(Value::Null);
        payload["resultText"] = Value::String(result.result.toString());
        payload["resultJson"] =
            serde_json::from_str::<Value>(&result.result.toJson()).unwrap_or(Value::Null);
        deliver("tool_execution_result", payload);
    }

    fn onToolExecutionError(&self, tool: &AITool, message: &str) {
        let mut payload = build_base_payload(tool);
        payload["success"] = Value::Bool(false);
        payload["errorMessage"] = Value::String(message.to_string());
        deliver("tool_execution_error", payload);
    }

    fn onToolExecutionFinished(&self, tool: &AITool) {
        deliver("tool_execution_finished", build_base_payload(tool));
    }
}

fn build_base_payload(tool: &AITool) -> Value {
    let parameters = tool
        .parameters
        .iter()
        .map(|parameter| {
            (
                parameter.name.clone(),
                Value::String(parameter.value.clone()),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    serde_json::json!({
        "toolName": tool.name,
        "parameters": parameters,
        "description": null
    })
}

fn deliver(eventName: &str, eventPayload: Value) {
    let snapshot = TOOL_LIFECYCLE_HOOKS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .expect("toolpkg tool lifecycle hook mutex poisoned")
        .clone();
    ChainLogger::info(
        PLUGIN_CHAIN,
        "plugin.toolpkg.tool_lifecycle.scan",
        &[
            ("event", eventName.to_string()),
            ("hookCount", snapshot.len().to_string()),
        ],
    );
    let manager = toolPkgPackageManager();
    for hook in snapshot {
        ChainLogger::info(
            PLUGIN_CHAIN,
            "plugin.toolpkg.tool_lifecycle.run.start",
            &[
                ("event", eventName.to_string()),
                ("package", hook.containerPackageName.clone()),
                ("hookId", hook.hookId.clone()),
                ("function", hook.functionName.clone()),
            ],
        );
        match manager.runToolPkgMainHook(
            &hook.containerPackageName,
            &hook.functionName,
            TOOLPKG_EVENT_TOOL_LIFECYCLE,
            Some(eventName),
            Some(&hook.hookId),
            hook.functionSource.as_deref(),
            eventPayload.clone(),
            None,
            None,
            None,
        ) {
            Ok(_) => ChainLogger::info(
                PLUGIN_CHAIN,
                "plugin.toolpkg.tool_lifecycle.run.done",
                &[
                    ("event", eventName.to_string()),
                    ("package", hook.containerPackageName.clone()),
                    ("hookId", hook.hookId.clone()),
                ],
            ),
            Err(error) => ChainLogger::error(
                PLUGIN_CHAIN,
                "plugin.toolpkg.tool_lifecycle.run.error",
                &[
                    ("event", eventName.to_string()),
                    ("package", hook.containerPackageName.clone()),
                    ("hookId", hook.hookId.clone()),
                    ("function", hook.functionName.clone()),
                    ("error", error),
                ],
            ),
        }
    }
}
