use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use serde_json::Value;

use crate::core::chat::plugins::MessageProcessingPluginRegistry::{
    MessageProcessingController, MessageProcessingExecution, MessageProcessingHookParams,
    MessageProcessingPlugin, MessageProcessingPluginRegistry,
};
use crate::core::tools::packTool::PackageManager::PackageManager;
use crate::core::tools::packTool::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_MESSAGE_PROCESSING;
use crate::core::tools::packTool::ToolPkgParser::ToolPkgContainerRuntime;
use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::{
    decodeToolPkgHookResult, toolPkgPackageManager, ToolPkgMessageProcessingHookRegistration,
};
use crate::util::stream::HotStream::MutableSharedStreamImpl;
use crate::util::ChainLogger::{self, PLUGIN_CHAIN};

static MESSAGE_PROCESSING_HOOKS: OnceLock<Mutex<Vec<ToolPkgMessageProcessingHookRegistration>>> =
    OnceLock::new();

pub struct ToolPkgMessageProcessingBridge;

impl ToolPkgMessageProcessingBridge {
    pub fn register() {
        MessageProcessingPluginRegistry::register(Arc::new(MessageProcessingBridge));
    }

    #[allow(non_snake_case)]
    pub fn syncToolPkgRegistrations(activeContainers: Vec<ToolPkgContainerRuntime>) {
        let mut hooks = activeContainers
            .iter()
            .flat_map(|container| {
                container.messageProcessingPlugins.iter().map(|hook| {
                    ToolPkgMessageProcessingHookRegistration {
                        containerPackageName: container.packageName.clone(),
                        pluginId: hook.id.clone(),
                        functionName: hook.function.clone(),
                        functionSource: hook.functionSource.clone(),
                    }
                })
            })
            .collect::<Vec<_>>();
        hooks.sort_by(|left, right| {
            left.containerPackageName
                .cmp(&right.containerPackageName)
                .then(left.pluginId.cmp(&right.pluginId))
        });
        let hookCount = hooks.len();
        *MESSAGE_PROCESSING_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg message processing hook mutex poisoned") = hooks;
        ChainLogger::info(
            PLUGIN_CHAIN,
            "plugin.toolpkg.message_processing.sync",
            &[("hookCount", hookCount.to_string())],
        );
    }
}

struct MessageProcessingBridge;

impl MessageProcessingPlugin for MessageProcessingBridge {
    fn id(&self) -> &str {
        "builtin.toolpkg.message-processing-bridge"
    }

    #[allow(non_snake_case)]
    fn createExecutionIfMatched(
        &self,
        params: &MessageProcessingHookParams,
    ) -> Option<MessageProcessingExecution<Box<dyn MessageProcessingController + Send + Sync>>>
    {
        let hooks = MESSAGE_PROCESSING_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg message processing hook mutex poisoned")
            .clone();
        ChainLogger::info(
            PLUGIN_CHAIN,
            "plugin.toolpkg.message_processing.probe.scan",
            &[
                ("hookCount", hooks.len().to_string()),
                (
                    "messageChars",
                    ChainLogger::lenField(&params.message_content),
                ),
                ("historyCount", params.chat_history.len().to_string()),
            ],
        );
        let probeEventPayload = buildMessageEventPayload(params, true);
        let manager = toolPkgPackageManager();
        for hook in hooks {
            ChainLogger::info(
                PLUGIN_CHAIN,
                "plugin.toolpkg.message_processing.probe.start",
                &[
                    ("package", hook.containerPackageName.clone()),
                    ("hookId", hook.pluginId.clone()),
                    ("function", hook.functionName.clone()),
                ],
            );
            let probeDecoded =
                runMessageProcessingHook(&manager, &hook, probeEventPayload.clone(), None);
            let probeResult = parseMessageProcessingResult(probeDecoded.as_ref());
            let Some(probeResult) = probeResult else {
                continue;
            };
            if !probeResult.matched {
                continue;
            }
            ChainLogger::info(
                PLUGIN_CHAIN,
                "plugin.toolpkg.message_processing.probe.matched",
                &[
                    ("package", hook.containerPackageName.clone()),
                    ("hookId", hook.pluginId.clone()),
                ],
            );

            let executionId = format!(
                "toolpkg-msg:{}:{}:{}",
                hook.containerPackageName,
                hook.pluginId,
                operit_host_api::TimeUtils::currentTimeMillis()
            );
            let mut eventPayload = buildMessageEventPayload(params, false);
            if let Value::Object(object) = &mut eventPayload {
                object.insert(
                    "executionId".to_string(),
                    Value::String(executionId.clone()),
                );
            }
            let stream = MutableSharedStreamImpl::new(usize::MAX);
            let stream_for_intermediate = stream.clone();
            let stream_for_final = stream.clone();
            let hook_for_worker = hook.clone();
            let manager_for_worker = manager.clone();
            let executionIdForWorker = executionId.clone();
            thread::spawn(move || {
                ChainLogger::info(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.message_processing.run.start",
                    &[
                        ("package", hook_for_worker.containerPackageName.clone()),
                        ("hookId", hook_for_worker.pluginId.clone()),
                        ("executionId", executionIdForWorker.clone()),
                    ],
                );
                let emittedAny = Arc::new(AtomicBool::new(false));
                let emittedAnyForIntermediate = emittedAny.clone();
                let decoded = runMessageProcessingHook(
                    &manager_for_worker,
                    &hook_for_worker,
                    eventPayload,
                    Some(Arc::new(move |raw| {
                        let decoded = decodeToolPkgHookResult(Some(raw));
                        for chunk in extractMessageChunks(decoded.as_ref()) {
                            if !chunk.is_empty() {
                                emittedAnyForIntermediate.store(true, Ordering::Relaxed);
                                stream_for_intermediate.emit(chunk);
                            }
                        }
                    })),
                );
                let parsed = parseMessageProcessingResult(decoded.as_ref());
                let mut emittedChunkCount = 0usize;
                if let Some(parsed) = parsed {
                    if parsed.matched && !emittedAny.load(Ordering::Relaxed) {
                        for chunk in parsed.chunks {
                            if !chunk.is_empty() {
                                emittedChunkCount += 1;
                                stream_for_final.emit(chunk);
                            }
                        }
                    }
                }
                ChainLogger::info(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.message_processing.run.done",
                    &[
                        ("package", hook_for_worker.containerPackageName.clone()),
                        ("hookId", hook_for_worker.pluginId.clone()),
                        ("executionId", executionIdForWorker.clone()),
                        ("chunkCount", emittedChunkCount.to_string()),
                    ],
                );
                stream_for_final.close();
            });
            return Some(MessageProcessingExecution {
                controller: Box::new(RegisteredMessageProcessingController { executionId, hook }),
                stream,
            });
        }
        None
    }
}

