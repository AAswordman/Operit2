use crate::core::chat::AIMessageManager::AIMessageManager;
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::plugins::toolpkg::ToolPkgChatInputHookBridge::{
    ChatInputHookContext, ChatInputHookResult, ToolPkgChatInputHookBridge,
    CHAT_INPUT_EVENT_INPUT_CHANGED, CHAT_INPUT_EVENT_SUBMITTED, CHAT_INPUT_EVENT_SUBMIT_REQUESTED,
    CHAT_INPUT_SUBMIT_ACTION_ALLOW, CHAT_INPUT_SUBMIT_ACTION_BLOCK,
    CHAT_INPUT_SUBMIT_ACTION_CONSUME, CHAT_INPUT_SUBMIT_ACTION_REPLACE,
};
use crate::plugins::toolpkg::ToolPkgXmlRenderBridge::ToolPkgXmlRenderBridge;
use crate::services::core::ChatHistoryDelegate::{ChatHistoryDelegate, ChatSelectionMode};
use crate::services::core::MessageCoordinationDelegate::MessageCoordinationDelegate;
use crate::services::core::MessageProcessingDelegate::{
    coreResponseStreamSource, ChatExecutionState, MessageProcessingDelegate,
    SendUserMessageProcessingRequest,
};
use crate::services::core::TokenStatisticsDelegate::TokenStatisticsDelegate;
use crate::ui::features::chat::webview::workspace::WorkspaceBackupManager::{
    WorkspaceBackupManager, WorkspaceFileChange,
};
use crate::ui::features::chat::webview::workspace::WorkspaceUtils;
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_host_api::{FileSystemHost, HostRuntimeTaskSchedulerHost};
use operit_link::{CoreStream, CoreValue};
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_model::ChatHistory::ChatHistory;
use operit_model::ChatHistoryListItem::ChatHistoryListItem;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatMessageLocatorPreview::ChatMessageLocatorPreview;
use operit_model::ChatTurnOptions::ChatTurnOptions;
use operit_model::FunctionType::FunctionType;
use operit_model::InputProcessingState::InputProcessingState;
use operit_model::MessagePart::MessagePart;
use operit_model::MessagePartCodec::MessagePartCodec;
use operit_model::PendingQueueMessageItem::PendingQueueMessageItem;
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_providers::chat::llmprovider::AIService::SharedAiResponseStream;
use operit_providers::chat::EnhancedAIService::{CoreHandoffContinuation, EnhancedAIService};
use operit_store::repository::ChatHistoryManager::ChatImportResult;
use operit_store::PreferencesDataStore::{
    combine2, combine3, mutableStateFlow, MutableStateFlow, StateFlow,
};
use operit_tools::files::PathMapper::PathMapper;
use operit_tools::tools::skill_runtime::SkillRepository::SkillRepository;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{AITool, ToolParameter};
use operit_util::stream::HotStream::mutable_shared_stream;
use operit_util::AppLogger::AppLogger;
use operit_util::MarkdownRenderStream::{MarkdownRenderEventStream, MarkdownStreamEvent};
use operit_util::OCRUtils::{OCRUtils, Quality as OCRQuality};
use operit_util::OperitPaths;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use url::Url;

const PACKAGE_ATTACHMENT_PREFIX: &str = "package_attach:";
const PASTED_TEXT_ATTACHMENT_PREFIX: &str = "pasted_text:";
const OCR_INLINE_INSTRUCTION: &str = "Do not read the file, answer the user's question directly based on the attachment content and the user's question.";
pub trait ChatServiceUiBridge {}

pub struct EmptyChatServiceUiBridge;

impl ChatServiceUiBridge for EmptyChatServiceUiBridge {}

pub const CORE_HANDOFF_PROBE_PROVIDER_ID: &str = "__core_handoff_probe__";
pub const CORE_HANDOFF_PROBE_MODEL_ID: &str = "__core_handoff_probe__";

/// Carries the opened chat state that must exist on the target Core before a handoff segment starts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct CoreHandoffRuntimeSnapshot {
    pub chatId: String,
    pub messages: Vec<ChatMessage>,
}

/// Serializes a ToolPkg chat input result into the proxy-facing JSON shape.
#[allow(non_snake_case)]
fn serializeChatInputHookResult(result: Option<ChatInputHookResult>) -> serde_json::Value {
    match result {
        Some(result) => serde_json::json!({
            "action": result.action,
            "text": result.text,
            "message": result.message,
            "clearInput": result.clearInput,
            "timedOut": result.timedOut,
            "metadata": result.metadata,
        }),
        None => serde_json::Value::Null,
    }
}

/// Describes the runtime state of one explicitly routed chat.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatState {
    pub currentChatId: String,
    pub currentChatTitle: String,
    pub currentCharacterCardName: Option<String>,
    pub currentCharacterCardAvatarUri: Option<String>,
    pub currentWorkspacePath: Option<String>,
    pub isLoading: bool,
    pub inputProcessingState: InputProcessingState,
    pub hasOlderDisplayHistory: bool,
    pub hasNewerDisplayHistory: bool,
    pub isLoadingDisplayWindow: bool,
}

/// Stores the runtime-owned pending message queue for one chat.
#[derive(Clone, Debug, PartialEq)]
struct PendingChatQueueState {
    messages: Vec<PendingQueueMessageItem>,
    isExpanded: bool,
    nextMessageId: i64,
    wasBlocked: bool,
    suppressNextAutoDequeue: bool,
}

impl PendingChatQueueState {
    /// Creates the initial queue state for a chat.
    fn new() -> Self {
        Self {
            messages: Vec::new(),
            isExpanded: true,
            nextMessageId: 1,
            wasBlocked: false,
            suppressNextAutoDequeue: false,
        }
    }
}

/// Shares pending-message queues between every chat surface in one runtime.
#[derive(Clone)]
pub(crate) struct PendingChatQueueStore {
    stateFlow: MutableStateFlow<HashMap<String, PendingChatQueueState>>,
}

impl PendingChatQueueStore {
    /// Creates an empty queue store shared by chat runtime slots.
    pub(crate) fn new() -> Self {
        Self {
            stateFlow: MutableStateFlow::new(HashMap::new()),
        }
    }

    /// Returns the queue-state flow shared by all chat runtime slots.
    fn stateFlow(&self) -> MutableStateFlow<HashMap<String, PendingChatQueueState>> {
        self.stateFlow.clone()
    }
}

/// Resolves a chat-card avatar URI for one chat-state emission.
fn characterCardAvatarUriByName(
    characterCardManager: &CharacterCardManager,
    name: &str,
) -> Option<String> {
    let normalizedName = name.trim();
    if normalizedName.is_empty() {
        return None;
    }
    let card = characterCardManager
        .findCharacterCardByName(normalizedName)
        .expect("CharacterCardManager.findCharacterCardByName must succeed")?;
    card.avatarUri.and_then(|value| {
        let trimmed = value.trim().to_string();
        (!trimmed.is_empty()).then_some(trimmed)
    })
}

/// Returns whether a handoff continuation is the CLI's deterministic route probe.
#[allow(non_snake_case)]
fn isCoreHandoffProbeContinuation(continuation: &CoreHandoffContinuation) -> bool {
    continuation.chatProviderIdOverride.as_deref() == Some(CORE_HANDOFF_PROBE_PROVIDER_ID)
        && continuation.chatModelIdOverride.as_deref() == Some(CORE_HANDOFF_PROBE_MODEL_ID)
}

pub struct ChatServiceCore {
    fileSystemHost: Arc<dyn FileSystemHost>,
    pub selectionMode: ChatSelectionMode,
    pub enhancedAiService: Option<EnhancedAIService>,
    pub messageProcessingDelegate: MessageProcessingDelegate,
    pub chatHistoryDelegate: ChatHistoryDelegate,
    pub messageCoordinationDelegate: Option<MessageCoordinationDelegate>,
    pub initialized: bool,
    pub onEnhancedAiServiceReady: Option<fn(&EnhancedAIService)>,
    pub additionalOnTurnComplete: Option<fn(Option<String>, i32, i32, i32)>,
    pub uiBridge: EmptyChatServiceUiBridge,
    pub attachments: Vec<AttachmentInfo>,
    pendingQueueStore: Arc<PendingChatQueueStore>,
}

impl ChatServiceCore {
    /// Creates a chat service core for the selected chat target mode.
    pub fn new(selectionMode: ChatSelectionMode, fileSystemHost: Arc<dyn FileSystemHost>) -> Self {
        Self::newWithPendingQueueStore(
            selectionMode,
            fileSystemHost,
            Arc::new(PendingChatQueueStore::new()),
        )
    }

    /// Creates a chat service core backed by a queue store shared with sibling runtime slots.
    pub(crate) fn newWithPendingQueueStore(
        selectionMode: ChatSelectionMode,
        fileSystemHost: Arc<dyn FileSystemHost>,
        pendingQueueStore: Arc<PendingChatQueueStore>,
    ) -> Self {
        let mut core = Self {
            fileSystemHost,
            selectionMode: selectionMode.clone(),
            enhancedAiService: None,
            messageProcessingDelegate: MessageProcessingDelegate::default(),
            chatHistoryDelegate: ChatHistoryDelegate::new(selectionMode),
            messageCoordinationDelegate: None,
            initialized: false,
            onEnhancedAiServiceReady: None,
            additionalOnTurnComplete: None,
            uiBridge: EmptyChatServiceUiBridge,
            attachments: Vec::new(),
            pendingQueueStore,
        };
        core.initializeDelegates();
        core
    }

    /// Returns the tool handler bound to this chat runtime core.
    fn runtimeToolHandler(&self) -> AIToolHandler {
        self.enhancedAiService
            .as_ref()
            .expect("ChatServiceCore requires an enhanced AI service for runtime tool access")
            .tool_handler
            .clone()
    }

    /// Returns the shared pending-message queue state for this chat runtime.
    fn pendingQueueStateFlow(&self) -> MutableStateFlow<HashMap<String, PendingChatQueueState>> {
        self.pendingQueueStore.stateFlow()
    }

