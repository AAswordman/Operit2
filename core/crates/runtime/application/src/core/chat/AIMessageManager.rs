use std::collections::HashMap;
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};

use crate::core::chat::plugins::MessageProcessingPluginRegistry::{
    MessageProcessingHookParams, MessageProcessingPluginRegistry,
};
use crate::data::preferences::ApiPreferences::ApiPreferences;
use operit_host_api::FileSystemHost;
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatMessageTimestampAllocator::ChatMessageTimestampAllocator;
use operit_model::MessagePart::{MessagePart, MessagePartKind};
use operit_model::MessagePartCodec::MessagePartCodec;
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
use operit_providers::chat::EnhancedAIService::{
    EnhancedAIService, ResumeRequest, SendMessageCallbacks, SendMessageOptions, SendMessageRuntime,
};
use operit_providers::chat::enhance::InputProcessor::{InputProcessor, ProcessUserInputRequest};
use operit_providers::chat::llmprovider::AIService::{AiServiceError, SharedAiResponseStream};
use operit_providers::chat::llmprovider::MediaLinkBuilder::MediaLinkBuilder;
use operit_providers::chat::llmprovider::MediaLinkParser::MediaLinkParser;
use operit_store::PreferencesDataStore::FlowLike;
use operit_util::AppLogger::AppLogger;
use operit_util::ChainLogger::{self, PLUGIN_CHAIN, RECEIVE_CHAIN, SEND_CHAIN};
use operit_util::ImagePoolManager::ImagePoolManager;
use operit_util::stream::RevisableTextStream::with_event_channel_shared;
use operit_util::stream::Stream::Stream;

const DEFAULT_CHAT_KEY: &str = "__DEFAULT_CHAT__";
const MESSAGE_PROCESS_TIMING_TAG: &str = "MessageProcessTiming";

pub struct AIMessageManager;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageTiming {
    pub startedAtMs: u64,
}

#[derive(Clone)]
pub struct BuildUserMessageContentRequest {
    pub messageText: String,
    pub proxySenderName: Option<String>,
    pub attachments: Vec<AttachmentInfo>,
    pub fileSystemHost: Option<Arc<dyn FileSystemHost>>,
    pub workspacePath: Option<String>,
    pub replyToMessage: Option<ChatMessage>,
    pub enableDirectImageProcessing: bool,
    pub enableDirectAudioProcessing: bool,
    pub enableDirectVideoProcessing: bool,
    pub chatId: Option<String>,
    pub roleCardId: Option<String>,
    /// Host callback used to surface a timed-out Prompt Input Hook.
    pub onHookTimeout: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

pub struct SendMessageRequest<'a> {
    pub enhancedAiService: &'a mut EnhancedAIService,
    pub chatId: Option<String>,
    pub messageContent: String,
    pub chatHistory: Vec<ChatMessage>,
    pub promptHistoryOverride: Option<Vec<PromptTurn>>,
    pub workspacePath: Option<String>,
    pub promptFunctionType: PromptFunctionType,
    pub enableThinking: bool,
    pub enableMemoryAutoUpdate: bool,
    pub maxTokens: i32,
    pub tokenUsageThreshold: f64,
    pub characterName: Option<String>,
    pub avatarUri: Option<String>,
    pub roleCardId: String,
    pub currentRoleName: Option<String>,
    pub splitHistoryByRole: bool,
    pub groupOrchestrationMode: bool,
    pub groupParticipantNamesText: Option<String>,
    pub proxySenderName: Option<String>,
    pub notifyReplyOverride: Option<bool>,
    pub chatProviderIdOverride: Option<String>,
    pub chatModelIdOverride: Option<String>,
    pub disableWarning: bool,
    pub callbacks: Option<Arc<dyn SendMessageCallbacks + Send + Sync>>,
    pub onToolInvocation: Option<Arc<dyn Fn(String) + Send + Sync>>,
    pub resume: bool,
}

pub struct StableContextWindowRequest<'a> {
    pub enhancedAiService: &'a mut EnhancedAIService,
    pub chatId: Option<String>,
    pub messageContent: String,
    pub chatHistory: Vec<ChatMessage>,
    pub workspacePath: Option<String>,
    pub promptFunctionType: PromptFunctionType,
    pub roleCardId: Option<String>,
    pub currentRoleName: Option<String>,
    pub splitHistoryByRole: bool,
    pub groupOrchestrationMode: bool,
    pub groupParticipantNamesText: Option<String>,
    pub proxySenderName: Option<String>,
    pub chatProviderIdOverride: Option<String>,
    pub chatModelIdOverride: Option<String>,
    pub publishEstimate: bool,
    pub runtime: SendMessageRuntime,
}

static ACTIVE_CHAT_KEYS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
static LAST_ACTIVE_CHAT_KEY: OnceLock<Mutex<String>> = OnceLock::new();
static ACTIVE_ENHANCED_AI_SERVICE_BY_CHAT_ID: OnceLock<Mutex<HashMap<String, EnhancedAIService>>> =
    OnceLock::new();
static ACTIVE_RESPONSE_STREAM_BY_CHAT_ID: OnceLock<Mutex<HashMap<String, SharedAiResponseStream>>> =
    OnceLock::new();

pub fn messageTimingNow() -> MessageTiming {
    let startedAtMs = operit_host_api::TimeUtils::currentTimeMillis() as u64;
    MessageTiming { startedAtMs }
}

pub fn logMessageTiming(stage: &str, startTimeMs: MessageTiming, details: Option<String>) {
    let now = operit_host_api::TimeUtils::currentTimeMillis() as u64;
    let elapsed = now.saturating_sub(startTimeMs.startedAtMs);
    let suffix = details
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!(", {value}"))
        .unwrap_or_default();
    AppLogger::d(
        MESSAGE_PROCESS_TIMING_TAG,
        &format!("{stage} 耗时={elapsed}ms{suffix}"),
    );
}

