use std::sync::{Arc, Mutex, OnceLock};
use std::collections::VecDeque;

use operit_model::ChatMessage::ChatMessage;
use operit_model::MessagePart::MessagePartKind;
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::{TOOLPKG_EVENT_MESSAGE_TRANSFORM, TOOLPKG_EVENT_MESSAGE_STREAM_TRANSFORM};
use operit_plugin_sdk::toolpkg::ToolPkgHooks::ToolPkgChatMessageHookRegistration;
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;
use operit_store::PreferencesDataStore::{MutableStateFlow, StateFlow};
use serde_json::{json, Value};
use serde::Serialize;

use super::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use super::ToolPkgPreHookTimeout::ToolPkgPreHookTimeout;
use super::NativeTextTransform::{CompiledTextTransformProgram, TextTransformBatch};

static RUNTIME: OnceLock<ToolPkgBridgeRuntime> = OnceLock::new();
static HOOKS: OnceLock<Mutex<Vec<ToolPkgChatMessageHookRegistration>>> = OnceLock::new();
static STREAM_HOOKS: OnceLock<Mutex<Vec<ToolPkgChatMessageHookRegistration>>> = OnceLock::new();
static REVISION: OnceLock<MutableStateFlow<i64>> = OnceLock::new();
static PREPARED: OnceLock<Mutex<PreparedDisplayCache>> = OnceLock::new();
static FILE_CHANGES: OnceLock<Vec<operit_vfs::FileChanges::FileChangeSubscription>> = OnceLock::new();

/// Compares hook-visible metadata directly; JSON is constructed only on preparation misses.
#[derive(PartialEq, Serialize)]
struct DisplayContextKey {
    chatId: String,
    messages: Vec<DisplayContext>,
}

#[derive(PartialEq, Serialize)]
struct DisplayContext {
    timestamp: i64,
    sender: String,
    participantId: Option<Value>,
    depth: i32,
    isEdit: bool,
}

/// Bounds immutable preparation reuse to sixteen exact context windows until plugin data changes.
#[derive(Default)]
struct PreparedDisplayCache {
    revision: u64,
    windows: VecDeque<(DisplayContextKey, Vec<CompiledTextTransformProgram>)>,
}

impl PreparedDisplayCache {
    /// Retires every prior program before notifying presentation observers.
    fn invalidate(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.windows.clear();
    }

    /// Rejects preparation completed after its configuration revision was superseded.
    fn insert(&mut self, revision: u64, key: DisplayContextKey, programs: Vec<CompiledTextTransformProgram>) {
        if self.revision != revision { return; }
        if self.windows.len() == 16 { self.windows.pop_front(); }
        self.windows.push_back((key, programs));
    }
}

/// Executes ordered, awaited text edits while preserving canonical message structure.
pub struct ToolPkgMessageTransformBridge;

impl ToolPkgMessageTransformBridge {
    /// Binds the transformer to the owning application runtime.
    pub fn register(runtime: ToolPkgBridgeRuntime) {
        FILE_CHANGES.get_or_init(|| {
            let files = runtime.host_manager().vfs().expect("Message transforms require VFS");
            ["device", "space"].into_iter().map(|scope| files.watchChanges(
                &format!("/operitapp/data/extensions/{scope}/plugins"), Arc::new(Self::invalidate),
            ).expect("Plugin data directory must resolve through VFS")).collect()
        });
        RUNTIME.get_or_init(|| runtime);
    }

    /// Replaces the enabled handlers before invalidating existing display projections.
    pub fn syncToolPkgRegistrations(containers: &[ToolPkgContainerRuntime]) {
        let hooks = containers.iter().flat_map(|container| {
            container.messageTransformHooks.iter().map(|hook| ToolPkgChatMessageHookRegistration {
                containerPackageName: container.packageName.clone(), hookId: hook.id.clone(),
                functionName: hook.function.clone(), functionSource: hook.functionSource.clone(),
            })
        }).collect();
        *HOOKS.get_or_init(|| Mutex::new(Vec::new())).lock().expect("message transform hooks mutex poisoned") = hooks;
        let hooks = containers.iter().flat_map(|container| {
            container.messageStreamTransforms.iter().map(|hook| ToolPkgChatMessageHookRegistration {
                containerPackageName: container.packageName.clone(), hookId: hook.id.clone(),
                functionName: hook.function.clone(), functionSource: hook.functionSource.clone(),
            })
        }).collect();
        *STREAM_HOOKS.get_or_init(|| Mutex::new(Vec::new())).lock().expect("native transform hooks mutex poisoned") = hooks;
        Self::invalidate();
    }

