use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;

use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use crate::plugins::toolpkg::ToolPkgPreHookTimeout::ToolPkgPreHookTimeout;
use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::{
    TOOLPKG_EVENT_PROMPT_ESTIMATE_FINALIZE, TOOLPKG_EVENT_PROMPT_ESTIMATE_HISTORY,
    TOOLPKG_EVENT_PROMPT_FINALIZE, TOOLPKG_EVENT_PROMPT_HISTORY, TOOLPKG_EVENT_PROMPT_INPUT,
    TOOLPKG_EVENT_SYSTEM_PROMPT_COMPOSE, TOOLPKG_EVENT_TOOL_PROMPT_COMPOSE,
};
use operit_plugin_sdk::toolpkg::ToolPkgHooks::{
    decodeToolPkgHookResult, ToolPkgPromptHookRegistration,
};
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;
use operit_providers::chat::hooks::PromptHookRegistry::{
    PromptEstimateFinalizeHook, PromptEstimateHistoryHook, PromptFinalizeHook, PromptHistoryHook,
    PromptHookContext, PromptHookMutation, PromptHookRegistry, PromptInputHook,
    SystemPromptComposeHook, ToolPromptComposeHook, ToolPromptHookFuture,
};
use operit_tools::ToolExecutionManager::ToolExecutionManager;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use super::ToolPkgToolLifecycleBridge::ToolPkgToolLifecycleBridge;
use operit_util::AppLogger::AppLogger;
use operit_util::ChainLogger::{self, PLUGIN_CHAIN};

const TAG: &str = "ToolPkgPromptHookBridge";

static PROMPT_INPUT_HOOKS: OnceLock<Mutex<Vec<ToolPkgPromptHookRegistration>>> = OnceLock::new();
static PROMPT_HISTORY_HOOKS: OnceLock<Mutex<Vec<ToolPkgPromptHookRegistration>>> = OnceLock::new();
static PROMPT_ESTIMATE_HISTORY_HOOKS: OnceLock<Mutex<Vec<ToolPkgPromptHookRegistration>>> =
    OnceLock::new();
static SYSTEM_PROMPT_COMPOSE_HOOKS: OnceLock<Mutex<Vec<ToolPkgPromptHookRegistration>>> =
    OnceLock::new();
static TOOL_PROMPT_COMPOSE_HOOKS: OnceLock<Mutex<Vec<ToolPkgPromptHookRegistration>>> =
    OnceLock::new();
static PROMPT_FINALIZE_HOOKS: OnceLock<Mutex<Vec<ToolPkgPromptHookRegistration>>> = OnceLock::new();
static PROMPT_ESTIMATE_FINALIZE_HOOKS: OnceLock<Mutex<Vec<ToolPkgPromptHookRegistration>>> =
    OnceLock::new();

pub struct ToolPkgPromptHookBridge;

impl ToolPkgPromptHookBridge {
    /// Registers prompt hooks for one application runtime.
    pub fn register(runtime: ToolPkgBridgeRuntime) {
        PromptHookRegistry::registerPromptInputHook(Arc::new(PromptInputBridge {
            runtime: runtime.clone(),
        }));
        PromptHookRegistry::registerPromptHistoryHook(Arc::new(PromptHistoryBridge {
            runtime: runtime.clone(),
        }));
        PromptHookRegistry::registerPromptEstimateHistoryHook(Arc::new(
            PromptEstimateHistoryBridge {
                runtime: runtime.clone(),
            },
        ));
        PromptHookRegistry::registerSystemPromptComposeHook(Arc::new(SystemPromptComposeBridge {
            runtime: runtime.clone(),
        }));
        PromptHookRegistry::registerToolPromptComposeHook(Arc::new(ToolPromptComposeBridge {
            runtime: runtime.clone(),
        }));
        PromptHookRegistry::registerPromptFinalizeHook(Arc::new(PromptFinalizeBridge {
            runtime: runtime.clone(),
        }));
        PromptHookRegistry::registerPromptEstimateFinalizeHook(Arc::new(
            PromptEstimateFinalizeBridge { runtime },
        ));
    }

