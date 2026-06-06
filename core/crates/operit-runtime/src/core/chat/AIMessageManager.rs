use std::collections::HashMap;
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};

use crate::api::chat::llmprovider::AIService::SharedAiResponseStream;
use crate::api::chat::llmprovider::MediaLinkParser::MediaLinkParser;
use crate::api::chat::EnhancedAIService::{
    EnhancedAIService, SendMessageCallbacks, SendMessageOptions, SendMessageRuntime,
};
use crate::core::chat::hooks::PromptTurn::{PromptTurn, PromptTurnKind};
use crate::core::chat::plugins::MessageProcessingPluginRegistry::{
    MessageProcessingHookParams, MessageProcessingPluginRegistry,
};
use crate::data::model::AttachmentInfo::AttachmentInfo;
use crate::data::model::ChatMessage::ChatMessage;
use crate::data::model::ChatMessageTimestampAllocator::ChatMessageTimestampAllocator;
use crate::data::model::PromptFunctionType::PromptFunctionType;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::util::stream::HotStream::StreamStart;
use crate::util::stream::RevisableTextStream::{share_revisable, with_event_channel_shared};
use operit_store::PreferencesDataStore::FlowLike;

const DEFAULT_CHAT_KEY: &str = "__DEFAULT_CHAT__";

pub struct AIMessageManager;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageTiming {
    pub startedAtMs: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuildUserMessageContentRequest {
    pub messageText: String,
    pub proxySenderName: Option<String>,
    pub attachments: Vec<AttachmentInfo>,
    pub workspacePath: Option<String>,
    pub workspaceEnv: Option<String>,
    pub replyToMessage: Option<ChatMessage>,
    pub enableDirectImageProcessing: bool,
    pub enableDirectAudioProcessing: bool,
    pub enableDirectVideoProcessing: bool,
    pub chatId: Option<String>,
    pub roleCardId: Option<String>,
}

pub struct SendMessageRequest<'a> {
    pub enhancedAiService: &'a mut EnhancedAIService,
    pub chatId: Option<String>,
    pub messageContent: String,
    pub chatHistory: Vec<ChatMessage>,
    pub workspacePath: Option<String>,
    pub workspaceEnv: Option<String>,
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
    pub chatModelConfigIdOverride: Option<String>,
    pub chatModelIndexOverride: Option<i32>,
    pub preferenceProfileIdOverride: Option<String>,
    pub disableWarning: bool,
    pub callbacks: Option<Arc<dyn SendMessageCallbacks + Send + Sync>>,
    pub onToolInvocation: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

pub struct StableContextWindowRequest<'a> {
    pub enhancedAiService: &'a mut EnhancedAIService,
    pub chatId: Option<String>,
    pub messageContent: String,
    pub chatHistory: Vec<ChatMessage>,
    pub workspacePath: Option<String>,
    pub workspaceEnv: Option<String>,
    pub promptFunctionType: PromptFunctionType,
    pub roleCardId: Option<String>,
    pub currentRoleName: Option<String>,
    pub splitHistoryByRole: bool,
    pub groupOrchestrationMode: bool,
    pub groupParticipantNamesText: Option<String>,
    pub proxySenderName: Option<String>,
    pub chatModelConfigIdOverride: Option<String>,
    pub chatModelIndexOverride: Option<i32>,
    pub preferenceProfileIdOverride: Option<String>,
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

pub fn logMessageTiming(_stage: &str, _startTimeMs: MessageTiming, _details: Option<String>) {}

impl AIMessageManager {
    pub fn initialize() {
        let _ = ACTIVE_CHAT_KEYS.get_or_init(|| Mutex::new(HashMap::new()));
        let _ = LAST_ACTIVE_CHAT_KEY.get_or_init(|| Mutex::new(DEFAULT_CHAT_KEY.to_string()));
        let _ = ACTIVE_ENHANCED_AI_SERVICE_BY_CHAT_ID.get_or_init(|| Mutex::new(HashMap::new()));
        let _ = ACTIVE_RESPONSE_STREAM_BY_CHAT_ID.get_or_init(|| Mutex::new(HashMap::new()));
    }