#[allow(non_snake_case)]
fn buildMessageEventPayload(params: &MessageProcessingHookParams, probeOnly: bool) -> Value {
    let chatHistory = params
        .chat_history
        .iter()
        .map(|turn| {
            serde_json::json!({
                "kind": turn.kind,
                "content": turn.content,
                "toolName": turn.tool_name,
                "metadata": turn.metadata,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "chatId": params.chat_id,
        "messageContent": params.message_content,
        "chatHistory": chatHistory,
        "workspacePath": params.workspace_path,
        "maxTokens": params.max_tokens,
        "tokenUsageThreshold": params.token_usage_threshold,
        "probeOnly": probeOnly,
    })
}

#[allow(non_snake_case)]
fn runMessageProcessingHook(
    manager: &PackageManager,
    hook: &ToolPkgMessageProcessingHookRegistration,
    eventPayload: Value,
    onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
) -> Option<Value> {
    match manager.runToolPkgMainHook(
        &hook.containerPackageName,
        &hook.functionName,
        TOOLPKG_EVENT_MESSAGE_PROCESSING,
        None,
        Some(&hook.pluginId),
        hook.functionSource.as_deref(),
        eventPayload,
        None,
        None,
        onIntermediateResult,
    ) {
        Ok(raw) => decodeToolPkgHookResult(raw),
        Err(error) => {
            ChainLogger::error(
                PLUGIN_CHAIN,
                "plugin.toolpkg.message_processing.run.error",
                &[
                    ("package", hook.containerPackageName.clone()),
                    ("hookId", hook.pluginId.clone()),
                    ("function", hook.functionName.clone()),
                    ("error", error),
                ],
            );
            None
        }
    }
}

struct ParsedMessageProcessingResult {
    matched: bool,
    chunks: Vec<String>,
}

#[allow(non_snake_case)]
fn parseMessageProcessingResult(decoded: Option<&Value>) -> Option<ParsedMessageProcessingResult> {
    match decoded? {
        Value::Bool(value) => {
            if *value {
                Some(ParsedMessageProcessingResult {
                    matched: true,
                    chunks: Vec::new(),
                })
            } else {
                None
            }
        }
        Value::String(value) => {
            if value.is_empty() {
                None
            } else {
                Some(ParsedMessageProcessingResult {
                    matched: true,
                    chunks: vec![value.clone()],
                })
            }
        }
        Value::Object(object) => {
            let matched = object
                .get("matched")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            if !matched {
                return None;
            }
            Some(ParsedMessageProcessingResult {
                matched: true,
                chunks: extractMessageChunks(decoded),
            })
        }
        _ => None,
    }
}

#[allow(non_snake_case)]
fn extractMessageChunks(decoded: Option<&Value>) -> Vec<String> {
    let Some(decoded) = decoded else {
        return Vec::new();
    };
    match decoded {
        Value::String(value) => {
            if value.is_empty() {
                Vec::new()
            } else {
                vec![value.clone()]
            }
        }
        Value::Object(object) => {
            let mut chunks = Vec::new();
            if let Some(value) = object.get("chunk").and_then(Value::as_str) {
                if !value.is_empty() {
                    chunks.push(value.to_string());
                }
            }
            if let Some(Value::Array(values)) = object.get("chunks") {
                for value in values {
                    if let Some(chunk) = value.as_str() {
                        if !chunk.is_empty() {
                            chunks.push(chunk.to_string());
                        }
                    }
                }
            }
            if let Some(value) = object.get("text").and_then(Value::as_str) {
                if !value.is_empty() {
                    chunks.push(value.to_string());
                }
            } else if let Some(value) = object.get("content").and_then(Value::as_str) {
                if !value.is_empty() {
                    chunks.push(value.to_string());
                }
            }
            chunks
        }
        _ => Vec::new(),
    }
}

struct RegisteredMessageProcessingController {
    executionId: String,
    hook: ToolPkgMessageProcessingHookRegistration,
}

impl MessageProcessingController for RegisteredMessageProcessingController {
    fn cancel(&self) {
        let _ = (&self.executionId, &self.hook);
    }
}
