use std::collections::{HashMap, HashSet};

use std::sync::{Arc, Mutex};

use crate::core::chat::AIMessageManager::{
    logMessageTiming, messageTimingNow, AIMessageManager, BuildUserMessageContentRequest,
    SendMessageRequest as AIMessageSendRequest, StableContextWindowRequest,
};
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use crate::services::core::ChatHistoryDelegate::ChatHistoryDelegate;
use crate::services::core::MessageCoordinationDelegate::MessageCoordinationDelegate;
use crate::services::ChatServiceCore::CoreHandoffRuntimeSnapshot;
use crate::services::RuntimeHostInteractionService::{
    publishOwnerAppNotification, RuntimeHostInteractionAppNotificationPayload,
};
use crate::ui::features::chat::webview::workspace::WorkspaceBackupManager::WorkspaceBackupManager;
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_link::{
    CoreEvent, CoreEventKind, CoreEventStream, CoreLinkError, CoreRequestId, CoreStream,
    CoreStreamSource, CoreValue, CoreWatchRequest,
};
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_model::ChatHistory::ChatHistory;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatMessageDisplayMode::ChatMessageDisplayMode;
use operit_model::ChatMessageTimestampAllocator::ChatMessageTimestampAllocator;
use operit_model::ChatTurnOptions::ChatTurnOptions;
use operit_model::FunctionType::FunctionType;
use operit_model::InputProcessingState::InputProcessingState;
use operit_model::MessagePart::MessagePart;
use operit_model::MessagePartCodec::{AssistantMarkupStreamState, MessagePartCodec};
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_model::PromptTurn::PromptTurn;
use operit_providers::chat::llmprovider::AIService::SharedAiResponseStream;
use operit_providers::chat::EnhancedAIService::{
    EnhancedAIService, SendMessageCallbacks, SendMessageOptions,
};
use operit_store::PreferencesDataStore::{mutableStateFlow, MutableStateFlow, StateFlow};
use operit_tools::tools::ToolProgressBus::ToolProgressBus;
use operit_util::stream::HotStream::SharedStream;
use operit_util::stream::RevisableTextStream::{
    RenderableTextStream, ResponseStreamItem, RevisableTextStream,
};
use operit_util::stream::Stream::Stream;
use operit_util::stream::TextStreamRevisionTracker::TextStreamRevisionTracker;
use operit_util::AppLogger::AppLogger;
use operit_util::ChainLogger::{self, MESSAGE_STORE_CHAIN, RECEIVE_CHAIN, SEND_CHAIN};
use operit_util::MarkdownRenderStream::{MarkdownRenderEventStream, MarkdownStreamEvent};

/// Maximum text length used when preparing automatic speech previews.
pub const AUTO_READ_PREVIEW_MAX: usize = 48;

/// Creates a stable Core stream source for one physical AI response segment.
pub fn coreResponseStreamSource(
    stream: SharedAiResponseStream,
    streamKey: String,
) -> Arc<CoreStreamSource> {
    Arc::new(CoreStreamSource::new(move |request| {
        openCoreResponseStream(stream.clone(), streamKey.clone(), request)
    }))
}

/// Opens one physical AI response segment as Markdown Core events.
fn openCoreResponseStream(
    stream: SharedAiResponseStream,
    streamKey: String,
    request: CoreWatchRequest,
) -> Result<CoreEventStream, CoreLinkError> {
    let (sender, receiver) = CoreEventStream::channel();
    let initialContent = stream.initial_render_content();
    let mut orderedStream = stream;
    AppLogger::i(
        "CoreStreamTrace",
        &format!(
            "response.open requestId={} target={} property={} streamKey={} initialChars={}",
            request.requestId.0,
            request.targetObjectId,
            request.propertyName,
            streamKey,
            initialContent.chars().count()
        ),
    );
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "core-runtime-text-markdown",
            Box::new(move || {
                Box::pin(async move {
                    let mut markdownStream = MarkdownRenderEventStream::new(streamKey.clone());
                    let snapshotEvents = markdownStream.beginSnapshot(&initialContent);
                    for event in snapshotEvents {
                        sendCoreTextEvent(
                            &sender,
                            &request.requestId,
                            request.targetObjectId,
                            &request.propertyName,
                            CoreEventKind::Changed,
                            operit_link::toCoreValue(event)
                                .expect("MarkdownStreamEvent must serialize"),
                        );
                    }
                    let mut textRevisions = TextStreamRevisionTracker::new(&initialContent);
                    let mut itemCollector = |item: ResponseStreamItem| match item {
                        ResponseStreamItem::Chunk(chunk) => {
                            let _ = textRevisions.append(&chunk);
                            let events = markdownStream.pushChunk(&chunk);
                            for event in events {
                                sendCoreTextEvent(
                                    &sender,
                                    &request.requestId,
                                    request.targetObjectId,
                                    &request.propertyName,
                                    CoreEventKind::Changed,
                                    operit_link::toCoreValue(event)
                                        .expect("MarkdownStreamEvent must serialize"),
                                );
                            }
                        }
                        ResponseStreamItem::Revision(event) => {
                            let markdownEvent = match event.event_type {
                                operit_util::stream::RevisableTextStream::TextStreamEventType::Savepoint => {
                                    textRevisions.savepoint(&event.id);
                                    AppLogger::i(
                                        "CoreStreamTrace",
                                        &format!(
                                            "response.revision.savepoint streamKey={} id={}",
                                            streamKey, event.id
                                        ),
                                    );
                                    MarkdownStreamEvent::savepoint(streamKey.clone(), event.id)
                                }
                                operit_util::stream::RevisableTextStream::TextStreamEventType::Rollback => {
                                    let content = textRevisions.rollback(&event.id).expect(
                                        "markdown rollback must reference an active savepoint",
                                    );
                                    markdownStream.restoreContent(content);
                                    AppLogger::i(
                                        "CoreStreamTrace",
                                        &format!(
                                            "response.revision.rollback streamKey={} id={}",
                                            streamKey, event.id
                                        ),
                                    );
                                    MarkdownStreamEvent::rollback(streamKey.clone(), event.id)
                                }
                            };
                            sendCoreTextEvent(
                                &sender,
                                &request.requestId,
                                request.targetObjectId,
                                &request.propertyName,
                                CoreEventKind::Changed,
                                operit_link::toCoreValue(markdownEvent)
                                    .expect("MarkdownStreamEvent must serialize"),
                            );
                        }
                    };
                    orderedStream.collect_ordered(&mut itemCollector).await;
                    let completionValue = operit_link::toCoreValue(markdownStream.completed())
                        .expect("MarkdownStreamEvent must serialize");
                    AppLogger::i(
                        "CoreStreamTrace",
                        &format!("response.completed streamKey={}", streamKey),
                    );
                    sendCoreTextEvent(
                        &sender,
                        &request.requestId,
                        request.targetObjectId,
                        &request.propertyName,
                        CoreEventKind::Completed,
                        completionValue,
                    );
                })
            }),
        )
        .map_err(|error| CoreLinkError::internal(error.to_string()))?;
    Ok(receiver)
}

/// Sends one typed Markdown event through a Core stream channel.
fn sendCoreTextEvent(
    sender: &tokio::sync::mpsc::UnboundedSender<CoreEvent>,
    requestId: &CoreRequestId,
    targetObjectId: u32,
    propertyName: &str,
    kind: CoreEventKind,
    value: CoreValue,
) {
    if sender
        .send(CoreEvent {
            requestId: Some(requestId.clone()),
            targetObjectId,
            propertyName: propertyName.to_string(),
            kind,
            value,
        })
        .is_err()
    {
        AppLogger::e(
            "CoreStreamTrace",
            &format!(
                "response.event.receiver_closed requestId={} target={} property={}",
                requestId.0, targetObjectId, propertyName
            ),
        );
    }
}

/// Builds the localized host notice for a timed-out ToolPkg pre-send hook.
fn buildToolPkgHookTimeoutNotice(pluginIdentifier: String) -> String {
    format!("前置插件「{pluginIdentifier}」响应超时，已跳过并继续发送")
}

/// Per-chat runtime state for one active or recently active send turn.
#[derive(Clone, Debug)]
pub struct ChatRuntime {
    pub sendJob: Option<String>,
    pub responseStream: Option<SharedAiResponseStream>,
    pub streamCollectionJob: Option<String>,
    pub stateCollectionJob: Option<String>,
    pub currentTurnOptions: ChatTurnOptions,
    pub requestSentAt: i64,
    pub requestStartElapsed: i64,
    pub firstResponseElapsed: Option<i64>,
    pub isLoading: bool,
}

impl ChatRuntime {
    /// Creates an idle chat runtime state.
    pub fn new() -> Self {
        Self {
            sendJob: None,
            responseStream: None,
            streamCollectionJob: None,
            stateCollectionJob: None,
            currentTurnOptions: ChatTurnOptions::default(),
            requestSentAt: 0,
            requestStartElapsed: 0,
            firstResponseElapsed: None,
            isLoading: false,
        }
    }
}

/// Stores the observable lifecycle of one logical chat execution.
#[derive(Clone, Debug, PartialEq)]
pub struct ChatExecutionState {
    pub isLoading: bool,
    pub inputProcessingState: InputProcessingState,
}

impl ChatExecutionState {
    /// Creates an idle logical execution state.
    pub fn idle() -> Self {
        Self {
            isLoading: false,
            inputProcessingState: InputProcessingState::Idle,
        }
    }
}

/// Captures enough turn state to preserve or discard partial output during cancellation.
#[derive(Clone, Debug)]
pub struct TurnCancellationSnapshot {
    pub chatId: String,
    pub aiMessage: Option<ChatMessage>,
    pub partialContent: String,
    pub turnOptions: ChatTurnOptions,
}