    /// Invalidates presentation copies after plugin settings or registrations change.
    pub fn invalidate() {
        PREPARED.get_or_init(Default::default).lock().expect("display preparation cache mutex poisoned").invalidate();
        REVISION.get_or_init(|| MutableStateFlow::new(0)).update(|revision| *revision += 1);
    }

    /// Observes presentation invalidations without modifying stored messages.
    pub fn revisionFlow() -> StateFlow<i64> {
        REVISION.get_or_init(|| MutableStateFlow::new(0)).asStateFlow()
    }

    /// Reports whether text transformation is enabled for this runtime.
    pub fn hasDisplayHooks() -> bool {
        !STREAM_HOOKS.get_or_init(|| Mutex::new(Vec::new())).lock().expect("native transform hooks mutex poisoned").is_empty()
    }

    /// Prepares a single live message through the same ordered batch contract as history.
    pub async fn prepareDisplay(chatId: &str, message: &ChatMessage, depth: i32, isEdit: bool) -> Result<CompiledTextTransformProgram, String> {
        Ok(Self::prepareDisplays(chatId, std::slice::from_ref(message), &[depth], isEdit).await?.remove(0))
    }

    /// Reads plugin configuration once per window and compiles identical programs only once.
    pub async fn prepareDisplays(chatId: &str, messages: &[ChatMessage], depths: &[i32], isEdit: bool) -> Result<Vec<CompiledTextTransformProgram>, String> {
        if messages.len() != depths.len() { return Err("Display context count does not match messages".into()); }
        let revision = PREPARED.get_or_init(Default::default).lock().expect("display preparation cache mutex poisoned").revision;
        let hooks = STREAM_HOOKS.get_or_init(|| Mutex::new(Vec::new())).lock()
            .expect("native transform hooks mutex poisoned").clone();
        let mut programs = vec![CompiledTextTransformProgram::default(); messages.len()];
        if hooks.is_empty() || messages.is_empty() { return Ok(programs); }
        let runtime = RUNTIME.get().ok_or("Message transform runtime is unavailable")?;
        let configuration = crate::services::ProviderRuntimeSupportService::ChatConfigurationApi::ready(&runtime.tool_handler()).await;
        let key = DisplayContextKey { chatId: chatId.into(), messages: messages.iter().zip(depths).map(|(message, depth)| DisplayContext {
            timestamp: message.timestamp, sender: message.sender.clone(),
            participantId: message.pluginExtensions.get(configuration.extensionOwner())
                .and_then(|value| value.get("participantId")).cloned(),
            depth: *depth, isEdit,
        }).collect() };
        {
            let cache = PREPARED.get().unwrap().lock().expect("display preparation cache mutex poisoned");
            if cache.revision == revision {
                if let Some((_, programs)) = cache.windows.iter().find(|(context, _)| context == &key) {
                    return Ok(programs.clone());
                }
            }
        }
        let manager = runtime.package_manager();
        let payload = serde_json::to_value(&key).map_err(|error| error.to_string())?;
        let budget = ToolPkgPreHookTimeout::fromPreferences()?;
        for hook in hooks {
            let raw = ToolPkgPreHookTimeout::runHook(
                Some(&budget), &manager, &hook.containerPackageName, &hook.functionName,
                TOOLPKG_EVENT_MESSAGE_STREAM_TRANSFORM, "prepare_display", &hook.hookId,
                hook.functionSource.as_deref(), payload.clone(),
            ).await?;
            if let Some(raw) = raw.filter(|raw| !raw.is_empty()) {
                let prepared: Option<TextTransformBatch> = serde_json::from_str(&raw)
                    .map_err(|error| format!("Invalid native transform {}: {error}", hook.hookId))?;
                if let Some(prepared) = prepared {
                    let prepared = CompiledTextTransformProgram::compile_batch(prepared, programs.len())
                        .map_err(|error| format!("Native transform {}: {error}", hook.hookId))?;
                    for (program, prepared) in programs.iter_mut().zip(prepared) {
                        program.extend(prepared);
                    }
                }
            }
        }
        PREPARED.get().unwrap().lock().expect("display preparation cache mutex poisoned")
            .insert(revision, key, programs.clone());
        Ok(programs)
    }

