use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::api::chat::llmprovider::AIService::collect_stream_chunks;
use crate::api::chat::EnhancedAIService::{EnhancedAIService, SendMessageOptions};
use crate::core::chat::AIMessageManager::{AIMessageManager, StableContextWindowRequest};
use crate::core::config::FunctionalPrompts::FunctionalPrompts;
use crate::data::model::ActivePrompt::ActivePrompt;
use crate::data::model::AttachmentInfo::AttachmentInfo;
use crate::data::model::CharacterCard::CharacterCard;
use crate::data::model::CharacterGroupCard::{CharacterGroupCard, GroupMemberConfig};
use crate::data::model::ChatMessage::ChatMessage;
use crate::data::model::ChatMessageDisplayMode::ChatMessageDisplayMode;
use crate::data::model::ChatTurnOptions::ChatTurnOptions;
use crate::data::model::FunctionType::FunctionType;
use crate::data::model::InputProcessingState::InputProcessingState;
use crate::data::model::PromptFunctionType::PromptFunctionType;
use crate::data::preferences::ActivePromptManager::ActivePromptManager;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::CharacterGroupCardManager::CharacterGroupCardManager;
use crate::services::core::ChatHistoryDelegate::ChatHistoryDelegate;
use crate::services::core::MessageProcessingDelegate::{
    BuildUserMessageContentForGroupOrchestrationRequest, MessageProcessingDelegate,
    RegenerateAiMessageVariantRequest, SendUserMessageProcessingRequest,
};
use crate::services::core::TokenStatisticsDelegate::TokenStatisticsDelegate;
use crate::util::stream::Stream::Stream;
use crate::util::ChainLogger::{self, SEND_CHAIN};

