use std::collections::{HashMap, HashSet};

use crate::api::chat::EnhancedAIService::{EnhancedAIService, SendMessageExecution};
use crate::core::chat::AIMessageManager::{
    logMessageTiming, messageTimingNow, AIMessageManager, BuildUserMessageContentRequest,
    SendMessageRequest as AIMessageSendRequest,
};
use crate::data::model::AttachmentInfo::AttachmentInfo;
use crate::data::model::ChatMessage::ChatMessage;
use crate::data::model::ChatMessageDisplayMode::ChatMessageDisplayMode;
use crate::data::model::ChatMessageTimestampAllocator::ChatMessageTimestampAllocator;
use crate::data::model::ChatTurnOptions::ChatTurnOptions;
use crate::data::model::FunctionType::FunctionType;
use crate::data::model::InputProcessingState::InputProcessingState;
use crate::data::model::PromptFunctionType::PromptFunctionType;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;

pub const STREAM_SCROLL_THROTTLE_MS: i64 = 200;
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
pub struct SharedStream<T> {
    pub values: Vec<T>,
}

impl<T> SharedStream<T> {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }
}

#[derive(Clone, Debug)]
pub struct ChatRuntime {
    pub sendJob: Option<String>,
    pub responseStream: Option<SharedStream<String>>,
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
    pub chatId: String,
    pub messageContent: String,
    pub chatHistory: Vec<ChatMessage>,
    pub workspacePath: Option<String>,
    pub workspaceEnv: Option<String>,
    pub promptFunctionType: PromptFunctionType,
    pub roleCardId: String,
    pub currentRoleName: Option<String>,
    pub characterName: Option<String>,
    pub avatarUri: Option<String>,
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
    pub turnOptions: ChatTurnOptions,
}

#[derive(Clone, Debug)]
pub struct SendUserMessageProcessingResult {
    pub aiMessage: ChatMessage,
    pub execution: SendMessageExecution,
    pub nextWindowSize: Option<i32>,
}

