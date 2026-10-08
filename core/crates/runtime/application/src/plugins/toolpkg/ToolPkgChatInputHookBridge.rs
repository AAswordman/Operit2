use std::sync::{Mutex, OnceLock};

use serde::Deserialize;
use serde_json::Value;
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use crate::data::preferences::ApiPreferences::ApiPreferences;

use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use crate::plugins::toolpkg::ToolPkgPreHookTimeout::ToolPkgPreHookTimeout;
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_CHAT_INPUT;
use operit_plugin_sdk::toolpkg::ToolPkgHooks::ToolPkgChatInputHookRegistration;
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;

static CHAT_INPUT_HOOKS: OnceLock<Mutex<Vec<ToolPkgChatInputHookRegistration>>> = OnceLock::new();
static CHAT_INPUT_RUNTIME: OnceLock<ToolPkgBridgeRuntime> = OnceLock::new();

pub const CHAT_INPUT_EVENT_INPUT_CHANGED: &str = "input_changed";
pub const CHAT_INPUT_EVENT_SUBMIT_REQUESTED: &str = "submit_requested";
pub const CHAT_INPUT_EVENT_SUBMITTED: &str = "submitted";
pub const CHAT_INPUT_SUBMIT_ACTION_ALLOW: &str = "Allow";
pub const CHAT_INPUT_SUBMIT_ACTION_BLOCK: &str = "Block";
pub const CHAT_INPUT_SUBMIT_ACTION_CONSUME: &str = "Consume";
pub const CHAT_INPUT_SUBMIT_ACTION_REPLACE: &str = "Replace";

/// Builds the chat-input timeout notice with the exact package and Hook ID.
fn build_chat_input_timeout_message(hook: &ToolPkgChatInputHookRegistration) -> String {
    format!(
        "Chat input hook timed out: {}:{}",
        hook.containerPackageName, hook.hookId
    )
}

#[derive(Clone, Debug)]
pub struct ChatInputHookContext {
    pub chatId: String,
    /// Identifies the actual send-capable runtime slot; detached slots have no current SDK send representation.
    pub runtime: Option<operit_plugin_sdk::js_sdk::chat::ChatRuntime>,
    /// Preserves the originating request's completion-notification policy.
    pub notifyReply: bool,
    pub text: String,
    pub selectionStart: i32,
    pub selectionEnd: i32,
    pub hasAttachments: bool,
    pub attachmentCount: i32,
    /// Contains exact host-provided attachment objects, or null for count-only input notifications.
    pub attachments: Option<Vec<AttachmentInfo>>,
    /// Identifies the real reply target without serializing or guessing a display name.
    pub replyToMessageTimestamp: Option<i64>,
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

    /// Synchronizes actual input hooks whenever the enabled package catalog changes.
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