    /// Reports whether the specified chat is currently unable to accept a new turn.
    fn isChatQueueBlocked(&self, chatId: &str) -> bool {
        let activeStreamingChatIds = self
            .messageProcessingDelegate
            .activeStreamingChatIdsFlow()
            .value();
        if activeStreamingChatIds.contains(chatId) {
            return true;
        }
        let inputProcessingStateByChatId = self
            .messageProcessingDelegate
            .inputProcessingStateByChatIdFlow()
            .value();
        match inputProcessingStateByChatId.get(chatId) {
            Some(InputProcessingState::Idle)
            | Some(InputProcessingState::Completed)
            | Some(InputProcessingState::Error { .. })
            | None => false,
            Some(_) => true,
        }
    }

    /// Marks an existing queue as blocked when a new turn starts for its chat.
    fn markPendingQueueBlocked(&mut self, chatId: &str) {
        let mut queueStateByChatId = self.pendingQueueStateFlow().value();
        let Some(queueState) = queueStateByChatId.get_mut(chatId) else {
            return;
        };
        if queueState.messages.is_empty() || queueState.wasBlocked {
            return;
        }
        queueState.wasBlocked = true;
        self.pendingQueueStateFlow().set_value(queueStateByChatId);
    }

    fn initializeDelegates(&mut self) {
        self.chatHistoryDelegate = ChatHistoryDelegate::new(self.selectionMode.clone());
        self.chatHistoryDelegate.initialize();
        self.messageProcessingDelegate = MessageProcessingDelegate::default();
        let messageProcessingDelegate = self.messageProcessingDelegate.clone_for_core();
        self.messageCoordinationDelegate = Some(MessageCoordinationDelegate::new(
            self.chatHistoryDelegate.clone_for_core(),
            messageProcessingDelegate,
        ));
        self.syncTokenStatisticsForCurrentChat();
        self.initialized = true;
    }

    #[allow(non_snake_case)]
    fn syncTokenStatisticsForCurrentChat(&mut self) {
        let chatId = self.chatHistoryDelegate.currentChatIdFlow.value();
        if let Some(delegate) = self.messageCoordinationDelegate.as_mut() {
            delegate
                .tokenStatisticsDelegate
                .setActiveChatId(chatId.clone());
            if let Some(chatId) = chatId {
                if let Some(chat) = self
                    .chatHistoryDelegate
                    .chatHistoriesFlow()
                    .value()
                    .into_iter()
                    .find(|chat| chat.id == chatId)
                {
                    delegate.tokenStatisticsDelegate.setTokenCounts(
                        Some(chat.id),
                        chat.inputTokens,
                        chat.outputTokens,
                        chat.currentWindowSize,
                    );
                }
            }
        }
    }

    /// Builds a ToolPkg chat input context for the current runtime send surface.
    #[allow(non_snake_case)]
    fn buildChatInputHookContext(
        &self,
        chatId: &str,
        text: &str,
        selectionStart: i32,
        selectionEnd: i32,
        attachmentCount: usize,
        eventName: &str,
    ) -> ChatInputHookContext {
        ChatInputHookContext {
            chatId: chatId.to_string(),
            text: text.to_string(),
            selectionStart,
            selectionEnd,
            hasAttachments: attachmentCount > 0,
            attachmentCount: attachmentCount as i32,
            isProcessing: self
                .messageProcessingDelegate
                .isChatLoading(chatId.to_string()),
            inputStyle: "Runtime".to_string(),
            source: "Runtime".to_string(),
            submitSource: "Send".to_string(),
            eventName: eventName.to_string(),
        }
    }

    /// Builds a chat input hook context using the caret at the end of the text.
    #[allow(non_snake_case)]
    fn buildChatInputHookContextAtEnd(
        &self,
        chatId: &str,
        text: &str,
        attachmentCount: usize,
        eventName: &str,
    ) -> ChatInputHookContext {
        let textCharCount = text.chars().count() as i32;
        self.buildChatInputHookContext(
            chatId,
            text,
            textCharCount,
            textCharCount,
            attachmentCount,
            eventName,
        )
    }

    /// Dispatches chat input change notifications from host-owned input widgets.
    #[allow(non_snake_case)]
    pub fn dispatchChatInputChanged(
        &self,
        chatIdOverride: Option<String>,
        messageText: String,
        selectionStart: i32,
        selectionEnd: i32,
        attachmentCount: usize,
    ) {
        let hookChatId = chatIdOverride
            .or_else(|| self.chatHistoryDelegate.currentChatIdFlow.value())
            .unwrap_or_default();
        ToolPkgChatInputHookBridge::dispatchRegisteredChatInputHooks(
            self.buildChatInputHookContext(
                &hookChatId,
                &messageText,
                selectionStart,
                selectionEnd,
                attachmentCount,
                CHAT_INPUT_EVENT_INPUT_CHANGED,
            ),
        );
    }

    /// Dispatches submit_requested and returns the ToolPkg decision for the host input widget.
    #[allow(non_snake_case)]
    pub fn dispatchChatInputSubmitRequested(
        &self,
        chatIdOverride: Option<String>,
        messageText: String,
        selectionStart: i32,
        selectionEnd: i32,
        attachmentCount: usize,
    ) -> serde_json::Value {
        let hookChatId = chatIdOverride
            .or_else(|| self.chatHistoryDelegate.currentChatIdFlow.value())
            .unwrap_or_default();
        let decision = ToolPkgChatInputHookBridge::dispatchRegisteredChatInputHooks(
            self.buildChatInputHookContext(
                &hookChatId,
                &messageText,
                selectionStart,
                selectionEnd,
                attachmentCount,
                CHAT_INPUT_EVENT_SUBMIT_REQUESTED,
            ),
        );
        serializeChatInputHookResult(decision)
    }

    /// Sends a user-authored message through the active chat runtime.
    #[operit_route_macros::operit_core_route(binding = chatIdOverride)]
    pub async fn sendUserMessage(
        &mut self,
        promptFunctionType: PromptFunctionType,
        roleCardIdOverride: Option<String>,
        chatIdOverride: Option<String>,
        mut messageText: String,
        proxySenderNameOverride: Option<String>,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
        attachments: Vec<AttachmentInfo>,
        replyToMessage: Option<ChatMessage>,
        turnOptions: ChatTurnOptions,
    ) {
        let hookChatId = match chatIdOverride.as_ref() {
            Some(chatId) => chatId.clone(),
            None => self
                .chatHistoryDelegate
                .currentChatIdFlow
                .value()
                .unwrap_or_default(),
        };
        AppLogger::i(
            "ChatServiceCore",
            &format!(
                "send accepted chatId={} persisted={} attachments={}",
                hookChatId,
                turnOptions.persistTurn,
                attachments.len()
            ),
        );
        let attachmentCount = attachments.len();
        if !turnOptions.chatInputSubmitRequestedHandled {
            let submitDecision = ToolPkgChatInputHookBridge::dispatchRegisteredChatInputHooks(
                self.buildChatInputHookContextAtEnd(
                    &hookChatId,
                    &messageText,
                    attachmentCount,
                    CHAT_INPUT_EVENT_SUBMIT_REQUESTED,
                ),
            );
            if let Some(decision) = submitDecision {
                match decision.action.as_str() {
                    CHAT_INPUT_SUBMIT_ACTION_BLOCK | CHAT_INPUT_SUBMIT_ACTION_CONSUME => {
                        if let Some(message) = decision.message {
                            self.messageProcessingDelegate.showToast(message);
                        }
                        return;
                    }
                    CHAT_INPUT_SUBMIT_ACTION_REPLACE | CHAT_INPUT_SUBMIT_ACTION_ALLOW => {
                        if let Some(message) = decision.message {
                            self.messageProcessingDelegate.showToast(message);
                        }
                        if let Some(updatedText) = decision.text {
                            messageText = updatedText;
                        }
                    }
                    _ => {}
                };
            }
        }
        ToolPkgChatInputHookBridge::dispatchRegisteredChatInputHooks(
            self.buildChatInputHookContextAtEnd(
                &hookChatId,
                &messageText,
                attachmentCount,
                CHAT_INPUT_EVENT_SUBMITTED,
            ),
        );
        if self.enhancedAiService.is_some() && self.messageCoordinationDelegate.is_some() {
            self.markPendingQueueBlocked(&hookChatId);
        }
        if let (Some(service), Some(delegate)) = (
            self.enhancedAiService.as_mut(),
            self.messageCoordinationDelegate.as_mut(),
        ) {
            delegate.chatHistoryDelegate = self.chatHistoryDelegate.clone_for_core();
            delegate.messageProcessingDelegate = self.messageProcessingDelegate.clone_for_core();
            delegate
                .sendUserMessage(
                    service,
                    promptFunctionType,
                    roleCardIdOverride,
                    chatIdOverride,
                    messageText,
                    proxySenderNameOverride,
                    chatProviderIdOverride,
                    chatModelIdOverride,
                    attachments,
                    replyToMessage,
                    turnOptions,
                )
                .await;
            self.chatHistoryDelegate = delegate.chatHistoryDelegate.clone_for_core();
            self.messageProcessingDelegate = delegate.messageProcessingDelegate.clone_for_core();
        }
        AppLogger::i(
            "ChatServiceCore",
            &format!("send scheduled chatId={hookChatId}"),
        );
    }

