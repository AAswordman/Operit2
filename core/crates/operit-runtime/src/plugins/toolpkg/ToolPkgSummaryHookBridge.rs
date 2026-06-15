use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;

use crate::core::chat::hooks::SummaryHookRegistry::{
    SummaryGenerateHook, SummaryHookContext, SummaryHookMutation, SummaryHookRegistry,
};
use crate::core::tools::packTool::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_SUMMARY_GENERATE;
use crate::core::tools::packTool::ToolPkgParser::ToolPkgContainerRuntime;
use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::{
    decodeToolPkgHookResult, toolPkgPackageManager, ToolPkgPromptHookRegistration,
};
use crate::util::ChainLogger::{self, PLUGIN_CHAIN};

static SUMMARY_GENERATE_HOOKS: OnceLock<Mutex<Vec<ToolPkgPromptHookRegistration>>> =
    OnceLock::new();

pub struct ToolPkgSummaryHookBridge;

impl ToolPkgSummaryHookBridge {
    pub fn register() {
        SummaryHookRegistry::registerSummaryGenerateHook(Arc::new(SummaryGenerateBridge));
    }

    #[allow(non_snake_case)]
    pub fn syncToolPkgRegistrations(activeContainers: Vec<ToolPkgContainerRuntime>) {
        let hooks = activeContainers
            .iter()
            .flat_map(|container| {
                container
                    .summaryGenerateHooks
                    .iter()
                    .map(|hook| ToolPkgPromptHookRegistration {
                        containerPackageName: container.packageName.clone(),
                        hookId: hook.id.clone(),
                        functionName: hook.function.clone(),
                        functionSource: hook.functionSource.clone(),
                    })
            })
            .collect::<Vec<_>>();
        *SUMMARY_GENERATE_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg summary hook mutex poisoned") = hooks;
    }
}

struct SummaryGenerateBridge;

impl SummaryGenerateHook for SummaryGenerateBridge {
    fn id(&self) -> &str {
        "builtin.toolpkg.summary-generate-bridge"
    }

    fn on_event(&self, context: &SummaryHookContext) -> Option<SummaryHookMutation> {
        let snapshot = SUMMARY_GENERATE_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg summary hook mutex poisoned")
            .clone();
        ChainLogger::info(
            PLUGIN_CHAIN,
            "plugin.toolpkg.summary.scan",
            &[
                ("stage", context.stage.clone()),
                ("hookCount", snapshot.len().to_string()),
            ],
        );
        let mut mutation = SummaryHookMutation::default();
        let mut changed = false;
        let manager = toolPkgPackageManager();
        for hook in snapshot {
            ChainLogger::info(
                PLUGIN_CHAIN,
                "plugin.toolpkg.summary.run.start",
                &[
                    ("stage", context.stage.clone()),
                    ("package", hook.containerPackageName.clone()),
                    ("hookId", hook.hookId.clone()),
                    ("function", hook.functionName.clone()),
                ],
            );
            let result = match manager.runToolPkgMainHook(
                &hook.containerPackageName,
                &hook.functionName,
                TOOLPKG_EVENT_SUMMARY_GENERATE,
                None,
                Some(&hook.hookId),
                hook.functionSource.as_deref(),
                summary_context_to_value(context),
                None,
                None,
                None,
            ) {
                Ok(raw) => decodeToolPkgHookResult(raw),
                Err(error) => {
                    ChainLogger::error(
                        PLUGIN_CHAIN,
                        "plugin.toolpkg.summary.run.error",
                        &[
                            ("stage", context.stage.clone()),
                            ("package", hook.containerPackageName.clone()),
                            ("hookId", hook.hookId.clone()),
                            ("function", hook.functionName.clone()),
                            ("error", error),
                        ],
                    );
                    None
                }
            };
            if let Some(Value::Object(object)) = result {
                let hookChanged = apply_summary_object_result(&mut mutation, object);
                changed |= hookChanged;
                ChainLogger::info(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.summary.run.done",
                    &[
                        ("stage", context.stage.clone()),
                        ("package", hook.containerPackageName.clone()),
                        ("hookId", hook.hookId.clone()),
                        ("changed", ChainLogger::boolField(hookChanged)),
                    ],
                );
            } else {
                ChainLogger::info(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.summary.run.done",
                    &[
                        ("stage", context.stage.clone()),
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
}

fn summary_context_to_value(context: &SummaryHookContext) -> Value {
    serde_json::json!({
        "stage": context.stage,
        "useEnglish": context.use_english,
        "previousSummary": context.previous_summary,
        "systemPrompt": context.system_prompt,
        "summaryPrompt": context.summary_prompt,
        "summaryResult": context.summary_result,
        "modelParameters": context.model_parameters,
        "metadata": context.metadata
    })
}

fn apply_summary_object_result(
    mutation: &mut SummaryHookMutation,
    object: serde_json::Map<String, Value>,
) -> bool {
    let mut changed = false;
    if let Some(value) = object.get("systemPrompt").and_then(Value::as_str) {
        mutation.system_prompt = Some(value.to_string());
        changed = true;
    }
    if let Some(value) = object.get("summaryPrompt").and_then(Value::as_str) {
        mutation.summary_prompt = Some(value.to_string());
        changed = true;
    }
    if let Some(value) = object.get("summaryResult").and_then(Value::as_str) {
        mutation.summary_result = Some(value.to_string());
        changed = true;
    }
    if let Some(Value::Object(metadata)) = object.get("metadata") {
        mutation.metadata.extend(metadata.clone());
        changed = true;
    }
    changed
}
