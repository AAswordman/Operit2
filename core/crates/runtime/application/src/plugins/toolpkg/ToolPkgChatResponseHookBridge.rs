use std::sync::{Arc, Mutex, OnceLock};

use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_model::ChatMessage::ChatMessage;
use operit_model::MessagePart::MessagePart;
use operit_model::MessagePartCodec::AssistantMarkupStreamState;
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_CHAT_RESPONSE;
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgFunctionHookRuntime;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use serde::Deserialize;
use tokio::sync::{oneshot, watch};

use super::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;

pub use operit_model::ChatMessagePresentation::{
    ChatMessagePresentation as Presentation, PRESENTATION_EXTENSION,
};
static RUNTIME: OnceLock<ToolPkgBridgeRuntime> = OnceLock::new();

/// Connects generic response presentation hooks to the application's chat pipeline.
pub struct ToolPkgChatResponseHookBridge;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Bubble {
    parts: Vec<MessagePart>,
    delay_ms: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Delta {
    bubbles: Vec<Bubble>,
}

#[derive(Clone)]
struct Snapshot {
    content: String,
    finished: bool,
}

/// Keeps response observation independent from the native typing delivery queue.
pub struct ResponsePresentationSession {
    snapshots: watch::Sender<Snapshot>,
    completion: oneshot::Receiver<Result<(), String>>,
}

impl ResponsePresentationSession {
    /// Publishes a complete semantic snapshot while allowing the worker to coalesce observations.
    pub fn snapshot(&self, content: &str) {
        self.snapshots.send_replace(Snapshot {
            content: content.into(),
            finished: false,
        });
    }

    /// Drains the final presentation before the native turn commits its metrics and receipt.
    pub async fn finish(self, content: String) -> Result<(), String> {
        self.snapshots.send_replace(Snapshot {
            content,
            finished: true,
        });
        self.completion
            .await
            .map_err(|error| format!("Response presentation worker stopped: {error}"))?
    }
}

impl ToolPkgChatResponseHookBridge {
    /// Attaches the existing runtime and its Host capabilities to response presentation.
    pub fn register(runtime: ToolPkgBridgeRuntime) {
        RUNTIME.get_or_init(|| runtime);
    }

    /// Selects an explicitly activated presenter before publishing the assistant placeholder.
    pub async fn start(
        chat_id: String,
        message: Arc<Mutex<ChatMessage>>,
        is_active: Arc<dyn Fn() -> bool + Send + Sync>,
        publish: Arc<dyn Fn(ChatMessage) -> Result<(), String> + Send + Sync>,
    ) -> Result<Option<ResponsePresentationSession>, String> {
        let runtime = RUNTIME
            .get()
            .ok_or("Chat response hook runtime is not registered")?;
        let manager = RuntimePackageManager::readySnapshot(
            runtime.tool_handler().getOrCreatePackageManager(),
        )
        .await?;
        let initial = message
            .lock()
            .expect("presentation message mutex poisoned")
            .clone();
        let mut selected = None;
        for container in manager.getEnabledToolPkgContainerRuntimes() {
            for hook in container.chatResponseHooks {
                let raw = invoke(
                    &manager,
                    &container.packageName,
                    &hook,
                    "started",
                    &chat_id,
                    &initial,
                    &[],
                )
                .await?;
                let active = match raw.as_deref() {
                    None | Some("null") | Some("false") => false,
                    Some("true") => true,
                    Some(_) => {
                        return Err(format!(
                            "Response started hook must return a boolean: {}:{}",
                            container.packageName, hook.id
                        ))
                    }
                };
                if active {
                    if selected.is_some() {
                        return Err(
                            "Only one response presenter may own a native assistant message".into(),
                        );
                    }
                    selected = Some((container.packageName.clone(), hook));
                }
            }
        }
        let Some((owner, hook)) = selected else {
            return Ok(None);
        };
        let presentation = Presentation {
            bubbles: Vec::new(),
            pending: true,
        };
        message
            .lock()
            .expect("presentation message mutex poisoned")
            .pluginExtensions
            .insert(
                PRESENTATION_EXTENSION.into(),
                serde_json::to_value(&presentation).map_err(|error| error.to_string())?,
            );
        let (snapshots, receiver) = watch::channel(Snapshot {
            content: String::new(),
            finished: false,
        });
        let (complete, completion) = oneshot::channel();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "chat-response-presentation",
                Box::new(move || {
                    Box::pin(async move {
                        let result = deliver(
                            manager,
                            owner,
                            hook,
                            chat_id,
                            message,
                            receiver,
                            is_active,
                            publish,
                            presentation,
                        )
                        .await;
                        let _ = complete.send(result);
                    })
                }),
            )
            .map_err(|error| error.to_string())?;
        Ok(Some(ResponsePresentationSession {
            snapshots,
            completion,
        }))
    }
}

