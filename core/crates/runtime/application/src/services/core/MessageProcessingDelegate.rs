use operit_store::ChatExecutionLease::ChatExecutionLease;
use crate::plugins::toolpkg::ToolPkgChatResponseHookBridge::{ToolPkgChatResponseHookBridge, PRESENTATION_EXTENSION};
use std::collections::{HashMap, HashSet};

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use futures_util::future::{AbortHandle, BoxFuture, Shared};

use crate::core::chat::AIMessageManager::{
    logMessageTiming, messageTimingNow, AIMessageManager, BuildUserMessageContentRequest,
    SendMessageRequest as AIMessageSendRequest,
};
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use crate::services::core::ChatHistoryDelegate::ChatHistoryDelegate;
use crate::services::core::MessageCoordinationDelegate::{MessageCoordinationDelegate, SummaryContinuationRequest};
use crate::services::RuntimeHostInteractionService::{
    publishOwnerAppNotification, RuntimeHostInteractionAppNotificationPayload,
};
use crate::ui::features::chat::webview::workspace::WorkspaceBackupManager::WorkspaceBackupManager;
use operit_host_api::FileSystemHost;
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
use operit_model::ChatTurnOptions::{ChatTurnContinuation, ChatTurnOptions};
use operit_model::FunctionType::FunctionType;
use operit_model::InputProcessingState::InputProcessingState;
use operit_model::MessagePart::{MessagePart, MessagePartKind};
use operit_model::MessagePartCodec::{AssistantMarkupStreamState, MessagePartCodec};
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_model::PromptTurn::PromptTurn;
use operit_providers::chat::llmprovider::AIService::SharedAiResponseStream;
use operit_providers::chat::EnhancedAIService::{EnhancedAIService, SendMessageCallbacks};
use operit_providers::runtime_support::ChatConfigurationResult;
use operit_plugin_sdk::js_sdk::results::{ChatCommittedMessage, JsOptional, MessageSendOutcome, MessageSendResultData, MessageSendStatus};
use tokio::sync::oneshot;
use operit_store::PreferencesDataStore::{mutableStateFlow, MutableStateFlow, StateFlow};
use operit_tools::runtime_support::CoreRouteResumeContext;
use operit_tools::tools::ToolProgressBus::ToolProgressBus;
use operit_util::stream::RevisableTextStream::{
    RenderableTextStream, ResponseStreamItem, RevisableTextStream,
};
use operit_util::stream::Stream::Stream;
use operit_util::stream::TextStreamRevisionTracker::TextStreamRevisionTracker;
use operit_util::AppLogger::AppLogger;
use operit_util::ChainLogger::{self, MESSAGE_STORE_CHAIN, RECEIVE_CHAIN, SEND_CHAIN};
use operit_util::ChatMarkupRegex::tag_ranges;
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
    AppLogger::trace(
        "CoreStreamTrace",
        &format!(
            "response.open requestId={} target={} property={} streamKey={}",
            request.requestId.0,
            request.target.clone(),
            request.propertyName,
            streamKey,
        ),
    );
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "core-runtime-text-markdown",
            Box::new(move || {
                Box::pin(collectCoreResponseStream(stream, streamKey, request, sender))
            }),
        )
        .map_err(|error| CoreLinkError::internal(error.to_string()))?;
    Ok(receiver)
}