    #[allow(non_snake_case)]
    pub fn buildUserMessageContent(request: BuildUserMessageContentRequest) -> String {
        let processedMessageText = request.messageText;
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
                let workspaceEnv = match request.workspaceEnv {
                    Some(value) => value,
                    None => String::new(),
                };
                format!("<workspace_attachment>{workspaceEnv}</workspace_attachment>")
            }
            _ => String::new(),
        };

        let attachmentTags = request
            .attachments
            .iter()
            .map(|attachment| {
                Self::buildAttachmentTag(
                    attachment,
                    request.enableDirectImageProcessing,
                    request.enableDirectAudioProcessing,
                    request.enableDirectVideoProcessing,
                )
            })
            .collect::<Vec<_>>()
            .join(" ");

        [
            proxySenderTag,
            processedMessageText,
            attachmentTags,
            workspaceTag,
            replyTag,
        ]
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ")
    }

    #[allow(non_snake_case)]
    pub async fn sendMessage(
        request: SendMessageRequest<'_>,
    ) -> Result<
        crate::api::chat::llmprovider::AIService::SharedAiResponseStream,
        crate::api::chat::llmprovider::AIService::AiServiceError,
    > {
        let chatKey = match &request.chatId {
            Some(chatId) => chatId.clone(),
            None => DEFAULT_CHAT_KEY.to_string(),
        };
        Self::rememberActiveChatKey(chatKey.clone());
        Self::setLastActiveChatKey(chatKey.clone());
        Self::rememberActiveEnhancedAiService(chatKey.clone(), request.enhancedAiService.clone());

        let memory = Self::getMemoryFromMessages(
            request.chatHistory.clone(),
            request.splitHistoryByRole,
            request.currentRoleName.clone(),
            request.groupOrchestrationMode,
        );

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
            Self::forgetActiveChatKey(&chatKey);
            Self::forgetActiveEnhancedAiService(&chatKey);
            return Ok(with_event_channel_shared(
                pluginExecution.stream,
                crate::util::stream::HotStream::mutable_shared_stream(usize::MAX),
            ));
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
        options.workspaceEnv = request.workspaceEnv;
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
        options.chatModelConfigIdOverride = request.chatModelConfigIdOverride;
        options.chatModelIndexOverride = request.chatModelIndexOverride;
        options.preferenceProfileIdOverride = request.preferenceProfileIdOverride;
        options.disableWarning = request.disableWarning;
        options.callbacks = request.callbacks;
        options.onToolInvocation = request.onToolInvocation;
        options.stream = enableStream;

        let result = request
            .enhancedAiService
            .sendMessage(options)
            .await
            .map(|stream| {
                let shared = share_revisable(
                    ActiveChatTextStream {
                        chatKey: chatKey.clone(),
                        stream,
                    },
                    usize::MAX,
                    StreamStart::Lazily,
                );
                Self::rememberActiveResponseStream(chatKey.clone(), shared.clone());
                shared
            });
        if result.is_err() {
            Self::forgetActiveChatKey(&chatKey);
            Self::forgetActiveEnhancedAiService(&chatKey);
            Self::forgetActiveResponseStream(&chatKey);
        }
        result
    }

    #[allow(non_snake_case)]
    pub async fn summarizeMemory(
        enhancedAiService: &mut EnhancedAIService,
        messages: Vec<ChatMessage>,
        autoContinue: bool,
        isGroupChat: bool,
    ) -> Result<Option<ChatMessage>, crate::api::chat::llmprovider::AIService::AiServiceError> {
        let lastSummaryIndex = messages
            .iter()
            .rposition(|message| message.sender == "summary");
        let previousSummary = lastSummaryIndex.and_then(|index| {
            let content = messages[index].content.trim().to_string();
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
                    condenseAssistantForReview(&cleanedContent)
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
                            condenseAssistantForReview(&cleanedContent)
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
            content: finalSummary,
            timestamp: ChatMessageTimestampAllocator::next(),
            roleName: "system".to_string(),
            ..ChatMessage::new("summary".to_string())
        }))
    }

    #[allow(non_snake_case)]
    pub async fn calculateStableContextWindow(
        request: StableContextWindowRequest<'_>,
    ) -> Result<i32, crate::api::chat::llmprovider::AIService::AiServiceError> {
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
                request.workspaceEnv,
                request.promptFunctionType,
                request.roleCardId,
                request.groupOrchestrationMode,
                request.groupParticipantNamesText,
                request.proxySenderName,
                request.chatModelConfigIdOverride,
                request.chatModelIndexOverride,
                request.preferenceProfileIdOverride,
                request.publishEstimate,
                request.runtime,
            )
            .await
    }

    #[allow(non_snake_case)]
    pub fn shouldGenerateSummary(
        messages: Vec<ChatMessage>,
        currentTokens: i32,
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
            let usageRatio = currentTokens as f64 / maxTokens as f64;
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
            .filter_map(|message| match message.sender.as_str() {
                "ai" => Self::processAiMessage(message, isRoleScopedMode, &normalizedTargetRole),
                "user" => Some(Self::processUserMessage(
                    message,
                    isRoleScopedMode,
                    groupOrchestrationMode,
                )),
                "summary" => Some(PromptTurn::new(
                    PromptTurnKind::SUMMARY,
                    message.content.clone(),
                )),
                _ => None,
            })
            .collect()
    }

    #[allow(non_snake_case)]
    fn processAiMessage(
        message: &ChatMessage,
        isRoleScopedMode: bool,
        targetRoleName: &str,
    ) -> Option<PromptTurn> {
        if !isRoleScopedMode {
            return Some(PromptTurn::new(
                PromptTurnKind::ASSISTANT,
                message.content.clone(),
            ));
        }

        let messageRoleName = message.roleName.trim();
        if messageRoleName == targetRoleName {
            return Some(PromptTurn::new(
                PromptTurnKind::ASSISTANT,
                message.content.clone(),
            ));
        }

        let cleanedContent = remove_status_tags(&remove_thinking_content(&message.content));
        if cleanedContent.trim().is_empty() {
            return None;
        }

        let roleLabel = if messageRoleName.is_empty() {
            "unknown"
        } else {
            messageRoleName
        };
        Some(PromptTurn::new(
            PromptTurnKind::USER,
            format!("[From role: {roleLabel}]\n{cleanedContent}"),
        ))
    }

    #[allow(non_snake_case)]
    fn processUserMessage(
        message: &ChatMessage,
        isRoleScopedMode: bool,
        groupOrchestrationMode: bool,
    ) -> PromptTurn {
        let baseContent = message.content.clone();
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
        let cleanContent = strip_xml_tags(&message.content).trim().to_string();
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

    fn buildAttachmentTag(
        attachment: &AttachmentInfo,
        enableDirectImageProcessing: bool,
        enableDirectAudioProcessing: bool,
        enableDirectVideoProcessing: bool,
    ) -> String {
        let hasInlineContent = !attachment.content.trim().is_empty();
        if !hasInlineContent
            && enableDirectImageProcessing
            && attachment
                .mimeType
                .to_ascii_lowercase()
                .starts_with("image/")
        {
            return format!("<image_link id=\"{}\"/>", attachment.filePath);
        }
        if !hasInlineContent
            && enableDirectAudioProcessing
            && attachment
                .mimeType
                .to_ascii_lowercase()
                .starts_with("audio/")
        {
            return format!("<audio_link id=\"{}\"/>", attachment.filePath);
        }
        if !hasInlineContent
            && enableDirectVideoProcessing
            && attachment
                .mimeType
                .to_ascii_lowercase()
                .starts_with("video/")
        {
            return format!("<video_link id=\"{}\"/>", attachment.filePath);
        }

        let mut attributes = format!(
            "id=\"{}\" filename=\"{}\" type=\"{}\"",
            attachment.filePath, attachment.fileName, attachment.mimeType
        );
        if attachment.fileSize > 0 {
            attributes.push_str(&format!(" size=\"{}\"", attachment.fileSize));
        }
        format!(
            "<attachment {attributes}>{}</attachment>",
            attachment.content
        )
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
    pub fn cancelCurrentOperation() {
        let lock = LAST_ACTIVE_CHAT_KEY.get_or_init(|| Mutex::new(DEFAULT_CHAT_KEY.to_string()));
        let chatKey = lock
            .lock()
            .expect("last active chat key mutex poisoned")
            .clone();
        Self::cancelOperation(chatKey);
    }

    #[allow(non_snake_case)]
    pub fn cancelOperation(chatId: String) {
        let chatKey = if chatId.trim().is_empty() {
            DEFAULT_CHAT_KEY.to_string()
        } else {
            chatId
        };
        if let Some(stream) = Self::takeActiveResponseStream(&chatKey) {
            stream.upstream.close();
            stream.event_channel.close();
        }
        if let Some(mut service) = Self::takeActiveEnhancedAiService(&chatKey) {
            service.cancelConversation();
        }
        Self::forgetActiveChatKey(&chatKey);
    }

    #[allow(non_snake_case)]
    pub fn cancelAllOperations() {
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
            Self::cancelOperation(key);
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
    fn takeActiveEnhancedAiService(chatKey: &str) -> Option<EnhancedAIService> {
        let map = ACTIVE_ENHANCED_AI_SERVICE_BY_CHAT_ID.get_or_init(|| Mutex::new(HashMap::new()));
        map.lock()
            .expect("active enhanced ai service mutex poisoned")
            .remove(chatKey)
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

struct ActiveChatTextStream {
    chatKey: String,
    stream: Box<dyn crate::util::stream::RevisableTextStream::RevisableTextStreamLike>,
}

impl crate::util::stream::Stream::Stream for ActiveChatTextStream {
    type Item = String;

    fn is_locked(&self) -> bool {
        self.stream.is_locked()
    }

    fn buffered_count(&self) -> usize {
        self.stream.buffered_count()
    }

    fn lock(&mut self) {
        self.stream.lock();
    }

    fn unlock(&mut self) {
        self.stream.unlock();
    }

    fn clear_buffer(&mut self) {
        self.stream.clear_buffer();
    }

    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item)) {
        self.stream.collect(collector);
        AIMessageManager::forgetActiveChatKey(&self.chatKey);
        AIMessageManager::forgetActiveEnhancedAiService(&self.chatKey);
        AIMessageManager::forgetActiveResponseStream(&self.chatKey);
    }
}

impl crate::util::stream::RevisableTextStream::TextStreamEventCarrier for ActiveChatTextStream {
    fn event_channel(
        &self,
    ) -> &crate::util::stream::HotStream::MutableSharedStreamImpl<
        crate::util::stream::RevisableTextStream::TextStreamEvent,
    > {
        self.stream.event_channel()
    }
}

impl crate::util::stream::RevisableTextStream::RevisableTextStream for ActiveChatTextStream {}

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
    let mut cleaned = strip_tag_blocks(&message.content, "memory");
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

#[allow(non_snake_case)]
fn condenseAssistantForReview(text: &str) -> String {
    let cleaned = strip_xml_tags(&remove_thinking_content(text));
    let condensed = condense_head_tail(&cleaned, 280, 120);
    if condensed.trim().is_empty() {
        "[Empty]".to_string()
    } else {
        condensed
    }
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