/// Calls a single registered response hook using immutable native message identity.
async fn invoke(
    manager: &RuntimePackageManager,
    owner: &str,
    hook: &ToolPkgFunctionHookRuntime,
    event: &str,
    chat_id: &str,
    message: &ChatMessage,
    parts: &[MessagePart],
) -> Result<Option<String>, String> {
    manager
        .runToolPkgMainHook(
            owner,
            &hook.function,
            TOOLPKG_EVENT_CHAT_RESPONSE,
            Some(event),
            Some(&hook.id),
            hook.functionSource.as_deref(),
            serde_json::json!({
                "chatId": chat_id, "messageTimestamp": message.timestamp,
                "variantIndex": message.selectedVariantIndex, "parts": parts,
            }),
            None,
            None,
            None,
        )
        .await
        .map_err(|error| format!("Response {event} hook {owner}:{} failed: {error}", hook.id))
}

/// Applies validated plugin deltas in order and persists only bubbles actually delivered.
async fn deliver(
    manager: RuntimePackageManager,
    owner: String,
    hook: ToolPkgFunctionHookRuntime,
    chat_id: String,
    message: Arc<Mutex<ChatMessage>>,
    mut snapshots: watch::Receiver<Snapshot>,
    is_active: Arc<dyn Fn() -> bool + Send + Sync>,
    publish: Arc<dyn Fn(ChatMessage) -> Result<(), String> + Send + Sync>,
    mut presentation: Presentation,
) -> Result<(), String> {
    let initial = message
        .lock()
        .expect("presentation message mutex poisoned")
        .clone();
    let result = async {
        while snapshots.changed().await.is_ok() {
            if !is_active() {
                return Ok(());
            }
            let snapshot = snapshots.borrow_and_update().clone();
            let parts = semantic_snapshot(&snapshot.content, snapshot.finished)?;
            let raw = invoke(
                &manager,
                &owner,
                &hook,
                if snapshot.finished {
                    "finished"
                } else {
                    "snapshot"
                },
                &chat_id,
                &initial,
                &parts,
            )
            .await?;
            let delta: Delta = serde_json::from_str(
                raw.as_deref()
                    .ok_or("Active response presenter returned no delta")?,
            )
            .map_err(|error| format!("Invalid response presentation delta: {error}"))?;
            for bubble in delta.bubbles {
                validate_bubble(&bubble, presentation.bubbles.len())?;
                let mut remaining = bubble.delay_ms;
                while remaining > 0 {
                    if !is_active() {
                        return Ok(());
                    }
                    let interval = remaining.min(50);
                    defaultHostRuntimeTaskSchedulerHost()
                        .waitForHostRuntimeDelay(interval)
                        .await;
                    remaining -= interval;
                }
                if !is_active() {
                    return Ok(());
                }
                presentation.bubbles.push(bubble.parts);
                publish_presentation(&message, &presentation, &parts, &publish)?;
            }
            if snapshot.finished {
                presentation.pending = false;
                publish_presentation(&message, &presentation, &parts, &publish)?;
                return Ok(());
            }
        }
        Ok(())
    }
    .await;
    let closed = invoke(&manager, &owner, &hook, "closed", &chat_id, &initial, &[]).await;
    result.and(closed.map(|_| ()))
}

/// Keeps incomplete semantic tags buffered until they can be classified without leaking markup.
fn semantic_snapshot(content: &str, finished: bool) -> Result<Vec<MessagePart>, String> {
    let mut parser = AssistantMarkupStreamState::new();
    parser.pushSnapshot(content)?;
    if finished {
        parser.finish()
    } else {
        Ok(parser.parts().to_vec())
    }
}