    /// Starts one target-Core EnhanceAI continuation after a route handoff.
    #[operit_route_macros::operit_core_internal]
    pub async fn continueCoreHandoff(
        &mut self,
        continuation: CoreHandoffContinuation,
    ) -> Result<(), String> {
        if isCoreHandoffProbeContinuation(&continuation) {
            return Ok(());
        }
        let chatId = continuation
            .chatId
            .clone()
            .ok_or_else(|| "Core handoff requires a chat id".to_string())?;
        let roleCardId = continuation
            .roleCardId
            .clone()
            .ok_or_else(|| "Core handoff requires a role card id".to_string())?;
        let chatHistory = self
            .chatHistoryDelegate
            .openedRuntimeChatHistorySnapshot(&chatId)?;
        let enhancedAiService = self.enhancedAiService.as_mut().ok_or_else(|| {
            "ChatServiceCore requires an EnhancedAIService for handoff".to_string()
        })?;
        self.messageProcessingDelegate
            .sendUserMessage(SendUserMessageProcessingRequest {
                enhancedAiService,
                chatHistoryDelegate: &mut self.chatHistoryDelegate,
                chatId: chatId.clone(),
                messageText: String::new(),
                chatHistory,
                promptHistoryOverride: Some(continuation.chatHistory.clone()),
                workspacePath: continuation.workspacePath.clone(),
                promptFunctionType: continuation.promptFunctionType.clone(),
                roleCardId,
                currentRoleName: continuation.characterName.clone(),
                characterName: continuation.characterName.clone(),
                avatarUri: continuation.avatarUri.clone(),
                attachments: Vec::new(),
                replyToMessage: None,
                enableThinking: continuation.enableThinking,
                enableMemoryAutoUpdate: continuation.enableMemoryAutoUpdate,
                maxTokens: continuation.maxTokens,
                tokenUsageThreshold: continuation.tokenUsageThreshold,
                chatProviderIdOverride: continuation.chatProviderIdOverride.clone(),
                chatModelIdOverride: continuation.chatModelIdOverride.clone(),
                isGroupOrchestrationTurn: continuation.enableGroupOrchestrationHint,
                groupParticipantNamesText: continuation.groupParticipantNamesText.clone(),
                proxySenderNameOverride: continuation.proxySenderName.clone(),
                suppressUserMessageInHistory: true,
                isAutoContinuation: false,
                assistantMessageTimestamp: Some(continuation.assistantMessageTimestamp),
                executionGeneration: Some(continuation.executionGeneration),
                executionSegmentIndex: Some(continuation.segmentIndex),
                turnOptions: ChatTurnOptions {
                    persistTurn: true,
                    notifyReply: continuation.notifyReplyOverride,
                    hideUserMessage: false,
                    disableWarning: continuation.disableWarning,
                    chatInputSubmitRequestedHandled: false,
                },
            })
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    /// Applies the source Core's opened chat runtime snapshot before the target segment starts.
    #[allow(non_snake_case)]
    pub fn applyCoreHandoffRuntimeSnapshotValue(
        &mut self,
        snapshot: CoreValue,
    ) -> Result<(), String> {
        let snapshot: CoreHandoffRuntimeSnapshot = operit_link::fromCoreValue(snapshot)
            .map_err(|error| format!("invalid handoff runtime snapshot: {error}"))?;
        self.chatHistoryDelegate
            .applyRuntimeChatHistorySnapshot(snapshot.chatId, snapshot.messages);
        Ok(())
    }

    /// Continues one handoff from the protocol-owned serialized continuation value.
    pub async fn continueCoreHandoffValue(
        &mut self,
        continuation: CoreValue,
    ) -> Result<(), String> {
        let continuation: CoreHandoffContinuation = operit_link::fromCoreValue(continuation)
            .map_err(|error| format!("invalid handoff state: {error}"))?;
        self.continueCoreHandoff(continuation).await
    }

    /// Cancels message generation for a specific chat id.
    #[operit_route_macros::operit_core_route(binding = chatId)]
    pub async fn cancelMessage(&mut self, chatId: String) {
        self.messageProcessingDelegate.cancelMessage(chatId).await;
    }

    /// Adds one message to the queue owned by a specific chat.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId)]
    pub fn enqueuePendingQueueMessage(&mut self, chatId: String, messageText: String) {
        let mut queueStateByChatId = self.pendingQueueStateFlow().value();
        let queueState = queueStateByChatId
            .entry(chatId)
            .or_insert_with(PendingChatQueueState::new);
        let messageId = queueState.nextMessageId;
        queueState.nextMessageId += 1;
        queueState.messages.push(PendingQueueMessageItem {
            id: messageId,
            text: messageText,
        });
        queueState.isExpanded = true;
        queueState.wasBlocked = true;
        self.pendingQueueStateFlow().set_value(queueStateByChatId);
    }

    /// Deletes one queued message from a specific chat.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId)]
    pub fn deletePendingQueueMessage(&mut self, chatId: String, messageId: i64) {
        let mut queueStateByChatId = self.pendingQueueStateFlow().value();
        let Some(queueState) = queueStateByChatId.get_mut(&chatId) else {
            return;
        };
        queueState.messages.retain(|item| item.id != messageId);
        self.pendingQueueStateFlow().set_value(queueStateByChatId);
    }

    /// Removes one queued message for editing or explicit user delivery.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId)]
    pub fn takePendingQueueMessage(
        &mut self,
        chatId: String,
        messageId: i64,
        suppressNextAutoDequeue: bool,
    ) -> Option<PendingQueueMessageItem> {
        let shouldSuppressAutoDequeue = suppressNextAutoDequeue && self.isChatQueueBlocked(&chatId);
        let mut queueStateByChatId = self.pendingQueueStateFlow().value();
        let queueState = queueStateByChatId.get_mut(&chatId)?;
        let messageIndex = queueState
            .messages
            .iter()
            .position(|item| item.id == messageId)?;
        let message = queueState.messages.remove(messageIndex);
        if shouldSuppressAutoDequeue && !queueState.messages.is_empty() {
            queueState.suppressNextAutoDequeue = true;
        }
        self.pendingQueueStateFlow().set_value(queueStateByChatId);
        Some(message)
    }

    /// Clears a manual-send suppression after that message is not delivered.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId)]
    pub fn clearPendingQueueAutoDequeueSuppression(&mut self, chatId: String) {
        let mut queueStateByChatId = self.pendingQueueStateFlow().value();
        let Some(queueState) = queueStateByChatId.get_mut(&chatId) else {
            return;
        };
        if !queueState.suppressNextAutoDequeue {
            return;
        }
        queueState.suppressNextAutoDequeue = false;
        self.pendingQueueStateFlow().set_value(queueStateByChatId);
    }

    /// Atomically removes the next queued message after a chat becomes ready.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId)]
    pub fn takeNextPendingQueueMessageIfReady(
        &mut self,
        chatId: String,
    ) -> Option<PendingQueueMessageItem> {
        if self.isChatQueueBlocked(&chatId) {
            return None;
        }
        let mut queueStateByChatId = self.pendingQueueStateFlow().value();
        let queueState = queueStateByChatId.get_mut(&chatId)?;
        if !queueState.wasBlocked {
            return None;
        }
        queueState.wasBlocked = false;
        if queueState.suppressNextAutoDequeue {
            queueState.suppressNextAutoDequeue = false;
            self.pendingQueueStateFlow().set_value(queueStateByChatId);
            return None;
        }
        let Some(message) = queueState.messages.first().cloned() else {
            self.pendingQueueStateFlow().set_value(queueStateByChatId);
            return None;
        };
        queueState.messages.remove(0);
        self.pendingQueueStateFlow().set_value(queueStateByChatId);
        Some(message)
    }

    /// Inserts a rejected queued message back at the front of its chat queue.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId)]
    pub fn restorePendingQueueMessage(&mut self, chatId: String, message: PendingQueueMessageItem) {
        let mut queueStateByChatId = self.pendingQueueStateFlow().value();
        let queueState = queueStateByChatId
            .entry(chatId)
            .or_insert_with(PendingChatQueueState::new);
        if queueState.messages.iter().any(|item| item.id == message.id) {
            return;
        }
        queueState.nextMessageId = queueState.nextMessageId.max(message.id + 1);
        queueState.messages.insert(0, message);
        self.pendingQueueStateFlow().set_value(queueStateByChatId);
    }

    /// Updates whether a chat's pending-message queue is expanded in the UI.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId)]
    pub fn setPendingQueueExpanded(&mut self, chatId: String, isExpanded: bool) {
        let mut queueStateByChatId = self.pendingQueueStateFlow().value();
        let queueState = queueStateByChatId
            .entry(chatId)
            .or_insert_with(PendingChatQueueState::new);
        queueState.isExpanded = isExpanded;
        self.pendingQueueStateFlow().set_value(queueStateByChatId);
    }

    /// Splits markdown content into stable render events for the client.
    pub fn splitMarkdownContent(&self, content: String) -> Vec<MarkdownStreamEvent> {
        MarkdownRenderEventStream::fromContent(content)
    }

    /// Renders one XML block through registered ToolPkg XML render hooks.
    #[allow(non_snake_case)]
    pub fn renderToolPkgXml(&self, tagName: String, xmlContent: String) -> serde_json::Value {
        ToolPkgXmlRenderBridge::renderRegisteredXml(tagName, xmlContent)
    }

    /// Creates a new chat and makes it available through chat history state.
    pub fn createNewChat(
        &mut self,
        characterCardName: Option<String>,
        group: Option<String>,
        inheritGroupFromCurrent: bool,
        setAsCurrentChat: bool,
        characterGroupId: Option<String>,
    ) {
        self.chatHistoryDelegate.createNewChat(
            characterCardName,
            characterGroupId,
            group,
            inheritGroupFromCurrent,
            setAsCurrentChat,
            None,
        );
        self.syncTokenStatisticsForCurrentChat();
    }

    /// Switches the active chat and refreshes its runtime state.
    pub fn switchChat(&mut self, chatId: String) {
        self.chatHistoryDelegate.switchChat(chatId, true);
        self.syncTokenStatisticsForCurrentChat();
    }

    /// Switches the local runtime selection without writing the global chat selection.
    pub fn switchChatLocal(&mut self, chatId: String) {
        self.chatHistoryDelegate.switchChat(chatId, false);
        self.syncTokenStatisticsForCurrentChat();
    }

    /// Changes the active character card target used when new chat turns are sent.
    #[allow(non_snake_case)]
    pub fn switchActiveCharacterCardTarget(&mut self, characterCardId: String) {
        self.chatHistoryDelegate
            .switchActiveCharacterCardTarget(characterCardId);
        self.syncTokenStatisticsForCurrentChat();
    }

    /// Changes the active character group target used when new group chat turns are sent.
    #[allow(non_snake_case)]
    pub fn switchActiveCharacterGroupTarget(&mut self, characterGroupId: String) {
        self.chatHistoryDelegate
            .switchActiveCharacterGroupTarget(characterGroupId);
        self.syncTokenStatisticsForCurrentChat();
    }

    /// Updates the character card binding stored on an existing chat.
    #[allow(non_snake_case)]
    pub fn updateChatCharacterCard(&mut self, chatId: String, characterCardName: Option<String>) {
        self.chatHistoryDelegate
            .updateChatCharacterCard(chatId, characterCardName);
        self.syncTokenStatisticsForCurrentChat();
    }

    /// Updates the character group binding stored on an existing chat.
    #[allow(non_snake_case)]
    pub fn updateChatCharacterGroup(&mut self, chatId: String, characterGroupId: Option<String>) {
        self.chatHistoryDelegate
            .updateChatCharacterGroup(chatId, characterGroupId);
        self.syncTokenStatisticsForCurrentChat();
    }

    /// Synchronizes the current runtime chat id to the global chat selection.
    pub fn syncCurrentChatIdToGlobal(&mut self) {}

    /// Deletes a chat history and updates current chat selection.
    pub fn deleteChatHistory(&mut self, chatId: String) -> bool {
        let deleted = self.chatHistoryDelegate.deleteChatHistory(chatId);
        self.syncTokenStatisticsForCurrentChat();
        deleted
    }

    /// Deletes one message from the current chat by visible message index.
    pub fn deleteMessage(&mut self, index: usize) {
        self.chatHistoryDelegate.deleteMessage(index);
    }

    /// Deletes multiple messages from the current chat by visible message indices.
    #[allow(non_snake_case)]
    pub fn deleteMessages(&mut self, indices: Vec<usize>) -> bool {
        let Some(chatId) = self.chatHistoryDelegate.currentChatIdFlow.value() else {
            return false;
        };
        let mut timestamps = Vec::new();
        let currentMessages = self.chatHistoryDelegate.currentChatMessagesSnapshot();
        for index in indices {
            let Some(message) = currentMessages.get(index) else {
                return false;
            };
            timestamps.push(message.timestamp);
        }
        self.chatHistoryDelegate
            .deleteMessagesByTimestamps(chatId, timestamps);
        true
    }

    /// Replaces the content of one message and refreshes the stable context window.
    #[allow(non_snake_case)]
    pub async fn updateMessage(&mut self, index: usize, editedContent: String) -> bool {
        let currentMessages = self.chatHistoryDelegate.currentChatMessagesSnapshot();
        let Some(message) = currentMessages.get(index).cloned() else {
            return false;
        };
        let editedParts = match message.sender.as_str() {
            "ai" => match MessagePartCodec::parseAssistantMarkup(&editedContent) {
                Ok(parts) => parts,
                Err(error) => {
                    AppLogger::e(
                        "ChatServiceCore",
                        &format!("cannot update assistant message: invalid markup: {error}"),
                    );
                    return false;
                }
            },
            "user" => vec![MessagePart::markdown(
                "part-0".to_string(),
                0,
                editedContent,
            )],
            sender => {
                AppLogger::e(
                    "ChatServiceCore",
                    &format!("cannot update message from unsupported sender: {sender}"),
                );
                return false;
            }
        };
        let editedMessage = ChatMessage {
            parts: editedParts,
            ..message
        };
        self.chatHistoryDelegate
            .addMessageToChat(editedMessage, None);
        if let (Some(service), Some(delegate)) = (
            self.enhancedAiService.as_mut(),
            self.messageCoordinationDelegate.as_mut(),
        ) {
            delegate.chatHistoryDelegate = self.chatHistoryDelegate.clone_for_core();
            delegate
                .refreshStableContextWindow(
                    service,
                    self.chatHistoryDelegate.currentChatIdFlow.value(),
                    None,
                    Some(PromptFunctionType::CHAT),
                    false,
                    None,
                    None,
                    None,
                )
                .await;
            self.chatHistoryDelegate = delegate.chatHistoryDelegate.clone_for_core();
        }
        true
    }

    /// Deletes the selected message and every following message in the current chat.
    #[allow(non_snake_case)]
    pub fn deleteMessagesFrom(&mut self, index: usize) -> bool {
        self.chatHistoryDelegate.deleteMessagesFrom(index)
    }

    /// Deletes one alternate response variant from a message timestamp.
    #[allow(non_snake_case)]
    pub fn deleteMessageVariant(&mut self, timestamp: i64, variantIndex: i32) {
        self.chatHistoryDelegate
            .deleteMessageVariant(timestamp, variantIndex);
    }

    /// Creates a branch chat from the current conversation at an optional message timestamp.
    pub fn createBranch(&mut self, upToMessageTimestamp: Option<i64>) {
        self.chatHistoryDelegate.createBranch(upToMessageTimestamp);
        self.syncTokenStatisticsForCurrentChat();
        self.messageProcessingDelegate.scrollToBottom();
    }

    /// Generates and inserts a summary message around the selected user or AI message.
    #[allow(non_snake_case)]
    pub async fn insertSummary(&mut self, message: ChatMessage) -> bool {
        if message.sender != "user" && message.sender != "ai" {
            return false;
        }
        let Some(currentChatId) = self.chatHistoryDelegate.currentChatIdFlow.value() else {
            return false;
        };
        let Some(enhancedAiService) = self.enhancedAiService.as_mut() else {
            return false;
        };
        self.messageProcessingDelegate
            .setInputProcessingStateForChat(
                currentChatId.clone(),
                InputProcessingState::Summarizing {
                    message: "chat_summarizing_generating".to_string(),
                },
            );
        let beforeTimestamp = if message.sender == "ai" {
            Some(message.timestamp)
        } else {
            None
        };
        let afterTimestamp = if message.sender == "user" {
            Some(message.timestamp)
        } else {
            None
        };
        let messagesToSummarize = self
            .chatHistoryDelegate
            .loadMessagesForSummaryInsertion(currentChatId.clone(), afterTimestamp, beforeTimestamp)
            .into_iter()
            .filter(|message| message.sender == "user" || message.sender == "ai")
            .collect::<Vec<_>>();
        if messagesToSummarize.is_empty() {
            self.messageProcessingDelegate
                .setInputProcessingStateForChat(currentChatId, InputProcessingState::Idle);
            return false;
        }
        let isGroupChat = self
            .chatHistoryDelegate
            .chatHistoriesFlow()
            .value()
            .into_iter()
            .find(|chat| chat.id == currentChatId)
            .and_then(|chat| chat.characterGroupId)
            .is_some();
        let summaryMessage = match AIMessageManager::summarizeMemory(
            enhancedAiService,
            messagesToSummarize,
            false,
            isGroupChat,
        )
        .await
        {
            Ok(Some(summaryMessage)) => summaryMessage,
            _ => {
                self.messageProcessingDelegate
                    .setInputProcessingStateForChat(currentChatId, InputProcessingState::Idle);
                return false;
            }
        };
        self.chatHistoryDelegate.addSummaryMessage(
            summaryMessage,
            beforeTimestamp,
            afterTimestamp,
            Some(currentChatId.clone()),
        );
        if let Some(delegate) = self.messageCoordinationDelegate.as_mut() {
            delegate.chatHistoryDelegate = self.chatHistoryDelegate.clone_for_core();
            delegate.messageProcessingDelegate = self.messageProcessingDelegate.clone_for_core();
            delegate
                .refreshStableContextWindow(
                    enhancedAiService,
                    Some(currentChatId.clone()),
                    None,
                    None,
                    false,
                    None,
                    None,
                    None,
                )
                .await;
            self.chatHistoryDelegate = delegate.chatHistoryDelegate.clone_for_core();
            self.messageProcessingDelegate = delegate.messageProcessingDelegate.clone_for_core();
        }
        self.messageProcessingDelegate
            .setInputProcessingStateForChat(currentChatId, InputProcessingState::Idle);
        true
    }

    /// Returns branch chats that were derived from the requested parent chat.
    pub fn getBranches(&self, parentChatId: String) -> Vec<operit_model::ChatHistory::ChatHistory> {
        self.chatHistoryDelegate.getBranches(parentChatId)
    }

    /// Updates whether a chat is locked against destructive changes.
    pub fn updateChatLocked(&mut self, chatId: String, locked: bool) {
        self.chatHistoryDelegate.updateChatLocked(chatId, locked);
    }

    /// Updates whether a chat is pinned in chat history ordering.
    pub fn updateChatPinned(&mut self, chatId: String, pinned: bool) {
        self.chatHistoryDelegate.updateChatPinned(chatId, pinned);
    }

    /// Applies a reordered chat list and optionally moves the active item into a group.
    #[allow(non_snake_case)]
    pub fn updateChatOrderAndGroup(
        &mut self,
        reorderedHistories: Vec<ChatHistoryListItem>,
        movedItem: ChatHistoryListItem,
        targetGroup: Option<String>,
    ) {
        self.chatHistoryDelegate.updateChatOrderAndGroup(
            reorderedHistories,
            movedItem,
            targetGroup,
        );
    }

    /// Removes every message from the currently selected chat.
    pub fn clearCurrentChat(&mut self) {
        self.chatHistoryDelegate.clearCurrentChat();
    }

    /// Serializes all chat histories into a JSON archive string.
    #[allow(non_snake_case)]
    pub fn exportChatHistoriesToJson(&self) -> Result<String, String> {
        self.chatHistoryDelegate
            .chatHistoryManager
            .exportChatHistoriesToJson()
            .map_err(|error| error.to_string())
    }

    /// Imports chat histories from a JSON archive string.
    #[allow(non_snake_case)]
    pub fn importChatHistoriesFromJson(
        &mut self,
        jsonString: String,
    ) -> Result<ChatImportResult, String> {
        let result = self
            .chatHistoryDelegate
            .chatHistoryManager
            .importChatHistoriesFromJson(jsonString)
            .map_err(|error| error.to_string())?;
        Ok(result)
    }

    /// Updates the stored title of a chat history.
    pub fn updateChatTitle(&mut self, chatId: String, title: String) {
        self.chatHistoryDelegate.updateChatTitle(chatId, title);
    }

    /// Binds a chat to an existing workspace path.
    #[allow(non_snake_case)]
    pub fn bindChatToWorkspace(&mut self, chatId: String, workspace: String) -> Result<(), String> {
        let workspace = PathMapper::normalizeWorkspaceBindingPath(&workspace)?;
        self.chatHistoryDelegate
            .bindChatToWorkspace(chatId, workspace);
        Ok(())
    }

    /// Creates the default workspace directory for a chat and returns its path.
    #[allow(non_snake_case)]
    pub fn createAndGetDefaultWorkspace(
        &mut self,
        chatId: String,
        projectType: Option<String>,
    ) -> String {
        WorkspaceUtils::createAndGetDefaultWorkspace(chatId, projectType)
            .expect("WorkspaceUtils.createAndGetDefaultWorkspace must succeed")
    }

    /// Creates the default workspace for a chat and stores the workspace binding.
    #[allow(non_snake_case)]
    pub fn createAndBindDefaultWorkspace(
        &mut self,
        chatId: String,
        projectType: Option<String>,
    ) -> String {
        let workspacePath =
            WorkspaceUtils::createAndGetDefaultWorkspace(chatId.clone(), projectType)
                .expect("WorkspaceUtils.createAndGetDefaultWorkspace must succeed");
        self.chatHistoryDelegate
            .bindChatToWorkspace(chatId, workspacePath.clone());
        workspacePath
    }

    /// Removes the workspace binding from a chat without deleting workspace files.
    #[allow(non_snake_case)]
    pub fn unbindChatFromWorkspace(&mut self, chatId: String) {
        self.chatHistoryDelegate.unbindChatFromWorkspace(chatId);
    }

    /// Renames the workspace binding and chat title together.
    #[allow(non_snake_case)]
    pub fn renameWorkspaceAndChat(
        &mut self,
        chatId: String,
        newWorkspace: String,
        newTitle: String,
    ) {
        let newWorkspace = PathMapper::normalizeWorkspaceBindingPath(&newWorkspace).expect(
            "ChatServiceCore.renameWorkspaceAndChat requires a workspace path that maps to VFS",
        );
        self.chatHistoryDelegate
            .renameWorkspaceAndChat(chatId, newWorkspace, newTitle);
    }

    /// Shows file changes that would be applied when rewinding to a message index.
    #[allow(non_snake_case)]
    pub fn previewWorkspaceChangesForMessage(&mut self, index: usize) -> Vec<WorkspaceFileChange> {
        let Some((chatId, workspacePath, rewindTimestamp)) =
            self.resolveWorkspaceRewindTarget(index)
        else {
            return Vec::new();
        };
        WorkspaceBackupManager::getInstance(self.runtimeToolHandler().getContext())
            .previewChangesForRewind(workspacePath, rewindTimestamp, Some(chatId))
    }

    /// Restores the bound workspace to the snapshot before a message index.
    #[allow(non_snake_case)]
    pub fn rewindWorkspaceForMessage(&mut self, index: usize) -> bool {
        let Some((chatId, workspacePath, rewindTimestamp)) =
            self.resolveWorkspaceRewindTarget(index)
        else {
            return false;
        };
        WorkspaceBackupManager::getInstance(self.runtimeToolHandler().getContext()).syncState(
            workspacePath,
            rewindTimestamp,
            Some(chatId),
        );
        true
    }

    /// Rolls the current chat back to a prior message index.
    #[allow(non_snake_case)]
    pub fn rollbackToMessage(&mut self, index: usize) -> Option<String> {
        let currentMessages = self.chatHistoryDelegate.currentChatMessagesSnapshot();
        let Some(targetMessage) = currentMessages.get(index).cloned() else {
            return None;
        };
        if targetMessage.sender != "user" {
            return None;
        }
        self.rewindWorkspaceForMessage(index);
        self.chatHistoryDelegate
            .truncateChatHistory(Some(targetMessage.timestamp));
        Some(stripXmlLikeTags(&targetMessage.displayText()))
    }

    /// Rewinds a user message and sends edited content as a new turn.
    #[allow(non_snake_case)]
    pub async fn rewindAndResendMessage(&mut self, index: usize, editedContent: String) -> bool {
        let currentMessages = self.chatHistoryDelegate.currentChatMessagesSnapshot();
        let Some(targetMessage) = currentMessages.get(index).cloned() else {
            return false;
        };
        if targetMessage.sender != "user" {
            return false;
        }
        self.rewindWorkspaceForMessage(index);
        self.chatHistoryDelegate
            .truncateChatHistory(Some(targetMessage.timestamp));
        self.sendUserMessage(
            PromptFunctionType::CHAT,
            None,
            None,
            editedContent,
            None,
            None,
            None,
            Vec::new(),
            None,
            ChatTurnOptions::default(),
        )
        .await;
        true
    }

    /// Regenerates one AI message in place while preserving the surrounding chat history.
    #[allow(non_snake_case)]
    pub async fn regenerateSingleAiMessage(&mut self, index: usize) -> Result<(), String> {
        let Some(service) = self.enhancedAiService.as_mut() else {
            return Err("EnhancedAIService is not initialized".to_string());
        };
        let Some(delegate) = self.messageCoordinationDelegate.as_mut() else {
            return Err("MessageCoordinationDelegate is not initialized".to_string());
        };
        delegate.chatHistoryDelegate = self.chatHistoryDelegate.clone_for_core();
        delegate.messageProcessingDelegate = self.messageProcessingDelegate.clone_for_core();
        delegate.regenerateSingleAiMessage(service, index).await?;
        self.chatHistoryDelegate = delegate.chatHistoryDelegate.clone_for_core();
        self.messageProcessingDelegate = delegate.messageProcessingDelegate.clone_for_core();
        self.syncTokenStatisticsForCurrentChat();
        Ok(())
    }

    #[allow(non_snake_case)]
    fn resolveWorkspaceRewindTarget(&self, index: usize) -> Option<(String, String, i64)> {
        let chatId = self.chatHistoryDelegate.currentChatIdFlow.value()?;
        let currentMessages = self.chatHistoryDelegate.currentChatMessagesSnapshot();
        if index >= currentMessages.len() {
            return None;
        }
        let rewindTimestamp = if index > 0 {
            currentMessages[index - 1].timestamp
        } else {
            0
        };
        let currentChat = self
            .chatHistoryDelegate
            .chatHistoriesFlow
            .value()
            .into_iter()
            .find(|history| history.id == chatId)?;
        let workspacePath = currentChat
            .workspace
            .clone()
            .filter(|value| !value.trim().is_empty())?;
        Some((chatId, workspacePath, rewindTimestamp))
    }

    /// Clears the token counters associated with the current chat service.
    pub fn resetTokenStatistics(&mut self) {
        let service = self.enhancedAiService.as_mut();
        if let Some(delegate) = self.messageCoordinationDelegate.as_mut() {
            delegate
                .tokenStatisticsDelegate
                .resetTokenStatistics(service);
        }
    }

    /// Recomputes cumulative token statistics for the current chat and service.
    pub fn updateCumulativeStatistics(&mut self) {
        let chatId = self.chatHistoryDelegate.currentChatIdFlow.value();
        let service = self.enhancedAiService.as_ref();
        if let Some(delegate) = self.messageCoordinationDelegate.as_mut() {
            delegate
                .tokenStatisticsDelegate
                .updateCumulativeStatistics(chatId, service);
        }
    }

    /// Adds a file, pasted text, package, screen capture, notification capture, or location capture as an attachment.
    pub fn handleAttachment(&mut self, _filePath: String) {
        if let Some(content) = _filePath.strip_prefix(PASTED_TEXT_ATTACHMENT_PREFIX) {
            self.attachPastedText(content.to_string());
            return;
        }

        let filePath = _filePath.trim();
        if filePath.is_empty() {
            self.messageProcessingDelegate
                .showToast("无法添加空附件路径".to_string());
            return;
        }

        if filePath == "screen_capture" {
            self.captureScreenContent();
            return;
        }
        if filePath == "notifications_capture" {
            self.captureNotifications(10);
            return;
        }
        if filePath == "location_capture" {
            self.captureLocation(true);
            return;
        }
        if let Some(packageName) = filePath.strip_prefix(PACKAGE_ATTACHMENT_PREFIX) {
            self.attachPackageInternal(packageName.trim());
            return;
        }

        match self.createAttachmentInfo(filePath) {
            Ok(attachmentInfo) => {
                let currentPath = attachmentInfo.filePath.clone();
                if !self
                    .attachments
                    .iter()
                    .any(|attachment| attachment.filePath == currentPath)
                {
                    let fileName = attachmentInfo.fileName.clone();
                    self.attachments.push(attachmentInfo);
                    self.messageProcessingDelegate
                        .showToast(format!("已添加附件: {fileName}"));
                }
            }
            Err(message) => {
                self.messageProcessingDelegate.showToast(message);
            }
        }
    }

    /// Adds the supplied pasted text as an in-memory plain-text attachment.
    #[allow(non_snake_case)]
    fn attachPastedText(&mut self, content: String) {
        let attachmentInfo = AttachmentInfo {
            filePath: format!(
                "pasted_text_{}_{}",
                currentTimeMillis(),
                self.attachments.len()
            ),
            fileName: "pasted_text.txt".to_string(),
            mimeType: "text/plain".to_string(),
            fileSize: content.len() as i64,
            content,
        };
        self.attachments.push(attachmentInfo);
        self.messageProcessingDelegate
            .showToast("已添加粘贴文本附件".to_string());
    }

    #[allow(non_snake_case)]
    fn captureScreenContent(&mut self) {
        let mut toolHandler = self.runtimeToolHandler();
        let result = toolHandler.executeTool(AITool {
            name: "capture_screenshot".to_string(),
            parameters: Vec::new(),
        });
        if !result.success {
            self.messageProcessingDelegate
                .showToast(format!("添加屏幕内容失败: {}", toolFailureMessage(&result)));
            return;
        }

        let screenshotPath = result.result.toString().trim().to_string();
        if screenshotPath.is_empty() {
            self.messageProcessingDelegate
                .showToast("添加屏幕内容失败: 截图失败".to_string());
            return;
        }

        let screenshotBytes = match self.fileSystemHost.readFileBytes(&screenshotPath) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.messageProcessingDelegate
                    .showToast(format!("添加屏幕内容失败: {}", error.message));
                return;
            }
        };
        let positionInfo = match image::load_from_memory(&screenshotBytes) {
            Ok(image) if image.width() > 0 && image.height() > 0 => {
                let width = image.width();
                let height = image.height();
                format!("【位置】full_screen; image_px={}x{}", width, height)
            }
            _ => "【位置】full_screen".to_string(),
        };

