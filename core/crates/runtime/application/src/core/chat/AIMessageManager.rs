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
use operit_providers::chat::enhance::InputProcessor::{InputProcessor, ProcessUserInputRequest};
use operit_providers::chat::llmprovider::AIService::{AiServiceError, SharedAiResponseStream};
use operit_providers::chat::llmprovider::MediaLinkBuilder::MediaLinkBuilder;
use operit_providers::chat::llmprovider::MediaLinkParser::MediaLinkParser;
use operit_providers::chat::EnhancedAIService::{
    EnhancedAIService, MessageCancellationToken, ResumeRequest, SendMessageCallbacks,
    SendMessageOptions, SendMessageRuntime,
};
use operit_providers::runtime_support::ChatConfigurationResult;
use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
use operit_store::PreferencesDataStore::FlowLike;
use operit_util::stream::RevisableTextStream::with_event_channel_shared;
use operit_util::stream::Stream::Stream;
use operit_util::AppLogger::AppLogger;
use operit_util::ChainLogger::{self, PLUGIN_CHAIN, RECEIVE_CHAIN, SEND_CHAIN};
use operit_util::ImagePoolManager::ImagePoolManager;
use operit_util::MediaPoolManager::MediaPoolManager;

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
    pub participantId: Option<String>,
    /// Host callback used to surface a timed-out Prompt Input Hook.
    pub onHookTimeout: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

pub struct SendMessageRequest<'a> {
    pub chatConfiguration: ChatConfigurationResult,
    pub enhancedAiService: &'a mut EnhancedAIService,
    pub chatId: Option<String>,
    pub messageContent: String,
    pub chatHistory: Vec<ChatMessage>,
    pub promptHistoryOverride: Option<Vec<PromptTurn>>,
    pub workspacePath: Option<String>,
    pub workspaceFolders: Vec<String>,
    pub promptFunctionType: PromptFunctionType,
    pub enableThinking: bool,
    pub enableMemoryAutoUpdate: bool,
    pub maxTokens: i32,
    pub tokenUsageThreshold: f64,
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
    pub workspaceFolders: Vec<String>,
    pub promptFunctionType: PromptFunctionType,
    pub proxySenderName: Option<String>,
    pub chatProviderIdOverride: Option<String>,
    pub chatModelIdOverride: Option<String>,
    pub publishEstimate: bool,
    pub runtime: SendMessageRuntime,
}

/// Owns the cancellation handle and response stream for the same send operation.
struct ActiveChatOperation {
    cancellationToken: MessageCancellationToken,
    enhancedAiService: EnhancedAIService,
    responseStream: Option<SharedAiResponseStream>,
}

/// Prevents a completed or cancelled send from cleaning up a newer send for its chat.
struct ChatOperationRegistry<T> {
    nextOperationId: u64,
    operations: HashMap<String, (u64, T)>,
}

impl<T> Default for ChatOperationRegistry<T> {
    fn default() -> Self {
        Self {
            nextOperationId: 0,
            operations: HashMap::new(),
        }
    }
}

impl<T> ChatOperationRegistry<T> {
    fn register(&mut self, chatKey: String, operation: T) -> u64 {
        self.nextOperationId = self
            .nextOperationId
            .checked_add(1)
            .expect("chat operation id must not overflow");
        let operationId = self.nextOperationId;
        self.operations.insert(chatKey, (operationId, operation));
        operationId
    }

    fn get_mut(&mut self, chatKey: &str, operationId: u64) -> Option<&mut T> {
        self.operations
            .get_mut(chatKey)
            .filter(|(currentId, _)| *currentId == operationId)
            .map(|(_, operation)| operation)
    }

    fn remove(&mut self, chatKey: &str, operationId: u64) -> Option<T> {
        self.get_mut(chatKey, operationId)?;
        self.operations
            .remove(chatKey)
            .map(|(_, operation)| operation)
    }

    fn take(&mut self, chatKey: &str) -> Option<(u64, T)> {
        self.operations.remove(chatKey)
    }

    fn snapshot(&self) -> Vec<(String, u64)> {
        self.operations
            .iter()
            .map(|(chatKey, (operationId, _))| (chatKey.clone(), *operationId))
            .collect()
    }
}