/// Rejects oversized or invalid display work at the plugin boundary.
fn validate_bubble(bubble: &Bubble, count: usize) -> Result<(), String> {
    if count >= 256 || bubble.parts.is_empty() || bubble.delay_ms > 3000 {
        return Err("Response presentation exceeds its bubble or typing delay limits".into());
    }
    Ok(())
}

/// Publishes display progress while retaining the original semantic model response.
fn publish_presentation(
    message: &Mutex<ChatMessage>,
    presentation: &Presentation,
    parts: &[MessagePart],
    publish: &Arc<dyn Fn(ChatMessage) -> Result<(), String> + Send + Sync>,
) -> Result<(), String> {
    let snapshot = {
        let mut message = message.lock().expect("presentation message mutex poisoned");
        message.parts = parts.to_vec();
        message.pluginExtensions.insert(
            PRESENTATION_EXTENSION.into(),
            serde_json::to_value(presentation).map_err(|error| error.to_string())?,
        );
        let mut snapshot = message.clone();
        if !presentation.pending {
            snapshot.contentStream = None;
        }
        snapshot
    };
    publish(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Holds incomplete tool tags outside visible Markdown and keeps thinking semantic.
    #[test]
    fn partial_snapshots_never_expose_internal_protocol_as_sentences() {
        let parts = semantic_snapshot("<think>private</think>hello。<tool name=", false).unwrap();
        assert!(parts.iter().any(|part| part.kind
            == operit_model::MessagePart::MessagePartKind::Thinking
            && part.content == "private"));
        let visible: String = parts
            .iter()
            .filter(|part| part.kind == operit_model::MessagePart::MessagePartKind::Markdown)
            .map(|part| part.content.as_str())
            .collect();
        assert_eq!(visible, "hello。");
    }

    /// Saves display bubbles without rewriting the original semantic assistant history.
    #[test]
    fn display_progress_preserves_canonical_response_parts() {
        let original = vec![MessagePart::markdown(
            "canonical".into(),
            0,
            "first。second。".into(),
        )];
        let message = Mutex::new(ChatMessage::new("ai".into()));
        let presentation = Presentation {
            bubbles: vec![vec![MessagePart::markdown(
                "sentence".into(),
                0,
                "first。".into(),
            )]],
            pending: true,
        };
        let delivered = Arc::new(Mutex::new(None));
        let destination = delivered.clone();
        let publish: Arc<dyn Fn(ChatMessage) -> Result<(), String> + Send + Sync> =
            Arc::new(move |message| {
                *destination.lock().unwrap() = Some(message);
                Ok(())
            });
        publish_presentation(&message, &presentation, &original, &publish).unwrap();
        let delivered = delivered.lock().unwrap().take().unwrap();
        assert_eq!(delivered.parts, original);
        assert_eq!(
            delivered.pluginExtensions[PRESENTATION_EXTENSION]["bubbles"][0][0]["content"],
            "first。"
        );
        assert_eq!(
            delivered.pluginExtensions[PRESENTATION_EXTENSION]["pending"],
            true
        );
    }

    /// Reports native persistence failures to the response worker instead of reporting delivery success.
    #[test]
    fn display_progress_propagates_persistence_errors() {
        let message = Mutex::new(ChatMessage::new("ai".into()));
        let publish: Arc<dyn Fn(ChatMessage) -> Result<(), String> + Send + Sync> =
            Arc::new(|_| Err("native commit failed".into()));
        assert_eq!(
            publish_presentation(&message, &Presentation::default(), &[], &publish),
            Err("native commit failed".into())
        );
    }

    /// Rejects invalid delivery work instead of changing its timing or content.
    #[test]
    fn presentation_limits_are_explicit() {
        let bubble = Bubble {
            parts: vec![MessagePart::markdown("sentence".into(), 0, "hello".into())],
            delay_ms: 3001,
        };
        assert!(validate_bubble(&bubble, 0).is_err());
        let bubble = Bubble {
            delay_ms: 0,
            ..bubble
        };
        assert!(validate_bubble(&bubble, 256).is_err());
        assert!(validate_bubble(&bubble, 255).is_ok());
    }
}
