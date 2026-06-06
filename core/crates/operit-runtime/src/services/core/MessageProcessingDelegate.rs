use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::api::chat::llmprovider::AIService::SharedAiResponseStream;
use crate::api::chat::EnhancedAIService::{
    EnhancedAIService, SendMessageCallbacks, SendMessageOptions,
};
use crate::core::chat::AIMessageManager::{
    logMessageTiming, messageTimingNow, AIMessageManager, BuildUserMessageContentRequest,
    SendMessageRequest as AIMessageSendRequest, StableContextWindowRequest,
};
use crate::core::tools::ToolProgressBus::ToolProgressBus;
use crate::data::model::AttachmentInfo::AttachmentInfo;
use crate::data::model::ChatMessage::ChatMessage;
use crate::data::model::ChatMessageDisplayMode::ChatMessageDisplayMode;
use crate::data::model::ChatMessageTimestampAllocator::ChatMessageTimestampAllocator;
use crate::data::model::ChatTurnOptions::ChatTurnOptions;
use crate::data::model::FunctionType::FunctionType;
use crate::data::model::InputProcessingState::InputProcessingState;
use crate::data::model::PromptFunctionType::PromptFunctionType;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use crate::services::core::ChatHistoryDelegate::ChatHistoryDelegate;
use crate::ui::features::chat::webview::workspace::WorkspaceBackupManager::WorkspaceBackupManager;
use crate::util::stream::HotStream::SharedStream;
use crate::util::stream::RevisableTextStream::{TextStreamEventCarrier, TextStreamEventType};
use crate::util::stream::Stream::Stream;
use crate::util::stream::TextStreamRevisionTracker::TextStreamRevisionTracker;
use operit_store::PreferencesDataStore::{mutableStateFlow, MutableStateFlow, StateFlow};

pub const STREAM_PERSIST_INTERVAL_MS: i64 = 1000;
pub const AUTO_READ_PREVIEW_MAX: usize = 48;

#[derive(Clone, Debug, PartialEq)]
pub struct TextFieldValue {
    pub text: String,
}

impl TextFieldValue {
    pub fn new(text: String) -> Self {
        Self { text }
    }
}

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

#[derive(Clone, Debug)]
pub struct TurnCancellationSnapshot {
    pub chatId: String,
    pub aiMessage: Option<ChatMessage>,
    pub partialContent: String,
    pub turnOptions: ChatTurnOptions,
}

pub struct BuildUserMessageContentForSendRequest {
    pub messageText: String,
    pub proxySenderNameOverride: Option<String>,
    pub attachments: Vec<AttachmentInfo>,
    pub workspacePath: Option<String>,
    pub workspaceEnv: Option<String>,
    pub replyToMessage: Option<ChatMessage>,
    pub chatId: String,
    pub roleCardId: String,
    pub chatModelConfigIdOverride: Option<String>,
}

pub struct BuildUserMessageContentForGroupOrchestrationRequest {
    pub messageText: String,
    pub attachments: Vec<AttachmentInfo>,
    pub workspacePath: Option<String>,
    pub workspaceEnv: Option<String>,
    pub replyToMessage: Option<ChatMessage>,
    pub chatId: String,
    pub roleCardId: String,
}

pub struct SendUserMessageProcessingRequest<'a> {
    pub enhancedAiService: &'a mut EnhancedAIService,
    pub chatHistoryDelegate: &'a mut ChatHistoryDelegate,
    pub chatId: String,
    pub messageText: String,
    pub chatHistory: Vec<ChatMessage>,
    pub workspacePath: Option<String>,
    pub workspaceEnv: Option<String>,
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
    pub chatModelConfigIdOverride: Option<String>,
    pub chatModelIndexOverride: Option<i32>,
    pub preferenceProfileIdOverride: Option<String>,
    pub isGroupOrchestrationTurn: bool,
    pub groupParticipantNamesText: Option<String>,
    pub proxySenderNameOverride: Option<String>,
    pub suppressUserMessageInHistory: bool,
    pub isAutoContinuation: bool,
    pub turnOptions: ChatTurnOptions,
}

#[derive(Clone, Debug)]
pub struct SendUserMessageProcessingResult {
    pub aiMessage: ChatMessage,
    pub nextWindowSize: Option<i32>,
}

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
    pub chatModelConfigIdOverride: Option<String>,
    pub chatModelIndexOverride: Option<i32>,
    pub preferenceProfileIdOverride: Option<String>,
}