impl AIMessageManager {
    pub fn initialize() {
        let _ = ACTIVE_CHAT_KEYS.get_or_init(|| Mutex::new(HashMap::new()));
        let _ = LAST_ACTIVE_CHAT_KEY.get_or_init(|| Mutex::new(DEFAULT_CHAT_KEY.to_string()));
        let _ = ACTIVE_ENHANCED_AI_SERVICE_BY_CHAT_ID.get_or_init(|| Mutex::new(HashMap::new()));
        let _ = ACTIVE_RESPONSE_STREAM_BY_CHAT_ID.get_or_init(|| Mutex::new(HashMap::new()));
    }

    #[allow(non_snake_case)]
    /// Builds user message content with processed text, attachments, workspace, and reply context.
    pub fn buildUserMessageContent(
        request: BuildUserMessageContentRequest,
    ) -> Result<String, AiServiceError> {
        let promptInputStartTime = messageTimingNow();
        let originalMessageLength = request.messageText.len();
        let processedMessageText = InputProcessor::process_user_input(ProcessUserInputRequest {
            input: request.messageText,
            chat_id: request.chatId.clone(),
            role_card_id: request.roleCardId.clone(),
            on_hook_timeout: request.onHookTimeout.clone(),
        });
        logMessageTiming(
            "buildUserMessageContent.processUserInput",
            promptInputStartTime,
            Some(format!(
                "originalLength={}, processedLength={}",
                originalMessageLength,
                processedMessageText.len()
            )),
        );
        let proxySenderTag = match request.proxySenderName {
            Some(proxySenderName)
                if !proxySenderName.trim().is_empty()
                    && !processedMessageText
                        .to_ascii_lowercase()
                        .contains("<proxy_sender") =>
            {
                format!(
                    "<proxy_sender name=\"{}\"/>",
                    proxySenderName.replace('"', "'")
                )
            }
            _ => String::new(),
        };

        let replyTag = match request.replyToMessage.as_ref() {
            Some(message) => Self::buildReplyTag(message),
            None => String::new(),
        };

        let workspaceTag = match request.workspacePath.as_ref().map(|value| value.trim()) {
            Some(path)
                if !path.is_empty()
                    && !processedMessageText
                        .to_ascii_lowercase()
                        .contains("<workspace_attachment") =>
            {
                format!("<workspace_attachment>{path}</workspace_attachment>")
            }
            _ => String::new(),
        };

        let attachmentTags = request
            .attachments
            .iter()
            .map(|attachment| {
                Self::buildAttachmentTag(
                    attachment,
                    request.fileSystemHost.as_deref(),
                    request.enableDirectImageProcessing,
                    request.enableDirectAudioProcessing,
                    request.enableDirectVideoProcessing,
                )
                .map_err(AiServiceError::RequestFailed)
            })
            .collect::<Result<Vec<_>, AiServiceError>>()?
            .join(" ");

        Ok([
            proxySenderTag,
            processedMessageText,
            attachmentTags,
            workspaceTag,
            replyTag,
        ]
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" "))
    }

    #[allow(non_snake_case)]
    pub async fn sendMessage(
        request: SendMessageRequest<'_>,
    ) -> Result<
        operit_providers::chat::llmprovider::AIService::SharedAiResponseStream,
        operit_providers::chat::llmprovider::AIService::AiServiceError,
    > {
        let chatKey = match &request.chatId {
            Some(chatId) => chatId.clone(),
            None => DEFAULT_CHAT_KEY.to_string(),
        };
        Self::rememberActiveChatKey(chatKey.clone());
        Self::setLastActiveChatKey(chatKey.clone());
        Self::rememberActiveEnhancedAiService(chatKey.clone(), request.enhancedAiService.clone());
        ChainLogger::info(
            SEND_CHAIN,
            "send.ai_manager.start",
            &[
                ("chatKey", chatKey.clone()),
                (
                    "messageChars",
                    ChainLogger::lenField(&request.messageContent),
                ),
                ("historyCount", request.chatHistory.len().to_string()),
            ],
        );

        let memory = match request.promptHistoryOverride.clone() {
            Some(promptHistory) => promptHistory,
            None => Self::getMemoryFromMessages(
                request.chatHistory.clone(),
                request.splitHistoryByRole,
                request.currentRoleName.clone(),
                request.groupOrchestrationMode,
            ),
        };

        let apiPreferences = ApiPreferences::getInstance();
        let maxImageHistoryUserTurns = apiPreferences
            .maxImageHistoryUserTurnsFlow()
            .first()
            .unwrap_or(2);
        let maxMediaHistoryUserTurns = apiPreferences
            .maxMediaHistoryUserTurnsFlow()
            .first()
            .unwrap_or(1);
        let memoryAfterImageLimit =
            Self::limitImageLinksInChatHistory(memory, maxImageHistoryUserTurns);
        let memoryForRequest =
            Self::limitMediaLinksInChatHistory(memoryAfterImageLimit, maxMediaHistoryUserTurns);

        let pluginExecution = MessageProcessingPluginRegistry::createExecutionIfMatched(
            MessageProcessingHookParams {
                chat_id: request.chatId.clone(),
                message_content: request.messageContent.clone(),
                chat_history: memoryForRequest.clone(),
                workspace_path: request.workspacePath.clone(),
                max_tokens: request.maxTokens,
                token_usage_threshold: request.tokenUsageThreshold,
            },
        );
        if let Some(pluginExecution) = pluginExecution {
            ChainLogger::info(
                PLUGIN_CHAIN,
                "plugin.message_processing.matched",
                &[
                    ("chatKey", chatKey.clone()),
                    (
                        "messageChars",
                        ChainLogger::lenField(&request.messageContent),
                    ),
                ],
            );
            Self::forgetActiveChatKey(&chatKey);
            Self::forgetActiveEnhancedAiService(&chatKey);
            return Ok(SharedAiResponseStream::from(with_event_channel_shared(
                pluginExecution.stream,
                operit_util::stream::HotStream::mutable_shared_stream(usize::MAX),
            )));
        }

        let disableStreamOutput = apiPreferences
            .disableStreamOutputFlow()
            .first()
            .unwrap_or(false);
        let enableStream = !disableStreamOutput;

        let mut options = SendMessageOptions::new();
        options.message = request.messageContent;
        options.chatId = request.chatId;
        options.chatHistory = memoryForRequest;
        options.workspacePath = request.workspacePath;
        options.promptFunctionType = request.promptFunctionType;
        options.enableThinking = request.enableThinking;
        options.enableMemoryAutoUpdate = request.enableMemoryAutoUpdate;
        options.maxTokens = request.maxTokens;
        options.tokenUsageThreshold = request.tokenUsageThreshold;
        options.characterName = request.characterName;
        options.avatarUri = request.avatarUri;
        options.roleCardId = Some(request.roleCardId);
        options.enableGroupOrchestrationHint = request.groupOrchestrationMode;
        options.groupParticipantNamesText = request.groupParticipantNamesText;
        options.proxySenderName = request.proxySenderName;
        options.notifyReplyOverride = request.notifyReplyOverride;
        options.chatProviderIdOverride = request.chatProviderIdOverride;
        options.chatModelIdOverride = request.chatModelIdOverride;
        options.disableWarning = request.disableWarning;
        options.callbacks = request.callbacks;
        options.onToolInvocation = request.onToolInvocation;
        options.stream = enableStream;

        let providerOverrideSet = match options.chatProviderIdOverride.as_ref() {
            Some(value) => !value.trim().is_empty(),
            None => false,
        };
        let modelOverrideSet = match options.chatModelIdOverride.as_ref() {
            Some(value) => !value.trim().is_empty(),
            None => false,
        };
        ChainLogger::info(
            SEND_CHAIN,
            "send.provider.request",
            &[
                ("chatKey", chatKey.clone()),
                ("stream", ChainLogger::boolField(enableStream)),
                ("historyCount", options.chatHistory.len().to_string()),
                (
                    "providerOverrideSet",
                    ChainLogger::boolField(providerOverrideSet),
                ),
                ("modelOverrideSet", ChainLogger::boolField(modelOverrideSet)),
            ],
        );
        let providerResponse = if request.resume {
            request
                .enhancedAiService
                .resume(ResumeRequest { options })
                .await
        } else {
            request.enhancedAiService.sendMessage(options).await
        };
        match providerResponse {
            Ok(stream) => {
                let cleanupChatKey = chatKey.clone();
                let mut cleanupStream = stream.chunk_stream();
                defaultHostRuntimeTaskSchedulerHost()
                    .scheduleHostRuntimeAsyncTask(
                        "ai-message-manager-response-cleanup",
                        Box::new(move || {
                            Box::pin(async move {
                                cleanupStream.collect(&mut |_| {}).await;
                                Self::forgetActiveChatKey(&cleanupChatKey);
                                Self::forgetActiveEnhancedAiService(&cleanupChatKey);
                                Self::forgetActiveResponseStream(&cleanupChatKey);
                            })
                        }),
                    )
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
                Self::rememberActiveResponseStream(chatKey.clone(), stream.clone());
                ChainLogger::info(
                    RECEIVE_CHAIN,
                    "receive.provider.stream.ready",
                    &[("chatKey", chatKey.clone())],
                );
                Ok(stream)
            }
            Err(error) => {
                ChainLogger::error(
                    SEND_CHAIN,
                    "send.provider.error",
                    &[("chatKey", chatKey.clone()), ("error", error.to_string())],
                );
                Self::forgetActiveChatKey(&chatKey);
                Self::forgetActiveEnhancedAiService(&chatKey);
                Self::forgetActiveResponseStream(&chatKey);
                Err(error)
            }
        }
    }

    #[allow(non_snake_case)]
    pub async fn summarizeMemory(
        enhancedAiService: &mut EnhancedAIService,
        messages: Vec<ChatMessage>,
        autoContinue: bool,
        isGroupChat: bool,
    ) -> Result<Option<ChatMessage>, operit_providers::chat::llmprovider::AIService::AiServiceError>
    {
        let lastSummaryIndex = messages
            .iter()
            .rposition(|message| message.sender == "summary");
        let previousSummary = lastSummaryIndex.and_then(|index| {
            let content = messages[index].displayText().trim().to_string();
            if content.is_empty() {
                None
            } else {
                Some(content)
            }
        });

        let messagesToSummarize = match lastSummaryIndex {
            Some(index) => messages[index + 1..]
                .iter()
                .filter(|message| message.sender == "user" || message.sender == "ai")
                .cloned()
                .collect::<Vec<_>>(),
            None => messages
                .iter()
                .filter(|message| message.sender == "user" || message.sender == "ai")
                .cloned()
                .collect::<Vec<_>>(),
        };

        if messagesToSummarize.is_empty() {
            return Ok(None);
        }

        let mut conversationReviewEntries = Vec::<(String, String)>::new();
        let conversationToSummarize = if isGroupChat {
            let mut packedContent = String::new();
            for message in &messagesToSummarize {
                let cleanedContent = cleanSummarySourceMessage(message);
                if cleanedContent.trim().is_empty() {
                    continue;
                }
                let displayContent = if message.sender == "ai" {
                    condenseAssistantMessageForReview(message)
                } else {
                    condenseUserForReview(&cleanedContent)
                };
                let speakerLabel = summarySpeakerLabel(message);
                conversationReviewEntries.push((speakerLabel.clone(), displayContent));
                if !packedContent.is_empty() {
                    packedContent.push(' ');
                }
                packedContent.push_str(&format!("{speakerLabel}: {cleanedContent}"));
            }
            vec![("user".to_string(), packedContent)]
        } else {
            messagesToSummarize
                .iter()
                .enumerate()
                .map(|(index, message)| {
                    let role = if message.sender == "user" {
                        "user".to_string()
                    } else {
                        "assistant".to_string()
                    };
                    let cleanedContent = cleanSummarySourceMessage(message);
                    if !cleanedContent.trim().is_empty() {
                        let displayContent = if role == "assistant" {
                            condenseAssistantMessageForReview(message)
                        } else {
                            condenseUserForReview(&cleanedContent)
                        };
                        conversationReviewEntries
                            .push((summarySpeakerLabel(message), displayContent));
                    }
                    (role, format!("#{}: {cleanedContent}", index + 1))
                })
                .collect::<Vec<_>>()
        };

        let summary = enhancedAiService
            .generateSummary(conversationToSummarize, previousSummary)
            .await?;
        if summary.trim().is_empty() {
            return Ok(None);
        }

        let mut summaryWithQuotes = summary.trim().to_string();
        if !conversationReviewEntries.is_empty() {
            summaryWithQuotes.push_str("\n\n【对话回顾】\n");
            for (speaker, content) in conversationReviewEntries {
                summaryWithQuotes.push_str("- ");
                summaryWithQuotes.push_str(&speaker);
                summaryWithQuotes.push_str(": ");
                summaryWithQuotes.push_str(&content);
                summaryWithQuotes.push('\n');
            }
        }

        let finalSummary = if autoContinue {
            format!(
                "{}\n\n如果任务尚未完成，请基于以上摘要继续。",
                summaryWithQuotes.trim_end()
            )
        } else {
            summaryWithQuotes.trim_end().to_string()
        };

        Ok(Some(ChatMessage {
            sender: "summary".to_string(),
            parts: vec![operit_model::MessagePart::MessagePart::markdown(
                "part-0".to_string(),
                0,
                finalSummary,
            )],
            timestamp: ChatMessageTimestampAllocator::next(),
            roleName: "system".to_string(),
            ..ChatMessage::new("summary".to_string())
        }))
    }

    #[allow(non_snake_case)]
    pub async fn calculateStableContextWindow(
        request: StableContextWindowRequest<'_>,
    ) -> Result<i64, operit_providers::chat::llmprovider::AIService::AiServiceError> {
        let memory = Self::getMemoryFromMessages(
            request.chatHistory,
            request.splitHistoryByRole,
            request.currentRoleName,
            request.groupOrchestrationMode,
        );
        request
            .enhancedAiService
            .estimateRequestWindowFromMemory(
                request.messageContent,
                memory,
                request.chatId,
                request.workspacePath,
                request.promptFunctionType,
                request.roleCardId,
                request.groupOrchestrationMode,
                request.groupParticipantNamesText,
                request.proxySenderName,
                request.chatProviderIdOverride,
                request.chatModelIdOverride,
                request.publishEstimate,
                request.runtime,
            )
            .await
    }

    #[allow(non_snake_case)]
    pub fn shouldGenerateSummary(
        messages: Vec<ChatMessage>,
        currentTokens: i64,
        maxTokens: i32,
        tokenUsageThreshold: f64,
        enableSummary: bool,
        enableSummaryByMessageCount: bool,
        summaryMessageCountThreshold: i32,
    ) -> bool {
        if !enableSummary {
            return false;
        }
        if maxTokens > 0 {
            let usageRatio = currentTokens as f64 / f64::from(maxTokens);
            if usageRatio >= tokenUsageThreshold {
                return true;
            }
        }
        if enableSummaryByMessageCount {
            let lastSummaryIndex = messages
                .iter()
                .rposition(|message| message.sender == "summary");
            let relevantMessages = match lastSummaryIndex {
                Some(index) => &messages[index + 1..],
                None => messages.as_slice(),
            };
            let userAiMessagesSinceLastSummary = relevantMessages
                .iter()
                .filter(|message| message.sender == "user")
                .count() as i32;
            return userAiMessagesSinceLastSummary >= summaryMessageCountThreshold;
        }
        false
    }

    #[allow(non_snake_case)]
    pub fn getMemoryFromMessages(
        messages: Vec<ChatMessage>,
        splitByRole: bool,
        targetRoleName: Option<String>,
        groupOrchestrationMode: bool,
    ) -> Vec<PromptTurn> {
        let lastSummaryIndex = messages
            .iter()
            .rposition(|message| message.sender == "summary");
        let relevantMessages = match lastSummaryIndex {
            Some(index) => &messages[index..],
            None => messages.as_slice(),
        };
        let normalizedTargetRole = match targetRoleName {
            Some(roleName) => roleName.trim().to_string(),
            None => String::new(),
        };
        let isRoleScopedMode = splitByRole && !normalizedTargetRole.is_empty();

        relevantMessages
            .iter()
            .flat_map(|message| match message.sender.as_str() {
                "ai" => Self::processAiMessage(message, isRoleScopedMode, &normalizedTargetRole),
                "user" => vec![Self::processUserMessage(
                    message,
                    isRoleScopedMode,
                    groupOrchestrationMode,
                )],
                "summary" => vec![PromptTurn::new(
                    PromptTurnKind::SUMMARY,
                    message.displayText(),
                )],
                _ => Vec::new(),
            })
            .collect()
    }

    #[allow(non_snake_case)]
    fn processAiMessage(
        message: &ChatMessage,
        isRoleScopedMode: bool,
        targetRoleName: &str,
    ) -> Vec<PromptTurn> {
        if !isRoleScopedMode {
            return Self::assistantPromptTurnsFromParts(message);
        }

        let messageRoleName = message.roleName.trim();
        if messageRoleName == targetRoleName {
            return Self::assistantPromptTurnsFromParts(message);
        }

        let cleanedContent = message
            .parts
            .iter()
            .filter(|part| part.kind == MessagePartKind::Markdown)
            .map(|part| part.content.as_str())
            .collect::<String>();
        if cleanedContent.trim().is_empty() {
            return Vec::new();
        }

        let roleLabel = if messageRoleName.is_empty() {
            "unknown"
        } else {
            messageRoleName
        };
        vec![PromptTurn::new(
            PromptTurnKind::USER,
            format!("[From role: {roleLabel}]\n{cleanedContent}"),
        )]
    }

    /// Converts stored assistant parts into provider turns while preserving tool boundaries.
    #[allow(non_snake_case)]
    fn assistantPromptTurnsFromParts(message: &ChatMessage) -> Vec<PromptTurn> {
        let mut turns = Vec::new();
        let mut assistantMarkup = String::new();
        for part in MessagePartCodec::orderedParts(&message.parts) {
            match part.kind {
                MessagePartKind::Markdown | MessagePartKind::Thinking | MessagePartKind::Status => {
                    assistantMarkup.push_str(&Self::assistantMarkupForPart(part));
                }
                MessagePartKind::ToolCall => {
                    if !assistantMarkup.trim().is_empty() {
                        turns.push(PromptTurn::new(
                            PromptTurnKind::ASSISTANT,
                            assistantMarkup.trim().to_string(),
                        ));
                        assistantMarkup.clear();
                    }
                    turns.push(PromptTurn {
                        kind: PromptTurnKind::TOOL_CALL,
                        content: Self::assistantMarkupForPart(part),
                        tool_name: part.toolName.clone(),
                        metadata: HashMap::new(),
                    });
                }
                MessagePartKind::ToolResult => {
                    if !assistantMarkup.trim().is_empty() {
                        turns.push(PromptTurn::new(
                            PromptTurnKind::ASSISTANT,
                            assistantMarkup.trim().to_string(),
                        ));
                        assistantMarkup.clear();
                    }
                    turns.push(PromptTurn {
                        kind: PromptTurnKind::TOOL_RESULT,
                        content: Self::assistantMarkupForPart(part),
                        tool_name: part.toolName.clone(),
                        metadata: HashMap::new(),
                    });
                }
            }
        }
        if !assistantMarkup.trim().is_empty() {
            turns.push(PromptTurn::new(
                PromptTurnKind::ASSISTANT,
                assistantMarkup.trim().to_string(),
            ));
        }
        turns
    }

    /// Serializes one assistant message part into provider protocol markup.
    #[allow(non_snake_case)]
    fn assistantMarkupForPart(part: &MessagePart) -> String {
        MessagePartCodec::assistantMarkup(&[part.clone()])
    }

    #[allow(non_snake_case)]
    fn processUserMessage(
        message: &ChatMessage,
        isRoleScopedMode: bool,
        groupOrchestrationMode: bool,
    ) -> PromptTurn {
        let baseContent = message.displayText();
        if groupOrchestrationMode && isRoleScopedMode {
            let trimmed = baseContent.trim();
            if trimmed.is_empty() {
                return PromptTurn::new(PromptTurnKind::USER, baseContent);
            }
            if trimmed.starts_with("[From user]") {
                return PromptTurn::new(PromptTurnKind::USER, trimmed.to_string());
            }
            return PromptTurn::new(PromptTurnKind::USER, format!("[From user]\n{trimmed}"));
        }
        PromptTurn::new(PromptTurnKind::USER, baseContent)
    }

    fn buildReplyTag(message: &ChatMessage) -> String {
        let cleanContent = strip_xml_tags(&message.displayText()).trim().to_string();
        let clipped = if cleanContent.chars().count() > 100 {
            let mut text = cleanContent.chars().take(100).collect::<String>();
            text.push_str("...");
            text
        } else {
            cleanContent
        };
        let roleName = if message.roleName.trim().is_empty() {
            if message.sender == "ai" {
                "AI".to_string()
            } else {
                "user".to_string()
            }
        } else {
            message.roleName.clone()
        };
        format!(
            "<reply_to sender=\"{}\" timestamp=\"{}\">replying to previous message \"{}\"</reply_to>",
            roleName, message.timestamp, clipped
        )
    }

    /// Builds the serialized markup for one attachment.
    fn buildAttachmentTag(
        attachment: &AttachmentInfo,
        fileSystemHost: Option<&dyn FileSystemHost>,
        enableDirectImageProcessing: bool,
        enableDirectAudioProcessing: bool,
        enableDirectVideoProcessing: bool,
    ) -> Result<String, String> {
        let hasInlineContent = !attachment.content.trim().is_empty();
        if enableDirectImageProcessing
            && attachment
                .mimeType
                .to_ascii_lowercase()
                .starts_with("image/")
        {
            return Self::buildDirectImageAttachmentTag(attachment, fileSystemHost);
        }
        if !hasInlineContent
            && enableDirectAudioProcessing
            && attachment
                .mimeType
                .to_ascii_lowercase()
                .starts_with("audio/")
        {
            return Ok(format!("<audio_link id=\"{}\"/>", attachment.filePath));
        }
        if !hasInlineContent
            && enableDirectVideoProcessing
            && attachment
                .mimeType
                .to_ascii_lowercase()
                .starts_with("video/")
        {
            return Ok(format!("<video_link id=\"{}\"/>", attachment.filePath));
        }

        let attributes = Self::buildAttachmentAttributes(attachment);
        Ok(format!(
            "<attachment {attributes}>{}</attachment>",
            attachment.content
        ))
    }

    /// Builds an attachment tag and media link for a direct image attachment.
    fn buildDirectImageAttachmentTag(
        attachment: &AttachmentInfo,
        fileSystemHost: Option<&dyn FileSystemHost>,
    ) -> Result<String, String> {
        let attributes = Self::buildAttachmentAttributes(attachment);
        let imageId = Self::registerDirectImageAttachment(attachment, fileSystemHost)?;
        let imageLink = MediaLinkBuilder::image(&imageId);
        let mut attachedContent = "Image content has been attached as multimodal input with this message. Do not call file reading tools to read this path.".to_string();
        if !attachment.content.trim().is_empty() {
            attachedContent.push('\n');
            attachedContent.push_str(&attachment.content);
        }
        Ok(format!(
            "{imageLink} <attachment {attributes}>{attachedContent}</attachment>"
        ))
    }

    /// Registers an image attachment in the process image pool.
    fn registerDirectImageAttachment(
        attachment: &AttachmentInfo,
        fileSystemHost: Option<&dyn FileSystemHost>,
    ) -> Result<String, String> {
        let Some(fileSystemHost) = fileSystemHost else {
            return Err("FileSystemHost is required for direct image attachment".to_string());
        };
        let bytes = fileSystemHost
            .readFileBytes(&attachment.filePath)
            .map_err(|error| error.message)?;
        if bytes.is_empty() {
            return Err(format!(
                "image attachment is empty: {}",
                attachment.filePath
            ));
        }
        let imageId = ImagePoolManager::add_image_bytes(&bytes, Some(&attachment.mimeType), None);
        if imageId == "error" {
            return Err(format!(
                "image attachment could not be registered: {}",
                attachment.filePath
            ));
        }
        Ok(imageId)
    }

    /// Builds the XML attributes for an attachment notice.
    fn buildAttachmentAttributes(attachment: &AttachmentInfo) -> String {
        let mut attributes = format!(
            "id=\"{}\" filename=\"{}\" type=\"{}\"",
            attachment.filePath, attachment.fileName, attachment.mimeType
        );
        if attachment.fileSize > 0 {
            attributes.push_str(&format!(" size=\"{}\"", attachment.fileSize));
        }
        attributes
    }

    fn rememberActiveChatKey(chatKey: String) {
        let map = ACTIVE_CHAT_KEYS.get_or_init(|| Mutex::new(HashMap::new()));
        let mut guard = map.lock().expect("active chat key mutex poisoned");
        guard.insert(chatKey.clone(), chatKey);
    }

    fn setLastActiveChatKey(chatKey: String) {
        let lock = LAST_ACTIVE_CHAT_KEY.get_or_init(|| Mutex::new(DEFAULT_CHAT_KEY.to_string()));
        let mut guard = lock.lock().expect("last active chat key mutex poisoned");
        *guard = chatKey;
    }

    fn forgetActiveChatKey(chatKey: &str) {
        let map = ACTIVE_CHAT_KEYS.get_or_init(|| Mutex::new(HashMap::new()));
        let mut guard = map.lock().expect("active chat key mutex poisoned");
        guard.remove(chatKey);
    }

    #[allow(non_snake_case)]
    fn limitMediaLinksInChatHistory(
        history: Vec<PromptTurn>,
        keepLastUserMediaTurns: i32,
    ) -> Vec<PromptTurn> {
        let limit = keepLastUserMediaTurns.max(0) as usize;
        let totalUserTurns = history
            .iter()
            .filter(|turn| turn.kind == PromptTurnKind::USER)
            .count();
        let keepFromTurn = totalUserTurns.saturating_sub(limit);

        let mut currentUserTurnIndex = usize::MAX;
        history
            .into_iter()
            .map(|turn| {
                if turn.kind == PromptTurnKind::USER {
                    currentUserTurnIndex = currentUserTurnIndex.saturating_add(1);
                }
                let shouldKeepMedia = limit > 0 && currentUserTurnIndex >= keepFromTurn;
                if !shouldKeepMedia && MediaLinkParser::has_media_links(&turn.content) {
                    let removed = MediaLinkParser::remove_media_links(&turn.content)
                        .trim()
                        .to_string();
                    turn.with_content(if removed.is_empty() {
                        "[Media omitted]".to_string()
                    } else {
                        removed
                    })
                } else {
                    turn
                }
            })
            .collect()
    }

    #[allow(non_snake_case)]
    fn limitImageLinksInChatHistory(
        history: Vec<PromptTurn>,
        keepLastUserImageTurns: i32,
    ) -> Vec<PromptTurn> {
        let limit = keepLastUserImageTurns.max(0) as usize;
        let totalUserTurns = history
            .iter()
            .filter(|turn| turn.kind == PromptTurnKind::USER)
            .count();
        let keepFromTurn = totalUserTurns.saturating_sub(limit);

        let mut currentUserTurnIndex = usize::MAX;
        history
            .into_iter()
            .map(|turn| {
                if turn.kind == PromptTurnKind::USER {
                    currentUserTurnIndex = currentUserTurnIndex.saturating_add(1);
                }
                let shouldKeepImages = limit > 0 && currentUserTurnIndex >= keepFromTurn;
                if !shouldKeepImages && MediaLinkParser::has_image_links(&turn.content) {
                    let removed = MediaLinkParser::remove_image_links(&turn.content)
                        .trim()
                        .to_string();
                    turn.with_content(if removed.is_empty() {
                        "[Image omitted]".to_string()
                    } else {
                        removed
                    })
                } else {
                    turn
                }
            })
            .collect()
    }

    #[allow(non_snake_case)]
    pub async fn cancelCurrentOperation() {
        let lock = LAST_ACTIVE_CHAT_KEY.get_or_init(|| Mutex::new(DEFAULT_CHAT_KEY.to_string()));
        let chatKey = lock
            .lock()
            .expect("last active chat key mutex poisoned")
            .clone();
        Self::cancelOperation(chatKey).await;
    }

    #[allow(non_snake_case)]
    pub async fn cancelOperation(chatId: String) {
        let chatKey = if chatId.trim().is_empty() {
            DEFAULT_CHAT_KEY.to_string()
        } else {
            chatId
        };
        if let Some(stream) = Self::takeActiveResponseStream(&chatKey) {
            stream.close();
        }
        if let Some(mut service) = Self::cloneActiveEnhancedAiService(&chatKey) {
            service.cancelConversation().await;
        }
    }

    #[allow(non_snake_case)]
    pub async fn cancelAllOperations() {
        let keys = {
            let map = ACTIVE_CHAT_KEYS.get_or_init(|| Mutex::new(HashMap::new()));
            map.lock()
                .expect("active chat key mutex poisoned")
                .keys()
                .cloned()
                .collect::<Vec<_>>()
        };
        let service_keys = {
            let map =
                ACTIVE_ENHANCED_AI_SERVICE_BY_CHAT_ID.get_or_init(|| Mutex::new(HashMap::new()));
            map.lock()
                .expect("active enhanced ai service mutex poisoned")
                .keys()
                .cloned()
                .collect::<Vec<_>>()
        };
        let stream_keys = {
            let map = ACTIVE_RESPONSE_STREAM_BY_CHAT_ID.get_or_init(|| Mutex::new(HashMap::new()));
            map.lock()
                .expect("active response stream mutex poisoned")
                .keys()
                .cloned()
                .collect::<Vec<_>>()
        };
        let keys = keys
            .into_iter()
            .chain(service_keys)
            .chain(stream_keys)
            .collect::<std::collections::BTreeSet<_>>();
        for key in keys {
            Self::cancelOperation(key).await;
        }
    }

    #[allow(non_snake_case)]
    fn rememberActiveEnhancedAiService(chatKey: String, enhancedAiService: EnhancedAIService) {
        let map = ACTIVE_ENHANCED_AI_SERVICE_BY_CHAT_ID.get_or_init(|| Mutex::new(HashMap::new()));
        map.lock()
            .expect("active enhanced ai service mutex poisoned")
            .insert(chatKey, enhancedAiService);
    }

    #[allow(non_snake_case)]
    fn forgetActiveEnhancedAiService(chatKey: &str) {
        let map = ACTIVE_ENHANCED_AI_SERVICE_BY_CHAT_ID.get_or_init(|| Mutex::new(HashMap::new()));
        map.lock()
            .expect("active enhanced ai service mutex poisoned")
            .remove(chatKey);
    }

    #[allow(non_snake_case)]
    fn cloneActiveEnhancedAiService(chatKey: &str) -> Option<EnhancedAIService> {
        let map = ACTIVE_ENHANCED_AI_SERVICE_BY_CHAT_ID.get_or_init(|| Mutex::new(HashMap::new()));
        map.lock()
            .expect("active enhanced ai service mutex poisoned")
            .get(chatKey)
            .cloned()
    }

    #[allow(non_snake_case)]
    fn rememberActiveResponseStream(chatKey: String, responseStream: SharedAiResponseStream) {
        let map = ACTIVE_RESPONSE_STREAM_BY_CHAT_ID.get_or_init(|| Mutex::new(HashMap::new()));
        map.lock()
            .expect("active response stream mutex poisoned")
            .insert(chatKey, responseStream);
    }

    #[allow(non_snake_case)]
    fn forgetActiveResponseStream(chatKey: &str) {
        let map = ACTIVE_RESPONSE_STREAM_BY_CHAT_ID.get_or_init(|| Mutex::new(HashMap::new()));
        map.lock()
            .expect("active response stream mutex poisoned")
            .remove(chatKey);
    }

    #[allow(non_snake_case)]
    fn takeActiveResponseStream(chatKey: &str) -> Option<SharedAiResponseStream> {
        let map = ACTIVE_RESPONSE_STREAM_BY_CHAT_ID.get_or_init(|| Mutex::new(HashMap::new()));
        map.lock()
            .expect("active response stream mutex poisoned")
            .remove(chatKey)
    }
}