        let ocrText =
            OCRUtils::recognizeText(&toolHandler.getContext(), &screenshotPath, OCRQuality::HIGH);
        let ocrText = ocrText.trim().to_string();
        if ocrText.is_empty() {
            self.messageProcessingDelegate
                .showToast("添加屏幕内容失败: 未识别到屏幕文字".to_string());
            return;
        }

        let captureId = format!("screen_ocr_{}", currentTimeMillis());
        let content = format!("屏幕内容{positionInfo}\n\n{ocrText}\n\n{OCR_INLINE_INSTRUCTION}");
        self.attachments.push(AttachmentInfo {
            filePath: captureId,
            fileName: "screen_content.txt".to_string(),
            mimeType: "text/plain".to_string(),
            fileSize: content.len() as i64,
            content,
        });
        self.messageProcessingDelegate
            .showToast("已添加屏幕内容".to_string());

        if let Err(error) = self.fileSystemHost.deleteFile(&screenshotPath, false) {
            AppLogger::w(
                "ChatServiceCore",
                &format!("cannot remove captured screenshot: {}", error.message),
            );
        }
    }

    #[allow(non_snake_case)]
    fn captureNotifications(&mut self, limit: i32) {
        let mut toolHandler = self.runtimeToolHandler();
        let result = toolHandler.executeTool(AITool {
            name: "get_notifications".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "limit".to_string(),
                    value: limit.to_string(),
                },
                ToolParameter {
                    name: "include_ongoing".to_string(),
                    value: "true".to_string(),
                },
            ],
        });
        if !result.success {
            self.messageProcessingDelegate
                .showToast(format!("添加当前通知失败: {}", toolFailureMessage(&result)));
            return;
        }

        let content = result.result.toString();
        let attachmentInfo = AttachmentInfo {
            filePath: format!("notifications_{}", currentTimeMillis()),
            fileName: "notifications.json".to_string(),
            mimeType: "application/json".to_string(),
            fileSize: content.len() as i64,
            content,
        };
        self.attachments.push(attachmentInfo);
        self.messageProcessingDelegate
            .showToast("已添加当前通知".to_string());
    }

    #[allow(non_snake_case)]
    fn captureLocation(&mut self, highAccuracy: bool) {
        let mut toolHandler = self.runtimeToolHandler();
        let result = toolHandler.executeTool(AITool {
            name: "get_device_location".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "high_accuracy".to_string(),
                    value: highAccuracy.to_string(),
                },
                ToolParameter {
                    name: "timeout".to_string(),
                    value: "10".to_string(),
                },
            ],
        });
        if !result.success {
            self.messageProcessingDelegate
                .showToast(format!("添加当前位置失败: {}", toolFailureMessage(&result)));
            return;
        }

        let content = result.result.toString();
        let attachmentInfo = AttachmentInfo {
            filePath: format!("location_{}", currentTimeMillis()),
            fileName: "location.json".to_string(),
            mimeType: "application/json".to_string(),
            fileSize: content.len() as i64,
            content,
        };
        self.attachments.push(attachmentInfo);
        self.messageProcessingDelegate
            .showToast("已添加当前位置".to_string());
    }

    #[allow(non_snake_case)]
    fn attachPackageInternal(&mut self, packageName: &str) {
        if packageName.is_empty() {
            self.messageProcessingDelegate
                .showToast(format!("添加包失败: {packageName}"));
            return;
        }

        let toolHandler = self.runtimeToolHandler();
        let packageManager = toolHandler.getOrCreatePackageManager();
        let isStandardPackage;
        let isSkillPackage;
        let isMcpPackage;
        {
            let packageManagerGuard = packageManager
                .lock()
                .expect("package manager mutex poisoned");
            isStandardPackage = packageManagerGuard
                .getAvailablePackages()
                .contains_key(packageName)
                && !packageManagerGuard.isToolPkgContainer(packageName);
            isMcpPackage = packageManagerGuard
                .getAvailableServerPackages()
                .contains_key(packageName);
        }
        isSkillPackage =
            SkillRepository::getInstance(&toolHandler.getContext(), toolHandler.runtimeSupport())
                .getAiVisibleSkillPackages()
                .contains_key(packageName);

        if !isStandardPackage && !isSkillPackage && !isMcpPackage {
            self.messageProcessingDelegate
                .showToast(format!("添加包失败: {packageName}"));
            return;
        }

        {
            let mut packageManagerGuard = packageManager
                .lock()
                .expect("package manager mutex poisoned");
            if isStandardPackage {
                packageManagerGuard.enablePackage(packageName);
            }
            let packageContent = packageManagerGuard.usePackage(packageName);
            if isPackageAttachmentError(packageName, &packageContent) {
                self.messageProcessingDelegate
                    .showToast(format!("添加包失败: {packageName}"));
                return;
            }

            let attachmentInfo = AttachmentInfo {
                filePath: packageAttachmentPath(packageName),
                fileName: packageAttachmentDisplayName(packageName),
                mimeType: "text/plain".to_string(),
                fileSize: packageContent.len() as i64,
                content: packageContent,
            };
            self.attachments
                .retain(|attachment| attachment.filePath != attachmentInfo.filePath);
            self.attachments.push(attachmentInfo);
        }

        self.messageProcessingDelegate
            .showToast(format!("已添加包: {packageName}"));
    }

    #[allow(non_snake_case)]
    fn createAttachmentInfo(&self, filePath: &str) -> Result<AttachmentInfo, String> {
        let localPath = resolveAttachmentPath(filePath)?;
        let localPathText = localPath.to_string_lossy();
        let source = self
            .fileSystemHost
            .fileExists(&localPathText)
            .map_err(|error| error.message)?;
        if !source.exists {
            return Err("附件文件不存在".to_string());
        }
        if source.isDirectory {
            return Err(format!("无法添加附件: {}", localPath.display()));
        }

        let fileName = localPath
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("无法添加附件: {}", localPath.display()))?
            .to_string();
        let mimeType = getMimeTypeFromPath(&localPath).to_string();
        let tempFile = createTempFileFromPath(self.fileSystemHost.as_ref(), &localPath, &fileName)?;
        let fileSize = self
            .fileSystemHost
            .fileExists(&tempFile.to_string_lossy())
            .map_err(|error| format!("无法读取附件大小: {}", error.message))?
            .size;

        Ok(AttachmentInfo {
            filePath: tempFile.to_string_lossy().into_owned(),
            fileName,
            mimeType,
            fileSize,
            content: String::new(),
        })
    }

    /// Removes one attachment by its stored file path.
    pub fn removeAttachment(&mut self, _filePath: String) {
        self.attachments
            .retain(|attachment| attachment.filePath != _filePath);
    }

    /// Removes every pending attachment from the chat input.
    pub fn clearAttachments(&mut self) {
        self.attachments.clear();
    }

    /// Returns chat ids that currently have active streaming turns.
    pub fn activeStreamingChatIds(&self) -> Vec<String> {
        self.messageProcessingDelegate
            .activeStreamingChatIdsFlow()
            .value()
            .into_iter()
            .collect()
    }

    /// Returns the state flow of chat ids that currently have active streaming turns.
    pub fn activeStreamingChatIdsFlow(&self) -> StateFlow<std::collections::HashSet<String>> {
        self.messageProcessingDelegate.activeStreamingChatIdsFlow()
    }

    /// Returns the state flow of processing states keyed by chat id.
    pub fn inputProcessingStateByChatIdFlow(
        &self,
    ) -> StateFlow<std::collections::HashMap<String, InputProcessingState>> {
        self.messageProcessingDelegate
            .inputProcessingStateByChatIdFlow()
    }

    /// Returns transient toast messages emitted by chat input actions.
    #[allow(non_snake_case)]
    pub fn toastEventFlow(&self) -> StateFlow<Option<String>> {
        self.messageProcessingDelegate.toastEventFlow()
    }

    /// Clears the current transient toast event after the UI has consumed it.
    #[allow(non_snake_case)]
    pub fn clearToastEvent(&mut self) {
        self.messageProcessingDelegate.clearToastEvent();
    }

    /// Returns the processing state for the currently selected chat.
    #[allow(non_snake_case)]
    pub fn currentChatInputProcessingState(&self) -> InputProcessingState {
        let Some(chatId) = self.chatHistoryDelegate.currentChatIdFlow().value() else {
            return InputProcessingState::Idle;
        };
        match self
            .messageProcessingDelegate
            .inputProcessingStateByChatIdFlow()
            .value()
            .get(&chatId)
            .cloned()
        {
            Some(state) => state,
            None => InputProcessingState::Idle,
        }
    }

    /// Returns whether the currently selected chat is actively streaming.
    #[allow(non_snake_case)]
    pub fn currentChatIsLoading(&self) -> bool {
        let Some(chatId) = self.chatHistoryDelegate.currentChatIdFlow().value() else {
            return false;
        };
        self.messageProcessingDelegate
            .activeStreamingChatIdsFlow()
            .value()
            .contains(&chatId)
    }

    /// Returns whether older messages exist beyond the current display window.
    #[allow(non_snake_case)]
    pub fn hasOlderDisplayHistory(&self) -> bool {
        self.chatHistoryDelegate.hasOlderDisplayHistory
    }

    /// Returns whether newer messages exist beyond the current display window.
    #[allow(non_snake_case)]
    pub fn hasNewerDisplayHistory(&self) -> bool {
        self.chatHistoryDelegate.hasNewerDisplayHistory
    }

    /// Returns whether the display-window loader is currently fetching messages.
    #[allow(non_snake_case)]
    pub fn isLoadingDisplayWindow(&self) -> bool {
        self.chatHistoryDelegate.isLoadingDisplayWindow
    }

    /// Returns tool invocation counts for the current turn keyed by chat id.
    pub fn currentTurnToolInvocationCountByChatId(
        &self,
    ) -> &std::collections::HashMap<String, i32> {
        &self
            .messageProcessingDelegate
            .currentTurnToolInvocationCountByChatId
    }

    /// Returns the state flow of tool invocation counts keyed by chat id.
    pub fn currentTurnToolInvocationCountByChatIdFlow(
        &self,
    ) -> StateFlow<std::collections::HashMap<String, i32>> {
        self.messageProcessingDelegate
            .currentTurnToolInvocationCountByChatIdFlow()
    }

    /// Returns the in-memory messages for the current chat.
    pub fn chatHistory(&self) -> Vec<ChatMessage> {
        self.chatHistoryDelegate.currentChatMessagesSnapshot()
    }

    /// Returns the state flow of the currently selected chat id.
    #[allow(non_snake_case)]
    pub fn currentChatIdFlow(&self) -> StateFlow<Option<String>> {
        self.chatHistoryDelegate.currentChatIdFlow()
    }

    /// Returns a current snapshot of all persisted chat histories.
    pub fn chatHistories(&self) -> Vec<operit_model::ChatHistory::ChatHistory> {
        self.chatHistoryDelegate.chatHistoriesFlow().value()
    }

    /// Returns the state flow of all persisted chat histories.
    #[allow(non_snake_case)]
    pub fn chatHistoriesFlow(&self) -> StateFlow<Vec<operit_model::ChatHistory::ChatHistory>> {
        self.chatHistoryDelegate.chatHistoriesFlow()
    }

    /// Returns chat history list items prepared for grouped history UI.
    #[allow(non_snake_case)]
    pub fn chatHistoryListItemsFlow(&self) -> StateFlow<Vec<ChatHistoryListItem>> {
        self.chatHistoryDelegate.chatHistoryListItemsFlow()
    }

    /// Returns messages from the Core selected by Binding for one explicit chat.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId)]
    pub async fn chatMessagesFlow(&self, chatId: String) -> StateFlow<Vec<ChatMessage>> {
        self.localChatMessagesFlow(chatId)
    }

    /// Builds a routed diagnostic chat message flow with one embedded response stream.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId)]
    pub async fn routeProbeChatMessagesFlow(
        &self,
        chatId: String,
        streamText: String,
    ) -> StateFlow<Vec<ChatMessage>> {
        let responseStream = SharedAiResponseStream::new_ordered(
            mutable_shared_stream(usize::MAX),
            mutable_shared_stream(usize::MAX),
        );
        let streamKey = format!("route-probe:{chatId}");
        let streamId = format!("route-probe-stream:{chatId}");
        let source = coreResponseStreamSource(responseStream.clone(), streamKey);
        let mut message = ChatMessage::new("ai".to_string());
        message.contentStream = Some(CoreStream::fromSourceWithId(streamId, source));
        let flow = mutableStateFlow(vec![message]).asStateFlow();
        let chunkOne = format!("{streamText} / chunk-one");
        let chunkTwo = " / chunk-two".to_string();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "core-route-probe-response-stream",
                Box::new(move || {
                    Box::pin(async move {
                        responseStream.emit_chunk(chunkOne);
                        responseStream.emit_chunk(chunkTwo);
                        responseStream.close();
                    })
                }),
            )
            .expect("route probe stream task must schedule");
        flow
    }

    /// Returns messages from this local Core for one explicit chat.
    #[allow(non_snake_case)]
    pub fn localChatMessagesFlow(&self, chatId: String) -> StateFlow<Vec<ChatMessage>> {
        self.chatHistoryDelegate.chatMessageFlowForChat(chatId)
    }

    /// Returns runtime state from the Core selected by Binding for one explicit chat.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId)]
    pub async fn chatStateFlow(&self, chatId: String) -> StateFlow<ChatState> {
        self.localChatStateFlow(chatId)
    }

    /// Returns runtime state from this local Core for one explicit chat.
    #[allow(non_snake_case)]
    pub fn localChatStateFlow(&self, chatId: String) -> StateFlow<ChatState> {
        let selectedChatId = chatId;
        let displayWindowStateFlow = self
            .chatHistoryDelegate
            .displayWindowStateFlowForChat(selectedChatId.clone());
        let executionStateFlow = self
            .messageProcessingDelegate
            .executionStateByChatIdFlow()
            .map({
                let selectedChatId = selectedChatId.clone();
                move |states| {
                    states
                        .get(&selectedChatId)
                        .cloned()
                        .unwrap_or_else(ChatExecutionState::idle)
                }
            });
        let chatHistoriesFlow = self.chatHistoryDelegate.chatHistoriesFlow();
        let characterCardManager = self.chatHistoryDelegate.characterCardManager.clone();
        combine3(
            &executionStateFlow,
            &displayWindowStateFlow,
            &chatHistoriesFlow,
            move |executionState, displayWindowState, chatHistories| {
                let currentChat = chatHistories.iter().find(|chat| chat.id == selectedChatId);
                let currentCharacterCardName =
                    currentChat.and_then(|chat| chat.characterCardName.clone());
                ChatState {
                    currentChatId: selectedChatId.clone(),
                    currentChatTitle: currentChat
                        .map(|chat| chat.title.clone())
                        .unwrap_or_default(),
                    currentCharacterCardAvatarUri: currentCharacterCardName
                        .as_deref()
                        .and_then(|name| characterCardAvatarUriByName(&characterCardManager, name)),
                    currentCharacterCardName,
                    currentWorkspacePath: currentChat.and_then(|chat| chat.workspace.clone()),
                    isLoading: executionState.isLoading,
                    inputProcessingState: executionState.inputProcessingState,
                    hasOlderDisplayHistory: displayWindowState.hasOlderDisplayHistory,
                    hasNewerDisplayHistory: displayWindowState.hasNewerDisplayHistory,
                    isLoadingDisplayWindow: displayWindowState.isLoadingDisplayWindow,
                }
            },
        )
    }

    /// Returns whether the chat history selector should be visible.
    pub fn showChatHistorySelector(&self) -> bool {
        self.chatHistoryDelegate.showChatHistorySelector
    }

    /// Returns a snapshot of pending input attachments.
    pub fn attachments(&self) -> Vec<AttachmentInfo> {
        self.attachments.clone()
    }

    /// Returns mutable access to chat history operations for host-side integrations.
    pub fn getChatHistoryDelegate(&mut self) -> &mut ChatHistoryDelegate {
        &mut self.chatHistoryDelegate
    }

    /// Returns mutable access to message processing state for host-side integrations.
    pub fn getMessageProcessingDelegate(&mut self) -> &mut MessageProcessingDelegate {
        &mut self.messageProcessingDelegate
    }

    /// Returns mutable access to message coordination state when enhanced AI is initialized.
    pub fn getMessageCoordinationDelegate(&mut self) -> Option<&mut MessageCoordinationDelegate> {
        self.messageCoordinationDelegate.as_mut()
    }

    /// Returns token statistics state owned by the coordination delegate.
    #[allow(non_snake_case)]
    pub fn getTokenStatisticsDelegate(&self) -> Option<&TokenStatisticsDelegate> {
        self.messageCoordinationDelegate
            .as_ref()
            .map(|delegate| &delegate.tokenStatisticsDelegate)
    }

    /// Returns the current context window size state flow.
    #[allow(non_snake_case)]
    pub fn currentWindowSizeFlow(&self) -> StateFlow<i64> {
        self.getTokenStatisticsDelegate()
            .expect("TokenStatisticsDelegate must be initialized")
            .currentWindowSizeFlow()
    }

    /// Returns the cumulative input token count state flow.
    #[allow(non_snake_case)]
    pub fn inputTokenCountFlow(&self) -> StateFlow<i64> {
        self.getTokenStatisticsDelegate()
            .expect("TokenStatisticsDelegate must be initialized")
            .cumulativeInputTokensFlow()
    }

    /// Returns the cumulative output token count state flow.
    #[allow(non_snake_case)]
    pub fn outputTokenCountFlow(&self) -> StateFlow<i64> {
        self.getTokenStatisticsDelegate()
            .expect("TokenStatisticsDelegate must be initialized")
            .cumulativeOutputTokensFlow()
    }

    /// Returns the enhanced AI service used by this chat core.
    pub fn getEnhancedAiService(&self) -> Option<&EnhancedAIService> {
        self.enhancedAiService.as_ref()
    }

    /// Returns whether this chat core has finished delegate initialization.
    pub fn isInitialized(&self) -> bool {
        self.initialized
    }

    /// Registers a callback invoked when the enhanced AI service becomes ready.
    pub fn setOnEnhancedAiServiceReady(&mut self, callback: fn(&EnhancedAIService)) {
        self.onEnhancedAiServiceReady = Some(callback);
    }

    /// Registers an optional callback invoked when a chat turn completes.
    pub fn setAdditionalOnTurnComplete(
        &mut self,
        callback: Option<fn(Option<String>, i32, i32, i32)>,
    ) {
        self.additionalOnTurnComplete = callback;
    }

    /// Replaces the UI bridge used by this chat core.
    pub fn setUiBridge(&mut self, uiBridge: EmptyChatServiceUiBridge) {
        self.uiBridge = uiBridge;
    }

    /// Registers the speech handler used by message playback actions.
    pub fn setSpeakMessageHandler(&mut self, handler: fn(String, bool)) {
        self.messageProcessingDelegate
            .setSpeakMessageHandler(handler);
    }

    /// Reloads chat messages using the display-window strategy for the requested chat.
    pub fn reloadChatMessagesSmart(&mut self, chatId: String) {
        self.chatHistoryDelegate.reloadChatMessagesSmart(chatId);
    }

    /// Loads older messages into the current chat display window.
    pub fn loadOlderMessagesForCurrentChat(&mut self) {
        self.chatHistoryDelegate.loadOlderMessagesForCurrentChat();
    }

    /// Loads newer messages into the current chat display window.
    pub fn loadNewerMessagesForCurrentChat(&mut self) {
        self.chatHistoryDelegate.loadNewerMessagesForCurrentChat();
    }

    /// Moves the current chat display window to the latest messages.
    pub fn showLatestMessagesForCurrentChat(&mut self) {
        self.chatHistoryDelegate.showLatestMessagesForCurrentChat();
    }

    /// Searches a chat and returns lightweight message previews for navigation.
    #[allow(non_snake_case)]
    pub fn loadChatMessageLocatorPreviews(
        &self,
        chatId: String,
        query: String,
    ) -> Vec<ChatMessageLocatorPreview> {
        self.chatHistoryDelegate
            .loadChatMessageLocatorPreviews(chatId, query)
    }

    /// Marks or unmarks one message as a favorite by message timestamp.
    #[allow(non_snake_case)]
    pub fn setMessageFavorite(&mut self, timestamp: i64, isFavorite: bool) {
        self.chatHistoryDelegate
            .setMessageFavorite(timestamp, isFavorite);
    }
}

