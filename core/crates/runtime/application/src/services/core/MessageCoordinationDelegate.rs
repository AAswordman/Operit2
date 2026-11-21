use futures_util::future::{AbortHandle, Abortable, BoxFuture, Shared};
use futures_util::FutureExt;
use operit_store::ChatExecutionLease::ChatExecutionLease;
use operit_util::stream::Stream::Stream;
use std::sync::Arc;
use tokio::sync::oneshot;

use crate::core::chat::AIMessageManager::{AIMessageManager, StableContextWindowRequest};
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
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatMessageTimestampAllocator::ChatMessageTimestampAllocator;
use operit_model::ChatMessageDisplayMode::ChatMessageDisplayMode;
use operit_model::ChatTurnOptions::ChatTurnOptions;
use operit_model::FunctionType::FunctionType;
use operit_model::InputProcessingState::InputProcessingState;
use operit_model::MessagePart::MessagePart;
use operit_model::MessagePartCodec::MessagePartCodec;
use operit_model::ModelConfigData::ModelSummarySettings;
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_providers::chat::EnhancedAIService::{EnhancedAIService, SendMessageOptions};
use operit_providers::chat::enhance::ConversationService::{SummaryTriggerContext, SummaryTriggerPoint};
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
    configuration: ChatConfigurationResult,
    participantId: Option<String>,
    promptFunctionType: PromptFunctionType,
    chatProviderIdOverride: Option<String>,
    chatModelIdOverride: Option<String>,
    autoContinue: bool,
    trigger: Option<SummaryTriggerContext>,
}

/// Separates optional history scheduling from the token guard needed during every persisted tool loop.
struct SummarySendPolicy {
    tokenUsageThreshold: f64,
    checkBeforeSend: bool,
}