fn strip_xml_tags(input: &str) -> String {
    let mut output = String::new();
    let mut inside_tag = false;
    for character in input.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => output.push(character),
            _ => {}
        }
    }
    output
}

fn remove_thinking_content(input: &str) -> String {
    let mut remaining = input.to_string();
    loop {
        let Some(start) = remaining.find("<think>") else {
            break;
        };
        let Some(end_relative) = remaining[start + "<think>".len()..].find("</think>") else {
            break;
        };
        let end = start + "<think>".len() + end_relative + "</think>".len();
        remaining.replace_range(start..end, " ");
    }
    remaining
}

fn remove_status_tags(input: &str) -> String {
    input
        .replace("<status>", " ")
        .replace("</status>", " ")
        .replace("<status/>", " ")
        .trim()
        .to_string()
}

#[allow(non_snake_case)]
fn cleanSummarySourceMessage(message: &ChatMessage) -> String {
    let source = if message.sender == "ai" {
        message.assistantProtocolMarkup()
    } else {
        message.displayText()
    };
    let mut cleaned = strip_tag_blocks(&source, "memory");
    if message.sender == "ai" {
        cleaned = remove_thinking_content(&cleaned);
    }
    strip_media_links(&cleaned).trim().to_string()
}

#[allow(non_snake_case)]
fn summarySpeakerLabel(message: &ChatMessage) -> String {
    if message.sender == "user" {
        "user".to_string()
    } else if message.roleName.trim().is_empty() {
        "AI".to_string()
    } else {
        message.roleName.clone()
    }
}