static ACTIVE_CHAT_OPERATIONS: OnceLock<Mutex<ChatOperationRegistry<ActiveChatOperation>>> =
    OnceLock::new();
static LAST_ACTIVE_CHAT_KEY: OnceLock<Mutex<String>> = OnceLock::new();

fn activeChatOperations() -> &'static Mutex<ChatOperationRegistry<ActiveChatOperation>> {
    ACTIVE_CHAT_OPERATIONS.get_or_init(|| Mutex::new(ChatOperationRegistry::default()))
}

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
    /// Attaches a validated execution snapshot only to the newly generated message revision.
    pub fn applyConfigurationSnapshot(
        message: &mut ChatMessage,
        configuration: &ChatConfigurationResult,
    ) -> Result<(), String> {
        configuration.validate()?;
        message.pluginExtensions.insert(
            configuration.extensionOwner.clone(),
            serde_json::Value::Object(configuration.messageExtension.clone()),
        );
        Ok(())
    }

    pub fn initialize() {
        let _ = activeChatOperations();
        let _ = LAST_ACTIVE_CHAT_KEY.get_or_init(|| Mutex::new(DEFAULT_CHAT_KEY.to_string()));
    }

    #[allow(non_snake_case)]
    /// Builds user message content with processed text, attachments, workspace, and reply context.
    pub async fn buildUserMessageContent(
        request: BuildUserMessageContentRequest,
    ) -> Result<String, AiServiceError> {
        let promptInputStartTime = messageTimingNow();
        let originalMessageLength = request.messageText.len();
        let processedMessageText = InputProcessor::process_user_input(ProcessUserInputRequest {
            input: request.messageText,
            chat_id: request.chatId.clone(),
            participant_id: request.participantId.clone(),
            on_hook_timeout: request.onHookTimeout.clone(),
        })
        .await;
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

    /// Serializes original user-only input without selecting a participant, invoking a model, or rewriting submitted text.
    pub(crate) fn buildRecordedUserMessageContent(
        text: &str, attachments: &[AttachmentInfo], reply: Option<&ChatMessage>,
    ) -> Result<String, String> {
        let mut parts = vec![text.to_string()];
        for attachment in attachments {
            parts.push(Self::buildAttachmentTag(attachment, None, false, false, false)?);
        }
        if let Some(reply) = reply { parts.push(Self::buildReplyTag(reply)); }
        Ok(parts.into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" "))
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
        let cancellationToken = MessageCancellationToken::default();
        let operationId = activeChatOperations()
            .lock()
            .expect("active chat operations mutex poisoned")
            .register(
                chatKey.clone(),
                ActiveChatOperation {
                    cancellationToken: cancellationToken.clone(),
                    enhancedAiService: request.enhancedAiService.clone(),
                    responseStream: None,
                },
            );
        Self::setLastActiveChatKey(chatKey.clone());
        ChainLogger::info(
            SEND_CHAIN,
            "send.ai_manager.start",
            &[
                ("chatKey", chatKey.clone()),
                ("operationId", operationId.to_string()),
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
                request.chatHistory.clone()),
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
        )
        .await;
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
            Self::finishOperation(&chatKey, operationId);
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
        options.cancellationToken = Some(cancellationToken);
        options.message = request.messageContent;
        options.chatId = request.chatId;
        options.chatHistory = memoryForRequest;
        options.workspacePath = request.workspacePath;
        options.workspaceFolders = request.workspaceFolders;
        options.promptFunctionType = request.promptFunctionType;
        options.executionParticipantId = Some(request.chatConfiguration.profile.id.clone());
        options.enableThinking = request.enableThinking;
        options.enableMemoryAutoUpdate = request.enableMemoryAutoUpdate;
        options.maxTokens = request.maxTokens;
        options.tokenUsageThreshold = request.tokenUsageThreshold;
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
        // A pre-send hook may have yielded while the user cancelled this operation.
        if activeChatOperations()
            .lock()
            .expect("active chat operations mutex poisoned")
            .get_mut(&chatKey, operationId)
            .is_none()
        {
            return Err(AiServiceError::RequestCancelled);
        }
        options.chatConfiguration = Some(request.chatConfiguration.clone());
        let providerResponse = if request.resume {
            request
                .enhancedAiService
                .resume(ResumeRequest { options })
                .await
        } else {
            request.enhancedAiService.sendMessageWithConfiguration(options, request.chatConfiguration).await
        };
        match providerResponse {
            Ok(stream) => {
                // Register before scheduling cleanup: an already-closed stream can be
                // collected immediately, and a cancelled send must never re-register itself.
                let registered = {
                    let mut operations = activeChatOperations()
                        .lock()
                        .expect("active chat operations mutex poisoned");
                    if let Some(operation) = operations.get_mut(&chatKey, operationId) {
                        operation.responseStream = Some(stream.clone());
                        true
                    } else {
                        false
                    }
                };
                if !registered {
                    stream.close();
                    return Err(AiServiceError::RequestCancelled);
                }
                let cleanupChatKey = chatKey.clone();
                let mut cleanupStream = stream.chunk_stream();
                if let Err(error) = defaultHostRuntimeTaskSchedulerHost()
                    .scheduleHostRuntimeAsyncTask(
                        "ai-message-manager-response-cleanup",
                        Box::new(move || {
                            Box::pin(async move {
                                cleanupStream.collect(&mut |_| {}).await;
                                Self::finishOperation(&cleanupChatKey, operationId);
                            })
                        }),
                    )
                {
                    Self::cancelOperationForId(&chatKey, Some(operationId)).await;
                    return Err(AiServiceError::RequestFailed(error.to_string()));
                }
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
                Self::finishOperation(&chatKey, operationId);
                Err(error)
            }
        }
    }

    #[allow(non_snake_case)]
    pub async fn summarizeMemory(
        enhancedAiService: &mut EnhancedAIService,
        messages: Vec<ChatMessage>,
        autoContinue: bool) -> Result<Option<ChatMessage>, operit_providers::chat::llmprovider::AIService::AiServiceError>
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
        let conversationToSummarize = messagesToSummarize.iter().enumerate().map(|(index, message)| {
            let role = match message.sender.as_str() { "user" => "user", _ => "assistant" }.to_string();
            let cleanedContent = cleanSummarySourceMessage(message);
            if !cleanedContent.trim().is_empty() {
                let displayContent = if message.sender == "ai" {
                    condenseAssistantMessageForReview(message)
                } else { condenseUserForReview(&cleanedContent) };
                conversationReviewEntries.push((summarySpeakerLabel(message), displayContent));
            }
            (role, format!("#{}: {cleanedContent}", index + 1))
        }).collect::<Vec<_>>();

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

    /// Calculates a stable context window with the complete workspace folder set.
    #[allow(non_snake_case)]
    pub async fn calculateStableContextWindow(
        request: StableContextWindowRequest<'_>,
    ) -> Result<i64, operit_providers::chat::llmprovider::AIService::AiServiceError> {
        let memory = Self::getMemoryFromMessages(
            request.chatHistory);
        request
            .enhancedAiService
            .estimateRequestWindowFromMemory(
                request.messageContent,
                memory,
                request.chatId,
                request.workspacePath,
                request.workspaceFolders,
                request.promptFunctionType,
                Some(request.runtime.chatConfiguration.profile.id.clone()),
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
    pub fn getMemoryFromMessages(messages: Vec<ChatMessage>) -> Vec<PromptTurn> {
        let lastSummaryIndex = messages.iter().rposition(|message| message.sender == "summary");
        let relevantMessages = match lastSummaryIndex { Some(index) => &messages[index..], None => messages.as_slice() };
        relevantMessages.iter().flat_map(|message| match message.sender.as_str() {
            "ai" => Self::assistantPromptTurnsFromParts(message),
            "user" => vec![PromptTurn::new(PromptTurnKind::USER, message.displayText())],
            "summary" => vec![PromptTurn::new(PromptTurnKind::SUMMARY, message.displayText())],
            _ => Vec::new(),
        }).collect()
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

    /// Builds display-only reply markup without selecting an execution identity.
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
        let currentNodeId = CoreNodeIdentityStore::localNodeId();
        if !attachment.isLocalToNode(currentNodeId.as_deref()) {
            // File hosts and media pools are node-local. Preserve the source locator instead
            // of attempting to open a remote path (especially fatal for audio/video).
            return Ok(format!(
                "<attachment {}>{}</attachment>",
                Self::buildAttachmentAttributes(attachment),
                attachment.content
            ));
        }
        if enableDirectImageProcessing
            && attachment
                .mimeType
                .to_ascii_lowercase()
                .starts_with("image/")
        {
            match Self::buildDirectImageAttachmentTag(attachment, fileSystemHost) {
                Ok(tag) => return Ok(tag),
                Err(error) => {
                    // Match the Kotlin chain: a failed image-pool registration must fall
                    // back to the ordinary attachment instead of aborting the whole send.
                    AppLogger::e(
                        "AIMessageManager",
                        &format!(
                            "direct image attachment failed, falling back to ordinary attachment: {}: {}",
                            attachment.filePath, error
                        ),
                    );
                }
            }
        }
        if enableDirectAudioProcessing
            && attachment
                .mimeType
                .to_ascii_lowercase()
                .starts_with("audio/")
        {
            return Self::buildDirectMediaAttachmentTag(attachment, fileSystemHost, "audio");
        }
        if enableDirectVideoProcessing
            && attachment
                .mimeType
                .to_ascii_lowercase()
                .starts_with("video/")
        {
            return Self::buildDirectMediaAttachmentTag(attachment, fileSystemHost, "video");
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

    /// Builds a canonical audio or video media link backed by the process media pool.
    fn buildDirectMediaAttachmentTag(
        attachment: &AttachmentInfo,
        fileSystemHost: Option<&dyn FileSystemHost>,
        mediaType: &str,
    ) -> Result<String, String> {
        let mediaId = Self::registerDirectMediaAttachment(attachment, fileSystemHost, mediaType)?;
        let mediaLink = match mediaType {
            "audio" => MediaLinkBuilder::audio(&mediaId),
            "video" => MediaLinkBuilder::video(&mediaId),
            _ => return Err(format!("Unsupported direct media type: {mediaType}")),
        };
        Ok(format!(
            "{mediaLink} <attachment {}>{mediaType} content has been attached as multimodal input with this message. Do not call file reading tools to read this path.</attachment>",
            Self::buildAttachmentAttributes(attachment)
        ))
    }

    /// Registers an audio or video attachment in the process media pool.
    fn registerDirectMediaAttachment(
        attachment: &AttachmentInfo,
        fileSystemHost: Option<&dyn FileSystemHost>,
        mediaType: &str,
    ) -> Result<String, String> {
        let Some(fileSystemHost) = fileSystemHost else {
            return Err(format!(
                "FileSystemHost is required for direct {mediaType} attachment"
            ));
        };
        let bytes = fileSystemHost
            .readFileBytes(&attachment.filePath)
            .map_err(|error| error.message)?;
        if bytes.is_empty() {
            return Err(format!(
                "{mediaType} attachment is empty: {}",
                attachment.filePath
            ));
        }
        let mediaId = MediaPoolManager::add_media_bytes(&bytes, &attachment.mimeType);
        if mediaId == "error" {
            return Err(format!(
                "{mediaType} attachment could not be registered: {}",
                attachment.filePath
            ));
        }
        Ok(mediaId)
    }

    /// Builds the XML attributes for an attachment notice.
    fn buildAttachmentAttributes(attachment: &AttachmentInfo) -> String {
        let escape = |value: &str| {
            value
                .replace('&', "&amp;")
                .replace('"', "&quot;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('\'', "&#39;")
        };
        let toolPath = attachment.fileToolPath();
        let mut attributes = format!(
            "id=\"{}\" filename=\"{}\" type=\"{}\"",
            escape(&toolPath),
            escape(&attachment.fileName),
            escape(&attachment.mimeType)
        );
        if let Some(nodeId) = attachment
            .nodeId
            .as_deref()
            .filter(|nodeId| !nodeId.is_empty())
        {
            attributes.push_str(&format!(" node_id=\"{}\"", escape(nodeId)));
        }
        if toolPath.starts_with("/app/data/temp/clean_on_exit/") {
            attributes.push_str(&format!(" path=\"{}\"", escape(&toolPath)));
        }
        if attachment.fileSize > 0 {
            attributes.push_str(&format!(" size=\"{}\"", attachment.fileSize));
        }
        attributes
    }

    fn setLastActiveChatKey(chatKey: String) {
        let lock = LAST_ACTIVE_CHAT_KEY.get_or_init(|| Mutex::new(DEFAULT_CHAT_KEY.to_string()));
        let mut guard = lock.lock().expect("last active chat key mutex poisoned");
        *guard = chatKey;
    }

    fn finishOperation(chatKey: &str, operationId: u64) {
        activeChatOperations()
            .lock()
            .expect("active chat operations mutex poisoned")
            .remove(chatKey, operationId);
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
        Self::cancelOperationForId(&chatKey, None).await;
    }

    /// Takes ownership of the service before anything can close the response stream.
    async fn cancelOperationForId(chatKey: &str, expectedOperationId: Option<u64>) {
        let operation = {
            let mut operations = activeChatOperations()
                .lock()
                .expect("active chat operations mutex poisoned");
            match expectedOperationId {
                Some(operationId) => operations
                    .remove(chatKey, operationId)
                    .map(|operation| (operationId, operation)),
                None => operations.take(chatKey),
            }
        };
        let Some((operationId, mut operation)) = operation else {
            return;
        };
        // Close the per-send gate immediately, including a send that has not yet
        // entered the provider or registered its execution context.
        operation.cancellationToken.cancel();
        ChainLogger::info(
            SEND_CHAIN,
            "send.cancel.start",
            &[
                ("chatKey", chatKey.to_string()),
                ("operationId", operationId.to_string()),
            ],
        );
        // The token already stops response callbacks and new tool rounds. Close
        // output promptly, without waiting for a busy provider mutex; cleanup cannot
        // lose the service because this call has taken ownership of the whole operation.
        if let Some(stream) = operation.responseStream {
            stream.close();
        }
        operation.enhancedAiService.cancelConversation().await;
        ChainLogger::info(
            SEND_CHAIN,
            "send.cancel.done",
            &[
                ("chatKey", chatKey.to_string()),
                ("operationId", operationId.to_string()),
            ],
        );
    }

    #[allow(non_snake_case)]
    pub async fn cancelAllOperations() {
        let operations = activeChatOperations()
            .lock()
            .expect("active chat operations mutex poisoned")
            .snapshot();
        for (chatKey, operationId) in operations {
            Self::cancelOperationForId(&chatKey, Some(operationId)).await;
        }
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

    #[test]
    fn attachment_markup_keeps_the_source_node_and_portable_tool_path() {
        let mut attachment = AttachmentInfo::new(
            "/device-b/runtime/temp/clean_on_exit/report.pdf".into(),
            "报价单\".pdf".into(),
            "application/pdf".into(),
            3,
        );
        attachment.nodeId = Some("core-b".into());
        let tag =
            AIMessageManager::buildAttachmentTag(&attachment, None, false, false, false).unwrap();
        assert!(tag.contains("node_id=\"core-b\""));
        assert!(tag.contains("path=\"/app/data/temp/clean_on_exit/report.pdf\""));
        assert!(tag.contains("filename=\"报价单&quot;.pdf\""));
        assert!(tag.contains("id=\"/app/data/temp/clean_on_exit/report.pdf\""));
        assert!(!tag.contains("/device-b/"));
    }

    #[test]
    fn remote_media_never_reads_the_current_file_host() {
        for mimeType in ["image/png", "audio/mpeg", "video/mp4"] {
            let mut attachment = AttachmentInfo::new(
                "/device-b/runtime/temp/clean_on_exit/media".into(),
                "media".into(),
                mimeType.into(),
                3,
            );
            attachment.nodeId = Some("test-remote-attachment-node".into());
            // No host: attempting direct audio/video registration would fail the send.
            let tag =
                AIMessageManager::buildAttachmentTag(&attachment, None, true, true, true).unwrap();
            assert!(tag.starts_with("<attachment "));
            assert!(tag.contains("node_id=\"test-remote-attachment-node\""));
            assert!(!tag.contains("_link"));
        }
    }

    #[test]
    fn inline_attachment_markup_keeps_its_content_identifier() {
        let mut attachment = AttachmentInfo::new(
            "pasted_text_1".into(), "pasted_text.txt".into(), "text/plain".into(), 3,
        );
        attachment.content = "text".into();
        let tag = AIMessageManager::buildAttachmentTag(&attachment, None, false, false, false).unwrap();
        assert!(tag.contains("id=\"pasted_text_1\""));
        assert!(!tag.contains(" path="));
        assert!(!tag.contains("node_id="));
        assert!(tag.ends_with(">text</attachment>"));
    }

    #[test]
    fn external_host_paths_are_not_mislabeled_as_vfs_paths() {
        let mut attachment = AttachmentInfo::new(
            "/Users/source/report.pdf".into(),
            "report.pdf".into(),
            "application/pdf".into(),
            3,
        );
        attachment.nodeId = Some("core-b".into());
        let tag =
            AIMessageManager::buildAttachmentTag(&attachment, None, false, false, false).unwrap();
        assert!(tag.contains("node_id=\"core-b\""));
        assert!(!tag.contains(" path="));
    }

    #[test]
    fn legacy_attachment_markup_does_not_claim_a_current_node() {
        let attachment = AttachmentInfo::new(
            "/old/report.pdf".into(),
            "report.pdf".into(),
            "application/pdf".into(),
            3,
        );
        let tag =
            AIMessageManager::buildAttachmentTag(&attachment, None, false, false, false).unwrap();
        assert!(!tag.contains("node_id="));
    }

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

        let turns = AIMessageManager::getMemoryFromMessages(vec![message]);

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
        assert!(turns[2]
            .content
            .starts_with("<tool_result name=\"switch_core\""));
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

#[cfg(test)]
mod cancellation_tests {
    use super::{ChatOperationRegistry, MessageCancellationToken};
    use std::sync::{Arc, Mutex};

    #[test]
    fn cancellation_keeps_its_handle_when_stream_cleanup_runs() {
        let registry = Arc::new(Mutex::new(ChatOperationRegistry::default()));
        let cancellationHandle = MessageCancellationToken::default();
        let operationId = registry
            .lock()
            .unwrap()
            .register("chat".into(), cancellationHandle.clone());

        // Cancellation takes the service before closing the stream. Reproduce
        // cleanup waking on a different thread while cancellation owns it.
        let (_, ownedHandle) = registry.lock().unwrap().take("chat").unwrap();
        let cleanupRegistry = registry.clone();
        std::thread::spawn(move || {
            assert!(cleanupRegistry
                .lock()
                .unwrap()
                .remove("chat", operationId)
                .is_none());
        })
        .join()
        .unwrap();

        ownedHandle.cancel();
        assert!(cancellationHandle.isCancelled());
        assert!(registry.lock().unwrap().take("chat").is_none());
    }

    #[test]
    fn old_stream_cleanup_does_not_remove_new_send() {
        let mut registry = ChatOperationRegistry::default();
        let oldId = registry.register("chat".into(), "old-service");
        let newId = registry.register("chat".into(), "new-service");

        assert_ne!(oldId, newId);
        assert!(registry.remove("chat", oldId).is_none());
        assert_eq!(registry.get_mut("chat", newId), Some(&mut "new-service"));
    }

    #[test]
    fn cancelled_send_cannot_register_a_late_response_stream() {
        let mut registry = ChatOperationRegistry::default();
        let oldId = registry.register("chat".into(), None::<&str>);
        registry.take("chat").unwrap();
        assert!(registry.get_mut("chat", oldId).is_none());

        let newId = registry.register("chat".into(), Some("new-stream"));
        assert!(registry.get_mut("chat", oldId).is_none());
        assert_eq!(registry.take("chat"), Some((newId, Some("new-stream"))));
    }

    #[test]
    fn cancel_all_snapshot_cannot_cancel_a_later_send() {
        let mut registry = ChatOperationRegistry::default();
        let oldId = registry.register("chat".into(), "old-service");
        let snapshot = registry.snapshot();
        registry.remove("chat", oldId).unwrap();
        let newId = registry.register("chat".into(), "new-service");

        for (chatKey, operationId) in snapshot {
            assert!(registry.remove(&chatKey, operationId).is_none());
        }
        assert_eq!(registry.take("chat"), Some((newId, "new-service")));
    }

    #[test]
    fn finishing_one_chat_does_not_clear_another_chats_operation() {
        let mut registry = ChatOperationRegistry::default();
        let firstId = registry.register("first".into(), "first-service");
        let secondId = registry.register("second".into(), "second-service");

        assert_eq!(registry.remove("first", firstId), Some("first-service"));
        assert_eq!(registry.take("second"), Some((secondId, "second-service")));
        assert!(registry.snapshot().is_empty());
    }
}