    #[allow(non_snake_case)]
    pub fn syncToolPkgRegistrations(activeContainers: Vec<ToolPkgContainerRuntime>) {
        replace_hooks(
            PROMPT_INPUT_HOOKS.get_or_init(|| Mutex::new(Vec::new())),
            activeContainers
                .iter()
                .flat_map(|container| registrations(container, &container.promptInputHooks))
                .collect(),
        );
        replace_hooks(
            PROMPT_HISTORY_HOOKS.get_or_init(|| Mutex::new(Vec::new())),
            activeContainers
                .iter()
                .flat_map(|container| registrations(container, &container.promptHistoryHooks))
                .collect(),
        );
        replace_hooks(
            PROMPT_ESTIMATE_HISTORY_HOOKS.get_or_init(|| Mutex::new(Vec::new())),
            activeContainers
                .iter()
                .flat_map(|container| {
                    registrations(container, &container.promptEstimateHistoryHooks)
                })
                .collect(),
        );
        replace_hooks(
            SYSTEM_PROMPT_COMPOSE_HOOKS.get_or_init(|| Mutex::new(Vec::new())),
            activeContainers
                .iter()
                .flat_map(|container| registrations(container, &container.systemPromptComposeHooks))
                .collect(),
        );
        replace_hooks(
            TOOL_PROMPT_COMPOSE_HOOKS.get_or_init(|| Mutex::new(Vec::new())),
            activeContainers
                .iter()
                .flat_map(|container| registrations(container, &container.toolPromptComposeHooks))
                .collect(),
        );
        replace_hooks(
            PROMPT_FINALIZE_HOOKS.get_or_init(|| Mutex::new(Vec::new())),
            activeContainers
                .iter()
                .flat_map(|container| registrations(container, &container.promptFinalizeHooks))
                .collect(),
        );
        replace_hooks(
            PROMPT_ESTIMATE_FINALIZE_HOOKS.get_or_init(|| Mutex::new(Vec::new())),
            activeContainers
                .iter()
                .flat_map(|container| {
                    registrations(container, &container.promptEstimateFinalizeHooks)
                })
                .collect(),
        );
    }
}

fn replace_hooks(
    target: &Mutex<Vec<ToolPkgPromptHookRegistration>>,
    mut updated: Vec<ToolPkgPromptHookRegistration>,
) {
    updated.sort_by(|left, right| {
        left.containerPackageName
            .cmp(&right.containerPackageName)
            .then(left.hookId.cmp(&right.hookId))
    });
    *target.lock().expect("toolpkg prompt hook mutex poisoned") = updated;
}

fn registrations(
    container: &ToolPkgContainerRuntime,
    hooks: &[operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgFunctionHookRuntime],
) -> Vec<ToolPkgPromptHookRegistration> {
    hooks
        .iter()
        .map(|hook| ToolPkgPromptHookRegistration {
            containerPackageName: container.packageName.clone(),
            hookId: hook.id.clone(),
            functionName: hook.function.clone(),
            functionSource: hook.functionSource.clone(),
        })
        .collect()
}

/// Reports a timed-out ToolPkg prompt hook through the active send turn's Toast callback.
fn report_prompt_hook_timeout(context: &PromptHookContext, hook: &ToolPkgPromptHookRegistration) {
    if let Some(callback) = context.on_hook_timeout.as_ref() {
        callback(format!("{}:{}", hook.containerPackageName, hook.hookId));
    }
}

struct PromptInputBridge {
    runtime: ToolPkgBridgeRuntime,
}
struct PromptHistoryBridge {
    runtime: ToolPkgBridgeRuntime,
}
struct PromptEstimateHistoryBridge {
    runtime: ToolPkgBridgeRuntime,
}
struct SystemPromptComposeBridge {
    runtime: ToolPkgBridgeRuntime,
}
struct ToolPromptComposeBridge {
    runtime: ToolPkgBridgeRuntime,
}
struct PromptFinalizeBridge {
    runtime: ToolPkgBridgeRuntime,
}
struct PromptEstimateFinalizeBridge {
    runtime: ToolPkgBridgeRuntime,
}