/// Request data used to construct the user message sent to the chat model.
pub struct BuildUserMessageContentForSendRequest {
    pub messageText: String,
    pub proxySenderNameOverride: Option<String>,
    pub attachments: Vec<AttachmentInfo>,
    pub workspacePath: Option<String>,
    pub replyToMessage: Option<ChatMessage>,
    pub chatId: String,
    pub roleCardId: String,
    pub chatProviderIdOverride: Option<String>,
    pub chatModelIdOverride: Option<String>,
}

/// Request data used to construct a group-orchestration user message.
pub struct BuildUserMessageContentForGroupOrchestrationRequest {
    pub messageText: String,
    pub attachments: Vec<AttachmentInfo>,
    pub workspacePath: Option<String>,
    pub replyToMessage: Option<ChatMessage>,
    pub chatId: String,
    pub roleCardId: String,
}

/// End-to-end request for sending a user message through enhanced AI processing.
pub struct SendUserMessageProcessingRequest<'a> {
    pub enhancedAiService: &'a mut EnhancedAIService,
    pub chatHistoryDelegate: &'a mut ChatHistoryDelegate,
    pub chatId: String,
    pub messageText: String,
    pub chatHistory: Vec<ChatMessage>,
    pub promptHistoryOverride: Option<Vec<PromptTurn>>,
    pub workspacePath: Option<String>,
    pub promptFunctionType: PromptFunctionType,
    pub roleCardId: String,
    pub currentRoleName: Option<String>,
    pub characterName: Option<String>,
    pub avatarUri: Option<String>,
    pub attachments: Vec<AttachmentInfo>,
    pub replyToMessage: Option<ChatMessage>,
    pub enableThinking: bool,
    pub enableMemoryAutoUpdate: bool,
    pub maxTokens: i32,
    pub tokenUsageThreshold: f64,
    pub chatProviderIdOverride: Option<String>,
    pub chatModelIdOverride: Option<String>,
    pub isGroupOrchestrationTurn: bool,
    pub groupParticipantNamesText: Option<String>,
    pub proxySenderNameOverride: Option<String>,
    pub suppressUserMessageInHistory: bool,
    pub isAutoContinuation: bool,
    pub assistantMessageTimestamp: Option<i64>,
    pub executionGeneration: Option<i64>,
    pub executionSegmentIndex: Option<i64>,
    pub turnOptions: ChatTurnOptions,
}

/// Result returned after a user message send finishes and history is updated.
#[derive(Clone, Debug)]
pub struct SendUserMessageProcessingResult {
    pub aiMessage: ChatMessage,
    pub nextWindowSize: Option<i64>,
}

/// Request data used to regenerate one AI message variant.
pub struct RegenerateAiMessageVariantRequest<'a> {
    pub enhancedAiService: &'a mut EnhancedAIService,
    pub chatHistoryDelegate: &'a mut ChatHistoryDelegate,
    pub chatId: String,
    pub targetMessageTimestamp: i64,
    pub requestMessageContent: String,
    pub requestHistory: Vec<ChatMessage>,
    pub workspacePath: Option<String>,
    pub promptFunctionType: PromptFunctionType,
    pub roleCardId: String,
    pub currentRoleName: String,
    pub attachments: Vec<AttachmentInfo>,
    pub replyToMessage: Option<ChatMessage>,
    pub enableThinking: bool,
    pub enableMemoryAutoUpdate: bool,
    pub maxTokens: i32,
    pub tokenUsageThreshold: f64,
    pub chatProviderIdOverride: Option<String>,
    pub chatModelIdOverride: Option<String>,
}

/// Manages message send state, streaming persistence, cancellation, and UI flows.
pub struct MessageProcessingDelegate {
    pub functionalConfigManager: FunctionalConfigManager,
    pub modelConfigManager: ModelConfigManager,
    executionStateByChatIdFlow: MutableStateFlow<HashMap<String, ChatExecutionState>>,
    pub scrollToBottomEvent: Vec<()>,
    pub nonFatalErrorEvent: Vec<String>,
    pub nonFatalErrorEventFlow: MutableStateFlow<Option<String>>,
    pub toastEventFlow: MutableStateFlow<Option<String>>,
    pub turnCompleteCounterByChatId: HashMap<String, i64>,
    pub turnCompleteCounterByChatIdFlow: MutableStateFlow<HashMap<String, i64>>,
    pub currentTurnToolInvocationCountByChatId: HashMap<String, i32>,
    pub currentTurnToolInvocationCountByChatIdFlow: MutableStateFlow<HashMap<String, i32>>,
    pub chatRuntimes: Arc<Mutex<HashMap<String, ChatRuntime>>>,
    pub lastScrollEmitMsByChatKey: Arc<Mutex<HashMap<String, i64>>>,
    pub suppressIdleCompletedStateByChatId: Arc<Mutex<HashMap<String, bool>>>,
    pub pendingAsyncSummaryUiByChatId: Arc<Mutex<HashMap<String, bool>>>,
    pub speakMessageHandler: Option<fn(String, bool)>,
}

/// Bridges enhanced AI callbacks back into processing delegate state flows.
struct MessageProcessingCallbacks {
    nonFatalErrorEventFlow: MutableStateFlow<Option<String>>,
    chatId: String,
    chatHistoryDelegate: ChatHistoryDelegate,
}

impl SendMessageCallbacks for MessageProcessingCallbacks {
    /// Publishes non-fatal model/provider errors to observers.
    #[allow(non_snake_case)]
    fn onNonFatalError(&self, error: String) {
        self.nonFatalErrorEventFlow.set_value(Some(error));
    }

    /// Captures the opened chat flow state at the exact Core handoff boundary.
    #[allow(non_snake_case)]
    fn runtimeChatSnapshot(
        &self,
        assistantMessageTimestamp: Option<i64>,
        assistantContent: Option<String>,
    ) -> Option<CoreValue> {
        let messages = self
            .chatHistoryDelegate
            .openedRuntimeChatHistorySnapshot(&self.chatId)
            .ok()?
            .into_iter()
            .map(|mut message| {
                if Some(message.timestamp) == assistantMessageTimestamp {
                    message = assistantMessageWithHandoffProtocolSnapshot(
                        message,
                        assistantContent.as_deref(),
                    )
                    .ok()?;
                } else {
                    message.contentStream = None;
                }
                Some(message)
            })
            .collect::<Option<Vec<_>>>()?;
        if let Some(timestamp) = assistantMessageTimestamp {
            if let Some(message) = messages
                .iter()
                .find(|message| message.timestamp == timestamp)
            {
                AppLogger::i(
                    "CoreHandoffTrace",
                    &format!(
                        "runtime.snapshot chatId={} timestamp={} messages={} assistantContentChars={} parts={} displayChars={} protocolChars={}",
                        self.chatId,
                        timestamp,
                        messages.len(),
                        assistantContent
                            .as_ref()
                            .map(|content| content.chars().count())
                            .unwrap_or(0),
                        message.parts.len(),
                        message.displayText().chars().count(),
                        message.assistantProtocolMarkup().chars().count()
                    ),
                );
            }
        }
        operit_link::toCoreValue(CoreHandoffRuntimeSnapshot {
            chatId: self.chatId.clone(),
            messages,
        })
        .ok()
    }
}

/// Builds the assistant message snapshot that crosses a Core handoff boundary.
#[allow(non_snake_case)]
fn assistantMessageWithHandoffProtocolSnapshot(
    mut message: ChatMessage,
    assistantSegmentContent: Option<&str>,
) -> Result<ChatMessage, String> {
    let mut content = message.assistantProtocolMarkup();
    if let Some(segmentContent) = assistantSegmentContent {
        content.push_str(segmentContent);
    }
    message.parts = MessagePartCodec::parseAssistantMarkup(&content)?;
    message.contentStream = None;
    Ok(message)
}

impl MessageProcessingDelegate {
    /// Creates a processing delegate backed by the supplied config managers.
    pub fn new(
        functionalConfigManager: FunctionalConfigManager,
        modelConfigManager: ModelConfigManager,
    ) -> Self {
        Self {
            functionalConfigManager,
            modelConfigManager,
            executionStateByChatIdFlow: mutableStateFlow(HashMap::new()),
            scrollToBottomEvent: Vec::new(),
            nonFatalErrorEvent: Vec::new(),
            nonFatalErrorEventFlow: mutableStateFlow(None),
            toastEventFlow: mutableStateFlow(None),
            turnCompleteCounterByChatId: HashMap::new(),
            turnCompleteCounterByChatIdFlow: mutableStateFlow(HashMap::new()),
            currentTurnToolInvocationCountByChatId: HashMap::new(),
            currentTurnToolInvocationCountByChatIdFlow: mutableStateFlow(HashMap::new()),
            chatRuntimes: Arc::new(Mutex::new(HashMap::new())),
            lastScrollEmitMsByChatKey: Arc::new(Mutex::new(HashMap::new())),
            suppressIdleCompletedStateByChatId: Arc::new(Mutex::new(HashMap::new())),
            pendingAsyncSummaryUiByChatId: Arc::new(Mutex::new(HashMap::new())),
            speakMessageHandler: None,
        }
    }