    /// Executes submit handlers strictly; invocation, timeout and invalid-result errors never authorize sending.
    #[allow(non_snake_case)]
    pub async fn dispatchChatInputHooks(
        runtime: &ToolPkgBridgeRuntime,
        context: ChatInputHookContext,
    ) -> Result<Option<ChatInputHookResult>, String> {
        validateChatInputContext(&context)?;
        let manager = RuntimePackageManager::readySnapshot(runtime.tool_handler().getOrCreatePackageManager()).await?;
        let mut activeHooks = manager.getEnabledToolPkgContainerRuntimes().into_iter()
            .flat_map(|container| container.chatInputHooks.into_iter().map(move |hook| ToolPkgChatInputHookRegistration {
                containerPackageName: container.packageName.clone(), hookId: hook.id,
                functionName: hook.function, functionSource: hook.functionSource,
            }))
            .collect::<Vec<_>>();
        activeHooks.sort_by(|left, right| left.containerPackageName.cmp(&right.containerPackageName).then(left.hookId.cmp(&right.hookId)));
        if activeHooks.is_empty() {
            return Ok(None);
        }
        let seconds = ApiPreferences::getInstance().getToolPkgPreHookTimeoutSeconds()
            .map_err(|error| format!("Chat input timeout preference read failed: {error}"))?;
        let budget = ToolPkgPreHookTimeout::fromSeconds(seconds);
        let mut current = context;
        let mut finalDecision = None;
        for hook in activeHooks {
            let timeoutMillis = budget.remainingTimeoutMillis()
                .ok_or_else(|| build_chat_input_timeout_message(&hook))?;
            let raw = manager.runToolPkgMainHookWithTimeoutMillis(
                &hook.containerPackageName, &hook.functionName, TOOLPKG_EVENT_CHAT_INPUT,
                Some(&current.eventName), Some(&hook.hookId), hook.functionSource.as_deref(),
                buildChatInputEventPayload(&current), None, None, None, timeoutMillis,
            ).await.map_err(|error| format!("Chat input {} hook failed for {}:{}: {error}",
                current.eventName, hook.containerPackageName, hook.hookId))?;
            if budget.hasExpired() {
                return Err(build_chat_input_timeout_message(&hook));
            }
            let parsed = parseChatInputHookResult(raw)
                .map_err(|error| format!("Chat input {} invalid result for {}:{}: {error}",
                    current.eventName, hook.containerPackageName, hook.hookId))?;
            let Some(mut parsed) = parsed else { continue; };
            if current.eventName != CHAT_INPUT_EVENT_SUBMIT_REQUESTED {
                continue;
            }
            match parsed.action.as_str() {
                CHAT_INPUT_SUBMIT_ACTION_BLOCK | CHAT_INPUT_SUBMIT_ACTION_CONSUME => return Ok(Some(parsed)),
                CHAT_INPUT_SUBMIT_ACTION_REPLACE | CHAT_INPUT_SUBMIT_ACTION_ALLOW => {
                    if let Some(replacement) = &parsed.text {
                        current.text = replacement.clone();
                        current.selectionStart = current.text.chars().count() as i32;
                        current.selectionEnd = current.selectionStart;
                    }
                    parsed.text = Some(current.text.clone());
                    finalDecision = Some(parsed);
                }
                _ => return Err("Chat input returned an unvalidated action".to_string()),
            }
        }
        Ok(finalDecision)
    }

    /// Requires the process-owned runtime instead of treating an unregistered submit bridge as approval.
    #[allow(non_snake_case)]
    pub async fn dispatchRegisteredChatInputHooks(
        context: ChatInputHookContext,
    ) -> Result<Option<ChatInputHookResult>, String> {
        let runtime = CHAT_INPUT_RUNTIME.get()
            .ok_or_else(|| "Chat input hook runtime is not registered".to_string())?;
        Self::dispatchChatInputHooks(runtime, context).await
    }

}

/// Emits the real attachments and reply target separately from count-only input notification data.
#[allow(non_snake_case)]
fn buildChatInputEventPayload(context: &ChatInputHookContext) -> Value {
    serde_json::json!({
        "chatId": context.chatId,
        "runtime": context.runtime,
        "notifyReply": context.notifyReply,
        "text": context.text,
        "selectionStart": context.selectionStart,
        "selectionEnd": context.selectionEnd,
        "hasAttachments": context.hasAttachments,
        "attachmentCount": context.attachmentCount,
        "attachments": context.attachments,
        "replyToMessageTimestamp": context.replyToMessageTimestamp,
        "isProcessing": context.isProcessing,
        "inputStyle": context.inputStyle,
        "source": context.source,
        "submitSource": context.submitSource,
    })
}

/// Rejects count mismatches and missing actual attachments on the authoritative submit path.
#[allow(non_snake_case)]
fn validateChatInputContext(context: &ChatInputHookContext) -> Result<(), String> {
    if context.chatId.trim().is_empty() { return Err("Chat input requires an existing chatId".to_string()); }
    if context.attachmentCount < 0 || context.hasAttachments != (context.attachmentCount > 0) {
        return Err("Chat input attachment flags do not match the supplied count".to_string());
    }
    if let Some(attachments) = &context.attachments {
        if attachments.len() as i32 != context.attachmentCount {
            return Err("Chat input attachment count differs from actual attachment records".to_string());
        }
    } else if context.eventName == CHAT_INPUT_EVENT_SUBMIT_REQUESTED || context.eventName == CHAT_INPUT_EVENT_SUBMITTED {
        return Err("Chat submit requires complete attachment records".to_string());
    }
    Ok(())
}

