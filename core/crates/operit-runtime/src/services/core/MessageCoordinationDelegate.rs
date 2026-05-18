use std::collections::HashMap;

use crate::api::chat::EnhancedAIService::EnhancedAIService;
use crate::core::chat::AIMessageManager::AIMessageManager;
use crate::data::model::ChatMessage::ChatMessage;
use crate::data::model::ChatTurnOptions::ChatTurnOptions;
use crate::data::model::InputProcessingState::InputProcessingState;
use crate::data::model::PromptFunctionType::PromptFunctionType;
use crate::services::core::ChatHistoryDelegate::ChatHistoryDelegate;
use crate::services::core::MessageProcessingDelegate::{
    BuildUserMessageContentForSendRequest, MessageProcessingDelegate,
    RegenerateAiMessageVariantRequest, SendUserMessageProcessingRequest,
};
use crate::services::core::TokenStatisticsDelegate::TokenStatisticsDelegate;

#[derive(Clone, Debug, PartialEq)]
pub struct PendingAutoContinuationRequest {
    pub chatId: String,
    pub promptFunctionType: PromptFunctionType,
    pub chatModelConfigIdOverride: Option<String>,
    pub chatModelIndexOverride: Option<i32>,
    pub preferenceProfileIdOverride: Option<String>,
    pub roleCardIdOverride: Option<String>,
    pub isGroupOrchestrationTurn: bool,
    pub groupParticipantNamesText: Option<String>,
    pub waitJob: Option<String>,
}

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
    pub currentChatModelConfigIdOverride: Option<String>,
    pub currentChatModelIndexOverride: Option<i32>,
    pub currentPreferenceProfileIdOverride: Option<String>,
    pub nonFatalErrorCollectorJob: Option<String>,
    pub pendingAutoContinuationByChatId: HashMap<String, PendingAutoContinuationRequest>,
}