#[allow(non_snake_case)]
fn stripXmlLikeTags(text: &str) -> String {
    let mut value = text.to_string();
    for _ in 0..5 {
        let updated = removePairedXmlLikeTags(&value);
        if updated == value {
            break;
        }
        value = updated;
    }
    value = removeSelfClosingXmlLikeTags(&value);
    removeRemainingXmlLikeTags(&value).trim().to_string()
}

#[allow(non_snake_case)]
fn removePairedXmlLikeTags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;

    while let Some(openRelativeStart) = text[cursor..].find('<') {
        let openStart = cursor + openRelativeStart;
        let Some(openEnd) = text[openStart..].find('>').map(|offset| openStart + offset) else {
            break;
        };

        if let Some(tagName) = parseOpeningXmlLikeTag(text, openStart, openEnd) {
            if let Some(closeEnd) = findClosingXmlLikeTagEnd(text, openEnd + 1, tagName) {
                result.push_str(&text[cursor..openStart]);
                cursor = closeEnd;
                continue;
            }
        }

        result.push_str(&text[cursor..openStart + 1]);
        cursor = openStart + 1;
    }

    result.push_str(&text[cursor..]);
    result
}

#[allow(non_snake_case)]
fn removeSelfClosingXmlLikeTags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;

    while let Some(openRelativeStart) = text[cursor..].find('<') {
        let openStart = cursor + openRelativeStart;
        let Some(openEnd) = text[openStart..].find('>').map(|offset| openStart + offset) else {
            break;
        };

        if parseSelfClosingXmlLikeTag(text, openStart, openEnd) {
            result.push_str(&text[cursor..openStart]);
            cursor = openEnd + 1;
            continue;
        }

        result.push_str(&text[cursor..openStart + 1]);
        cursor = openStart + 1;
    }

    result.push_str(&text[cursor..]);
    result
}