pub struct MessageProcessingDelegate {
    pub functionalConfigManager: FunctionalConfigManager,
    pub modelConfigManager: ModelConfigManager,
    pub userMessage: TextFieldValue,
    pub userMessageFlow: MutableStateFlow<TextFieldValue>,
    pub isLoading: bool,
    pub isLoadingFlow: MutableStateFlow<bool>,
    pub activeStreamingChatIds: HashSet<String>,
    pub activeStreamingChatIdsFlow: MutableStateFlow<HashSet<String>>,
    pub inputProcessingStateByChatId: HashMap<String, InputProcessingState>,
    pub inputProcessingStateByChatIdFlow: MutableStateFlow<HashMap<String, InputProcessingState>>,
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

struct MessageProcessingCallbacks {
    nonFatalErrorEventFlow: MutableStateFlow<Option<String>>,
}

impl SendMessageCallbacks for MessageProcessingCallbacks {
    #[allow(non_snake_case)]
    fn onNonFatalError(&self, error: String) {
        self.nonFatalErrorEventFlow.set_value(Some(error));
    }
}

impl MessageProcessingDelegate {
    pub fn new(
        functionalConfigManager: FunctionalConfigManager,
        modelConfigManager: ModelConfigManager,
    ) -> Self {
        Self {
            functionalConfigManager,
            modelConfigManager,
            userMessage: TextFieldValue::new(String::new()),
            userMessageFlow: mutableStateFlow(TextFieldValue::new(String::new())),
            isLoading: false,
            isLoadingFlow: mutableStateFlow(false),
            activeStreamingChatIds: HashSet::new(),
            activeStreamingChatIdsFlow: mutableStateFlow(HashSet::new()),
            inputProcessingStateByChatId: HashMap::new(),
            inputProcessingStateByChatIdFlow: mutableStateFlow(HashMap::new()),
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

    #[allow(non_snake_case)]
    pub fn clone_for_core(&self) -> Self {
        let rootDir = ApiPreferences::data_dir();
        Self {
            functionalConfigManager: FunctionalConfigManager::new(rootDir.clone()),
            modelConfigManager: ModelConfigManager::new(rootDir),
            userMessage: self.userMessageFlow.value(),
            userMessageFlow: self.userMessageFlow.clone(),
            isLoading: self.isLoadingFlow.value(),
            isLoadingFlow: self.isLoadingFlow.clone(),
            activeStreamingChatIds: self.activeStreamingChatIdsFlow.value(),
            activeStreamingChatIdsFlow: self.activeStreamingChatIdsFlow.clone(),
            inputProcessingStateByChatId: self.inputProcessingStateByChatIdFlow.value(),
            inputProcessingStateByChatIdFlow: self.inputProcessingStateByChatIdFlow.clone(),
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

    #[allow(non_snake_case)]
    pub fn showToast(&mut self, message: String) {
        self.toastEventFlow.set_value(Some(message));
    }

    #[allow(non_snake_case)]
    pub fn emitNonFatalError(&mut self, message: String) {
        self.nonFatalErrorEvent.push(message.clone());
        self.nonFatalErrorEventFlow.set_value(Some(message));
    }

    #[allow(non_snake_case)]
    pub fn nonFatalErrorEventFlow(&self) -> StateFlow<Option<String>> {
        self.nonFatalErrorEventFlow.asStateFlow()
    }

    #[allow(non_snake_case)]
    pub fn clearToastEvent(&mut self) {
        self.toastEventFlow.set_value(None);
    }

    #[allow(non_snake_case)]
    pub fn toastEventFlow(&self) -> StateFlow<Option<String>> {
        self.toastEventFlow.asStateFlow()
    }

    #[allow(non_snake_case)]
    pub fn speechPreview(text: String) -> String {
        text.replace('\n', "\\n")
            .chars()
            .take(AUTO_READ_PREVIEW_MAX)
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn chatKey(chatId: Option<String>) -> String {
        chatId.unwrap_or_else(|| "__DEFAULT_CHAT__".to_string())
    }

    #[allow(non_snake_case)]
    pub fn tryEmitScrollToBottomThrottled(&mut self, chatId: Option<String>) {
        let key = Self::chatKey(chatId);
        self.lastScrollEmitMsByChatKey
            .lock()
            .expect("last scroll emit map mutex poisoned")
            .insert(key, messageTimingNow().startedAtMs as i64);
        self.scrollToBottomEvent.push(());
    }

    #[allow(non_snake_case)]
    pub fn forceEmitScrollToBottom(&mut self, chatId: Option<String>) {
        let key = Self::chatKey(chatId);
        self.lastScrollEmitMsByChatKey
            .lock()
            .expect("last scroll emit map mutex poisoned")
            .insert(key, messageTimingNow().startedAtMs as i64);
        self.scrollToBottomEvent.push(());
    }

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

    #[allow(non_snake_case)]
    pub fn updateGlobalLoadingState(&mut self) {
        let (isLoading, activeStreamingChatIds) = {
            let runtimes = self
                .chatRuntimes
                .lock()
                .expect("chat runtimes mutex poisoned");
            let isLoading = runtimes.values().any(|runtime| runtime.isLoading);
            let activeStreamingChatIds = runtimes
                .iter()
                .filter(|(key, runtime)| key.as_str() != "__DEFAULT_CHAT__" && runtime.isLoading)
                .map(|(key, _)| key.clone())
                .collect();
            (isLoading, activeStreamingChatIds)
        };
        self.isLoading = isLoading;
        self.activeStreamingChatIds = activeStreamingChatIds;
        self.isLoadingFlow.set_value(self.isLoading);
        self.activeStreamingChatIdsFlow
            .set_value(self.activeStreamingChatIds.clone());
    }
    #[allow(non_snake_case)]
    pub fn refreshGlobalLoadingState(&mut self) {
        self.updateGlobalLoadingState();
    }

    #[allow(non_snake_case)]
    pub fn isTerminalInputState(state: &InputProcessingState) -> bool {
        matches!(
            state,
            InputProcessingState::Idle | InputProcessingState::Completed
        )
    }

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
        let mut states = self.inputProcessingStateByChatIdFlow.value();
        states.insert(key, state);
        self.inputProcessingStateByChatId = states.clone();
        self.inputProcessingStateByChatIdFlow.set_value(states);
    }

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

    #[allow(non_snake_case)]
    pub fn setInputProcessingStateForChat(&mut self, chatId: String, state: InputProcessingState) {
        self.setChatInputProcessingState(Some(chatId), state);
    }

    #[allow(non_snake_case)]
    pub fn buildUserMessageContentForGroupOrchestration(
        &self,
        request: BuildUserMessageContentForGroupOrchestrationRequest,
    ) -> Result<String, crate::api::chat::llmprovider::AIService::AiServiceError> {
        self.buildUserMessageContentForSend(BuildUserMessageContentForSendRequest {
            messageText: request.messageText,
            proxySenderNameOverride: None,
            attachments: request.attachments,
            workspacePath: request.workspacePath,
            workspaceEnv: request.workspaceEnv,
            replyToMessage: request.replyToMessage,
            chatId: request.chatId,
            roleCardId: request.roleCardId,
            chatModelConfigIdOverride: None,
        })
    }

    #[allow(non_snake_case)]
    pub fn buildUserMessageContentForSend(
        &self,
        request: BuildUserMessageContentForSendRequest,
    ) -> Result<String, crate::api::chat::llmprovider::AIService::AiServiceError> {
        let configId = match request.chatModelConfigIdOverride.as_ref() {
            Some(value) if !value.trim().is_empty() => value.clone(),
            _ => self
                .functionalConfigManager
                .getConfigIdForFunction(FunctionType::CHAT)
                .map_err(|error| {
                    crate::api::chat::llmprovider::AIService::AiServiceError::RequestFailed(
                        error.to_string(),
                    )
                })?,
        };

        let loadModelConfigStartTime = messageTimingNow();
        let currentModelConfig = self
            .modelConfigManager
            .getModelConfigFlow(&configId)
            .map_err(|error| {
                crate::api::chat::llmprovider::AIService::AiServiceError::RequestFailed(
                    error.to_string(),
                )
            })?
            .first()
            .map_err(|error| {
                crate::api::chat::llmprovider::AIService::AiServiceError::RequestFailed(
                    error.to_string(),
                )
            })?;
        let enableDirectImageProcessing = currentModelConfig.enableDirectImageProcessing;
        let enableDirectAudioProcessing = currentModelConfig.enableDirectAudioProcessing;
        let enableDirectVideoProcessing = currentModelConfig.enableDirectVideoProcessing;
        logMessageTiming(
            "delegate.loadModelConfig",
            loadModelConfigStartTime,
            Some(format!("chatId={}, configId={configId}", request.chatId)),
        );

        let buildUserMessageStartTime = messageTimingNow();
        let finalMessageContent =
            AIMessageManager::buildUserMessageContent(BuildUserMessageContentRequest {
                messageText: request.messageText,
                proxySenderName: request.proxySenderNameOverride,
                attachments: request.attachments,
                workspacePath: request.workspacePath,
                workspaceEnv: request.workspaceEnv,
                replyToMessage: request.replyToMessage,
                enableDirectImageProcessing,
                enableDirectAudioProcessing,
                enableDirectVideoProcessing,
                chatId: Some(request.chatId.clone()),
                roleCardId: Some(request.roleCardId),
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

    #[allow(non_snake_case)]
    pub fn getResponseStream(&self, chatId: String) -> Option<SharedAiResponseStream> {
        self.withExistingRuntime(Some(chatId), |runtime| runtime.responseStream.clone())
            .flatten()
    }

    #[allow(non_snake_case)]
    pub fn resolveFinalContent(aiMessage: ChatMessage) -> String {
        let replayChunks = aiMessage
            .contentStream
            .as_ref()
            .map(|stream| stream.replay_cache());
        let eventCarrier = aiMessage
            .contentStream
            .as_ref()
            .map(|stream| stream as &dyn TextStreamEventCarrier);

        if eventCarrier
            .map(|carrier| !carrier.event_channel().replay_cache().is_empty())
            .unwrap_or(false)
        {
            aiMessage.content
        } else if replayChunks
            .as_ref()
            .map(|chunks| !chunks.is_empty())
            .unwrap_or(false)
        {
            replayChunks.unwrap_or_default().join("")
        } else {
            aiMessage.content
        }
    }

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

    #[allow(non_snake_case)]
    pub fn detachStreamingAiMessage(&mut self, chatId: String) -> Option<ChatMessage> {
        let snapshot = self.readCurrentTurnCancellationSnapshot(chatId)?;
        snapshot.aiMessage
    }

    #[allow(non_snake_case)]
    pub fn cancelMessageInternal(&mut self, chatId: String, keepPartialResponse: bool) {
        if !keepPartialResponse {
            self.detachStreamingAiMessage(chatId.clone());
        }
        self.clearCurrentTurnToolInvocationCount(chatId.clone());
        AIMessageManager::cancelOperation(chatId.clone());
        self.withExistingRuntime(Some(chatId.clone()), |runtime| {
            if let Some(responseStream) = runtime.responseStream.as_ref() {
                responseStream.upstream.close();
                responseStream.event_channel.close();
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
        self.setInputProcessingStateForChat(chatId, InputProcessingState::Idle);
        self.updateGlobalLoadingState();
    }

    #[allow(non_snake_case)]
    pub fn cancelMessage(&mut self, chatId: String) {
        self.cancelMessageInternal(chatId, true);
    }

    #[allow(non_snake_case)]
    pub fn cancelMessageForDestructiveMutation(&mut self, chatId: String) {
        self.cancelMessageInternal(chatId, false);
    }

    #[allow(non_snake_case)]
    pub fn updateUserMessage(&mut self, message: String) {
        let value = TextFieldValue::new(message);
        self.userMessage = value.clone();
        self.userMessageFlow.set_value(value);
    }

    #[allow(non_snake_case)]
    pub fn updateUserMessageValue(&mut self, value: TextFieldValue) {
        self.userMessage = value.clone();
        self.userMessageFlow.set_value(value);
    }

    pub fn userMessageFlow(&self) -> StateFlow<TextFieldValue> {
        self.userMessageFlow.asStateFlow()
    }

    pub fn isLoadingFlow(&self) -> StateFlow<bool> {
        self.isLoadingFlow.asStateFlow()
    }

    pub fn activeStreamingChatIdsFlow(&self) -> StateFlow<HashSet<String>> {
        self.activeStreamingChatIdsFlow.asStateFlow()
    }

    pub fn inputProcessingStateByChatIdFlow(
        &self,
    ) -> StateFlow<HashMap<String, InputProcessingState>> {
        self.inputProcessingStateByChatIdFlow.asStateFlow()
    }

    pub fn turnCompleteCounterByChatIdFlow(&self) -> StateFlow<HashMap<String, i64>> {
        self.turnCompleteCounterByChatIdFlow.asStateFlow()
    }

    pub fn currentTurnToolInvocationCountByChatIdFlow(&self) -> StateFlow<HashMap<String, i32>> {
        self.currentTurnToolInvocationCountByChatIdFlow
            .asStateFlow()
    }

    #[allow(non_snake_case)]
    pub fn scrollToBottom(&mut self) {
        self.forceEmitScrollToBottom(None);
    }

    #[allow(non_snake_case)]
    pub fn getTurnCompleteCounter(&self, chatId: String) -> i64 {
        *self
            .turnCompleteCounterByChatIdFlow
            .value()
            .get(&chatId)
            .unwrap_or(&0)
    }

    #[allow(non_snake_case)]
    pub fn isChatLoading(&self, chatId: String) -> bool {
        self.withExistingRuntime(Some(chatId), |runtime| runtime.isLoading)
            .unwrap_or(false)
    }

    #[allow(non_snake_case)]
    pub fn setSpeakMessageHandler(&mut self, handler: fn(String, bool)) {
        self.speakMessageHandler = Some(handler);
    }

    #[allow(non_snake_case)]
    pub fn resetCurrentTurnToolInvocationCount(&mut self, chatId: String) {
        let mut counts = self.currentTurnToolInvocationCountByChatIdFlow.value();
        counts.insert(chatId, 0);
        self.currentTurnToolInvocationCountByChatId = counts.clone();
        self.currentTurnToolInvocationCountByChatIdFlow
            .set_value(counts);
    }

    #[allow(non_snake_case)]
    pub fn incrementCurrentTurnToolInvocationCount(&mut self, chatId: String) {
        let mut counts = self.currentTurnToolInvocationCountByChatIdFlow.value();
        let value = counts.get(&chatId).copied().unwrap_or(0) + 1;
        counts.insert(chatId, value);
        self.currentTurnToolInvocationCountByChatId = counts.clone();
        self.currentTurnToolInvocationCountByChatIdFlow
            .set_value(counts);
    }

    #[allow(non_snake_case)]
    pub fn clearCurrentTurnToolInvocationCount(&mut self, chatId: String) {
        let mut counts = self.currentTurnToolInvocationCountByChatIdFlow.value();
        counts.remove(&chatId);
        self.currentTurnToolInvocationCountByChatId = counts.clone();
        self.currentTurnToolInvocationCountByChatIdFlow
            .set_value(counts);
    }

    #[allow(non_snake_case)]
    pub async fn sendUserMessage(
        &mut self,
        mut request: SendUserMessageProcessingRequest<'_>,
    ) -> Result<
        SendUserMessageProcessingResult,
        crate::api::chat::llmprovider::AIService::AiServiceError,
    > {
        let chatId = request.chatId.clone();
        let originalMessageText = request.messageText.trim().to_string();
        self.resetCurrentTurnToolInvocationCount(chatId.clone());
        self.withRuntime(Some(chatId.clone()), |runtime| {
            runtime.currentTurnOptions = request.turnOptions.clone();
            runtime.requestSentAt = messageTimingNow().startedAtMs as i64;
            runtime.requestStartElapsed = messageTimingNow().startedAtMs as i64;
            runtime.firstResponseElapsed = None;
            runtime.isLoading = true;
            runtime.responseStream = None;
        });
        self.updateGlobalLoadingState();
        self.setInputProcessingStateForChat(
            chatId.clone(),
            InputProcessingState::Processing {
                message: "message_processing".to_string(),
            },
        );

        let finalMessageContent =
            match self.buildUserMessageContentForSend(BuildUserMessageContentForSendRequest {
                messageText: originalMessageText.clone(),
                proxySenderNameOverride: request.proxySenderNameOverride.clone(),
                attachments: request.attachments.clone(),
                workspacePath: request.workspacePath.clone(),
                workspaceEnv: request.workspaceEnv.clone(),
                replyToMessage: request.replyToMessage.clone(),
                chatId: chatId.clone(),
                roleCardId: request.roleCardId.clone(),
                chatModelConfigIdOverride: request.chatModelConfigIdOverride.clone(),
            }) {
                Ok(content) => content,
                Err(error) => {
                    self.withExistingRuntime(Some(chatId.clone()), |runtime| {
                        runtime.isLoading = false;
                        runtime.responseStream = None;
                        runtime.sendJob = None;
                        runtime.streamCollectionJob = None;
                        runtime.stateCollectionJob = None;
                    });
                    self.updateGlobalLoadingState();
                    self.setInputProcessingStateForChat(
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
        if request.turnOptions.persistTurn && isFirstMessage {
            let newTitle = if !originalMessageText.trim().is_empty() {
                originalMessageText.clone()
            } else if let Some(attachment) = request.attachments.first() {
                attachment.fileName.clone()
            } else {
                "New Chat".to_string()
            };
            request
                .chatHistoryDelegate
                .updateChatTitle(chatId.clone(), newTitle);
        }
        let mut userMessageAdded = false;
        let mut userMessage = ChatMessage {
            sender: "user".to_string(),
            content: finalMessageContent.clone(),
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
                        request.workspaceEnv.clone(),
                        userMessage.timestamp,
                        Some(chatId.clone()),
                    );
            workspaceToolHookHandler.addToolHook(session.clone());
            workspaceToolHookSession = Some(session);
        }
        if shouldAddUserMessageToChat {
            request
                .chatHistoryDelegate
                .addMessageToChat(userMessage.clone(), Some(chatId.clone()));
            userMessageAdded = true;
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
        let calculateNextWindowSize = {
            let workspacePath = request.workspacePath.clone();
            let workspaceEnv = request.workspaceEnv.clone();
            let promptFunctionType = request.promptFunctionType.clone();
            let roleCardId = request.roleCardId.clone();
            let currentRoleName = currentRoleName.clone();
            let groupOrchestrationMode = request.isGroupOrchestrationTurn;
            let groupParticipantNamesText = request.groupParticipantNamesText.clone();
            let proxySenderName = request.proxySenderNameOverride.clone();
            let chatModelConfigIdOverride = request.chatModelConfigIdOverride.clone();
            let chatModelIndexOverride = request.chatModelIndexOverride;
            let preferenceProfileIdOverride = request.preferenceProfileIdOverride.clone();
            move |service: &mut EnhancedAIService,
                  chatHistoryDelegate: &ChatHistoryDelegate,
                  chatId: String|
                  -> Option<i32> {
                let runtimeOptions = SendMessageOptions {
                    roleCardId: Some(roleCardId.clone()),
                    promptFunctionType: promptFunctionType.clone(),
                    chatModelConfigIdOverride: chatModelConfigIdOverride.clone(),
                    chatModelIndexOverride,
                    preferenceProfileIdOverride: preferenceProfileIdOverride.clone(),
                    ..SendMessageOptions::new()
                };
                let runtime = service.createSendMessageRuntime(&runtimeOptions).ok()?;
                let calculation = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .ok()?;
                calculation
                    .block_on(AIMessageManager::calculateStableContextWindow(
                        StableContextWindowRequest {
                            enhancedAiService: service,
                            chatId: Some(chatId.clone()),
                            messageContent: String::new(),
                            chatHistory: chatHistoryDelegate.getRuntimeChatHistory(chatId),
                            workspacePath,
                            workspaceEnv,
                            promptFunctionType,
                            roleCardId: Some(roleCardId),
                            currentRoleName: Some(currentRoleName),
                            splitHistoryByRole: true,
                            groupOrchestrationMode,
                            groupParticipantNamesText,
                            proxySenderName,
                            chatModelConfigIdOverride,
                            chatModelIndexOverride,
                            preferenceProfileIdOverride,
                            publishEstimate: true,
                            runtime,
                        },
                    ))
                    .ok()
            }
        };

        let completionStream = match AIMessageManager::sendMessage(AIMessageSendRequest {
            enhancedAiService: request.enhancedAiService,
            chatId: Some(chatId.clone()),
            messageContent: requestMessageContent,
            chatHistory: request.chatHistory,
            workspacePath: request.workspacePath.clone(),
            workspaceEnv: request.workspaceEnv.clone(),
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
            chatModelConfigIdOverride: request.chatModelConfigIdOverride.clone(),
            chatModelIndexOverride: request.chatModelIndexOverride,
            preferenceProfileIdOverride: request.preferenceProfileIdOverride.clone(),
            disableWarning: request.turnOptions.disableWarning,
            callbacks: Some(Arc::new(MessageProcessingCallbacks {
                nonFatalErrorEventFlow: self.nonFatalErrorEventFlow.clone(),
            })),
            onToolInvocation: None,
        })
        .await
        {
            Ok(stream) => stream,
            Err(error) => {
                if let Some(session) = workspaceToolHookSession.as_ref() {
                    workspaceToolHookHandler.removeToolHook(session.hookId());
                    session.close();
                }
                self.setInputProcessingStateForChat(
                    chatId.clone(),
                    InputProcessingState::Error {
                        message: error.to_string(),
                    },
                );
                self.withExistingRuntime(Some(chatId.clone()), |runtime| {
                    runtime.isLoading = false;
                    runtime.responseStream = None;
                    runtime.sendJob = None;
                    runtime.streamCollectionJob = None;
                    runtime.stateCollectionJob = None;
                });
                self.updateGlobalLoadingState();
                return Err(error);
            }
        };
        let sharedResponseStream = completionStream.clone();
        self.withRuntime(Some(chatId.clone()), |runtime| {
            runtime.responseStream = Some(sharedResponseStream.clone());
        });
        let initialProviderModel = request
            .enhancedAiService
            .getLastProviderModel()
            .unwrap_or_default();
        let (initialProvider, initialModelName) = split_provider_model(&initialProviderModel);
        let mut aiMessage = ChatMessage {
            sender: "ai".to_string(),
            content: String::new(),
            timestamp: ChatMessageTimestampAllocator::next(),
            roleName: currentRoleName.clone(),
            provider: initialProvider,
            modelName: initialModelName,
            inputTokens: 0,
            outputTokens: 0,
            cachedInputTokens: 0,
            displayMode: ChatMessageDisplayMode::NORMAL,
            contentStream: Some(completionStream.clone()),
            ..ChatMessage::new("ai".to_string())
        };
        let workerChatId = chatId.clone();
        let workerTurnOptions = request.turnOptions.clone();
        let mut workerAiMessage = aiMessage.clone();
        let mut workerResponseStream = sharedResponseStream.clone();
        let workerEventCollector = sharedResponseStream.event_channel().clone();
        let workerRevisionTracker = Arc::new(Mutex::new(TextStreamRevisionTracker::new("")));
        let workerEventTracker = workerRevisionTracker.clone();
        let mut workerService = request.enhancedAiService.clone();
        let mut workerChatHistoryDelegate = request.chatHistoryDelegate.clone_for_core();
        let mut workerMessageProcessingDelegate = self.clone_for_core();
        let workerCalculateNextWindowSize = calculateNextWindowSize;
        let (workerRequestSentAt, workerRequestStartElapsed) = self
            .withRuntime(Some(chatId.clone()), |runtime| {
                (runtime.requestSentAt, runtime.requestStartElapsed)
            });
        let workerWorkspaceToolHookSession = workspaceToolHookSession.clone();
        let mut workerWorkspaceToolHookHandler = workspaceToolHookHandler.clone();
        if userMessageAdded {
            userMessage.sentAt = workerRequestSentAt;
            request
                .chatHistoryDelegate
                .addMessageToChat(userMessage, Some(chatId.clone()));
        }
        if workerTurnOptions.persistTurn {
            request
                .chatHistoryDelegate
                .addMessageToChat(aiMessage.clone(), Some(chatId.clone()));
        }
        std::thread::spawn(move || {
            let eventWorker = std::thread::spawn(move || {
                let mut events = workerEventCollector;
                events.collect(&mut |event| match event.event_type {
                    TextStreamEventType::Savepoint => {
                        if let Ok(mut tracker) = workerEventTracker.lock() {
                            tracker.savepoint(&event.id);
                        }
                    }
                    TextStreamEventType::Rollback => {
                        if let Ok(mut tracker) = workerEventTracker.lock() {
                            let _ = tracker.rollback(&event.id);
                        }
                    }
                });
            });
            let mut firstResponseElapsed = None::<i64>;
            workerResponseStream.collect(&mut |chunk| {
                if firstResponseElapsed.is_none() {
                    firstResponseElapsed = Some(messageTimingNow().startedAtMs as i64);
                }
                let content = if let Ok(mut tracker) = workerRevisionTracker.lock() {
                    tracker.append(&chunk)
                } else {
                    workerAiMessage.content.clone()
                };
                workerAiMessage.content = content;
            });
            if let Some(session) = workerWorkspaceToolHookSession.as_ref() {
                workerWorkspaceToolHookHandler.removeToolHook(session.hookId());
                session.close();
            }
            let _ = eventWorker.join();
            let finalContent = workerRevisionTracker
                .lock()
                .map(|tracker| tracker.current_content())
                .unwrap_or_else(|_| workerAiMessage.content.clone());
            let providerModel = workerService.getLastProviderModel().unwrap_or_default();
            let (provider, modelName) = split_provider_model(&providerModel);
            let tokenSnapshot = workerService.getLastTurnTokenSnapshot().unwrap_or(
                crate::api::chat::EnhancedAIService::TurnTokenSnapshot {
                    inputTokens: 0,
                    outputTokens: 0,
                    cachedInputTokens: 0,
                },
            );
            let completedElapsed = messageTimingNow().startedAtMs as i64;
            workerAiMessage.provider = provider;
            workerAiMessage.modelName = modelName;
            workerAiMessage.inputTokens = tokenSnapshot.inputTokens;
            workerAiMessage.outputTokens = tokenSnapshot.outputTokens;
            workerAiMessage.cachedInputTokens = tokenSnapshot.cachedInputTokens;
            workerAiMessage.content = finalContent;
            workerAiMessage.contentStream = None;
            let finalMessage = MessageProcessingDelegate::withTurnMetrics(
                ChatMessage {
                    completedAt: completedElapsed,
                    ..workerAiMessage
                },
                workerRequestSentAt,
                workerRequestStartElapsed,
                firstResponseElapsed,
                completedElapsed,
            );
            if workerTurnOptions.persistTurn {
                workerChatHistoryDelegate
                    .addMessageToChat(finalMessage.clone(), Some(workerChatId.clone()));
            }
            let nextWindowSize = workerCalculateNextWindowSize(
                &mut workerService,
                &workerChatHistoryDelegate,
                workerChatId.clone(),
            );
            if let Some(windowSize) = nextWindowSize {
                let previousTokens = workerChatHistoryDelegate
                    .chatHistoriesFlow()
                    .value()
                    .into_iter()
                    .find(|history| history.id == workerChatId)
                    .map(|history| (history.inputTokens, history.outputTokens));
                let (inputTokens, outputTokens) = match previousTokens {
                    Some((inputTokens, outputTokens)) => (
                        inputTokens + workerAiMessage.inputTokens,
                        outputTokens + workerAiMessage.outputTokens,
                    ),
                    None => (workerAiMessage.inputTokens, workerAiMessage.outputTokens),
                };
                workerChatHistoryDelegate.saveCurrentChat(
                    inputTokens,
                    outputTokens,
                    windowSize,
                    Some(workerChatId.clone()),
                );
            }
            workerMessageProcessingDelegate.finalizeMessageAndNotify(
                workerChatId,
                finalMessage,
                nextWindowSize,
                workerTurnOptions,
            );
        });
        Ok(SendUserMessageProcessingResult {
            aiMessage,
            nextWindowSize: None,
        })
    }

    #[allow(non_snake_case)]
    pub async fn regenerateAiMessageVariant(
        &mut self,
        request: RegenerateAiMessageVariantRequest<'_>,
    ) -> Result<ChatMessage, crate::api::chat::llmprovider::AIService::AiServiceError> {
        let targetMessageTimestamp = request.targetMessageTimestamp;
        let result = self
            .sendUserMessage(SendUserMessageProcessingRequest {
                enhancedAiService: request.enhancedAiService,
                chatHistoryDelegate: request.chatHistoryDelegate,
                chatId: request.chatId,
                messageText: request.requestMessageContent,
                chatHistory: request.requestHistory,
                workspacePath: request.workspacePath,
                workspaceEnv: None,
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
                chatModelConfigIdOverride: request.chatModelConfigIdOverride,
                chatModelIndexOverride: request.chatModelIndexOverride,
                preferenceProfileIdOverride: request.preferenceProfileIdOverride,
                isGroupOrchestrationTurn: false,
                groupParticipantNamesText: None,
                proxySenderNameOverride: None,
                suppressUserMessageInHistory: true,
                isAutoContinuation: false,
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

    #[allow(non_snake_case)]
    pub fn notifyTurnComplete(
        &mut self,
        chatId: Option<String>,
        _service: &EnhancedAIService,
        _nextWindowSize: Option<i32>,
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

    #[allow(non_snake_case)]
    pub fn finalizeMessageAndNotify(
        &mut self,
        chatId: String,
        _aiMessage: ChatMessage,
        nextWindowSize: Option<i32>,
        turnOptions: ChatTurnOptions,
    ) {
        self.cleanupRuntimeAfterSend(chatId.clone(), turnOptions);
        self.setInputProcessingStateForChat(chatId.clone(), InputProcessingState::Completed);
        let mut counters = self.turnCompleteCounterByChatIdFlow.value();
        let next = counters.get(&chatId).copied().unwrap_or(0) + 1;
        counters.insert(chatId.clone(), next);
        self.turnCompleteCounterByChatId = counters.clone();
        self.turnCompleteCounterByChatIdFlow.set_value(counters);
        let _ = nextWindowSize;
    }

    #[allow(non_snake_case)]
    pub fn cleanupRuntimeAfterSend(&mut self, chatId: String, _turnOptions: ChatTurnOptions) {
        self.withExistingRuntime(Some(chatId.clone()), |runtime| {
            runtime.isLoading = false;
            runtime.sendJob = None;
            runtime.streamCollectionJob = None;
            runtime.stateCollectionJob = None;
        });
        self.clearCurrentTurnToolInvocationCount(chatId);
        self.updateGlobalLoadingState();
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

fn split_provider_model(providerModel: &str) -> (String, String) {
    let Some(index) = providerModel.find(':') else {
        return (providerModel.to_string(), String::new());
    };
    (
        providerModel[..index].to_string(),
        providerModel[index + 1..].to_string(),
    )
}