impl MessageCoordinationDelegate {
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
            currentChatModelConfigIdOverride: None,
            currentChatModelIndexOverride: None,
            currentPreferenceProfileIdOverride: None,
            nonFatalErrorCollectorJob: None,
            pendingAutoContinuationByChatId: HashMap::new(),
        };
        delegate.ensureNonFatalErrorCollectorStarted();
        delegate
    }

    fn ensureNonFatalErrorCollectorStarted(&mut self) {
        if self.nonFatalErrorCollectorJob.is_some() {
            return;
        }
        self.nonFatalErrorCollectorJob = Some("nonFatalErrorCollectorJob".to_string());
    }

    pub async fn recalculateStableWindowSize(
        &mut self,
        service: &mut EnhancedAIService,
        chatId: Option<String>,
        roleCardId: Option<String>,
        promptFunctionType: PromptFunctionType,
        groupOrchestrationMode: bool,
        groupParticipantNamesText: Option<String>,
        chatModelConfigIdOverride: Option<String>,
        chatModelIndexOverride: Option<i32>,
        preferenceProfileIdOverride: Option<String>,
    ) -> i32 {
        let workspacePath = chatId.as_ref().and_then(|chatId| {
            self.chatHistoryDelegate
                .chatHistories
                .iter()
                .find(|history| history.id == *chatId)
                .and_then(|history| history.workspace.clone())
        });
        let workspaceEnv = chatId.as_ref().and_then(|chatId| {
            self.chatHistoryDelegate
                .chatHistories
                .iter()
                .find(|history| history.id == *chatId)
                .and_then(|history| history.workspaceEnv.clone())
        });
        let _ = (
            service,
            workspacePath,
            workspaceEnv,
            promptFunctionType,
            roleCardId,
            groupOrchestrationMode,
            groupParticipantNamesText,
            chatModelConfigIdOverride,
            chatModelIndexOverride,
            preferenceProfileIdOverride,
        );
        chatId
            .map(|id| self.chatHistoryDelegate.getRuntimeChatHistory(id).len() as i32)
            .unwrap_or(0)
    }

    pub async fn refreshStableContextWindow(
        &mut self,
        service: &mut EnhancedAIService,
        chatId: Option<String>,
        roleCardId: Option<String>,
        promptFunctionType: Option<PromptFunctionType>,
        groupOrchestrationMode: bool,
        groupParticipantNamesText: Option<String>,
        chatModelConfigIdOverride: Option<String>,
        chatModelIndexOverride: Option<i32>,
        preferenceProfileIdOverride: Option<String>,
    ) -> Option<i32> {
        let targetChatId = chatId.or_else(|| self.chatHistoryDelegate.currentChatId.clone())?;
        let effectivePromptFunctionType =
            promptFunctionType.unwrap_or_else(|| self.currentPromptFunctionType.clone());
        let effectiveChatModelConfigIdOverride =
            chatModelConfigIdOverride.or_else(|| self.currentChatModelConfigIdOverride.clone());
        let effectiveChatModelIndexOverride =
            chatModelIndexOverride.or(self.currentChatModelIndexOverride);
        let effectivePreferenceProfileIdOverride =
            preferenceProfileIdOverride.or_else(|| self.currentPreferenceProfileIdOverride.clone());
        let newWindowSize = self.recalculateStableWindowSize(
            service,
            Some(targetChatId.clone()),
            roleCardId,
            effectivePromptFunctionType,
            groupOrchestrationMode,
            groupParticipantNamesText,
            effectiveChatModelConfigIdOverride,
            effectiveChatModelIndexOverride,
            effectivePreferenceProfileIdOverride,
        )
        .await;
        let (inputTokens, outputTokens) = self
            .tokenStatisticsDelegate
            .getCumulativeTokenCounts(Some(targetChatId.clone()));
        self.chatHistoryDelegate.saveCurrentChat(
            inputTokens,
            outputTokens,
            newWindowSize,
            Some(targetChatId.clone()),
        );
        Some(newWindowSize)
    }

    pub async fn sendUserMessage(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        promptFunctionType: PromptFunctionType,
        roleCardIdOverride: Option<String>,
        chatIdOverride: Option<String>,
        messageTextOverride: Option<String>,
        proxySenderNameOverride: Option<String>,
        chatModelConfigIdOverride: Option<String>,
        chatModelIndexOverride: Option<i32>,
        turnOptions: ChatTurnOptions,
    ) {
        if chatIdOverride.as_ref().map(|id| id.trim().is_empty()).unwrap_or(true)
            && self.chatHistoryDelegate.currentChatId.is_none()
        {
            self.chatHistoryDelegate
                .createNewChat(None, None, None, true, true, None);
        }
        self.sendMessageInternal(
            enhancedAiService,
            promptFunctionType,
            false,
            false,
            roleCardIdOverride,
            chatIdOverride,
            messageTextOverride,
            proxySenderNameOverride,
            chatModelConfigIdOverride,
            chatModelIndexOverride,
            None,
            false,
            None,
            turnOptions,
        )
        .await;
    }

    pub async fn regenerateSingleAiMessage(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        index: usize,
    ) -> Result<(), String> {
        let chatId = self
            .chatHistoryDelegate
            .currentChatId
            .clone()
            .ok_or_else(|| "No active conversation".to_string())?;
        if self.messageProcessingDelegate.isChatLoading(chatId.clone()) {
            return Err("Chat is busy".to_string());
        }
        let currentHistory = self.chatHistoryDelegate.chatHistory.clone();
        let targetMessage = currentHistory
            .get(index)
            .cloned()
            .ok_or_else(|| "Invalid message index".to_string())?;
        if targetMessage.sender != "ai" {
            return Err("Only AI message allowed".to_string());
        }
        let prefixHistory = currentHistory[..index].to_vec();
        let (requestHistory, requestMessageContent) =
            if prefixHistory.last().map(|message| message.sender.as_str()) == Some("user") {
                (
                    prefixHistory[..prefixHistory.len() - 1].to_vec(),
                    prefixHistory
                        .last()
                        .map(|message| message.content.clone())
                        .unwrap_or_default(),
                )
            } else {
                (prefixHistory, String::new())
            };
        let currentChat = self
            .chatHistoryDelegate
            .chatHistories
            .iter()
            .find(|history| history.id == chatId)
            .cloned();
        let workspacePath = currentChat.and_then(|chat| chat.workspace);
        let _ = self
            .messageProcessingDelegate
            .regenerateAiMessageVariant(RegenerateAiMessageVariantRequest {
                enhancedAiService,
                chatId,
                targetMessageTimestamp: targetMessage.timestamp,
                requestMessageContent,
                requestHistory,
                workspacePath,
                promptFunctionType: self.currentPromptFunctionType.clone(),
                roleCardId: String::new(),
                currentRoleName: targetMessage.roleName,
                enableThinking: false,
                enableMemoryAutoUpdate: false,
                maxTokens: 0,
                tokenUsageThreshold: 0.0,
                chatModelConfigIdOverride: None,
                chatModelIndexOverride: None,
                preferenceProfileIdOverride: None,
            })
            .await;
        Ok(())
    }

    pub async fn sendMessageInternal(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        promptFunctionType: PromptFunctionType,
        isContinuation: bool,
        isAutoContinuation: bool,
        roleCardIdOverride: Option<String>,
        chatIdOverride: Option<String>,
        messageTextOverride: Option<String>,
        proxySenderNameOverride: Option<String>,
        chatModelConfigIdOverride: Option<String>,
        chatModelIndexOverride: Option<i32>,
        preferenceProfileIdOverride: Option<String>,
        isGroupOrchestrationTurn: bool,
        groupParticipantNamesText: Option<String>,
        turnOptions: ChatTurnOptions,
    ) {
        self.currentPromptFunctionType = promptFunctionType.clone();
        self.currentChatModelConfigIdOverride = chatModelConfigIdOverride.clone();
        self.currentChatModelIndexOverride = chatModelIndexOverride;
        self.currentPreferenceProfileIdOverride = preferenceProfileIdOverride.clone();
        let chatId = chatIdOverride
            .or_else(|| self.chatHistoryDelegate.currentChatId.clone())
            .unwrap_or_else(|| {
                self.chatHistoryDelegate
                    .createNewChat(None, None, None, true, true, None);
                self.chatHistoryDelegate
                    .currentChatId
                    .clone()
                    .unwrap_or_default()
            });
        let messageText = messageTextOverride.unwrap_or_else(|| {
            self.messageProcessingDelegate.userMessage.text.clone()
        });
        let currentChat = self
            .chatHistoryDelegate
            .chatHistories
            .iter()
            .find(|history| history.id == chatId)
            .cloned();
        let workspacePath = currentChat.clone().and_then(|chat| chat.workspace);
        let workspaceEnv = currentChat.and_then(|chat| chat.workspaceEnv);
        let roleCardId = roleCardIdOverride.clone().unwrap_or_default();
        let messageContent = self
            .messageProcessingDelegate
            .buildUserMessageContentForSend(BuildUserMessageContentForSendRequest {
                messageText,
                proxySenderNameOverride,
                attachments: Vec::new(),
                workspacePath: workspacePath.clone(),
                workspaceEnv: workspaceEnv.clone(),
                replyToMessage: None,
                chatId: chatId.clone(),
                roleCardId: roleCardId.clone(),
                chatModelConfigIdOverride: chatModelConfigIdOverride.clone(),
            })
            .unwrap_or_default();
        if !isContinuation {
            self.chatHistoryDelegate.addMessageToChat(
                ChatMessage::new_with_content("user".to_string(), messageContent.clone()),
                Some(chatId.clone()),
            );
        }
        let result = self
            .messageProcessingDelegate
            .sendUserMessage(SendUserMessageProcessingRequest {
                enhancedAiService,
                chatId: chatId.clone(),
                messageContent,
                chatHistory: self.chatHistoryDelegate.getRuntimeChatHistory(chatId.clone()),
                workspacePath,
                workspaceEnv,
                promptFunctionType,
                roleCardId,
                currentRoleName: None,
                characterName: None,
                avatarUri: None,
                enableThinking: false,
                enableMemoryAutoUpdate: false,
                maxTokens: 0,
                tokenUsageThreshold: 0.0,
                chatModelConfigIdOverride,
                chatModelIndexOverride,
                preferenceProfileIdOverride,
                isGroupOrchestrationTurn,
                groupParticipantNamesText,
                proxySenderNameOverride: None,
                turnOptions: turnOptions.clone(),
            })
            .await;
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                self.messageProcessingDelegate.setInputProcessingStateForChat(
                    chatId.clone(),
                    InputProcessingState::Error {
                        message: error.to_string(),
                    },
                );
                return;
            }
        };
        self.chatHistoryDelegate
            .addMessageToChat(result.aiMessage.clone(), Some(chatId.clone()));
        self.tokenStatisticsDelegate
            .updateCumulativeStatistics(Some(chatId.clone()), Some(enhancedAiService));
        let (inputTokens, outputTokens) = self
            .tokenStatisticsDelegate
            .getCumulativeTokenCounts(Some(chatId.clone()));
        let windowSize = result
            .nextWindowSize
            .unwrap_or_else(|| self.tokenStatisticsDelegate.getLastCurrentWindowSize(Some(chatId.clone())));
        self.tokenStatisticsDelegate
            .setTokenCounts(Some(chatId.clone()), inputTokens, outputTokens, windowSize);
        if turnOptions.persistTurn {
            self.chatHistoryDelegate
                .saveCurrentChat(inputTokens, outputTokens, windowSize, Some(chatId.clone()));
        }
        if isAutoContinuation {
            self.removePendingAutoContinuation(chatId);
        }
    }

    pub fn handleManualMemoryUpdate(&mut self, _chatId: Option<String>) {
        if self.isUpdatingMemory {
            return;
        }
        self.isUpdatingMemory = true;
        self.isUpdatingMemory = false;
    }

    pub async fn manuallySummarizeConversation(&mut self, enhancedAiService: &mut EnhancedAIService) {
        if self.isSummarizing {
            return;
        }
        let currentChatId = self.chatHistoryDelegate.currentChatId.clone();
        self.summarizeHistory(
            enhancedAiService,
            false,
            None,
            currentChatId,
            None,
            None,
            None,
            None,
            false,
            false,
            None,
        )
        .await;
    }

    pub async fn handleTokenLimitExceeded(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        chatId: Option<String>,
        roleCardId: Option<String>,
        isGroupOrchestrationTurn: bool,
        groupParticipantNamesText: Option<String>,
    ) {
        self.summaryJob = Some("summaryJob".to_string());
        self.summarizeHistory(
            enhancedAiService,
            true,
            None,
            chatId,
            None,
            None,
            None,
            roleCardId,
            false,
            isGroupOrchestrationTurn,
            groupParticipantNamesText,
        )
        .await;
        self.summaryJob = None;
    }

    fn cancelSummaryStreamingInternal(&mut self, _enhancedAiService: &mut EnhancedAIService) {
    }

    fn cancelSummaryInternal(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        targetChatId: Option<String>,
    ) {
        let currentChatId = targetChatId
            .clone()
            .or_else(|| self.chatHistoryDelegate.currentChatId.clone());
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
        if !shouldCancelSummary && !shouldCancelAsyncSummary && !shouldCancelPendingAutoContinuation {
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
        self.messageProcessingDelegate.refreshGlobalLoadingState();
    }

    pub fn cancelSummary(&mut self, enhancedAiService: &mut EnhancedAIService) {
        self.cancelSummaryInternal(enhancedAiService, None);
    }

    pub fn cancelSummaryForChat(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        chatId: String,
    ) {
        self.cancelSummaryInternal(enhancedAiService, Some(chatId));
    }

    pub fn cancelSummaryForDestructiveMutation(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        chatId: String,
    ) {
        self.cancelSummaryInternal(enhancedAiService, Some(chatId));
    }

    async fn launchAsyncSummaryForSend(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        snapshotMessages: Vec<ChatMessage>,
        beforeTimestamp: Option<i64>,
        afterTimestamp: Option<i64>,
        originalChatId: Option<String>,
        roleCardId: Option<String>,
        chatModelConfigIdOverride: Option<String>,
        chatModelIndexOverride: Option<i32>,
        preferenceProfileIdOverride: Option<String>,
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
        self.messageProcessingDelegate.setInputProcessingStateForChat(
            originalChatId.clone(),
            InputProcessingState::Summarizing { message: "compressing history".to_string() },
        );
        let isGroupChat = self
            .chatHistoryDelegate
            .chatHistories
            .iter()
            .find(|history| history.id == originalChatId)
            .and_then(|history| history.characterGroupId.clone())
            .is_some();
        if let Ok(Some(summaryMessage)) =
            AIMessageManager::summarizeMemory(enhancedAiService, snapshotMessages, false, isGroupChat).await
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
                roleCardId,
                None,
                false,
                None,
                chatModelConfigIdOverride,
                chatModelIndexOverride,
                preferenceProfileIdOverride,
            )
            .await;
        }
        self.isSendTriggeredSummarizing = false;
        self.sendTriggeredSummarizingChatId = None;
        self.messageProcessingDelegate
            .setPendingAsyncSummaryUiForChat(originalChatId.clone(), false);
        self.messageProcessingDelegate
            .setSuppressIdleCompletedStateForChat(originalChatId.clone(), false);
        self.messageProcessingDelegate.setInputProcessingStateForChat(
            originalChatId,
            InputProcessingState::Idle,
        );
    }

    async fn summarizeHistory(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        autoContinue: bool,
        promptFunctionType: Option<PromptFunctionType>,
        chatIdOverride: Option<String>,
        chatModelConfigIdOverride: Option<String>,
        chatModelIndexOverride: Option<i32>,
        preferenceProfileIdOverride: Option<String>,
        roleCardIdOverride: Option<String>,
        isGroupChat: bool,
        isGroupOrchestrationTurn: bool,
        groupParticipantNamesText: Option<String>,
    ) -> bool {
        if self.isSummarizing {
            return false;
        }
        self.isSummarizing = true;
        let currentChatId = chatIdOverride.or_else(|| self.chatHistoryDelegate.currentChatId.clone());
        self.summarizingChatId = currentChatId.clone();
        if let Some(currentChatId) = currentChatId.clone() {
            self.messageProcessingDelegate
                .setSuppressIdleCompletedStateForChat(currentChatId.clone(), true);
            self.messageProcessingDelegate.setInputProcessingStateForChat(
                currentChatId,
                InputProcessingState::Summarizing { message: "compressing history".to_string() },
            );
        }
        let effectiveChatModelConfigIdOverride =
            chatModelConfigIdOverride.or_else(|| self.currentChatModelConfigIdOverride.clone());
        let effectiveChatModelIndexOverride =
            chatModelIndexOverride.or(self.currentChatModelIndexOverride);
        let effectivePreferenceProfileIdOverride =
            preferenceProfileIdOverride.or_else(|| self.currentPreferenceProfileIdOverride.clone());
        let currentMessages = currentChatId
            .clone()
            .map(|chatId| self.chatHistoryDelegate.getRuntimeChatHistory(chatId))
            .unwrap_or_default();
        if currentMessages.is_empty() {
            self.isSummarizing = false;
            self.summarizingChatId = None;
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
        if let Ok(Some(summaryMessage)) =
            AIMessageManager::summarizeMemory(enhancedAiService, currentMessages, autoContinue, isGroupChat).await
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
                roleCardIdOverride.clone(),
                None,
                isGroupOrchestrationTurn,
                groupParticipantNamesText.clone(),
                effectiveChatModelConfigIdOverride.clone(),
                effectiveChatModelIndexOverride,
                effectivePreferenceProfileIdOverride.clone(),
            )
            .await;
            summarySuccess = true;
        }
        self.isSummarizing = false;
        if self.summarizingChatId == currentChatId {
            self.summarizingChatId = None;
        }
        self.messageProcessingDelegate.refreshGlobalLoadingState();
        if summarySuccess && autoContinue {
            if let Some(currentChatId) = currentChatId {
                let continuationPromptType =
                    promptFunctionType.unwrap_or_else(|| self.currentPromptFunctionType.clone());
                if self.messageProcessingDelegate.isChatLoading(currentChatId.clone()) {
                    self.queuePendingAutoContinuation(
                        currentChatId,
                        continuationPromptType,
                        effectiveChatModelConfigIdOverride,
                        effectiveChatModelIndexOverride,
                        effectivePreferenceProfileIdOverride,
                        roleCardIdOverride,
                        isGroupOrchestrationTurn,
                        groupParticipantNamesText,
                    );
                } else {
                    self.messageProcessingDelegate
                        .setSuppressIdleCompletedStateForChat(currentChatId.clone(), false);
                    self.sendMessageInternal(
                        enhancedAiService,
                        continuationPromptType,
                        true,
                        true,
                        roleCardIdOverride,
                        Some(currentChatId),
                        None,
                        None,
                        effectiveChatModelConfigIdOverride,
                        effectiveChatModelIndexOverride,
                        effectivePreferenceProfileIdOverride,
                        isGroupOrchestrationTurn,
                        groupParticipantNamesText,
                        ChatTurnOptions::default(),
                    )
                    .await;
                }
            }
        }
        summarySuccess
    }

    fn queuePendingAutoContinuation(
        &mut self,
        chatId: String,
        promptFunctionType: PromptFunctionType,
        chatModelConfigIdOverride: Option<String>,
        chatModelIndexOverride: Option<i32>,
        preferenceProfileIdOverride: Option<String>,
        roleCardIdOverride: Option<String>,
        isGroupOrchestrationTurn: bool,
        groupParticipantNamesText: Option<String>,
    ) {
        self.pendingAutoContinuationByChatId.insert(
            chatId.clone(),
            PendingAutoContinuationRequest {
                chatId,
                promptFunctionType,
                chatModelConfigIdOverride,
                chatModelIndexOverride,
                preferenceProfileIdOverride,
                roleCardIdOverride,
                isGroupOrchestrationTurn,
                groupParticipantNamesText,
                waitJob: Some("waitJob".to_string()),
            },
        );
    }

    fn removePendingAutoContinuation(&mut self, chatId: String) {
        self.pendingAutoContinuationByChatId.remove(&chatId);
    }

    pub fn setUiBridge(&mut self) {}
}
