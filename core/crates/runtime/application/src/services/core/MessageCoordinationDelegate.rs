use operit_util::stream::Stream::Stream;
use operit_store::ChatExecutionLease::ChatExecutionLease;
use std::collections::HashMap;

use crate::core::chat::AIMessageManager::{AIMessageManager, StableContextWindowRequest};
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::services::core::ChatHistoryDelegate::ChatHistoryDelegate;
use crate::services::core::MessageProcessingDelegate::{
    MessageProcessingDelegate, RegenerateAiMessageVariantRequest, SendUserMessageProcessingRequest,
};
use crate::services::core::TokenStatisticsDelegate::TokenStatisticsDelegate;
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatMessageDisplayMode::ChatMessageDisplayMode;
use operit_model::ChatTurnOptions::ChatTurnOptions;
use crate::services::core::MessageProcessingDelegate::ChatTurnSubmission;
use operit_model::FunctionType::FunctionType;
use operit_model::InputProcessingState::InputProcessingState;
use operit_model::MessagePart::MessagePart;
use operit_model::MessagePartCodec::MessagePartCodec;
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_providers::chat::EnhancedAIService::{EnhancedAIService, SendMessageOptions};
use operit_providers::runtime_support::ChatConfigurationResult;
use operit_util::AppLogger::AppLogger;
use operit_util::ChainLogger::{self, SEND_CHAIN};

/// Queued continuation work scheduled after a summary completes.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingAutoContinuationRequest {
    pub chatId: String,
    pub promptFunctionType: PromptFunctionType,
    pub chatProviderIdOverride: Option<String>,
    pub chatModelIdOverride: Option<String>,
    pub participantId: Option<String>,
    pub waitJob: Option<String>,
}

/// Coordinates high-level chat sends, generic participant execution and conversation summaries.
pub struct MessageCoordinationDelegate {
    pub chatHistoryDelegate: ChatHistoryDelegate,
    pub messageProcessingDelegate: MessageProcessingDelegate,
    pub tokenStatisticsDelegate: TokenStatisticsDelegate,
    pub isSummarizing: bool,
    pub isUpdatingMemory: bool,
    pub summarizingChatId: Option<String>,
    pub isSendTriggeredSummarizing: bool,
    pub sendTriggeredSummarizingChatId: Option<String>,
    pub summaryJob: Option<String>,
    pub sendTriggeredSummaryJob: Option<String>,
    pub currentPromptFunctionType: PromptFunctionType,
    pub currentChatProviderIdOverride: Option<String>,
    pub currentChatModelIdOverride: Option<String>,
    pub nonFatalErrorCollectorJob: Option<String>,
    pub pendingAutoContinuationByChatId: HashMap<String, PendingAutoContinuationRequest>,
}

impl MessageCoordinationDelegate {
    /// Creates a coordinator from history and message-processing delegates.
    pub fn new(
        chatHistoryDelegate: ChatHistoryDelegate,
        messageProcessingDelegate: MessageProcessingDelegate,
    ) -> Self {
        let mut delegate = Self {
            chatHistoryDelegate,
            messageProcessingDelegate,
            tokenStatisticsDelegate: TokenStatisticsDelegate::default(),
            isSummarizing: false,
            isUpdatingMemory: false,
            summarizingChatId: None,
            isSendTriggeredSummarizing: false,
            sendTriggeredSummarizingChatId: None,
            summaryJob: None,
            sendTriggeredSummaryJob: None,
            currentPromptFunctionType: PromptFunctionType::CHAT,
            currentChatProviderIdOverride: None,
            currentChatModelIdOverride: None,
            nonFatalErrorCollectorJob: None,
            pendingAutoContinuationByChatId: HashMap::new(),
        };
        delegate.ensureNonFatalErrorCollectorStarted();
        delegate
    }

    /// Subscribes non-fatal processing errors into toast notifications.
    fn ensureNonFatalErrorCollectorStarted(&mut self) {
        if self.nonFatalErrorCollectorJob.is_some() {
            return;
        }
        let nonFatalErrorEventFlow = self.messageProcessingDelegate.nonFatalErrorEventFlow();
        let toastEventFlow = self.messageProcessingDelegate.toastEventFlow.clone();
        nonFatalErrorEventFlow.subscribe(move |errorMessage| {
            if let Some(errorMessage) = errorMessage {
                toastEventFlow.set_value(Some(errorMessage));
            }
        });
        self.nonFatalErrorCollectorJob = Some("nonFatalErrorCollectorJob".to_string());
    }

