use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;

use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use crate::plugins::toolpkg::ToolPkgPreHookTimeout::ToolPkgPreHookTimeout;
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_SUMMARY_GENERATE;
use operit_plugin_sdk::toolpkg::ToolPkgHooks::{
    decodeToolPkgHookResult, ToolPkgPromptHookRegistration,
};
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;
use operit_providers::chat::hooks::SummaryHookRegistry::{
    applySummaryHookMutation, SummaryGenerateHook, SummaryHookContext, SummaryHookMutation,
    SummaryHookRegistry,
};
use operit_util::ChainLogger::{self, PLUGIN_CHAIN};

static SUMMARY_GENERATE_HOOKS: OnceLock<Mutex<Vec<ToolPkgPromptHookRegistration>>> =
    OnceLock::new();

pub struct ToolPkgSummaryHookBridge;

impl ToolPkgSummaryHookBridge {
    /// Registers summary hooks for one application runtime.
    pub fn register(runtime: ToolPkgBridgeRuntime) {
        SummaryHookRegistry::registerSummaryGenerateHook(Arc::new(SummaryGenerateBridge {
            runtime,
        }));
    }

    /// Synchronizes this bridge from the committed active package catalog.
    #[allow(non_snake_case)]
    pub fn syncToolPkgRegistrations(activeContainers: &[ToolPkgContainerRuntime]) {
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

struct SummaryGenerateBridge {
    runtime: ToolPkgBridgeRuntime,
}

impl SummaryGenerateHook for SummaryGenerateBridge {
    /// Identifies the single ToolPkg summary dispatcher.
    fn id(&self) -> &str {
        "builtin.toolpkg.summary-generate-bridge"
    }

    /// Runs summary handlers and propagates any notified before-stage failure.
    fn on_event_async<'a>(
        &'a self,
        context: &'a SummaryHookContext,
    ) -> operit_providers::chat::hooks::SummaryHookRegistry::SummaryHookFuture<'a> {
        Box::pin(async move {
            let snapshot = SUMMARY_GENERATE_HOOKS
                .get_or_init(|| Mutex::new(Vec::new()))
                .lock()
                .expect("toolpkg summary hook mutex poisoned")
                .clone();
            let mut current = context.clone();
            let mut changed = false;
            let manager = self.runtime.package_manager();
            let budget = ToolPkgPreHookTimeout::forStage(&context.stage)?;
            for hook in snapshot {
                ChainLogger::verbose_with_level(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.summary.run.start",
                    &[
                        ("stage", context.stage.clone()),
                        ("package", hook.containerPackageName.clone()),
                        ("hookId", hook.hookId.clone()),
                        ("function", hook.functionName.clone()),
                    ],
                    operit_util::AppLogger::VERBOSE_LEVEL_3,
                );
                let raw = ToolPkgPreHookTimeout::runHook(
                    budget.as_ref(),
                    &manager,
                    &hook.containerPackageName,
                    &hook.functionName,
                    TOOLPKG_EVENT_SUMMARY_GENERATE,
                    &context.stage,
                    &hook.hookId,
                    hook.functionSource.as_deref(),
                    summary_context_to_value(&current),
                )
                .await?;
                let result = decodeToolPkgHookResult(raw);
                if let Some(mutation) = parse_summary_hook_result(&context.stage, result)
                    .map_err(|error| format!("Summary hook {}:{}: {error}", hook.containerPackageName, hook.hookId))?
                {
                    current = applySummaryHookMutation(current, mutation);
                    changed = true;
                    ChainLogger::verbose_with_level(
                        PLUGIN_CHAIN,
                        "plugin.toolpkg.summary.run.done",
                        &[
                            ("stage", context.stage.clone()),
                            ("package", hook.containerPackageName.clone()),
                            ("hookId", hook.hookId.clone()),
                            ("changed", ChainLogger::boolField(true)),
                        ],
                        operit_util::AppLogger::VERBOSE_LEVEL_3,
                    );
                } else {
                    ChainLogger::verbose_with_level(
                        PLUGIN_CHAIN,
                        "plugin.toolpkg.summary.run.done",
                        &[
                            ("stage", context.stage.clone()),
                            ("package", hook.containerPackageName.clone()),
                            ("hookId", hook.hookId.clone()),
                            ("changed", ChainLogger::boolField(false)),
                        ],
                        operit_util::AppLogger::VERBOSE_LEVEL_3,
                    );
                }
            }
            Ok(if changed {
                Some(SummaryHookMutation {
                    chat_history: Some(current.chat_history),
                    prepared_history: Some(current.prepared_history),
                    system_prompt: current.system_prompt,
                    summary_prompt: current.summary_prompt,
                    summary_result: current.summary_result,
                    metadata: current.metadata,
                })
            } else { None })
        })
    }
}

/// Supplies complete summary histories and the actual conversation identity to plugins.
fn summary_context_to_value(context: &SummaryHookContext) -> Value {
    serde_json::json!({
        "stage": context.stage,
        "chatId": context.chat_id,
        "useEnglish": context.use_english,
        "previousSummary": context.previous_summary,
        "chatHistory": context.chat_history,
        "preparedHistory": context.prepared_history,
        "systemPrompt": context.system_prompt,
        "summaryPrompt": context.summary_prompt,
        "summaryResult": context.summary_result,
        "modelParameters": context.model_parameters,
        "metadata": context.metadata
    })
}

