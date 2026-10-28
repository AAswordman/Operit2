use futures_util::future::{AbortHandle, Abortable, BoxFuture, Shared};
use futures_util::FutureExt;
use operit_store::ChatExecutionLease::ChatExecutionLease;
use operit_util::stream::Stream::Stream;
use std::sync::Arc;
use tokio::sync::oneshot;

use crate::core::chat::AIMessageManager::{AIMessageManager, StableContextWindowRequest};
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::services::core::ChatHistoryDelegate::ChatHistoryDelegate;
use crate::services::core::MessageProcessingDelegate::ChatTurnSubmission;
use crate::services::core::MessageProcessingDelegate::{
    ChatSummaryTask, MessageProcessingDelegate, RegenerateAiMessageVariantRequest,
    SendUserMessageProcessingRequest,
};
use crate::services::core::TokenStatisticsDelegate::TokenStatisticsDelegate;
use crate::services::ProviderRuntimeSupportService::{ChatConfigurationApi, MODEL_CONFIGURATION_OWNER};
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatMessageDisplayMode::ChatMessageDisplayMode;
use operit_model::ChatTurnOptions::ChatTurnOptions;
use operit_model::FunctionType::FunctionType;
use operit_model::InputProcessingState::InputProcessingState;
use operit_model::MessagePart::MessagePart;
use operit_model::MessagePartCodec::MessagePartCodec;
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_providers::chat::EnhancedAIService::{EnhancedAIService, SendMessageOptions};
use operit_providers::runtime_support::ChatConfigurationResult;
use operit_util::AppLogger::AppLogger;
use operit_util::ChainLogger::{self, SEND_CHAIN};

/// Preserves the originating user turn and execution lease through context compaction.
pub(crate) struct SummaryContinuationRequest {
    pub chatId: String,
    pub turnId: u64,
    pub participantId: Option<String>,
    pub promptFunctionType: PromptFunctionType,
    pub chatProviderIdOverride: Option<String>,
    pub chatModelIdOverride: Option<String>,
    pub turnOptions: ChatTurnOptions,
    pub executionLease: ChatExecutionLease,
}

/// Captures the exact history and execution context of one summary operation.
struct ConversationSummaryRequest {
    chatId: String,
    participantId: Option<String>,
    promptFunctionType: PromptFunctionType,
    chatProviderIdOverride: Option<String>,
    chatModelIdOverride: Option<String>,
    autoContinue: bool,
    requireCurrentChat: bool,
}

/// Coordinates high-level chat sends, generic participant execution and conversation summaries.
pub struct MessageCoordinationDelegate {
    pub chatHistoryDelegate: ChatHistoryDelegate,
    pub messageProcessingDelegate: MessageProcessingDelegate,
    pub tokenStatisticsDelegate: TokenStatisticsDelegate,
    pub isUpdatingMemory: bool,
    pub currentPromptFunctionType: PromptFunctionType,
    pub currentChatProviderIdOverride: Option<String>,
    pub currentChatModelIdOverride: Option<String>,
    pub nonFatalErrorCollectorJob: Option<String>,
}

impl MessageCoordinationDelegate {
    /// Creates a coordinator from history and message-processing delegates.
    pub fn new(
        chatHistoryDelegate: ChatHistoryDelegate,
        messageProcessingDelegate: MessageProcessingDelegate,
    ) -> Self {
        let mut delegate = Self::forBackgroundWork(chatHistoryDelegate, messageProcessingDelegate);
        delegate.ensureNonFatalErrorCollectorStarted();
        delegate
    }