    /// Recalculates the stable context window size for a chat and prompt mode.
    pub async fn recalculateStableWindowSize(
        &mut self,
        service: &mut EnhancedAIService,
        chatId: Option<String>,
        participantId: Option<String>,
        promptFunctionType: PromptFunctionType,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>) -> Result<i64, String> {
        let currentChat = chatId.as_ref().and_then(|chatId| {
            self.chatHistoryDelegate
                .chatHistoriesFlow()
                .value()
                .into_iter()
                .find(|history| history.id == *chatId)
        });
        let runtimeOptions = SendMessageOptions {
            chatId: chatId.clone(),
            executionParticipantId: participantId.clone(),
            promptFunctionType: promptFunctionType.clone(),
            chatProviderIdOverride: chatProviderIdOverride.clone(),
            chatModelIdOverride: chatModelIdOverride.clone(),
            ..SendMessageOptions::new()
        };
        let configuration = service
            .resolveChatConfigurationForOptions(&runtimeOptions)
            .await
            .map_err(|error| error.to_string())?;
        let runtime = service
            .createSendMessageRuntime(&runtimeOptions, configuration)
            .map_err(|error| error.to_string())?;
        AIMessageManager::calculateStableContextWindow(StableContextWindowRequest {
            enhancedAiService: service,
            chatId: chatId.clone(),
            messageContent: String::new(),
            chatHistory: chatId
                .map(|id| self.chatHistoryDelegate.getRuntimeChatHistory(id))
                .unwrap_or_default(),
            workspacePath: currentChat
                .as_ref()
                .and_then(|chat| self.chatHistoryDelegate.primaryWorkspacePathForChat(chat)),
            workspaceFolders: currentChat
                .iter()
                .flat_map(|chat| self.chatHistoryDelegate.workspaceFolderPathsForChat(chat))
                .collect(),
            promptFunctionType,
            proxySenderName: None,
            chatProviderIdOverride,
            chatModelIdOverride,
            publishEstimate: false,
            runtime,
        })
        .await
        .map_err(|error| error.to_string())
    }

    /// Recalculates and persists the stable context window for a chat.
    pub async fn refreshStableContextWindow(
        &mut self,
        service: &mut EnhancedAIService,
        chatId: Option<String>,
        participantId: Option<String>,
        promptFunctionType: Option<PromptFunctionType>,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>) -> Option<i64> {
        let targetChatId = chatId.or_else(|| self.chatHistoryDelegate.currentChatIdFlow.value())?;
        let executionParticipantId = participantId;
        let effectivePromptFunctionType =
            promptFunctionType.unwrap_or_else(|| self.currentPromptFunctionType.clone());
        let effectiveChatModelIdOverride =
            chatModelIdOverride.or_else(|| self.currentChatModelIdOverride.clone());
        let effectiveChatProviderIdOverride =
            chatProviderIdOverride.or_else(|| self.currentChatProviderIdOverride.clone());
        let newWindowSize = match self
            .recalculateStableWindowSize(
                service,
                Some(targetChatId.clone()),
                executionParticipantId,
                effectivePromptFunctionType,
                effectiveChatProviderIdOverride,
                effectiveChatModelIdOverride)
            .await
        {
            Ok(newWindowSize) => newWindowSize,
            Err(error) => {
                ChainLogger::error(
                    SEND_CHAIN,
                    "stable_window.recalculate.error",
                    &[("chatId", targetChatId.clone()), ("error", error.clone())],
                );
                self.messageProcessingDelegate
                    .setInputProcessingStateForChat(
                        targetChatId.clone(),
                        InputProcessingState::Error { message: error },
                    );
                return None;
            }
        };
        let (inputTokens, outputTokens) = self
            .tokenStatisticsDelegate
            .getCumulativeTokenCounts(Some(targetChatId.clone()));
        self.chatHistoryDelegate.saveCurrentChat(
            inputTokens,
            outputTokens,
            newWindowSize,
            Some(targetChatId.clone()),
        );
        self.tokenStatisticsDelegate.setTokenCounts(
            Some(targetChatId.clone()),
            inputTokens,
            outputTokens,
            newWindowSize,
        );
        Some(newWindowSize)
    }

    /// Public entry point for sending a user message from the active chat UI.
    pub async fn sendUserMessage(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        promptFunctionType: PromptFunctionType,
        participantId: Option<String>,
        chatIdOverride: Option<String>,
        messageText: String,
        proxySenderNameOverride: Option<String>,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
        attachments: Vec<AttachmentInfo>,
        replyToMessage: Option<ChatMessage>,
        turnOptions: ChatTurnOptions,
        admittedLease: Option<ChatExecutionLease>,
    ) -> Result<ChatTurnSubmission, String> {
        AppLogger::i(
            "CoreSend",
            &format!(
                "dispatch entry messageChars={} attachments={} prompt={:?}",
                messageText.chars().count(),
                attachments.len(),
                promptFunctionType
            ),
        );
        let result = self.sendMessageInternal(
            enhancedAiService,
            promptFunctionType,
            turnOptions.continuation.is_some(),
            false,
            false,
            participantId,
            chatIdOverride,
            messageText,
            proxySenderNameOverride,
            chatProviderIdOverride,
            chatModelIdOverride,
            attachments,
            replyToMessage,
            false,
            None,
            turnOptions,
            admittedLease)
        .await;
        AppLogger::i("CoreSend", "dispatch return");
        result
    }

