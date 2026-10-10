use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;

use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_TOOL_LIFECYCLE;
use operit_plugin_sdk::toolpkg::ToolPkgHooks::ToolPkgToolLifecycleHookRegistration;
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;
use operit_tools::tools::climode::CliToolModeSupport::HiddenToolCatalogEntry;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_tools::tools::AIToolHook::{AIToolHook, AIToolHookDecision};
use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{AITool, ToolExecutionManager, ToolRuntimeContext};
use operit_util::ChainLogger::{self, PLUGIN_CHAIN};

static TOOL_LIFECYCLE_HOOKS: OnceLock<Mutex<Vec<ToolPkgToolLifecycleHookRegistration>>> =
    OnceLock::new();

pub struct ToolPkgToolLifecycleBridge;

impl ToolPkgToolLifecycleBridge {
    /// Requires the exact registered, enabled package identity before exposing any execution snapshot.
    fn requireExecutionOwner(manager: &RuntimePackageManager, owner: &str) -> Result<(), String> {
        if owner.trim().is_empty() || owner.trim() != owner {
            return Err("Tool execution owner must be exact nonblank text".to_string());
        }
        let runtime = manager.getToolPkgContainerRuntime(owner)
            .ok_or_else(|| format!("Unknown tool execution owner: {owner}"))?;
        if runtime.packageName != owner {
            return Err(format!("Tool execution owner is not its canonical registered identity: {owner}"));
        }
        let count = manager.getEnabledToolPkgContainerRuntimes().into_iter()
            .filter(|runtime| runtime.packageName == owner).count();
        match count {
            1 => Ok(()),
            0 => Err(format!("Tool execution owner is disabled: {owner}")),
            _ => Err(format!("Duplicate enabled tool execution owner: {owner}")),
        }
    }

    /// Validates the native configuration snapshot even when this registry has no tool-policy hooks.
    pub fn validateExecutionSnapshot(
        manager: &RuntimePackageManager,
        context: Option<&ToolRuntimeContext>,
    ) -> Result<(), String> {
        if let Some(context) = context {
            context.validateSnapshot()?;
            Self::requireExecutionOwner(manager, &context.extensionOwner)?;
        }
        Ok(())
    }

    /// Projects only the receiving registered owner's namespace; non-participation is explicit null.
    pub fn executionContextForOwner(
        manager: &RuntimePackageManager,
        context: Option<&ToolRuntimeContext>,
        registeredOwner: &str,
    ) -> Result<Value, String> {
        Self::requireExecutionOwner(manager, registeredOwner)?;
        Self::validateExecutionSnapshot(manager, context)?;
        match context {
            Some(context) => {
                let messageExtension = if context.extensionOwner == registeredOwner {
                    Value::Object(context.messageExtension.clone())
                } else {
                    Value::Null
                };
                Ok(serde_json::json!({
                    "chatId": context.callerChatId,
                    "participantId": context.callerParticipantId,
                    "extensionOwner": registeredOwner,
                    "messageExtension": messageExtension,
                }))
            }
            None => Ok(Value::Null),
        }
    }

    /// Registers tool lifecycle hooks for one application runtime.
    pub fn register(runtime: ToolPkgBridgeRuntime) {
        let mut handler = runtime.tool_handler();
        handler.addToolHook(Arc::new(ToolLifecycleBridge { runtime }));
    }

    /// Synchronizes lifecycle notification registrations with the actual active containers.
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

struct ToolLifecycleBridge {
    runtime: ToolPkgBridgeRuntime,
}

impl AIToolHook for ToolLifecycleBridge {
    fn id(&self) -> &str {
        "builtin.toolpkg.tool-lifecycle-bridge"
    }

    fn onToolCallRequested(&self, tool: &AITool) {
        deliver(
            &self.runtime,
            "tool_call_requested",
            build_base_payload(tool),
        );
    }