/// Decodes declared optional controls without inferring actions from unknown strings or invalid JSON.
#[derive(Deserialize)]
#[allow(non_snake_case)]
struct ChatInputObjectDecision {
    action: Option<String>,
    text: Option<String>,
    message: Option<String>,
    clearInput: Option<bool>,
    metadata: Option<serde_json::Map<String, Value>>,
}

/// Accepts only protocol null/void as no contribution and rejects every malformed declared decision.
#[allow(non_snake_case)]
fn parseChatInputHookResult(raw: Option<String>) -> Result<Option<ChatInputHookResult>, String> {
    let Some(raw) = raw else { return Ok(None); };
    let value: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("Chat input result must be valid JSON: {error}"))?;
    let decision = match value {
        Value::Null => return Ok(None),
        Value::String(text) => ChatInputObjectDecision {
            action: Some(CHAT_INPUT_SUBMIT_ACTION_REPLACE.to_string()), text: Some(text),
            message: None, clearInput: None, metadata: None,
        },
        Value::Object(object) => serde_json::from_value::<ChatInputObjectDecision>(Value::Object(object))
            .map_err(|error| format!("Invalid chat input result fields: {error}"))?,
        _ => return Err("Chat input result must be an object, string, null or void".to_string()),
    };
    let action = match (decision.action.as_deref(), decision.text.as_ref()) {
        (Some(CHAT_INPUT_SUBMIT_ACTION_ALLOW), _) => CHAT_INPUT_SUBMIT_ACTION_ALLOW,
        (Some(CHAT_INPUT_SUBMIT_ACTION_BLOCK), _) => CHAT_INPUT_SUBMIT_ACTION_BLOCK,
        (Some(CHAT_INPUT_SUBMIT_ACTION_CONSUME), _) => CHAT_INPUT_SUBMIT_ACTION_CONSUME,
        (Some(CHAT_INPUT_SUBMIT_ACTION_REPLACE), Some(_)) | (None, Some(_)) => CHAT_INPUT_SUBMIT_ACTION_REPLACE,
        (Some(CHAT_INPUT_SUBMIT_ACTION_REPLACE), None) => return Err("Chat input replace requires text".to_string()),
        (Some(action), _) => return Err(format!("Unknown chat input action: {action}")),
        (None, None) => return Err("Chat input object requires a declared action or replacement text".to_string()),
    };
    Ok(Some(ChatInputHookResult {
        action: action.to_string(), text: decision.text, message: decision.message,
        clearInput: match decision.clearInput { Some(value) => value, None => false },
        timedOut: false,
        metadata: match decision.metadata { Some(value) => value, None => serde_json::Map::new() },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Supplies actual attachment content and a real reply identity to production payload construction.
    fn submit() -> ChatInputHookContext {
        let mut attachment = AttachmentInfo::new("runtime/files/photo.png".into(), "photo.png".into(), "image/png".into(), 73);
        attachment.nodeId = Some("node-origin".into());
        attachment.content = "opaque-inline-content".into();
        ChatInputHookContext {
            chatId: "real-chat".into(), runtime: Some(operit_plugin_sdk::js_sdk::chat::ChatRuntime::Main),
            notifyReply: true, text: "original text".into(), selectionStart: 2, selectionEnd: 7,
            hasAttachments: true, attachmentCount: 1, attachments: Some(vec![attachment]),
            replyToMessageTimestamp: Some(42), isProcessing: false, inputStyle: "Runtime".into(),
            source: "Runtime".into(), submitSource: "Send".into(), eventName: CHAT_INPUT_EVENT_SUBMIT_REQUESTED.into(),
        }
    }

    /// Keeps every attachment field and the exact reply target instead of substituting an empty list for counts.
    #[test]
    fn submit_payload_preserves_complete_original_data() {
        let context = submit();
        validateChatInputContext(&context).unwrap();
        let payload = buildChatInputEventPayload(&context);
        assert_eq!(payload["attachments"], serde_json::to_value(&context.attachments).unwrap());
        assert_eq!(payload["attachments"][0]["nodeId"], "node-origin");
        assert_eq!(payload["attachments"][0]["content"], "opaque-inline-content");
        assert_eq!(payload["runtime"], "main");
        assert_eq!(payload["notifyReply"], true);
        assert_eq!(payload["replyToMessageTimestamp"], 42);
        assert_eq!(payload["selectionStart"], 2);
    }

    /// Requires actual attachment objects for submit while keeping unavailable notification records explicitly null.
    #[test]
    fn count_only_notification_is_not_a_complete_submit() {
        let mut context = submit();
        context.attachments = None;
        assert!(validateChatInputContext(&context).is_err());
        context.eventName = CHAT_INPUT_EVENT_INPUT_CHANGED.into();
        validateChatInputContext(&context).unwrap();
        assert_eq!(buildChatInputEventPayload(&context)["attachments"], Value::Null);
        context.attachments = Some(Vec::new());
        assert!(validateChatInputContext(&context).is_err());
    }

    /// Accepts exactly the public SDK action spelling and rejects lowercase protocol changes.
    #[test]
    fn chat_input_actions_match_public_sdk_contract() {
        use operit_plugin_sdk::js_sdk::toolpkg::ToolPkgChatInputHookObjectResultAction;

        for (action, expected) in [
            (ToolPkgChatInputHookObjectResultAction::Allow, CHAT_INPUT_SUBMIT_ACTION_ALLOW),
            (ToolPkgChatInputHookObjectResultAction::Block, CHAT_INPUT_SUBMIT_ACTION_BLOCK),
            (ToolPkgChatInputHookObjectResultAction::Replace, CHAT_INPUT_SUBMIT_ACTION_REPLACE),
            (ToolPkgChatInputHookObjectResultAction::Consume, CHAT_INPUT_SUBMIT_ACTION_CONSUME),
        ] {
            let raw = serde_json::json!({"action": action, "text": "submitted text"}).to_string();
            assert_eq!(serde_json::to_value(action).unwrap(), Value::String(expected.to_string()));
            assert_eq!(parseChatInputHookResult(Some(raw)).unwrap().unwrap().action, expected);
            let lowercase = serde_json::json!({"action": expected.to_lowercase(), "text": "submitted text"}).to_string();
            assert!(parseChatInputHookResult(Some(lowercase)).is_err());
        }
    }

    /// Distinguishes legal non-contributions from malformed decisions without authorizing failed planners.
    #[test]
    fn invalid_results_never_become_allow_or_none() {
        assert!(parseChatInputHookResult(None).unwrap().is_none());
        assert!(parseChatInputHookResult(Some("null".into())).unwrap().is_none());
        for raw in ["", "undefined", "unquoted text", "false", "42", "[]", "{}", "{\"action\":\"unknown\"}", "{\"action\":\"Replace\"}", "{\"action\":42}", "{\"action\":\"Consume\",\"clearInput\":42}", "{\"action\":\"Allow\",\"metadata\":[]}"] {
            assert!(parseChatInputHookResult(Some(raw.into())).is_err(), "{raw}");
        }
        assert_eq!(parseChatInputHookResult(Some("{\"action\":\"Consume\",\"clearInput\":true}".into())).unwrap().unwrap().action, CHAT_INPUT_SUBMIT_ACTION_CONSUME);
        assert_eq!(parseChatInputHookResult(Some("{\"text\":\"replacement\"}".into())).unwrap().unwrap().action, CHAT_INPUT_SUBMIT_ACTION_REPLACE);
    }
}