macro_rules! prompt_bridge {
    ($bridge:ident, $trait_name:ident, $id:literal, $hooks:ident, $event:expr) => {
        impl $trait_name for $bridge {
            fn id(&self) -> &str {
                $id
            }

            /// Awaits the ToolPkg mutation on the Host-owned execution boundary.
            fn on_event_async<'a>(&'a self, context: &'a PromptHookContext) -> operit_providers::chat::hooks::PromptHookRegistry::PromptHookFuture<'a> {
                Box::pin(async move { dispatch_prompt_hooks(
                    &self.runtime,
                    $hooks.get_or_init(|| Mutex::new(Vec::new())),
                    $event,
                    context,
                ).await })
            }
        }
    };
}

prompt_bridge!(
    PromptInputBridge,
    PromptInputHook,
    "builtin.toolpkg.prompt-input-bridge",
    PROMPT_INPUT_HOOKS,
    TOOLPKG_EVENT_PROMPT_INPUT
);
prompt_bridge!(
    PromptHistoryBridge,
    PromptHistoryHook,
    "builtin.toolpkg.prompt-history-bridge",
    PROMPT_HISTORY_HOOKS,
    TOOLPKG_EVENT_PROMPT_HISTORY
);
prompt_bridge!(
    PromptEstimateHistoryBridge,
    PromptEstimateHistoryHook,
    "builtin.toolpkg.prompt-estimate-history-bridge",
    PROMPT_ESTIMATE_HISTORY_HOOKS,
    TOOLPKG_EVENT_PROMPT_ESTIMATE_HISTORY
);
prompt_bridge!(
    SystemPromptComposeBridge,
    SystemPromptComposeHook,
    "builtin.toolpkg.system-prompt-compose-bridge",
    SYSTEM_PROMPT_COMPOSE_HOOKS,
    TOOLPKG_EVENT_SYSTEM_PROMPT_COMPOSE
);
impl ToolPromptComposeHook for ToolPromptComposeBridge {
    /// Identifies the existing ToolPkg bridge for the real tool-prompt policy dispatcher.
    fn id(&self) -> &str {
        "builtin.toolpkg.tool-prompt-compose-bridge"
    }

    /// Awaits enabled owner-isolated policies and propagates any invoked handler failure.
    fn on_event_async<'a>(&'a self, context: &'a PromptHookContext) -> ToolPromptHookFuture<'a> {
        let executionContext = ToolExecutionManager::currentToolRuntimeContext();
        Box::pin(async move {
            let manager = RuntimePackageManager::readySnapshot(
                self.runtime.tool_handler().getOrCreatePackageManager(),
            ).await?;
            ToolPkgToolLifecycleBridge::validateExecutionSnapshot(&manager, executionContext.as_ref())?;
            let budget = ToolPkgPreHookTimeout::fromPreferences();
            let mut current = context.clone();
            let mut combined = PromptHookMutation::default();
            let mut changed = false;
            for container in manager.getEnabledToolPkgContainerRuntimes() {
                for hook in container.toolPromptComposeHooks {
                    let ownerContext = ToolPkgToolLifecycleBridge::executionContextForOwner(
                        &manager, executionContext.as_ref(), &container.packageName,
                    )?;
                    let mut payload = prompt_context_to_value(&current);
                    payload["metadata"]["executionContext"] = ownerContext;
                    let timeoutMillis = budget.remainingTimeoutMillis().ok_or_else(|| format!(
                        "Tool-prompt policy timed out before handler {}:{}", container.packageName, hook.id,
                    ))?;
                    let raw = manager.runToolPkgMainHookWithTimeoutMillis(
                        &container.packageName, &hook.function, TOOLPKG_EVENT_TOOL_PROMPT_COMPOSE,
                        Some(&current.stage), Some(&hook.id), hook.functionSource.as_deref(),
                        payload, None, None, None, timeoutMillis,
                    ).await?;
                    if budget.hasExpired() {
                        return Err(format!("Tool-prompt policy timed out after handler {}:{}", container.packageName, hook.id));
                    }
                    if let Some(mutation) = decode_tool_prompt_policy_result(raw)? {
                        apply_prompt_mutation(&mut current, mutation.clone());
                        merge_prompt_mutation(&mut combined, mutation);
                        changed = true;
                    }
                }
            }
            Ok(if changed { Some(combined) } else { None })
        })
    }
}