#[allow(non_snake_case)]
fn condenseUserForReview(text: &str) -> String {
    let pruned = strip_tag_blocks(
        &strip_tag_blocks(
            &strip_tag_blocks(text, "workspace_attachment"),
            "attachment",
        ),
        "reply_to",
    );
    condense_head_tail(&strip_xml_tags(&pruned), 240, 96)
}

/// Condenses structured assistant parts without exposing full tool payloads in the review.
#[allow(non_snake_case)]
fn condenseAssistantMessageForReview(message: &ChatMessage) -> String {
    let mut segments = Vec::<String>::new();
    for part in MessagePartCodec::orderedParts(&message.parts) {
        let segment = match part.kind {
            MessagePartKind::Markdown | MessagePartKind::Status => {
                let cleaned = strip_xml_tags(&strip_media_links(&strip_tag_blocks(
                    &part.content,
                    "memory",
                )));
                let condensed = condense_head_tail(&cleaned, 120, 48);
                (!condensed.trim().is_empty()).then_some(condensed)
            }
            MessagePartKind::Thinking => None,
            MessagePartKind::ToolCall => {
                let tool_name = part
                    .toolName
                    .as_deref()
                    .expect("tool-call parts require a tool name");
                let params = part
                    .attributes
                    .iter()
                    .take(8)
                    .map(|(name, value)| {
                        format!(
                            "{}={}",
                            name,
                            condense_head_tail(&strip_xml_tags(value), 72, 32)
                        )
                    })
                    .collect::<Vec<_>>();
                let params_text = if params.is_empty() {
                    String::new()
                } else {
                    format!(" {}", params.join("; "))
                };
                Some(format!("[工具: {tool_name}]{params_text}"))
            }
            MessagePartKind::ToolResult => {
                let tool_name = part
                    .toolName
                    .as_deref()
                    .expect("tool-result parts require a tool name");
                let status = part
                    .attributes
                    .get("status")
                    .map(|value| match value.to_ascii_lowercase().as_str() {
                        "success" => "成功",
                        "error" => "失败",
                        _ => value.as_str(),
                    })
                    .unwrap_or("");
                let result = condense_head_tail(&strip_xml_tags(&part.content), 140, 56);
                let status_text = if status.is_empty() {
                    String::new()
                } else {
                    format!(" {status}")
                };
                let result_text = if result.is_empty() {
                    String::new()
                } else {
                    format!(" {result}")
                };
                Some(format!("[结果: {tool_name}{status_text}]{result_text}"))
            }
        };
        if let Some(segment) = segment {
            segments.push(segment);
        }
    }

    if segments.is_empty() {
        return "[Empty]".to_string();
    }
    if segments.len() > 25 {
        let omitted = segments.len() - 22;
        let mut bounded = segments[..12].to_vec();
        bounded.push(format!("[...省略{omitted}段...]"));
        bounded.extend_from_slice(&segments[segments.len() - 10..]);
        segments = bounded;
    }
    segments.join(" ").trim().to_string()
}