    /// Clones the delegate for use by another service core while sharing runtime state flows.
    #[allow(non_snake_case)]
    pub fn clone_for_core(&self) -> Self {
        let rootDir = ApiPreferences::data_dir();
        Self {
            functionalConfigManager: FunctionalConfigManager::new(rootDir.clone()),
            modelConfigManager: ModelConfigManager::new(rootDir),
            executionStateByChatIdFlow: self.executionStateByChatIdFlow.clone(),
            scrollToBottomEvent: self.scrollToBottomEvent.clone(),
            nonFatalErrorEvent: self.nonFatalErrorEvent.clone(),
            nonFatalErrorEventFlow: self.nonFatalErrorEventFlow.clone(),
            toastEventFlow: self.toastEventFlow.clone(),
            turnCompleteCounterByChatId: self.turnCompleteCounterByChatIdFlow.value(),
            turnCompleteCounterByChatIdFlow: self.turnCompleteCounterByChatIdFlow.clone(),
            currentTurnToolInvocationCountByChatId: self
                .currentTurnToolInvocationCountByChatIdFlow
                .value(),
            currentTurnToolInvocationCountByChatIdFlow: self
                .currentTurnToolInvocationCountByChatIdFlow
                .clone(),
            chatRuntimes: self.chatRuntimes.clone(),
            lastScrollEmitMsByChatKey: self.lastScrollEmitMsByChatKey.clone(),
            suppressIdleCompletedStateByChatId: self.suppressIdleCompletedStateByChatId.clone(),
            pendingAsyncSummaryUiByChatId: self.pendingAsyncSummaryUiByChatId.clone(),
            speakMessageHandler: self.speakMessageHandler,
        }
    }

    /// Builds the initial visible title used while title generation is in progress.
    #[allow(non_snake_case)]
    pub fn provisionalConversationTitle(attachments: &[AttachmentInfo]) -> String {
        attachments
            .first()
            .and_then(|attachment| {
                let file_name = attachment.fileName.trim();
                (!file_name.is_empty()).then_some(file_name.to_string())
            })
            .unwrap_or_else(|| "New Chat".to_string())
    }