/// Strictly decodes an invoked tool policy without converting malformed fields into absent mutations.
pub(super) fn decode_tool_prompt_policy_result(raw: Option<String>) -> Result<Option<PromptHookMutation>, String> {
    let Some(raw) = raw else { return Ok(None); };
    let value: Value = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    match value {
        Value::Null => Ok(None),
        Value::String(prompt) => Ok(Some(PromptHookMutation {
            tool_prompt: Some(prompt), ..PromptHookMutation::default()
        })),
        Value::Object(mut object) => {
            let mut mutation = PromptHookMutation::default();
            for (key, target) in [
                ("rawInput", &mut mutation.raw_input),
                ("processedInput", &mut mutation.processed_input),
                ("systemPrompt", &mut mutation.system_prompt),
                ("toolPrompt", &mut mutation.tool_prompt),
            ] {
                if let Some(value) = object.remove(key) {
                    *target = Some(serde_json::from_value::<String>(value).map_err(|error| format!("Tool-prompt policy {key}: {error}"))?);
                }
            }
            if let Some(value) = object.remove("availableTools") {
                mutation.available_tools = Some(serde_json::from_value(value).map_err(|error| format!("Tool-prompt policy availableTools: {error}"))?);
            }
            if let Some(value) = object.remove("chatHistory") {
                mutation.chat_history = Some(serde_json::from_value(value).map_err(|error| format!("Tool-prompt policy chatHistory: {error}"))?);
            }
            if let Some(value) = object.remove("preparedHistory") {
                mutation.prepared_history = Some(serde_json::from_value(value).map_err(|error| format!("Tool-prompt policy preparedHistory: {error}"))?);
            }
            if let Some(value) = object.remove("metadata") {
                mutation.metadata = serde_json::from_value(value).map_err(|error| format!("Tool-prompt policy metadata: {error}"))?;
                if mutation.metadata.contains_key("executionContext") {
                    return Err("Tool-prompt policy cannot replace its authenticated executionContext".to_string());
                }
            }
            Ok(Some(mutation))
        }
        _ => Err("Tool-prompt policy result must be a string, object, explicit null or void".to_string()),
    }
}
prompt_bridge!(
    PromptFinalizeBridge,
    PromptFinalizeHook,
    "builtin.toolpkg.prompt-finalize-bridge",
    PROMPT_FINALIZE_HOOKS,
    TOOLPKG_EVENT_PROMPT_FINALIZE
);
prompt_bridge!(
    PromptEstimateFinalizeBridge,
    PromptEstimateFinalizeHook,
    "builtin.toolpkg.prompt-estimate-finalize-bridge",
    PROMPT_ESTIMATE_FINALIZE_HOOKS,
    TOOLPKG_EVENT_PROMPT_ESTIMATE_FINALIZE
);