#[allow(non_snake_case)]
fn removeRemainingXmlLikeTags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;

    while let Some(openRelativeStart) = text[cursor..].find('<') {
        let openStart = cursor + openRelativeStart;
        let Some(openEnd) = text[openStart..].find('>').map(|offset| openStart + offset) else {
            break;
        };

        result.push_str(&text[cursor..openStart]);
        cursor = openEnd + 1;
    }

    result.push_str(&text[cursor..]);
    result
}

#[allow(non_snake_case)]
fn parseOpeningXmlLikeTag(text: &str, openStart: usize, openEnd: usize) -> Option<&str> {
    let body = text.get(openStart + 1..openEnd)?;
    if body.starts_with('/') || body.trim_end().ends_with('/') {
        return None;
    }
    parseXmlLikeTagName(body)
}

#[allow(non_snake_case)]
fn parseSelfClosingXmlLikeTag(text: &str, openStart: usize, openEnd: usize) -> bool {
    let Some(body) = text.get(openStart + 1..openEnd) else {
        return false;
    };
    if body.starts_with('/') || !body.trim_end().ends_with('/') {
        return false;
    }
    parseXmlLikeTagName(body).is_some()
}

#[allow(non_snake_case)]
fn parseXmlLikeTagName(body: &str) -> Option<&str> {
    let bytes = body.as_bytes();
    let first = *bytes.first()?;
    if !isXmlLikeTagNameStart(first) {
        return None;
    }

    let mut end = 1;
    while end < bytes.len() && isXmlLikeTagNameChar(bytes[end]) {
        end += 1;
    }

    if end < bytes.len() {
        let rest = &body[end..];
        if !rest
            .chars()
            .next()
            .is_some_and(|value| value.is_whitespace())
        {
            return None;
        }
    }

    Some(&body[..end])
}

