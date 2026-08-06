use std::sync::{Mutex, OnceLock};

use serde_json::Value;

use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use crate::plugins::toolpkg::ToolPkgPreHookTimeout::ToolPkgPreHookTimeout;
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_CHAT_INPUT;
use operit_plugin_sdk::toolpkg::ToolPkgHooks::{
    decodeToolPkgHookResult, ToolPkgChatInputHookRegistration,
};
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;
use operit_util::ChainLogger::{self, PLUGIN_CHAIN};

static CHAT_INPUT_HOOKS: OnceLock<Mutex<Vec<ToolPkgChatInputHookRegistration>>> = OnceLock::new();
static CHAT_INPUT_RUNTIME: OnceLock<ToolPkgBridgeRuntime> = OnceLock::new();

pub const CHAT_INPUT_EVENT_INPUT_CHANGED: &str = "input_changed";
pub const CHAT_INPUT_EVENT_SUBMIT_REQUESTED: &str = "submit_requested";
pub const CHAT_INPUT_EVENT_SUBMITTED: &str = "submitted";
pub const CHAT_INPUT_SUBMIT_ACTION_ALLOW: &str = "allow";
pub const CHAT_INPUT_SUBMIT_ACTION_BLOCK: &str = "block";
pub const CHAT_INPUT_SUBMIT_ACTION_CONSUME: &str = "consume";
pub const CHAT_INPUT_SUBMIT_ACTION_REPLACE: &str = "replace";
const TOOLPKG_PRE_HOOK_TIMEOUT_MESSAGE: &str = "ToolPkg 前置 Hook 执行超时，已跳过未完成的 Hook。";

#[derive(Clone, Debug)]
pub struct ChatInputHookContext {
    pub chatId: String,
    pub text: String,
    pub selectionStart: i32,
    pub selectionEnd: i32,
    pub hasAttachments: bool,
    pub attachmentCount: i32,
    pub isProcessing: bool,
    pub inputStyle: String,
    pub source: String,
    pub submitSource: String,
    pub eventName: String,
}

#[derive(Clone, Debug)]
pub struct ChatInputHookResult {
    pub action: String,
    pub text: Option<String>,
    pub message: Option<String>,
    pub clearInput: bool,
    pub timedOut: bool,
    pub metadata: serde_json::Map<String, Value>,
}

pub struct ToolPkgChatInputHookBridge;

impl ToolPkgChatInputHookBridge {
    /// Registers chat input hooks for one application runtime.
    pub fn register(runtime: ToolPkgBridgeRuntime) {
        CHAT_INPUT_RUNTIME.get_or_init(|| runtime.clone());
        let manager = runtime.package_manager();
        manager.addToolPkgRuntimeChangeListener(std::sync::Arc::new(|activeContainers| {
            ToolPkgChatInputHookBridge::syncToolPkgRegistrations(activeContainers);
        }));
    }

    #[allow(non_snake_case)]
    pub fn syncToolPkgRegistrations(activeContainers: Vec<ToolPkgContainerRuntime>) {
        let mut hooks = activeContainers
            .iter()
            .flat_map(|runtime| {
                runtime
                    .chatInputHooks
                    .iter()
                    .map(|hook| ToolPkgChatInputHookRegistration {
                        containerPackageName: runtime.packageName.clone(),
                        hookId: hook.id.clone(),
                        functionName: hook.function.clone(),
                        functionSource: hook.functionSource.clone(),
                    })
            })
            .collect::<Vec<_>>();
        hooks.sort_by(|left, right| {
            left.containerPackageName
                .cmp(&right.containerPackageName)
                .then(left.hookId.cmp(&right.hookId))
        });
        *CHAT_INPUT_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg chat input hook mutex poisoned") = hooks;
    }