async fn dispatch_prompt_hooks(
    runtime: &ToolPkgBridgeRuntime,
    hooks: &Mutex<Vec<ToolPkgPromptHookRegistration>>,
    event: &str,
    context: &PromptHookContext,
) -> Option<PromptHookMutation> {
    let snapshot = hooks
        .lock()
        .expect("toolpkg prompt hook mutex poisoned")
        .clone();
    let mut current = context.clone();
    let mut mutation = PromptHookMutation::default();
    let mut changed = false;
    let package_manager = runtime.package_manager();
    let budget = ToolPkgPreHookTimeout::fromPreferences();
    for hook in snapshot {
        let Some(timeoutMillis) = budget.remainingTimeoutMillis() else {
            report_prompt_hook_timeout(&current, &hook);
            ChainLogger::error(
                PLUGIN_CHAIN,
                "plugin.toolpkg.prompt.timeout",
                &[
                    ("event", event.to_string()),
                    ("stage", current.stage.clone()),
                    ("phase", "before_hook".to_string()),
                ],
            );
            break;
        };
        ChainLogger::info(
            PLUGIN_CHAIN,
            "plugin.toolpkg.prompt.run.start",
            &[
                ("event", event.to_string()),
                ("stage", current.stage.clone()),
                ("package", hook.containerPackageName.clone()),
                ("hookId", hook.hookId.clone()),
                ("function", hook.functionName.clone()),
            ],
        );
        let raw = package_manager
            .runToolPkgMainHookWithTimeoutMillis(
                &hook.containerPackageName,
                &hook.functionName,
                event,
                Some(&current.stage),
                Some(&hook.hookId),
                hook.functionSource.as_deref(),
                prompt_context_to_value(&current),
                None,
                None,
                None,
                timeoutMillis,
            )
            .await;
        let hookTimedOut = raw
            .as_ref()
            .err()
            .map(|error| ToolPkgPreHookTimeout::isTimeoutError(error))
            .unwrap_or(false);
        if hookTimedOut || budget.hasExpired() {
            report_prompt_hook_timeout(&current, &hook);
            ChainLogger::error(
                PLUGIN_CHAIN,
                "plugin.toolpkg.prompt.timeout",
                &[
                    ("event", event.to_string()),
                    ("stage", current.stage.clone()),
                    ("package", hook.containerPackageName.clone()),
                    ("hookId", hook.hookId.clone()),
                ],
            );
            break;
        }
        let result = match raw {
            Ok(raw) => decodeToolPkgHookResult(raw),
            Err(error) => {
                ChainLogger::error(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.prompt.run.error",
                    &[
                        ("event", event.to_string()),
                        ("stage", current.stage.clone()),
                        ("package", hook.containerPackageName.clone()),
                        ("hookId", hook.hookId.clone()),
                        ("function", hook.functionName.clone()),
                        ("error", error.clone()),
                    ],
                );
                AppLogger::e(
                    TAG,
                    &format!(
                        "ToolPkg prompt hook failed: {}:{} {}",
                        hook.containerPackageName, hook.hookId, error
                    ),
                );
                None
            }
        };
        if let Some(next_mutation) = parse_prompt_hook_result(event, result.as_ref(), &current) {
            apply_prompt_mutation(&mut current, next_mutation.clone());
            merge_prompt_mutation(&mut mutation, next_mutation);
            changed = true;
            ChainLogger::info(
                PLUGIN_CHAIN,
                "plugin.toolpkg.prompt.run.changed",
                &[
                    ("event", event.to_string()),
                    ("stage", current.stage.clone()),
                    ("package", hook.containerPackageName.clone()),
                    ("hookId", hook.hookId.clone()),
                ],
            );
        } else {
            ChainLogger::info(
                PLUGIN_CHAIN,
                "plugin.toolpkg.prompt.run.done",
                &[
                    ("event", event.to_string()),
                    ("stage", current.stage.clone()),
                    ("package", hook.containerPackageName.clone()),
                    ("hookId", hook.hookId.clone()),
                    ("changed", ChainLogger::boolField(false)),
                ],
            );
        }
    }
    if changed {
        Some(mutation)
    } else {
        None
    }
}

/// Serializes prompt histories and omits absent optional fields for the plugin SDK contract.
fn prompt_context_to_value(context: &PromptHookContext) -> Value {
    let mut payload = serde_json::json!({
        "stage": context.stage,
        "chatId": context.chat_id,
        "functionType": context.function_type,
        "promptFunctionType": context.prompt_function_type,
        "useEnglish": context.use_english,
        "rawInput": context.raw_input,
        "processedInput": context.processed_input,
        "chatHistory": context.chat_history,
        "preparedHistory": context.prepared_history,
        "systemPrompt": context.system_prompt,
        "toolPrompt": context.tool_prompt,
        "modelParameters": context.model_parameters,
        "availableTools": context.available_tools,
        "metadata": context.metadata
    });
    payload.as_object_mut().expect("prompt payload must be an object")
        .retain(|_, value| !value.is_null());
    payload
}

#[cfg(test)]
mod payload_tests {
    use super::*;

    /// Preserves complete prompt turns across the plugin payload and mutation boundary.
    #[test]
    fn prompt_histories_round_trip_through_plugin_payload() {
        let context = PromptHookContext {
            chat_history: vec![PromptTurn::new(PromptTurnKind::USER, "Design login")],
            prepared_history: vec![PromptTurn::new(PromptTurnKind::SYSTEM, "Plan instructions")],
            ..PromptHookContext::default()
        };
        let payload = prompt_context_to_value(&context);
        let mutation = parse_prompt_object_result(payload.as_object().unwrap());
        assert_eq!(mutation.chat_history.unwrap(), context.chat_history);
        assert_eq!(mutation.prepared_history.unwrap(), context.prepared_history);
        assert!(payload.get("promptFunctionType").is_none());
        assert!(payload.get("functionType").is_none());

        let explicit = PromptHookContext {
            prompt_function_type: Some("VOICE".to_string()),
            use_english: Some(false),
            ..context
        };
        let payload = prompt_context_to_value(&explicit);
        assert_eq!(payload["promptFunctionType"], "VOICE");
        assert_eq!(payload["useEnglish"], false);
    }
}