    /// Awaits enabled owner-isolated policies and propagates invoked failures without converting them to denials.
    fn onToolCallInterceptAsync<'a>(
        &'a self,
        tool: &'a AITool,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<AIToolHookDecision, String>> + Send + 'a>> {
        let executionContext = ToolExecutionManager::currentToolRuntimeContext();
        Box::pin(async move {
            let manager = RuntimePackageManager::readySnapshot(
                self.runtime.tool_handler().getOrCreatePackageManager(),
            ).await?;
            ToolPkgToolLifecycleBridge::validateExecutionSnapshot(&manager, executionContext.as_ref())?;
            let budget = super::ToolPkgPreHookTimeout::ToolPkgPreHookTimeout::fromPreferences();
            for container in manager.getEnabledToolPkgContainerRuntimes() {
                let ownerContext = ToolPkgToolLifecycleBridge::executionContextForOwner(
                    &manager, executionContext.as_ref(), &container.packageName,
                )?;
                for hook in container.toolLifecycleHooks {
                    let mut payload = build_base_payload(tool);
                    payload["runtimeContext"] = ownerContext.clone();
                    let timeoutMillis = budget.remainingTimeoutMillis().ok_or_else(|| format!(
                        "Tool interception timed out before handler {}:{}", container.packageName, hook.id,
                    ))?;
                    let raw = manager.runToolPkgMainHookWithTimeoutMillis(
                        &container.packageName, &hook.function, TOOLPKG_EVENT_TOOL_LIFECYCLE,
                        Some("tool_call_intercept"), Some(&hook.id), hook.functionSource.as_deref(),
                        payload, None, None, None, timeoutMillis,
                    ).await?;
                    if budget.hasExpired() {
                        return Err(format!("Tool interception timed out after handler {}:{}", container.packageName, hook.id));
                    }
                    let decision = decode_intercept_result(raw)?;
                    if let AIToolHookDecision::Block(_) = decision {
                        return Ok(decision);
                    }
                }
            }
            Ok(AIToolHookDecision::Allow)
        })
    }

    fn onToolPermissionChecked(&self, tool: &AITool, granted: bool, reason: Option<&str>) {
        let mut payload = build_base_payload(tool);
        payload["granted"] = Value::Bool(granted);
        payload["reason"] = reason
            .map(|value| Value::String(value.to_string()))
            .unwrap_or(Value::Null);
        deliver(&self.runtime, "tool_permission_checked", payload);
    }

    fn onToolExecutionStarted(&self, tool: &AITool) {
        deliver(
            &self.runtime,
            "tool_execution_started",
            build_base_payload(tool),
        );
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
        deliver(&self.runtime, "tool_execution_result", payload);
    }

    fn onToolExecutionError(&self, tool: &AITool, message: &str) {
        let mut payload = build_base_payload(tool);
        payload["success"] = Value::Bool(false);
        payload["errorMessage"] = Value::String(message.to_string());
        deliver(&self.runtime, "tool_execution_error", payload);
    }