    #[allow(non_snake_case)]
    pub fn dispatchChatInputHooks(
        runtime: &ToolPkgBridgeRuntime,
        context: ChatInputHookContext,
    ) -> Option<ChatInputHookResult> {
        let activeHooks = CHAT_INPUT_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg chat input hook mutex poisoned")
            .clone();
        if activeHooks.is_empty() {
            return None;
        }

        let mut current = context;
        ChainLogger::info(
            PLUGIN_CHAIN,
            "plugin.toolpkg.chat_input.scan",
            &[
                ("event", current.eventName.clone()),
                ("chatId", current.chatId.clone()),
                ("hookCount", activeHooks.len().to_string()),
                ("textChars", ChainLogger::lenField(&current.text)),
            ],
        );
        let manager = runtime.package_manager();
        let budget = ToolPkgPreHookTimeout::fromPreferences();
        let mut timedOut = false;
        for hook in activeHooks {
            let Some(timeoutMillis) = budget.remainingTimeoutMillis() else {
                timedOut = true;
                ChainLogger::error(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.chat_input.timeout",
                    &[
                        ("event", current.eventName.clone()),
                        ("stage", "before_hook".to_string()),
                    ],
                );
                break;
            };
            ChainLogger::info(
                PLUGIN_CHAIN,
                "plugin.toolpkg.chat_input.run.start",
                &[
                    ("event", current.eventName.clone()),
                    ("package", hook.containerPackageName.clone()),
                    ("hookId", hook.hookId.clone()),
                    ("function", hook.functionName.clone()),
                ],
            );
            let raw = manager.runToolPkgMainHookWithTimeoutMillis(
                &hook.containerPackageName,
                &hook.functionName,
                TOOLPKG_EVENT_CHAT_INPUT,
                Some(&current.eventName),
                Some(&hook.hookId),
                hook.functionSource.as_deref(),
                buildChatInputEventPayload(&current),
                None,
                None,
                None,
                timeoutMillis,
            );
            if budget.hasExpired() {
                timedOut = true;
                ChainLogger::error(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.chat_input.timeout",
                    &[
                        ("event", current.eventName.clone()),
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
                        "plugin.toolpkg.chat_input.run.error",
                        &[
                            ("event", current.eventName.clone()),
                            ("package", hook.containerPackageName.clone()),
                            ("hookId", hook.hookId.clone()),
                            ("function", hook.functionName.clone()),
                            ("error", error),
                        ],
                    );
                    None
                }
            };

            let Some(parsed) = parseChatInputHookResult(result.as_ref()) else {
                ChainLogger::info(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.chat_input.run.done",
                    &[
                        ("event", current.eventName.clone()),
                        ("package", hook.containerPackageName.clone()),
                        ("hookId", hook.hookId.clone()),
                        ("matched", ChainLogger::boolField(false)),
                    ],
                );
                continue;
            };
            if current.eventName != CHAT_INPUT_EVENT_SUBMIT_REQUESTED {
                continue;
            }
            match parsed.action.as_str() {
                CHAT_INPUT_SUBMIT_ACTION_BLOCK | CHAT_INPUT_SUBMIT_ACTION_CONSUME => {
                    ChainLogger::info(
                        PLUGIN_CHAIN,
                        "plugin.toolpkg.chat_input.run.decision",
                        &[
                            ("event", current.eventName.clone()),
                            ("package", hook.containerPackageName.clone()),
                            ("hookId", hook.hookId.clone()),
                            ("action", parsed.action.clone()),
                        ],
                    );
                    return Some(parsed);
                }
                CHAT_INPUT_SUBMIT_ACTION_REPLACE => {
                    let replacement = parsed
                        .text
                        .clone()
                        .expect("ToolPkg chat input replace action must include text");
                    current.text = replacement;
                    current.selectionStart = current.text.len() as i32;
                    current.selectionEnd = current.text.len() as i32;
                    ChainLogger::info(
                        PLUGIN_CHAIN,
                        "plugin.toolpkg.chat_input.run.decision",
                        &[
                            ("event", current.eventName.clone()),
                            ("package", hook.containerPackageName.clone()),
                            ("hookId", hook.hookId.clone()),
                            ("action", parsed.action.clone()),
                            ("textChars", ChainLogger::lenField(&current.text)),
                        ],
                    );
                }
                _ => {}
            }
        }

        if current.eventName == CHAT_INPUT_EVENT_SUBMIT_REQUESTED {
            Some(ChatInputHookResult {
                action: CHAT_INPUT_SUBMIT_ACTION_ALLOW.to_string(),
                text: Some(current.text),
                message: timedOut.then(|| TOOLPKG_PRE_HOOK_TIMEOUT_MESSAGE.to_string()),
                clearInput: false,
                timedOut,
                metadata: serde_json::Map::new(),
            })
        } else {
            None
        }
    }

    /// Dispatches chat input hooks through the runtime registered by the common bridge.
    #[allow(non_snake_case)]
    pub fn dispatchRegisteredChatInputHooks(
        context: ChatInputHookContext,
    ) -> Option<ChatInputHookResult> {
        let runtime = CHAT_INPUT_RUNTIME.get()?;
        Self::dispatchChatInputHooks(runtime, context)
    }
}

#[allow(non_snake_case)]
fn buildChatInputEventPayload(context: &ChatInputHookContext) -> Value {
    serde_json::json!({
        "chatId": context.chatId,
        "text": context.text,
        "selectionStart": context.selectionStart,
        "selectionEnd": context.selectionEnd,
        "hasAttachments": context.hasAttachments,
        "attachmentCount": context.attachmentCount,
        "isProcessing": context.isProcessing,
        "inputStyle": context.inputStyle,
        "source": context.source,
        "submitSource": context.submitSource,
    })
}

#[allow(non_snake_case)]
fn parseChatInputHookResult(decoded: Option<&Value>) -> Option<ChatInputHookResult> {
    match decoded? {
        Value::String(value) => {
            if value.trim().is_empty() {
                None
            } else {
                Some(ChatInputHookResult {
                    action: CHAT_INPUT_SUBMIT_ACTION_REPLACE.to_string(),
                    text: Some(value.clone()),
                    message: None,
                    clearInput: false,
                    timedOut: false,
                    metadata: serde_json::Map::new(),
                })
            }
        }
        Value::Object(object) => {
            let actionValue = object
                .get("action")
                .and_then(Value::as_str)
                .map(str::trim)
                .map(str::to_ascii_lowercase);
            let containsText = object.contains_key("text");
            let action = match actionValue.as_deref() {
                Some(CHAT_INPUT_SUBMIT_ACTION_BLOCK) => CHAT_INPUT_SUBMIT_ACTION_BLOCK,
                Some(CHAT_INPUT_SUBMIT_ACTION_CONSUME) => CHAT_INPUT_SUBMIT_ACTION_CONSUME,
                Some(CHAT_INPUT_SUBMIT_ACTION_REPLACE) => CHAT_INPUT_SUBMIT_ACTION_REPLACE,
                Some(CHAT_INPUT_SUBMIT_ACTION_ALLOW) => CHAT_INPUT_SUBMIT_ACTION_ALLOW,
                Some("") | None if containsText => CHAT_INPUT_SUBMIT_ACTION_REPLACE,
                _ => return None,
            };
            let metadata = object
                .get("metadata")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            Some(ChatInputHookResult {
                action: action.to_string(),
                text: object
                    .get("text")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                message: object
                    .get("message")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                clearInput: object
                    .get("clearInput")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                timedOut: false,
                metadata,
            })
        }
        _ => None,
    }
}