    /// Projects a complete history window without repeating plugin calls for each row.
    pub async fn transformDisplayBatch(chatId: &str, messages: Vec<ChatMessage>, depths: Vec<i32>) -> Result<Vec<ChatMessage>, String> {
        let programs = Self::prepareDisplays(chatId, &messages, &depths, false).await?;
        let mut projected = Vec::with_capacity(messages.len());
        for ((message, depth), program) in messages.into_iter().zip(depths).zip(programs) {
            projected.push(Self::transformPrepared(chatId, message, "before_display", depth, false, &program).await?);
        }
        Ok(projected)
    }

    /// Transforms only Markdown and reasoning text, never tool payloads or message identity.
    pub async fn transform(
        chatId: &str, message: ChatMessage, stage: &str, depth: i32, isEdit: bool,
    ) -> Result<ChatMessage, String> {
        if !matches!(stage, "before_persist" | "before_display") {
            return Err(format!("Unsupported message transform stage: {stage}"));
        }
        let program = if stage == "before_display" {
            Self::prepareDisplay(chatId, &message, depth, isEdit).await?
        } else { CompiledTextTransformProgram::default() };
        Self::transformPrepared(chatId, message, stage, depth, isEdit, &program).await
    }

    /// Reuses one prepared native program across the message and all presentation bubbles.
    async fn transformPrepared(
        chatId: &str, mut message: ChatMessage, stage: &str, depth: i32, isEdit: bool,
        program: &CompiledTextTransformProgram,
    ) -> Result<ChatMessage, String> {
        let hooks = if stage == "before_persist" {
            HOOKS.get_or_init(|| Mutex::new(Vec::new())).lock()
                .expect("message transform hooks mutex poisoned").clone()
        } else { Vec::new() };
        if hooks.is_empty() && program.is_empty() { return Ok(message); }
        for part in &mut message.parts {
            let kind = match part.kind {
                MessagePartKind::Markdown => "markdown", MessagePartKind::Thinking => "thinking", _ => continue,
            };
            if !program.is_empty() { part.content = program.apply(kind, &part.content); }
        }
        if !hooks.is_empty() {
            let runtime = RUNTIME.get().ok_or("Message transform runtime is unavailable")?;
            let manager = runtime.package_manager();
            let budget = ToolPkgPreHookTimeout::fromPreferences()?;
            let configuration = crate::services::ProviderRuntimeSupportService::ChatConfigurationApi::ready(&runtime.tool_handler()).await;
            let participant = message.pluginExtensions.get(configuration.extensionOwner())
                .and_then(|value| value.get("participantId")).cloned();
            let hookList = &hooks;
            let managerRef = &manager;
            let budgetRef = &budget;
            transformMessageText(&mut message, chatId, participant, depth, isEdit, hooks.len(), |index, payload| async move {
                let hook = &hookList[index];
                let raw = ToolPkgPreHookTimeout::runHook(
                    Some(budgetRef), managerRef, &hook.containerPackageName, &hook.functionName,
                    TOOLPKG_EVENT_MESSAGE_TRANSFORM, stage, &hook.hookId,
                    hook.functionSource.as_deref(), payload,
                ).await?;
                decodeReplacement(raw)
            }).await?;
        }
        let presentationKey = operit_model::ChatMessagePresentation::PRESENTATION_EXTENSION;
        if let Some(value) = message.pluginExtensions.remove(presentationKey) {
            let mut presentation: operit_model::ChatMessagePresentation::ChatMessagePresentation =
                serde_json::from_value(value).map_err(|error| error.to_string())?;
            for bubble in &mut presentation.bubbles {
                let mut bubbleMessage = message.clone();
                bubbleMessage.parts = bubble.clone();
                *bubble = Box::pin(Self::transformPrepared(chatId, bubbleMessage, stage, depth, isEdit, program)).await?.parts;
            }
            message.pluginExtensions.insert(presentationKey.into(),
                serde_json::to_value(presentation).map_err(|error| error.to_string())?);
        }
        Ok(message)
    }
}