    /// Reuses shared delegates without registering another permanent UI error observer.
    pub(crate) fn forBackgroundWork(
        chatHistoryDelegate: ChatHistoryDelegate,
        messageProcessingDelegate: MessageProcessingDelegate,
    ) -> Self {
        Self {
            chatHistoryDelegate,
            messageProcessingDelegate,
            tokenStatisticsDelegate: TokenStatisticsDelegate::default(),
            isUpdatingMemory: false,
            currentPromptFunctionType: PromptFunctionType::CHAT,
            currentChatProviderIdOverride: None,
            currentChatModelIdOverride: None,
            nonFatalErrorCollectorJob: None,
        }
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

    /// Reads a revision's persisted author and propagates storage or missing-identity errors.
    pub(crate) async fn messageAuthorParticipantId(
        &self,
        service: &EnhancedAIService,
        chatId: &str,
        messageTimestamp: i64,
        variantIndex: i32,
    ) -> Result<String, String> {
        let api = ChatConfigurationApi::ready(&service.tool_handler).await;
        if api.extensionOwner() == MODEL_CONFIGURATION_OWNER {
            return Ok("model:chat".into());
        }
        let extensions = self
            .chatHistoryDelegate
            .chatHistoryManager
            .readPluginExtensions(&operit_model::PluginExtensionTarget::PluginExtensionTarget::Message {
                chatId: chatId.to_string(),
                messageTimestamp,
                variantIndex,
            })
            .map_err(|error| error.to_string())?;
        extensions
            .get(MODEL_CONFIGURATION_OWNER)
            .or_else(|| extensions.get(api.extensionOwner()))
            .and_then(|identity| identity.get("participantId"))
            .and_then(serde_json::Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .map(str::to_string)
            .ok_or_else(|| format!(
                "Message revision has no recorded execution participant: {chatId}/{messageTimestamp}/{variantIndex}"
            ))
    }

    /// Recalculates the stable context window size for a chat and prompt mode.
    pub async fn recalculateStableWindowSize(
        &mut self,
        service: &mut EnhancedAIService,
        chatId: String,
        participantId: Option<String>,
        promptFunctionType: PromptFunctionType,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
    ) -> Result<i64, String> {
        let currentChat = self
            .chatHistoryDelegate
            .chatHistoriesFlow()
            .value()
            .into_iter()
            .find(|history| history.id == chatId)
            .ok_or_else(|| format!("Chat not found for context estimation: {chatId}"))?;
        let runtimeOptions = SendMessageOptions {
            chatId: Some(chatId.clone()),
            executionParticipantId: participantId.clone(),
            promptFunctionType: promptFunctionType.clone(),
            chatProviderIdOverride: chatProviderIdOverride.clone(),
            chatModelIdOverride: chatModelIdOverride.clone(),
            ..SendMessageOptions::new()
        };
        let request = service
            .chatConfigurationRequestForOptions(&runtimeOptions)
            .map_err(|error| error.to_string())?;
        let configurations = ChatConfigurationApi::ready(&service.tool_handler)
            .await
            .resolveContextParticipants(request)
            .await?;
        let mut windows = Vec::with_capacity(configurations.len());
        for configuration in configurations {
            let runtime = service
                .createSendMessageRuntime(&runtimeOptions, configuration)
                .map_err(|error| error.to_string())?;
            let window = AIMessageManager::calculateStableContextWindow(StableContextWindowRequest {
                enhancedAiService: service,
                chatId: Some(chatId.clone()),
                messageContent: String::new(),
                chatHistory: self.chatHistoryDelegate.getRuntimeChatHistory(chatId.clone()),
                workspacePath: self.chatHistoryDelegate.primaryWorkspacePathForChat(&currentChat),
                workspaceFolders: self.chatHistoryDelegate.workspaceFolderPathsForChat(&currentChat),
                promptFunctionType: promptFunctionType.clone(),
                proxySenderName: None,
                chatProviderIdOverride: chatProviderIdOverride.clone(),
                chatModelIdOverride: chatModelIdOverride.clone(),
                publishEstimate: false,
                runtime,
            })
            .await
            .map_err(|error| error.to_string())?;
            windows.push(window);
        }
        windows.into_iter().max().ok_or_else(|| {
            "Conversation context requires a current bound participant".to_string()
        })
    }

    /// Recalculates and persists the stable context window for a chat.
    pub async fn refreshStableContextWindow(
        &mut self,
        service: &mut EnhancedAIService,
        chatId: Option<String>,
        participantId: Option<String>,
        promptFunctionType: Option<PromptFunctionType>,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
    ) -> Option<i64> {
        let targetChatId = chatId.or_else(|| self.chatHistoryDelegate.currentChatIdFlow.value())?;
        let effectivePromptFunctionType =
            promptFunctionType.unwrap_or_else(|| self.currentPromptFunctionType.clone());
        let effectiveChatModelIdOverride =
            chatModelIdOverride.or_else(|| self.currentChatModelIdOverride.clone());
        let effectiveChatProviderIdOverride =
            chatProviderIdOverride.or_else(|| self.currentChatProviderIdOverride.clone());
        let newWindowSize = match self
            .recalculateStableWindowSize(
                service,
                targetChatId.clone(),
                participantId,
                effectivePromptFunctionType,
                effectiveChatProviderIdOverride,
                effectiveChatModelIdOverride,
            )
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
        let chat = match self
            .chatHistoryDelegate
            .chatHistoryManager
            .updateChatContextWindow(targetChatId.clone(), newWindowSize)
        {
            Ok(chat) => chat,
            Err(error) => {
                self.messageProcessingDelegate
                    .setInputProcessingStateForChat(
                        targetChatId,
                        InputProcessingState::Error {
                            message: error.to_string(),
                        },
                    );
                return None;
            }
        };
        let (inputTokens, outputTokens) = (chat.inputTokens, chat.outputTokens);
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
                executionParticipantId: Some(
                    self.messageAuthorParticipantId(
                        enhancedAiService,
                        &chatId,
                        targetMessage.timestamp,
                        targetMessage.selectedVariantIndex,
                    )
                    .await?,
                ),
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
        modelProviderId: String,
        modelId: String,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>) -> Result<(), String> {
        let chatContextSettings = self
            .messageProcessingDelegate
            .modelConfigManager
            .getResolvedModelConfig(&modelProviderId, &modelId)
            .map_err(|error| error.to_string())?;
        let maxTokens = chatContextSettings.context.maxContextTokens();
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
                maxTokens,
                tokenUsageThreshold: chatContextSettings.summary.tokenThresholdRatio(),
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
        if configuration.extensionOwner != MODEL_CONFIGURATION_OWNER
            && (configuration.extensionOwner != context.extensionOwner
                || configuration.profile.id != context.participantId)
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
                    .createNewChat(enhancedAiService, true, None, None, None)
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
        ChainLogger::verbose_with_level(
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
            operit_util::AppLogger::VERBOSE_LEVEL_2,
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
        let isBackgroundSend = self.chatHistoryDelegate.currentChatIdFlow.value().as_ref() != Some(&chatId);
        let (maxTokens, tokenUsageThreshold) = self.prepareSummaryForSend(
            enhancedAiService,
            &configuration,
            &chatId,
            participantId.clone(),
            promptFunctionType.clone(),
            turnOptions.persistTurn && !isBackgroundSend && !turnOptions.deferSummary,
            !isContinuation && !isAutoContinuation && !isResume,
        )?;
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
                maxTokens,
                tokenUsageThreshold,
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
        ChainLogger::verbose_with_level(
            SEND_CHAIN,
            "send.dispatch.accepted",
            &[
                ("chatId", chatId.clone()),
                ("inputTokens", inputTokens.to_string()),
                ("outputTokens", outputTokens.to_string()),
                ("windowSize", windowSize.to_string()),
            ],
            operit_util::AppLogger::VERBOSE_LEVEL_2,
        );
        Ok(result.completion)
    }

    /// Starts snapshot compaction without delaying dispatch and retains the tool-round token guard.
    #[allow(clippy::too_many_arguments)]
    fn prepareSummaryForSend(
        &mut self,
        enhancedAiService: &EnhancedAIService,
        configuration: &ChatConfigurationResult,
        chatId: &str,
        participantId: Option<String>,
        promptFunctionType: PromptFunctionType,
        enableSummary: bool,
        checkBeforeSend: bool,
    ) -> Result<(i32, f64), String> {
        let providerId = configuration.profile.modelBinding.providerId.clone();
        let modelId = configuration.profile.modelBinding.modelId.clone();
        let settings = self
            .messageProcessingDelegate
            .modelConfigManager
            .getResolvedModelConfig(&providerId, &modelId)
            .map_err(|error| error.to_string())?;
        let maxTokens = settings.context.maxContextTokens();
        if !enableSummary || !settings.summary.enableSummary {
            return Ok((maxTokens, f64::MAX));
        }
        let mut threshold = settings.summary.tokenThresholdRatio();
        if checkBeforeSend
            && self
                .messageProcessingDelegate
                .summaryTaskForChat(chatId)
                .is_none()
        {
            let messages = self
                .chatHistoryDelegate
                .getRuntimeChatHistory(chatId.to_string());
            let chat = self
                .chatHistoryDelegate
                .chatHistoryManager
                .loadChatHistory(chatId.to_string())
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("Chat does not exist: {chatId}"))?;
            if !messages.is_empty()
                && AIMessageManager::shouldGenerateSummary(
                    messages.clone(),
                    chat.currentWindowSize,
                    maxTokens,
                    threshold,
                    true,
                    settings.summary.enableSummaryByMessageCount,
                    settings.summary.summaryMessageCountThreshold,
                )
            {
                let _summaryCompletion = self.launchSummary(
                    enhancedAiService,
                    messages,
                    ConversationSummaryRequest {
                        chatId: chatId.to_string(),
                        participantId,
                        promptFunctionType,
                        chatProviderIdOverride: Some(providerId),
                        chatModelIdOverride: Some(modelId),
                        autoContinue: false,
                        requireCurrentChat: true,
                    },
                )?;
                threshold += 0.5;
            }
        }
        Ok((maxTokens, threshold))
    }

    /// Checks the completed conversation against every current participant's configured summary threshold.
    pub(crate) async fn checkSummaryAfterReply(
        &mut self, service: &mut EnhancedAIService, chatId: String, participantId: String,
    ) -> Result<(), String> {
        if let Some(task) = self.messageProcessingDelegate.summaryTaskForChat(&chatId) {
            task.completion.await?;
        }
        let configuration = service.resolveChatConfigurationForOptions(&SendMessageOptions {
            chatId: Some(chatId.clone()), executionParticipantId: Some(participantId),
            ..SendMessageOptions::new()
        }).await.map_err(|error| error.to_string())?;
        let window = self.recalculateStableWindowSize(
            service, chatId.clone(), None, PromptFunctionType::CHAT, None, None,
        ).await?;
        let messages = self.chatHistoryDelegate.getRuntimeChatHistory(chatId.clone());
        for participant in &configuration.participants {
            let settings = self.messageProcessingDelegate.modelConfigManager.getResolvedModelConfig(
                &participant.modelBinding.providerId, &participant.modelBinding.modelId,
            ).map_err(|error| error.to_string())?;
            if AIMessageManager::shouldGenerateSummary(
                messages.clone(), window, settings.context.maxContextTokens(),
                settings.summary.tokenThresholdRatio(), settings.summary.enableSummary,
                settings.summary.enableSummaryByMessageCount, settings.summary.summaryMessageCountThreshold,
            ) {
                return self.launchSummary(service, messages, ConversationSummaryRequest {
                    chatId, participantId: None, promptFunctionType: PromptFunctionType::CHAT,
                    chatProviderIdOverride: None, chatModelIdOverride: None,
                    autoContinue: false, requireCurrentChat: false,
                })?.await;
            }
        }
        Ok(())
    }

    /// Schedules one cancellable summary through the host and shares its actual completion.
    fn launchSummary(
        &self,
        enhancedAiService: &EnhancedAIService,
        messages: Vec<ChatMessage>,
        request: ConversationSummaryRequest,
    ) -> Result<Shared<BoxFuture<'static, Result<(), String>>>, String> {
        let chatId = request.chatId.clone();
        let (abortHandle, registration) = AbortHandle::new_pair();
        let (sender, receiver) = oneshot::channel();
        let completion = receiver
            .map(|result| {
                result.map_err(|error| format!("Summary completion channel closed: {error}"))?
            })
            .boxed()
            .shared();
        let taskId = Arc::new(());
        self.messageProcessingDelegate.registerSummaryTask(
            chatId.clone(),
            ChatSummaryTask {
                taskId: taskId.clone(),
                abortHandle,
                completion: completion.clone(),
            },
        )?;
        let mut worker = Self::forBackgroundWork(
            self.chatHistoryDelegate.clone_for_core(),
            self.messageProcessingDelegate.clone_for_core(),
        );
        worker.tokenStatisticsDelegate = self.tokenStatisticsDelegate.clone();
        let mut cleanup = self.messageProcessingDelegate.clone_for_core();
        cleanup.setPendingAsyncSummaryUiForChat(chatId.clone(), true);
        cleanup.setSuppressIdleCompletedStateForChat(chatId.clone(), true);
        cleanup.setInputProcessingStateForChat(
            chatId.clone(),
            InputProcessingState::Summarizing {
                message: "chat_compressing_history".to_string(),
            },
        );
        let mut service = enhancedAiService.clone();
        let workerChatId = chatId.clone();
        let workerTaskId = taskId.clone();
        let scheduled = defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
            "chat-summary",
            Box::new(move || {
                Box::pin(async move {
                    let outcome = Abortable::new(
                        worker.generateAndStoreSummary(&mut service, messages, request),
                        registration,
                    )
                    .await;
                    let result = match outcome {
                        Ok(result) => {
                            if let Err(error) = &result {
                                AppLogger::e(
                                    "MessageCoordinationDelegate",
                                    &format!("Conversation summary failed: {error}"),
                                );
                                worker
                                    .messageProcessingDelegate
                                    .nonFatalErrorEventFlow
                                    .set_value(Some(error.clone()));
                            }
                            result
                        }
                        Err(_) => {
                            match service
                                .multi_service_manager
                                .getServiceForFunction(FunctionType::SUMMARY)
                            {
                                Ok(summaryService) => {
                                    summaryService.lock().await.cancel_streaming()
                                }
                                Err(error) => {
                                    AppLogger::w(
                                        "MessageCoordinationDelegate",
                                        &format!("Failed to cancel summary stream: {error}"),
                                    );
                                }
                            }
                            Err("Conversation summary cancelled".to_string())
                        }
                    };
                    worker
                        .messageProcessingDelegate
                        .finishSummaryTask(&workerChatId, &workerTaskId);
                    if sender.send(result).is_err() {
                        AppLogger::v_with_level(
                            "MessageCoordinationDelegate",
                            "Summary observer detached",
                            operit_util::AppLogger::VERBOSE_LEVEL_4,
                        );
                    }
                })
            }),
        );
        if let Err(error) = scheduled {
            cleanup.finishSummaryTask(&chatId, &taskId);
            return Err(error.to_string());
        }
        Ok(completion)
    }