pub struct RegenerateAiMessageVariantRequest<'a> {
    pub enhancedAiService: &'a mut EnhancedAIService,
    pub chatId: String,
    pub targetMessageTimestamp: i64,
    pub requestMessageContent: String,
    pub requestHistory: Vec<ChatMessage>,
    pub workspacePath: Option<String>,
    pub promptFunctionType: PromptFunctionType,
    pub roleCardId: String,
    pub currentRoleName: String,
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
    pub isLoading: bool,
    pub activeStreamingChatIds: HashSet<String>,
    pub inputProcessingStateByChatId: HashMap<String, InputProcessingState>,
    pub scrollToBottomEvent: Vec<()>,
    pub nonFatalErrorEvent: Vec<String>,
    pub turnCompleteCounterByChatId: HashMap<String, i64>,
    pub currentTurnToolInvocationCountByChatId: HashMap<String, i32>,
    pub chatRuntimes: HashMap<String, ChatRuntime>,
    pub lastScrollEmitMsByChatKey: HashMap<String, i64>,
    pub suppressIdleCompletedStateByChatId: HashMap<String, bool>,
    pub pendingAsyncSummaryUiByChatId: HashMap<String, bool>,
    pub speakMessageHandler: Option<fn(String, bool)>,
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
            isLoading: false,
            activeStreamingChatIds: HashSet::new(),
            inputProcessingStateByChatId: HashMap::new(),
            scrollToBottomEvent: Vec::new(),
            nonFatalErrorEvent: Vec::new(),
            turnCompleteCounterByChatId: HashMap::new(),
            currentTurnToolInvocationCountByChatId: HashMap::new(),
            chatRuntimes: HashMap::new(),
            lastScrollEmitMsByChatKey: HashMap::new(),
            suppressIdleCompletedStateByChatId: HashMap::new(),
            pendingAsyncSummaryUiByChatId: HashMap::new(),
            speakMessageHandler: None,
        }
    }

    #[allow(non_snake_case)]
    pub fn clone_for_core(&self) -> Self {
        let rootDir = ApiPreferences::data_dir();
        Self {
            functionalConfigManager: FunctionalConfigManager::new(rootDir.clone()),
            modelConfigManager: ModelConfigManager::new(rootDir),
            userMessage: self.userMessage.clone(),
            isLoading: self.isLoading,
            activeStreamingChatIds: self.activeStreamingChatIds.clone(),
            inputProcessingStateByChatId: self.inputProcessingStateByChatId.clone(),
            scrollToBottomEvent: self.scrollToBottomEvent.clone(),
            nonFatalErrorEvent: self.nonFatalErrorEvent.clone(),
            turnCompleteCounterByChatId: self.turnCompleteCounterByChatId.clone(),
            currentTurnToolInvocationCountByChatId: self.currentTurnToolInvocationCountByChatId.clone(),
            chatRuntimes: self.chatRuntimes.clone(),
            lastScrollEmitMsByChatKey: self.lastScrollEmitMsByChatKey.clone(),
            suppressIdleCompletedStateByChatId: self.suppressIdleCompletedStateByChatId.clone(),
            pendingAsyncSummaryUiByChatId: self.pendingAsyncSummaryUiByChatId.clone(),
            speakMessageHandler: self.speakMessageHandler,
        }
    }

    #[allow(non_snake_case)]
    pub fn speechPreview(text: String) -> String {
        text.replace('\n', "\\n").chars().take(AUTO_READ_PREVIEW_MAX).collect()
    }

    #[allow(non_snake_case)]
    pub fn chatKey(chatId: Option<String>) -> String {
        chatId.unwrap_or_else(|| "__DEFAULT_CHAT__".to_string())
    }

    #[allow(non_snake_case)]
    pub fn tryEmitScrollToBottomThrottled(&mut self, chatId: Option<String>) {
        let key = Self::chatKey(chatId);
        self.lastScrollEmitMsByChatKey.insert(key, messageTimingNow().startedAtMs as i64);
        self.scrollToBottomEvent.push(());
    }

    #[allow(non_snake_case)]
    pub fn forceEmitScrollToBottom(&mut self, chatId: Option<String>) {
        let key = Self::chatKey(chatId);
        self.lastScrollEmitMsByChatKey.insert(key, messageTimingNow().startedAtMs as i64);
        self.scrollToBottomEvent.push(());
    }

    #[allow(non_snake_case)]
    pub fn runtimeFor(&mut self, chatId: Option<String>) -> &mut ChatRuntime {
        let key = Self::chatKey(chatId);
        self.chatRuntimes.entry(key).or_insert_with(ChatRuntime::new)
    }

    #[allow(non_snake_case)]
    pub fn updateGlobalLoadingState(&mut self) {
        self.isLoading = self.chatRuntimes.values().any(|runtime| runtime.isLoading);
        self.activeStreamingChatIds = self
            .chatRuntimes
            .iter()
            .filter(|(key, runtime)| key.as_str() != "__DEFAULT_CHAT__" && runtime.isLoading)
            .map(|(key, _)| key.clone())
            .collect();
    }

    #[allow(non_snake_case)]
    pub fn refreshGlobalLoadingState(&mut self) {
        self.updateGlobalLoadingState();
    }

    #[allow(non_snake_case)]
    pub fn isTerminalInputState(state: &InputProcessingState) -> bool {
        matches!(state, InputProcessingState::Idle | InputProcessingState::Completed)
    }

    #[allow(non_snake_case)]
    pub fn setChatInputProcessingState(&mut self, chatId: Option<String>, state: InputProcessingState) {
        let key = Self::chatKey(chatId);
        self.inputProcessingStateByChatId.insert(key, state);
    }

    #[allow(non_snake_case)]
    pub fn setSuppressIdleCompletedStateForChat(&mut self, chatId: String, suppress: bool) {
        if suppress {
            self.suppressIdleCompletedStateByChatId.insert(chatId, true);
        } else {
            self.suppressIdleCompletedStateByChatId.remove(&chatId);
        }
    }

    #[allow(non_snake_case)]
    pub fn setPendingAsyncSummaryUiForChat(&mut self, chatId: String, pending: bool) {
        if pending {
            self.pendingAsyncSummaryUiByChatId.insert(chatId, true);
        } else {
            self.pendingAsyncSummaryUiByChatId.remove(&chatId);
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
                .map_err(|error| crate::api::chat::llmprovider::AIService::AiServiceError::RequestFailed(error.to_string()))?,
        };

        let loadModelConfigStartTime = messageTimingNow();
        let currentModelConfig = self
            .modelConfigManager
            .getModelConfigFlow(&configId)
            .map_err(|error| crate::api::chat::llmprovider::AIService::AiServiceError::RequestFailed(error.to_string()))?;
        let enableDirectImageProcessing = currentModelConfig.enableDirectImageProcessing;
        let enableDirectAudioProcessing = currentModelConfig.enableDirectAudioProcessing;
        let enableDirectVideoProcessing = currentModelConfig.enableDirectVideoProcessing;
        logMessageTiming(
            "delegate.loadModelConfig",
            loadModelConfigStartTime,
            Some(format!("chatId={}, configId={configId}", request.chatId)),
        );

        let buildUserMessageStartTime = messageTimingNow();
        let finalMessageContent = AIMessageManager::buildUserMessageContent(
            BuildUserMessageContentRequest {
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
            },
        );
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
    pub fn getResponseStream(&self, chatId: String) -> Option<SharedStream<String>> {
        self.chatRuntimes
            .get(&Self::chatKey(Some(chatId)))
            .and_then(|runtime| runtime.responseStream.clone())
    }

    #[allow(non_snake_case)]
    pub fn resolveFinalContent(aiMessage: ChatMessage) -> String {
        aiMessage.content
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
        aiMessage.waitDurationMs = firstResponseElapsed.map(|first| first - requestStartElapsed).unwrap_or(0);
        aiMessage.outputDurationMs = firstResponseElapsed.map(|first| completedElapsed - first).unwrap_or(0);
        aiMessage.completedAt = completedElapsed;
        aiMessage
    }

    #[allow(non_snake_case)]
    pub fn readCurrentTurnCancellationSnapshot(&self, chatId: String) -> Option<TurnCancellationSnapshot> {
        self.chatRuntimes.get(&Self::chatKey(Some(chatId.clone()))).map(|runtime| {
            TurnCancellationSnapshot {
                chatId,
                aiMessage: None,
                partialContent: runtime
                    .responseStream
                    .as_ref()
                    .map(|stream| stream.values.join(""))
                    .unwrap_or_default(),
                turnOptions: runtime.currentTurnOptions.clone(),
            }
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
        if let Some(runtime) = self.chatRuntimes.get_mut(&Self::chatKey(Some(chatId.clone()))) {
            runtime.isLoading = false;
            runtime.sendJob = None;
            runtime.streamCollectionJob = None;
            runtime.stateCollectionJob = None;
        }
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
        self.userMessage = TextFieldValue::new(message);
    }

    #[allow(non_snake_case)]
    pub fn updateUserMessageValue(&mut self, value: TextFieldValue) {
        self.userMessage = value;
    }

    #[allow(non_snake_case)]
    pub fn scrollToBottom(&mut self) {
        self.forceEmitScrollToBottom(None);
    }

    #[allow(non_snake_case)]
    pub fn getTurnCompleteCounter(&self, chatId: String) -> i64 {
        *self.turnCompleteCounterByChatId.get(&chatId).unwrap_or(&0)
    }

    #[allow(non_snake_case)]
    pub fn isChatLoading(&self, chatId: String) -> bool {
        self.chatRuntimes
            .get(&Self::chatKey(Some(chatId)))
            .map(|runtime| runtime.isLoading)
            .unwrap_or(false)
    }

    #[allow(non_snake_case)]
    pub fn setSpeakMessageHandler(&mut self, handler: fn(String, bool)) {
        self.speakMessageHandler = Some(handler);
    }

    #[allow(non_snake_case)]
    pub fn resetCurrentTurnToolInvocationCount(&mut self, chatId: String) {
        self.currentTurnToolInvocationCountByChatId.insert(chatId, 0);
    }

    #[allow(non_snake_case)]
    pub fn incrementCurrentTurnToolInvocationCount(&mut self, chatId: String) {
        let value = self.currentTurnToolInvocationCountByChatId.get(&chatId).copied().unwrap_or(0) + 1;
        self.currentTurnToolInvocationCountByChatId.insert(chatId, value);
    }

    #[allow(non_snake_case)]
    pub fn clearCurrentTurnToolInvocationCount(&mut self, chatId: String) {
        self.currentTurnToolInvocationCountByChatId.remove(&chatId);
    }

    #[allow(non_snake_case)]
    pub async fn sendUserMessage(
        &mut self,
        request: SendUserMessageProcessingRequest<'_>,
    ) -> Result<SendUserMessageProcessingResult, crate::api::chat::llmprovider::AIService::AiServiceError> {
        let chatId = request.chatId.clone();
        self.resetCurrentTurnToolInvocationCount(chatId.clone());
        {
            let runtime = self.runtimeFor(Some(chatId.clone()));
            runtime.currentTurnOptions = request.turnOptions.clone();
            runtime.requestSentAt = messageTimingNow().startedAtMs as i64;
            runtime.requestStartElapsed = messageTimingNow().startedAtMs as i64;
            runtime.firstResponseElapsed = None;
            runtime.isLoading = true;
            runtime.responseStream = Some(SharedStream::new());
        }
        self.setInputProcessingStateForChat(
            chatId.clone(),
            InputProcessingState::Connecting {
                message: String::new(),
            },
        );
        self.updateGlobalLoadingState();

        let execution = AIMessageManager::sendMessage(AIMessageSendRequest {
            enhancedAiService: request.enhancedAiService,
            chatId: Some(chatId.clone()),
            messageContent: request.messageContent,
            chatHistory: request.chatHistory,
            workspacePath: request.workspacePath,
            workspaceEnv: request.workspaceEnv,
            promptFunctionType: request.promptFunctionType,
            enableThinking: request.enableThinking,
            enableMemoryAutoUpdate: request.enableMemoryAutoUpdate,
            maxTokens: request.maxTokens,
            tokenUsageThreshold: request.tokenUsageThreshold,
            characterName: request.characterName.clone(),
            avatarUri: request.avatarUri,
            roleCardId: request.roleCardId.clone(),
            currentRoleName: request.currentRoleName,
            splitHistoryByRole: true,
            groupOrchestrationMode: request.isGroupOrchestrationTurn,
            groupParticipantNamesText: request.groupParticipantNamesText,
            proxySenderName: request.proxySenderNameOverride,
            notifyReplyOverride: request.turnOptions.notifyReply,
            chatModelConfigIdOverride: request.chatModelConfigIdOverride,
            chatModelIndexOverride: request.chatModelIndexOverride,
            preferenceProfileIdOverride: request.preferenceProfileIdOverride,
            disableWarning: request.turnOptions.disableWarning,
        })
        .await?;

        let (provider, modelName) = split_provider_model(&execution.providerModel);
        let completedElapsed = messageTimingNow().startedAtMs as i64;
        let runtime = self.runtimeFor(Some(chatId.clone())).clone();
        let aiMessage = Self::withTurnMetrics(
            ChatMessage {
                sender: "ai".to_string(),
                content: execution.responseChunks.join(""),
                timestamp: ChatMessageTimestampAllocator::next(),
                roleName: request.characterName.clone().unwrap_or_else(|| "AI".to_string()),
                provider,
                modelName,
                inputTokens: execution.tokenSnapshot.inputTokens,
                outputTokens: execution.tokenSnapshot.outputTokens,
                cachedInputTokens: execution.tokenSnapshot.cachedInputTokens,
                displayMode: ChatMessageDisplayMode::NORMAL,
                ..ChatMessage::new("ai".to_string())
            },
            runtime.requestSentAt,
            runtime.requestStartElapsed,
            runtime.firstResponseElapsed,
            completedElapsed,
        );

        self.finalizeMessageAndNotify(chatId.clone(), aiMessage.clone(), request.turnOptions.clone());
        Ok(SendUserMessageProcessingResult {
            aiMessage,
            execution,
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
                chatId: request.chatId,
                messageContent: request.requestMessageContent,
                chatHistory: request.requestHistory,
                workspacePath: request.workspacePath,
                workspaceEnv: None,
                promptFunctionType: request.promptFunctionType,
                roleCardId: request.roleCardId,
                currentRoleName: Some(request.currentRoleName),
                characterName: None,
                avatarUri: None,
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
                turnOptions: ChatTurnOptions::default(),
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
            let next = self.turnCompleteCounterByChatId.get(&chatId).copied().unwrap_or(0) + 1;
            self.turnCompleteCounterByChatId.insert(chatId, next);
        }
    }

    #[allow(non_snake_case)]
    pub fn finalizeMessageAndNotify(
        &mut self,
        chatId: String,
        _aiMessage: ChatMessage,
        turnOptions: ChatTurnOptions,
    ) {
        self.setInputProcessingStateForChat(chatId.clone(), InputProcessingState::Completed);
        let next = self.turnCompleteCounterByChatId.get(&chatId).copied().unwrap_or(0) + 1;
        self.turnCompleteCounterByChatId.insert(chatId.clone(), next);
        self.cleanupRuntimeAfterSend(chatId, turnOptions);
    }

    #[allow(non_snake_case)]
    pub fn cleanupRuntimeAfterSend(&mut self, chatId: String, _turnOptions: ChatTurnOptions) {
        if let Some(runtime) = self.chatRuntimes.get_mut(&Self::chatKey(Some(chatId.clone()))) {
            runtime.isLoading = false;
            runtime.sendJob = None;
            runtime.streamCollectionJob = None;
            runtime.stateCollectionJob = None;
        }
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