#[allow(non_snake_case)]
fn findClosingXmlLikeTagEnd(text: &str, from: usize, tagName: &str) -> Option<usize> {
    let mut searchStart = 0;

    while let Some(relativeStart) = text[from + searchStart..].find("</") {
        let closeStart = from + searchStart + relativeStart;
        if let Some(closeEnd) = text[closeStart..]
            .find('>')
            .map(|offset| closeStart + offset)
        {
            let body = &text[closeStart + 2..closeEnd];
            if body.eq_ignore_ascii_case(tagName) {
                return Some(closeEnd + 1);
            }
        }
        searchStart += relativeStart + 2;
    }

    None
}

#[allow(non_snake_case)]
fn isXmlLikeTagNameStart(value: u8) -> bool {
    value.is_ascii_alphabetic()
}

#[allow(non_snake_case)]
fn isXmlLikeTagNameChar(value: u8) -> bool {
    value.is_ascii_alphanumeric() || matches!(value, b':' | b'_' | b'-')
}

#[allow(non_snake_case)]
fn resolveAttachmentPath(filePath: &str) -> Result<PathBuf, String> {
    if filePath.starts_with("file://") {
        let url = Url::parse(filePath).map_err(|_| format!("无法添加附件: {filePath}"))?;
        return fileUrlToPathBuf(&url).map_err(|_| format!("无法添加附件: {filePath}"));
    }
    Ok(PathBuf::from(filePath))
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(non_snake_case)]
fn fileUrlToPathBuf(url: &Url) -> Result<PathBuf, ()> {
    url.to_file_path().map_err(|_| ())
}