    /// Summarizes the captured history and persists it between its original timestamp anchors.
    async fn generateAndStoreSummary(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        messages: Vec<ChatMessage>,
        request: ConversationSummaryRequest,
    ) -> Result<(), String> {
        let insertPosition = self
            .chatHistoryDelegate
            .findProperSummaryPosition(messages.clone());
        let beforeTimestamp = insertPosition
            .checked_sub(1)
            .and_then(|index| messages.get(index))
            .map(|message| message.timestamp);
        let afterTimestamp = messages
            .get(insertPosition)
            .map(|message| message.timestamp);
        let summary =
            AIMessageManager::summarizeMemory(enhancedAiService, messages, request.autoContinue)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "No valid conversation summary was generated".to_string())?;
        if request.requireCurrentChat
            && self.chatHistoryDelegate.currentChatIdFlow.value().as_ref() != Some(&request.chatId)
        {
            return Ok(());
        }
        self.chatHistoryDelegate.addSummaryMessage(
            summary,
            beforeTimestamp,
            afterTimestamp,
            Some(request.chatId.clone()),
        );
        self.refreshStableContextWindow(
            enhancedAiService,
            Some(request.chatId),
            request.participantId,
            Some(request.promptFunctionType),
            request.chatProviderIdOverride,
            request.chatModelIdOverride,
        )
        .await
        .ok_or_else(|| "Failed to recalculate context after conversation summary".to_string())?;
        Ok(())
    }

    /// Starts a user-requested summary and waits for its host-owned task to finish.
    pub async fn manuallySummarizeConversation(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
    ) {
        let Some(chatId) = self.chatHistoryDelegate.currentChatIdFlow.value() else {
            return;
        };
        if self
            .messageProcessingDelegate
            .summaryTaskForChat(&chatId)
            .is_some()
        {
            return;
        }
        let messages = self
            .chatHistoryDelegate
            .getRuntimeChatHistory(chatId.clone());
        let result = match self.launchSummary(
            enhancedAiService,
            messages,
            ConversationSummaryRequest {
                chatId,
                participantId: None,
                promptFunctionType: self.currentPromptFunctionType.clone(),
                chatProviderIdOverride: self.currentChatProviderIdOverride.clone(),
                chatModelIdOverride: self.currentChatModelIdOverride.clone(),
                autoContinue: false,
                requireCurrentChat: false,
            },
        ) {
            Ok(completion) => completion.await,
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            self.messageProcessingDelegate
                .nonFatalErrorEventFlow
                .set_value(Some(error));
        }
    }

    /// Compacts a persisted tool round and resumes the same committed user turn after compaction.
    pub(crate) async fn handleTokenLimitExceeded(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        request: SummaryContinuationRequest,
    ) -> Result<ChatTurnSubmission, String> {
        if let Some(task) = self
            .messageProcessingDelegate
            .summaryTaskForChat(&request.chatId)
        {
            task.completion.await?;
        }
        if !self
            .messageProcessingDelegate
            .isCurrentChatTurn(&request.chatId, request.turnId)
        {
            return Err("Summary continuation no longer owns the chat turn".to_string());
        }
        let messages = self
            .chatHistoryDelegate
            .getRuntimeChatHistory(request.chatId.clone());
        self.launchSummary(
            enhancedAiService,
            messages,
            ConversationSummaryRequest {
                chatId: request.chatId.clone(),
                participantId: request.participantId.clone(),
                promptFunctionType: request.promptFunctionType.clone(),
                chatProviderIdOverride: request.chatProviderIdOverride.clone(),
                chatModelIdOverride: request.chatModelIdOverride.clone(),
                autoContinue: true,
                requireCurrentChat: false,
            },
        )?
        .await?;
        if !self
            .messageProcessingDelegate
            .isCurrentChatTurn(&request.chatId, request.turnId)
        {
            return Err("Summary continuation no longer owns the chat turn".to_string());
        }
        Box::pin(self.sendMessageInternal(
            enhancedAiService,
            request.promptFunctionType,
            true,
            true,
            false,
            request.participantId,
            Some(request.chatId),
            String::new(),
            None,
            request.chatProviderIdOverride,
            request.chatModelIdOverride,
            Vec::new(),
            None,
            true,
            None,
            request.turnOptions,
            Some(request.executionLease),
        ))
        .await
    }

    /// Cancels all host-owned summary tasks associated with this coordinator.
    pub fn cancelSummary(&mut self, _enhancedAiService: &mut EnhancedAIService) {
        self.messageProcessingDelegate.cancelSummaryTasks(None);
    }

    /// Cancels the host-owned summary task for one chat.
    pub fn cancelSummaryForChat(
        &mut self,
        _enhancedAiService: &mut EnhancedAIService,
        chatId: String,
    ) {
        self.messageProcessingDelegate
            .cancelSummaryTasks(Some(&chatId));
    }

    /// Cancels summary work before its source history is changed destructively.
    pub fn cancelSummaryForDestructiveMutation(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        chatId: String,
    ) {
        self.cancelSummaryForChat(enhancedAiService, chatId);
    }

    /// Installs UI bridge callbacks used by platform integrations.
    pub fn setUiBridge(&mut self) {}
}
