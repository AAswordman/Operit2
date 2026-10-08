use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use crate::core_proxy::SharedLocalCore;
use operit_link::{
    CoreEvent, CoreEventKind, CoreLinkClient, CoreLinkError, CoreRequestId, CoreStreamDescriptor,
    CoreValue, CoreWatchRequest,
};
use operit_model::ChatMessage::ChatMessage;
use operit_proxy_local::{GeneratedCoreProxy, LocalCoreProxy};
use operit_util::AppLogger::AppLogger;
use serde::de::DeserializeOwned;

pub(super) struct TuiCore {
    proxy: GeneratedCoreProxy<SharedLocalCore>,
    eventSender: tokio::sync::mpsc::UnboundedSender<CoreEvent>,
    eventReceiver: tokio::sync::mpsc::UnboundedReceiver<CoreEvent>,
    stateFlowValues: TuiStateFlowValues,
    messageWatchTask: Option<tokio::task::JoinHandle<()>>,
    messageWatchChatId: Option<String>,
    messageWatchRequestId: Option<CoreRequestId>,
    messageWatchGeneration: u64,
    stateWatchTask: Option<tokio::task::JoinHandle<()>>,
    stateWatchChatId: Option<String>,
    stateWatchRequestId: Option<CoreRequestId>,
    stateWatchGeneration: u64,
    contentStreamWatches: BTreeMap<String, TuiContentStreamWatch>,
    contentStreamGeneration: u64,
}

type TuiStateFlowKey = (Option<CoreRequestId>, String, String);

/// Retains complete wire values independently for each StateFlow subscription.
#[derive(Default)]
struct TuiStateFlowValues {
    values: BTreeMap<TuiStateFlowKey, CoreValue>,
}

impl TuiStateFlowValues {
    /// Applies one ordered StateFlow event before decoding its complete typed value.
    fn decode<T: DeserializeOwned>(&mut self, event: &CoreEvent) -> Result<Option<T>, String> {
        let key = (
            event.requestId.clone(),
            event.target.clone(),
            event.propertyName.clone(),
        );
        let context = format!(
            "requestId={:?} target={} property={} kind={:?}",
            event.requestId.as_ref().map(|id| id.0.as_str()),
            event.target,
            event.propertyName,
            event.kind,
        );
        let value = match event.kind {
            CoreEventKind::Snapshot | CoreEventKind::Changed => event.value.clone(),
            CoreEventKind::Delta => self
                .values
                .get(&key)
                .ok_or_else(|| format!("TUI StateFlow delta has no complete base: {context}"))?
                .applyIncrementalDelta(&event.value)
                .map_err(|error| format!("TUI StateFlow delta failed: {context}: {error}"))?,
            CoreEventKind::Completed => {
                self.values.remove(&key);
                return Ok(None);
            }
        };
        let decoded = operit_link::fromCoreValue(value.clone())
            .map_err(|error| format!("TUI StateFlow decode failed: {context}: {error}"))?;
        self.values.insert(key, value);
        Ok(Some(decoded))
    }

    /// Removes the retained values owned by a cancelled StateFlow subscription.
    fn clear_request(&mut self, requestId: &CoreRequestId) {
        self.values
            .retain(|(id, _, _), _| id.as_ref() != Some(requestId));
    }
}

#[derive(Clone, Debug)]
pub(super) struct TuiContentStreamEventInfo {
    pub(super) streamId: String,
    pub(super) messageTimestamp: i64,
}

struct TuiContentStreamWatch {
    requestId: CoreRequestId,
    streamId: String,
    messageTimestamp: i64,
    task: tokio::task::JoinHandle<()>,
}

/// Creates a TUI proxy wrapper with an internal event queue.
pub(super) fn tui_core(client: Arc<LocalCoreProxy>) -> TuiCore {
    let (eventSender, eventReceiver) = tokio::sync::mpsc::unbounded_channel();
    TuiCore {
        proxy: GeneratedCoreProxy::new(SharedLocalCore(client)),
        eventSender,
        eventReceiver,
        stateFlowValues: TuiStateFlowValues::default(),
        messageWatchTask: None,
        messageWatchChatId: None,
        messageWatchRequestId: None,
        messageWatchGeneration: 0,
        stateWatchTask: None,
        stateWatchChatId: None,
        stateWatchRequestId: None,
        stateWatchGeneration: 0,
        contentStreamWatches: BTreeMap::new(),
        contentStreamGeneration: 0,
    }
}

impl TuiCore {
    /// Decodes an ordered StateFlow event without treating embedded stream events as state.
    #[allow(non_snake_case)]
    pub(super) fn decodeStateFlowEvent<T: DeserializeOwned>(
        &mut self,
        event: &CoreEvent,
    ) -> Result<Option<T>, String> {
        self.stateFlowValues.decode(event)
    }

    #[allow(non_snake_case)]
    /// Watches every zero-argument state flow exposed by the main chat runtime.
    pub(super) async fn watchMainChatGeneratedStateFlows(&mut self) -> Result<(), CoreLinkError> {
        self.proxy
            .chat_runtime_holder_main()
            .watchAllGeneratedStateFlows(self.eventSender.clone())
            .await
    }