    /// Regenerates a single AI message variant using the surrounding conversation state.
    pub async fn regenerateSingleAiMessage(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        chatId: String,
        messageTimestamp: i64,
    ) -> Result<(), String> {
        if self.messageProcessingDelegate.isChatLoading(chatId.clone()) {
            return Err("Chat is busy".to_string());
        }
        let targetMessage = self
            .chatHistoryDelegate
            .chatMessagesSnapshotForChat(chatId.clone())
            .into_iter()
            .find(|message| message.timestamp == messageTimestamp)
            .ok_or_else(|| format!("Message timestamp not found: {messageTimestamp}"))?;
        if targetMessage.sender != "ai" {
            return Err("Only AI message allowed".to_string());
        }
        let runtimeHistory = self
            .chatHistoryDelegate
            .getRuntimeChatHistoryUpTo(chatId.clone(), targetMessage.timestamp);
        let targetRuntimeIndex = runtimeHistory
            .iter()
            .position(|message| message.timestamp == targetMessage.timestamp)
            .ok_or_else(|| format!("Runtime message timestamp not found: {messageTimestamp}"))?;
        let executionLease = self
            .chatHistoryDelegate
            .chatHistoryManager
            .beginChatExecution(&chatId)
            .map_err(|error| error.to_string())?;
        executionLease
            .protectRevision(targetMessage.timestamp, targetMessage.variantCount)
            .map_err(|error| error.to_string())?;
        let configuration = enhancedAiService
            .resolveChatConfigurationForOptions(&SendMessageOptions {
                chatId: Some(chatId.clone()),
                promptFunctionType: self.currentPromptFunctionType.clone(),
                ..SendMessageOptions::new()
            })
            .await
            .map_err(|error| error.to_string())?;
        let chatProviderIdOverride = Some(configuration.profile.modelBinding.providerId.clone());
        let chatModelIdOverride = Some(configuration.profile.modelBinding.modelId.clone());
        let requestHistory = runtimeHistory[..targetRuntimeIndex].to_vec();
        let requestMessageContent = requestHistory
            .last()
            .filter(|message| message.sender == "user")
            .map(ChatMessage::displayText)
            .unwrap_or_default();
        let currentChat = self
            .chatHistoryDelegate
            .chatHistoriesFlow
            .value()
            .iter()
            .find(|history| history.id == chatId)
            .cloned();
        let workspacePath = currentChat
            .as_ref()
            .and_then(|chat| self.chatHistoryDelegate.primaryWorkspacePathForChat(chat));
        let enableThinking = ApiPreferences::getInstance()
            .enableThinkingModeFlow()
            .first()
            .expect("enable_thinking_mode preference must be readable");
        let enableMemoryAutoUpdate = ApiPreferences::getInstance()
            .enableMemoryAutoUpdateFlow()
            .first()
            .expect("enable_memory_auto_update preference must be readable");
        let modelProviderId = configuration.profile.modelBinding.providerId.clone();
        let modelId = configuration.profile.modelBinding.modelId.clone();
        self.regenerateSingleAiMessageWithRequest(
            executionLease,
            configuration,
            enhancedAiService,
            chatId,
            targetMessage,
            requestMessageContent,
            requestHistory,
            workspacePath,
            enableThinking,
            enableMemoryAutoUpdate,
            modelProviderId.clone(),
            modelId.clone(),
            chatProviderIdOverride,
            chatModelIdOverride)
        .await
    }