#[cfg(target_arch = "wasm32")]
#[allow(non_snake_case)]
fn fileUrlToPathBuf(url: &Url) -> Result<PathBuf, ()> {
    if url.scheme() != "file" {
        return Err(());
    }
    Ok(PathBuf::from(url.path()))
}

#[allow(non_snake_case)]
/// Copies an attachment into clean-on-exit storage through the supplied file-system host.
fn createTempFileFromPath(
    fileSystemHost: &dyn FileSystemHost,
    sourcePath: &Path,
    fileName: &str,
) -> Result<PathBuf, String> {
    let fileExtension = fileName
        .rsplit_once('.')
        .map(|(_, extension)| extension)
        .filter(|extension| !extension.trim().is_empty())
        .unwrap_or("jpg");
    let externalDir = OperitPaths::cleanOnExitDir()?;
    let externalDirText = externalDir.to_string_lossy();
    fileSystemHost
        .makeDirectory(&externalDirText, true)
        .map_err(|error| format!("无法创建附件临时目录: {}", error.message))?;
    let noMediaFile = externalDir.join(".nomedia");
    fileSystemHost
        .writeFile(&noMediaFile.to_string_lossy(), "", false)
        .map_err(|error| format!("无法创建附件媒体标记: {}", error.message))?;
    let tempFile = externalDir.join(format!("img_{}.{}", currentTimeMillis(), fileExtension));
    fileSystemHost
        .copyFile(
            &sourcePath.to_string_lossy(),
            &tempFile.to_string_lossy(),
            false,
        )
        .map_err(|error| format!("无法复制附件: {}", error.message))?;
    let copied = fileSystemHost
        .fileExists(&tempFile.to_string_lossy())
        .map_err(|error| format!("无法读取附件临时文件: {}", error.message))?;
    if !copied.exists || copied.isDirectory || copied.size == 0 {
        return Err(format!("无法添加附件: {}", sourcePath.display()));
    }
    Ok(tempFile)
}

#[allow(non_snake_case)]
fn getMimeTypeFromPath(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("heic") => "image/heic",
        Some("txt") => "text/plain",
        Some("json") => "application/json",
        Some("xml") => "application/xml",
        Some("pdf") => "application/pdf",
        Some("doc") | Some("docx") => "application/msword",
        Some("xls") | Some("xlsx") => "application/vnd.ms-excel",
        Some("zip") => "application/zip",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("m4a") => "audio/mp4",
        Some("aac") => "audio/aac",
        Some("ogg") => "audio/ogg",
        Some("flac") => "audio/flac",
        Some("mp4") => "video/mp4",
        Some("mkv") => "video/x-matroska",
        Some("webm") => "video/webm",
        Some("3gp") => "video/3gpp",
        Some("avi") => "video/x-msvideo",
        Some("mov") => "video/quicktime",
        _ => "application/octet-stream",
    }
}

#[allow(non_snake_case)]
fn packageAttachmentPath(packageName: &str) -> String {
    format!("{PACKAGE_ATTACHMENT_PREFIX}{packageName}")
}

#[allow(non_snake_case)]
fn packageAttachmentDisplayName(packageName: &str) -> String {
    format!("包: {packageName}")
}

#[allow(non_snake_case)]
fn isPackageAttachmentError(packageName: &str, packageContent: &str) -> bool {
    if packageContent.trim().is_empty() {
        return true;
    }
    packageContent.starts_with("Package not found: ")
        || packageContent.starts_with("Failed to load package data for: ")
        || packageContent.starts_with("Missing required environment variables for package ")
        || packageContent.starts_with("ToolPkg container '")
        || packageContent.starts_with("MCP server '")
        || packageContent.starts_with("Cannot connect to MCP server")
        || packageContent.starts_with("Cannot get MCP server configuration")
        || packageContent == format!("Skill '{packageName}' is set to not show to AI")
}

#[allow(non_snake_case)]
fn toolFailureMessage(result: &ToolResult) -> String {
    let message = result.error.clone().unwrap_or_default();
    if !message.trim().is_empty() {
        return message;
    }
    result.result.toString()
}