/// Validates summary results and assigns string results to the stage's primary text.
fn parse_summary_hook_result(
    stage: &str,
    decoded: Option<Value>,
) -> Result<Option<SummaryHookMutation>, String> {
    let mutation = match decoded {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::Object(object)) => serde_json::from_value(Value::Object(object))
            .map_err(|error| format!("Invalid summary mutation: {error}"))?,
        Some(Value::String(text)) => match stage {
            "before_prepare_summary_prompt" | "before_send_to_model" => SummaryHookMutation {
                summary_prompt: Some(text), ..SummaryHookMutation::default()
            },
            "after_generate_summary" => SummaryHookMutation {
                summary_result: Some(text), ..SummaryHookMutation::default()
            },
            _ => return Err(format!("Unsupported summary string result stage: {stage}")),
        },
        Some(_) => return Err("Summary hook must return an object, string, or null".into()),
    };
    let changed = mutation.chat_history.is_some() || mutation.prepared_history.is_some()
        || mutation.system_prompt.is_some() || mutation.summary_prompt.is_some()
        || mutation.summary_result.is_some() || !mutation.metadata.is_empty();
    Ok(changed.then_some(mutation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};

    /// Builds the complete summary boundary without requiring a provider or plugin engine.
    fn context() -> SummaryHookContext {
        SummaryHookContext {
            stage: "before_send_to_model".into(), chat_id: Some("summary-chat".into()),
            use_english: Some(false), previous_summary: None,
            chat_history: vec![PromptTurn::new(PromptTurnKind::USER, "original")],
            prepared_history: vec![PromptTurn::new(PromptTurnKind::SYSTEM, "instructions")],
            system_prompt: Some("system".into()), summary_prompt: Some("summarize".into()),
            summary_result: None, model_parameters: Vec::new(), metadata: Default::default(),
        }
    }

    /// Exposes exact history changes from one handler to the next, including cleared arrays.
    #[test]
    fn summary_histories_round_trip_and_chain() {
        let original = context();
        let payload = summary_context_to_value(&original);
        assert_eq!(payload["chatId"], "summary-chat");
        assert_eq!(payload["chatHistory"][0]["content"], "original");
        assert_eq!(payload["preparedHistory"][0]["content"], "instructions");
        let mutation = parse_summary_hook_result(&original.stage, Some(serde_json::json!({
            "chatHistory": [], "preparedHistory": [{"kind":"USER", "content":"rewritten", "metadata":{"source":"plugin"}}],
            "summaryPrompt":"", "metadata":{"first":true}
        }))).unwrap().unwrap();
        let next = applySummaryHookMutation(original.clone(), mutation);
        let next_payload = summary_context_to_value(&next);
        assert_eq!(next_payload["chatHistory"], serde_json::json!([]));
        assert_eq!(next_payload["preparedHistory"][0]["content"], "rewritten");
        assert_eq!(next_payload["summaryPrompt"], "");
        let final_mutation = parse_summary_hook_result(&next.stage, Some(serde_json::json!({"metadata":{"second":true}}))).unwrap().unwrap();
        let final_context = applySummaryHookMutation(next, final_mutation);
        assert_eq!(final_context.prepared_history[0].metadata["source"], "plugin");
        assert_eq!(final_context.metadata.len(), 2);
        assert_eq!(original.chat_history[0].content, "original");
    }

    /// Maps strings to summary prompts before generation and to summary output afterwards.
    #[test]
    fn string_results_follow_the_summary_stage() {
        for stage in ["before_prepare_summary_prompt", "before_send_to_model"] {
            let mutation = parse_summary_hook_result(stage, Some(Value::String(String::new()))).unwrap().unwrap();
            assert_eq!(mutation.summary_prompt.as_deref(), Some(""));
            assert!(mutation.summary_result.is_none());
        }
        let mutation = parse_summary_hook_result("after_generate_summary", Some(Value::String("edited summary".into()))).unwrap().unwrap();
        assert_eq!(mutation.summary_result.as_deref(), Some("edited summary"));
        assert!(parse_summary_hook_result("unknown", Some(Value::String("text".into()))).is_err());
    }

    /// Rejects malformed history contracts and preserves explicit nonparticipation.
    #[test]
    fn invalid_summary_results_fail_instead_of_disappearing() {
        for value in [serde_json::json!(true), serde_json::json!([]),
            serde_json::json!({"chatHistory":"wrong"}),
            serde_json::json!({"preparedHistory":[{"kind":"invalid", "content":"text"}]})] {
            assert!(parse_summary_hook_result("before_send_to_model", Some(value)).is_err());
        }
        for value in [None, Some(Value::Null), Some(serde_json::json!({}))] {
            assert!(parse_summary_hook_result("before_send_to_model", value).unwrap().is_none());
        }
    }
}