    /// Runs the prepared regeneration request using its resolved model configuration.
    async fn regenerateSingleAiMessageWithRequest(
        &mut self,
        executionLease: ChatExecutionLease,
        configuration: ChatConfigurationResult,
        enhancedAiService: &mut EnhancedAIService,
        chatId: String,
        targetMessage: ChatMessage,
        requestMessageContent: String,
        requestHistory: Vec<ChatMessage>,
        workspacePath: Option<String>,
        enableThinking: bool,
        enableMemoryAutoUpdate: bool,
        modelProviderId: String,
        modelId: String,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>) -> Result<(), String> {
        let chatContextSettings = self
            .messageProcessingDelegate
            .modelConfigManager
            .getResolvedModelConfig(&modelProviderId, &modelId)
            .map_err(|error| error.to_string())?;
        let maxTokens = (chatContextSettings.context.maxContextLength * 1024.0)
            .clamp(0.0, i32::MAX as f32) as i32;
        let mut variantMessage = self
            .messageProcessingDelegate
            .regenerateAiMessageVariant(RegenerateAiMessageVariantRequest {
                executionLease,
                chatConfiguration: configuration,
                enhancedAiService,
                chatHistoryDelegate: &mut self.chatHistoryDelegate,
                chatId: chatId.clone(),
                targetMessageTimestamp: targetMessage.timestamp,
                requestMessageContent,
                requestHistory,
                workspacePath,
                promptFunctionType: self.currentPromptFunctionType.clone(),
                attachments: Vec::new(),
                replyToMessage: None,
                enableThinking,
                enableMemoryAutoUpdate,
                maxTokens,
                tokenUsageThreshold: chatContextSettings.summary.summaryTokenThreshold as f64,
                chatProviderIdOverride,
                chatModelIdOverride,
            })
            .await
            .map_err(|error| error.to_string())?;
        self.chatHistoryDelegate.addMessageToChat(
            ChatMessage {
                parts: vec![MessagePart::markdown(
                    "part-0".to_string(),
                    0,
                    String::new(),
                )],
                selectedVariantIndex: targetMessage.variantCount,
                variantCount: targetMessage.variantCount + 1,
                isVariantPreview: true,
                ..variantMessage.clone()
            },
            Some(chatId.clone()),
        );
        let Some(mut contentStream) = self
            .messageProcessingDelegate
            .activeResponseStreamForChat(chatId.clone())
        else {
            return Err("Regenerated message stream is missing".to_string());
        };
        let mut content = String::new();
        contentStream
            .collect(&mut |chunk| {
                content.push_str(&chunk);
            })
            .await;
        variantMessage.parts = MessagePartCodec::parseAssistantMarkup(&content)
            .expect("regenerated assistant markup must parse into message parts");
        variantMessage.isVariantPreview = false;
        self.chatHistoryDelegate.addMessageVariant(
            targetMessage.timestamp,
            variantMessage,
            Some(chatId),
        );
        Ok(())
    }

    /// Executes a routed continuation with its authenticated persisted profile snapshot rather than current selection.
    pub async fn sendRoutedContinuation(
        &mut self,
        service: &mut EnhancedAIService,
        chatId: String,
        configuration: ChatConfigurationResult,
        executionLease: ChatExecutionLease,
        context: operit_tools::runtime_support::CoreRouteResumeContext,
    ) -> Result<(), String> {
        configuration.validate()?;
        if configuration.extensionOwner != context.extensionOwner
            || configuration.profile.id != context.participantId
        {
            return Err(
                "Routed configuration does not match its execution snapshot identity".to_string(),
            );
        }
        let workspaceFolders = self
            .chatHistoryDelegate
            .chatHistoryManager
            .getWorkspaceForChat(&chatId)
            .map_err(|error| error.to_string())?
            .map(|workspace| workspace.folderPaths())
            .into_iter()
            .flatten()
            .collect();
        let result = self
            .messageProcessingDelegate
            .sendUserMessage(SendUserMessageProcessingRequest {
                executionLease,
                chatConfiguration: configuration,
                enhancedAiService: service,
                chatHistoryDelegate: &mut self.chatHistoryDelegate,
                chatId: chatId.clone(),
                messageText: String::new(),
                chatHistory: context.runtimeChatHistory,
                promptHistoryOverride: None,
                workspacePath: context.workspacePath,
                workspaceFolders,
                promptFunctionType: context.promptFunctionType,
                attachments: Vec::new(),
                replyToMessage: None,
                enableThinking: context.enableThinking,
                enableMemoryAutoUpdate: false,
                maxTokens: 0,
                tokenUsageThreshold: 0.0,
                chatProviderIdOverride: context.chatProviderIdOverride,
                chatModelIdOverride: context.chatModelIdOverride,
                proxySenderNameOverride: context.proxySenderName,
                suppressUserMessageInHistory: true,
                isAutoContinuation: false,
                isResume: true,
                turnOptions: context.turnOptions.clone(),
            })
            .await
            .map_err(|error| error.to_string())?;
        self.messageProcessingDelegate.notifyTurnComplete(
            Some(chatId),
            service,
            result.nextWindowSize,
            context.turnOptions,
        );
        Ok(())
    }