fn parse_prompt_hook_result(
    event: &str,
    decoded: Option<&Value>,
    context: &PromptHookContext,
) -> Option<PromptHookMutation> {
    match decoded? {
        Value::String(value) => parse_prompt_string_result(event, value),
        Value::Array(values) => parse_prompt_array_result(event, values, context),
        Value::Object(object) => Some(parse_prompt_object_result(object)),
        _ => None,
    }
}

fn parse_prompt_string_result(event: &str, value: &str) -> Option<PromptHookMutation> {
    if value.trim().is_empty() {
        return None;
    }
    let mut mutation = PromptHookMutation::default();
    match event {
        TOOLPKG_EVENT_PROMPT_INPUT
        | TOOLPKG_EVENT_PROMPT_FINALIZE
        | TOOLPKG_EVENT_PROMPT_ESTIMATE_FINALIZE => {
            mutation.processed_input = Some(value.to_string());
        }
        TOOLPKG_EVENT_SYSTEM_PROMPT_COMPOSE => {
            mutation.system_prompt = Some(value.to_string());
        }
        TOOLPKG_EVENT_TOOL_PROMPT_COMPOSE => {
            mutation.tool_prompt = Some(value.to_string());
        }
        _ => return None,
    }
    Some(mutation)
}

fn parse_prompt_array_result(
    event: &str,
    values: &[Value],
    context: &PromptHookContext,
) -> Option<PromptHookMutation> {
    let turns = parse_prompt_turns(values)?;
    let mut mutation = PromptHookMutation::default();
    match event {
        TOOLPKG_EVENT_PROMPT_HISTORY | TOOLPKG_EVENT_PROMPT_ESTIMATE_HISTORY => {
            if context.stage == "before_prepare_history" {
                mutation.chat_history = Some(turns);
            } else {
                mutation.prepared_history = Some(turns);
            }
        }
        TOOLPKG_EVENT_PROMPT_FINALIZE | TOOLPKG_EVENT_PROMPT_ESTIMATE_FINALIZE => {
            mutation.prepared_history = Some(turns);
        }
        _ => return None,
    }
    Some(mutation)
}

fn parse_prompt_object_result(object: &serde_json::Map<String, Value>) -> PromptHookMutation {
    let mut mutation = PromptHookMutation::default();
    if let Some(value) = object
        .get("rawInput")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        mutation.raw_input = Some(value.to_string());
    }
    if let Some(value) = object
        .get("processedInput")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        mutation.processed_input = Some(value.to_string());
    }
    if let Some(Value::Array(values)) = object.get("chatHistory") {
        mutation.chat_history = parse_prompt_turns(values);
    }
    if let Some(Value::Array(values)) = object.get("preparedHistory") {
        mutation.prepared_history = parse_prompt_turns(values);
    }
    if let Some(value) = object
        .get("systemPrompt")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        mutation.system_prompt = Some(value.to_string());
    }
    if let Some(value) = object
        .get("toolPrompt")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        mutation.tool_prompt = Some(value.to_string());
    }
    if let Some(Value::Array(values)) = object.get("availableTools") {
        mutation.available_tools = Some(
            values
                .iter()
                .filter_map(|value| value.as_object().cloned())
                .map(|object| object.into_iter().collect())
                .collect(),
        );
    }
    if let Some(Value::Object(metadata)) = object.get("metadata") {
        mutation.metadata.extend(metadata.clone());
    }
    mutation
}

fn parse_prompt_turns(values: &[Value]) -> Option<Vec<PromptTurn>> {
    let mut turns = Vec::new();
    for value in values {
        let Some(object) = value.as_object() else {
            continue;
        };
        let Some(kind) = object
            .get("kind")
            .and_then(Value::as_str)
            .and_then(parse_prompt_turn_kind)
        else {
            continue;
        };
        let content = object
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let tool_name = object
            .get("toolName")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        let metadata = object
            .get("metadata")
            .and_then(Value::as_object)
            .cloned()
            .map(|metadata| metadata.into_iter().collect())
            .unwrap_or_default();
        turns.push(PromptTurn {
            kind,
            content,
            tool_name,
            metadata,
        });
    }
    Some(turns)
}