    /// Schedules title generation without delaying the primary chat response.
    #[allow(non_snake_case)]
    pub fn launchConversationTitleGeneration(
        mut enhanced_ai_service: EnhancedAIService,
        mut chat_history_delegate: ChatHistoryDelegate,
        chat_id: String,
        user_text: String,
        attachments: Vec<AttachmentInfo>,
        provisional_title: String,
    ) {
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "conversation-title-generation",
                Box::new(move || {
                    Box::pin(async move {
                        match enhanced_ai_service
                            .generateConversationTitle(
                                user_text,
                                attachments
                                    .into_iter()
                                    .map(|attachment| attachment.fileName)
                                    .collect(),
                            )
                            .await
                        {
                            Ok(generated_title) => {
                                let current_title = chat_history_delegate
                                    .chatHistoriesFlow()
                                    .value()
                                    .into_iter()
                                    .find(|history| history.id == chat_id)
                                    .map(|history| history.title);
                                if !generated_title.is_empty()
                                    && current_title.as_deref() == Some(provisional_title.as_str())
                                {
                                    chat_history_delegate.updateChatTitle(chat_id, generated_title);
                                }
                            }
                            Err(error) => {
                                AppLogger::e(
                                    "MessageProcessingDelegate",
                                    &format!("conversation title generation failed: {error}"),
                                );
                            }
                        }
                    })
                }),
            )
            .expect("conversation title generation task must be scheduled by the Host");
    }

    /// Emits a toast message to UI observers.
    #[allow(non_snake_case)]
    pub fn showToast(&mut self, message: String) {
        self.toastEventFlow.set_value(Some(message));
    }

    /// Emits a non-fatal error event and stores it in the local event list.
    #[allow(non_snake_case)]
    pub fn emitNonFatalError(&mut self, message: String) {
        self.nonFatalErrorEvent.push(message.clone());
        self.nonFatalErrorEventFlow.set_value(Some(message));
    }

    /// Returns the observable non-fatal error event flow.
    #[allow(non_snake_case)]
    pub fn nonFatalErrorEventFlow(&self) -> StateFlow<Option<String>> {
        self.nonFatalErrorEventFlow.asStateFlow()
    }

    /// Clears the active toast event.
    #[allow(non_snake_case)]
    pub fn clearToastEvent(&mut self) {
        self.toastEventFlow.set_value(None);
    }

    /// Returns the observable toast event flow.
    #[allow(non_snake_case)]
    pub fn toastEventFlow(&self) -> StateFlow<Option<String>> {
        self.toastEventFlow.asStateFlow()
    }

    /// Builds a compact single-line preview for spoken message output.
    #[allow(non_snake_case)]
    pub fn speechPreview(text: String) -> String {
        text.replace('\n', "\\n")
            .chars()
            .take(AUTO_READ_PREVIEW_MAX)
            .collect()
    }

    /// Maps an optional chat id to the runtime map key used by state flows.
    #[allow(non_snake_case)]
    pub fn chatKey(chatId: Option<String>) -> String {
        chatId.unwrap_or_else(|| "__DEFAULT_CHAT__".to_string())
    }

    /// Emits a scroll-to-bottom event for a chat and records the emission time.
    #[allow(non_snake_case)]
    pub fn tryEmitScrollToBottomThrottled(&mut self, chatId: Option<String>) {
        let key = Self::chatKey(chatId);
        self.lastScrollEmitMsByChatKey
            .lock()
            .expect("last scroll emit map mutex poisoned")
            .insert(key, messageTimingNow().startedAtMs as i64);
        self.scrollToBottomEvent.push(());
    }

    /// Emits a scroll-to-bottom event regardless of recent scroll emissions.
    #[allow(non_snake_case)]
    pub fn forceEmitScrollToBottom(&mut self, chatId: Option<String>) {
        let key = Self::chatKey(chatId);
        self.lastScrollEmitMsByChatKey
            .lock()
            .expect("last scroll emit map mutex poisoned")
            .insert(key, messageTimingNow().startedAtMs as i64);
        self.scrollToBottomEvent.push(());
    }

    /// Looks up or creates a runtime state entry and applies an action to it.
    #[allow(non_snake_case)]
    fn withRuntime<R>(
        &self,
        chatId: Option<String>,
        action: impl FnOnce(&mut ChatRuntime) -> R,
    ) -> R {
        let key = Self::chatKey(chatId);
        let mut runtimes = self
            .chatRuntimes
            .lock()
            .expect("chat runtimes mutex poisoned");
        action(runtimes.entry(key).or_insert_with(ChatRuntime::new))
    }

    /// Applies an action only when a runtime state entry already exists.
    #[allow(non_snake_case)]
    fn withExistingRuntime<R>(
        &self,
        chatId: Option<String>,
        action: impl FnOnce(&mut ChatRuntime) -> R,
    ) -> Option<R> {
        let key = Self::chatKey(chatId);
        let mut runtimes = self
            .chatRuntimes
            .lock()
            .expect("chat runtimes mutex poisoned");
        runtimes.get_mut(&key).map(action)
    }

    /// Clones the active provider response stream for internal runtime coordination.
    #[allow(non_snake_case)]
    pub(crate) fn activeResponseStreamForChat(
        &self,
        chatId: String,
    ) -> Option<SharedAiResponseStream> {
        self.withExistingRuntime(Some(chatId), |runtime| runtime.responseStream.clone())
            .flatten()
    }

    /// Updates one chat's observable logical execution state in a single publication.
    #[allow(non_snake_case)]
    fn updateChatExecutionState(
        &self,
        chatId: String,
        update: impl FnOnce(&mut ChatExecutionState),
    ) {
        let key = Self::chatKey(Some(chatId));
        let mut states = self.executionStateByChatIdFlow.value();
        update(states.entry(key).or_insert_with(ChatExecutionState::idle));
        self.executionStateByChatIdFlow.set_value(states);
    }

    /// Recomputes the chat ids that currently own active streaming turns.
    #[allow(non_snake_case)]
    pub fn updateActiveStreamingChatIds(&mut self) {
        let activeStreamingChatIds = {
            let runtimes = self
                .chatRuntimes
                .lock()
                .expect("chat runtimes mutex poisoned");
            runtimes
                .iter()
                .filter(|(key, runtime)| key.as_str() != "__DEFAULT_CHAT__" && runtime.isLoading)
                .map(|(key, _)| key.clone())
                .collect::<HashSet<_>>()
        };
        let mut states = self.executionStateByChatIdFlow.value();
        for (chatId, state) in states.iter_mut() {
            state.isLoading = activeStreamingChatIds.contains(chatId);
        }
        for chatId in activeStreamingChatIds {
            states
                .entry(chatId)
                .or_insert_with(ChatExecutionState::idle)
                .isLoading = true;
        }
        self.executionStateByChatIdFlow.set_value(states);
    }

    /// Refreshes active streaming chat ids from the current runtime map.
    #[allow(non_snake_case)]
    pub fn refreshActiveStreamingChatIds(&mut self) {
        self.updateActiveStreamingChatIds();
    }

    /// Returns whether an input-processing state represents an inactive terminal state.
    #[allow(non_snake_case)]
    pub fn isTerminalInputState(state: &InputProcessingState) -> bool {
        matches!(
            state,
            InputProcessingState::Idle | InputProcessingState::Completed
        )
    }

    /// Updates the observable input-processing state for a chat.
    #[allow(non_snake_case)]
    pub fn setChatInputProcessingState(
        &mut self,
        chatId: Option<String>,
        state: InputProcessingState,
    ) {
        if let Some(chatId) = chatId.as_ref() {
            if self.withRuntime(Some(chatId.clone()), |runtime| runtime.isLoading)
                && Self::isTerminalInputState(&state)
            {
                return;
            }
            let suppressIdleCompleted = self
                .suppressIdleCompletedStateByChatId
                .lock()
                .expect("suppress idle completed map mutex poisoned")
                .contains_key(chatId);
            if suppressIdleCompleted && Self::isTerminalInputState(&state) {
                return;
            }
        }
        if !matches!(
            state,
            InputProcessingState::ExecutingTool { .. } | InputProcessingState::Summarizing { .. }
        ) {
            ToolProgressBus::clear();
        }
        let key = Self::chatKey(chatId);
        let mut states = self.executionStateByChatIdFlow.value();
        states
            .entry(key)
            .or_insert_with(ChatExecutionState::idle)
            .inputProcessingState = state;
        self.executionStateByChatIdFlow.set_value(states);
    }

    /// Enables or clears suppression of idle/completed UI state for one chat.
    #[allow(non_snake_case)]
    pub fn setSuppressIdleCompletedStateForChat(&mut self, chatId: String, suppress: bool) {
        let mut states = self
            .suppressIdleCompletedStateByChatId
            .lock()
            .expect("suppress idle completed map mutex poisoned");
        if suppress {
            states.insert(chatId, true);
        } else {
            states.remove(&chatId);
        }
    }

    /// Marks whether a send-triggered summary is pending for one chat.
    #[allow(non_snake_case)]
    pub fn setPendingAsyncSummaryUiForChat(&mut self, chatId: String, pending: bool) {
        let mut states = self
            .pendingAsyncSummaryUiByChatId
            .lock()
            .expect("pending async summary map mutex poisoned");
        if pending {
            states.insert(chatId, true);
        } else {
            states.remove(&chatId);
        }
    }

    /// Updates input-processing state for a concrete chat id.
    #[allow(non_snake_case)]
    pub fn setInputProcessingStateForChat(&mut self, chatId: String, state: InputProcessingState) {
        self.setChatInputProcessingState(Some(chatId), state);
    }

    /// Publishes the beginning of one logical chat execution atomically.
    #[allow(non_snake_case)]
    fn startChatExecution(&mut self, chatId: String) {
        ToolProgressBus::clear();
        self.updateChatExecutionState(chatId, |state| {
            state.isLoading = true;
            state.inputProcessingState = InputProcessingState::Processing {
                message: "message_processing".to_string(),
            };
        });
    }

    /// Publishes the terminal state of one logical chat execution atomically.
    #[allow(non_snake_case)]
    pub fn finishChatExecution(&mut self, chatId: String, terminalState: InputProcessingState) {
        debug_assert!(matches!(
            &terminalState,
            InputProcessingState::Idle
                | InputProcessingState::Completed
                | InputProcessingState::Error { .. }
        ));
        ToolProgressBus::clear();
        self.updateChatExecutionState(chatId, |state| {
            state.isLoading = false;
            state.inputProcessingState = terminalState;
        });
    }

    /// Builds the user message payload used by group orchestration turns.
    #[allow(non_snake_case)]
    pub fn buildUserMessageContentForGroupOrchestration(
        &self,
        request: BuildUserMessageContentForGroupOrchestrationRequest,
    ) -> Result<String, operit_providers::chat::llmprovider::AIService::AiServiceError> {
        self.buildUserMessageContentForSend(BuildUserMessageContentForSendRequest {
            messageText: request.messageText,
            proxySenderNameOverride: None,
            attachments: request.attachments,
            workspacePath: request.workspacePath,
            replyToMessage: request.replyToMessage,
            chatId: request.chatId,
            roleCardId: request.roleCardId,
            chatProviderIdOverride: None,
            chatModelIdOverride: None,
        })
    }

    /// Builds model-ready user message content with attachments, workspace, and reply context.
    #[allow(non_snake_case)]
    pub fn buildUserMessageContentForSend(
        &self,
        request: BuildUserMessageContentForSendRequest,
    ) -> Result<String, operit_providers::chat::llmprovider::AIService::AiServiceError> {
        let (providerId, modelId) = match (
            request.chatProviderIdOverride.as_ref(),
            request.chatModelIdOverride.as_ref(),
        ) {
            (Some(providerId), Some(modelId))
                if !providerId.trim().is_empty() && !modelId.trim().is_empty() =>
            {
                (providerId.clone(), modelId.clone())
            }
            (None, None) => {
                let binding = self
                    .functionalConfigManager
                    .getModelBindingForFunction(FunctionType::CHAT)
                    .map_err(|error| {
                        operit_providers::chat::llmprovider::AIService::AiServiceError::RequestFailed(
                            error.to_string(),
                        )
                    })?;
                (binding.providerId, binding.modelId)
            }
            _ => {
                return Err(
                    operit_providers::chat::llmprovider::AIService::AiServiceError::RequestFailed(
                        "chat provider and model override must be set together".to_string(),
                    ),
                );
            }
        };

        let loadModelConfigStartTime = messageTimingNow();
        let currentModelConfig = self
            .modelConfigManager
            .getResolvedModelConfig(&providerId, &modelId)
            .map_err(|error| {
                operit_providers::chat::llmprovider::AIService::AiServiceError::RequestFailed(
                    error.to_string(),
                )
            })?;
        let enableDirectImageProcessing = currentModelConfig.capabilities.directImage;
        let enableDirectAudioProcessing = currentModelConfig.capabilities.directAudio;
        let enableDirectVideoProcessing = currentModelConfig.capabilities.directVideo;
        logMessageTiming(
            "delegate.loadModelConfig",
            loadModelConfigStartTime,
            Some(format!("chatId={}, modelId={modelId}", request.chatId)),
        );

        let buildUserMessageStartTime = messageTimingNow();
        let nonFatalErrorEventFlow = self.nonFatalErrorEventFlow.clone();
        let onHookTimeout = Arc::new(move |pluginIdentifier: String| {
            nonFatalErrorEventFlow.set_value(Some(buildToolPkgHookTimeoutNotice(pluginIdentifier)));
        });
        let finalMessageContent =
            AIMessageManager::buildUserMessageContent(BuildUserMessageContentRequest {
                messageText: request.messageText,
                proxySenderName: request.proxySenderNameOverride,
                attachments: request.attachments,
                workspacePath: request.workspacePath,
                replyToMessage: request.replyToMessage,
                enableDirectImageProcessing,
                enableDirectAudioProcessing,
                enableDirectVideoProcessing,
                chatId: Some(request.chatId.clone()),
                roleCardId: Some(request.roleCardId),
                onHookTimeout: Some(onHookTimeout),
            });
        logMessageTiming(
            "delegate.buildUserMessageContent",
            buildUserMessageStartTime,
            Some(format!(
                "chatId={}, finalLength={}",
                request.chatId,
                finalMessageContent.len()
            )),
        );
        Ok(finalMessageContent)
    }

    /// Runs an action while attaching timing metrics to the current turn.
    #[allow(non_snake_case)]
    pub fn withTurnMetrics(
        mut aiMessage: ChatMessage,
        requestSentAt: i64,
        requestStartElapsed: i64,
        firstResponseElapsed: Option<i64>,
        completedElapsed: i64,
    ) -> ChatMessage {
        aiMessage.sentAt = requestSentAt;
        aiMessage.waitDurationMs = firstResponseElapsed
            .map(|first| first - requestStartElapsed)
            .unwrap_or(0);
        aiMessage.outputDurationMs = firstResponseElapsed
            .map(|first| completedElapsed - first)
            .unwrap_or(0);
        aiMessage.completedAt = completedElapsed;
        aiMessage
    }

    /// Reads the latest cancellation snapshot for a chat's active turn.
    #[allow(non_snake_case)]
    pub fn readCurrentTurnCancellationSnapshot(
        &self,
        chatId: String,
    ) -> Option<TurnCancellationSnapshot> {
        self.withExistingRuntime(Some(chatId.clone()), |runtime| TurnCancellationSnapshot {
            chatId,
            aiMessage: None,
            partialContent: runtime
                .responseStream
                .as_ref()
                .map(|stream| stream.replay_cache().join(""))
                .unwrap_or_default(),
            turnOptions: runtime.currentTurnOptions.clone(),
        })
    }

    /// Removes and returns the active streaming AI message for a chat.
    #[allow(non_snake_case)]
    pub fn detachStreamingAiMessage(&mut self, chatId: String) -> Option<ChatMessage> {
        let snapshot = self.readCurrentTurnCancellationSnapshot(chatId)?;
        snapshot.aiMessage
    }

    /// Cancels an active message turn and optionally keeps partial response content.
    #[allow(non_snake_case)]
    pub async fn cancelMessageInternal(&mut self, chatId: String, keepPartialResponse: bool) {
        if !keepPartialResponse {
            self.detachStreamingAiMessage(chatId.clone());
        }
        self.clearCurrentTurnToolInvocationCount(chatId.clone());
        AIMessageManager::cancelOperation(chatId.clone()).await;
        self.withExistingRuntime(Some(chatId.clone()), |runtime| {
            if let Some(responseStream) = runtime.responseStream.as_ref() {
                responseStream.close();
            }
            runtime.isLoading = false;
            runtime.responseStream = None;
            runtime.sendJob = None;
            runtime.streamCollectionJob = None;
            runtime.stateCollectionJob = None;
            runtime.currentTurnOptions = ChatTurnOptions::default();
            runtime.requestSentAt = 0;
            runtime.requestStartElapsed = 0;
            runtime.firstResponseElapsed = None;
        });
        self.finishChatExecution(chatId, InputProcessingState::Idle);
    }

    /// Cancels an active message turn while preserving partial response content.
    #[allow(non_snake_case)]
    pub async fn cancelMessage(&mut self, chatId: String) {
        self.cancelMessageInternal(chatId, true).await;
    }

    /// Cancels an active message turn before destructive history mutation.
    #[allow(non_snake_case)]
    pub async fn cancelMessageForDestructiveMutation(&mut self, chatId: String) {
        self.cancelMessageInternal(chatId, false).await;
    }

    /// Returns the observable set of chat ids with active streaming turns.
    pub fn activeStreamingChatIdsFlow(&self) -> StateFlow<HashSet<String>> {
        self.executionStateByChatIdFlow.asStateFlow().map(|states| {
            states
                .into_iter()
                .filter_map(|(chatId, state)| {
                    (chatId != "__DEFAULT_CHAT__" && state.isLoading).then_some(chatId)
                })
                .collect()
        })
    }

    /// Returns the observable input-processing state map.
    pub fn inputProcessingStateByChatIdFlow(
        &self,
    ) -> StateFlow<HashMap<String, InputProcessingState>> {
        self.executionStateByChatIdFlow.asStateFlow().map(|states| {
            states
                .into_iter()
                .map(|(chatId, state)| (chatId, state.inputProcessingState))
                .collect()
        })
    }

    /// Returns the observable logical execution state map.
    #[allow(non_snake_case)]
    pub fn executionStateByChatIdFlow(&self) -> StateFlow<HashMap<String, ChatExecutionState>> {
        self.executionStateByChatIdFlow.asStateFlow()
    }

    /// Returns the observable turn-completion counter map.
    pub fn turnCompleteCounterByChatIdFlow(&self) -> StateFlow<HashMap<String, i64>> {
        self.turnCompleteCounterByChatIdFlow.asStateFlow()
    }

    /// Returns the observable current-turn tool invocation count map.
    pub fn currentTurnToolInvocationCountByChatIdFlow(&self) -> StateFlow<HashMap<String, i32>> {
        self.currentTurnToolInvocationCountByChatIdFlow
            .asStateFlow()
    }

    /// Emits a scroll-to-bottom event for the default chat target.
    #[allow(non_snake_case)]
    pub fn scrollToBottom(&mut self) {
        self.forceEmitScrollToBottom(None);
    }

    /// Returns the completion counter for one chat.
    #[allow(non_snake_case)]
    pub fn getTurnCompleteCounter(&self, chatId: String) -> i64 {
        *self
            .turnCompleteCounterByChatIdFlow
            .value()
            .get(&chatId)
            .unwrap_or(&0)
    }

    /// Reports whether one chat currently has a loading runtime.
    #[allow(non_snake_case)]
    pub fn isChatLoading(&self, chatId: String) -> bool {
        self.withExistingRuntime(Some(chatId), |runtime| runtime.isLoading)
            .unwrap_or(false)
    }

    /// Installs the callback used to speak assistant messages.
    #[allow(non_snake_case)]
    pub fn setSpeakMessageHandler(&mut self, handler: fn(String, bool)) {
        self.speakMessageHandler = Some(handler);
    }

    /// Resets current-turn tool invocation count for one chat.
    #[allow(non_snake_case)]
    pub fn resetCurrentTurnToolInvocationCount(&mut self, chatId: String) {
        let mut counts = self.currentTurnToolInvocationCountByChatIdFlow.value();
        counts.insert(chatId, 0);
        self.currentTurnToolInvocationCountByChatId = counts.clone();
        self.currentTurnToolInvocationCountByChatIdFlow
            .set_value(counts);
    }

    /// Increments current-turn tool invocation count for one chat.
    #[allow(non_snake_case)]
    pub fn incrementCurrentTurnToolInvocationCount(&mut self, chatId: String) {
        let mut counts = self.currentTurnToolInvocationCountByChatIdFlow.value();
        let value = counts.get(&chatId).copied().unwrap_or(0) + 1;
        counts.insert(chatId, value);
        self.currentTurnToolInvocationCountByChatId = counts.clone();
        self.currentTurnToolInvocationCountByChatIdFlow
            .set_value(counts);
    }

    /// Clears current-turn tool invocation count for one chat.
    #[allow(non_snake_case)]
    pub fn clearCurrentTurnToolInvocationCount(&mut self, chatId: String) {
        let mut counts = self.currentTurnToolInvocationCountByChatIdFlow.value();
        counts.remove(&chatId);
        self.currentTurnToolInvocationCountByChatId = counts.clone();
        self.currentTurnToolInvocationCountByChatIdFlow
            .set_value(counts);
    }

    /// Sends a user message, streams the AI response, persists history, and updates UI state.
    #[allow(non_snake_case)]
    pub async fn sendUserMessage(
        &mut self,
        mut request: SendUserMessageProcessingRequest<'_>,
    ) -> Result<
        SendUserMessageProcessingResult,
        operit_providers::chat::llmprovider::AIService::AiServiceError,
    > {
        let chatId = request.chatId.clone();
        let originalMessageText = request.messageText.trim().to_string();
        let continuationAssistantMessage = match request.assistantMessageTimestamp {
            Some(timestamp) => {
                let message = request
                    .chatHistory
                    .iter()
                    .find(|message| message.timestamp == timestamp)
                    .cloned()
                    .ok_or_else(|| {
                        operit_providers::chat::llmprovider::AIService::AiServiceError::RequestFailed(
                            format!(
                                "response continuation assistant message does not exist: {timestamp}"
                            ),
                        )
                    })?;
                if message.sender != "ai" {
                    return Err(
                        operit_providers::chat::llmprovider::AIService::AiServiceError::RequestFailed(
                            format!(
                                "response continuation timestamp is not an assistant message: {timestamp}"
                            ),
                        ),
                    );
                }
                Some(message)
            }
            None => None,
        };
        let continuationContentPrefix = match continuationAssistantMessage.as_ref() {
            Some(message) => message.assistantProtocolMarkup(),
            None => String::new(),
        };
        let isResponseExecutionContinuation = continuationAssistantMessage.is_some();
        AppLogger::i(
            "CoreSend",
            &format!(
                "processing start chatId={} messageChars={} attachments={}",
                chatId,
                originalMessageText.chars().count(),
                request.attachments.len()
            ),
        );
        ChainLogger::info(
            SEND_CHAIN,
            "send.processing.start",
            &[
                ("chatId", chatId.clone()),
                ("messageChars", ChainLogger::lenField(&originalMessageText)),
                ("attachments", request.attachments.len().to_string()),
                (
                    "suppressUserMessage",
                    ChainLogger::boolField(request.suppressUserMessageInHistory),
                ),
                (
                    "groupOrchestration",
                    ChainLogger::boolField(request.isGroupOrchestrationTurn),
                ),
            ],
        );
        self.resetCurrentTurnToolInvocationCount(chatId.clone());
        self.withRuntime(Some(chatId.clone()), |runtime| {
            runtime.currentTurnOptions = request.turnOptions.clone();
            runtime.requestSentAt = messageTimingNow().startedAtMs as i64;
            runtime.requestStartElapsed = messageTimingNow().startedAtMs as i64;
            runtime.firstResponseElapsed = None;
            runtime.isLoading = true;
            runtime.responseStream = None;
        });
        self.startChatExecution(chatId.clone());

        let finalMessageContent =
            match self.buildUserMessageContentForSend(BuildUserMessageContentForSendRequest {
                messageText: originalMessageText.clone(),
                proxySenderNameOverride: request.proxySenderNameOverride.clone(),
                attachments: request.attachments.clone(),
                workspacePath: request.workspacePath.clone(),
                replyToMessage: request.replyToMessage.clone(),
                chatId: chatId.clone(),
                roleCardId: request.roleCardId.clone(),
                chatProviderIdOverride: request.chatProviderIdOverride.clone(),
                chatModelIdOverride: request.chatModelIdOverride.clone(),
            }) {
                Ok(content) => content,
                Err(error) => {
                    ChainLogger::error(
                        SEND_CHAIN,
                        "send.processing.build_user_content.error",
                        &[("chatId", chatId.clone()), ("error", error.to_string())],
                    );
                    self.withExistingRuntime(Some(chatId.clone()), |runtime| {
                        runtime.isLoading = false;
                        runtime.responseStream = None;
                        runtime.sendJob = None;
                        runtime.streamCollectionJob = None;
                        runtime.stateCollectionJob = None;
                    });
                    self.finishChatExecution(
                        chatId.clone(),
                        InputProcessingState::Error {
                            message: error.to_string(),
                        },
                    );
                    return Err(error);
                }
            };
        let shouldAddUserMessageToChat = request.turnOptions.persistTurn
            && !request.suppressUserMessageInHistory
            && !(request.isAutoContinuation
                && originalMessageText.is_empty()
                && request.attachments.is_empty())
            && !(request.isGroupOrchestrationTurn
                && originalMessageText.is_empty()
                && request.attachments.is_empty());
        let isFirstMessage = !request.chatHistoryDelegate.hasUserMessage(chatId.clone());
        let provisionalTitle = if request.turnOptions.persistTurn && isFirstMessage {
            let title = Self::provisionalConversationTitle(&request.attachments);
            request
                .chatHistoryDelegate
                .updateChatTitle(chatId.clone(), title.clone());
            Some(title)
        } else {
            None
        };
        let mut userMessageAdded = false;
        let mut userMessage = ChatMessage {
            sender: "user".to_string(),
            parts: vec![MessagePart::markdown(
                "part-0".to_string(),
                0,
                finalMessageContent.clone(),
            )],
            roleName: "user".to_string(),
            displayMode: if request.turnOptions.hideUserMessage {
                ChatMessageDisplayMode::HIDDEN_PLACEHOLDER
            } else {
                ChatMessageDisplayMode::NORMAL
            },
            ..ChatMessage::new("user".to_string())
        };
        let mut workspaceToolHookSession = None;
        let mut workspaceToolHookHandler = request.enhancedAiService.tool_handler.clone();
        if let Some(workspacePath) = request
            .workspacePath
            .clone()
            .filter(|path| !path.trim().is_empty())
        {
            let session =
                WorkspaceBackupManager::getInstance(workspaceToolHookHandler.getContext())
                    .createWorkspaceToolHookSession(
                        workspacePath,
                        userMessage.timestamp,
                        Some(chatId.clone()),
                    );
            workspaceToolHookHandler.addToolHook(session.clone());
            workspaceToolHookSession = Some(session);
        }
        if shouldAddUserMessageToChat {
            ChainLogger::info(
                MESSAGE_STORE_CHAIN,
                "message.store.user.start",
                &[
                    ("chatId", chatId.clone()),
                    ("timestamp", userMessage.timestamp.to_string()),
                    (
                        "contentChars",
                        userMessage.displayText().chars().count().to_string(),
                    ),
                ],
            );
            request
                .chatHistoryDelegate
                .addMessageToChat(userMessage.clone(), Some(chatId.clone()));
            ChainLogger::info(
                MESSAGE_STORE_CHAIN,
                "message.store.user.done",
                &[
                    ("chatId", chatId.clone()),
                    ("timestamp", userMessage.timestamp.to_string()),
                ],
            );
            userMessageAdded = true;
            if let Some(provisionalTitle) = provisionalTitle {
                Self::launchConversationTitleGeneration(
                    request.enhancedAiService.clone(),
                    request.chatHistoryDelegate.clone_for_core(),
                    chatId.clone(),
                    originalMessageText.clone(),
                    request.attachments.clone(),
                    provisionalTitle,
                );
            }
        }
        request
            .enhancedAiService
            .setInputProcessingState(InputProcessingState::Processing {
                message: "message_processing".to_string(),
            });
        {
            let activeChatId = chatId.clone();
            let mut stateDelegate = self.clone_for_core();
            let stateFlow = request.enhancedAiService.inputProcessingState();
            stateFlow.subscribe(move |state| {
                stateDelegate.setInputProcessingStateForChat(activeChatId.clone(), state);
            });
        }

        let characterName = CharacterCardManager::getInstance()
            .getCharacterCard(&request.roleCardId)
            .ok()
            .map(|card| card.name)
            .filter(|name| !name.trim().is_empty());
        let currentRoleName = characterName
            .clone()
            .unwrap_or_else(|| "Operit".to_string());
        let requestMessageContent = if request.isGroupOrchestrationTurn
            && !finalMessageContent.trim_start().is_empty()
            && !finalMessageContent.trim_start().starts_with("[From user]")
        {
            format!("[From user]\n{}", finalMessageContent)
        } else {
            finalMessageContent
        };
        AppLogger::i(
            "CoreSend",
            &format!("response stream create start chatId={}", chatId),
        );
        let assistantMessageTimestamp = continuationAssistantMessage
            .as_ref()
            .map(|message| message.timestamp)
            .unwrap_or_else(ChatMessageTimestampAllocator::next);
        let completionStream = match AIMessageManager::sendMessage(AIMessageSendRequest {
            enhancedAiService: request.enhancedAiService,
            chatId: Some(chatId.clone()),
            messageContent: requestMessageContent,
            chatHistory: request.chatHistory,
            promptHistoryOverride: request.promptHistoryOverride.clone(),
            workspacePath: request.workspacePath.clone(),
            promptFunctionType: request.promptFunctionType.clone(),
            enableThinking: request.enableThinking,
            enableMemoryAutoUpdate: request.enableMemoryAutoUpdate,
            maxTokens: request.maxTokens,
            tokenUsageThreshold: request.tokenUsageThreshold,
            characterName: characterName.clone(),
            avatarUri: request.avatarUri,
            roleCardId: request.roleCardId.clone(),
            currentRoleName: Some(currentRoleName.clone()),
            splitHistoryByRole: true,
            groupOrchestrationMode: request.isGroupOrchestrationTurn,
            groupParticipantNamesText: request.groupParticipantNamesText.clone(),
            proxySenderName: request.proxySenderNameOverride.clone(),
            notifyReplyOverride: request.turnOptions.notifyReply,
            chatProviderIdOverride: request.chatProviderIdOverride.clone(),
            chatModelIdOverride: request.chatModelIdOverride.clone(),
            disableWarning: request.turnOptions.disableWarning,
            callbacks: Some(Arc::new(MessageProcessingCallbacks {
                nonFatalErrorEventFlow: self.nonFatalErrorEventFlow.clone(),
                chatId: chatId.clone(),
                chatHistoryDelegate: request.chatHistoryDelegate.clone_for_core(),
            })),
            onToolInvocation: None,
            assistantMessageTimestamp: Some(assistantMessageTimestamp),
            executionGeneration: request.executionGeneration.unwrap_or(0),
            segmentIndex: request.executionSegmentIndex.unwrap_or(0),
        })
        .await
        {
            Ok(stream) => {
                AppLogger::i(
                    "CoreSend",
                    &format!("response stream created chatId={}", chatId),
                );
                ChainLogger::info(
                    RECEIVE_CHAIN,
                    "receive.stream.created",
                    &[("chatId", chatId.clone())],
                );
                stream
            }
            Err(error) => {
                ChainLogger::error(
                    RECEIVE_CHAIN,
                    "receive.stream.create.error",
                    &[("chatId", chatId.clone()), ("error", error.to_string())],
                );
                if let Some(session) = workspaceToolHookSession.as_ref() {
                    workspaceToolHookHandler.removeToolHook(session.hookId());
                    session.close();
                }
                self.withExistingRuntime(Some(chatId.clone()), |runtime| {
                    runtime.isLoading = false;
                    runtime.responseStream = None;
                    runtime.sendJob = None;
                    runtime.streamCollectionJob = None;
                    runtime.stateCollectionJob = None;
                });
                self.finishChatExecution(
                    chatId.clone(),
                    InputProcessingState::Error {
                        message: error.to_string(),
                    },
                );
                return Err(error);
            }
        };
        let sharedResponseStream = completionStream.clone();
        sharedResponseStream.set_initial_content(continuationContentPrefix.clone());
        self.withRuntime(Some(chatId.clone()), |runtime| {
            runtime.responseStream = Some(sharedResponseStream.clone());
        });
        let initialProviderModel = request
            .enhancedAiService
            .getLastProviderModel()
            .unwrap_or_default();
        let (initialProvider, initialModelName) = split_provider_model(&initialProviderModel);
        let mut aiMessage = match continuationAssistantMessage {
            Some(message) => message,
            None => ChatMessage {
                sender: "ai".to_string(),
                timestamp: assistantMessageTimestamp,
                roleName: currentRoleName.clone(),
                provider: initialProvider,
                modelName: initialModelName,
                inputTokens: 0,
                outputTokens: 0,
                cachedInputTokens: 0,
                displayMode: ChatMessageDisplayMode::NORMAL,
                ..ChatMessage::new("ai".to_string())
            },
        };
        let streamKey = format!("chat-message-stream:{}", aiMessage.timestamp);
        let segmentSource = coreResponseStreamSource(sharedResponseStream.clone(), streamKey);
        aiMessage.contentStream = Some(CoreStream::fromSourceWithId(
            format!("chat-message-stream:{}", aiMessage.timestamp),
            segmentSource,
        ));
        AppLogger::i(
            "ResponseExecutionTrace",
            &format!(
                "response_segment_started chatId={} timestamp={} continuation={} initialChars={}",
                chatId,
                aiMessage.timestamp,
                isResponseExecutionContinuation,
                continuationContentPrefix.chars().count()
            ),
        );
        let workerChatId = chatId.clone();
        let workerTurnOptions = request.turnOptions.clone();
        let workerAiMessage = Arc::new(Mutex::new(aiMessage.clone()));
        let workerResponseStream = sharedResponseStream.clone();
        let workerRevisionTracker = Arc::new(Mutex::new(TextStreamRevisionTracker::new(
            &continuationContentPrefix,
        )));
        let workerPartStream = Arc::new(Mutex::new(AssistantMarkupStreamState::new()));
        let workerService = request.enhancedAiService.clone();
        let workerChatHistoryDelegate =
            Arc::new(Mutex::new(request.chatHistoryDelegate.clone_for_core()));
        let workerMessageProcessingDelegate = Arc::new(Mutex::new(self.clone_for_core()));
        let completionContextWorkspacePath = request.workspacePath.clone();
        let completionContextPromptFunctionType = request.promptFunctionType.clone();
        let completionContextRoleCardId = request.roleCardId.clone();
        let completionContextRoleName = currentRoleName.clone();
        let completionContextGroupOrchestrationMode = request.isGroupOrchestrationTurn;
        let completionContextGroupParticipantNamesText = request.groupParticipantNamesText.clone();
        let completionContextProxySenderName = request.proxySenderNameOverride.clone();
        let completionContextProviderIdOverride = request.chatProviderIdOverride.clone();
        let completionContextModelIdOverride = request.chatModelIdOverride.clone();
        let (workerRequestSentAt, workerRequestStartElapsed) = self
            .withRuntime(Some(chatId.clone()), |runtime| {
                (runtime.requestSentAt, runtime.requestStartElapsed)
            });
        let workerWorkspaceToolHookSession = Arc::new(Mutex::new(workspaceToolHookSession.clone()));
        let workerWorkspaceToolHookHandler = Arc::new(Mutex::new(workspaceToolHookHandler.clone()));
        if userMessageAdded {
            userMessage.sentAt = workerRequestSentAt;
            request
                .chatHistoryDelegate
                .addMessageToChat(userMessage, Some(chatId.clone()));
        }
        if workerTurnOptions.persistTurn {
            ChainLogger::info(
                MESSAGE_STORE_CHAIN,
                "message.store.ai.placeholder",
                &[
                    ("chatId", chatId.clone()),
                    ("timestamp", aiMessage.timestamp.to_string()),
                ],
            );
            request
                .chatHistoryDelegate
                .addMessageToChat(aiMessage.clone(), Some(chatId.clone()));
        }
        let workerFirstResponseElapsed = Arc::new(Mutex::new(None::<i64>));
        let chunkChatId = workerChatId.clone();
        let chunkFirstResponseElapsed = workerFirstResponseElapsed.clone();
        let chunkRevisionTracker = workerRevisionTracker.clone();
        let completionChatId = workerChatId.clone();
        let completionTurnOptions = workerTurnOptions.clone();
        let completionAiMessage = workerAiMessage.clone();
        let completionChatHistoryDelegate = workerChatHistoryDelegate.clone();
        let completionRevisionTracker = workerRevisionTracker.clone();
        let completionPartStream = workerPartStream.clone();
        let completionMessageProcessingDelegate = workerMessageProcessingDelegate.clone();
        let completionWorkspaceToolHookSession = workerWorkspaceToolHookSession.clone();
        let completionWorkspaceToolHookHandler = workerWorkspaceToolHookHandler.clone();
        let completionFirstResponseElapsed = workerFirstResponseElapsed.clone();
        let completionResponseStream = workerResponseStream.clone();
        let completionExecutionGeneration = request.executionGeneration;
        let mut responseItems = workerResponseStream.clone();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "message-response-collection",
                Box::new(move || {
                    Box::pin(async move {
                        responseItems
                            .collect_ordered(&mut move |item| {
                                let chunk = match item {
                                    ResponseStreamItem::Chunk(chunk) => chunk,
                                    ResponseStreamItem::Revision(event) => {
                                        let mut tracker = chunkRevisionTracker
                                            .lock()
                                            .expect("revision tracker mutex poisoned");
                                        match event.event_type {
                                            operit_util::stream::RevisableTextStream::TextStreamEventType::Savepoint => {
                                                tracker.savepoint(&event.id);
                                            }
                                            operit_util::stream::RevisableTextStream::TextStreamEventType::Rollback => {
                                                tracker
                                                    .rollback(&event.id)
                                                    .expect("response rollback must reference an active savepoint");
                                            }
                                        }
                                        return;
                                    }
                                };
                                let mut firstResponseElapsed = chunkFirstResponseElapsed
                                    .lock()
                                    .expect("first response elapsed mutex poisoned");
                                if firstResponseElapsed.is_none() {
                                    *firstResponseElapsed =
                                        Some(messageTimingNow().startedAtMs as i64);
                                    AppLogger::i(
                                        "CoreSend",
                                        &format!(
                                            "response first chunk delivered chatId={} chars={}",
                                            chunkChatId,
                                            chunk.chars().count()
                                        ),
                                    );
                                    ChainLogger::info(
                                        RECEIVE_CHAIN,
                                        "receive.first_chunk",
                                        &[("chatId", chunkChatId.clone())],
                                    );
                                }
                                drop(firstResponseElapsed);
                                chunkRevisionTracker
                                    .lock()
                                    .expect("revision tracker mutex poisoned")
                                    .append(&chunk);
                            })
                            .await;
                        AppLogger::i(
                            "CoreSend",
                            &format!("response stream closed chatId={}", completionChatId),
                        );
                        let workspaceToolHookSession = completionWorkspaceToolHookSession
                            .lock()
                            .expect("workspace tool hook session mutex poisoned")
                            .take();
                        if let Some(session) = workspaceToolHookSession.as_ref() {
                            completionWorkspaceToolHookHandler
                                .lock()
                                .expect("workspace tool hook handler mutex poisoned")
                                .removeToolHook(session.hookId());
                            session.close();
                        }
                        if let Some(error) = completionResponseStream.terminal_failure() {
                            ChainLogger::error(
                                RECEIVE_CHAIN,
                                "receive.stream.failed",
                                &[
                                    ("chatId", completionChatId.clone()),
                                    ("error", error.clone()),
                                ],
                            );
                            if completionTurnOptions.persistTurn {
                                let failedMessageTimestamp = completionAiMessage
                                    .lock()
                                    .expect("worker AI message mutex poisoned")
                                    .timestamp;
                                completionChatHistoryDelegate
                                    .lock()
                                    .expect("worker chat history mutex poisoned")
                                    .discardFailedAssistantMessage(
                                        completionChatId.clone(),
                                        failedMessageTimestamp,
                                    );
                            }
                            let mut delegate = completionMessageProcessingDelegate
                                .lock()
                                .expect("worker message processing delegate mutex poisoned");
                            delegate.cleanupRuntimeAfterSend(
                                completionChatId.clone(),
                                completionTurnOptions.clone(),
                            );
                            delegate.finishChatExecution(
                                completionChatId.clone(),
                                InputProcessingState::Error { message: error },
                            );
                            return;
                        }
                        let finalContent = completionRevisionTracker
                            .lock()
                            .expect("revision tracker mutex poisoned")
                            .current_content()
                            .to_owned();
                        let mut workerService = workerService;
                        let providerModel =
                            workerService.getLastProviderModel().unwrap_or_default();
                        let (provider, modelName) = split_provider_model(&providerModel);
                        let tokenSnapshot = workerService.getLastTurnTokenSnapshot().unwrap_or(
                            operit_providers::chat::EnhancedAIService::TurnTokenSnapshot {
                                inputTokens: 0,
                                outputTokens: 0,
                                cachedInputTokens: 0,
                            },
                        );
                        let completedElapsed = messageTimingNow().startedAtMs as i64;
                        let finalMessage = {
                            let parts = {
                                let mut partStream = completionPartStream
                                    .lock()
                                    .expect("assistant message-part stream mutex poisoned");
                                partStream.resetToSnapshot(&finalContent).expect(
                                    "completed assistant snapshot must parse into message parts",
                                );
                                partStream.finish().expect(
                                    "completed assistant markup must parse into message parts",
                                )
                            };
                            let mut workerAiMessage = completionAiMessage
                                .lock()
                                .expect("worker AI message mutex poisoned");
                            workerAiMessage.provider = provider;
                            workerAiMessage.modelName = modelName;
                            workerAiMessage.inputTokens += tokenSnapshot.inputTokens;
                            workerAiMessage.outputTokens += tokenSnapshot.outputTokens;
                            workerAiMessage.cachedInputTokens += tokenSnapshot.cachedInputTokens;
                            workerAiMessage.parts = parts;
                            if let Some(executionGeneration) = completionExecutionGeneration {
                                workerAiMessage.completedExecutionGeneration = workerAiMessage
                                    .completedExecutionGeneration
                                    .max(executionGeneration);
                            }
                            MessageProcessingDelegate::withTurnMetrics(
                                ChatMessage {
                                    completedAt: completedElapsed,
                                    ..workerAiMessage.clone()
                                },
                                workerRequestSentAt,
                                workerRequestStartElapsed,
                                *completionFirstResponseElapsed
                                    .lock()
                                    .expect("first response elapsed mutex poisoned"),
                                completedElapsed,
                            )
                        };
                        if workerTurnOptions.persistTurn {
                            ChainLogger::info(
                                MESSAGE_STORE_CHAIN,
                                "message.store.ai.final",
                                &[
                                    ("chatId", workerChatId.clone()),
                                    ("timestamp", finalMessage.timestamp.to_string()),
                                    (
                                        "contentChars",
                                        finalMessage.displayText().chars().count().to_string(),
                                    ),
                                ],
                            );
                        }
                        let mut completionChatHistory = {
                            let workerChatHistoryDelegate = completionChatHistoryDelegate
                                .lock()
                                .expect("worker chat history mutex poisoned");
                            workerChatHistoryDelegate
                                .getRuntimeChatHistory(completionChatId.clone())
                        };
                        if workerTurnOptions.persistTurn {
                            let persistedAssistant = completionChatHistory
                                .iter_mut()
                                .find(|message| message.timestamp == finalMessage.timestamp)
                                .expect("persisted assistant placeholder must remain in chat history");
                            *persistedAssistant = finalMessage.clone();
                        }
                        let nextWindowSize = async {
                            let runtimeOptions = SendMessageOptions {
                                roleCardId: Some(completionContextRoleCardId.clone()),
                                promptFunctionType: completionContextPromptFunctionType.clone(),
                                chatProviderIdOverride: completionContextProviderIdOverride.clone(),
                                chatModelIdOverride: completionContextModelIdOverride.clone(),
                                ..SendMessageOptions::new()
                            };
                            let runtime = workerService
                                .createSendMessageRuntime(&runtimeOptions)
                                .map_err(|_| ())?;
                            AIMessageManager::calculateStableContextWindow(
                                StableContextWindowRequest {
                                    enhancedAiService: &mut workerService,
                                    chatId: Some(completionChatId.clone()),
                                    messageContent: String::new(),
                                    chatHistory: completionChatHistory,
                                    workspacePath: completionContextWorkspacePath,
                                    promptFunctionType: completionContextPromptFunctionType,
                                    roleCardId: Some(completionContextRoleCardId),
                                    currentRoleName: Some(completionContextRoleName),
                                    splitHistoryByRole: true,
                                    groupOrchestrationMode: completionContextGroupOrchestrationMode,
                                    groupParticipantNamesText:
                                        completionContextGroupParticipantNamesText,
                                    proxySenderName: completionContextProxySenderName,
                                    chatProviderIdOverride: completionContextProviderIdOverride,
                                    chatModelIdOverride: completionContextModelIdOverride,
                                    publishEstimate: true,
                                    runtime,
                                },
                            )
                            .await
                            .map_err(|_| ())
                        }
                        .await
                        .ok();
                        let mut workerChatHistoryDelegate = completionChatHistoryDelegate
                            .lock()
                            .expect("worker chat history mutex poisoned");
                        let chatMetrics = nextWindowSize.map(|windowSize| {
                            let previousTokens = workerChatHistoryDelegate
                                .chatHistoriesFlow()
                                .value()
                                .into_iter()
                                .find(|history| history.id == completionChatId)
                                .map(|history| (history.inputTokens, history.outputTokens));
                            let (inputTokens, outputTokens) = match previousTokens {
                                Some((inputTokens, outputTokens)) => (
                                    inputTokens + finalMessage.inputTokens,
                                    outputTokens + finalMessage.outputTokens,
                                ),
                                None => (finalMessage.inputTokens, finalMessage.outputTokens),
                            };
                            (inputTokens, outputTokens, windowSize)
                        });
                        if workerTurnOptions.persistTurn {
                            let completedMessage = ChatMessage {
                                contentStream: None,
                                ..finalMessage.clone()
                            };
                            let commitResult = workerChatHistoryDelegate
                                .commitAssistantMessageSegment(
                                    completionChatId.clone(),
                                    completedMessage.clone(),
                                    chatMetrics,
                                );
                            if let Err(error) = commitResult {
                                drop(workerChatHistoryDelegate);
                                let message = format!(
                                    "failed to commit completed assistant message: {error}"
                                );
                                let mut delegate = completionMessageProcessingDelegate
                                    .lock()
                                    .expect(
                                        "worker message processing delegate mutex poisoned",
                                    );
                                delegate.cleanupRuntimeAfterSend(
                                    completionChatId.clone(),
                                    completionTurnOptions.clone(),
                                );
                                delegate.finishChatExecution(
                                    completionChatId.clone(),
                                    InputProcessingState::Error { message },
                                );
                                return;
                            }
                        }
                        drop(workerChatHistoryDelegate);
                        completionMessageProcessingDelegate
                            .lock()
                            .expect("worker message processing delegate mutex poisoned")
                            .finalizeMessageAndNotify(
                                completionChatId.clone(),
                                ChatMessage {
                                    contentStream: None,
                                    ..finalMessage
                                },
                                nextWindowSize,
                                completionTurnOptions.clone(),
                            );
                    })
                }),
            )
            .map_err(|error| {
                operit_providers::chat::llmprovider::AIService::AiServiceError::RequestFailed(
                    error.to_string(),
                )
            })?;
        Ok(SendUserMessageProcessingResult {
            aiMessage,
            nextWindowSize: None,
        })
    }

    /// Regenerates one AI message variant from a prior user request and history snapshot.
    #[allow(non_snake_case)]
    pub async fn regenerateAiMessageVariant(
        &mut self,
        request: RegenerateAiMessageVariantRequest<'_>,
    ) -> Result<ChatMessage, operit_providers::chat::llmprovider::AIService::AiServiceError> {
        let targetMessageTimestamp = request.targetMessageTimestamp;
        let result = self
            .sendUserMessage(SendUserMessageProcessingRequest {
                enhancedAiService: request.enhancedAiService,
                chatHistoryDelegate: request.chatHistoryDelegate,
                chatId: request.chatId,
                messageText: request.requestMessageContent,
                chatHistory: request.requestHistory,
                promptHistoryOverride: None,
                workspacePath: request.workspacePath,
                promptFunctionType: request.promptFunctionType,
                roleCardId: request.roleCardId,
                currentRoleName: Some(request.currentRoleName),
                characterName: None,
                avatarUri: None,
                attachments: request.attachments,
                replyToMessage: request.replyToMessage,
                enableThinking: request.enableThinking,
                enableMemoryAutoUpdate: request.enableMemoryAutoUpdate,
                maxTokens: request.maxTokens,
                tokenUsageThreshold: request.tokenUsageThreshold,
                chatProviderIdOverride: request.chatProviderIdOverride,
                chatModelIdOverride: request.chatModelIdOverride,
                isGroupOrchestrationTurn: false,
                groupParticipantNamesText: None,
                proxySenderNameOverride: None,
                suppressUserMessageInHistory: true,
                isAutoContinuation: false,
                assistantMessageTimestamp: None,
                executionGeneration: None,
                executionSegmentIndex: None,
                turnOptions: ChatTurnOptions {
                    persistTurn: false,
                    ..ChatTurnOptions::default()
                },
            })
            .await?;
        Ok(ChatMessage {
            timestamp: targetMessageTimestamp,
            ..result.aiMessage
        })
    }

    /// Updates completion counters and clears send-time processing state.
    #[allow(non_snake_case)]
    pub fn notifyTurnComplete(
        &mut self,
        chatId: Option<String>,
        _service: &EnhancedAIService,
        _nextWindowSize: Option<i64>,
        _turnOptions: ChatTurnOptions,
    ) {
        if let Some(chatId) = chatId {
            let mut counters = self.turnCompleteCounterByChatIdFlow.value();
            let next = counters.get(&chatId).copied().unwrap_or(0) + 1;
            counters.insert(chatId, next);
            self.turnCompleteCounterByChatId = counters.clone();
            self.turnCompleteCounterByChatIdFlow.set_value(counters);
        }
    }

    /// Finalizes a completed AI message and publishes completion notifications.
    #[allow(non_snake_case)]
    pub fn finalizeMessageAndNotify(
        &mut self,
        chatId: String,
        aiMessage: ChatMessage,
        nextWindowSize: Option<i64>,
        turnOptions: ChatTurnOptions,
    ) {
        let shouldNotifyReply = turnOptions.persistTurn && turnOptions.notifyReply != Some(false);
        self.cleanupRuntimeAfterSend(chatId.clone(), turnOptions);
        self.finishChatExecution(chatId.clone(), InputProcessingState::Completed);
        let mut counters = self.turnCompleteCounterByChatIdFlow.value();
        let next = counters.get(&chatId).copied().unwrap_or(0) + 1;
        counters.insert(chatId.clone(), next);
        self.turnCompleteCounterByChatId = counters.clone();
        self.turnCompleteCounterByChatIdFlow.set_value(counters);
        if shouldNotifyReply {
            publishOwnerAppNotification(RuntimeHostInteractionAppNotificationPayload {
                notificationType: "ai_message_completed".to_string(),
                title: "Operit".to_string(),
                message: aiMessageNotificationPreview(&aiMessage.displayText()),
                chatId: Some(chatId),
                messageTimestamp: Some(aiMessage.timestamp),
            });
        }
        let _ = nextWindowSize;
    }

    /// Clears runtime state after a send has finished.
    #[allow(non_snake_case)]
    pub fn cleanupRuntimeAfterSend(&mut self, chatId: String, _turnOptions: ChatTurnOptions) {
        self.withExistingRuntime(Some(chatId.clone()), |runtime| {
            runtime.isLoading = false;
            runtime.sendJob = None;
            runtime.streamCollectionJob = None;
            runtime.stateCollectionJob = None;
        });
        self.clearCurrentTurnToolInvocationCount(chatId);
    }
}