#[derive(Clone, Debug, PartialEq)]
pub struct PendingAutoContinuationRequest {
    pub chatId: String,
    pub promptFunctionType: PromptFunctionType,
    pub chatProviderIdOverride: Option<String>,
    pub chatModelIdOverride: Option<String>,
    pub roleCardIdOverride: Option<String>,
    pub isGroupOrchestrationTurn: bool,
    pub groupParticipantNamesText: Option<String>,
    pub waitJob: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PlannedMember {
    id: String,
    speak: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PlannedRounds {
    rounds: Vec<Vec<PlannedMember>>,
}

pub struct MessageCoordinationDelegate {
    pub chatHistoryDelegate: ChatHistoryDelegate,
    pub messageProcessingDelegate: MessageProcessingDelegate,
    pub tokenStatisticsDelegate: TokenStatisticsDelegate,
    pub characterCardManager: CharacterCardManager,
    pub characterGroupCardManager: CharacterGroupCardManager,
    pub activePromptManager: ActivePromptManager,
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
    pub fn new(
        chatHistoryDelegate: ChatHistoryDelegate,
        messageProcessingDelegate: MessageProcessingDelegate,
    ) -> Self {
        let mut delegate = Self {
            chatHistoryDelegate,
            messageProcessingDelegate,
            tokenStatisticsDelegate: TokenStatisticsDelegate::default(),
            characterCardManager: CharacterCardManager::getInstance(),
            characterGroupCardManager: CharacterGroupCardManager::getInstance(),
            activePromptManager: ActivePromptManager::getInstance(),
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

    pub async fn recalculateStableWindowSize(
        &mut self,
        service: &mut EnhancedAIService,
        chatId: Option<String>,
        roleCardId: Option<String>,
        promptFunctionType: PromptFunctionType,
        groupOrchestrationMode: bool,
        groupParticipantNamesText: Option<String>,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
    ) -> i32 {
        let currentChat = chatId.as_ref().and_then(|chatId| {
            self.chatHistoryDelegate
                .chatHistoriesFlow()
                .value()
                .into_iter()
                .find(|history| history.id == *chatId)
        });
        let currentRoleName = roleCardId.as_ref().and_then(|roleCardId| {
            self.characterCardManager
                .getCharacterCard(roleCardId)
                .ok()
                .map(|card| card.name)
        });
        let runtimeOptions = SendMessageOptions {
            roleCardId: roleCardId.clone(),
            promptFunctionType: promptFunctionType.clone(),
            chatProviderIdOverride: chatProviderIdOverride.clone(),
            chatModelIdOverride: chatModelIdOverride.clone(),
            ..SendMessageOptions::new()
        };
        let runtime = service
            .createSendMessageRuntime(&runtimeOptions)
            .expect("stable context window runtime must be created");
        AIMessageManager::calculateStableContextWindow(StableContextWindowRequest {
            enhancedAiService: service,
            chatId: chatId.clone(),
            messageContent: String::new(),
            chatHistory: chatId
                .map(|id| self.chatHistoryDelegate.getRuntimeChatHistory(id))
                .unwrap_or_default(),
            workspacePath: currentChat.clone().and_then(|chat| chat.workspace),
            promptFunctionType,
            roleCardId,
            currentRoleName,
            splitHistoryByRole: true,
            groupOrchestrationMode,
            groupParticipantNamesText,
            proxySenderName: None,
            chatProviderIdOverride,
            chatModelIdOverride,
            publishEstimate: false,
            runtime,
        })
        .await
        .expect("stable context window must be calculated")
    }

    pub async fn refreshStableContextWindow(
        &mut self,
        service: &mut EnhancedAIService,
        chatId: Option<String>,
        roleCardId: Option<String>,
        promptFunctionType: Option<PromptFunctionType>,
        groupOrchestrationMode: bool,
        groupParticipantNamesText: Option<String>,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
    ) -> Option<i32> {
        let targetChatId = chatId.or_else(|| self.chatHistoryDelegate.currentChatId.clone())?;
        let effectivePromptFunctionType =
            promptFunctionType.unwrap_or_else(|| self.currentPromptFunctionType.clone());
        let effectiveChatModelIdOverride =
            chatModelIdOverride.or_else(|| self.currentChatModelIdOverride.clone());
        let effectiveChatProviderIdOverride =
            chatProviderIdOverride.or_else(|| self.currentChatProviderIdOverride.clone());
        let newWindowSize = self
            .recalculateStableWindowSize(
                service,
                Some(targetChatId.clone()),
                roleCardId,
                effectivePromptFunctionType,
                groupOrchestrationMode,
                groupParticipantNamesText,
                effectiveChatProviderIdOverride,
                effectiveChatModelIdOverride,
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
        self.tokenStatisticsDelegate.setTokenCounts(
            Some(targetChatId.clone()),
            inputTokens,
            outputTokens,
            newWindowSize,
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
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
        attachments: Vec<AttachmentInfo>,
        replyToMessage: Option<ChatMessage>,
        turnOptions: ChatTurnOptions,
    ) {
        if chatIdOverride
            .as_ref()
            .map(|id| id.trim().is_empty())
            .unwrap_or(true)
            && self.chatHistoryDelegate.currentChatId.is_none()
        {
            self.chatHistoryDelegate
                .createNewChat(None, None, None, true, true, None);
        }
        if self.shouldRunGroupOrchestration(
            promptFunctionType.clone(),
            false,
            false,
            false,
            roleCardIdOverride.clone(),
            proxySenderNameOverride.clone(),
            messageTextOverride.clone(),
            chatIdOverride.clone(),
        ) {
            let chatId = self
                .chatHistoryDelegate
                .currentChatId
                .clone()
                .unwrap_or_else(|| {
                    self.chatHistoryDelegate
                        .createNewChat(None, None, None, true, true, None);
                    self.chatHistoryDelegate
                        .currentChatId
                        .clone()
                        .unwrap_or_default()
                });
            if self
                .orchestrateGroupConversation(
                    enhancedAiService,
                    chatId,
                    promptFunctionType.clone(),
                    attachments.clone(),
                    replyToMessage.clone(),
                    turnOptions.clone(),
                )
                .await
            {
                return;
            }
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
            chatProviderIdOverride,
            chatModelIdOverride,
            attachments,
            replyToMessage,
            false,
            None,
            false,
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
        let enableThinking = ApiPreferences::getInstance()
            .enableThinkingModeFlow()
            .first()
            .expect("enable_thinking_mode preference must be readable");
        let mut variantMessage = self
            .messageProcessingDelegate
            .regenerateAiMessageVariant(RegenerateAiMessageVariantRequest {
                enhancedAiService,
                chatHistoryDelegate: &mut self.chatHistoryDelegate,
                chatId: chatId.clone(),
                targetMessageTimestamp: targetMessage.timestamp,
                requestMessageContent,
                requestHistory,
                workspacePath,
                promptFunctionType: self.currentPromptFunctionType.clone(),
                roleCardId: String::new(),
                currentRoleName: targetMessage.roleName,
                attachments: Vec::new(),
                replyToMessage: None,
                enableThinking,
                enableMemoryAutoUpdate: false,
                maxTokens: 0,
                tokenUsageThreshold: 0.0,
                chatProviderIdOverride: None,
                chatModelIdOverride: None,
            })
            .await
            .map_err(|error| error.to_string())?;
        self.chatHistoryDelegate.addMessageToChat(
            ChatMessage {
                content: String::new(),
                selectedVariantIndex: targetMessage.variantCount,
                variantCount: targetMessage.variantCount + 1,
                isVariantPreview: true,
                ..variantMessage.clone()
            },
            Some(chatId.clone()),
        );
        let Some(mut contentStream) = variantMessage.contentStream.clone() else {
            return Err("Regenerated message stream is missing".to_string());
        };
        let mut content = String::new();
        contentStream.collect(&mut |chunk| {
            content.push_str(&chunk);
        });
        variantMessage.content = content;
        variantMessage.contentStream = None;
        variantMessage.isVariantPreview = false;
        self.chatHistoryDelegate.addMessageVariant(
            targetMessage.timestamp,
            variantMessage,
            Some(chatId),
        );
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
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
        attachments: Vec<AttachmentInfo>,
        replyToMessage: Option<ChatMessage>,
        isGroupOrchestrationTurn: bool,
        groupParticipantNamesText: Option<String>,
        suppressUserMessageInHistory: bool,
        turnOptions: ChatTurnOptions,
    ) {
        self.currentPromptFunctionType = promptFunctionType.clone();
        self.currentChatProviderIdOverride = chatProviderIdOverride.clone();
        self.currentChatModelIdOverride = chatModelIdOverride.clone();
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
                (
                    "groupOrchestration",
                    ChainLogger::boolField(isGroupOrchestrationTurn),
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
        let messageText = messageTextOverride
            .unwrap_or_else(|| self.messageProcessingDelegate.userMessage.text.clone());
        let currentChat = self
            .chatHistoryDelegate
            .chatHistories
            .iter()
            .find(|history| history.id == chatId)
            .cloned();
        let workspacePath = currentChat.clone().and_then(|chat| chat.workspace);
        let roleCardId = match roleCardIdOverride
            .clone()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            Some(roleCardId) => roleCardId,
            None => match self.resolveRoleCardIdForSend(currentChat.as_ref()) {
                Ok(roleCardId) => roleCardId,
                Err(error) => {
                    ChainLogger::error(
                        SEND_CHAIN,
                        "send.dispatch.role.resolve.error",
                        &[("chatId", chatId.clone()), ("error", error.to_string())],
                    );
                    self.messageProcessingDelegate
                        .setInputProcessingStateForChat(
                            chatId.clone(),
                            InputProcessingState::Error {
                                message: error.to_string(),
                            },
                        );
                    return;
                }
            },
        };
        let runtimeChatHistory = self
            .chatHistoryDelegate
            .getRuntimeChatHistory(chatId.clone());
        let enableThinking = ApiPreferences::getInstance()
            .enableThinkingModeFlow()
            .first()
            .expect("enable_thinking_mode preference must be readable");
        let result = self
            .messageProcessingDelegate
            .sendUserMessage(SendUserMessageProcessingRequest {
                enhancedAiService,
                chatHistoryDelegate: &mut self.chatHistoryDelegate,
                chatId: chatId.clone(),
                messageText,
                chatHistory: runtimeChatHistory,
                workspacePath,
                promptFunctionType,
                roleCardId,
                currentRoleName: None,
                characterName: None,
                avatarUri: None,
                attachments,
                replyToMessage,
                enableThinking,
                enableMemoryAutoUpdate: false,
                maxTokens: 0,
                tokenUsageThreshold: 0.0,
                chatProviderIdOverride,
                chatModelIdOverride,
                isGroupOrchestrationTurn,
                groupParticipantNamesText,
                proxySenderNameOverride,
                suppressUserMessageInHistory: suppressUserMessageInHistory || isContinuation,
                isAutoContinuation,
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
                    .setInputProcessingStateForChat(
                        chatId.clone(),
                        InputProcessingState::Error {
                            message: error.to_string(),
                        },
                    );
                return;
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
    }

    #[allow(non_snake_case)]
    fn resolveRoleCardIdForSend(
        &self,
        currentChat: Option<&crate::data::model::ChatHistory::ChatHistory>,
    ) -> Result<String, operit_store::PreferencesDataStore::PreferencesDataStoreError> {
        if let Some(chat) = currentChat {
            let hasGroupBinding = chat
                .characterGroupId
                .as_ref()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false);
            if !hasGroupBinding {
                if let Some(characterCardName) = chat
                    .characterCardName
                    .as_ref()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                {
                    if let Some(card) = self
                        .characterCardManager
                        .findCharacterCardByName(&characterCardName)?
                    {
                        return Ok(card.id);
                    }
                }
            }
        }
        ActivePromptManager::getInstance().resolveActiveCardIdForSend()
    }

    pub fn handleManualMemoryUpdate(&mut self, _chatId: Option<String>) {
        if self.isUpdatingMemory {
            return;
        }
        self.isUpdatingMemory = true;
        self.isUpdatingMemory = false;
    }

    #[allow(non_snake_case)]
    fn shouldRunGroupOrchestration(
        &self,
        promptFunctionType: PromptFunctionType,
        isContinuation: bool,
        isAutoContinuation: bool,
        skipSummaryCheck: bool,
        roleCardIdOverride: Option<String>,
        proxySenderNameOverride: Option<String>,
        messageTextOverride: Option<String>,
        chatIdOverride: Option<String>,
    ) -> bool {
        if promptFunctionType != PromptFunctionType::CHAT {
            return false;
        }
        if isContinuation || isAutoContinuation || skipSummaryCheck {
            return false;
        }
        if roleCardIdOverride
            .as_ref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        {
            return false;
        }
        if proxySenderNameOverride
            .as_ref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        {
            return false;
        }
        if messageTextOverride
            .as_ref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        {
            return false;
        }
        if chatIdOverride
            .as_ref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        {
            return false;
        }
        matches!(
            self.activePromptManager.getActivePrompt(),
            Ok(ActivePrompt::CharacterGroup { .. })
        )
    }

    #[allow(non_snake_case)]
    async fn orchestrateGroupConversation(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        chatId: String,
        promptFunctionType: PromptFunctionType,
        attachments: Vec<AttachmentInfo>,
        replyToMessage: Option<ChatMessage>,
        turnOptions: ChatTurnOptions,
    ) -> bool {
        let Some(group) = self.resolveTargetGroupForChat(&chatId) else {
            return false;
        };
        let mut orderedMembers = group.members.clone();
        orderedMembers.sort_by_key(|member| member.orderIndex);
        orderedMembers.retain(|member| !member.characterCardId.trim().is_empty());
        if orderedMembers.is_empty() {
            return false;
        }
        let existingBinding = self
            .chatHistoryDelegate
            .chatHistories
            .iter()
            .find(|history| history.id == chatId)
            .and_then(|history| history.characterGroupId.clone());
        if existingBinding.as_deref() != Some(group.id.as_str()) {
            self.chatHistoryDelegate.updateChatCharacterBinding(
                chatId.clone(),
                None,
                Some(group.id.clone()),
            );
        }

        let originalUserText = self
            .messageProcessingDelegate
            .userMessage
            .text
            .trim()
            .to_string();
        if originalUserText.is_empty() && attachments.is_empty() {
            return false;
        }
        if !originalUserText.is_empty() {
            self.messageProcessingDelegate
                .updateUserMessage(String::new());
        }
        self.messageProcessingDelegate
            .setInputProcessingStateForChat(
                chatId.clone(),
                InputProcessingState::Processing {
                    message: "role_response_planner_planning".to_string(),
                },
            );

        let currentChat = self
            .chatHistoryDelegate
            .chatHistories
            .iter()
            .find(|history| history.id == chatId)
            .cloned();
        let workspacePath = currentChat.clone().and_then(|chat| chat.workspace);
        if !self.chatHistoryDelegate.hasUserMessage(chatId.clone()) {
            let newTitle = if !originalUserText.is_empty() {
                originalUserText.clone()
            } else {
                attachments
                    .first()
                    .map(|attachment| attachment.fileName.clone())
                    .unwrap_or_else(|| "New Chat".to_string())
            };
            self.chatHistoryDelegate
                .updateChatTitle(chatId.clone(), newTitle);
        }

        let finalUserMessageContent = match self
            .messageProcessingDelegate
            .buildUserMessageContentForGroupOrchestration(
                BuildUserMessageContentForGroupOrchestrationRequest {
                    messageText: originalUserText.clone(),
                    attachments: attachments.clone(),
                    workspacePath: workspacePath.clone(),
                    replyToMessage,
                    chatId: chatId.clone(),
                    roleCardId: CharacterCardManager::DEFAULT_CHARACTER_CARD_ID.to_string(),
                },
            ) {
            Ok(content) => content,
            Err(error) => {
                self.messageProcessingDelegate
                    .setInputProcessingStateForChat(
                        chatId,
                        InputProcessingState::Error {
                            message: error.to_string(),
                        },
                    );
                return true;
            }
        };
        self.chatHistoryDelegate.addMessageToChat(
            ChatMessage {
                sender: "user".to_string(),
                content: finalUserMessageContent,
                roleName: "用户".to_string(),
                displayMode: if turnOptions.hideUserMessage {
                    ChatMessageDisplayMode::HIDDEN_PLACEHOLDER
                } else {
                    ChatMessageDisplayMode::NORMAL
                },
                ..ChatMessage::new("user".to_string())
            },
            Some(chatId.clone()),
        );

        let mut timeline = Vec::<(String, String)>::new();
        if !originalUserText.trim().is_empty() {
            timeline.push(("用户".to_string(), originalUserText.clone()));
        }

        let memberCardsById = orderedMembers
            .iter()
            .filter_map(|member| {
                self.characterCardManager
                    .getCharacterCard(&member.characterCardId)
                    .ok()
                    .map(|card| (member.characterCardId.clone(), card))
            })
            .collect::<HashMap<_, _>>();
        let groupParticipantNamesText =
            self.buildGroupParticipantNamesText(&orderedMembers, &memberCardsById);
        let Some(plannedRounds) = self
            .planResponseOrder(
                enhancedAiService,
                &originalUserText,
                &orderedMembers,
                &memberCardsById,
            )
            .await
        else {
            self.messageProcessingDelegate
                .setInputProcessingStateForChat(
                    chatId,
                    InputProcessingState::Error {
                        message: "role_response_planner_failed".to_string(),
                    },
                );
            return true;
        };
        if plannedRounds.rounds.is_empty()
            || plannedRounds
                .rounds
                .iter()
                .all(|round| round.iter().all(|member| !member.speak))
        {
            self.messageProcessingDelegate
                .setInputProcessingStateForChat(chatId, InputProcessingState::Completed);
            return true;
        }

        for (roundIndex, roundMembers) in plannedRounds.rounds.iter().enumerate() {
            for (memberIndex, plannedMember) in roundMembers.iter().enumerate() {
                if !plannedMember.speak {
                    continue;
                }
                let Some(member) = orderedMembers
                    .iter()
                    .find(|member| member.characterCardId == plannedMember.id)
                    .cloned()
                else {
                    continue;
                };
                let Some(memberCard) = memberCardsById.get(&member.characterCardId).cloned() else {
                    continue;
                };
                self.messageProcessingDelegate
                    .setInputProcessingStateForChat(
                        chatId.clone(),
                        InputProcessingState::Processing {
                            message: format!(
                                "role_response_planner_member_replying|{}",
                                memberCard.name
                            ),
                        },
                    );
                let beforeLastAiTimestamp = self
                    .chatHistoryDelegate
                    .getRuntimeChatHistory(chatId.clone())
                    .iter()
                    .filter(|message| message.sender == "ai")
                    .map(|message| message.timestamp)
                    .max()
                    .unwrap_or(i64::MIN);
                let targetTurnCounter = self
                    .messageProcessingDelegate
                    .getTurnCompleteCounter(chatId.clone())
                    + 1;
                let isFirstMemberOfFirstRound = roundIndex == 0 && memberIndex == 0;
                let memberMessage = if isFirstMemberOfFirstRound {
                    originalUserText.clone()
                } else {
                    String::new()
                };
                self.sendMessageInternal(
                    enhancedAiService,
                    promptFunctionType.clone(),
                    !isFirstMemberOfFirstRound,
                    false,
                    Some(member.characterCardId),
                    Some(chatId.clone()),
                    Some(memberMessage),
                    None,
                    None,
                    None,
                    Vec::new(),
                    None,
                    true,
                    Some(groupParticipantNamesText.clone()),
                    true,
                    turnOptions.clone(),
                )
                .await;
                if !self
                    .awaitTurnComplete(chatId.clone(), targetTurnCounter, 180_000)
                    .await
                {
                    continue;
                }
                let newAiMessage = self
                    .chatHistoryDelegate
                    .getRuntimeChatHistory(chatId.clone())
                    .into_iter()
                    .rev()
                    .find(|message| {
                        message.sender == "ai" && message.timestamp > beforeLastAiTimestamp
                    });
                if let Some(newAiMessage) = newAiMessage {
                    if !newAiMessage.content.trim().is_empty() {
                        let effectiveSpeech = extractEffectiveSpeechContent(&newAiMessage.content);
                        if !effectiveSpeech.trim().is_empty() {
                            timeline.push((
                                format!("AI({})", memberCard.name),
                                shrinkForMemberPrompt(&effectiveSpeech, 220),
                            ));
                        }
                    }
                }
            }
        }
        self.maybeSummarizeAfterGroupRound(enhancedAiService, chatId, promptFunctionType)
            .await;
        true
    }

    #[allow(non_snake_case)]
    async fn planResponseOrder(
        &self,
        enhancedAiService: &mut EnhancedAIService,
        userText: &str,
        members: &[GroupMemberConfig],
        memberCardsById: &HashMap<String, CharacterCard>,
    ) -> Option<PlannedRounds> {
        let memberLines = members
            .iter()
            .filter_map(|member| {
                let card = memberCardsById.get(&member.characterCardId)?;
                Some(format!(
                    "- id: {}, name: {}",
                    member.characterCardId, card.name
                ))
            })
            .collect::<Vec<_>>()
            .join("\n");
        let prompt =
            FunctionalPrompts::buildGroupRoleResponsePlannerPrompt(&memberLines, userText, false);
        let mut options = SendMessageOptions::new();
        options.message = prompt;
        options.functionType = FunctionType::ROLE_RESPONSE_PLANNER;
        options.promptFunctionType = PromptFunctionType::CHAT;
        options.enableThinking = false;
        options.stream = false;
        let response = enhancedAiService.sendMessage(options).await.ok()?;
        let rawContent = removeThinkingContent(&collect_stream_chunks(response).join(""))
            .trim()
            .to_string();
        self.parsePlannedRounds(
            &rawContent,
            members
                .iter()
                .map(|member| member.characterCardId.clone())
                .collect(),
            memberCardsById
                .values()
                .map(|card| (card.name.trim().to_string(), card.id.clone()))
                .collect(),
        )
    }

    #[allow(non_snake_case)]
    fn parsePlannedRounds(
        &self,
        rawContent: &str,
        memberIds: std::collections::HashSet<String>,
        memberNameToId: HashMap<String, String>,
    ) -> Option<PlannedRounds> {
        if rawContent.trim().is_empty() {
            return None;
        }
        let trimmed = rawContent.trim();
        let jsonText = if trimmed.starts_with('{') && trimmed.ends_with('}') {
            trimmed.to_string()
        } else if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
            if end > start {
                trimmed[start..=end].to_string()
            } else {
                trimmed.to_string()
            }
        } else {
            trimmed.to_string()
        };
        let obj = serde_json::from_str::<Value>(&jsonText).ok()?;
        let resolveId = |value: Option<&str>| -> Option<String> {
            let trimmedValue = value.unwrap_or_default().trim();
            if trimmedValue.is_empty() {
                return None;
            }
            if memberIds.contains(trimmedValue) {
                return Some(trimmedValue.to_string());
            }
            memberNameToId.get(trimmedValue).cloned()
        };
        let parseMember = |item: &Value| -> Option<PlannedMember> {
            match item {
                Value::String(value) => {
                    resolveId(Some(value)).map(|id| PlannedMember { id, speak: true })
                }
                Value::Object(map) => {
                    let id = resolveId(
                        map.get("id")
                            .and_then(Value::as_str)
                            .or_else(|| map.get("memberId").and_then(Value::as_str))
                            .or_else(|| map.get("roleId").and_then(Value::as_str))
                            .or_else(|| map.get("name").and_then(Value::as_str)),
                    )?;
                    let skip = map.get("skip").and_then(Value::as_bool).unwrap_or(false);
                    let speak = map.get("speak").and_then(Value::as_bool).unwrap_or(!skip);
                    Some(PlannedMember { id, speak })
                }
                _ => None,
            }
        };
        if let Some(roundsArray) = obj.get("rounds").and_then(Value::as_array) {
            let mut rounds = Vec::new();
            for roundItem in roundsArray {
                let Some(roundArray) = roundItem.as_array() else {
                    continue;
                };
                let mut roundMembers = Vec::new();
                let mut seen = std::collections::HashSet::new();
                for item in roundArray {
                    let Some(member) = parseMember(item) else {
                        continue;
                    };
                    if seen.insert(member.id.clone()) {
                        roundMembers.push(member);
                    }
                }
                if !roundMembers.is_empty() {
                    rounds.push(roundMembers);
                }
            }
            return Some(PlannedRounds { rounds });
        }
        let orderArray = obj
            .get("order")
            .and_then(Value::as_array)
            .or_else(|| obj.get("plan").and_then(Value::as_array))
            .or_else(|| obj.get("members").and_then(Value::as_array))?;
        let mut members = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for item in orderArray {
            let Some(member) = parseMember(item) else {
                continue;
            };
            if seen.insert(member.id.clone()) {
                members.push(member);
            }
        }
        Some(PlannedRounds {
            rounds: vec![members],
        })
    }

    #[allow(non_snake_case)]
    fn buildGroupParticipantNamesText(
        &self,
        members: &[GroupMemberConfig],
        memberCardsById: &HashMap<String, CharacterCard>,
    ) -> String {
        let mut orderedMembers = members.to_vec();
        orderedMembers.sort_by_key(|member| member.orderIndex);
        let mut participantNames = Vec::new();
        for member in orderedMembers {
            let Some(card) = memberCardsById.get(&member.characterCardId) else {
                continue;
            };
            let name = card.name.trim();
            if !name.is_empty() && !participantNames.iter().any(|entry| entry == name) {
                participantNames.push(name.to_string());
            }
        }
        participantNames.push("用户（用户）".to_string());
        participantNames.join("、")
    }

    #[allow(non_snake_case)]
    fn resolveTargetGroupForChat(&self, chatId: &str) -> Option<CharacterGroupCard> {
        let activePrompt = self.activePromptManager.getActivePrompt().ok()?;
        let activeGroupId = match activePrompt {
            ActivePrompt::CharacterGroup { id } if !id.trim().is_empty() => id,
            _ => return None,
        };
        let _boundGroupId = self
            .chatHistoryDelegate
            .chatHistories
            .iter()
            .find(|history| history.id == chatId)
            .and_then(|history| history.characterGroupId.clone())
            .filter(|value| !value.trim().is_empty());
        self.characterGroupCardManager
            .getCharacterGroupCard(&activeGroupId)
            .ok()
            .flatten()
    }

    #[allow(non_snake_case)]
    async fn awaitTurnComplete(&self, chatId: String, targetCounter: i64, timeoutMs: u64) -> bool {
        let started = Instant::now();
        while started.elapsed() < Duration::from_millis(timeoutMs) {
            if self
                .messageProcessingDelegate
                .getTurnCompleteCounter(chatId.clone())
                >= targetCounter
            {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        self.messageProcessingDelegate
            .getTurnCompleteCounter(chatId)
            >= targetCounter
    }

    #[allow(non_snake_case)]
    async fn maybeSummarizeAfterGroupRound(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        chatId: String,
        promptFunctionType: PromptFunctionType,
    ) {
        let (providerId, modelId) = match (
            self.currentChatProviderIdOverride.clone(),
            self.currentChatModelIdOverride.clone(),
        ) {
            (Some(providerId), Some(modelId))
                if !providerId.trim().is_empty() && !modelId.trim().is_empty() =>
            {
                (providerId, modelId)
            }
            _ => match self
                .messageProcessingDelegate
                .functionalConfigManager
                .getModelBindingForFunction(FunctionType::CHAT)
            {
                Ok(binding) => (binding.providerId, binding.modelId),
                Err(_) => return,
            },
        };
        let chatContextSettings = match self
            .messageProcessingDelegate
            .modelConfigManager
            .getResolvedModelConfig(&providerId, &modelId)
        {
            Ok(config) => config,
            Err(_) => return,
        };
        if !chatContextSettings.summary.enableSummary {
            return;
        }
        let currentMessages = self
            .chatHistoryDelegate
            .getRuntimeChatHistory(chatId.clone());
        let currentTokens = self
            .tokenStatisticsDelegate
            .getLastCurrentWindowSize(Some(chatId.clone()));
        let effectiveContextLength = if chatContextSettings.context.enableMaxContextMode {
            chatContextSettings.context.maxContextLength
        } else {
            chatContextSettings.context.maxContextLength * 0.4
        };
        let maxTokens = (effectiveContextLength * 1024.0) as i32;
        let shouldSummarize = AIMessageManager::shouldGenerateSummary(
            currentMessages.clone(),
            currentTokens,
            maxTokens,
            chatContextSettings.summary.summaryTokenThreshold as f64,
            chatContextSettings.summary.enableSummary,
            chatContextSettings.summary.enableSummaryByMessageCount,
            chatContextSettings.summary.summaryMessageCountThreshold,
        );
        if shouldSummarize {
            self.summarizeHistory(
                enhancedAiService,
                false,
                Some(promptFunctionType),
                Some(chatId),
                self.currentChatProviderIdOverride.clone(),
                self.currentChatModelIdOverride.clone(),
                None,
                true,
                false,
                None,
            )
            .await;
        }
    }

    pub async fn manuallySummarizeConversation(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
    ) {
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
            roleCardId,
            false,
            isGroupOrchestrationTurn,
            groupParticipantNamesText,
        )
        .await;
        self.summaryJob = None;
    }

    fn cancelSummaryStreamingInternal(&mut self, _enhancedAiService: &mut EnhancedAIService) {}

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
                    message: "compressing history".to_string(),
                },
            );
        let isGroupChat = self
            .chatHistoryDelegate
            .chatHistories
            .iter()
            .find(|history| history.id == originalChatId)
            .and_then(|history| history.characterGroupId.clone())
            .is_some();
        if let Ok(Some(summaryMessage)) = AIMessageManager::summarizeMemory(
            enhancedAiService,
            snapshotMessages,
            false,
            isGroupChat,
        )
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
                roleCardId,
                None,
                false,
                None,
                chatProviderIdOverride,
                chatModelIdOverride,
            )
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

    async fn summarizeHistory(
        &mut self,
        enhancedAiService: &mut EnhancedAIService,
        autoContinue: bool,
        promptFunctionType: Option<PromptFunctionType>,
        chatIdOverride: Option<String>,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
        roleCardIdOverride: Option<String>,
        isGroupChat: bool,
        isGroupOrchestrationTurn: bool,
        groupParticipantNamesText: Option<String>,
    ) -> bool {
        if self.isSummarizing {
            return false;
        }
        self.isSummarizing = true;
        let currentChatId =
            chatIdOverride.or_else(|| self.chatHistoryDelegate.currentChatId.clone());
        self.summarizingChatId = currentChatId.clone();
        if let Some(currentChatId) = currentChatId.clone() {
            self.messageProcessingDelegate
                .setSuppressIdleCompletedStateForChat(currentChatId.clone(), true);
            self.messageProcessingDelegate
                .setInputProcessingStateForChat(
                    currentChatId,
                    InputProcessingState::Summarizing {
                        message: "compressing history".to_string(),
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
            autoContinue,
            isGroupChat,
        )
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
                roleCardIdOverride.clone(),
                None,
                isGroupOrchestrationTurn,
                groupParticipantNamesText.clone(),
                effectiveChatProviderIdOverride.clone(),
                effectiveChatModelIdOverride.clone(),
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
                if self
                    .messageProcessingDelegate
                    .isChatLoading(currentChatId.clone())
                {
                    self.queuePendingAutoContinuation(
                        currentChatId,
                        continuationPromptType,
                        effectiveChatProviderIdOverride.clone(),
                        effectiveChatModelIdOverride,
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
                        effectiveChatProviderIdOverride,
                        effectiveChatModelIdOverride,
                        Vec::new(),
                        None,
                        isGroupOrchestrationTurn,
                        groupParticipantNamesText,
                        false,
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
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
        roleCardIdOverride: Option<String>,
        isGroupOrchestrationTurn: bool,
        groupParticipantNamesText: Option<String>,
    ) {
        self.pendingAutoContinuationByChatId.insert(
            chatId.clone(),
            PendingAutoContinuationRequest {
                chatId,
                promptFunctionType,
                chatProviderIdOverride,
                chatModelIdOverride,
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

#[allow(non_snake_case)]
fn removeThinkingContent(input: &str) -> String {
    let mut output = String::new();
    let mut rest = input;
    loop {
        let Some(start) = rest.find("<think>") else {
            output.push_str(rest);
            break;
        };
        output.push_str(&rest[..start]);
        let afterStart = &rest[start + "<think>".len()..];
        let Some(end) = afterStart.find("</think>") else {
            break;
        };
        rest = &afterStart[end + "</think>".len()..];
    }
    output
}

#[allow(non_snake_case)]
fn extractEffectiveSpeechContent(content: &str) -> String {
    let withoutThinking = removeThinkingContent(content);
    let withoutStatus = removeTagBlocks(&withoutThinking, "status");
    removeSelfClosingTags(&withoutStatus, "status")
        .trim()
        .to_string()
}

#[allow(non_snake_case)]
fn shrinkForMemberPrompt(content: &str, maxLength: usize) -> String {
    let normalized = content.replace('\n', " ").trim().to_string();
    if normalized.chars().count() <= maxLength {
        normalized
    } else {
        let prefix = normalized.chars().take(maxLength).collect::<String>();
        format!("{prefix}...")
    }
}

#[allow(non_snake_case)]
fn removeTagBlocks(content: &str, tagName: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    let openTag = format!("<{tagName}");
    let closeTag = format!("</{tagName}>");
    while let Some(openOffset) = content[cursor..].find(&openTag) {
        let openStart = cursor + openOffset;
        output.push_str(&content[cursor..openStart]);
        let Some(openEndOffset) = content[openStart..].find('>') else {
            cursor = openStart;
            break;
        };
        let bodyStart = openStart + openEndOffset + 1;
        let Some(closeOffset) = content[bodyStart..].find(&closeTag) else {
            cursor = bodyStart;
            break;
        };
        cursor = bodyStart + closeOffset + closeTag.len();
        output.push(' ');
    }
    output.push_str(&content[cursor..]);
    output
}

#[allow(non_snake_case)]
fn removeSelfClosingTags(content: &str, tagName: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    let openTag = format!("<{tagName}");
    while let Some(openOffset) = content[cursor..].find(&openTag) {
        let openStart = cursor + openOffset;
        output.push_str(&content[cursor..openStart]);
        let Some(endOffset) = content[openStart..].find("/>") else {
            cursor = openStart;
            break;
        };
        cursor = openStart + endOffset + 2;
        output.push(' ');
    }
    output.push_str(&content[cursor..]);
    output
}
