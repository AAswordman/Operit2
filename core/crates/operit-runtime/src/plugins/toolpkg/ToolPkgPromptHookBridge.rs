use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;

use crate::core::chat::hooks::PromptHookRegistry::{
    PromptEstimateFinalizeHook, PromptEstimateHistoryHook, PromptFinalizeHook, PromptHistoryHook,
    PromptHookContext, PromptHookMutation, PromptHookRegistry, PromptInputHook,
    SystemPromptComposeHook, ToolPromptComposeHook,
};
use crate::core::chat::hooks::PromptTurn::{PromptTurn, PromptTurnKind};
use crate::core::tools::packTool::ToolPkgCommonPluginConstants::{
    TOOLPKG_EVENT_PROMPT_ESTIMATE_FINALIZE, TOOLPKG_EVENT_PROMPT_ESTIMATE_HISTORY,
    TOOLPKG_EVENT_PROMPT_FINALIZE, TOOLPKG_EVENT_PROMPT_HISTORY, TOOLPKG_EVENT_PROMPT_INPUT,
    TOOLPKG_EVENT_SYSTEM_PROMPT_COMPOSE, TOOLPKG_EVENT_TOOL_PROMPT_COMPOSE,
};
use crate::core::tools::packTool::ToolPkgParser::ToolPkgContainerRuntime;
use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::{
    decodeToolPkgHookResult, toolPkgPackageManager, ToolPkgPromptHookRegistration,
};
use crate::util::AppLogger::AppLogger;
use crate::util::ChainLogger::{self, PLUGIN_CHAIN};

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
    pub fn register() {
        PromptHookRegistry::registerPromptInputHook(Arc::new(PromptInputBridge));
        PromptHookRegistry::registerPromptHistoryHook(Arc::new(PromptHistoryBridge));
        PromptHookRegistry::registerPromptEstimateHistoryHook(Arc::new(
            PromptEstimateHistoryBridge,
        ));
        PromptHookRegistry::registerSystemPromptComposeHook(Arc::new(SystemPromptComposeBridge));
        PromptHookRegistry::registerToolPromptComposeHook(Arc::new(ToolPromptComposeBridge));
        PromptHookRegistry::registerPromptFinalizeHook(Arc::new(PromptFinalizeBridge));
        PromptHookRegistry::registerPromptEstimateFinalizeHook(Arc::new(
            PromptEstimateFinalizeBridge,
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
    hooks: &[crate::core::tools::packTool::ToolPkgParser::ToolPkgFunctionHookRuntime],
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

struct PromptInputBridge;
struct PromptHistoryBridge;
struct PromptEstimateHistoryBridge;
struct SystemPromptComposeBridge;
struct ToolPromptComposeBridge;
struct PromptFinalizeBridge;
struct PromptEstimateFinalizeBridge;

macro_rules! prompt_bridge {
    ($bridge:ident, $trait_name:ident, $id:literal, $hooks:ident, $event:expr) => {
        impl $trait_name for $bridge {
            fn id(&self) -> &str {
                $id
            }

            fn on_event(&self, context: &PromptHookContext) -> Option<PromptHookMutation> {
                dispatch_prompt_hooks(
                    $hooks.get_or_init(|| Mutex::new(Vec::new())),
                    $event,
                    context,
                )
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
prompt_bridge!(
    ToolPromptComposeBridge,
    ToolPromptComposeHook,
    "builtin.toolpkg.tool-prompt-compose-bridge",
    TOOL_PROMPT_COMPOSE_HOOKS,
    TOOLPKG_EVENT_TOOL_PROMPT_COMPOSE
);
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

fn dispatch_prompt_hooks(
    hooks: &Mutex<Vec<ToolPkgPromptHookRegistration>>,
    event: &str,
    context: &PromptHookContext,
) -> Option<PromptHookMutation> {
    let snapshot = hooks
        .lock()
        .expect("toolpkg prompt hook mutex poisoned")
        .clone();
    ChainLogger::info(
        PLUGIN_CHAIN,
        "plugin.toolpkg.prompt.scan",
        &[
            ("event", event.to_string()),
            ("stage", context.stage.clone()),
            ("hookCount", snapshot.len().to_string()),
        ],
    );
    let mut current = context.clone();
    let mut mutation = PromptHookMutation::default();
    let mut changed = false;
    let package_manager = toolPkgPackageManager();
    for hook in snapshot {
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
        let result = match package_manager.runToolPkgMainHook(
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
        ) {
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

fn prompt_context_to_value(context: &PromptHookContext) -> Value {
    serde_json::json!({
        "stage": context.stage,
        "chatId": context.chat_id,
        "functionType": context.function_type,
        "promptFunctionType": context.prompt_function_type,
        "useEnglish": context.use_english,
        "rawInput": context.raw_input,
        "processedInput": context.processed_input,
        "systemPrompt": context.system_prompt,
        "toolPrompt": context.tool_prompt,
        "modelParameters": context.model_parameters,
        "availableTools": context.available_tools,
        "metadata": context.metadata
    })
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