impl SummarySendPolicy {
    fn new(
        settings: &ModelSummarySettings,
        turnOptions: &ChatTurnOptions,
        checkBeforeSend: bool,
        isBackgroundSend: bool,
    ) -> Self {
        let enabled = settings.enableSummary && turnOptions.persistTurn;
        Self {
            tokenUsageThreshold: if enabled {
                settings.tokenThresholdRatio()
            } else {
                f64::MAX
            },
            checkBeforeSend: enabled
                && checkBeforeSend
                && !isBackgroundSend
                && !turnOptions.deferSummary,
        }
    }
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
        if self.messageProcessingDelegate.isChatProcessing(&chatId) {
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
        let chatContextSettings = self
            .messageProcessingDelegate
            .modelConfigManager
            .getResolvedModelConfig(
                &configuration.profile.modelBinding.providerId,
                &configuration.profile.modelBinding.modelId,
            )
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
                enableThinking: true,
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
        // Match normal sends: accept the live preview now and collect outside Holder.
        let mut history = self.chatHistoryDelegate.clone_for_core();
        defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
            "message-variant-collection",
            Box::new(move || Box::pin(async move {
                let mut content = String::new();
                contentStream
                    .collect(&mut |chunk| {
                        content.push_str(&chunk);
                    })
                    .await;
                variantMessage.parts = MessagePartCodec::parseAssistantMarkup(&content)
                    .expect("regenerated assistant markup must parse into message parts");
                variantMessage.isVariantPreview = false;
                history.addMessageVariant(targetMessage.timestamp, variantMessage, Some(chatId));
            })),
        ).map_err(|error| error.to_string())?;
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
                    .createNewChat(enhancedAiService, true, None, None, None, None)
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
            if self.chatHistoryDelegate.chatHistoryManager.hasUserMessageAfter(&chatId, source.timestamp)
                .map_err(|error| error.to_string())? {
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
            &turnOptions,
            !isContinuation && !isAutoContinuation && !isResume,
            isBackgroundSend,
        ).await?;
        let runtimeChatHistory = match chatHistoryOverride {
            Some(history) => history,
            None => self
                .chatHistoryDelegate
                .getRuntimeChatHistory(chatId.clone()),
        };
        // Chats think at the level set on their model.
        let enableThinking = true;
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
        // Usage is committed with the terminal assistant segment. Dispatch returns before
        // streaming settles and must not overwrite persisted totals from a previous turn.
        ChainLogger::verbose_with_level(
            SEND_CHAIN,
            "send.dispatch.accepted",
            &[("chatId", chatId.clone())],
            operit_util::AppLogger::VERBOSE_LEVEL_2,
        );
        Ok(result.completion)
    }

    /// Starts snapshot compaction without delaying dispatch and retains the tool-round token guard.
    #[allow(clippy::too_many_arguments)]
    async fn prepareSummaryForSend(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        configuration: &ChatConfigurationResult,
        chatId: &str,
        participantId: Option<String>,
        promptFunctionType: PromptFunctionType,
        turnOptions: &ChatTurnOptions,
        checkBeforeSend: bool,
        isBackgroundSend: bool,
    ) -> Result<(i32, f64), String> {
        let providerId = configuration.profile.modelBinding.providerId.clone();
        let modelId = configuration.profile.modelBinding.modelId.clone();
        let settings = self
            .messageProcessingDelegate
            .modelConfigManager
            .getResolvedModelConfig(&providerId, &modelId)
            .map_err(|error| error.to_string())?;
        let maxTokens = settings.context.maxContextTokens();
        let policy = SummarySendPolicy::new(
            &settings.summary, turnOptions, checkBeforeSend, isBackgroundSend,
        );
        if policy.checkBeforeSend
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
                    chat.currentWindowSize,
                    maxTokens,
                    policy.tokenUsageThreshold,
                    true,
                )
            {
                // The cached chats-table window can lag behind a completed compaction, so
                // revalidate with the fresh stable window before committing to an LLM call.
                // If estimation is unavailable, keep the cached decision; the worker still
                // revalidates before executing.
                let currentTokens = match self
                    .recalculateStableWindowSize(
                        enhancedAiService,
                        chatId.to_string(),
                        participantId.clone(),
                        promptFunctionType.clone(),
                        Some(providerId.clone()),
                        Some(modelId.clone()),
                    )
                    .await
                {
                    Ok(window) => window,
                    Err(error) => {
                        ChainLogger::warn(
                            SEND_CHAIN,
                            "summary.presend.recalculate.error",
                            &[("chatId", chatId.to_string()), ("error", error)],
                        );
                        chat.currentWindowSize
                    }
                };
                let trigger = SummaryTriggerContext {
                    current_tokens: currentTokens,
                    max_tokens: maxTokens,
                    token_usage_threshold: policy.tokenUsageThreshold,
                    trigger_point: SummaryTriggerPoint::BeforeSend,
                };
                if Self::summaryExecutionStillRequired(Some(trigger), currentTokens) {
                    Self::resolveSummaryLaunchConflict(
                        self.launchSummary(
                            enhancedAiService,
                            messages,
                            ConversationSummaryRequest {
                                chatId: chatId.to_string(),
                                configuration: configuration.clone(),
                                participantId,
                                promptFunctionType,
                                chatProviderIdOverride: Some(providerId),
                                chatModelIdOverride: Some(modelId),
                                autoContinue: false,
                                trigger: Some(trigger),
                            },
                        ),
                        self.messageProcessingDelegate.summaryTaskForChat(chatId),
                    )?;
                }
            }
        }
        Ok((maxTokens, policy.tokenUsageThreshold))
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
                window, settings.context.maxContextTokens(),
                settings.summary.tokenThresholdRatio(), settings.summary.enableSummary,
            ) {
                return self.launchSummary(service, messages, ConversationSummaryRequest {
                    chatId, participantId: Some(configuration.profile.id.clone()),
                    configuration: configuration.clone(), promptFunctionType: PromptFunctionType::CHAT,
                    chatProviderIdOverride: None, chatModelIdOverride: None,
                    autoContinue: false,
                    trigger: Some(SummaryTriggerContext {
                        current_tokens: window,
                        max_tokens: settings.context.maxContextTokens(),
                        token_usage_threshold: settings.summary.tokenThresholdRatio(),
                        trigger_point: SummaryTriggerPoint::AfterReply,
                    }),
                })?.await;
            }
        }
        Ok(())
    }

    /// Decides whether a captured summary request must still run against a freshly measured
    /// window. Automatic threshold triggers revalidate; manual summaries and token-limit
    /// recovery continuations carry no trigger and always execute.
    fn summaryExecutionStillRequired(
        trigger: Option<SummaryTriggerContext>,
        currentWindow: i64,
    ) -> bool {
        match trigger {
            Some(trigger) => AIMessageManager::shouldGenerateSummary(
                currentWindow,
                trigger.max_tokens,
                trigger.token_usage_threshold,
                true,
            ),
            None => true,
        }
    }

    /// Resolves a summary launch result for the send path. A registration conflict means
    /// another summary already owns the chat, so the send continues without a second
    /// compaction; scheduler or other launch failures without an owner remain fatal.
    fn resolveSummaryLaunchConflict(
        launch: Result<Shared<BoxFuture<'static, Result<(), String>>>, String>,
        registeredTask: Option<ChatSummaryTask>,
    ) -> Result<(), String> {
        match launch {
            Ok(_completion) => Ok(()),
            Err(error) => {
                if registeredTask.is_some() {
                    Ok(())
                } else {
                    Err(error)
                }
            }
        }
    }

    /// Reports a summary failure directly to the consumable UI toast flow.
    fn reportSummaryFailure(processing: &mut MessageProcessingDelegate, error: &str) {
        AppLogger::e(
            "MessageCoordinationDelegate",
            &format!("Conversation summary failed: {error}"),
        );
        processing.showToast(format!("上下文压缩失败：{error}"));
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
                    let progressDelegate = std::sync::Mutex::new(worker.messageProcessingDelegate.clone_for_core());
                    let progressChatId = workerChatId.clone();
                    let progressTaskId = workerTaskId.clone();
                    let progress: operit_providers::chat::enhance::ConversationService::SummaryProgressCallback = Arc::new(move |stage| {
                        let mut delegate = progressDelegate.lock().expect("summary progress mutex poisoned");
                        let ownsTask = delegate.summaryTaskForChat(&progressChatId)
                            .is_some_and(|task| Arc::ptr_eq(&task.taskId, &progressTaskId));
                        if ownsTask {
                            delegate.setInputProcessingStateForChat(progressChatId.clone(),
                                InputProcessingState::Summarizing { message: stage.to_string() });
                        }
                    });
                    let errorDelegate = std::sync::Mutex::new(worker.messageProcessingDelegate.clone_for_core());
                    let errorChatId = workerChatId.clone();
                    let errorTaskId = workerTaskId.clone();
                    let onNonFatalError = Arc::new(move |error: String| {
                        let delegate = errorDelegate.lock().expect("summary error mutex poisoned");
                        let ownsTask = delegate.summaryTaskForChat(&errorChatId)
                            .is_some_and(|task| Arc::ptr_eq(&task.taskId, &errorTaskId));
                        if ownsTask {
                            delegate.toastEventFlow.set_value(Some(format!("上下文压缩：{error}")));
                        }
                    });
                    let outcome = Abortable::new(
                        worker.generateAndStoreSummary(&mut service, messages, request, Some(progress), onNonFatalError),
                        registration,
                    )
                    .await;
                    let result = match outcome {
                        Ok(result) => {
                            if let Err(error) = &result {
                                Self::reportSummaryFailure(
                                    &mut worker.messageProcessingDelegate,
                                    error,
                                );
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
            let error = error.to_string();
            Self::reportSummaryFailure(&mut cleanup, &error);
            return Err(error);
        }
        Ok(completion)
    }

    /// Summarizes the captured history and persists it between its original timestamp anchors.
    async fn generateAndStoreSummary(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        messages: Vec<ChatMessage>,
        request: ConversationSummaryRequest,
        progress: Option<operit_providers::chat::enhance::ConversationService::SummaryProgressCallback>,
        onNonFatalError: Arc<dyn Fn(String) + Send + Sync>,
    ) -> Result<(), String> {
        // A queued automatic summary may have been launched from a stale cached window while
        // an earlier compaction already ran. Revalidate against the fresh stable window before
        // paying for another LLM request; the launch wrapper's finishSummaryTask clears the
        // compressing UI when this skips with Ok(()). Manual and token-limit recovery requests
        // carry no trigger and always execute.
        let mut trigger = request.trigger;
        if let Some(captured) = trigger {
            let window = self
                .recalculateStableWindowSize(
                    enhancedAiService,
                    request.chatId.clone(),
                    request.participantId.clone(),
                    request.promptFunctionType.clone(),
                    request.chatProviderIdOverride.clone(),
                    request.chatModelIdOverride.clone(),
                )
                .await?;
            if !Self::summaryExecutionStillRequired(Some(captured), window) {
                ChainLogger::info(
                    SEND_CHAIN,
                    "summary.execution.skipped",
                    &[
                        ("chatId", request.chatId.clone()),
                        ("window", window.to_string()),
                        ("maxTokens", captured.max_tokens.to_string()),
                        ("capturedTokens", captured.current_tokens.to_string()),
                    ],
                );
                return Ok(());
            }
            trigger = Some(SummaryTriggerContext {
                current_tokens: window,
                ..captured
            });
        }
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
            AIMessageManager::summarizeMemory(
                enhancedAiService, messages, request.autoContinue, request.chatId.clone(), &request.configuration, progress, trigger, onNonFatalError,
            )
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "No valid conversation summary was generated".to_string())?;
        // Persist for the originating chat even when the user switched away mid-generation;
        // addSummaryMessage and refreshStableContextWindow are both scoped to the given chat.
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

    /// Starts a user-requested summary for one explicit chat and waits for its host-owned task.
    #[allow(non_snake_case)]
    pub async fn manuallySummarizeConversationForChat(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        chatId: String,
    ) -> bool {
        if self
            .messageProcessingDelegate
            .summaryTaskForChat(&chatId)
            .is_some()
        {
            return false;
        }
        let configuration = match enhancedAiService.resolveChatConfigurationForOptions(&SendMessageOptions {
            chatId: Some(chatId.clone()),
            promptFunctionType: self.currentPromptFunctionType.clone(),
            chatProviderIdOverride: self.currentChatProviderIdOverride.clone(),
            chatModelIdOverride: self.currentChatModelIdOverride.clone(),
            ..SendMessageOptions::new()
        }).await {
            Ok(configuration) => configuration,
            Err(error) => {
                Self::reportSummaryFailure(&mut self.messageProcessingDelegate, &error.to_string());
                return false;
            }
        };
        let messages = self
            .chatHistoryDelegate
            .getRuntimeChatHistory(chatId.clone());
        let result = match self.launchSummary(
            enhancedAiService,
            messages,
            ConversationSummaryRequest {
                chatId,
                participantId: Some(configuration.profile.id.clone()),
                configuration,
                promptFunctionType: self.currentPromptFunctionType.clone(),
                chatProviderIdOverride: self.currentChatProviderIdOverride.clone(),
                chatModelIdOverride: self.currentChatModelIdOverride.clone(),
                autoContinue: false,
                trigger: None,
            },
        ) {
            Ok(completion) => completion.await,
            Err(error) => Err(error),
        };
        result.is_ok()
    }

    /// Starts a user-requested summary and waits for its host-owned task to finish.
    pub async fn manuallySummarizeConversation(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
    ) {
        let Some(chatId) = self.chatHistoryDelegate.currentChatIdFlow.value() else {
            return;
        };
        self.manuallySummarizeConversationForChat(enhancedAiService, chatId)
            .await;
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
        let configuration = enhancedAiService.resolveChatConfigurationForOptions(&SendMessageOptions {
            chatId: Some(request.chatId.clone()),
            executionParticipantId: request.participantId.clone(),
            promptFunctionType: request.promptFunctionType.clone(),
            chatProviderIdOverride: request.chatProviderIdOverride.clone(),
            chatModelIdOverride: request.chatModelIdOverride.clone(),
            ..SendMessageOptions::new()
        }).await.map_err(|error| error.to_string())?;
        let messages = self
            .chatHistoryDelegate
            .getRuntimeChatHistory(request.chatId.clone());
        self.launchSummary(
            enhancedAiService,
            messages,
            ConversationSummaryRequest {
                chatId: request.chatId.clone(),
                configuration,
                participantId: request.participantId.clone(),
                promptFunctionType: request.promptFunctionType.clone(),
                chatProviderIdOverride: request.chatProviderIdOverride.clone(),
                chatModelIdOverride: request.chatModelIdOverride.clone(),
                autoContinue: true,
                trigger: None,
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
}

#[cfg(test)]
mod summary_send_policy_tests {
    use super::*;
    use operit_model::ChatTurnOptions::ChatTurnContinuation;
    use operit_model::ModelConfigData::ModelContextSpec;

    /// A continued or background tool loop must still compact at 98% with the default 85% threshold.
    #[test]
    fn continued_and_background_tool_loops_keep_the_configured_token_guard() {
        let settings = ModelSummarySettings::default();
        let maxTokens = ModelContextSpec::default().maxContextTokens();
        for deferred in [false, true] {
            for background in [false, true] {
                let options = ChatTurnOptions {
                    continuation: Some(ChatTurnContinuation {
                        userMessageTimestamp: 1,
                    }),
                    deferSummary: deferred,
                    ..Default::default()
                };
                let policy = SummarySendPolicy::new(&settings, &options, false, background);
                assert!(!policy.checkBeforeSend);
                assert!(AIMessageManager::shouldGenerateSummary(
                    i64::from(maxTokens) * 98 / 100,
                    maxTokens,
                    policy.tokenUsageThreshold,
                    true,
                ));
            }
        }
    }

    /// Scheduling a snapshot must leave the current tool loop protected at the configured threshold.
    #[test]
    fn proactive_summary_keeps_the_same_token_threshold_as_tool_continuations() {
        let settings = ModelSummarySettings::default();
        let initial = SummarySendPolicy::new(&settings, &ChatTurnOptions::default(), true, false);
        let continued = SummarySendPolicy::new(
            &settings,
            &ChatTurnOptions {
                deferSummary: true,
                ..Default::default()
            },
            false,
            false,
        );
        assert!(initial.checkBeforeSend);
        assert!(!continued.checkBeforeSend);
        assert_eq!(initial.tokenUsageThreshold, continued.tokenUsageThreshold);
        assert!(initial.tokenUsageThreshold <= 1.0);
    }

    /// Explicit summary opt-out and ephemeral turns must continue without automatic compaction.
    #[test]
    fn disabled_summaries_and_ephemeral_turns_disable_both_checks() {
        for (enabled, persisted) in [(false, true), (true, false)] {
            let settings = ModelSummarySettings {
                enableSummary: enabled,
                ..Default::default()
            };
            let options = ChatTurnOptions {
                persistTurn: persisted,
                ..Default::default()
            };
            let policy = SummarySendPolicy::new(&settings, &options, true, false);
            assert!(!policy.checkBeforeSend);
            assert!(!AIMessageManager::shouldGenerateSummary(
                196_000,
                200_000,
                policy.tokenUsageThreshold,
                true,
            ));
        }
    }
}

#[cfg(test)]
mod summary_execution_tests {
    use super::*;
    use futures_util::future::ready;

    /// Reproduces the device timeline: the second summary launched from a stale cached
    /// window of 115529 tokens while the first compaction had already reduced the real
    /// window to 15088 tokens against a 200000-token model at the default 0.85 threshold.
    fn stale_before_send_trigger() -> SummaryTriggerContext {
        SummaryTriggerContext {
            current_tokens: 115_529,
            max_tokens: 200_000,
            token_usage_threshold: 0.85,
            trigger_point: SummaryTriggerPoint::BeforeSend,
        }
    }

    /// A queued automatic summary must skip the LLM call once the fresh window is compacted.
    #[test]
    fn queued_automatic_summary_revalidates_the_fresh_window() {
        let trigger = stale_before_send_trigger();
        assert!(
            !MessageCoordinationDelegate::summaryExecutionStillRequired(Some(trigger), 15_088),
            "a compacted window must not launch a second compression",
        );
        assert!(
            !MessageCoordinationDelegate::summaryExecutionStillRequired(Some(trigger), 169_999),
            "the threshold boundary stays exclusive below 85 percent",
        );
        assert!(
            MessageCoordinationDelegate::summaryExecutionStillRequired(Some(trigger), 170_000),
            "a genuinely full window must still compress",
        );
        let after_reply = SummaryTriggerContext {
            trigger_point: SummaryTriggerPoint::AfterReply,
            ..trigger
        };
        assert!(
            !MessageCoordinationDelegate::summaryExecutionStillRequired(Some(after_reply), 15_088),
        );
    }

    /// Manual summaries and token-limit recovery are explicit requests, not threshold guesses.
    #[test]
    fn manual_and_recovery_summaries_never_skip() {
        for window in [0, 15_088, 115_529] {
            assert!(
                MessageCoordinationDelegate::summaryExecutionStillRequired(None, window),
                "trigger-free requests must run at any window size",
            );
        }
    }

    /// Builds an identifiable registered summary without a platform executor.
    fn registeredTask() -> ChatSummaryTask {
        let (abortHandle, _) = AbortHandle::new_pair();
        ChatSummaryTask {
            taskId: Arc::new(()),
            abortHandle,
            completion: ready(Ok(())).boxed().shared(),
        }
    }

    /// A concurrent summary owns the chat, so the send continues; real launch errors surface.
    #[test]
    fn overlap_launch_conflict_skips_but_other_errors_propagate() {
        assert!(
            MessageCoordinationDelegate::resolveSummaryLaunchConflict(
                Err("Summary task already registered for chat".to_string()),
                Some(registeredTask()),
            )
            .is_ok(),
            "an already-compressing chat must not fail the user's send",
        );
        assert_eq!(
            MessageCoordinationDelegate::resolveSummaryLaunchConflict(
                Err("scheduler unavailable".to_string()),
                None,
            )
            .unwrap_err(),
            "scheduler unavailable",
            "launch failures without a competing summary must stay fatal",
        );
        assert!(
            MessageCoordinationDelegate::resolveSummaryLaunchConflict(
                Ok(ready(Ok(())).boxed().shared()),
                None,
            )
            .is_ok(),
        );
    }
}