/// Builds a compact single-line preview for an AI reply notification.
fn aiMessageNotificationPreview(content: &str) -> String {
    const MAX_NOTIFICATION_PREVIEW_CHARACTERS: usize = 240;
    content
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_NOTIFICATION_PREVIEW_CHARACTERS)
        .collect()
}

impl Default for MessageProcessingDelegate {
    fn default() -> Self {
        let rootDir = ApiPreferences::data_dir();
        Self::new(
            FunctionalConfigManager::new(rootDir.clone()),
            ModelConfigManager::new(rootDir),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_model::MessagePart::MessagePartKind;

    /// Verifies that a handoff snapshot keeps prior protocol parts and the active segment.
    #[test]
    fn handoff_snapshot_appends_active_segment_to_existing_assistant_protocol() {
        let timestamp = 42_100;
        let prefix = concat!(
            "<think>first segment</think>",
            "第一个设备准备切换。",
            "<tool_AAAA name=\"switch_core\">",
            "<param name=\"node_id\">android-node</param>",
            "</tool_AAAA>",
            "<tool_result name=\"switch_core\" status=\"success\">",
            "<content>android-node</content>",
            "</tool_result>"
        );
        let activeSegment = concat!(
            "<think>second segment</think>",
            "第二个设备继续切换。",
            "<tool_BBBB name=\"switch_core\">",
            "<param name=\"node_id\">windows-node</param>",
            "</tool_BBBB>",
            "<tool_result name=\"switch_core\" status=\"success\">",
            "<content>windows-node</content>",
            "</tool_result>",
            "切换完成。"
        );
        let message = ChatMessage::new_with_markdown_timestamp(
            "ai".to_string(),
            prefix.to_string(),
            timestamp,
        );

        let snapshot = assistantMessageWithHandoffProtocolSnapshot(message, Some(activeSegment))
            .expect("handoff assistant protocol snapshot must parse");

        let toolCallCount = snapshot
            .parts
            .iter()
            .filter(|part| part.kind == MessagePartKind::ToolCall)
            .count();
        let toolResultCount = snapshot
            .parts
            .iter()
            .filter(|part| part.kind == MessagePartKind::ToolResult)
            .count();
        let toolNames = snapshot
            .parts
            .iter()
            .filter(|part| part.kind == MessagePartKind::ToolCall)
            .map(|part| part.toolName.as_deref())
            .collect::<Vec<_>>();

        assert_eq!(snapshot.timestamp, timestamp);
        assert_eq!(toolCallCount, 2);
        assert_eq!(toolResultCount, 2);
        assert_eq!(toolNames, vec![Some("switch_core"), Some("switch_core")]);
        assert_eq!(
            snapshot
                .parts
                .iter()
                .filter(|part| part.kind == MessagePartKind::Markdown)
                .map(|part| part.content.as_str())
                .collect::<Vec<_>>(),
            vec![
                "",
                "第一个设备准备切换。",
                "第二个设备继续切换。",
                "切换完成。"
            ]
        );
    }
}

/// Splits a provider/model identifier into its provider and model parts.
fn split_provider_model(providerModel: &str) -> (String, String) {
    let Some(index) = providerModel.find(':') else {
        return (providerModel.to_string(), String::new());
    };
    (
        providerModel[..index].to_string(),
        providerModel[index + 1..].to_string(),
    )
}