fn condense_head_tail(text: &str, head_chars: usize, tail_chars: usize) -> String {
    let normalized = normalize_for_review(text);
    let total_chars = normalized.chars().count();
    let min_total = head_chars + tail_chars;
    if total_chars <= min_total + 3 {
        return normalized;
    }
    if head_chars == 0 && tail_chars == 0 {
        return "...".to_string();
    }
    if head_chars == 0 {
        return format!(
            "...{}",
            normalized
                .chars()
                .rev()
                .take(tail_chars)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<String>()
        );
    }
    if tail_chars == 0 {
        return format!(
            "{}...",
            normalized.chars().take(head_chars).collect::<String>()
        );
    }
    let head = normalized.chars().take(head_chars).collect::<String>();
    let tail = normalized
        .chars()
        .rev()
        .take(tail_chars)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{head}...{tail}")
}

fn normalize_for_review(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn strip_media_links(text: &str) -> String {
    ["image_link", "audio_link", "video_link", "media_link"]
        .into_iter()
        .fold(text.to_string(), |current, tag| {
            strip_self_closing_tags(&current, tag)
        })
}

fn strip_self_closing_tags(text: &str, tag_name: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    let open_prefix = format!("<{tag_name}");
    while let Some(start_offset) = text[cursor..].find(&open_prefix) {
        let start = cursor + start_offset;
        output.push_str(&text[cursor..start]);
        let Some(end_offset) = text[start..].find("/>") else {
            output.push_str(&text[start..]);
            return output;
        };
        cursor = start + end_offset + 2;
    }
    output.push_str(&text[cursor..]);
    output
}

fn strip_tag_blocks(text: &str, tag_name: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    let open_prefix = format!("<{tag_name}");
    let close_tag = format!("</{tag_name}>");
    while let Some(start_offset) = text[cursor..].find(&open_prefix) {
        let start = cursor + start_offset;
        output.push_str(&text[cursor..start]);
        let Some(open_end_offset) = text[start..].find('>') else {
            output.push_str(&text[start..]);
            return output;
        };
        let body_start = start + open_end_offset + 1;
        let Some(close_offset) = text[body_start..].find(&close_tag) else {
            output.push_str(&text[start..]);
            return output;
        };
        cursor = body_start + close_offset + close_tag.len();
    }
    output.push_str(&text[cursor..]);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Verifies persisted assistant messages keep tool protocol roles in provider history.
    #[test]
    fn assistant_history_preserves_tool_turn_boundaries() {
        let message = ChatMessage::new_with_parts(
            "ai".to_string(),
            vec![
                MessagePart::markdown("part-0".to_string(), 0, "before".to_string()),
                MessagePart::toolCall(
                    "part-1".to_string(),
                    1,
                    "part-1".to_string(),
                    "switch_core".to_string(),
                    BTreeMap::from([("node_id".to_string(), "core-target".to_string())]),
                ),
                MessagePart::toolResult(
                    "part-2".to_string(),
                    2,
                    Some("part-1".to_string()),
                    "switch_core".to_string(),
                    "success".to_string(),
                    "core-target".to_string(),
                ),
                MessagePart::markdown("part-3".to_string(), 3, "after".to_string()),
            ],
        );

        let turns = AIMessageManager::getMemoryFromMessages(vec![message], false, None, false);

        assert_eq!(
            turns
                .iter()
                .map(|turn| turn.kind.clone())
                .collect::<Vec<_>>(),
            vec![
                PromptTurnKind::ASSISTANT,
                PromptTurnKind::TOOL_CALL,
                PromptTurnKind::TOOL_RESULT,
                PromptTurnKind::ASSISTANT,
            ]
        );
        assert_eq!(turns[0].content, "before");
        assert!(turns[1].content.starts_with("<tool name=\"switch_core\""));
        assert_eq!(turns[1].tool_name.as_deref(), Some("switch_core"));
        assert!(
            turns[2]
                .content
                .starts_with("<tool_result name=\"switch_core\"")
        );
        assert_eq!(turns[2].tool_name.as_deref(), Some("switch_core"));
        assert_eq!(turns[3].content, "after");
    }

    /// Verifies summary review condenses structured tool results instead of dumping payloads.
    #[test]
    fn summary_review_condenses_structured_tool_result() {
        let message = ChatMessage::new_with_parts(
            "ai".to_string(),
            vec![
                MessagePart::markdown("part-0".to_string(), 0, "已检查设备".to_string()),
                MessagePart::toolResult(
                    "part-1".to_string(),
                    1,
                    Some("call-1".to_string()),
                    "system_info".to_string(),
                    "success".to_string(),
                    format!("{{\"nodes\":[{}]}}", "{\"nodeId\":\"core\"},".repeat(80)),
                ),
            ],
        );

        let review = condenseAssistantMessageForReview(&message);

        assert!(review.contains("[结果: system_info 成功]"));
        assert!(review.contains("..."));
        assert!(review.len() < 500);
    }
}