    fn onToolExecutionFinished(&self, tool: &AITool) {
        deliver(
            &self.runtime,
            "tool_execution_finished",
            build_base_payload(tool),
        );
    }
}

/// Serializes actual tool arguments and their host-owned chat execution context.
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

/// Delivers lifecycle notifications to their actual registered main-runtime handlers.
fn deliver(runtime: &ToolPkgBridgeRuntime, eventName: &str, eventPayload: Value) {
    let runtime = runtime.to_owned();
    let eventName = eventName.to_owned();
    // Capture before scheduling: notification tasks do not inherit another turn's task-local state.
    let executionContext = ToolExecutionManager::currentToolRuntimeContext();
    super::ToolPkgHookBridgeSupport::scheduleToolPkgNotification(
        "operit-toolpkg-notification",
        move || {
            Box::pin(async move {
                let runtime = &runtime;
                let eventName = &eventName;
                let manager = match RuntimePackageManager::readySnapshot(
                    runtime.tool_handler().getOrCreatePackageManager(),
                ).await {
                    Ok(manager) => manager,
                    Err(error) => {
                        ChainLogger::error(PLUGIN_CHAIN, "plugin.toolpkg.tool_lifecycle.snapshot.error", &[("error", error)]);
                        return;
                    }
                };
                let hooks = manager.getEnabledToolPkgContainerRuntimes().into_iter()
                    .flat_map(|container| container.toolLifecycleHooks.into_iter().map(move |hook| ToolPkgToolLifecycleHookRegistration {
                        containerPackageName: container.packageName.clone(),
                        hookId: hook.id,
                        functionName: hook.function,
                        functionSource: hook.functionSource,
                    }));
                for hook in hooks {
                    let ownerContext = match ToolPkgToolLifecycleBridge::executionContextForOwner(
                        &manager, executionContext.as_ref(), &hook.containerPackageName,
                    ) {
                        Ok(context) => context,
                        Err(error) => {
                            ChainLogger::error(PLUGIN_CHAIN, "plugin.toolpkg.tool_lifecycle.context.error", &[("error", error)]);
                            return;
                        }
                    };
                    let mut payload = eventPayload.clone();
                    payload["runtimeContext"] = ownerContext;
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
                    match manager
                        .runToolPkgMainHook(
                            &hook.containerPackageName,
                            &hook.functionName,
                            TOOLPKG_EVENT_TOOL_LIFECYCLE,
                            Some(eventName),
                            Some(&hook.hookId),
                            hook.functionSource.as_deref(),
                            payload,
                            None,
                            None,
                            None,
                        )
                        .await
                    {
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
            })
        },
    );
}

/// Decodes an explicit SDK interception result and rejects malformed output rather than permitting it.
fn decode_intercept_result(raw: Option<String>) -> Result<AIToolHookDecision, String> {
    let Some(raw) = raw else {
        return Ok(AIToolHookDecision::Allow);
    };
    // The native JS completion ABI encodes a void return as an empty string.
    if raw.is_empty() {
        return Ok(AIToolHookDecision::Allow);
    }
    let value: Value = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    if value.is_null() {
        return Ok(AIToolHookDecision::Allow);
    }
    let object = value.as_object().ok_or_else(|| {
        "Tool lifecycle interception result must be an object or void".to_string()
    })?;
    match object.get("action").and_then(Value::as_str) {
        Some("allow") => Ok(AIToolHookDecision::Allow),
        Some("block") => {
            let reason = object
                .get("reason")
                .and_then(Value::as_str)
                .filter(|reason| !reason.trim().is_empty())
                .ok_or_else(|| {
                    "Blocked tool lifecycle result requires a nonblank reason".to_string()
                })?;
            Ok(AIToolHookDecision::Block(reason.to_string()))
        }
        _ => Err("Tool lifecycle interception action must be exactly allow or block".to_string()),
    }
}

impl ToolPkgToolLifecycleBridge {
    /// Filters the immutable hidden catalog using the same actual registered tool-prompt compose handlers.
    /// Filters an immutable catalog through the actual enabled tool-prompt registrations.
    #[allow(non_snake_case)]
    pub async fn filterToolCatalog(
        manager: &RuntimePackageManager,
        entries: Vec<HiddenToolCatalogEntry>,
        context: &ToolRuntimeContext,
    ) -> Result<Vec<HiddenToolCatalogEntry>, String> {
        use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_TOOL_PROMPT_COMPOSE;
        let mut available = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            let activation = match &entry.suggested_params_json {
                Some(raw) => {
                    let value: Value =
                        serde_json::from_str(raw).map_err(|error| error.to_string())?;
                    match value.get("package_name") {
                        Some(Value::String(name)) => Some(name.clone()),
                        None => None,
                        _ => {
                            return Err(
                                "Catalog activation package_name must be a string".to_string()
                            )
                        }
                    }
                }
                None => None,
            };
            available.push(serde_json::json!({ "catalogEntryId": index.to_string(), "name": entry.target_tool_name, "categoryName": "Hidden tool catalog", "description": entry.description, "parameters": entry.parameter_hints.join("\n"), "activationSource": activation }));
        }
        Self::validateExecutionSnapshot(manager, Some(context))?;
        let originalDescriptors = available.clone();
        let budget = super::ToolPkgPreHookTimeout::ToolPkgPreHookTimeout::fromPreferences();
        for container in manager.getEnabledToolPkgContainerRuntimes() {
            let ownerContext = Self::executionContextForOwner(manager, Some(context), &container.packageName)?;
            for hook in container.toolPromptComposeHooks {
                let payload = serde_json::json!({ "stage": "filter_tool_call_tools", "chatId": context.callerChatId, "availableTools": available, "metadata": { "executionContext": ownerContext } });
                let timeoutMillis = budget.remainingTimeoutMillis().ok_or_else(|| format!(
                    "Tool catalog policy timed out before handler {}:{}", container.packageName, hook.id,
                ))?;
                let raw = manager.runToolPkgMainHookWithTimeoutMillis(
                    &container.packageName, &hook.function, TOOLPKG_EVENT_TOOL_PROMPT_COMPOSE,
                    Some("filter_tool_call_tools"), Some(&hook.id), hook.functionSource.as_deref(),
                    payload, None, None, None, timeoutMillis,
                ).await?;
                if budget.hasExpired() {
                    return Err(format!("Tool catalog policy timed out after handler {}:{}", container.packageName, hook.id));
                }
                if let Some(mutation) = super::ToolPkgPromptHookBridge::decode_tool_prompt_policy_result(raw)? {
                    if let Some(items) = mutation.available_tools {
                        let descriptors = items.into_iter().map(serde_json::to_value)
                            .collect::<Result<Vec<Value>, _>>().map_err(|error| error.to_string())?;
                        validate_catalog_descriptors(&descriptors, &available)?;
                        available = descriptors;
                    }
                }
            }
        }
        validate_catalog_descriptors(&available, &originalDescriptors)?;
        let mut seen = std::collections::HashSet::new();
        let mut selected = Vec::new();
        for entry in available {
            let id = entry
                .get("catalogEntryId")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    "Filtered tool catalog entry requires its original catalogEntryId".to_string()
                })?;
            let index = id.parse::<usize>().map_err(|error| error.to_string())?;
            if index.to_string() != id || !seen.insert(index) {
                return Err(format!("Duplicate or invalid catalogEntryId: {id}"));
            }
            let original = entries
                .get(index)
                .ok_or_else(|| format!("Unknown catalogEntryId: {id}"))?;
            selected.push(original.clone());
        }
        Ok(selected)
    }
}