    /// Shared send pipeline used by direct sends, continuations, and explicitly identified continuation turns.
    pub async fn sendMessageInternal(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        promptFunctionType: PromptFunctionType,
        isContinuation: bool,
        isAutoContinuation: bool,
        isResume: bool,
        participantId: Option<String>,
        chatIdOverride: Option<String>,
        messageText: String,
        proxySenderNameOverride: Option<String>,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
        attachments: Vec<AttachmentInfo>,
        replyToMessage: Option<ChatMessage>,
        suppressUserMessageInHistory: bool,
        chatHistoryOverride: Option<Vec<ChatMessage>>,
        turnOptions: ChatTurnOptions,
        admittedLease: Option<ChatExecutionLease>) -> Result<ChatTurnSubmission, String> {
        self.currentPromptFunctionType = promptFunctionType.clone();
        self.currentChatProviderIdOverride = chatProviderIdOverride.clone();
        self.currentChatModelIdOverride = chatModelIdOverride.clone();
        let chatId = match chatIdOverride {
            Some(chatId) if !chatId.trim().is_empty() => chatId,
            Some(_) => {
                self.messageProcessingDelegate
                    .nonFatalErrorEventFlow
                    .set_value(Some("Chat id is empty".to_string()));
                return Err("Chat id is empty".to_string());
            }
            None => match self.chatHistoryDelegate.currentChatIdFlow.value() {
                Some(chatId) => chatId,
                None => match self
                    .chatHistoryDelegate
                    .createNewChat(enhancedAiService, true, None, None)
                    .await
                {
                    Ok(chatId) => chatId,
                    Err(error) => {
                        self.messageProcessingDelegate
                            .nonFatalErrorEventFlow
                            .set_value(Some(error.clone()));
                        return Err(error);
                    }
                },
            },
        };
        self.chatHistoryDelegate.requireChatExists(&chatId)?;
        if let Some(continuation) = &turnOptions.continuation {
            if !messageText.is_empty() || !attachments.is_empty() || replyToMessage.is_some() || proxySenderNameOverride.is_some() {
                return Err("Continuation cannot resubmit text, attachments, reply target or sender name".to_string());
            }
            let source = self.chatHistoryDelegate.chatHistoryManager
                .loadChatMessageVariant(&chatId, continuation.userMessageTimestamp, 0)
                .map_err(|error| error.to_string())?;
            if source.sender != "user" {
                return Err("Continuation requires a real committed user-message timestamp".to_string());
            }
            let messages = self.chatHistoryDelegate.chatHistoryManager.loadChatMessages(&chatId)
                .map_err(|error| error.to_string())?;
            if messages.iter().any(|message| message.sender == "user" && message.timestamp > source.timestamp) {
                return Err("Continuation source was superseded by another committed user turn".to_string());
            }
        }
        let providerOverrideSet = match chatProviderIdOverride.as_ref() {
            Some(value) => !value.trim().is_empty(),
            None => false,
        };
        let modelOverrideSet = match chatModelIdOverride.as_ref() {
            Some(value) => !value.trim().is_empty(),
            None => false,
        };
        ChainLogger::info(
            SEND_CHAIN,
            "send.dispatch.start",
            &[
                ("chatId", chatId.clone()),
                ("prompt", format!("{:?}", promptFunctionType)),
                ("continuation", ChainLogger::boolField(isContinuation)),
                (
                    "autoContinuation",
                    ChainLogger::boolField(isAutoContinuation),
                ),

                ("attachments", attachments.len().to_string()),
                (
                    "persistTurn",
                    ChainLogger::boolField(turnOptions.persistTurn),
                ),
                (
                    "providerOverrideSet",
                    ChainLogger::boolField(providerOverrideSet),
                ),
                ("modelOverrideSet", ChainLogger::boolField(modelOverrideSet)),
            ],
        );
        self.tokenStatisticsDelegate
            .setActiveChatId(Some(chatId.clone()));
        self.tokenStatisticsDelegate
            .bindChatService(Some(chatId.clone()), enhancedAiService);
        let currentChat = self
            .chatHistoryDelegate
            .chatHistoriesFlow
            .value()
            .iter()
            .find(|history| history.id == chatId)
            .cloned();
        let workspacePath = currentChat
            .as_ref()
            .and_then(|chat| self.chatHistoryDelegate.primaryWorkspacePathForChat(chat));
        let executionLease = match match admittedLease {
            Some(lease) => Ok(lease),
            None => self.chatHistoryDelegate.chatHistoryManager.beginChatExecution(&chatId),
        } {
            Ok(lease) => lease,
            Err(error) => {
                self.messageProcessingDelegate
                    .setInputProcessingStateForChat(
                        chatId.clone(),
                        InputProcessingState::Error {
                            message: error.to_string(),
                        },
                    );
                return Err(error.to_string());
            }
        };
        let configurationOptions = SendMessageOptions {
            chatId: Some(chatId.clone()),
            executionParticipantId: participantId.clone(),
            promptFunctionType: promptFunctionType.clone(),
            chatProviderIdOverride: chatProviderIdOverride.clone(),
            chatModelIdOverride: chatModelIdOverride.clone(),
            ..SendMessageOptions::new()
        };
        let configuration = match enhancedAiService
            .resolveChatConfigurationForOptions(&configurationOptions)
            .await
        {
            Ok(configuration) => configuration,
            Err(error) => {
                self.messageProcessingDelegate
                    .setInputProcessingStateForChat(
                        chatId.clone(),
                        InputProcessingState::Error {
                            message: error.to_string(),
                        },
                    );
                return Err(error.to_string());
            }
        };
        let runtimeChatHistory = match chatHistoryOverride {
            Some(history) => history,
            None => self
                .chatHistoryDelegate
                .getRuntimeChatHistory(chatId.clone()),
        };
        let enableThinking = ApiPreferences::getInstance()
            .enableThinkingModeFlow()
            .first()
            .expect("enable_thinking_mode preference must be readable");
        let workspaceFolders = currentChat
            .as_ref()
            .map(|chat| self.chatHistoryDelegate.workspaceFolderPathsForChat(chat))
            .unwrap_or_default();
        let result = self
            .messageProcessingDelegate
            .sendUserMessage(SendUserMessageProcessingRequest {
                executionLease,
                chatConfiguration: configuration,
                enhancedAiService,
                chatHistoryDelegate: &mut self.chatHistoryDelegate,
                chatId: chatId.clone(),
                messageText,
                chatHistory: runtimeChatHistory,
                promptHistoryOverride: None,
                workspacePath,
                workspaceFolders,
                promptFunctionType,
                attachments,
                replyToMessage,
                enableThinking,
                enableMemoryAutoUpdate: turnOptions.persistTurn
                    && proxySenderNameOverride
                        .as_ref()
                        .map(|s| s.trim().is_empty())
                        .unwrap_or(true)
                    && ApiPreferences::getInstance()
                        .enableMemoryAutoUpdateFlow()
                        .first()
                        .expect("memory auto-update preference must be readable"),
                maxTokens: 0,
                tokenUsageThreshold: 0.0,
                chatProviderIdOverride,
                chatModelIdOverride,
                proxySenderNameOverride,
                suppressUserMessageInHistory: suppressUserMessageInHistory || isContinuation,
                isAutoContinuation,
                isResume,
                turnOptions: turnOptions.clone(),
            })
            .await;
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                ChainLogger::error(
                    SEND_CHAIN,
                    "send.dispatch.error",
                    &[("chatId", chatId.clone()), ("error", error.to_string())],
                );
                self.messageProcessingDelegate
                    .finishChatExecution(
                        chatId.clone(),
                        InputProcessingState::Error {
                            message: error.to_string(),
                        },
                    );
                return Err(error.to_string());
            }
        };
        self.tokenStatisticsDelegate
            .updateCumulativeStatistics(Some(chatId.clone()), Some(enhancedAiService));
        let (inputTokens, outputTokens) = self
            .tokenStatisticsDelegate
            .getCumulativeTokenCounts(Some(chatId.clone()));
        let windowSize = result.nextWindowSize.unwrap_or_else(|| {
            self.tokenStatisticsDelegate
                .getLastCurrentWindowSize(Some(chatId.clone()))
        });
        self.tokenStatisticsDelegate.setTokenCounts(
            Some(chatId.clone()),
            inputTokens,
            outputTokens,
            windowSize,
        );
        if turnOptions.persistTurn {
            self.chatHistoryDelegate.saveCurrentChat(
                inputTokens,
                outputTokens,
                windowSize,
                Some(chatId.clone()),
            );
        }
        ChainLogger::info(
            SEND_CHAIN,
            "send.dispatch.accepted",
            &[
                ("chatId", chatId.clone()),
                ("inputTokens", inputTokens.to_string()),
                ("outputTokens", outputTokens.to_string()),
                ("windowSize", windowSize.to_string()),
            ],
        );
        if isAutoContinuation {
            self.removePendingAutoContinuation(chatId);
        }
        Ok(result.completion)
    }

    #[allow(non_snake_case)]
    /// Starts a user-requested conversation summary for the current chat.
    pub async fn manuallySummarizeConversation(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
    ) {
        if self.isSummarizing {
            return;
        }
        let currentChatId = self.chatHistoryDelegate.currentChatIdFlow.value();
        self.summarizeHistory(
            enhancedAiService,
            false,
            None,
            currentChatId,
            None,
            None,
            None)
        .await;
    }

    /// Summarizes history after context limits are exceeded, then continues the turn.
    pub async fn handleTokenLimitExceeded(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        chatId: Option<String>,
        participantId: Option<String>) {
        self.summaryJob = Some("summaryJob".to_string());
        self.summarizeHistory(
            enhancedAiService,
            true,
            None,
            chatId,
            None,
            None,
            participantId)
        .await;
        self.summaryJob = None;
    }

    /// Cancels active summary streaming work.
    fn cancelSummaryStreamingInternal(&mut self, _enhancedAiService: &mut EnhancedAIService) {}

    /// Cancels summary jobs and queued continuations for a chat target.
    fn cancelSummaryInternal(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        targetChatId: Option<String>,
    ) {
        let currentChatId = targetChatId
            .clone()
            .or_else(|| self.chatHistoryDelegate.currentChatIdFlow.value());
        let shouldCancelSummary = self.isSummarizing
            && (targetChatId.is_none() || self.summarizingChatId == targetChatId);
        let shouldCancelAsyncSummary = self.isSendTriggeredSummarizing
            && (targetChatId.is_none() || self.sendTriggeredSummarizingChatId == targetChatId);
        let shouldCancelPendingAutoContinuation = targetChatId
            .as_ref()
            .map(|chatId| self.pendingAutoContinuationByChatId.contains_key(chatId))
            .unwrap_or_else(|| {
                currentChatId
                    .as_ref()
                    .map(|chatId| self.pendingAutoContinuationByChatId.contains_key(chatId))
                    .unwrap_or(false)
            });
        if !shouldCancelSummary && !shouldCancelAsyncSummary && !shouldCancelPendingAutoContinuation
        {
            if targetChatId.is_none() {
                self.cancelSummaryStreamingInternal(enhancedAiService);
            }
            return;
        }
        self.cancelSummaryStreamingInternal(enhancedAiService);
        if shouldCancelSummary {
            self.summaryJob = None;
            self.isSummarizing = false;
            self.summarizingChatId = None;
        }
        if shouldCancelAsyncSummary {
            self.sendTriggeredSummaryJob = None;
            self.isSendTriggeredSummarizing = false;
            self.sendTriggeredSummarizingChatId = None;
        }
        if shouldCancelPendingAutoContinuation {
            if let Some(chatId) = currentChatId {
                self.removePendingAutoContinuation(chatId);
            }
        }
        self.messageProcessingDelegate
            .refreshActiveStreamingChatIds();
    }

    /// Cancels active summary work for the current chat.
    pub fn cancelSummary(&mut self, enhancedAiService: &mut EnhancedAIService) {
        self.cancelSummaryInternal(enhancedAiService, None);
    }

    /// Cancels active summary work for one chat.
    pub fn cancelSummaryForChat(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        chatId: String,
    ) {
        self.cancelSummaryInternal(enhancedAiService, Some(chatId));
    }

    /// Cancels summary work before destructive history mutation.
    pub fn cancelSummaryForDestructiveMutation(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        chatId: String,
    ) {
        self.cancelSummaryInternal(enhancedAiService, Some(chatId));
    }

    /// Runs the asynchronous summary task created by a send turn.
    async fn launchAsyncSummaryForSend(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        snapshotMessages: Vec<ChatMessage>,
        beforeTimestamp: Option<i64>,
        afterTimestamp: Option<i64>,
        originalChatId: Option<String>,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
    ) {
        if snapshotMessages.is_empty() || originalChatId.is_none() {
            return;
        }
        let originalChatId = originalChatId.expect("originalChatId checked");
        self.isSendTriggeredSummarizing = true;
        self.sendTriggeredSummarizingChatId = Some(originalChatId.clone());
        self.messageProcessingDelegate
            .setPendingAsyncSummaryUiForChat(originalChatId.clone(), true);
        self.messageProcessingDelegate
            .setSuppressIdleCompletedStateForChat(originalChatId.clone(), true);
        self.messageProcessingDelegate
            .setInputProcessingStateForChat(
                originalChatId.clone(),
                InputProcessingState::Summarizing {
                    message: "chat_compressing_history".to_string(),
                },
            );
        if let Ok(Some(summaryMessage)) = AIMessageManager::summarizeMemory(
            enhancedAiService,
            snapshotMessages,
            false)
        .await
        {
            self.chatHistoryDelegate.addSummaryMessage(
                summaryMessage,
                beforeTimestamp,
                afterTimestamp,
                Some(originalChatId.clone()),
            );
            self.refreshStableContextWindow(
                enhancedAiService,
                Some(originalChatId.clone()),
                None,
                None,
                chatProviderIdOverride,
                chatModelIdOverride)
            .await;
        }
        self.isSendTriggeredSummarizing = false;
        self.sendTriggeredSummarizingChatId = None;
        self.messageProcessingDelegate
            .setPendingAsyncSummaryUiForChat(originalChatId.clone(), false);
        self.messageProcessingDelegate
            .setSuppressIdleCompletedStateForChat(originalChatId.clone(), false);
        self.messageProcessingDelegate
            .setInputProcessingStateForChat(originalChatId, InputProcessingState::Idle);
    }

    /// Generates and stores a summary message for the selected chat history.
    async fn summarizeHistory(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        autoContinue: bool,
        promptFunctionType: Option<PromptFunctionType>,
        chatIdOverride: Option<String>,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
        participantId: Option<String>) -> bool {
        if self.isSummarizing {
            return false;
        }
        self.isSummarizing = true;
        let currentChatId =
            chatIdOverride.or_else(|| self.chatHistoryDelegate.currentChatIdFlow.value());
        self.summarizingChatId = currentChatId.clone();
        if let Some(currentChatId) = currentChatId.clone() {
            self.messageProcessingDelegate
                .setSuppressIdleCompletedStateForChat(currentChatId.clone(), true);
            self.messageProcessingDelegate
                .setInputProcessingStateForChat(
                    currentChatId,
                    InputProcessingState::Summarizing {
                        message: "chat_compressing_history".to_string(),
                    },
                );
        }
        let effectiveChatModelIdOverride =
            chatModelIdOverride.or_else(|| self.currentChatModelIdOverride.clone());
        let effectiveChatProviderIdOverride =
            chatProviderIdOverride.or_else(|| self.currentChatProviderIdOverride.clone());
        let currentMessages = currentChatId
            .clone()
            .map(|chatId| self.chatHistoryDelegate.getRuntimeChatHistory(chatId))
            .unwrap_or_default();
        if currentMessages.is_empty() {
            self.isSummarizing = false;
            self.summarizingChatId = None;
            if let Some(currentChatId) = currentChatId {
                self.messageProcessingDelegate
                    .setSuppressIdleCompletedStateForChat(currentChatId.clone(), false);
                self.messageProcessingDelegate
                    .setInputProcessingStateForChat(currentChatId, InputProcessingState::Idle);
            }
            self.messageProcessingDelegate
                .refreshActiveStreamingChatIds();
            return false;
        }
        let insertPosition = self
            .chatHistoryDelegate
            .findProperSummaryPosition(currentMessages.clone());
        let beforeTimestamp = currentMessages
            .get(insertPosition.saturating_sub(1))
            .map(|message| message.timestamp);
        let afterTimestamp = currentMessages
            .get(insertPosition)
            .map(|message| message.timestamp);
        let mut summarySuccess = false;
        if let Ok(Some(summaryMessage)) = AIMessageManager::summarizeMemory(
            enhancedAiService,
            currentMessages,
            autoContinue)
        .await
        {
            self.chatHistoryDelegate.addSummaryMessage(
                summaryMessage,
                beforeTimestamp,
                afterTimestamp,
                currentChatId.clone(),
            );
            self.refreshStableContextWindow(
                enhancedAiService,
                currentChatId.clone(),
                participantId.clone(),
                None,
                effectiveChatProviderIdOverride.clone(),
                effectiveChatModelIdOverride.clone())
            .await;
            summarySuccess = true;
        }
        self.isSummarizing = false;
        if self.summarizingChatId == currentChatId {
            self.summarizingChatId = None;
        }
        if let Some(currentChatIdForState) = currentChatId.clone() {
            if !summarySuccess || !autoContinue {
                self.messageProcessingDelegate
                    .setSuppressIdleCompletedStateForChat(currentChatIdForState.clone(), false);
                self.messageProcessingDelegate
                    .setInputProcessingStateForChat(
                        currentChatIdForState,
                        InputProcessingState::Idle,
                    );
            }
        }
        self.messageProcessingDelegate
            .refreshActiveStreamingChatIds();
        if summarySuccess && autoContinue {
            if let Some(currentChatId) = currentChatId {
                let continuationPromptType =
                    promptFunctionType.unwrap_or_else(|| self.currentPromptFunctionType.clone());
                if self
                    .messageProcessingDelegate
                    .isChatLoading(currentChatId.clone())
                {
                    self.queuePendingAutoContinuation(
                        currentChatId,
                        continuationPromptType,
                        effectiveChatProviderIdOverride.clone(),
                        effectiveChatModelIdOverride,
                        participantId);
                } else {
                    self.messageProcessingDelegate
                        .setSuppressIdleCompletedStateForChat(currentChatId.clone(), false);
                    self.sendMessageInternal(
                        enhancedAiService,
                        continuationPromptType,
                        true,
                        true,
                        false,
                        participantId,
                        Some(currentChatId),
                        String::new(),
                        None,
                        effectiveChatProviderIdOverride,
                        effectiveChatModelIdOverride,
                        Vec::new(),
                        None,
                        false,
                        None,
                        ChatTurnOptions::default(), None)
                    .await;
                }
            }
        }
        summarySuccess
    }

    /// Queues an automatic continuation to run after the active turn settles.
    fn queuePendingAutoContinuation(
        &mut self,
        chatId: String,
        promptFunctionType: PromptFunctionType,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
        participantId: Option<String>) {
        self.pendingAutoContinuationByChatId.insert(
            chatId.clone(),
            PendingAutoContinuationRequest {
                chatId,
                promptFunctionType,
                chatProviderIdOverride,
                chatModelIdOverride,
                participantId,
                waitJob: Some("waitJob".to_string()),
            },
        );
    }

    /// Removes queued automatic continuation state for one chat.
    fn removePendingAutoContinuation(&mut self, chatId: String) {
        self.pendingAutoContinuationByChatId.remove(&chatId);
    }

    /// Installs UI bridge callbacks used by platform integrations.
    pub fn setUiBridge(&mut self) {}
}