    #[allow(non_snake_case)]
    /// Watches message changes for one explicit main chat id.
    pub(super) async fn watchMainChatMessagesFlow(
        &mut self,
        chatId: String,
    ) -> Result<(), CoreLinkError> {
        if self.messageWatchChatId.as_ref() == Some(&chatId) {
            return Ok(());
        }
        self.clearMainChatMessagesWatch();
        self.messageWatchGeneration += 1;
        let requestId = CoreRequestId::new(format!(
            "tui-main-chat-messages-{}",
            self.messageWatchGeneration
        ));
        AppLogger::trace(
            "TuiStreamTrace",
            &format!(
                "messages_watch.open requestId={} chatId={}",
                requestId.0, chatId
            ),
        );
        let mut chatProxy = self.proxy.chat_runtime_holder_main();
        let target = LocalCoreProxy::generatedTargetForSchema("chatRuntimeHolderMain")
            .ok_or_else(|| CoreLinkError::internal("chat runtime object id is not generated"))?;
        let mut args = std::collections::BTreeMap::new();
        args.insert(
            "chatId".to_string(),
            operit_link::toCoreValue(Some(chatId.clone()))
                .map_err(|error| CoreLinkError::internal(error.to_string()))?,
        );
        let mut stream = chatProxy
            .generatedClientMut()
            .watch(CoreWatchRequest::new(
                requestId.0.clone(),
                target,
                "chatMessagesFlow",
                CoreValue::Map(args),
            ))
            .await?;
        let sender = self.eventSender.clone();
        let eventRequestId = requestId.clone();
        let eventChatId = chatId.clone();
        self.messageWatchChatId = Some(chatId);
        self.messageWatchRequestId = Some(requestId);
        self.messageWatchTask = Some(tokio::spawn(async move {
            let mut eventCount = 0_u64;
            while let Some(mut event) = stream.recv().await {
                eventCount += 1;
                if eventCount <= 3 || event.kind == operit_link::CoreEventKind::Completed {
                    AppLogger::trace(
                        "TuiStreamTrace",
                        &format!(
                            "messages_watch.event requestId={} chatId={} count={} kind={:?}",
                            eventRequestId.0, eventChatId, eventCount, event.kind
                        ),
                    );
                }
                event.requestId = Some(eventRequestId.clone());
                let _ = sender.send(event);
            }
            AppLogger::trace(
                "TuiStreamTrace",
                &format!(
                    "messages_watch.closed requestId={} chatId={} events={}",
                    eventRequestId.0, eventChatId, eventCount
                ),
            );
        }));
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Synchronizes embedded content stream watches for the current main chat messages.
    pub(super) async fn syncMainChatContentStreams(
        &mut self,
        messages: &[ChatMessage],
    ) -> Result<(), CoreLinkError> {
        let activeStreams = messages
            .iter()
            .filter_map(|message| {
                message.contentStream.as_ref().map(|stream| {
                    (
                        stream.descriptor.streamId.clone(),
                        message.timestamp,
                        stream.descriptor.clone(),
                    )
                })
            })
            .collect::<Vec<_>>();
        let activeStreamIds = activeStreams
            .iter()
            .map(|(streamId, _, _)| streamId.clone())
            .collect::<BTreeSet<_>>();
        let staleStreamIds = self
            .contentStreamWatches
            .keys()
            .filter(|streamId| !activeStreamIds.contains(*streamId))
            .cloned()
            .collect::<Vec<_>>();
        if !staleStreamIds.is_empty() || !activeStreamIds.is_empty() {
            AppLogger::trace(
                "TuiStreamTrace",
                &format!(
                    "content_watches.sync messages={} active={} stale={}",
                    messages.len(),
                    activeStreamIds
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(","),
                    staleStreamIds.join(",")
                ),
            );
        }
        for streamId in staleStreamIds {
            if let Some(watch) = self.contentStreamWatches.remove(&streamId) {
                AppLogger::trace(
                    "TuiStreamTrace",
                    &format!(
                        "content_watch.abort streamId={} requestId={} messageTimestamp={}",
                        watch.streamId, watch.requestId.0, watch.messageTimestamp
                    ),
                );
                watch.task.abort();
            }
        }
        for (streamId, messageTimestamp, descriptor) in activeStreams {
            if let Some(watch) = self.contentStreamWatches.get_mut(&streamId) {
                watch.messageTimestamp = messageTimestamp;
                continue;
            }
            self.openMainChatContentStream(streamId, messageTimestamp, descriptor)
                .await?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Opens one embedded content stream watch and forwards its events into the TUI queue.
    async fn openMainChatContentStream(
        &mut self,
        streamId: String,
        messageTimestamp: i64,
        descriptor: CoreStreamDescriptor,
    ) -> Result<(), CoreLinkError> {
        self.contentStreamGeneration += 1;
        let requestId = CoreRequestId::new(format!(
            "tui-main-content-stream-{}",
            self.contentStreamGeneration
        ));
        AppLogger::trace(
            "TuiStreamTrace",
            &format!(
                "content_watch.open streamId={} requestId={} messageTimestamp={}",
                streamId, requestId.0, messageTimestamp
            ),
        );
        let mut chatProxy = self.proxy.chat_runtime_holder_main();
        let mut stream = chatProxy
            .generatedClientMut()
            .watch(CoreWatchRequest::new(
                requestId.0.clone(),
                descriptor.target,
                descriptor.propertyName,
                descriptor.args,
            ))
            .await?;
        let sender = self.eventSender.clone();
        let eventRequestId = requestId.clone();
        let eventStreamId = streamId.clone();
        let task = tokio::spawn(async move {
            let mut eventCount = 0_u64;
            while let Some(mut event) = stream.recv().await {
                eventCount += 1;
                if eventCount <= 3 || event.kind == operit_link::CoreEventKind::Completed {
                    AppLogger::trace(
                        "TuiStreamTrace",
                        &format!(
                            "content_watch.event streamId={} requestId={} count={} kind={:?} property={}",
                            eventStreamId,
                            eventRequestId.0,
                            eventCount,
                            event.kind,
                            event.propertyName
                        ),
                    );
                }
                event.requestId = Some(eventRequestId.clone());
                let _ = sender.send(event);
            }
            AppLogger::trace(
                "TuiStreamTrace",
                &format!(
                    "content_watch.closed streamId={} requestId={} events={}",
                    eventStreamId, eventRequestId.0, eventCount
                ),
            );
        });
        self.contentStreamWatches.insert(
            streamId.clone(),
            TuiContentStreamWatch {
                requestId,
                streamId,
                messageTimestamp,
                task,
            },
        );
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Watches routed runtime state changes for one explicit main chat id.
    pub(super) async fn watchMainChatStateFlow(
        &mut self,
        chatId: String,
    ) -> Result<(), CoreLinkError> {
        if self.stateWatchChatId.as_ref() == Some(&chatId) {
            return Ok(());
        }
        self.clearMainChatStateWatch();
        self.stateWatchGeneration += 1;
        let requestId =
            CoreRequestId::new(format!("tui-main-chat-state-{}", self.stateWatchGeneration));
        AppLogger::trace(
            "TuiStreamTrace",
            &format!(
                "state_watch.open requestId={} chatId={}",
                requestId.0, chatId
            ),
        );
        let mut chatProxy = self.proxy.chat_runtime_holder_main();
        let target = LocalCoreProxy::generatedTargetForSchema("chatRuntimeHolderMain")
            .ok_or_else(|| CoreLinkError::internal("chat runtime object id is not generated"))?;
        let mut args = std::collections::BTreeMap::new();
        args.insert(
            "chatId".to_string(),
            operit_link::toCoreValue(Some(chatId.clone()))
                .map_err(|error| CoreLinkError::internal(error.to_string()))?,
        );
        let mut stream = chatProxy
            .generatedClientMut()
            .watch(CoreWatchRequest::new(
                requestId.0.clone(),
                target,
                "chatStateFlow",
                CoreValue::Map(args),
            ))
            .await?;
        let sender = self.eventSender.clone();
        let eventRequestId = requestId.clone();
        let eventChatId = chatId.clone();
        self.stateWatchChatId = Some(chatId);
        self.stateWatchRequestId = Some(requestId);
        self.stateWatchTask = Some(tokio::spawn(async move {
            let mut eventCount = 0_u64;
            while let Some(mut event) = stream.recv().await {
                eventCount += 1;
                if eventCount <= 3 || event.kind == operit_link::CoreEventKind::Completed {
                    AppLogger::trace(
                        "TuiStreamTrace",
                        &format!(
                            "state_watch.event requestId={} chatId={} count={} kind={:?}",
                            eventRequestId.0, eventChatId, eventCount, event.kind
                        ),
                    );
                }
                event.requestId = Some(eventRequestId.clone());
                let _ = sender.send(event);
            }
            AppLogger::trace(
                "TuiStreamTrace",
                &format!(
                    "state_watch.closed requestId={} chatId={} events={}",
                    eventRequestId.0, eventChatId, eventCount
                ),
            );
        }));
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Stops the active main chat state watch.
    pub(super) fn clearMainChatStateWatch(&mut self) {
        if let Some(requestId) = self.stateWatchRequestId.as_ref() {
            self.stateFlowValues.clear_request(requestId);
        }
        if let Some(task) = self.stateWatchTask.take() {
            AppLogger::trace(
                "TuiStreamTrace",
                &format!(
                    "state_watch.abort requestId={}",
                    self.stateWatchRequestId
                        .as_ref()
                        .map(|requestId| requestId.0.as_str())
                        .unwrap_or("<none>")
                ),
            );
            task.abort();
        }
        self.stateWatchChatId = None;
        self.stateWatchRequestId = None;
    }

    #[allow(non_snake_case)]
    /// Answers one pending chat tool permission request through the owning
    /// chat route, so the response reaches whichever node hosts the chat.
    pub(super) async fn respondToolPermission(
        &mut self,
        chatId: String,
        requestId: String,
        result: String,
    ) -> Result<(), String> {
        let mut chatProxy = self.proxy.chat_runtime_holder_main();
        chatProxy
            .respondChatToolPermission(chatId, requestId, result)
            .await
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    /// Reports whether an event belongs to the active main chat state watch.
    pub(super) fn isActiveMainChatStateEvent(&self, event: &CoreEvent) -> bool {
        event.propertyName == "chatStateFlow"
            && event.requestId.as_ref() == self.stateWatchRequestId.as_ref()
    }

    #[allow(non_snake_case)]
    /// Stops the active main chat message watch.
    pub(super) fn clearMainChatMessagesWatch(&mut self) {
        if let Some(requestId) = self.messageWatchRequestId.as_ref() {
            self.stateFlowValues.clear_request(requestId);
        }
        if let Some(task) = self.messageWatchTask.take() {
            AppLogger::trace(
                "TuiStreamTrace",
                &format!(
                    "messages_watch.abort requestId={}",
                    self.messageWatchRequestId
                        .as_ref()
                        .map(|requestId| requestId.0.as_str())
                        .unwrap_or("<none>")
                ),
            );
            task.abort();
        }
        self.messageWatchChatId = None;
        self.messageWatchRequestId = None;
        self.clearMainChatContentStreams();
    }

    #[allow(non_snake_case)]
    /// Stops every embedded content stream watch owned by the active main chat.
    pub(super) fn clearMainChatContentStreams(&mut self) {
        for (_, watch) in std::mem::take(&mut self.contentStreamWatches) {
            AppLogger::trace(
                "TuiStreamTrace",
                &format!(
                    "content_watch.abort streamId={} requestId={} messageTimestamp={}",
                    watch.streamId, watch.requestId.0, watch.messageTimestamp
                ),
            );
            watch.task.abort();
        }
    }

    #[allow(non_snake_case)]
    /// Returns the current embedded content stream ids owned by the active main chat.
    pub(super) fn activeMainChatContentStreamIds(&self) -> BTreeSet<String> {
        self.contentStreamWatches.keys().cloned().collect()
    }

    #[allow(non_snake_case)]
    /// Returns metadata for an event emitted by an active embedded content stream watch.
    pub(super) fn mainChatContentStreamEventInfo(
        &self,
        event: &CoreEvent,
    ) -> Option<TuiContentStreamEventInfo> {
        let requestId = event.requestId.as_ref()?;
        self.contentStreamWatches
            .values()
            .find(|watch| watch.requestId.0 == requestId.0)
            .map(|watch| TuiContentStreamEventInfo {
                streamId: watch.streamId.clone(),
                messageTimestamp: watch.messageTimestamp,
            })
    }

    #[allow(non_snake_case)]
    /// Reports whether an event belongs to the active main chat message watch.
    pub(super) fn isActiveMainChatMessagesEvent(&self, event: &CoreEvent) -> bool {
        event.propertyName == "chatMessagesFlow"
            && event.requestId.as_ref() == self.messageWatchRequestId.as_ref()
    }

    #[allow(non_snake_case)]
    /// Drains all queued generated Core events.
    pub(super) fn drainEvents(&mut self) -> Vec<CoreEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.eventReceiver.try_recv() {
            events.push(event);
        }
        events
    }
}

impl Deref for TuiCore {
    type Target = GeneratedCoreProxy<SharedLocalCore>;

    fn deref(&self) -> &Self::Target {
        &self.proxy
    }
}

impl DerefMut for TuiCore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.proxy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::HostManager::HostManager;
    use operit_host_api::{
        FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
        GrepCodeResult, HostEnvironmentDescriptor, HostError, HostResult,
    };
    use operit_host_native_common::{
        NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost,
        NativeRuntimeStorageHost,
    };
    use operit_link::{CoreEventKind, CORE_STREAM_TARGET};
    use operit_runtime::core::application::OperitApplication::OperitApplication;
    use operit_util::MarkdownRenderStream::MarkdownStreamEvent;
    use operit_util::RuntimeStorageLayout::{RUNTIME_ROOT_DIR_PATH, WORKSPACE_DIR_PATH};
    use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    static TEST_RUNTIME_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    /// Constructs a state event with explicit subscription identity for decoder tests.
    fn state_flow_event(
        request_id: &str,
        property: &str,
        kind: CoreEventKind,
        value: CoreValue,
    ) -> CoreEvent {
        CoreEvent {
            requestId: Some(CoreRequestId::new(request_id)),
            target: "chat-runtime".to_string(),
            propertyName: property.to_string(),
            kind,
            value,
        }
    }

    /// Produces a large stable value whose counter updates are encoded as deltas.
    fn state_flow_value(label: &str, counter: u64) -> CoreValue {
        CoreValue::Map(BTreeMap::from([
            (
                "background".to_string(),
                CoreValue::String(label.repeat(500)),
            ),
            ("counter".to_string(), CoreValue::Unsigned(counter)),
        ]))
    }

    /// Verifies successive message deltas add, update, complete, and remove messages.
    #[test]
    fn tui_state_flow_reconstructs_message_deltas() {
        let mut values = TuiStateFlowValues::default();
        let initial = vec![ChatMessage::new_with_markdown_timestamp(
            "user".to_string(),
            "stable history ".repeat(200),
            1,
        )];
        let mut receiving = initial.clone();
        receiving.push(ChatMessage::new_with_markdown_timestamp(
            "ai".to_string(),
            "B is continuing".to_string(),
            2,
        ));
        let mut completed = receiving.clone();
        completed[1].replace_with_markdown("B completed the task".to_string());
        completed[1].completedAt = 3;
        let mut previous = None;
        for (index, expected) in [initial.clone(), receiving, completed, initial]
            .into_iter()
            .enumerate()
        {
            let (kind, value) = CoreValue::incrementalEvent(
                &mut previous,
                operit_link::toCoreValue(expected.clone()).unwrap(),
            );
            assert_eq!(
                kind,
                if index == 0 {
                    CoreEventKind::Snapshot
                } else {
                    CoreEventKind::Delta
                }
            );
            let event = state_flow_event("messages-1", "chatMessagesFlow", kind, value);
            let decoded = values.decode::<Vec<ChatMessage>>(&event).unwrap().unwrap();
            assert_eq!(decoded, expected);
            assert_eq!(
                decoded.last().unwrap().completedAt,
                expected.last().unwrap().completedAt
            );
        }
    }

    /// Verifies a routed completion delta clears loading and replaces the connecting state.
    #[test]
    fn tui_state_flow_completion_delta_clears_loading() {
        use operit_model::InputProcessingState::InputProcessingState;
        use operit_runtime::services::ChatServiceCore::ChatState;

        let initial = ChatState {
            currentChatId: "handoff-chat".to_string(),
            currentChatTitle: "stable chat title ".repeat(100),
            currentCharacterCardName: None,
            currentCharacterCardAvatarUri: None,
            currentWorkspacePath: Some("/workspace/b".to_string()),
            isLoading: true,
            inputProcessingState: InputProcessingState::Connecting {
                message: "connecting".to_string(),
            },
            hasOlderDisplayHistory: false,
            hasNewerDisplayHistory: false,
            isLoadingDisplayWindow: false,
            pendingQueueMessages: Vec::new(),
            isPendingQueueExpanded: false,
            toolPermissionRequests: Vec::new(),
        };
        let mut completed = initial.clone();
        completed.isLoading = false;
        completed.inputProcessingState = InputProcessingState::Completed;
        let mut values = TuiStateFlowValues::default();
        let mut previous = None;
        for (index, expected) in [initial, completed].into_iter().enumerate() {
            let (kind, value) = CoreValue::incrementalEvent(
                &mut previous,
                operit_link::toCoreValue(expected.clone()).unwrap(),
            );
            assert_eq!(
                kind,
                if index == 0 {
                    CoreEventKind::Snapshot
                } else {
                    CoreEventKind::Delta
                }
            );
            let event = state_flow_event("state-1", "chatStateFlow", kind, value);
            let decoded = values.decode::<ChatState>(&event).unwrap().unwrap();
            assert_eq!(decoded, expected);
        }
    }

    /// Verifies pending chat tool permission requests survive the chat state
    /// round-trip, so the approval modal can be driven from chat state.
    #[test]
    fn tui_state_flow_decodes_tool_permission_requests() {
        use operit_model::InputProcessingState::InputProcessingState;
        use operit_runtime::services::ChatServiceCore::ChatState;
        use operit_runtime::services::RuntimeHostInteractionService::{
            RuntimeHostInteractionToolPermissionRequest,
            RuntimeHostInteractionToolPermissionTool,
            RuntimeHostInteractionToolPermissionToolParameter,
        };

        let request = RuntimeHostInteractionToolPermissionRequest {
            requestId: "permission-1".to_string(),
            chatId: "handoff-chat".to_string(),
            tool: RuntimeHostInteractionToolPermissionTool {
                name: "Tools.Terminal.execute".to_string(),
                parameters: vec![RuntimeHostInteractionToolPermissionToolParameter {
                    name: "command".to_string(),
                    value: "cargo test".to_string(),
                }],
            },
            description: "Run a terminal command".to_string(),
            requestedAtMillis: 42_000,
        };
        let mut state = ChatState {
            currentChatId: "handoff-chat".to_string(),
            currentChatTitle: "title".to_string(),
            currentCharacterCardName: None,
            currentCharacterCardAvatarUri: None,
            currentWorkspacePath: None,
            isLoading: false,
            inputProcessingState: InputProcessingState::Completed,
            hasOlderDisplayHistory: false,
            hasNewerDisplayHistory: false,
            isLoadingDisplayWindow: false,
            pendingQueueMessages: Vec::new(),
            isPendingQueueExpanded: false,
            toolPermissionRequests: vec![request],
        };
        let mut values = TuiStateFlowValues::default();
        let mut previous = None;
        let (kind, value) = CoreValue::incrementalEvent(
            &mut previous,
            operit_link::toCoreValue(state.clone()).unwrap(),
        );
        assert_eq!(kind, CoreEventKind::Snapshot);
        let event = state_flow_event("state-1", "chatStateFlow", kind, value);
        let decoded = values.decode::<ChatState>(&event).unwrap().unwrap();
        assert_eq!(decoded.toolPermissionRequests.len(), 1);
        assert_eq!(decoded.toolPermissionRequests[0].requestId, "permission-1");
        assert_eq!(
            decoded.toolPermissionRequests[0].tool.name,
            "Tools.Terminal.execute"
        );

        state.toolPermissionRequests.clear();
        let cleared = operit_link::toCoreValue(state).unwrap();
        let event = state_flow_event("state-1", "chatStateFlow", CoreEventKind::Snapshot, cleared);
        let decoded = values.decode::<ChatState>(&event).unwrap().unwrap();
        assert!(decoded.toolPermissionRequests.is_empty());
    }

    /// Verifies replacement snapshots and changed values become the next delta base.
    #[test]
    fn tui_state_flow_replaces_base_at_route_snapshot() {
        let mut values = TuiStateFlowValues::default();
        for (label, kind) in [
            ("A", CoreEventKind::Snapshot),
            ("B", CoreEventKind::Snapshot),
            ("C", CoreEventKind::Changed),
        ] {
            let base = state_flow_value(label, 0);
            let event = state_flow_event("state-1", "chatStateFlow", kind, base.clone());
            assert_eq!(
                values.decode::<CoreValue>(&event).unwrap(),
                Some(base.clone())
            );
            let expected = state_flow_value(label, 1);
            let (kind, value) = CoreValue::incrementalEvent(&mut Some(base), expected.clone());
            assert_eq!(kind, CoreEventKind::Delta);
            let event = state_flow_event("state-1", "chatStateFlow", kind, value);
            assert_eq!(values.decode::<CoreValue>(&event).unwrap(), Some(expected));
        }
    }

    /// Verifies request, target, and property identities cannot share incremental bases.
    #[test]
    fn tui_state_flow_isolates_subscription_keys() {
        let mut values = TuiStateFlowValues::default();
        let subscriptions = [
            ("request-a", "target-a", "chatStateFlow", "A"),
            ("request-b", "target-a", "chatStateFlow", "B"),
            ("request-a", "target-b", "chatStateFlow", "C"),
            ("request-a", "target-a", "chatHistoriesFlow", "D"),
        ];
        for (request, target, property, label) in subscriptions {
            let mut event = state_flow_event(
                request,
                property,
                CoreEventKind::Snapshot,
                state_flow_value(label, 0),
            );
            event.target = target.to_string();
            values.decode::<CoreValue>(&event).unwrap();
        }
        for (request, target, property, label) in subscriptions {
            let expected = state_flow_value(label, 1);
            let (kind, value) = CoreValue::incrementalEvent(
                &mut Some(state_flow_value(label, 0)),
                expected.clone(),
            );
            assert_eq!(kind, CoreEventKind::Delta);
            let mut event = state_flow_event(request, property, kind, value);
            event.target = target.to_string();
            assert_eq!(values.decode::<CoreValue>(&event).unwrap(), Some(expected));
        }
        assert_eq!(values.values.len(), subscriptions.len());
    }

    /// Verifies cancelling one request leaves other live subscription bases intact.
    #[test]
    fn tui_state_flow_cancellation_removes_request_base() {
        let mut values = TuiStateFlowValues::default();
        let base = state_flow_value("history", 0);
        for request in ["old-watch", "live-watch"] {
            let event = state_flow_event(
                request,
                "chatMessagesFlow",
                CoreEventKind::Snapshot,
                base.clone(),
            );
            values.decode::<CoreValue>(&event).unwrap();
        }
        values.clear_request(&CoreRequestId::new("old-watch"));
        assert_eq!(values.values.len(), 1);
        let expected = state_flow_value("history", 1);
        let (kind, delta) = CoreValue::incrementalEvent(&mut Some(base), expected.clone());
        let old = state_flow_event("old-watch", "chatMessagesFlow", kind.clone(), delta.clone());
        assert!(values.decode::<CoreValue>(&old).is_err());
        let live = state_flow_event("live-watch", "chatMessagesFlow", kind, delta);
        assert_eq!(values.decode::<CoreValue>(&live).unwrap(), Some(expected));
    }

    /// Verifies terminal events remove the base without decoding their empty payload.
    #[test]
    fn tui_state_flow_completed_event_removes_base() {
        let mut values = TuiStateFlowValues::default();
        let base = state_flow_value("history", 0);
        let snapshot = state_flow_event(
            "watch",
            "chatHistoriesFlow",
            CoreEventKind::Snapshot,
            base.clone(),
        );
        values.decode::<CoreValue>(&snapshot).unwrap();
        let completed = state_flow_event(
            "watch",
            "chatHistoriesFlow",
            CoreEventKind::Completed,
            CoreValue::Null,
        );
        assert_eq!(values.decode::<CoreValue>(&completed).unwrap(), None);
        assert!(values.values.is_empty());
        let (kind, delta) =
            CoreValue::incrementalEvent(&mut Some(base), state_flow_value("history", 1));
        let event = state_flow_event("watch", "chatHistoriesFlow", kind, delta);
        assert!(values.decode::<CoreValue>(&event).is_err());
    }

    /// Verifies missing snapshots, invalid patches, and typed decode errors are surfaced.
    #[test]
    fn tui_state_flow_rejects_invalid_events() {
        let mut values = TuiStateFlowValues::default();
        let event = state_flow_event(
            "watch",
            "chatStateFlow",
            CoreEventKind::Delta,
            CoreValue::Null,
        );
        assert_eq!(values.decode::<CoreValue>(&event).unwrap_err(),
            "TUI StateFlow delta has no complete base: requestId=Some(\"watch\") target=chat-runtime property=chatStateFlow kind=Delta");
        let snapshot = state_flow_event(
            "watch",
            "chatStateFlow",
            CoreEventKind::Snapshot,
            state_flow_value("base", 0),
        );
        assert!(values.decode::<i64>(&snapshot).is_err());
        assert!(values.values.is_empty());
        values.decode::<CoreValue>(&snapshot).unwrap();
        assert!(values.decode::<CoreValue>(&event).is_err());
        assert_eq!(values.values.len(), 1);
    }

    /// Provides file-system access for isolated TUI runtime initialization tests.
    #[derive(Clone, Debug, Default)]
    struct TestFileSystemHost;

    impl TestFileSystemHost {
        /// Creates the standard test error for operations outside this test's scope.
        fn unsupported(operation: &str) -> HostError {
            HostError::new(format!(
                "TUI embedded stream test file host does not support {operation}"
            ))
        }

        /// Converts a file length into the host metadata integer shape.
        fn host_size(size: u64) -> HostResult<i64> {
            i64::try_from(size).map_err(|error| HostError::new(error.to_string()))
        }
    }

    impl FileSystemHost for TestFileSystemHost {
        /// Returns the test environment label.
        #[allow(non_snake_case)]
        fn envLabel(&self) -> &str {
            "tui-test"
        }

        /// Returns the host descriptor used by the synthetic TUI test environment.
        #[allow(non_snake_case)]
        fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
            HostEnvironmentDescriptor::linux()
        }

        /// Validates that the supplied path is not empty.
        #[allow(non_snake_case)]
        fn validatePath(&self, path: &str, paramName: &str) -> HostResult<()> {
            if path.is_empty() {
                return Err(HostError::new(format!("{paramName} must not be empty")));
            }
            Ok(())
        }

        /// Lists direct children for one test directory.
        #[allow(non_snake_case)]
        fn listFiles(&self, path: &str) -> HostResult<Vec<FileEntry>> {
            let mut entries = Vec::new();
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                entries.push(FileEntry {
                    name: entry.file_name().to_string_lossy().to_string(),
                    isDirectory: metadata.is_dir(),
                    size: Self::host_size(metadata.len())?,
                    permissions: String::new(),
                    lastModified: String::new(),
                });
            }
            Ok(entries)
        }

        /// Reads one UTF-8 test file.
        #[allow(non_snake_case)]
        fn readFile(&self, path: &str) -> HostResult<String> {
            std::fs::read_to_string(path).map_err(HostError::from)
        }

        /// Reads at most the requested number of bytes from one UTF-8 test file.
        #[allow(non_snake_case)]
        fn readFileWithLimit(&self, path: &str, maxBytes: usize) -> HostResult<String> {
            let bytes = std::fs::read(path)?;
            let limit = maxBytes.min(bytes.len());
            String::from_utf8(bytes[..limit].to_vec())
                .map_err(|error| HostError::new(error.to_string()))
        }

        /// Reads one binary test file.
        #[allow(non_snake_case)]
        fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
            std::fs::read(path).map_err(HostError::from)
        }

        /// Writes one UTF-8 test file, creating parent directories as needed.
        #[allow(non_snake_case)]
        fn writeFile(&self, path: &str, content: &str, append: bool) -> HostResult<()> {
            if let Some(parent) = std::path::Path::new(path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut options = std::fs::OpenOptions::new();
            options.create(true).write(true);
            if append {
                options.append(true);
            } else {
                options.truncate(true);
            }
            options.open(path)?.write_all(content.as_bytes())?;
            Ok(())
        }

        /// Writes one binary test file, creating parent directories as needed.
        #[allow(non_snake_case)]
        fn writeFileBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
            if let Some(parent) = std::path::Path::new(path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, content).map_err(HostError::from)
        }

        /// Deletes one test file or directory.
        #[allow(non_snake_case)]
        fn deleteFile(&self, path: &str, recursive: bool) -> HostResult<()> {
            let metadata = std::fs::metadata(path)?;
            if metadata.is_dir() {
                if recursive {
                    std::fs::remove_dir_all(path)?;
                } else {
                    std::fs::remove_dir(path)?;
                }
            } else {
                std::fs::remove_file(path)?;
            }
            Ok(())
        }

        /// Reports whether one test path exists.
        #[allow(non_snake_case)]
        fn fileExists(&self, path: &str) -> HostResult<FileExistence> {
            match std::fs::metadata(path) {
                Ok(metadata) => Ok(FileExistence {
                    exists: true,
                    isDirectory: metadata.is_dir(),
                    size: Self::host_size(metadata.len())?,
                }),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(FileExistence {
                    exists: false,
                    isDirectory: false,
                    size: 0,
                }),
                Err(error) => Err(HostError::from(error)),
            }
        }

        /// Moves one test file or directory.
        #[allow(non_snake_case)]
        fn moveFile(&self, source: &str, destination: &str) -> HostResult<()> {
            if let Some(parent) = std::path::Path::new(destination).parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::rename(source, destination).map_err(HostError::from)
        }

        /// Copies one test file.
        #[allow(non_snake_case)]
        fn copyFile(&self, source: &str, destination: &str, recursive: bool) -> HostResult<()> {
            if recursive {
                return Err(Self::unsupported("recursive copyFile"));
            }
            if let Some(parent) = std::path::Path::new(destination).parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(source, destination)?;
            Ok(())
        }

        /// Creates one test directory.
        #[allow(non_snake_case)]
        fn makeDirectory(&self, path: &str, createParents: bool) -> HostResult<()> {
            if createParents {
                std::fs::create_dir_all(path)?;
            } else {
                std::fs::create_dir(path)?;
            }
            Ok(())
        }

        /// Rejects search because the embedded stream test does not search files.
        #[allow(non_snake_case)]
        fn findFiles(&self, _request: FindFilesRequest) -> HostResult<Vec<String>> {
            Err(Self::unsupported("findFiles"))
        }

        /// Returns metadata for one test path.
        #[allow(non_snake_case)]
        fn fileInfo(&self, path: &str) -> HostResult<FileInfo> {
            match std::fs::metadata(path) {
                Ok(metadata) => Ok(FileInfo {
                    path: path.to_string(),
                    exists: true,
                    fileType: if metadata.is_dir() {
                        "directory".to_string()
                    } else {
                        "file".to_string()
                    },
                    size: Self::host_size(metadata.len())?,
                    permissions: String::new(),
                    owner: String::new(),
                    group: String::new(),
                    lastModified: String::new(),
                    rawStatOutput: String::new(),
                }),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(FileInfo {
                    path: path.to_string(),
                    exists: false,
                    fileType: "missing".to_string(),
                    size: 0,
                    permissions: String::new(),
                    owner: String::new(),
                    group: String::new(),
                    lastModified: String::new(),
                    rawStatOutput: String::new(),
                }),
                Err(error) => Err(HostError::from(error)),
            }
        }

        /// Rejects grep because the embedded stream test does not search files.
        #[allow(non_snake_case)]
        fn grepCode(&self, _request: GrepCodeRequest) -> HostResult<GrepCodeResult> {
            Err(Self::unsupported("grepCode"))
        }

        /// Rejects archive creation because the embedded stream test does not archive files.
        #[allow(non_snake_case)]
        fn zipFiles(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Err(Self::unsupported("zipFiles"))
        }

        /// Rejects archive extraction because the embedded stream test does not archive files.
        #[allow(non_snake_case)]
        fn unzipFiles(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Err(Self::unsupported("unzipFiles"))
        }

        /// Rejects host opening because the embedded stream test does not open files.
        #[allow(non_snake_case)]
        fn openFile(&self, _path: &str) -> HostResult<()> {
            Err(Self::unsupported("openFile"))
        }

        /// Rejects host sharing because the embedded stream test does not share files.
        #[allow(non_snake_case)]
        fn shareFile(&self, _path: &str, _title: &str) -> HostResult<()> {
            Err(Self::unsupported("shareFile"))
        }
    }

    /// Creates a local proxy with isolated storage for TUI stream tests.
    fn test_local_proxy() -> Arc<LocalCoreProxy> {
        let mut hostManager = HostManager::withFileSystemHost(Arc::new(TestFileSystemHost));
        let sequence = TEST_RUNTIME_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "operit-tui-embedded-stream-{}-{}",
            std::process::id(),
            sequence
        ));
        let runtimeRoot = root.join(RUNTIME_ROOT_DIR_PATH);
        let workspaceRoot = root.join(WORKSPACE_DIR_PATH);
        std::fs::create_dir_all(&runtimeRoot).expect("test runtime root must be created");
        std::fs::create_dir_all(&workspaceRoot).expect("test workspace root must be created");
        setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(
            runtimeRoot.clone(),
            workspaceRoot.clone(),
        ));
        let storageHost = Arc::new(NativeRuntimeStorageHost::new(runtimeRoot, workspaceRoot));
        hostManager.runtimeStorageHost = Some(storageHost.clone());
        hostManager.runtimeSqliteHost = Some(storageHost);
        hostManager.hostJavaScriptRuntimeHost =
            Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
        hostManager.hostRuntimeTaskSchedulerHost =
            Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
        Arc::new(LocalCoreProxy::new(OperitApplication::newWithContext(
            hostManager,
        )))
    }

    /// Collects embedded content stream chunks using the same batched drain shape as the TUI loop.
    async fn collect_content_stream_text(tui: &mut TuiCore) -> String {
        let mut observed = Vec::new();
        let mut chunk_text = String::new();
        for _ in 0..100 {
            for event in tui.drainEvents() {
                if tui.mainChatContentStreamEventInfo(&event).is_none() {
                    observed.push(format!(
                        "{}:{:?}:{:?}",
                        event.propertyName, event.kind, event.requestId
                    ));
                    continue;
                }
                let event_kind = event.kind.clone();
                let markdown: MarkdownStreamEvent = operit_link::fromCoreValue(event.value)
                    .expect("content stream event must decode");
                if event_kind == CoreEventKind::Changed && markdown.eventType == "chunk" {
                    chunk_text.push_str(
                        markdown
                            .value
                            .as_deref()
                            .expect("chunk event must carry text"),
                    );
                }
                if event_kind == CoreEventKind::Completed {
                    return chunk_text;
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("TUI embedded content stream did not complete; observed={observed:?}");
    }

    /// Verifies TUI route permission errors retain the caller-facing diagnostics.
    #[test]
    fn tui_route_permission_error_keeps_structured_diagnostics() {
        let error = CoreLinkError::withDetails(
            "ROUTE_PERMISSION_DENIED",
            "Space route chatMessagesFlow requires capability chat.read on caller tui-client",
            CoreValue::Map(BTreeMap::from([
                (
                    "subject".to_string(),
                    CoreValue::String("caller".to_string()),
                ),
                (
                    "requiredCapability".to_string(),
                    CoreValue::String("chat.read".to_string()),
                ),
                (
                    "callerNodeId".to_string(),
                    CoreValue::String("tui-client".to_string()),
                ),
            ])),
        );
        let message = error.to_string();
        assert!(error.isRoutePermissionDenied());
        assert!(message.contains("ROUTE_PERMISSION_DENIED"));
        assert!(message.contains("caller"));
        assert!(message.contains("chat.read"));
        assert!(message.contains("tui-client"));
    }

    /// Verifies TUI opens ChatMessage.contentStream through the embedded Link protocol.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn tui_core_opens_embedded_content_stream_from_message_flow() {
        let mut tui = tui_core(test_local_proxy());
        let target_route = LocalCoreProxy::generatedTargetForSchema("chatRuntimeHolderMain")
            .expect("chat runtime object id must be generated");
        let chat_id = "tui-embedded-route-probe".to_string();
        let flow_args = CoreValue::Map(BTreeMap::from([
            ("chatId".to_string(), CoreValue::String(chat_id.clone())),
            (
                "streamText".to_string(),
                CoreValue::String("tui-embedded".to_string()),
            ),
        ]));
        let mut chat_proxy = tui.proxy.chat_runtime_holder_main();
        let mut flow_stream = chat_proxy
            .generatedClientMut()
            .watch(CoreWatchRequest::new(
                "tui-embedded-flow",
                target_route,
                "routeProbeChatMessagesFlow",
                flow_args,
            ))
            .await
            .expect("TUI route probe message Flow must open");
        let flow_event = flow_stream
            .recv()
            .await
            .expect("TUI route probe message Flow must emit");
        let messages: Vec<ChatMessage> =
            operit_link::fromCoreValue(flow_event.value).expect("message Flow must decode");

        assert_eq!(messages.len(), 1);
        let descriptor = messages[0]
            .contentStream
            .as_ref()
            .expect("message Flow must expose a content stream")
            .descriptor
            .clone();
        assert_eq!(descriptor.target, CORE_STREAM_TARGET);
        assert_eq!(descriptor.propertyName, "openCoreStream");

        tui.syncMainChatContentStreams(&messages)
            .await
            .expect("TUI content stream watch must open");
        let chunk_text = collect_content_stream_text(&mut tui).await;

        assert_eq!(chunk_text, "tui-embedded / chunk-one / chunk-two");
    }
}
