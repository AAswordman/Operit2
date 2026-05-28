use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;

use crate::core::chat::hooks::PromptHookRegistry::{
    PromptEstimateFinalizeHook, PromptEstimateHistoryHook, PromptFinalizeHook, PromptHistoryHook,
    PromptHookContext, PromptHookMutation, PromptHookRegistry, PromptInputHook,
    SystemPromptComposeHook, ToolPromptComposeHook,
};
use crate::core::tools::packTool::ToolPkgCommonPluginConstants::{
    TOOLPKG_EVENT_PROMPT_ESTIMATE_FINALIZE, TOOLPKG_EVENT_PROMPT_ESTIMATE_HISTORY,
    TOOLPKG_EVENT_PROMPT_FINALIZE, TOOLPKG_EVENT_PROMPT_HISTORY, TOOLPKG_EVENT_PROMPT_INPUT,
    TOOLPKG_EVENT_SYSTEM_PROMPT_COMPOSE, TOOLPKG_EVENT_TOOL_PROMPT_COMPOSE,
};
use crate::core::tools::packTool::ToolPkgParser::ToolPkgContainerRuntime;
use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::{
    decodeToolPkgHookResult, ToolPkgPromptHookRegistration,
};

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
    updated: Vec<ToolPkgPromptHookRegistration>,
) {
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
    let mut mutation = PromptHookMutation::default();
    let mut changed = false;
    for hook in snapshot {
        let result = crate::core::tools::AIToolHandler::AIToolHandler::getInstance(
            crate::core::application::OperitApplicationContext::OperitApplicationContext::new(),
        )
        .getOrCreatePackageManager()
        .lock()
        .expect("package manager mutex poisoned")
        .runToolPkgMainHook(
            &hook.containerPackageName,
            &hook.functionName,
            event,
            None,
            Some(&hook.hookId),
            hook.functionSource.as_deref(),
            prompt_context_to_value(context),
            None,
            None,
            None,
        )
        .ok()
        .and_then(decodeToolPkgHookResult);
        if let Some(Value::Object(object)) = result {
            changed |= apply_prompt_object_result(&mut mutation, object);
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

fn apply_prompt_object_result(
    mutation: &mut PromptHookMutation,
    object: serde_json::Map<String, Value>,
) -> bool {
    let mut changed = false;
    if let Some(value) = object.get("rawInput").and_then(Value::as_str) {
        mutation.raw_input = Some(value.to_string());
        changed = true;
    }
    if let Some(value) = object.get("processedInput").and_then(Value::as_str) {
        mutation.processed_input = Some(value.to_string());
        changed = true;
    }
    if let Some(value) = object.get("systemPrompt").and_then(Value::as_str) {
        mutation.system_prompt = Some(value.to_string());
        changed = true;
    }
    if let Some(value) = object.get("toolPrompt").and_then(Value::as_str) {
        mutation.tool_prompt = Some(value.to_string());
        changed = true;
    }
    if let Some(Value::Object(metadata)) = object.get("metadata") {
        mutation.metadata.extend(metadata.clone());
        changed = true;
    }
    changed
}