/// Accepts only a unique subset of the exact descriptors passed to this policy invocation.
fn validate_catalog_descriptors(selected: &[Value], supplied: &[Value]) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    for descriptor in selected {
        let id = descriptor.get("catalogEntryId").and_then(Value::as_str)
            .ok_or_else(|| "Filtered tool catalog entry requires its original catalogEntryId".to_string())?;
        if !seen.insert(id) {
            return Err(format!("Duplicate catalogEntryId: {id}"));
        }
        let original = supplied.iter().find(|entry| entry.get("catalogEntryId").and_then(Value::as_str) == Some(id))
            .ok_or_else(|| format!("Unknown or removed catalogEntryId: {id}"))?;
        if descriptor != original {
            return Err(format!("Filtered catalog entry changed its registered descriptor: {id}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod interception_tests {
    use super::*;
    /// Rejects tool descriptor mutation, duplication, and reintroducing entries removed by an earlier policy.
    #[test]
    fn catalog_policies_only_select_exact_supplied_descriptors() {
        let first = serde_json::json!({"catalogEntryId":"0", "name":"read_file", "description":" exact description "});
        let second = serde_json::json!({"catalogEntryId":"1", "name":"delete_file", "description":" exact removal "});
        let supplied = vec![first.clone(), second.clone()];
        assert!(validate_catalog_descriptors(&[first.clone()], &supplied).is_ok());
        assert!(validate_catalog_descriptors(&[], &supplied).is_ok());
        assert!(validate_catalog_descriptors(&[first.clone(), first.clone()], &supplied).is_err());
        let mut changed = first.clone();
        changed["description"] = Value::String("mutated".to_string());
        assert!(validate_catalog_descriptors(&[changed], &supplied).is_err());
        assert!(validate_catalog_descriptors(&[second], &[first]).is_err());
    }

    /// Accepts the actual native void encoding without treating malformed decisions as success.
    #[test]
    fn permits_native_void_interception_results() {
        for raw in [None, Some(String::new()), Some("null".to_string()), Some(r#"{"action":"allow"}"#.to_string())] {
            assert!(matches!(decode_intercept_result(raw), Ok(AIToolHookDecision::Allow)));
        }
        assert!(decode_intercept_result(Some(" ".to_string())).is_err());
    }

    /// Verifies malformed or incomplete decisions do not silently allow execution.
    #[test]
    fn rejects_malformed_interception_decisions() {
        for raw in [
            "not-json",
            "17",
            "{}",
            r#"{"action":"BLOCK","reason":"denied"}"#,
            r#"{"action":"block"}"#,
        ] {
            assert!(decode_intercept_result(Some(raw.to_string())).is_err());
        }
    }
    /// Preserves the exact plugin-provided denial message.
    #[test]
    fn preserves_interception_reason() {
        let decision = decode_intercept_result(Some(
            r#"{"action":"block","reason":" exact failure "}"#.to_string(),
        ))
        .unwrap();
        assert!(
            matches!(decision, AIToolHookDecision::Block(reason) if reason == " exact failure ")
        );
    }
}