/// Transforms one observer's stream until its receiver closes or generation completes.
async fn collectCoreResponseStream(
    mut orderedStream: SharedAiResponseStream,
    streamKey: String,
    request: CoreWatchRequest,
    sender: tokio::sync::mpsc::UnboundedSender<CoreEvent>,
) {
    let mut markdownStream = MarkdownRenderEventStream::new(streamKey.clone());
    for event in markdownStream.beginSnapshot("") {
        sendCoreTextEvent(&sender, &request.requestId, request.target.clone(), &request.propertyName,
            CoreEventKind::Changed, operit_link::toCoreValue(event).expect("MarkdownStreamEvent must serialize"));
    }
    let mut textRevisions = TextStreamRevisionTracker::new("");
    let mut itemCollector = |item: ResponseStreamItem| match item {
        ResponseStreamItem::Chunk(chunk) => {
            let _ = textRevisions.append(&chunk);
            let events = markdownStream.pushChunk(&chunk);
            for event in events {
                sendCoreTextEvent(
                    &sender,
                    &request.requestId,
                    request.target.clone(),
                    &request.propertyName,
                    CoreEventKind::Changed,
                    operit_link::toCoreValue(event).expect("MarkdownStreamEvent must serialize"),
                );
            }
        }
        ResponseStreamItem::Revision(event) => {
            let markdownEvent = match event.event_type {
                operit_util::stream::RevisableTextStream::TextStreamEventType::Savepoint => {
                    textRevisions.savepoint(&event.id);
                    AppLogger::trace_detail(
                        "CoreStreamTrace",
                        &format!(
                            "response.revision.savepoint streamKey={} id={}",
                            streamKey, event.id
                        ),
                    );
                    MarkdownStreamEvent::savepoint(streamKey.clone(), event.id)
                }
                operit_util::stream::RevisableTextStream::TextStreamEventType::Rollback => {
                    match textRevisions.rollback(&event.id) {
                        Some(content) => markdownStream.restoreContent(content),
                        None => {
                            AppLogger::w(
                                "CoreStreamTrace",
                                &format!(
                                    "markdown rollback savepoint missing; keeping current content streamKey={} id={}",
                                    streamKey, event.id
                                ),
                            );
                        }
                    }
                    AppLogger::trace_detail(
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
                request.target.clone(),
                &request.propertyName,
                CoreEventKind::Changed,
                operit_link::toCoreValue(markdownEvent)
                    .expect("MarkdownStreamEvent must serialize"),
            );
        }
    };
    tokio::select! {
        biased;
        _ = sender.closed() => return,
        _ = orderedStream.collect_ordered(&mut itemCollector) => {},
    }
    AppLogger::trace(
        "CoreStreamTrace",
        &format!(
            "response.completed requestId={} streamKey={}",
            request.requestId.0, streamKey
        ),
    );
    for event in markdownStream.finish() {
        let kind = if event.isRootCompletion() {
            CoreEventKind::Completed
        } else {
            CoreEventKind::Changed
        };
        sendCoreTextEvent(
            &sender,
            &request.requestId,
            request.target.clone(),
            &request.propertyName,
            kind,
            operit_link::toCoreValue(event).expect("MarkdownStreamEvent must serialize"),
        );
    }
}

/// Sends one typed Markdown event through a Core stream channel.
fn sendCoreTextEvent(
    sender: &tokio::sync::mpsc::UnboundedSender<CoreEvent>,
    requestId: &CoreRequestId,
    target: String,
    propertyName: &str,
    kind: CoreEventKind,
    value: CoreValue,
) {
    // A disposed observer owns no output channel; its producer remains independent.
    let _ = sender.send(CoreEvent {
        requestId: Some(requestId.clone()),
        target,
        propertyName: propertyName.to_string(),
        kind,
        value,
    });
}

/// Waits for the actual originating stream and persistence completion without retaining the chat holder lock.
#[derive(Debug)]
pub(crate) enum ChatTurnSubmission {
    /// Observes one specific originating generation, including its failure or cancellation.
    Pending(oneshot::Receiver<Result<MessageSendResultData, String>>),
    /// Returns an explicit input-ownership decision that is not a completed generation.
    Handled(MessageSendResultData),
}

impl ChatTurnSubmission {
    /// Awaits only this originating generation and reports a lost completion channel as an actual error.
    pub(crate) async fn wait(self) -> Result<MessageSendResultData, String> {
        match self {
            Self::Handled(result) => Ok(result),
            Self::Pending(receiver) => receiver.await
                .map_err(|error| format!("Chat turn completion channel closed before a terminal receipt: {error}"))?,
        }
    }
}

/// Tracks exact commit identities on one real turn instead of searching the most recent history row.
#[derive(Debug)]
struct ChatTurnReceiptState {
    turnId: u64,
    nativeExecutionId: Option<String>,
    sender: Option<oneshot::Sender<Result<MessageSendResultData, String>>>,
    result: MessageSendResultData,
}

impl ChatTurnReceiptState {
    /// Preserves the user's original submission while forwarding the final continuation result.
    fn deliverContinuation(&mut self, result: Result<MessageSendResultData, String>) {
        let result = result.map(|mut result| {
            result.message = self.result.message.clone();
            result.sentAt = self.result.sentAt;
            result
        });
        self.deliver(result);
    }

    /// Delivers a terminal result once; a detached UI observer does not alter the business result.
    fn deliver(&mut self, result: Result<MessageSendResultData, String>) {
        if let Some(sender) = self.sender.take() {
            if sender.send(result).is_err() {
                AppLogger::v_with_level("ChatTurnReceipt", "Originating UI observer detached from the accepted turn", operit_util::AppLogger::VERBOSE_LEVEL_4);
            }
        }
    }
}

/// Per-chat runtime state for one active or recently active send turn.
#[derive(Clone, Debug)]
pub struct ChatRuntime {
    pub activeTurnId: u64,
    pub workId: Option<String>,
    receipt: Option<Arc<Mutex<ChatTurnReceiptState>>>,
    pub executionLease: Option<ChatExecutionLease>,
    pub isCancelling: bool,
    pub activeStreamingAiMessage: Option<ChatMessage>,
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
            activeTurnId: 0,
            workId: None,
            receipt: None,
            executionLease: None,
            isCancelling: false,
            activeStreamingAiMessage: None,
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

impl ChatRuntime {
    /// Validates generation ownership and reserves cancellation while the caller holds its runtime lock.
    pub(crate) fn reserveCancellation(&mut self, expectedTurnId: Option<u64>) -> Result<Option<(u64, u64)>, String> {
        if expectedTurnId.is_some_and(|expected| self.activeTurnId != expected) {
            return Err("Work generation is no longer active".to_string());
        }
        if !self.isLoading || self.isCancelling {
            if expectedTurnId.is_some() { return Err("Work execution is no longer running".to_string()); }
            return Ok(None);
        }
        let cancelledTurnId = self.activeTurnId;
        self.isCancelling = true;
        self.activeTurnId = self.activeTurnId.checked_add(1).expect("chat turn id must not overflow");
        Ok(Some((cancelledTurnId, self.activeTurnId)))
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

/// Stores the chat runtime statistics delivered to one ToolPkg hook invocation.
struct ToolPkgChatRuntimeStatistics {
    activeChatIds: Vec<String>,
    currentTurnToolInvocationCount: i32,
    activeConversationCount: i32,
    currentSessionToolCount: i32,
}

/// Collects active-chat statistics from one atomic chat execution-state snapshot.
fn collectToolPkgChatRuntimeStatistics(
    chatId: &str,
    executionStates: &HashMap<String, ChatExecutionState>,
    currentTurnToolInvocationCounts: &HashMap<String, i32>,
) -> ToolPkgChatRuntimeStatistics {
    let activeChatIds = executionStates
        .iter()
        .filter_map(|(id, state)| state.isLoading.then_some(id.clone()))
        .collect::<Vec<_>>();
    let currentTurnToolInvocationCount = *currentTurnToolInvocationCounts
        .get(chatId)
        .expect("tool invocation count must be initialized before dispatching a chat runtime hook");
    let currentSessionToolCount = activeChatIds
        .iter()
        .map(|activeChatId| {
            currentTurnToolInvocationCounts
                .get(activeChatId)
                .copied()
                .expect("active chat tool invocation count must be initialized before dispatching a chat runtime hook")
        })
        .sum();
    ToolPkgChatRuntimeStatistics {
        activeConversationCount: activeChatIds.len() as i32,
        activeChatIds,
        currentTurnToolInvocationCount,
        currentSessionToolCount,
    }
}

/// Captures enough turn state to preserve or discard partial output during cancellation.
#[derive(Clone, Debug)]
pub struct TurnCancellationSnapshot {
    pub chatId: String,
    pub turnId: u64,
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
    pub participantId: String,
    pub chatProviderIdOverride: Option<String>,
    pub chatModelIdOverride: Option<String>,
}

/// End-to-end request for sending a user message through enhanced AI processing.
pub struct SendUserMessageProcessingRequest<'a> {
    pub executionLease: ChatExecutionLease,
    pub chatConfiguration: ChatConfigurationResult,
    pub enhancedAiService: &'a mut EnhancedAIService,
    pub chatHistoryDelegate: &'a mut ChatHistoryDelegate,
    pub chatId: String,
    pub messageText: String,
    pub chatHistory: Vec<ChatMessage>,
    pub promptHistoryOverride: Option<Vec<PromptTurn>>,
    pub workspacePath: Option<String>,
    pub workspaceFolders: Vec<String>,
    pub promptFunctionType: PromptFunctionType,
    pub attachments: Vec<AttachmentInfo>,
    pub replyToMessage: Option<ChatMessage>,
    pub enableThinking: bool,
    pub maxTokens: i32,
    pub tokenUsageThreshold: f64,
    pub chatProviderIdOverride: Option<String>,
    pub chatModelIdOverride: Option<String>,
    pub proxySenderNameOverride: Option<String>,
    pub suppressUserMessageInHistory: bool,
    pub isAutoContinuation: bool,
    pub isResume: bool,
    pub turnOptions: ChatTurnOptions,
}

/// Result returned after a user message send finishes and history is updated.
#[derive(Debug)]
pub struct SendUserMessageProcessingResult {
    pub(crate) completion: ChatTurnSubmission,
    pub aiMessage: ChatMessage,
    pub nextWindowSize: Option<i64>,
}

/// Request data used to regenerate one AI message variant.
pub struct RegenerateAiMessageVariantRequest<'a> {
    pub executionLease: ChatExecutionLease,
    pub chatConfiguration: ChatConfigurationResult,
    pub enhancedAiService: &'a mut EnhancedAIService,
    pub chatHistoryDelegate: &'a mut ChatHistoryDelegate,
    pub chatId: String,
    pub targetMessageTimestamp: i64,
    pub requestMessageContent: String,
    pub requestHistory: Vec<ChatMessage>,
    pub workspacePath: Option<String>,
    pub promptFunctionType: PromptFunctionType,
    pub attachments: Vec<AttachmentInfo>,
    pub replyToMessage: Option<ChatMessage>,
    pub enableThinking: bool,
    pub maxTokens: i32,
    pub tokenUsageThreshold: f64,
    pub chatProviderIdOverride: Option<String>,
    pub chatModelIdOverride: Option<String>,
}

/// Tracks one host-owned summary across all processing delegate clones.
#[derive(Clone)]
pub(crate) struct ChatSummaryTask {
    pub taskId: Arc<()>,
    pub abortHandle: AbortHandle,
    pub completion: Shared<BoxFuture<'static, Result<(), String>>>,
}

/// Shares summary ownership and cancellation across all service delegates.
#[derive(Clone, Default)]
struct ChatSummaryTasks {
    tasks: Arc<Mutex<HashMap<String, ChatSummaryTask>>>,
}

impl ChatSummaryTasks {
    /// Rejects overlapping summaries instead of replacing their cancellation handles.
    fn register(&self, chatId: String, task: ChatSummaryTask) -> Result<(), String> {
        let mut tasks = self
            .tasks
            .lock()
            .expect("summary task registry mutex poisoned");
        if tasks.contains_key(&chatId) {
            return Err(format!(
                "Conversation summary already running for chat: {chatId}"
            ));
        }
        tasks.insert(chatId, task);
        Ok(())
    }

    /// Returns the completion and cancellation identity of an active summary.
    fn get(&self, chatId: &str) -> Option<ChatSummaryTask> {
        self.tasks
            .lock()
            .expect("summary task registry mutex poisoned")
            .get(chatId)
            .cloned()
    }

    /// Aborts the selected chat's summaries without cancelling another chat's work.
    fn cancel(&self, chatId: Option<&str>) {
        let handles = self
            .tasks
            .lock()
            .expect("summary task registry mutex poisoned")
            .iter()
            .filter(|(id, _)| chatId.is_none_or(|chatId| id.as_str() == chatId))
            .map(|(_, task)| task.abortHandle.clone())
            .collect::<Vec<_>>();
        for handle in handles {
            handle.abort();
        }
    }

    /// Releases only the exact finishing operation after its UI cleanup has completed.
    fn finish(&self, chatId: &str, taskId: &Arc<()>) {
        let mut tasks = self
            .tasks
            .lock()
            .expect("summary task registry mutex poisoned");
        if tasks
            .get(chatId)
            .is_some_and(|task| Arc::ptr_eq(&task.taskId, taskId))
        {
            tasks.remove(chatId);
        }
    }
}
/// Manages message send state, streaming persistence, cancellation, and UI flows.
pub struct MessageProcessingDelegate {
    pub functionalConfigManager: FunctionalConfigManager,
    pub modelConfigManager: ModelConfigManager,
    pub fileSystemHost: Option<Arc<dyn FileSystemHost>>,
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
    summaryTasksByChatId: ChatSummaryTasks,
    pub speakMessageHandler: Option<fn(String, bool)>,
}

/// Bridges enhanced AI callbacks back into processing delegate state flows.
struct MessageProcessingCallbacks {
    nonFatalErrorEventFlow: MutableStateFlow<Option<String>>,
    tokenLimitExceeded: Arc<AtomicBool>,
}

impl SendMessageCallbacks for MessageProcessingCallbacks {
    /// Publishes non-fatal model/provider errors to observers.
    #[allow(non_snake_case)]
    fn onNonFatalError(&self, error: String) {
        self.nonFatalErrorEventFlow.set_value(Some(error));
    }

    /// Defers compaction until the completed stream has persisted its final tool results.
    fn onTokenLimitExceeded(&self) {
        self.tokenLimitExceeded.store(true, Ordering::Release);
    }
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
            fileSystemHost: None,
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
            summaryTasksByChatId: ChatSummaryTasks::default(),
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
            fileSystemHost: self.fileSystemHost.clone(),
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
            summaryTasksByChatId: self.summaryTasksByChatId.clone(),
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

    /// Starts a fresh chat turn and returns its chat-scoped ownership token.
    #[allow(non_snake_case)]
    fn beginChatTurn(
        &self,
        chatId: String,
        turnOptions: ChatTurnOptions,
        executionLease: ChatExecutionLease,
    ) -> u64 {
        self.withRuntime(Some(chatId), |runtime| {
            runtime.activeTurnId = runtime
                .activeTurnId
                .checked_add(1)
                .expect("chat turn id must not overflow");
            runtime.isCancelling = false;
            runtime.executionLease = Some(executionLease);
            runtime.currentTurnOptions = turnOptions;
            runtime.requestSentAt = messageTimingNow().startedAtMs as i64;
            runtime.requestStartElapsed = messageTimingNow().startedAtMs as i64;
            runtime.firstResponseElapsed = None;
            runtime.isLoading = true;
            runtime.activeStreamingAiMessage = None;
            runtime.responseStream = None;
            runtime.activeTurnId
        })
    }

    /// Captures a timestamp only after reading the actual committed user row.
    fn recordCommittedUser(&self, chatId: &str, turnId: u64, timestamp: i64) {
        self.withExistingRuntime(Some(chatId.to_string()), |runtime| {
            if let Some(receipt) = &runtime.receipt {
                let mut receipt = receipt.lock().expect("turn receipt mutex poisoned");
                if receipt.turnId == turnId {
                    if let MessageSendOutcome::Committed { userMessageTimestamp, .. } = &mut receipt.result.outcome {
                        *userMessageTimestamp = Some(timestamp);
                    }
                }
            }
        });
    }

    /// Captures the exact assistant revision only on a successful persistence path for this generation.
    pub(crate) fn recordCommittedAssistant(&self, chatId: &str, turnId: u64, message: &ChatMessage, variantIndex: i32) {
        self.withExistingRuntime(Some(chatId.to_string()), |runtime| {
            if let Some(receipt) = &runtime.receipt {
                let mut receipt = receipt.lock().expect("turn receipt mutex poisoned");
                if receipt.turnId == turnId {
                    if let MessageSendOutcome::Committed { assistant, .. } = &mut receipt.result.outcome {
                        *assistant = Some(ChatCommittedMessage { messageTimestamp: message.timestamp, variantIndex });
                    }
                }
            }
        });
    }

    /// Completes only the originating receipt with already captured persisted identities.
    fn completeTurnReceipt(&self, chatId: &str, turnId: u64, status: MessageSendStatus, message: Option<&ChatMessage>) {
        self.withExistingRuntime(Some(chatId.to_string()), |runtime| {
            if let Some(receipt) = &runtime.receipt {
                let mut receipt = receipt.lock().expect("turn receipt mutex poisoned");
                if receipt.turnId != turnId || receipt.sender.is_none() { return; }
                match &mut receipt.result.outcome {
                    MessageSendOutcome::Committed { status: current, .. } | MessageSendOutcome::NotPersisted { status: current } => *current = status.clone(),
                    MessageSendOutcome::Blocked { .. } | MessageSendOutcome::Consumed { .. } => unreachable!("Input decisions never own a generation receipt"),
                }
                if matches!(receipt.result.outcome, MessageSendOutcome::Committed { userMessageTimestamp: None, assistant: None, .. }) {
                    receipt.result.outcome = MessageSendOutcome::NotPersisted { status: status.clone() };
                }
                receipt.result.aiResponse = JsOptional::from_nullable_option(message.map(ChatMessage::displayText));
                receipt.result.receivedAt = JsOptional::Value(messageTimingNow().startedAtMs as i64);
                let result = receipt.result.clone();
                receipt.deliver(Ok(result));
            }
        });
    }

    /// Reads only the named native execution's actual committed receipt, including records retained when its terminal result is an error.
    pub(crate) fn nativeCommittedTurnReceipt(&self, chatId: &str, executionId: &str) -> Result<Option<MessageSendResultData>, String> {
        match self.withExistingRuntime(Some(chatId.to_string()), |runtime| {
            runtime.receipt.as_ref().map(|receipt| {
                let receipt = receipt.lock().map_err(|_| "Native committed receipt mutex poisoned".to_string())?;
                if receipt.nativeExecutionId.as_deref() == Some(executionId) { Ok(Some(receipt.result.clone())) }
                else { Ok(None) }
            }).transpose()
        }) {
            Some(result) => result.map(Option::flatten),
            None => Ok(None),
        }
    }

    /// Propagates an execution or persistence failure to its waiting send without returning empty success.
    pub(crate) fn failTurnReceipt(&self, chatId: &str, error: String) {
        self.withExistingRuntime(Some(chatId.to_string()), |runtime| {
            if let Some(receipt) = &runtime.receipt {
                receipt.lock().expect("turn receipt mutex poisoned").deliver(Err(error));
            }
        });
    }

    /// Settles cancellation only after Core commits its explicit partial output or confirms that no output is kept.
    pub(crate) fn completeCancelledTurnReceipt(&self, chatId: &str, message: Option<&ChatMessage>) {
        let turnId = self.withExistingRuntime(Some(chatId.to_string()), |runtime| {
            runtime.receipt.as_ref().map(|receipt| receipt.lock().expect("turn receipt mutex poisoned").turnId)
        }).flatten();
        if let Some(turnId) = turnId {
            if let Some(message) = message { self.recordCommittedAssistant(chatId, turnId, message, message.selectedVariantIndex); }
            self.completeTurnReceipt(chatId, turnId, MessageSendStatus::Cancelled, message);
        }
    }

    /// Reports whether an asynchronous action still owns the current chat turn.
    #[allow(non_snake_case)]
    pub(crate) fn isCurrentChatTurn(&self, chatId: &str, turnId: u64) -> bool {
        self.withExistingRuntime(Some(chatId.to_string()), |runtime| {
            runtime.activeTurnId == turnId && !runtime.isCancelling
        })
        .unwrap_or(false)
    }

    /// Clears one turn's runtime state without touching a newer turn for the chat.
    #[allow(non_snake_case)]
    fn cleanupRuntimeAfterTurn(&mut self, chatId: String, turnId: u64) -> bool {
        let cleaned = self
            .withExistingRuntime(Some(chatId.clone()), |runtime| {
                if runtime.activeTurnId != turnId || runtime.isCancelling {
                    return false;
                }
                if let Some(lease) = runtime.executionLease.take() {
                    if !runtime.currentTurnOptions.deferSequenceCompletion {
                        lease.release().expect("Execution lease release must succeed");
                    }
                }
                runtime.isLoading = false;
                runtime.sendJob = None;
                runtime.streamCollectionJob = None;
                runtime.stateCollectionJob = None;
                runtime.activeStreamingAiMessage = None;
                true
            })
            .unwrap_or(false);
        if cleaned {
            self.clearCurrentTurnToolInvocationCount(chatId);
        }
        cleaned
    }

    /// Publishes a terminal state only while the originating turn remains current.
    #[allow(non_snake_case)]
    fn finishChatExecutionForTurn(
        &mut self,
        chatId: String,
        turnId: u64,
        terminalState: InputProcessingState,
    ) -> bool {
        if !self.isCurrentChatTurn(&chatId, turnId) {
            return false;
        }
        self.finishChatExecution(chatId, terminalState);
        true
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

    /// Initializes tool invocation accounting for a chat state that is about to be published.
    #[allow(non_snake_case)]
    fn initializeCurrentTurnToolInvocationCount(&mut self, chatId: &str) {
        let mut counts = self.currentTurnToolInvocationCountByChatIdFlow.value();
        counts.entry(chatId.to_string()).or_insert(0);
        self.currentTurnToolInvocationCountByChatId = counts.clone();
        self.currentTurnToolInvocationCountByChatIdFlow
            .set_value(counts);
    }

    /// Dispatches a ToolPkg chat runtime state change with active-chat statistics.
    #[allow(non_snake_case)]
    fn dispatchToolPkgChatRuntimeStateChange(&self, chatId: &str, state: &InputProcessingState) {
        let executionStates = self.executionStateByChatIdFlow.value();
        let counts = self.currentTurnToolInvocationCountByChatIdFlow.value();
        let statistics = collectToolPkgChatRuntimeStatistics(chatId, &executionStates, &counts);
        crate::plugins::toolpkg::ToolPkgChatRuntimeHookBridge::ToolPkgChatRuntimeHookBridge::dispatchStateChanged(
            chatId,
            state,
            statistics.activeChatIds,
            statistics.currentTurnToolInvocationCount,
            statistics.activeConversationCount,
            statistics.currentSessionToolCount,
        );
    }

    /// Updates one chat's observable logical execution state in a single publication.
    #[allow(non_snake_case)]
    fn updateChatExecutionState(
        &mut self,
        chatId: String,
        update: impl FnOnce(&mut ChatExecutionState),
    ) {
        let key = Self::chatKey(Some(chatId.clone()));
        let mut states = self.executionStateByChatIdFlow.value();
        update(states.entry(key).or_insert_with(ChatExecutionState::idle));
        let changedState = states
            .get(&Self::chatKey(Some(chatId.clone())))
            .cloned()
            .expect("chat execution state must exist after update");
        self.executionStateByChatIdFlow.set_value(states);
        self.initializeCurrentTurnToolInvocationCount(&chatId);
        self.dispatchToolPkgChatRuntimeStateChange(&chatId, &changedState.inputProcessingState);
        self.publishWorkState(&chatId, &changedState.inputProcessingState);
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
        let activeChatIds = states
            .iter()
            .filter_map(|(chatId, state)| state.isLoading.then_some(chatId.clone()))
            .collect::<Vec<_>>();
        self.executionStateByChatIdFlow.set_value(states);
        for chatId in activeChatIds {
            self.initializeCurrentTurnToolInvocationCount(&chatId);
        }
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
            .entry(key.clone())
            .or_insert_with(ChatExecutionState::idle)
            .inputProcessingState = state;
        let changedState = states
            .get(&key)
            .cloned()
            .expect("chat execution state must exist after input processing state update");
        self.executionStateByChatIdFlow.set_value(states);
        self.initializeCurrentTurnToolInvocationCount(&key);
        self.dispatchToolPkgChatRuntimeStateChange(&key, &changedState.inputProcessingState);
        self.publishWorkState(&key, &changedState.inputProcessingState);
    }

    /// Publishes processing changes to the exact work item owned by this runtime.
    fn publishWorkState(&self, chatId: &str, state: &InputProcessingState) {
        let workId = self.withExistingRuntime(Some(chatId.to_string()), |runtime| runtime.workId.clone()).flatten();
        if let Some(workId) = workId {
            crate::services::WorkActivityService::updateWork(&workId, state);
        }
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

    /// Reports whether post-response summary UI is active for one chat.
    #[allow(non_snake_case)]
    fn hasPendingAsyncSummaryUiForChat(&self, chatId: &str) -> bool {
        self.pendingAsyncSummaryUiByChatId
            .lock()
            .expect("pending async summary map mutex poisoned")
            .contains_key(chatId)
    }

    /// Registers one summary task without replacing an active operation for the same chat.
    pub(crate) fn registerSummaryTask(
        &self,
        chatId: String,
        task: ChatSummaryTask,
    ) -> Result<(), String> {
        self.summaryTasksByChatId.register(chatId, task)
    }

    /// Reads the actual summary operation shared by every clone serving this chat.
    pub(crate) fn summaryTaskForChat(&self, chatId: &str) -> Option<ChatSummaryTask> {
        self.summaryTasksByChatId.get(chatId)
    }

    /// Aborts selected host-owned summaries without cancelling another chat's provider stream.
    pub(crate) fn cancelSummaryTasks(&self, chatId: Option<&str>) {
        self.summaryTasksByChatId.cancel(chatId);
    }

    /// Clears the finishing summary's UI before allowing another task to claim the chat.
    pub(crate) fn finishSummaryTask(&mut self, chatId: &str, taskId: &Arc<()>) {
        let ownsTask = self
            .summaryTasksByChatId
            .get(chatId)
            .is_some_and(|task| Arc::ptr_eq(&task.taskId, taskId));
        if !ownsTask {
            return;
        }
        self.setPendingAsyncSummaryUiForChat(chatId.to_string(), false);
        self.setSuppressIdleCompletedStateForChat(chatId.to_string(), false);
        let summarizing = self
            .executionStateByChatIdFlow
            .value()
            .get(chatId)
            .is_some_and(|state| {
                matches!(
                    state.inputProcessingState,
                    InputProcessingState::Summarizing { .. }
                )
            });
        if summarizing {
            self.setInputProcessingStateForChat(chatId.to_string(), InputProcessingState::Idle);
        }
        self.summaryTasksByChatId.finish(chatId, taskId);
    }
    /// Updates input-processing state for a concrete chat id.
    #[allow(non_snake_case)]
    pub fn setInputProcessingStateForChat(&mut self, chatId: String, state: InputProcessingState) {
        self.setChatInputProcessingState(Some(chatId), state);
    }

    /// Publishes provider state only while it belongs to the active chat turn.
    #[allow(non_snake_case)]
    fn setInputProcessingStateForChatIfCurrent(
        &mut self,
        chatId: String,
        turnId: u64,
        state: InputProcessingState,
    ) {
        if self.isCurrentChatTurn(&chatId, turnId) {
            self.setInputProcessingStateForChat(chatId, state);
        }
    }

    /// Publishes the beginning of one logical chat execution atomically.
    #[allow(non_snake_case)]
    fn startChatExecution(&mut self, chatId: String, title: String) -> crate::services::WorkActivityService::WorkRequestGuard {
        let workId = uuid::Uuid::new_v4().to_string();
        let (turnId, startedAtMillis) = self.withRuntime(Some(chatId.clone()), |runtime| {
            runtime.workId = Some(workId.clone());
            (runtime.activeTurnId, runtime.requestSentAt)
        });
        crate::services::WorkActivityService::beginWork(workId.clone(), chatId.clone(), title,
            turnId, startedAtMillis, self.clone_for_core());
        ToolProgressBus::clear();
        self.updateChatExecutionState(chatId, |state| {
            state.isLoading = true;
            state.inputProcessingState = InputProcessingState::Processing {
                message: "message_processing".to_string(),
            };
        });
        crate::services::WorkActivityService::WorkRequestGuard::new(workId)
    }

    /// Publishes the terminal state of one logical chat execution atomically.
    #[allow(non_snake_case)]
    pub fn finishChatExecution(&mut self, chatId: String, terminalState: InputProcessingState) {
        if let InputProcessingState::Error { message } = &terminalState {
            self.failTurnReceipt(&chatId, message.clone());
        }
        debug_assert!(matches!(
            &terminalState,
            InputProcessingState::Idle
                | InputProcessingState::Completed
                | InputProcessingState::Error { .. }
        ));
        self.withExistingRuntime(Some(chatId.clone()), |runtime| {
            if let Some(lease) = runtime.executionLease.take() {
                if !runtime.currentTurnOptions.deferSequenceCompletion {
                    lease.release().expect("Terminal execution lease release must succeed");
                }
            }
            runtime.isLoading = false;
            runtime.sendJob = None;
            runtime.responseStream = None;
            runtime.streamCollectionJob = None;
            runtime.stateCollectionJob = None;
            runtime.activeStreamingAiMessage = None;
        });
        ToolProgressBus::clear();
        self.updateChatExecutionState(chatId, |state| {
            state.isLoading = false;
            state.inputProcessingState = terminalState;
        });
    }

    /// Publishes summary UI after a reply stream has finished for the same turn.
    #[allow(non_snake_case)]
    fn finishChatExecutionWithSummaryForTurn(&mut self, chatId: String, turnId: u64) -> bool {
        if !self.isCurrentChatTurn(&chatId, turnId) {
            return false;
        }
        self.withExistingRuntime(Some(chatId.clone()), |runtime| {
            if let Some(lease) = runtime.executionLease.take() {
                if !runtime.currentTurnOptions.deferSequenceCompletion {
                    lease.release().expect("Completed execution lease release must succeed");
                }
            }
        });
        self.updateChatExecutionState(chatId, |state| {
            state.isLoading = false;
            state.inputProcessingState = InputProcessingState::Summarizing {
                message: "message_summarizing".to_string(),
            };
        });
        true
    }

    /// Builds model-ready user message content with attachments, workspace, and reply context.
    #[allow(non_snake_case)]
    pub async fn buildUserMessageContentForSend(
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
        let finalMessageContent =
            AIMessageManager::buildUserMessageContent(BuildUserMessageContentRequest {
                messageText: request.messageText,
                proxySenderName: request.proxySenderNameOverride,
                attachments: request.attachments,
                fileSystemHost: self.fileSystemHost.clone(),
                workspacePath: request.workspacePath,
                replyToMessage: request.replyToMessage,
                enableDirectImageProcessing,
                enableDirectAudioProcessing,
                enableDirectVideoProcessing,
                chatId: Some(request.chatId.clone()),
                participantId: Some(request.participantId),
            })
            .await?;
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
            turnId: runtime.activeTurnId,
            aiMessage: runtime.activeStreamingAiMessage.clone(),
            partialContent: runtime
                .responseStream
                .as_ref()
                .map(|stream| stream.current_render_content())
                .unwrap_or_default(),
            turnOptions: runtime.currentTurnOptions.clone(),
        })
    }

    /// Removes and returns the active streaming AI message for a chat.
    #[allow(non_snake_case)]
    pub fn detachStreamingAiMessage(&mut self, chatId: String) -> Option<ChatMessage> {
        let snapshot = self.readCurrentTurnCancellationSnapshot(chatId)?;
        let mut aiMessage = snapshot.aiMessage?;
        let mut partStream = AssistantMarkupStreamState::new();
        partStream
            .resetToSnapshot(&snapshot.partialContent)
            .expect("cancelled assistant snapshot must parse into message parts");
        aiMessage.parts = partStream
            .finish()
            .expect("cancelled assistant markup must parse into message parts");
        if let Some(observer) = &snapshot.turnOptions.outputObserver {
            observer.snapshot(aiMessage.timestamp, &aiMessage.parts);
        }
        aiMessage.contentStream = None;
        if let Some(presentation) = aiMessage.pluginExtensions.get_mut(PRESENTATION_EXTENSION) {
            presentation["pending"] = serde_json::Value::Bool(false);
        }
        aiMessage.completedAt = messageTimingNow().startedAtMs as i64;
        Some(aiMessage)
    }

    /// Matches only an admitted native execution while it owns the actual current streaming runtime.
    pub(crate) fn isNativeExecutionActive(&self, chatId: &str, executionId: &str) -> bool {
        match self.withExistingRuntime(Some(chatId.to_string()), |runtime| {
            runtime.isLoading && !runtime.isCancelling
                && runtime.currentTurnOptions.nativeExecutionId.as_deref() == Some(executionId)
        }) { Some(active) => active, None => false }
    }

    /// Cancels an active message turn and optionally keeps partial response content.
    #[allow(non_snake_case)]
    pub async fn cancelMessageInternal(
        &mut self,
        chatId: String,
        keepPartialResponse: bool,
    ) -> Option<ChatMessage> {
        self.cancelMessageInternalForTurn(chatId, keepPartialResponse, None).await
            .expect("Unscoped cancellation cannot fail generation validation")
    }

    /// Cancels only the execution reserved by an exact background-work action.
    pub(crate) async fn cancelWorkGeneration(&mut self, chatId: String, turnId: u64) -> Result<(), String> {
        self.cancelMessageInternalForTurn(chatId, true, Some(turnId)).await.map(|_| ())
    }

    /// Reserves cancellation and validates optional generation ownership under one runtime lock.
    async fn cancelMessageInternalForTurn(&mut self, chatId: String, keepPartialResponse: bool,
        expectedTurnId: Option<u64>) -> Result<Option<ChatMessage>, String> {
        if expectedTurnId.is_none() { self.cancelSummaryTasks(Some(&chatId)); }
        let cancellation = self.withExistingRuntime(Some(chatId.clone()), |runtime| {
            runtime.reserveCancellation(expectedTurnId).map(|reservation|
                reservation.map(|(cancelled, cancelling)| (cancelled, cancelling, runtime.responseStream.clone())))
        });
        let Some(reservation) = cancellation else {
            if expectedTurnId.is_some() { return Err("Work runtime no longer exists".to_string()); }
            return Ok(None);
        };
        let Some((cancelledTurnId, cancellationTurnId, responseStream)) = reservation? else { return Ok(None); };
        if expectedTurnId.is_some() { self.cancelSummaryTasks(Some(&chatId)); }
        let partialMessage = keepPartialResponse
            .then(|| self.detachStreamingAiMessage(chatId.clone()))
            .flatten();
        self.clearCurrentTurnToolInvocationCount(chatId.clone());
        // Cancel the producer first. Closing the stream earlier lets its cleanup task
        // remove the active service before AIMessageManager can invalidate execution.
        AIMessageManager::cancelOperation(chatId.clone()).await;
        if let Some(responseStream) = responseStream {
            responseStream.close();
        }
        let cancelled = self
            .withExistingRuntime(Some(chatId.clone()), |runtime| {
                if runtime.activeTurnId != cancellationTurnId || !runtime.isCancelling {
                    return false;
                }
                if let Some(lease) = runtime.executionLease.take() {
                    if !runtime.currentTurnOptions.deferSequenceCompletion {
                        lease.release().expect("Cancelled execution lease release must succeed");
                    }
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
                runtime.isCancelling = false;
                true
            })
            .unwrap_or(false);
        if cancelled {
            AppLogger::v_with_level(
                "ResponseExecutionTrace",
                &format!(
                    "response_cancelled chatId={} turnId={}",
                    chatId, cancelledTurnId
                ),
                operit_util::AppLogger::VERBOSE_LEVEL_2,
            );
            self.finishChatExecution(chatId, InputProcessingState::Idle);
        }
        Ok(partialMessage)
    }

    /// Cancels an active message turn while preserving partial response content.
    #[allow(non_snake_case)]
    pub async fn cancelMessage(&mut self, chatId: String) -> Option<ChatMessage> {
        self.cancelMessageInternal(chatId, true).await
    }

    /// Cancels an active message turn before destructive history mutation.
    #[allow(non_snake_case)]
    pub async fn cancelMessageForDestructiveMutation(&mut self, chatId: String) {
        let _ = self.cancelMessageInternal(chatId, false).await;
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
        let value = counts
            .get(&chatId)
            .copied()
            .expect("tool invocation count must be initialized before incrementing")
            + 1;
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
        let mut executionGuard = request.executionLease.requestGuard();
        if request.turnOptions.deferSequenceCompletion {
            executionGuard.handoff();
        }
        let chatId = request.chatId.clone();
        let configuration = request.chatConfiguration.clone();
        configuration.validate().map_err(
            operit_providers::chat::llmprovider::AIService::AiServiceError::RequestFailed,
        )?;
        request.chatProviderIdOverride =
            Some(configuration.profile.modelBinding.providerId.clone());
        request.chatModelIdOverride = Some(configuration.profile.modelBinding.modelId.clone());
        let originalMessageText = request.messageText.trim().to_string();
        ChainLogger::verbose_with_level(
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

            ],
            operit_util::AppLogger::VERBOSE_LEVEL_2,
        );
        self.resetCurrentTurnToolInvocationCount(chatId.clone());
        let workTitle = request.chatHistoryDelegate.chatHistoriesFlow().value().into_iter()
            .find(|history| history.id == chatId)
            .ok_or_else(|| operit_providers::chat::llmprovider::AIService::AiServiceError::RequestFailed(
                format!("Work conversation metadata is missing: {chatId}")))?.title;
        let workTitle = format!("{} · {}", workTitle,
            originalMessageText.chars().map(|character| if character.is_control() { ' ' } else { character }).take(80).collect::<String>());
        let turnId = self.beginChatTurn(
            chatId.clone(),
            request.turnOptions.clone(),
            request.executionLease.clone(),
        );
        let (receiptSender, receiptReceiver) = oneshot::channel();
        self.withRuntime(Some(chatId.clone()), |runtime| {
            runtime.receipt = Some(Arc::new(Mutex::new(ChatTurnReceiptState {
                turnId, nativeExecutionId: request.turnOptions.nativeExecutionId.clone(), sender: Some(receiptSender),
                result: MessageSendResultData {
                    chatId: chatId.clone(), message: request.messageText.clone(),
                    aiResponse: JsOptional::Null, receivedAt: JsOptional::Null, sentAt: runtime.requestSentAt,
                    outcome: if request.turnOptions.persistTurn {
                        MessageSendOutcome::Committed {
                            status: MessageSendStatus::Completed,
                            userMessageTimestamp: request.turnOptions.continuation.as_ref().map(|turn| turn.userMessageTimestamp),
                            assistant: None,
                        }
                    } else {
                        MessageSendOutcome::NotPersisted { status: MessageSendStatus::Completed }
                    },
                },
            })));
        });
        let mut workGuard = self.startChatExecution(chatId.clone(), workTitle);

        let finalMessageContent = if request.turnOptions.continuation.is_some() {
            String::new()
        } else { match self
            .buildUserMessageContentForSend(BuildUserMessageContentForSendRequest {
                messageText: originalMessageText.clone(),
                proxySenderNameOverride: request.proxySenderNameOverride.clone(),
                attachments: request.attachments.clone(),
                workspacePath: request.workspacePath.clone(),
                replyToMessage: request.replyToMessage.clone(),
                chatId: chatId.clone(),
                participantId: configuration.profile.id.clone(),
                chatProviderIdOverride: request.chatProviderIdOverride.clone(),
                chatModelIdOverride: request.chatModelIdOverride.clone(),
            })
            .await
        {
            Ok(content) => content,
            Err(error) => {
                ChainLogger::error(
                    SEND_CHAIN,
                    "send.processing.build_user_content.error",
                    &[("chatId", chatId.clone()), ("error", error.to_string())],
                );
                if self.cleanupRuntimeAfterTurn(chatId.clone(), turnId) {
                    self.finishChatExecutionForTurn(
                        chatId.clone(),
                        turnId,
                        InputProcessingState::Error {
                            message: error.to_string(),
                        },
                    );
                }
                return Err(error);
            }
        } };
        let shouldAddUserMessageToChat = request.turnOptions.persistTurn
            && request.turnOptions.continuation.is_none()
            && !request.suppressUserMessageInHistory
            && !(request.isAutoContinuation
                && originalMessageText.is_empty()
                && request.attachments.is_empty())
            ;
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
        if let Some(continuation) = &request.turnOptions.continuation {
            userMessage.timestamp = continuation.userMessageTimestamp;
        }
        if shouldAddUserMessageToChat {
        request
            .executionLease
            .protectRevision(userMessage.timestamp, 0)
            .map_err(|error| {
                operit_providers::chat::llmprovider::AIService::AiServiceError::RequestFailed(
                    error.to_string(),
                )
            })?;
        AIMessageManager::applyConfigurationIdentity(&mut userMessage, &configuration);
        }
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
            ChainLogger::verbose(
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
            let committedUser = request.chatHistoryDelegate.commitUserMessage(&chatId, userMessage.clone())
                .map_err(operit_providers::chat::llmprovider::AIService::AiServiceError::RequestFailed)?;
            ChainLogger::verbose(
                MESSAGE_STORE_CHAIN,
                "message.store.user.done",
                &[
                    ("chatId", chatId.clone()),
                    ("timestamp", userMessage.timestamp.to_string()),
                ],
            );

            self.recordCommittedUser(&chatId, turnId, committedUser.timestamp);
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
            let activeTurnId = turnId;
            let mut stateDelegate = self.clone_for_core();
            let stateFlow = request.enhancedAiService.inputProcessingState();
            stateFlow.subscribe(move |state| {
                stateDelegate.setInputProcessingStateForChatIfCurrent(
                    activeChatId.clone(),
                    activeTurnId,
                    state,
                );
            });
        }

        let participantName = configuration.profile.name.clone();
        let requestMessageContent = finalMessageContent;
        let assistantMessageTimestamp = ChatMessageTimestampAllocator::next();
        let tokenLimitExceeded = Arc::new(AtomicBool::new(false));
        let tokenUsageThreshold = if request.turnOptions.persistTurn {
            request.tokenUsageThreshold
        } else {
            f64::MAX
        };
        let completionStream = match AIMessageManager::sendMessage(AIMessageSendRequest {
            chatConfiguration: configuration.clone(),
            enhancedAiService: request.enhancedAiService,
            chatId: Some(chatId.clone()),
            messageContent: requestMessageContent,
            chatHistory: request.chatHistory,
            promptHistoryOverride: request.promptHistoryOverride.clone(),
            workspacePath: request.workspacePath.clone(),
            workspaceFolders: request.workspaceFolders.clone(),
            promptFunctionType: request.promptFunctionType.clone(),
            enableThinking: request.enableThinking,
            maxTokens: request.maxTokens,
            tokenUsageThreshold,
            proxySenderName: request.proxySenderNameOverride.clone(),
            notifyReplyOverride: request.turnOptions.notifyReply,
            chatProviderIdOverride: request.chatProviderIdOverride.clone(),
            chatModelIdOverride: request.chatModelIdOverride.clone(),
            disableWarning: request.turnOptions.disableWarning,
            callbacks: Some(Arc::new(MessageProcessingCallbacks {
                nonFatalErrorEventFlow: self.nonFatalErrorEventFlow.clone(),
                tokenLimitExceeded: tokenLimitExceeded.clone(),
            })),
            onToolInvocation: None,
            resume: request.isResume,
        })
        .await
        {
            Ok(stream) => {
                ChainLogger::verbose_with_level(
                    RECEIVE_CHAIN,
                    "receive.stream.created",
                    &[("chatId", chatId.clone())],
                    operit_util::AppLogger::VERBOSE_LEVEL_2,
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
                if self.cleanupRuntimeAfterTurn(chatId.clone(), turnId) {
                    self.finishChatExecutionForTurn(
                        chatId.clone(),
                        turnId,
                        InputProcessingState::Error {
                            message: error.to_string(),
                        },
                    );
                }
                return Err(error);
            }
        };
        let sharedResponseStream = completionStream.clone();
        sharedResponseStream.set_initial_content(String::new());
        let streamAccepted = self
            .withExistingRuntime(Some(chatId.clone()), |runtime| {
                if runtime.activeTurnId != turnId || runtime.isCancelling {
                    return false;
                }
                runtime.responseStream = Some(sharedResponseStream.clone());
                true
            })
            .unwrap_or(false);
        let modelIdentity = request
            .enhancedAiService
            .getLastTurnModelIdentity()
            .expect("accepted response stream must have a resolved model identity");
        let mut aiMessage = ChatMessage {
            sender: "ai".to_string(),
            timestamp: assistantMessageTimestamp,
            roleName: participantName.clone(),
            provider: modelIdentity.providerName,
            modelName: modelIdentity.modelName,
            inputTokens: 0,
            outputTokens: 0,
            cachedInputTokens: 0,
            displayMode: ChatMessageDisplayMode::NORMAL,
            ..ChatMessage::new("ai".to_string())
        };
        request
            .executionLease
            .protectRevision(aiMessage.timestamp, 0)
            .map_err(|error| {
                operit_providers::chat::llmprovider::AIService::AiServiceError::RequestFailed(
                    error.to_string(),
                )
            })?;
        AIMessageManager::applyConfigurationIdentity(&mut aiMessage, &configuration);
        let streamKey = format!("chat-message-stream:{}:{}", chatId, aiMessage.timestamp);
        let segmentSource = coreResponseStreamSource(sharedResponseStream.clone(), streamKey);
        aiMessage.contentStream = Some(CoreStream::fromSourceWithId(
            format!("chat-message-stream:{}:{}", chatId, aiMessage.timestamp),
            segmentSource,
        ));
        if !streamAccepted || !self.isCurrentChatTurn(&chatId, turnId) {
            sharedResponseStream.close();
            return Ok(SendUserMessageProcessingResult {
                completion: ChatTurnSubmission::Pending(receiptReceiver),
                aiMessage,
                nextWindowSize: None,
            });
        }
        self.withExistingRuntime(Some(chatId.clone()), |runtime| {
            runtime.activeStreamingAiMessage = Some(aiMessage.clone());
        });
        AppLogger::v_with_level(
            "ResponseExecutionTrace",
            &format!(
                "response_started chatId={} timestamp={}",
                chatId, aiMessage.timestamp,
            ),
            operit_util::AppLogger::VERBOSE_LEVEL_2,
        );
        let workerChatId = chatId.clone();
        let workerUserMessageTimestamp = userMessage.timestamp;
        let workerTurnId = turnId;
        let workerTurnOptions = request.turnOptions.clone();
        let workerAiMessage = Arc::new(Mutex::new(aiMessage.clone()));
        let workerResponseStream = sharedResponseStream.clone();
        let workerRevisionTracker = Arc::new(Mutex::new(TextStreamRevisionTracker::new("")));
        let workerPartStream = Arc::new(Mutex::new(AssistantMarkupStreamState::new()));
        let workerService = request.enhancedAiService.clone();
        let workerChatHistoryDelegate =
            Arc::new(Mutex::new(request.chatHistoryDelegate.clone_for_core()));
        let workerMessageProcessingDelegate = Arc::new(Mutex::new(self.clone_for_core()));
        let presentationSession = if workerTurnOptions.persistTurn {
            let activeDelegate = workerMessageProcessingDelegate.clone();
            let activeChatId = workerChatId.clone();
            let publishDelegate = workerMessageProcessingDelegate.clone();
            let publishHistory = workerChatHistoryDelegate.clone();
            let publishChatId = workerChatId.clone();
            match ToolPkgChatResponseHookBridge::start(
                workerChatId.clone(), workerAiMessage.clone(),
                Arc::new(move || activeDelegate.lock().expect("presentation delegate mutex poisoned").isCurrentChatTurn(&activeChatId, workerTurnId)),
                Arc::new(move |message| {
                    let delegate = publishDelegate.lock().expect("presentation delegate mutex poisoned");
                    if !delegate.isCurrentChatTurn(&publishChatId, workerTurnId) { return Ok(()); }
                    publishHistory.lock().expect("presentation history mutex poisoned")
                        .commitAssistantMessageSegment(publishChatId.clone(), message.clone(), None)?;
                    delegate.withExistingRuntime(Some(publishChatId.clone()), |runtime| {
                        runtime.activeStreamingAiMessage = Some(message);
                    });
                    Ok(())
                }),
            ).await {
                Ok(session) => session,
                Err(message) => {
                    sharedResponseStream.close();
                    self.cleanupRuntimeAfterTurn(chatId.clone(), turnId);
                    self.finishChatExecutionForTurn(chatId.clone(), turnId, InputProcessingState::Error { message: message.clone() });
                    return Err(operit_providers::chat::llmprovider::AIService::AiServiceError::RequestFailed(message));
                }
            }
        } else { None };
        let presentationSession = Arc::new(Mutex::new(presentationSession));
        aiMessage = workerAiMessage.lock().expect("presentation message mutex poisoned").clone();
        self.withExistingRuntime(Some(chatId.clone()), |runtime| {
            runtime.activeStreamingAiMessage = Some(aiMessage.clone());
        });
        let completionPresentationSession = presentationSession.clone();
        let completionContextWorkspacePath = request.workspacePath.clone();
        let completionContextPromptFunctionType = request.promptFunctionType.clone();
        let completionContextConfiguration = configuration.clone();
        let completionContextEnableThinking = request.enableThinking;
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
            ChainLogger::verbose(
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
        let chunkPartStream = workerPartStream.clone();
        let chunkOutputObserver = workerTurnOptions.outputObserver.clone();
        let chunkMessageTimestamp = aiMessage.timestamp;
        let completionChatId = workerChatId.clone();
        let completionTurnId = workerTurnId;
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
        let persistChatId = completionChatId.clone();
        let persistTurnId = completionTurnId;
        let persistTurnOptions = completionTurnOptions.clone();
        let persistAiMessage = completionAiMessage.clone();
        let persistChatHistoryDelegate = completionChatHistoryDelegate.clone();
        let persistMessageProcessingDelegate = completionMessageProcessingDelegate.clone();
        let mut lastStreamingPersistAt = messageTimingNow().startedAtMs as i64;
        let mut persistStreamingSnapshot = move |content: &str| {
            if !persistTurnOptions.persistTurn {
                return;
            }
            let now = messageTimingNow().startedAtMs as i64;
            if now.saturating_sub(lastStreamingPersistAt) < 1000 {
                return;
            }
            lastStreamingPersistAt = now;
            if !persistMessageProcessingDelegate
                .lock()
                .expect("worker message processing delegate mutex poisoned")
                .isCurrentChatTurn(&persistChatId, persistTurnId)
            {
                return;
            }
            let mut streamingMessage = persistAiMessage
                .lock()
                .expect("worker AI message mutex poisoned")
                .clone();
            let mut snapshotPartStream = AssistantMarkupStreamState::new();
            snapshotPartStream
                .resetToSnapshot(content)
                .expect("streaming assistant snapshot must parse into message parts");
            streamingMessage.parts = snapshotPartStream
                .finish()
                .expect("streaming assistant snapshot must finish into message parts");
            persistChatHistoryDelegate
                .lock()
                .expect("worker chat history mutex poisoned")
                .addMessageToChat(streamingMessage, Some(persistChatId.clone()));
        };
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
                                                if tracker.rollback(&event.id).is_none() {
                                                    AppLogger::w(
                                                        "CoreStreamTrace",
                                                        &format!(
                                                            "response rollback savepoint missing; keeping current content id={}",
                                                            event.id
                                                        ),
                                                    );
                                                }
                                            }
                                        }
                                        if event.event_type
                                            == operit_util::stream::RevisableTextStream::TextStreamEventType::Rollback
                                        {
                                            let snapshot = tracker.current_content().to_owned();
                                            drop(tracker);
                                            if let Some(observer) = &chunkOutputObserver {
                                                let mut parser = chunkPartStream.lock().expect("semantic stream parser mutex poisoned");
                                                match parser.resetToSnapshot(&snapshot) {
                                                    Ok(()) => observer.snapshot(chunkMessageTimestamp, parser.parts()),
                                                    Err(error) => observer.failed(error),
                                                }
                                            }
                                            if let Some(session) = presentationSession.lock().expect("presentation session mutex poisoned").as_ref() { session.snapshot(&snapshot); }
                                            persistStreamingSnapshot(&snapshot);
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
                                    ChainLogger::verbose_with_level(
                                        RECEIVE_CHAIN,
                                        "receive.first_chunk",
                                        &[("chatId", chunkChatId.clone())],
                                        operit_util::AppLogger::VERBOSE_LEVEL_2,
                                    );
                                }
                                drop(firstResponseElapsed);
                                let snapshot = {
                                    let mut tracker = chunkRevisionTracker
                                        .lock()
                                        .expect("revision tracker mutex poisoned");
                                    tracker.append(&chunk);
                                    tracker
                                        .current_content()
                                        .to_owned()
                                };
                                if let Some(observer) = &chunkOutputObserver {
                                    let mut parser = chunkPartStream.lock().expect("semantic stream parser mutex poisoned");
                                    match parser.pushSnapshot(&snapshot) {
                                        Ok(()) => observer.snapshot(chunkMessageTimestamp, parser.parts()),
                                        Err(error) => observer.failed(error),
                                    }
                                }
                                if let Some(session) = presentationSession.lock().expect("presentation session mutex poisoned").as_ref() { session.snapshot(&snapshot); }
                                persistStreamingSnapshot(&snapshot);
                            })
                            .await;
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
                        if !completionMessageProcessingDelegate
                            .lock()
                            .expect("worker message processing delegate mutex poisoned")
                            .isCurrentChatTurn(&completionChatId, completionTurnId)
                        {
                            return;
                        }
                        let finalContent = completionRevisionTracker
                            .lock()
                            .expect("revision tracker mutex poisoned")
                            .current_content()
                            .to_owned();
                        let presentation = completionPresentationSession.lock().expect("presentation session mutex poisoned").take();
                        let presentationError = match presentation {
                            Some(session) => session.finish(finalContent.clone()).await.err(),
                            None => None,
                        };
                        if !completionMessageProcessingDelegate.lock().expect("worker message processing delegate mutex poisoned")
                            .isCurrentChatTurn(&completionChatId, completionTurnId) { return; }
                        let mut workerService = workerService;
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
                                    "terminal assistant snapshot must parse into message parts",
                                );
                                partStream.finish().expect(
                                    "terminal assistant markup must parse into message parts",
                                )
                            };
                            let mut workerAiMessage = completionAiMessage
                                .lock()
                                .expect("worker AI message mutex poisoned");
                            workerAiMessage.inputTokens += tokenSnapshot.inputTokens;
                            workerAiMessage.outputTokens += tokenSnapshot.outputTokens;
                            workerAiMessage.cachedInputTokens += tokenSnapshot.cachedInputTokens;
                            if let Some(observer) = &completionTurnOptions.outputObserver {
                                observer.snapshot(workerAiMessage.timestamp, &parts);
                            }
                            workerAiMessage.parts = parts;
                            if let Some(presentation) = workerAiMessage.pluginExtensions.get_mut(PRESENTATION_EXTENSION) {
                                presentation["pending"] = serde_json::Value::Bool(false);
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
                        if let Some(mut error) = completionResponseStream.terminal_failure().or(presentationError) {
                            ChainLogger::error(
                                RECEIVE_CHAIN,
                                "receive.stream.failed",
                                &[
                                    ("chatId", completionChatId.clone()),
                                    ("error", error.clone()),
                                ],
                            );
                            if completionTurnOptions.persistTurn {
                                if !completionMessageProcessingDelegate
                                    .lock()
                                    .expect("worker message processing delegate mutex poisoned")
                                    .isCurrentChatTurn(&completionChatId, completionTurnId)
                                {
                                    return;
                                }
                                let failedMessage = ChatMessage {
                                    contentStream: None,
                                    ..finalMessage
                                };
                                let committed = completionChatHistoryDelegate
                                    .lock()
                                    .expect("worker chat history mutex poisoned")
                                    .commitAssistantMessageSegment(
                                        completionChatId.clone(), failedMessage.clone(), None,
                                    );
                                match committed {
                                    Ok(_) => completionMessageProcessingDelegate
                                        .lock()
                                        .expect("worker message processing delegate mutex poisoned")
                                        .recordCommittedAssistant(
                                            &completionChatId, completionTurnId, &failedMessage, 0,
                                        ),
                                    Err(commitError) => {
                                        error = format!("{error}; failed to persist assistant response: {commitError}");
                                    }
                                }
                            }
                            let mut delegate = completionMessageProcessingDelegate
                                .lock()
                                .expect("worker message processing delegate mutex poisoned");
                            delegate.cleanupRuntimeAfterTurn(
                                completionChatId.clone(),
                                completionTurnId,
                            );
                            delegate.finishChatExecutionForTurn(
                                completionChatId.clone(),
                                completionTurnId,
                                InputProcessingState::Error { message: error },
                            );
                            return;
                        }
                        if workerTurnOptions.persistTurn {
                            ChainLogger::verbose(
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
                        let pendingRouteChange = workerService.takePendingRouteChange();
                        let routeResumeContext =
                            pendingRouteChange.as_ref().map(|_| {
                                let mut completionChatHistory = {
                                    let delegate = completionChatHistoryDelegate
                                        .lock().expect("worker chat history mutex poisoned");
                                    delegate.getRuntimeChatHistory(completionChatId.clone())
                                };
                                if workerTurnOptions.persistTurn {
                                    let persistedAssistant = completionChatHistory.iter_mut()
                                        .find(|message| message.timestamp == finalMessage.timestamp)
                                        .expect("persisted assistant placeholder must remain in chat history");
                                    *persistedAssistant = finalMessage.clone();
                                }
                                CoreRouteResumeContext {
                                    runtimeChatHistory: completionChatHistory,
                                    workspacePath: completionContextWorkspacePath.clone(),
                                    promptFunctionType: completionContextPromptFunctionType.clone(),
                                    enableThinking: completionContextEnableThinking,
                                    participantId: completionContextConfiguration.profile.id.clone(),
                                    extensionOwner: completionContextConfiguration.extensionOwner.clone(),
                                    proxySenderName: completionContextProxySenderName.clone(),
                                    notifyReplyOverride: completionTurnOptions.notifyReply,
                                    chatProviderIdOverride: completionContextProviderIdOverride.clone(),
                                    chatModelIdOverride: completionContextModelIdOverride.clone(),
                                    turnOptions: completionTurnOptions.clone(),
                                }
                            });
                        // The provider completed this estimate from the actual prepared
                        // request and final reply before closing the response stream.
                        // Completion must not rebuild a send runtime or invoke plugins.
                        let nextWindowSize = workerService.getLastTurnContextWindow();
                        if !completionMessageProcessingDelegate
                            .lock()
                            .expect("worker message processing delegate mutex poisoned")
                            .isCurrentChatTurn(&completionChatId, completionTurnId)
                        {
                            return;
                        }
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
                            if !completionMessageProcessingDelegate
                                .lock()
                                .expect("worker message processing delegate mutex poisoned")
                                .isCurrentChatTurn(&completionChatId, completionTurnId)
                            {
                                return;
                            }
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
                                delegate.cleanupRuntimeAfterTurn(
                                    completionChatId.clone(),
                                    completionTurnId,
                                );
                                delegate.finishChatExecutionForTurn(
                                    completionChatId.clone(),
                                    completionTurnId,
                                    InputProcessingState::Error { message },
                                );
                                return;
                            }
                            completionMessageProcessingDelegate.lock()
                                .expect("worker message processing delegate mutex poisoned")
                                .recordCommittedAssistant(&completionChatId, completionTurnId, &completedMessage, 0);
                        }
                        drop(workerChatHistoryDelegate);
                        if let Some(routeChange) = pendingRouteChange {
                            ChainLogger::verbose_with_level(
                                SEND_CHAIN,
                                "send.route_change.request",
                                &[
                                    ("chatId", completionChatId.clone()),
                                    ("targetNodeId", routeChange.targetNodeId.clone()),
                                ],
                                operit_util::AppLogger::VERBOSE_LEVEL_2,
                            );
                            if let Err(error) = workerService
                                .requestCoreRouteChange(
                                    completionChatId.clone(),
                                    routeChange.targetNodeId,
                                    routeResumeContext.expect(
                                        "route resume context must be created with route intent",
                                    ),
                                )
                                .await
                            {
                                let message = format!("route change failed: {error}");
                                ChainLogger::error(
                                    SEND_CHAIN,
                                    "send.route_change.error",
                                    &[
                                        ("chatId", completionChatId.clone()),
                                        ("error", message.clone()),
                                    ],
                                );
                                completionMessageProcessingDelegate
                                    .lock()
                                    .expect(
                                        "worker message processing delegate mutex poisoned",
                                    )
                                    .finishChatExecutionForTurn(
                                        completionChatId.clone(),
                                        completionTurnId,
                                        InputProcessingState::Error { message },
                                    );
                                return;
                            }
                            ChainLogger::info(
                                SEND_CHAIN,
                                "send.route_change.completed",
                                &[("chatId", completionChatId.clone())],
                            );
                        }
                        if tokenLimitExceeded.load(Ordering::Acquire) {
                            let mut delegate = completionMessageProcessingDelegate
                                .lock()
                                .expect("worker message processing delegate mutex poisoned")
                                .clone_for_core();
                            let history = completionChatHistoryDelegate
                                .lock()
                                .expect("worker chat history mutex poisoned")
                                .clone_for_core();
                            delegate.withExistingRuntime(Some(completionChatId.clone()), |runtime| {
                                if runtime.activeTurnId == completionTurnId && !runtime.isCancelling {
                                    runtime.activeStreamingAiMessage = Some(ChatMessage {
                                        contentStream: None,
                                        ..finalMessage.clone()
                                    });
                                }
                            });
                            let mut options = completionTurnOptions.clone();
                            options.continuation = Some(ChatTurnContinuation {
                                userMessageTimestamp: workerUserMessageTimestamp,
                            });
                            delegate
                                .continueAfterTokenLimit(
                                    &mut workerService,
                                    history,
                                    completionContextConfiguration.clone(),
                                    completionContextPromptFunctionType.clone(),
                                    options,
                                    completionChatId.clone(),
                                    completionTurnId,
                                )
                                .await;
                            return;
                        }
                        completionMessageProcessingDelegate
                            .lock()
                            .expect("worker message processing delegate mutex poisoned")
                            .finalizeMessageAndNotify(
                                completionChatId.clone(),
                                completionTurnId,
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
        executionGuard.handoff();
        workGuard.handoff();
        Ok(SendUserMessageProcessingResult {
            completion: ChatTurnSubmission::Pending(receiptReceiver),
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
        let workspaceFolders = request
            .chatHistoryDelegate
            .chatHistoryManager
            .getWorkspaceForChat(&request.chatId)
            .ok()
            .flatten()
            .map(|workspace| workspace.folderPaths())
            .unwrap_or_default();
        let result = self
            .sendUserMessage(SendUserMessageProcessingRequest {
                executionLease: request.executionLease,
                chatConfiguration: request.chatConfiguration,
                enhancedAiService: request.enhancedAiService,
                chatHistoryDelegate: request.chatHistoryDelegate,
                chatId: request.chatId,
                messageText: request.requestMessageContent,
                chatHistory: request.requestHistory,
                promptHistoryOverride: None,
                workspacePath: request.workspacePath,
                workspaceFolders,
                promptFunctionType: request.promptFunctionType,
                attachments: request.attachments,
                replyToMessage: request.replyToMessage,
                enableThinking: request.enableThinking,
                maxTokens: request.maxTokens,
                tokenUsageThreshold: request.tokenUsageThreshold,
                chatProviderIdOverride: request.chatProviderIdOverride,
                chatModelIdOverride: request.chatModelIdOverride,
                proxySenderNameOverride: None,
                suppressUserMessageInHistory: true,
                isAutoContinuation: false,
                isResume: false,
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

    /// Waits for compaction and forwards the final continuation receipt to the original sender.
    #[allow(clippy::too_many_arguments)]
    async fn continueAfterTokenLimit(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        history: ChatHistoryDelegate,
        configuration: ChatConfigurationResult,
        promptFunctionType: PromptFunctionType,
        turnOptions: ChatTurnOptions,
        chatId: String,
        turnId: u64,
    ) {
        let ownership = self
            .withExistingRuntime(Some(chatId.clone()), |runtime| {
                if runtime.activeTurnId != turnId || runtime.isCancelling {
                    return None;
                }
                Some((
                    runtime
                        .receipt
                        .clone()
                        .expect("active turn must have a receipt"),
                    runtime
                        .executionLease
                        .clone()
                        .expect("active turn must have an execution lease"),
                ))
            })
            .flatten();
        let Some((receipt, executionLease)) = ownership else {
            return;
        };
        let mut coordinator = MessageCoordinationDelegate::forBackgroundWork(history, self.clone_for_core());
        let result = match coordinator
            .handleTokenLimitExceeded(
                enhancedAiService,
                SummaryContinuationRequest {
                    chatId: chatId.clone(),
                    turnId,
                    participantId: Some(configuration.profile.id.clone()),
                    promptFunctionType,
                    chatProviderIdOverride: Some(configuration.profile.modelBinding.providerId),
                    chatModelIdOverride: Some(configuration.profile.modelBinding.modelId),
                    turnOptions,
                    executionLease,
                },
            )
            .await
        {
            Ok(submission) => submission.wait().await,
            Err(error) => Err(error),
        };
        // Explicit cancellation owns the original receipt until its partial output is committed.
        let originCancelled = self
            .withExistingRuntime(Some(chatId.clone()), |runtime| {
                runtime.activeTurnId != turnId
                    && runtime
                        .receipt
                        .as_ref()
                        .is_some_and(|current| Arc::ptr_eq(current, &receipt))
            })
            .is_some_and(|cancelled| cancelled);
        if originCancelled {
            return;
        }
        if let Err(error) = &result {
            if self.isCurrentChatTurn(&chatId, turnId) {
                self.cleanupRuntimeAfterTurn(chatId.clone(), turnId);
                self.finishChatExecutionForTurn(
                    chatId,
                    turnId,
                    InputProcessingState::Error {
                        message: error.clone(),
                    },
                );
            }
        }
        receipt
            .lock()
            .expect("turn receipt mutex poisoned")
            .deliverContinuation(result);
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
        turnId: u64,
        aiMessage: ChatMessage,
        nextWindowSize: Option<i64>,
        turnOptions: ChatTurnOptions,
    ) {
        if !self.isCurrentChatTurn(&chatId, turnId) {
            return;
        }
        let shouldNotifyReply = turnOptions.persistTurn && !turnOptions.deferSequenceCompletion && turnOptions.notifyReply != Some(false);
        let pendingAsyncSummaryUi = self.hasPendingAsyncSummaryUiForChat(&chatId);
        self.publishWorkState(&chatId, &InputProcessingState::Completed);
        self.cleanupRuntimeAfterTurn(chatId.clone(), turnId);
        if pendingAsyncSummaryUi {
            self.setSuppressIdleCompletedStateForChat(chatId.clone(), true);
            self.finishChatExecutionWithSummaryForTurn(chatId.clone(), turnId);
        } else {
            self.finishChatExecutionForTurn(
                chatId.clone(),
                turnId,
                InputProcessingState::Completed,
            );
        }
        if !turnOptions.deferSequenceCompletion {
        let mut counters = self.turnCompleteCounterByChatIdFlow.value();
        let next = counters.get(&chatId).copied().unwrap_or(0) + 1;
        counters.insert(chatId.clone(), next);
        self.turnCompleteCounterByChatId = counters.clone();
        self.turnCompleteCounterByChatIdFlow.set_value(counters);
        }
        if shouldNotifyReply {
            publishOwnerAppNotification(RuntimeHostInteractionAppNotificationPayload {
                notificationType: "ai_message_completed".to_string(),
                title: "Operit".to_string(),
                message: aiMessageNotificationPreview(&aiMessage.parts),
                chatId: Some(chatId.clone()),
                messageTimestamp: Some(aiMessage.timestamp),
            });
        }
        self.completeTurnReceipt(&chatId, turnId, MessageSendStatus::Completed, Some(&aiMessage));
        let _ = nextWindowSize;
    }

    /// Publishes one native sequence completion using only its actually persisted records and terminal status.
    pub(crate) fn finalizeSequenceAndNotify(
        &mut self, chatId: &str, status: &operit_tools::runtime_support::ChatSequenceStatus,
        error: Option<&str>, hasCommittedUser: bool, assistant: Option<&ChatMessage>, notifyReply: bool,
    ) -> Result<(), String> {
        use operit_tools::runtime_support::ChatSequenceStatus;
        if self.isChatLoading(chatId.to_string()) { return Err("Cannot finalize a sequence with an active generation".to_string()); }
        let terminal = match status {
            ChatSequenceStatus::Completed => InputProcessingState::Completed,
            ChatSequenceStatus::Cancelled => InputProcessingState::Idle,
            ChatSequenceStatus::Failed => InputProcessingState::Error {
                message: error.ok_or_else(|| "Failed sequence omitted its native error".to_string())?.to_string(),
            },
        };
        let mut counters = self.turnCompleteCounterByChatIdFlow.value();
        let nextCounter = if hasCommittedUser && *status != ChatSequenceStatus::Failed {
            let previous = match counters.get(chatId) { Some(previous) => *previous, None => 0 };
            Some(previous.checked_add(1).ok_or_else(|| "Sequence completion counter overflow".to_string())?)
        } else { None };
        self.finishChatExecution(chatId.to_string(), terminal);
        if let Some(next) = nextCounter {
            counters.insert(chatId.to_string(), next);
            self.turnCompleteCounterByChatId = counters.clone();
            self.turnCompleteCounterByChatIdFlow.set_value(counters);
            if *status == ChatSequenceStatus::Completed && notifyReply {
                if let Some(assistant) = assistant {
                    publishOwnerAppNotification(RuntimeHostInteractionAppNotificationPayload {
                        notificationType: "ai_message_completed".to_string(), title: "Operit".to_string(),
                        message: aiMessageNotificationPreview(&assistant.parts), chatId: Some(chatId.to_string()),
                        messageTimestamp: Some(assistant.timestamp),
                    });
                }
            }
        }
        Ok(())
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

/// Builds a compact reply preview from Markdown parts without provider metadata.
fn aiMessageNotificationPreview(parts: &[MessagePart]) -> String {
    const MAX_NOTIFICATION_PREVIEW_CHARACTERS: usize = 240;
    let content: String = MessagePartCodec::orderedParts(parts)
        .into_iter()
        .filter(|part| part.kind == MessagePartKind::Markdown)
        .map(|part| part.content.as_str())
        .collect();
    let mut body = String::with_capacity(content.len());
    let mut cursor = 0;
    for (start, end) in tag_ranges(&content, "meta") {
        body.push_str(&content[cursor..start]);
        cursor = end;
    }
    body.push_str(&content[cursor..]);
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_NOTIFICATION_PREVIEW_CHARACTERS)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        aiMessageNotificationPreview, collectToolPkgChatRuntimeStatistics, ChatExecutionState,
        ChatSummaryTask, ChatSummaryTasks, ChatTurnReceiptState, MessageProcessingCallbacks,
    };
    use futures_util::future::{ready, AbortHandle};
    use futures_util::FutureExt;
    use operit_model::InputProcessingState::InputProcessingState;
    use operit_model::MessagePart::MessagePart;
    use operit_model::MessagePartCodec::MessagePartCodec;
    use operit_plugin_sdk::js_sdk::results::{
        ChatCommittedMessage, JsOptional, MessageSendOutcome, MessageSendResultData,
        MessageSendStatus,
    };
    use operit_providers::chat::EnhancedAIService::SendMessageCallbacks;
    use operit_store::PreferencesDataStore::mutableStateFlow;
    use std::collections::HashMap;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use tokio::sync::oneshot;

    /// Closing an observer ends its transform while production and a fresh live subscriber continue.
    #[tokio::test]
    async fn disposed_response_observer_stops_without_cancelling_generation() {
        use operit_link::{
            CoreEventKind, CoreEventStream, CoreRequestId, CoreValue, CoreWatchRequest,
        };
        use operit_providers::chat::llmprovider::AIService::SharedAiResponseStream;
        use operit_util::stream::HotStream::mutable_shared_stream;

        let producer = SharedAiResponseStream::new_ordered(
            mutable_shared_stream(usize::MAX),
            mutable_shared_stream(usize::MAX),
        );
        let request = CoreWatchRequest {
            requestId: CoreRequestId("observer".to_string()),
            target: "$core.stream.open".to_string(),
            propertyName: "openCoreStream".to_string(),
            args: CoreValue::emptyMap(),
        };
        let (sender, receiver) = CoreEventStream::channel();
        let first = super::collectCoreResponseStream(
            producer.clone(),
            "response".to_string(),
            request.clone(),
            sender,
        );
        tokio::pin!(first);
        assert!(first.as_mut().now_or_never().is_none());
        producer.emit_chunk("before ".to_string());
        assert!(first.as_mut().now_or_never().is_none());
        drop(receiver);
        assert!(first.as_mut().now_or_never().is_some());

        producer.emit_chunk("after".to_string());
        let (sender, mut receiver) = CoreEventStream::channel();
        let second = super::collectCoreResponseStream(
            producer.clone(),
            "response".to_string(),
            request,
            sender,
        );
        tokio::pin!(second);
        assert!(second.as_mut().now_or_never().is_none());
        producer.emit_chunk(" live".to_string());
        assert!(second.as_mut().now_or_never().is_none());
        producer.close();
        second.await;
        let mut text = String::new();
        let mut completed = false;
        while let Some(event) = receiver.recv().await {
            completed |= event.kind == CoreEventKind::Completed;
            if let CoreValue::Map(fields) = event.value {
                if fields.get("type") == Some(&CoreValue::String("chunk".to_string())) {
                    if let Some(CoreValue::String(content)) = fields.get("value") {
                        text.push_str(content);
                    }
                }
            }
        }
        assert_eq!(text, "before after live");
        assert!(completed);
    }

    #[tokio::test]
    async fn response_completion_flushes_pending_nested_inline_syntax_before_closing_transport() {
        use operit_link::{
            CoreEventKind, CoreEventStream, CoreRequestId, CoreValue, CoreWatchRequest,
        };
        use operit_providers::chat::llmprovider::AIService::SharedAiResponseStream;
        use operit_util::stream::HotStream::mutable_shared_stream;

        let producer = SharedAiResponseStream::new_ordered(
            mutable_shared_stream(usize::MAX),
            mutable_shared_stream(usize::MAX),
        );
        producer.emit_chunk("<think>**unfinished [label".to_string());
        producer.close();
        let request = CoreWatchRequest {
            requestId: CoreRequestId("completion".to_string()),
            target: "$core.stream.open".to_string(),
            propertyName: "openCoreStream".to_string(),
            args: CoreValue::emptyMap(),
        };
        let (sender, mut receiver) = CoreEventStream::channel();
        super::collectCoreResponseStream(producer, "response".to_string(), request, sender).await;
        let mut terminal = 0;
        let mut nested_completed = 0;
        let mut text = String::new();
        let mut last_kind = None;
        while let Some(event) = receiver.recv().await {
            assert_eq!(terminal, 0, "no event may follow transport completion");
            if event.kind == CoreEventKind::Completed {
                terminal += 1;
            }
            if let CoreValue::Map(fields) = &event.value {
                if fields.get("type") == Some(&CoreValue::String("completed".to_string()))
                    && event.kind == CoreEventKind::Changed
                {
                    nested_completed += 1;
                }
                if fields.get("type") == Some(&CoreValue::String("markdownInlineChunk".to_string())) {
                    if let Some(CoreValue::String(value)) = fields.get("value") {
                        text.push_str(value);
                    }
                }
            }
            last_kind = Some(event.kind);
        }
        assert_eq!(text, "unfinished [label");
        assert_eq!(terminal, 1);
        assert_eq!(nested_completed, 1);
        assert_eq!(last_kind, Some(CoreEventKind::Completed));
    }

    /// Removes long provider metadata before applying the notification length limit.
    #[test]
    fn notification_preview_excludes_provider_metadata_before_truncation() {
        let content = format!(
            "<meta provider=\"openai:responses_reasoning\">{}</meta>\n回复正文",
            "eyJ".repeat(200)
        );
        let parts = MessagePartCodec::parseAssistantMarkup(&content).unwrap();
        assert_eq!(aiMessageNotificationPreview(&parts), "回复正文");
    }

    /// Omits thinking, tool payloads, and status parts while preserving reply order.
    #[test]
    fn notification_preview_uses_only_ordered_reply_parts() {
        let mut parts = MessagePartCodec::parseAssistantMarkup(
            "<think>private reasoning</think>第一段\n\n\
             <tool name=\"run\"><param name=\"command\">secret</param></tool>\
             <tool_result name=\"run\" status=\"success\"><content>tool payload</content></tool_result>\
             <status type=\"complete\">completed</status>第二段",
        )
        .unwrap();
        parts.reverse();
        assert_eq!(aiMessageNotificationPreview(&parts), "第一段 第二段");
    }

    /// Removes every metadata provider without deleting surrounding reply text.
    #[test]
    fn notification_preview_removes_multiple_metadata_blocks() {
        let parts = vec![MessagePart::markdown(
            "part-0".to_string(),
            0,
            "前<meta provider=\"openai:responses_reasoning\">encoded</meta>中\
             <META provider='gemini:thought_signature'>signature</META>\
             <meta provider=\"openai:responses_output_item\">payload</meta>后"
                .to_string(),
        )];
        assert_eq!(aiMessageNotificationPreview(&parts), "前中后");
    }

    /// Prevents metadata-only and unfinished metadata replies from exposing payloads.
    #[test]
    fn notification_preview_hides_metadata_without_a_reply_body() {
        for content in [
            "<meta provider=\"openai:responses_reasoning\">encoded</meta>",
            "<meta provider=\"openai:responses_reasoning\">encoded",
        ] {
            let parts = vec![MessagePart::markdown(
                "part-0".to_string(),
                0,
                content.to_string(),
            )];
            assert_eq!(aiMessageNotificationPreview(&parts), "");
        }
    }

    /// Normalizes whitespace and counts Unicode characters when limiting reply text.
    #[test]
    fn notification_preview_limits_visible_unicode_characters() {
        let parts = vec![MessagePart::markdown(
            "part-0".to_string(),
            0,
            format!("\n\t你好  世界\n{}", "🙂".repeat(240)),
        )];
        assert_eq!(
            aiMessageNotificationPreview(&parts),
            format!("你好 世界 {}", "🙂".repeat(234)),
        );
    }

    /// Creates a terminal result retaining the committed user identity across a continuation.
    fn summaryContinuationResult(
        message: &str,
        sentAt: i64,
        assistantTimestamp: i64,
    ) -> MessageSendResultData {
        MessageSendResultData {
            chatId: "chat-a".to_string(),
            message: message.to_string(),
            sentAt,
            aiResponse: JsOptional::Value("final reply".to_string()),
            receivedAt: JsOptional::Value(300),
            outcome: MessageSendOutcome::Committed {
                status: MessageSendStatus::Completed,
                userMessageTimestamp: Some(42),
                assistant: Some(ChatCommittedMessage {
                    messageTimestamp: assistantTimestamp,
                    variantIndex: 0,
                }),
            },
        }
    }

    /// Forwards the final assistant revision while keeping the originating submission's content and time.
    #[test]
    fn summary_continuation_receipt_preserves_original_submission() {
        let (sender, mut receiver) = oneshot::channel();
        let mut receipt = ChatTurnReceiptState {
            turnId: 1,
            nativeExecutionId: None,
            sender: Some(sender),
            result: summaryContinuationResult("original input", 100, 200),
        };
        receipt.deliverContinuation(Ok(summaryContinuationResult("", 250, 290)));
        let result = receiver.try_recv().unwrap().unwrap();
        assert_eq!(result.message, "original input");
        assert_eq!(result.sentAt, 100);
        assert!(matches!(
            result.outcome,
            MessageSendOutcome::Committed {
                userMessageTimestamp: Some(42),
                assistant: Some(ChatCommittedMessage {
                    messageTimestamp: 290,
                    variantIndex: 0
                }),
                status: MessageSendStatus::Completed,
            }
        ));
        assert!(
            matches!(result.aiResponse, JsOptional::Value(ref content) if content == "final reply")
        );
    }

    /// Propagates failed compaction instead of reporting the interrupted reply as completed.
    #[test]
    fn summary_continuation_receipt_propagates_failure() {
        let (sender, mut receiver) = oneshot::channel();
        let mut receipt = ChatTurnReceiptState {
            turnId: 1,
            nativeExecutionId: None,
            sender: Some(sender),
            result: summaryContinuationResult("original input", 100, 200),
        };
        receipt.deliverContinuation(Err("summary provider failed".to_string()));
        assert_eq!(
            receiver.try_recv().unwrap().unwrap_err(),
            "summary provider failed"
        );
    }

    /// Creates an identifiable summary operation without using a platform executor.
    fn summaryTask() -> ChatSummaryTask {
        let (abortHandle, _) = AbortHandle::new_pair();
        ChatSummaryTask {
            taskId: Arc::new(()),
            abortHandle,
            completion: ready(Ok(())).boxed().shared(),
        }
    }

    /// Keeps the original operation when another request targets the same chat.
    #[test]
    fn summary_registry_rejects_duplicate_chat_tasks() {
        let tasks = ChatSummaryTasks::default();
        let original = summaryTask();
        tasks
            .register("chat-a".to_string(), original.clone())
            .unwrap();
        assert!(tasks.register("chat-a".to_string(), summaryTask()).is_err());
        assert!(Arc::ptr_eq(
            &tasks.get("chat-a").unwrap().taskId,
            &original.taskId
        ));
    }

    /// Shares cancellation with worker clones and isolates summaries in other chats.
    #[test]
    fn summary_registry_cancels_only_the_selected_chat_across_clones() {
        let tasks = ChatSummaryTasks::default();
        let first = summaryTask();
        let second = summaryTask();
        tasks.register("chat-a".to_string(), first.clone()).unwrap();
        tasks
            .register("chat-b".to_string(), second.clone())
            .unwrap();
        tasks.clone().cancel(Some("chat-a"));
        assert!(first.abortHandle.is_aborted());
        assert!(!second.abortHandle.is_aborted());
        tasks.cancel(None);
        assert!(second.abortHandle.is_aborted());
    }

    /// Prevents stale cleanup from removing a newer summary for the same chat.
    #[test]
    fn summary_registry_retains_newer_task_when_old_cleanup_arrives() {
        let tasks = ChatSummaryTasks::default();
        let first = summaryTask();
        tasks.register("chat-a".to_string(), first.clone()).unwrap();
        tasks.finish("chat-a", &first.taskId);
        let next = summaryTask();
        tasks.register("chat-a".to_string(), next.clone()).unwrap();
        tasks.finish("chat-a", &first.taskId);
        assert!(Arc::ptr_eq(
            &tasks.get("chat-a").unwrap().taskId,
            &next.taskId
        ));
    }

    /// Marks compaction on the shared callback gate without publishing a nonfatal error.
    #[test]
    fn summary_callback_marks_the_stream_for_compaction() {
        let tokenLimitExceeded = Arc::new(AtomicBool::new(false));
        let callbacks = MessageProcessingCallbacks {
            nonFatalErrorEventFlow: mutableStateFlow(None),
            tokenLimitExceeded: tokenLimitExceeded.clone(),
        };
        callbacks.onTokenLimitExceeded();
        assert!(tokenLimitExceeded.load(Ordering::Acquire));
        assert!(callbacks.nonFatalErrorEventFlow.value().is_none());
    }

    /// Builds an active chat execution state for runtime statistic tests.
    fn activeExecutionState() -> ChatExecutionState {
        ChatExecutionState {
            isLoading: true,
            inputProcessingState: InputProcessingState::Receiving {
                message: "receiving".to_string(),
            },
        }
    }

    /// Aggregates tool invocation counts across every active chat only.
    #[test]
    fn toolpkg_chat_runtime_statistics_sum_active_chat_tool_counts() {
        let executionStates = HashMap::from([
            ("chat-a".to_string(), activeExecutionState()),
            ("chat-b".to_string(), activeExecutionState()),
            ("chat-c".to_string(), ChatExecutionState::idle()),
        ]);
        let counts = HashMap::from([
            ("chat-a".to_string(), 2),
            ("chat-b".to_string(), 3),
            ("chat-c".to_string(), 5),
        ]);

        let statistics = collectToolPkgChatRuntimeStatistics("chat-a", &executionStates, &counts);

        assert_eq!(statistics.currentTurnToolInvocationCount, 2);
        assert_eq!(statistics.activeConversationCount, 2);
        assert_eq!(statistics.currentSessionToolCount, 5);
        assert_eq!(statistics.activeChatIds.len(), 2);
        assert!(statistics.activeChatIds.contains(&"chat-a".to_string()));
        assert!(statistics.activeChatIds.contains(&"chat-b".to_string()));
    }
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