fn parse_prompt_turn_kind(value: &str) -> Option<PromptTurnKind> {
    match value.trim().to_ascii_uppercase().as_str() {
        "SYSTEM" => Some(PromptTurnKind::SYSTEM),
        "USER" => Some(PromptTurnKind::USER),
        "ASSISTANT" => Some(PromptTurnKind::ASSISTANT),
        "TOOL_CALL" => Some(PromptTurnKind::TOOL_CALL),
        "TOOL_RESULT" => Some(PromptTurnKind::TOOL_RESULT),
        "SUMMARY" => Some(PromptTurnKind::SUMMARY),
        _ => None,
    }
}

fn merge_prompt_mutation(target: &mut PromptHookMutation, mutation: PromptHookMutation) {
    if mutation.raw_input.is_some() {
        target.raw_input = mutation.raw_input;
    }
    if mutation.processed_input.is_some() {
        target.processed_input = mutation.processed_input;
    }
    if mutation.chat_history.is_some() {
        target.chat_history = mutation.chat_history;
    }
    if mutation.prepared_history.is_some() {
        target.prepared_history = mutation.prepared_history;
    }
    if mutation.system_prompt.is_some() {
        target.system_prompt = mutation.system_prompt;
    }
    if mutation.tool_prompt.is_some() {
        target.tool_prompt = mutation.tool_prompt;
    }
    if mutation.available_tools.is_some() {
        target.available_tools = mutation.available_tools;
    }
    if !mutation.metadata.is_empty() {
        target.metadata.extend(mutation.metadata);
    }
}

fn apply_prompt_mutation(current: &mut PromptHookContext, mutation: PromptHookMutation) {
    if let Some(raw_input) = mutation.raw_input {
        current.raw_input = Some(raw_input);
    }
    if let Some(processed_input) = mutation.processed_input {
        current.processed_input = Some(processed_input);
    }
    if let Some(chat_history) = mutation.chat_history {
        current.chat_history = chat_history;
    }
    if let Some(prepared_history) = mutation.prepared_history {
        current.prepared_history = prepared_history;
    }
    if let Some(system_prompt) = mutation.system_prompt {
        current.system_prompt = Some(system_prompt);
    }
    if let Some(tool_prompt) = mutation.tool_prompt {
        current.tool_prompt = Some(tool_prompt);
    }
    if let Some(available_tools) = mutation.available_tools {
        current.available_tools = available_tools;
    }
    if !mutation.metadata.is_empty() {
        current.metadata.extend(mutation.metadata);
    }
}

#[cfg(test)]
mod tool_policy_result_tests {
    use super::decode_tool_prompt_policy_result;

    /// Rejects malformed invoked policies instead of treating invalid output as permission to continue.
    #[test]
    fn rejects_malformed_policy_results() {
        for raw in [
            "not-json", "17", "true", "[]",
            r#"{"toolPrompt":null}"#,
            r#"{"availableTools":{}}"#,
            r#"{"availableTools":[17]}"#,
            r#"{"chatHistory":"invalid"}"#,
            r#"{"preparedHistory":[{"kind":"invalid"}]}"#,
            r#"{"metadata":null}"#,
            r#"{"metadata":{"executionContext":null}}"#,
        ] {
            assert!(decode_tool_prompt_policy_result(Some(raw.to_string())).is_err(), "{raw}");
        }
    }

    /// Distinguishes an explicit no-contribution result from valid exact JSON mutations.
    #[test]
    fn preserves_explicit_nonparticipation_and_exact_tool_mutations() {
        assert!(decode_tool_prompt_policy_result(None).unwrap().is_none());
        assert!(decode_tool_prompt_policy_result(Some("null".to_string())).unwrap().is_none());
        let mutation = decode_tool_prompt_policy_result(Some(r#"{"toolPrompt":" exact prompt ","availableTools":[]}"#.to_string())).unwrap().unwrap();
        assert_eq!(mutation.tool_prompt.as_deref(), Some(" exact prompt "));
        assert_eq!(mutation.available_tools, Some(Vec::new()));
    }
}