/// Applies every handler to each text part while preserving tools, attributes, ordering, and identity.
async fn transformMessageText<F, Fut>(
    message: &mut ChatMessage, chatId: &str, participant: Option<Value>, depth: i32, isEdit: bool,
    hookCount: usize, mut run: F,
) -> Result<(), String>
where
    F: FnMut(usize, Value) -> Fut,
    Fut: std::future::Future<Output = Result<Option<String>, String>>,
{
    for part in &mut message.parts {
        let kind = match part.kind {
            MessagePartKind::Markdown => "markdown",
            MessagePartKind::Thinking => "thinking",
            _ => continue,
        };
        for index in 0..hookCount {
            let payload = json!({
                "chatId": chatId, "timestamp": message.timestamp, "sender": message.sender,
                "participantId": participant, "partId": part.partId, "kind": kind,
                "text": part.content, "depth": depth, "isEdit": isEdit,
            });
            if let Some(text) = run(index, payload).await? { part.content = text; }
        }
    }
    Ok(())
}

/// Distinguishes an explicit empty replacement from null or undefined participation.
fn decodeReplacement(raw: Option<String>) -> Result<Option<String>, String> {
    match raw.as_deref() {
        None | Some("") => Ok(None),
        Some(raw) => match serde_json::from_str(raw).map_err(|error| format!("Invalid message transform result: {error}"))? {
            Value::Null => Ok(None),
            Value::String(text) => Ok(Some(text)),
            _ => Err("Message transform hooks must return text, null, or undefined".into()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Invalidated in-flight results cannot repopulate a new revision and retained windows stay bounded.
    #[test]
    fn prepared_display_cache_rejects_stale_results_and_evicts_old_windows() {
        let mut cache = PreparedDisplayCache::default();
        let key = |chatId: &str| DisplayContextKey { chatId: chatId.into(), messages: vec![] };
        cache.insert(0, key("old"), vec![]);
        cache.invalidate();
        cache.insert(0, key("stale"), vec![]);
        assert!(cache.windows.is_empty());
        for index in 0..20 { cache.insert(1, key(&index.to_string()), vec![]); }
        assert_eq!(cache.windows.len(), 16);
        assert_eq!(cache.windows.front().unwrap().0.chatId, "4");
    }

    /// Applies the same native rules to completed parts and presentation bubbles without JavaScript display callbacks.
    #[tokio::test]
    async fn message_transform_native_display_preserves_canonical_structure() {
        use operit_model::MessagePart::MessagePart;
        use super::super::NativeTextTransform::{TextTransformProgram, TextTransformRule};
        let program = CompiledTextTransformProgram::compile(&TextTransformProgram { rules: vec![TextTransformRule {
            pattern: "secret".into(), replacement: "public".into(), flags: "g".into(),
            kinds: vec!["markdown".into(), "thinking".into()], trimStrings: vec![],
        }] }).unwrap();
        let mut message = ChatMessage::new_with_timestamp("ai".into(), vec![
            MessagePart::markdown("body".into(), 0, "secret".into()),
            MessagePart::thinking("reason".into(), 1, "secret".into()),
            MessagePart::toolCall("call".into(), 2, "call-id".into(), "secret".into(), Default::default()),
        ], 123);
        message.parts[0].attributes.insert("file_base_path".into(), "/workspace".into());
        let presentation = operit_model::ChatMessagePresentation::ChatMessagePresentation {
            bubbles: vec![vec![MessagePart::markdown("bubble".into(), 0, "secret".into())]],
            ..Default::default()
        };
        let key = operit_model::ChatMessagePresentation::PRESENTATION_EXTENSION;
        message.pluginExtensions.insert(key.into(), serde_json::to_value(presentation).unwrap());
        let original = message.clone();
        let projected = ToolPkgMessageTransformBridge::transformPrepared("chat", message, "before_display", 0, false, &program).await.unwrap();
        assert_eq!(projected.parts[0].content, "public");
        assert_eq!(projected.parts[1].content, "public");
        assert_eq!(projected.parts[2], original.parts[2]);
        assert_eq!(projected.parts[0].attributes, original.parts[0].attributes);
        assert_eq!(projected.timestamp, original.timestamp);
        assert_eq!(projected.pluginExtensions[key]["bubbles"][0][0]["content"], "public");
        assert_eq!(original.parts[0].content, "secret");
    }

    /// Chains real text mutations while keeping structured tools and message identity intact.
    #[tokio::test]
    async fn message_transform_preserves_structure_and_chains_handlers() {
        use operit_model::MessagePart::MessagePart;
        let mut message = ChatMessage::new_with_timestamp("ai".into(), vec![
            MessagePart::markdown("body".into(), 0, "original".into()),
            MessagePart::thinking("reason".into(), 1, "reason".into()),
            MessagePart::toolCall("call".into(), 2, "call-id".into(), "search".into(), Default::default()),
            MessagePart::toolResult("result".into(), 3, Some("call-id".into()), "search".into(), "success".into(), "raw result".into()),
        ], 123);
        message.pluginExtensions.insert("owner".into(), json!({"participantId":"card-a"}));
        message.parts[0].attributes.insert("file_base_path".into(), "/workspace".into());
        let original = message.clone();
        let mut events = Vec::new();
        transformMessageText(&mut message, "chat-a", Some(json!("card-a")), 2, true, 2, |index, payload| {
            events.push(payload.clone());
            std::future::ready(Ok(Some(format!("{}:{index}", payload["text"].as_str().unwrap()))))
        }).await.unwrap();
        assert_eq!(message.parts[0].content, "original:0:1");
        assert_eq!(message.parts[1].content, "reason:0:1");
        assert_eq!(message.parts[2..], original.parts[2..]);
        assert_eq!(message.parts[0].attributes, original.parts[0].attributes);
        assert_eq!(message.timestamp, original.timestamp);
        assert_eq!(message.pluginExtensions, original.pluginExtensions);
        assert_eq!(events.len(), 4);
        assert_eq!(events[1]["text"], "original:0");
        assert_eq!(events[0]["participantId"], "card-a");
        assert_eq!(events[0]["depth"], 2);
        assert_eq!(events[0]["isEdit"], true);
    }

    /// Propagates a failed handler without invoking later handlers or returning a publishable copy.
    #[tokio::test]
    async fn message_transform_failures_stop_the_chain() {
        let mut message = ChatMessage::new_with_markdown("user".into(), "original".into());
        let mut calls = 0;
        let result = transformMessageText(&mut message, "chat", None, 0, false, 3, |index, _| {
            calls += 1;
            std::future::ready(if index == 1 { Err("rule failed".into()) } else { Ok(Some(String::new())) })
        }).await;
        assert_eq!(result, Err("rule failed".into()));
        assert_eq!(calls, 2);
    }

    /// Keeps explicit clearing distinct from nonparticipation and rejects structural results.
    #[test]
    fn message_transform_replacements_are_strict() {
        assert_eq!(decodeReplacement(Some("\"\"".into())).unwrap(), Some(String::new()));
        assert_eq!(decodeReplacement(Some("\"  \"".into())).unwrap(), Some("  ".into()));
        for raw in [None, Some(String::new()), Some("null".into())] {
            assert_eq!(decodeReplacement(raw).unwrap(), None);
        }
        for raw in ["{}", "[]", "true", "invalid"] {
            assert!(decodeReplacement(Some(raw.into())).is_err());
        }
    }
}
