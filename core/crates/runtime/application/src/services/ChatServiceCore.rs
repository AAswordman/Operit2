use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use crate::core::chat::AIMessageManager::AIMessageManager;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use crate::plugins::toolpkg::ToolPkgChatInputHookBridge::{
    ChatInputHookContext, ChatInputHookResult, ToolPkgChatInputHookBridge,
    CHAT_INPUT_EVENT_INPUT_CHANGED, CHAT_INPUT_EVENT_SUBMITTED, CHAT_INPUT_EVENT_SUBMIT_REQUESTED,
    CHAT_INPUT_SUBMIT_ACTION_ALLOW, CHAT_INPUT_SUBMIT_ACTION_BLOCK,
    CHAT_INPUT_SUBMIT_ACTION_CONSUME, CHAT_INPUT_SUBMIT_ACTION_REPLACE,
};
use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use crate::plugins::toolpkg::ToolPkgInputMenuToggleBridge::InputMenuToggleDefinitionSnapshot;
use crate::plugins::toolpkg::ToolPkgInputMenuToggleBridge::ToolPkgInputMenuToggleBridge;
use crate::plugins::toolpkg::ToolPkgXmlRenderBridge::ToolPkgXmlRenderBridge;
use crate::services::core::ChatHistoryDelegate::{ChatHistoryDelegate, ChatSelectionMode};
use crate::services::core::MessageCoordinationDelegate::MessageCoordinationDelegate;
use crate::services::core::MessageProcessingDelegate::{
    ChatExecutionState, ChatTurnSubmission, MessageProcessingDelegate, SendUserMessageProcessingRequest,
};
use crate::services::core::TokenStatisticsDelegate::TokenStatisticsDelegate;
use crate::services::ProviderRuntimeSupportService::ChatConfigurationApi;
use crate::services::RuntimeHostInteractionService::{
    chatToolPermissionRequestsFlow, respondChatToolPermission,
    RuntimeHostInteractionToolPermissionRequest,
};
use crate::ui::features::chat::webview::workspace::WorkspaceBackupManager::{
    WorkspaceBackupManager, WorkspaceFileChange,
};
use crate::ui::features::chat::webview::workspace::WorkspaceUtils;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use operit_host_api::FileSystemHost;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_link::{
    CoreEvent, CoreEventKind, CoreEventStream, CoreStream, CoreStreamSource, CoreValue,
};
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_model::ChatHistory::ChatHistory;
use operit_model::ChatHistoryListItem::ChatHistoryListItem;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatMessageLocatorPreview::ChatMessageLocatorPreview;
use operit_model::ChatTurnOptions::ChatTurnOptions;
use operit_plugin_sdk::js_sdk::results::{JsOptional, MessageSendOutcome, MessageSendResultData};
use operit_model::FunctionType::FunctionType;
use operit_model::InputProcessingState::InputProcessingState;
use operit_model::MessagePart::MessagePart;
use operit_model::MessagePartCodec::MessagePartCodec;
use operit_model::PendingQueueMessageItem::PendingQueueMessageItem;
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_providers::runtime_support::{
    ChatConfigurationDisplayResult, ChatConfigurationPurpose, ChatConfigurationRequest,
    ChatConfigurationResult, ProviderRuntimeSupport,
};
use operit_store::repository::ChatHistoryManager::ChatImportResult;
use operit_store::repository::UsageStatisticsStore::UsageStatisticsStore;
use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
use operit_store::PreferencesDataStore::{
    combine2, combine4, combine5, mutableStateFlow, MutableStateFlow, StateFlow,
};
use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use operit_tools::files::PathMapper::PathMapper;
use operit_tools::files::VisualFileSystem::VisualFileSystem;
use operit_tools::runtime_support::CoreRouteResumeContext;
use operit_tools::tools::skill_runtime::SkillRepository::SkillRepository;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::tools::ToolPermissionSystem::{AiPermissionMode, ToolPermissionSystem};
use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{AITool, ToolParameter};
use operit_util::AppLogger::AppLogger;
use operit_util::MarkdownRenderStream::{MarkdownRenderEventStream, MarkdownStreamEvent};
use operit_util::OCRUtils::{OCRUtils, Quality as OCRQuality};
use operit_util::OperitPaths;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use url::Url;

const PACKAGE_ATTACHMENT_PREFIX: &str = "package_attach:";
const PASTED_TEXT_ATTACHMENT_PREFIX: &str = "pasted_text:";
const PASTED_IMAGE_ATTACHMENT_PREFIX: &str = "pasted_image:";
const WORKSPACE_MENTION_ATTACHMENT_PREFIX: &str = "workspace_mention:";
const OCR_INLINE_INSTRUCTION: &str = "Do not read the file, answer the user's question directly based on the attachment content and the user's question.";
pub trait ChatServiceUiBridge {}

pub struct EmptyChatServiceUiBridge;

impl ChatServiceUiBridge for EmptyChatServiceUiBridge {}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PastedImageAttachmentPayload {
    file_name: String,
    mime_type: String,
    file_size: i64,
    base64_content: String,
}

/// Serializes a ToolPkg chat input result into the proxy-facing JSON shape.
#[allow(non_snake_case)]
fn serializeChatInputHookResult(result: Option<ChatInputHookResult>) -> serde_json::Value {
    match result {
        Some(result) => serde_json::json!({
            "action": result.action,
            "text": result.text,
            "message": result.message,
            "clearInput": result.clearInput,
            "timedOut": result.timedOut,
            "metadata": result.metadata,
        }),
        None => serde_json::Value::Null,
    }
}

/// Settings belong to the execution Core selected by the chat binding, not the UI device.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatInputMenuSettings {
    pub permissionMode: AiPermissionMode,
    pub disableStreamOutput: bool,
    pub disableUserPreferenceDescription: bool,
    pub pluginChangeVersion: i64,
    pub pluginToggles: Vec<InputMenuToggleDefinitionSnapshot>,
}

/// Context and accounting for an explicit chat on its execution Core.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatInputMenuSummary {
    pub currentWindowSize: i64,
    pub inputTokenCount: i64,
    pub outputTokenCount: i64,
    pub maxContextLength: f64,
}

/// Bounded read-only plugin projection for the chat's execution Core.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatPluginItem {
    pub id: String,
    pub name: String,
    pub available: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatPluginTestResult {
    pub success: bool,
    pub message: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatEdgeEventAck {
    pub success: bool,
    pub next: u64,
    pub message: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatPluginPage {
    pub items: Vec<ChatPluginItem>,
    pub total: u32,
}

/// One selected package, never scripts/parameters or an unbounded tool catalog.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChatPluginDetails {
    pub id: String,
    pub description: String,
    pub tools: Vec<String>,
    pub toolOffset: u32,
    pub toolTotal: u32,
    pub error: String,
}
fn pluginDisplayText(text: &str, bytes: usize) -> String {
    let clean: String = text.chars().filter(|c| !c.is_control()).collect();
    let mut end = clean.len().min(bytes);
    while !clean.is_char_boundary(end) { end -= 1; }
    clean[..end].to_string()
}
fn pluginExclusive(extensions: &std::collections::BTreeMap<String, serde_json::Value>) -> bool {
    extensions.get("edge").and_then(|v| v.get("exclusive")).and_then(|v| v.as_bool()) == Some(true)
}

/// Describes the runtime state of one explicitly routed chat.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatState {
    pub currentChatId: String,
    pub currentChatTitle: String,
    pub currentCharacterCardName: Option<String>,
    pub currentCharacterCardAvatarUri: Option<String>,
    pub currentWorkspacePath: Option<String>,
    pub isLoading: bool,
    pub inputProcessingState: InputProcessingState,
    pub hasOlderDisplayHistory: bool,
    pub hasNewerDisplayHistory: bool,
    pub isLoadingDisplayWindow: bool,
    pub pendingQueueMessages: Vec<PendingQueueMessageItem>,
    pub isPendingQueueExpanded: bool,
    pub toolPermissionRequests: Vec<RuntimeHostInteractionToolPermissionRequest>,
}

/// Stores the runtime-owned pending message queue for one chat.
#[derive(Clone, Debug, PartialEq)]
struct PendingChatQueueState {
    messages: Vec<PendingQueueMessageItem>,
    isExpanded: bool,
    nextMessageId: i64,
    wasBlocked: bool,
    suppressNextAutoDequeue: bool,
}

impl PendingChatQueueState {
    /// Creates the initial queue state for a chat.
    fn new() -> Self {
        Self {
            messages: Vec::new(),
            isExpanded: true,
            nextMessageId: 1,
            wasBlocked: false,
            suppressNextAutoDequeue: false,
        }
    }
}

/// Shares pending-message queues between every chat surface in one runtime.
#[derive(Clone)]
pub(crate) struct PendingChatQueueStore {
    stateFlow: MutableStateFlow<HashMap<String, PendingChatQueueState>>,
}

impl PendingChatQueueStore {
    /// Creates an empty queue store shared by chat runtime slots.
    pub(crate) fn new() -> Self {
        Self {
            stateFlow: MutableStateFlow::new(HashMap::new()),
        }
    }

    /// Returns the queue-state flow shared by all chat runtime slots.
    fn stateFlow(&self) -> MutableStateFlow<HashMap<String, PendingChatQueueState>> {
        self.stateFlow.clone()
    }
}

pub struct ChatServiceCore {
    fileSystemHost: Arc<dyn FileSystemHost>,
    pub selectionMode: ChatSelectionMode,
    /// Carries the factory-authored slot supported by the current plugin send contract.
    pub(crate) chatInputRuntime: Option<operit_plugin_sdk::js_sdk::chat::ChatRuntime>,
    pub enhancedAiService: Option<EnhancedAIService>,
    pub messageProcessingDelegate: MessageProcessingDelegate,
    pub chatHistoryDelegate: ChatHistoryDelegate,
    pub messageCoordinationDelegate: Option<MessageCoordinationDelegate>,
    pub initialized: bool,
    chatConfigurationInitialization: Option<Result<(), String>>,
    pub onEnhancedAiServiceReady: Option<fn(&EnhancedAIService)>,
    pub additionalOnTurnComplete: Option<fn(Option<String>, i32, i32, i32)>,
    pub uiBridge: EmptyChatServiceUiBridge,
    pub attachments: Vec<AttachmentInfo>,
    pendingQueueStore: Arc<PendingChatQueueStore>,
    chatEnhancedAiServicesByChatId: Arc<Mutex<HashMap<String, EnhancedAIService>>>,
}

impl ChatServiceCore {
    /// Creates a chat service core for the selected chat target mode.
    pub fn new(selectionMode: ChatSelectionMode, fileSystemHost: Arc<dyn FileSystemHost>) -> Self {
        let chatInputRuntime = Some(match selectionMode {
            ChatSelectionMode::FOLLOW_GLOBAL => operit_plugin_sdk::js_sdk::chat::ChatRuntime::Main,
            ChatSelectionMode::LOCAL_ONLY => operit_plugin_sdk::js_sdk::chat::ChatRuntime::Floating,
        });
        Self::newWithPendingQueueStore(
            selectionMode,
            chatInputRuntime,
            fileSystemHost,
            Arc::new(PendingChatQueueStore::new()),
        )
    }

    /// Creates a chat service core backed by a queue store shared with sibling runtime slots.
    pub(crate) fn newWithPendingQueueStore(
        selectionMode: ChatSelectionMode,
        chatInputRuntime: Option<operit_plugin_sdk::js_sdk::chat::ChatRuntime>,
        fileSystemHost: Arc<dyn FileSystemHost>,
        pendingQueueStore: Arc<PendingChatQueueStore>,
    ) -> Self {
        let mut core = Self {
            fileSystemHost,
            selectionMode: selectionMode.clone(),
            chatInputRuntime,
            enhancedAiService: None,
            messageProcessingDelegate: MessageProcessingDelegate::default(),
            chatHistoryDelegate: ChatHistoryDelegate::new(selectionMode),
            messageCoordinationDelegate: None,
            initialized: false,
            chatConfigurationInitialization: None,
            onEnhancedAiServiceReady: None,
            additionalOnTurnComplete: None,
            uiBridge: EmptyChatServiceUiBridge,
            attachments: Vec::new(),
            pendingQueueStore,
            chatEnhancedAiServicesByChatId: Arc::new(Mutex::new(HashMap::new())),
        };
        core.initializeDelegates();
        core
    }

    /// Returns the tool handler bound to this chat runtime core.
    fn runtimeToolHandler(&self) -> AIToolHandler {
        self.enhancedAiService
            .as_ref()
            .expect("ChatServiceCore requires an enhanced AI service for runtime tool access")
            .tool_handler
            .clone()
    }

    /// Returns the persistent AI service instance owned by one chat id.
    fn newEnhancedAiServiceForChat(&self, chatId: &str) -> Option<EnhancedAIService> {
        let baseService = self.enhancedAiService.as_ref()?;
        let mut services = self
            .chatEnhancedAiServicesByChatId
            .lock()
            .expect("chat enhanced AI service mutex poisoned");
        Some(
            services
                .entry(chatId.to_string())
                .or_insert_with(|| {
                    EnhancedAIService::new(
                        baseService.tool_handler.clone(),
                        baseService.provider_runtime_context.clone(),
                    )
                })
                .clone(),
        )
    }

    /// Returns the shared pending-message queue state for this chat runtime.
    fn pendingQueueStateFlow(&self) -> MutableStateFlow<HashMap<String, PendingChatQueueState>> {
        self.pendingQueueStore.stateFlow()
    }

    /// Reports whether the specified chat is currently unable to accept a new turn.
    fn isChatQueueBlocked(&self, chatId: &str) -> bool {
        let activeStreamingChatIds = self
            .messageProcessingDelegate
            .activeStreamingChatIdsFlow()
            .value();
        if activeStreamingChatIds.contains(chatId) {
            return true;
        }
        let inputProcessingStateByChatId = self
            .messageProcessingDelegate
            .inputProcessingStateByChatIdFlow()
            .value();
        match inputProcessingStateByChatId.get(chatId) {
            Some(InputProcessingState::Idle)
            | Some(InputProcessingState::Completed)
            | Some(InputProcessingState::Error { .. })
            | None => false,
            Some(_) => true,
        }
    }

    /// Marks an existing queue as blocked when a new turn starts for its chat.
    fn markPendingQueueBlocked(&mut self, chatId: &str) {
        let mut queueStateByChatId = self.pendingQueueStateFlow().value();
        let Some(queueState) = queueStateByChatId.get_mut(chatId) else {
            return;
        };
        if queueState.messages.is_empty() || queueState.wasBlocked {
            return;
        }
        queueState.wasBlocked = true;
        self.pendingQueueStateFlow().set_value(queueStateByChatId);
    }

    /// Creates delegates without opening a chat before plugin runtime dependencies are ready.
    fn initializeDelegates(&mut self) {
        self.chatHistoryDelegate = ChatHistoryDelegate::new(self.selectionMode.clone());
        self.messageProcessingDelegate = MessageProcessingDelegate::default();
        self.messageProcessingDelegate.fileSystemHost = Some(self.fileSystemHost.clone());
        let messageProcessingDelegate = self.messageProcessingDelegate.clone_for_core();
        self.messageCoordinationDelegate = Some(MessageCoordinationDelegate::new(
            self.chatHistoryDelegate.clone_for_core(),
            messageProcessingDelegate,
        ));
        self.syncTokenStatisticsForCurrentChat();
    }

    /// Restores persisted history once without resolving execution configuration and retains actual storage errors.
    pub async fn initializeChatConfiguration(&mut self) -> Result<(), String> {
        if let Some(result) = &self.chatConfigurationInitialization {
            return result.clone();
        }
        let result = self.chatHistoryDelegate.initialize();
        self.chatConfigurationInitialization = Some(result.clone());
        if result.is_ok() {
            self.initialized = true;
            self.syncTokenStatisticsForCurrentChat();
        }
        result
    }

    #[allow(non_snake_case)]
    fn syncTokenStatisticsForCurrentChat(&mut self) {
        let chatId = self.chatHistoryDelegate.currentChatIdFlow.value();
        if let Some(delegate) = self.messageCoordinationDelegate.as_mut() {
            delegate
                .tokenStatisticsDelegate
                .setActiveChatId(chatId.clone());
            if let Some(chatId) = chatId {
                if let Some(chat) = self
                    .chatHistoryDelegate
                    .chatHistoriesFlow()
                    .value()
                    .into_iter()
                    .find(|chat| chat.id == chatId)
                {
                    delegate.tokenStatisticsDelegate.setTokenCounts(
                        Some(chat.id),
                        chat.inputTokens,
                        chat.outputTokens,
                        chat.currentWindowSize,
                    );
                }
            }
        }
    }

    /// Resolves the explicit or actually selected input conversation without manufacturing an empty target id.
    fn inputChatId(&self, chatIdOverride: Option<String>) -> Result<String, String> {
        let id = match chatIdOverride {
            Some(id) => id,
            None => self.chatHistoryDelegate.currentChatIdFlow.value()
                .ok_or_else(|| "Chat input has no selected conversation".to_string())?,
        };
        self.chatHistoryDelegate.requireChatExists(&id)?;
        Ok(id)
    }

    /// Builds the precise host input event, including real attachment records when they are actually available.
    #[allow(non_snake_case)]
    fn buildChatInputHookContext(
        &self, chatId: &str, text: &str, selectionStart: i32, selectionEnd: i32,
        attachmentCount: usize, attachments: Option<Vec<AttachmentInfo>>,
        replyToMessageTimestamp: Option<i64>, eventName: &str,
    ) -> ChatInputHookContext {
        ChatInputHookContext {
            chatId: chatId.to_string(), runtime: self.chatInputRuntime.clone(), notifyReply: true,
            text: text.to_string(), selectionStart, selectionEnd,
            hasAttachments: attachmentCount > 0, attachmentCount: attachmentCount as i32,
            attachments, replyToMessageTimestamp,
            isProcessing: self.messageProcessingDelegate.isChatLoading(chatId.to_string()),
            inputStyle: "Runtime".to_string(), source: "Runtime".to_string(),
            submitSource: "Send".to_string(), eventName: eventName.to_string(),
        }
    }

    /// Builds an authoritative submit event at the caret with complete original attachments and reply identity.
    #[allow(non_snake_case)]
    fn buildChatInputHookContextAtEnd(
        &self, chatId: &str, text: &str, attachments: &[AttachmentInfo],
        replyToMessageTimestamp: Option<i64>, eventName: &str,
    ) -> ChatInputHookContext {
        let textCharCount = text.chars().count() as i32;
        self.buildChatInputHookContext(chatId, text, textCharCount, textCharCount,
            attachments.len(), Some(attachments.to_vec()), replyToMessageTimestamp, eventName)
    }

    /// Dispatches count-only input notifications honestly and exposes notification failures to the host log.
    #[allow(non_snake_case)]
    pub async fn dispatchChatInputChanged(
        &self, chatIdOverride: Option<String>, messageText: String, selectionStart: i32,
        selectionEnd: i32, attachmentCount: usize,
    ) {
        let hookChatId = match self.inputChatId(chatIdOverride) {
            Ok(id) => id,
            Err(error) => { AppLogger::e("ChatInputHook", &error); return; }
        };
        if let Err(error) = ToolPkgChatInputHookBridge::dispatchRegisteredChatInputHooks(
            self.buildChatInputHookContext(&hookChatId, &messageText, selectionStart, selectionEnd,
                attachmentCount, None, None, CHAT_INPUT_EVENT_INPUT_CHANGED),
        ).await {
            AppLogger::e("ChatInputHook", &error);
        }
    }

    /// Captures an immutable authoritative submit payload so native consumers can release Holder before calling plugins.
    pub(crate) fn prepareChatInputSubmit(
        &self, chatId: String, text: String, attachments: Vec<AttachmentInfo>, replyToMessageTimestamp: Option<i64>,
    ) -> Result<ChatInputHookContext, String> {
        self.chatHistoryDelegate.requireChatExists(&chatId)?;
        if let Some(timestamp) = replyToMessageTimestamp {
            self.chatHistoryDelegate.chatHistoryManager.loadChatMessageVariant(&chatId, timestamp, 0)
                .map_err(|error| error.to_string())?;
        }
        Ok(self.buildChatInputHookContextAtEnd(&chatId, &text, &attachments, replyToMessageTimestamp, CHAT_INPUT_EVENT_SUBMIT_REQUESTED))
    }

    /// Resolves the real reply target and dispatches submit hooks strictly before any host sends or clears input.
    #[allow(non_snake_case)]
    pub async fn dispatchChatInputSubmitRequested(
        &self, chatIdOverride: Option<String>, messageText: String, selectionStart: i32,
        selectionEnd: i32, attachments: Vec<AttachmentInfo>, replyToMessageTimestamp: Option<i64>,
    ) -> Result<serde_json::Value, String> {
        let hookChatId = self.inputChatId(chatIdOverride)?;
        if let Some(timestamp) = replyToMessageTimestamp {
            self.chatHistoryDelegate.chatHistoryManager.loadChatMessageVariant(&hookChatId, timestamp, 0)
                .map_err(|error| error.to_string())?;
        }
        let decision = ToolPkgChatInputHookBridge::dispatchRegisteredChatInputHooks(
            self.buildChatInputHookContext(&hookChatId, &messageText, selectionStart, selectionEnd,
                attachments.len(), Some(attachments), replyToMessageTimestamp, CHAT_INPUT_EVENT_SUBMIT_REQUESTED),
        ).await?;
        Ok(serializeChatInputHookResult(decision))
    }

    /// Accepts a host-authored message through the same native turn pipeline and exposes immediate errors explicitly.
    #[operit_route_macros::operit_core_route(binding = chatIdOverride, permission = "target:runtime.execute")]
    pub async fn sendUserMessage(
        &mut self,
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
    ) -> Result<(), String> {
        self.startUserMessage(promptFunctionType, participantId, chatIdOverride, messageText,
            proxySenderNameOverride, chatProviderIdOverride, chatModelIdOverride, attachments, replyToMessage, turnOptions, None).await?;
        Ok(())
    }

    /// Starts the actual originating generation or returns an explicit hook ownership decision for native receipt observers.
    pub(crate) async fn startUserMessage(
        &mut self,
        promptFunctionType: PromptFunctionType,
        participantId: Option<String>,
        chatIdOverride: Option<String>,
        mut messageText: String,
        proxySenderNameOverride: Option<String>,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
        attachments: Vec<AttachmentInfo>,
        replyToMessage: Option<ChatMessage>,
        turnOptions: ChatTurnOptions,
        admittedLease: Option<operit_store::ChatExecutionLease::ChatExecutionLease>,
    ) -> Result<ChatTurnSubmission, String> {
        if admittedLease.is_some() != turnOptions.deferSequenceCompletion
            || turnOptions.deferSequenceCompletion != turnOptions.nativeExecutionId.is_some() {
            return Err("Native sequence execution requires its admitted lease and exact execution identity".to_string());
        }
        if chatIdOverride.is_none() { self.initializeChatConfiguration().await?; }
        let hookChatId = self.inputChatId(chatIdOverride.clone())?;
        if let Some(reply) = &replyToMessage {
            self.chatHistoryDelegate.chatHistoryManager.loadChatMessageVariant(&hookChatId, reply.timestamp, reply.selectedVariantIndex)
                .map_err(|error| error.to_string())?;
        }
        AppLogger::i(
            "ChatServiceCore",
            &format!(
                "send accepted chatId={} persisted={} attachments={}",
                hookChatId,
                turnOptions.persistTurn,
                attachments.len()
            ),
        );
        if turnOptions.continuation.is_none() && !turnOptions.chatInputSubmitRequestedHandled {
            let mut context = self.buildChatInputHookContextAtEnd(
                &hookChatId, &messageText, &attachments,
                replyToMessage.as_ref().map(|message| message.timestamp), CHAT_INPUT_EVENT_SUBMIT_REQUESTED,
            );
            context.notifyReply = turnOptions.notifyReply != Some(false);
            let submitDecision = ToolPkgChatInputHookBridge::dispatchRegisteredChatInputHooks(context)
            .await;
            let submitDecision = submitDecision?;
            if let Some(decision) = submitDecision {
                match decision.action.as_str() {
                    CHAT_INPUT_SUBMIT_ACTION_BLOCK | CHAT_INPUT_SUBMIT_ACTION_CONSUME => {
                        if let Some(message) = &decision.message {
                            self.messageProcessingDelegate.showToast(message.clone());
                        }
                        let outcome = if decision.action == CHAT_INPUT_SUBMIT_ACTION_CONSUME {
                            MessageSendOutcome::Consumed { metadata: decision.metadata.into_iter().collect() }
                        } else {
                            MessageSendOutcome::Blocked { message: decision.message }
                        };
                        return Ok(ChatTurnSubmission::Handled(MessageSendResultData {
                            chatId: hookChatId, message: messageText, aiResponse: JsOptional::Null,
                            receivedAt: JsOptional::Null, sentAt: currentTimeMillis(), outcome,
                        }));
                    }
                    CHAT_INPUT_SUBMIT_ACTION_REPLACE | CHAT_INPUT_SUBMIT_ACTION_ALLOW => {
                        if let Some(message) = &decision.message {
                            self.messageProcessingDelegate.showToast(message.clone());
                        }
                        if let Some(updatedText) = decision.text {
                            messageText = updatedText;
                        }
                    }
                    _ => return Err("Chat input returned an unvalidated submit action".to_string()),
                };
            }
        }
        if turnOptions.continuation.is_none() && !turnOptions.chatInputSubmitRequestedHandled {
            let mut context = self.buildChatInputHookContextAtEnd(&hookChatId, &messageText, &attachments,
                replyToMessage.as_ref().map(|message| message.timestamp), CHAT_INPUT_EVENT_SUBMITTED);
            context.notifyReply = turnOptions.notifyReply != Some(false);
            ToolPkgChatInputHookBridge::dispatchRegisteredChatInputHooks(context).await?;
        }
        ToolPkgInputMenuToggleBridge::invalidateToggleDefinitions();
        if self.enhancedAiService.is_some() && self.messageCoordinationDelegate.is_some() {
            self.markPendingQueueBlocked(&hookChatId);
        }
        let mut service = self.newEnhancedAiServiceForChat(&hookChatId)
            .ok_or_else(|| "Chat AI runtime is not initialized".to_string())?;
        let delegate = self.messageCoordinationDelegate.as_mut()
            .ok_or_else(|| "Chat coordination runtime is not initialized".to_string())?;
        delegate.chatHistoryDelegate = self.chatHistoryDelegate.clone_for_core();
        delegate.messageProcessingDelegate = self.messageProcessingDelegate.clone_for_core();
        let result = delegate.sendUserMessage(&mut service, promptFunctionType, participantId,
            Some(hookChatId), messageText, proxySenderNameOverride, chatProviderIdOverride,
            chatModelIdOverride, attachments, replyToMessage, turnOptions, admittedLease).await;
        self.chatHistoryDelegate = delegate.chatHistoryDelegate.clone_for_core();
        self.messageProcessingDelegate = delegate.messageProcessingDelegate.clone_for_core();
        result
    }

    /// Commits complete original input exactly once without requesting any participant, configuration, or model.
    pub(crate) fn recordOnlySequenceInput(
        &mut self, chatId: &str, text: &str, attachments: &[AttachmentInfo], replyTimestamp: Option<i64>,
        lease: &operit_store::ChatExecutionLease::ChatExecutionLease,
    ) -> Result<MessageSendResultData, String> {
        self.chatHistoryDelegate.requireChatExists(chatId)?;
        let reply = replyTimestamp.map(|timestamp| self.chatHistoryDelegate.chatHistoryManager
            .loadChatMessageVariant(chatId, timestamp, 0).map_err(|error| error.to_string())).transpose()?;
        let content = AIMessageManager::buildRecordedUserMessageContent(text, attachments, reply.as_ref())?;
        let mut message = ChatMessage::new("user".to_string());
        message.roleName = "user".to_string();
        message.parts = vec![operit_model::MessagePart::MessagePart::markdown("part-0".to_string(), 0, content)];
        lease.protectRevision(message.timestamp, 0).map_err(|error| error.to_string())?;
        let committed = self.chatHistoryDelegate.commitUserMessage(chatId, message)?;
        Ok(MessageSendResultData {
            chatId: chatId.to_string(), message: text.to_string(), aiResponse: JsOptional::Null,
            receivedAt: JsOptional::Null, sentAt: currentTimeMillis(),
            outcome: MessageSendOutcome::Committed {
                status: operit_plugin_sdk::js_sdk::results::MessageSendStatus::Completed,
                userMessageTimestamp: Some(committed.timestamp), assistant: None,
            },
        })
    }

    /// Cancels only the native execution that still owns this exact chat runtime, never a later admitted turn.
    pub(crate) async fn cancelNativeChatExecution(&mut self, chatId: &str, executionId: &str) -> Result<bool, String> {
        if !self.messageProcessingDelegate.isNativeExecutionActive(chatId, executionId) { return Ok(false); }
        self.cancelMessage(chatId.to_string()).await?;
        Ok(true)
    }

    /// Validates every aggregate locator against canonical records before publishing one sequence-level completion.
    pub(crate) fn finishNativeChatSequence(
        &mut self, result: &operit_tools::runtime_support::ChatSequenceFinishResultData, notifyReply: bool, hasSubmittedInput: bool,
    ) -> Result<(), String> {
        self.chatHistoryDelegate.requireChatExists(&result.chatId)?;
        if let Some(timestamp) = result.userMessageTimestamp {
            let user = self.chatHistoryDelegate.chatHistoryManager.loadChatMessageVariant(&result.chatId, timestamp, 0)
                .map_err(|error| error.to_string())?;
            if user.sender != "user" { return Err("Sequence receipt does not locate a persisted user message".to_string()); }
        }
        let mut lastAssistant = None;
        for locator in &result.assistants {
            let assistant = self.chatHistoryDelegate.chatHistoryManager.loadChatMessageVariant(
                &result.chatId, locator.messageTimestamp, locator.variantIndex).map_err(|error| error.to_string())?;
            if assistant.sender != "ai" { return Err("Sequence receipt does not locate a persisted assistant revision".to_string()); }
            lastAssistant = Some(assistant);
        }
        self.messageProcessingDelegate.finalizeSequenceAndNotify(&result.chatId, &result.status,
            result.error.as_deref(), hasSubmittedInput && result.userMessageTimestamp.is_some(), lastAssistant.as_ref(), notifyReply)
    }

    /// Resumes an AI round on the CoreNode that already owns the chat Binding.
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub async fn resume(&mut self, chatId: String) -> Result<(), String> {
        self.chatHistoryDelegate.requireChatExists(&chatId)?;
        let Some(mut service) = self.newEnhancedAiServiceForChat(&chatId) else {
            return Err(format!(
                "resume EnhancedAIService is not initialized chatId={chatId}"
            ));
        };
        let configuration = service.resolveChatConfigurationForOptions(
            &operit_providers::chat::EnhancedAIService::SendMessageOptions {
                chatId: Some(chatId.clone()),
                ..operit_providers::chat::EnhancedAIService::SendMessageOptions::new()
            },
        ).await.map_err(|error| error.to_string())?;
        self.chatHistoryDelegate.openChatHistory(chatId.clone(), false)?;
        let Some(delegate) = self.messageCoordinationDelegate.as_mut() else {
            return Err(format!("resume MessageCoordinationDelegate is not initialized chatId={chatId}"));
        };
        delegate.chatHistoryDelegate = self.chatHistoryDelegate.clone_for_core();
        delegate.messageProcessingDelegate = self.messageProcessingDelegate.clone_for_core();
        let runtimeChatHistory = self
            .chatHistoryDelegate
            .getRuntimeChatHistory(chatId.clone());
        delegate
            .sendMessageInternal(
                &mut service,
                PromptFunctionType::CHAT,
                false,
                true,
                true,
                None,
                Some(chatId.clone()),
                String::new(),
                None,
                None,
                None,
                Vec::new(),
                None,
                true,
                Some(runtimeChatHistory),
                ChatTurnOptions::default(), None)
            .await?;
        self.chatHistoryDelegate = delegate.chatHistoryDelegate.clone_for_core();
        self.messageProcessingDelegate = delegate.messageProcessingDelegate.clone_for_core();
        Ok(())
    }

    /// Resumes a route continuation using the context transported with the route change.
    #[allow(non_snake_case)]
    async fn resumeFromRouteContext(
        &mut self,
        chatId: String,
        resumeContext: CoreRouteResumeContext,
    ) -> Result<(), String> {
        AppLogger::i(
            "ChatServiceCore",
            &format!(
                "route resume context chatId={} historyMessages={} participantId={} extensionOwner={}",
                chatId,
                resumeContext.runtimeChatHistory.len(),
                resumeContext.participantId,
                resumeContext.extensionOwner
            ),
        );
        let Some(mut service) = self.newEnhancedAiServiceForChat(&chatId) else {
            return Err(format!(
                "route resume EnhancedAIService is not initialized chatId={chatId}"
            ));
        };
        let Some(delegate) = self.messageCoordinationDelegate.as_mut() else {
            return Err(format!(
                "route resume MessageCoordinationDelegate is not initialized chatId={chatId}"
            ));
        };
        self.chatHistoryDelegate.openChatHistory(chatId.clone(), false)?;
        delegate.chatHistoryDelegate = self.chatHistoryDelegate.clone_for_core();
        delegate.messageProcessingDelegate = self.messageProcessingDelegate.clone_for_core();
        let executionLease = self
            .chatHistoryDelegate
            .chatHistoryManager
            .beginChatExecution(&chatId)
            .map_err(|error| error.to_string())?;
        let api = ChatConfigurationApi::ready(&service.tool_handler).await?;
        if api.extensionOwner() != resumeContext.extensionOwner {
            return Err(
                "Routed message snapshot owner differs from the registered configuration owner"
                    .to_string(),
            );
        }
        let support = service.provider_runtime_context.support();
        let configuration = api
            .resolve(ChatConfigurationRequest {
                purpose: ChatConfigurationPurpose::Execution,
                chatId: Some(chatId.clone()),
                chatExtension: None,
                messageExtension: Some(resumeContext.messageExtension.clone()),
                participantId: Some(resumeContext.participantId.clone()),
                promptFunctionType: resumeContext.promptFunctionType.clone(),
                defaultModelBinding: support
                    .modelBindingForFunction(support.dataDir()?, FunctionType::CHAT)?,
                defaultTtsConfigId: support.defaultTtsConfigId()?,
            })
            .await?;
        delegate
            .sendRoutedContinuation(
                &mut service,
                chatId.clone(),
                configuration,
                executionLease,
                resumeContext,
            )
            .await?;
        self.chatHistoryDelegate = delegate.chatHistoryDelegate.clone_for_core();
        self.messageProcessingDelegate = delegate.messageProcessingDelegate.clone_for_core();
        Ok(())
    }

    /// Marks the source chat as paused while route synchronization is in progress.
    #[operit_route_macros::before_change_route]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub async fn beforeChangeRoute(&mut self, chatId: String) -> Result<(), String> {
        AppLogger::i(
            "ChatServiceCore",
            &format!("route change paused chatId={chatId}"),
        );
        Ok(())
    }

    /// Resumes the target chat after the route change reaches the selected Core.
    #[operit_route_macros::after_change_route]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub async fn afterChangeRoute(
        &mut self,
        chatId: String,
        resumeContext: CoreRouteResumeContext,
    ) -> Result<(), String> {
        AppLogger::i(
            "ChatServiceCore",
            &format!("route change reached target chatId={chatId}"),
        );
        self.resumeFromRouteContext(chatId, resumeContext).await
    }

    /// Cancels message generation for a specific chat id.
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub async fn cancelMessage(&mut self, chatId: String) -> Result<(), String> {
        let turn = self.messageProcessingDelegate.readCurrentTurnCancellationSnapshot(chatId.clone());
        let partialMessage = self.messageProcessingDelegate.cancelMessage(chatId.clone()).await;
        if let (Some(turn), Some(partial)) = (turn, &partialMessage) {
            if turn.turnOptions.persistTurn {
            if let Err(error) = self.chatHistoryDelegate.commitAssistantMessageSegment(chatId.clone(), partial.clone(), None) {
                self.messageProcessingDelegate.failTurnReceipt(&chatId, error.clone());
                return Err(error);
            }
        }
            }
        self.messageProcessingDelegate.completeCancelledTurnReceipt(&chatId, partialMessage.as_ref());
        Ok(())
    }

    /// Adds one message to the queue owned by a specific chat.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub fn enqueuePendingQueueMessage(&mut self, chatId: String, messageText: String) {
        let mut queueStateByChatId = self.pendingQueueStateFlow().value();
        let queueState = queueStateByChatId
            .entry(chatId)
            .or_insert_with(PendingChatQueueState::new);
        let messageId = queueState.nextMessageId;
        queueState.nextMessageId += 1;
        queueState.messages.push(PendingQueueMessageItem {
            id: messageId,
            text: messageText,
        });
        queueState.isExpanded = true;
        queueState.wasBlocked = true;
        self.pendingQueueStateFlow().set_value(queueStateByChatId);
    }

    /// Deletes one queued message from a specific chat.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub fn deletePendingQueueMessage(&mut self, chatId: String, messageId: i64) {
        let mut queueStateByChatId = self.pendingQueueStateFlow().value();
        let Some(queueState) = queueStateByChatId.get_mut(&chatId) else {
            return;
        };
        queueState.messages.retain(|item| item.id != messageId);
        self.pendingQueueStateFlow().set_value(queueStateByChatId);
    }

    /// Removes one queued message for editing or explicit user delivery.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub fn takePendingQueueMessage(
        &mut self,
        chatId: String,
        messageId: i64,
        suppressNextAutoDequeue: bool,
    ) -> Option<PendingQueueMessageItem> {
        let shouldSuppressAutoDequeue = suppressNextAutoDequeue && self.isChatQueueBlocked(&chatId);
        let mut queueStateByChatId = self.pendingQueueStateFlow().value();
        let queueState = queueStateByChatId.get_mut(&chatId)?;
        let messageIndex = queueState
            .messages
            .iter()
            .position(|item| item.id == messageId)?;
        let message = queueState.messages.remove(messageIndex);
        if shouldSuppressAutoDequeue && !queueState.messages.is_empty() {
            queueState.suppressNextAutoDequeue = true;
        }
        self.pendingQueueStateFlow().set_value(queueStateByChatId);
        Some(message)
    }

    /// Clears a manual-send suppression after that message is not delivered.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub fn clearPendingQueueAutoDequeueSuppression(&mut self, chatId: String) {
        let mut queueStateByChatId = self.pendingQueueStateFlow().value();
        let Some(queueState) = queueStateByChatId.get_mut(&chatId) else {
            return;
        };
        if !queueState.suppressNextAutoDequeue {
            return;
        }
        queueState.suppressNextAutoDequeue = false;
        self.pendingQueueStateFlow().set_value(queueStateByChatId);
    }

    /// Atomically removes the next queued message after a chat becomes ready.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub fn takeNextPendingQueueMessageIfReady(
        &mut self,
        chatId: String,
    ) -> Option<PendingQueueMessageItem> {
        if self.isChatQueueBlocked(&chatId) {
            return None;
        }
        let mut queueStateByChatId = self.pendingQueueStateFlow().value();
        let queueState = queueStateByChatId.get_mut(&chatId)?;
        if !queueState.wasBlocked {
            return None;
        }
        queueState.wasBlocked = false;
        if queueState.suppressNextAutoDequeue {
            queueState.suppressNextAutoDequeue = false;
            self.pendingQueueStateFlow().set_value(queueStateByChatId);
            return None;
        }
        let Some(message) = queueState.messages.first().cloned() else {
            self.pendingQueueStateFlow().set_value(queueStateByChatId);
            return None;
        };
        queueState.messages.remove(0);
        self.pendingQueueStateFlow().set_value(queueStateByChatId);
        Some(message)
    }

    /// Inserts a rejected queued message back at the front of its chat queue.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub fn restorePendingQueueMessage(&mut self, chatId: String, message: PendingQueueMessageItem) {
        let mut queueStateByChatId = self.pendingQueueStateFlow().value();
        let queueState = queueStateByChatId
            .entry(chatId)
            .or_insert_with(PendingChatQueueState::new);
        if queueState.messages.iter().any(|item| item.id == message.id) {
            return;
        }
        queueState.nextMessageId = queueState.nextMessageId.max(message.id + 1);
        queueState.messages.insert(0, message);
        self.pendingQueueStateFlow().set_value(queueStateByChatId);
    }

    /// Updates whether a chat's pending-message queue is expanded in the UI.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub fn setPendingQueueExpanded(&mut self, chatId: String, isExpanded: bool) {
        let mut queueStateByChatId = self.pendingQueueStateFlow().value();
        let queueState = queueStateByChatId
            .entry(chatId)
            .or_insert_with(PendingChatQueueState::new);
        queueState.isExpanded = isExpanded;
        self.pendingQueueStateFlow().set_value(queueStateByChatId);
    }

    /// Splits markdown content into stable render events for the client.
    pub fn splitMarkdownContent(&self, content: String) -> Vec<MarkdownStreamEvent> {
        MarkdownRenderEventStream::fromContent(content)
    }

    /// Observes committed XML render hook changes so existing message nodes can re-render.
    #[allow(non_snake_case)]
    pub fn xmlRenderRegistryRevisionFlow(&self) -> StateFlow<i64> {
        ToolPkgXmlRenderBridge::revisionFlow()
    }

    /// Renders one XML block through registered ToolPkg XML render hooks.
    #[allow(non_snake_case)]
    pub async fn renderToolPkgXml(
        &self,
        tagName: String,
        xmlContent: String,
        chatId: Option<String>,
    ) -> serde_json::Value {
        ToolPkgXmlRenderBridge::renderRegisteredXml(tagName, xmlContent, chatId).await
    }

    /// Creates a new chat and makes it available through chat history state.
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.write", create_binding = "runtime.execute")]
    pub async fn ensureRoutedChat(&mut self, chatId: String) -> Result<(), String> {
        self.chatHistoryDelegate
            .chatHistoryManager
            .ensureRoutedChat(chatId)
            .map_err(|error| error.to_string())
    }

    /// Creates an unpublished draft from an explicit source and opaque plugin input.
    pub async fn createNewChat(
        &mut self,
        setAsCurrentChat: bool,
        sourceChatId: Option<String>,
        input: Option<serde_json::Value>,
    ) -> Result<String, String> {
        let service = self
            .enhancedAiService
            .as_ref()
            .ok_or_else(|| "Chat creation requires an initialized AI runtime".to_string())?
            .clone();
        let chatId = self
            .chatHistoryDelegate
            .createNewChat(&service, setAsCurrentChat, sourceChatId, input)
            .await?;
        if setAsCurrentChat {
            self.initialized = true;
        }
        self.syncTokenStatisticsForCurrentChat();
        Ok(chatId)
    }

    /// Opens canonical history without requiring or synthesizing a plugin execution configuration.
    pub async fn switchChat(&mut self, chatId: String) -> Result<(), String> {
        self.chatHistoryDelegate.openChatHistory(chatId, true)?;
        self.initialized = true;
        self.syncTokenStatisticsForCurrentChat();
        Ok(())
    }

    /// Opens canonical history without requiring or synthesizing a plugin execution configuration.
    pub async fn switchChatLocal(&mut self, chatId: String) -> Result<(), String> {
        self.chatHistoryDelegate.openChatHistory(chatId, false)?;
        self.initialized = true;
        self.syncTokenStatisticsForCurrentChat();
        Ok(())
    }

    /// Resolves and publishes only generic display identity, ordered participants and plugin-authored initialization data.
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.read")]
    pub async fn chatConfiguration(
        &self,
        chatId: String,
    ) -> Result<ChatConfigurationDisplayResult, String> {
        self.chatHistoryDelegate.requireChatExists(&chatId)?;
        let service = self
            .enhancedAiService
            .as_ref()
            .ok_or_else(|| "Chat display requires an initialized plugin runtime".to_string())?;
        let api=ChatConfigurationApi::ready(&service.tool_handler).await?;
        let support=service.provider_runtime_context.support();
        let configuration=api.resolveDisplay(ChatConfigurationRequest {
            purpose: ChatConfigurationPurpose::Display,
            chatId: Some(chatId.clone()),
            chatExtension: None,
            messageExtension: None,
            participantId: None,
            promptFunctionType: PromptFunctionType::CHAT,
            defaultModelBinding: support.modelBindingForFunction(support.dataDir()?,FunctionType::CHAT)?,
            defaultTtsConfigId: support.defaultTtsConfigId()?,
        }).await?;
        self.chatHistoryDelegate
            .publishChatConfiguration(&chatId, configuration.clone());
        Ok(configuration)
    }

    /// Resolves the exact persisted message revision snapshot for identity, model and voice playback.
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.read")]
    pub async fn chatConfigurationForMessage(
        &self,
        chatId: String,
        messageTimestamp: i64,
        variantIndex: i32,
    ) -> Result<ChatConfigurationResult, String> {
        self.chatHistoryDelegate.requireChatExists(&chatId)?;
        let service = self.enhancedAiService.as_ref().ok_or_else(|| {
            "Message configuration requires an initialized AI runtime".to_string()
        })?;
        let api = ChatConfigurationApi::ready(&service.tool_handler).await?;
        let target = operit_model::PluginExtensionTarget::PluginExtensionTarget::Message {
            chatId: chatId.clone(),
            messageTimestamp,
            variantIndex,
        };
        let extension = self.chatHistoryDelegate.chatHistoryManager.readPluginExtension(api.extensionOwner(), &target).map_err(|error| error.to_string())?.ok_or_else(|| format!("Message revision has no configuration owner snapshot: {chatId}:{messageTimestamp}:{variantIndex}"))?;
        let snapshot = extension
            .as_object()
            .ok_or_else(|| {
                "Persisted message configuration extension must be an object".to_string()
            })?
            .clone();
        let support = service.provider_runtime_context.support();
        api.resolve(ChatConfigurationRequest {
            purpose: ChatConfigurationPurpose::Execution,
            chatId: Some(chatId),
            chatExtension: None,
            messageExtension: Some(snapshot),
            participantId: None,
            promptFunctionType: PromptFunctionType::CHAT,
            defaultModelBinding: support
                .modelBindingForFunction(support.dataDir()?, FunctionType::CHAT)?,
            defaultTtsConfigId: support.defaultTtsConfigId()?,
        })
        .await
    }

    /// Resolves the exact execution participant without matching display names or changing the active binding.
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.read")]
    pub async fn chatConfigurationForParticipant(
        &self,
        chatId: String,
        participantId: String,
    ) -> Result<ChatConfigurationResult, String> {
        self.chatHistoryDelegate.requireChatExists(&chatId)?;
        if participantId.trim().is_empty() {
            return Err("Chat execution participant id is empty".to_string());
        }
        let service = self
            .enhancedAiService
            .as_ref()
            .ok_or_else(|| "Chat configuration requires an initialized AI runtime".to_string())?;
        let configuration = service
            .resolveChatConfigurationForOptions(
                &operit_providers::chat::EnhancedAIService::SendMessageOptions {
                    chatId: Some(chatId),
                    executionParticipantId: Some(participantId.clone()),
                    ..operit_providers::chat::EnhancedAIService::SendMessageOptions::new()
                },
            )
            .await
            .map_err(|error| error.to_string())?;
        if configuration.profile.id != participantId {
            return Err(
                "Resolved profile does not match the requested execution participant".to_string(),
            );
        }
        Ok(configuration)
    }

    /// Persists a real local history selection without requiring an execution configuration.
    pub fn syncCurrentChatIdToGlobal(&mut self) -> Result<(), String> {
        let chatId = self
            .chatHistoryDelegate
            .currentChatIdFlow
            .value()
            .ok_or_else(|| "Global chat synchronization requires a selected chat".to_string())?;
        self.chatHistoryDelegate.requireChatExists(&chatId)?;
        self.chatHistoryDelegate
            .chatHistoryManager
            .setCurrentChatId(chatId)
            .map_err(|error| error.to_string())
    }

    /// Deletes the real conversation and its row-owned namespaces without a plugin binding mirror.
    pub async fn deleteChatHistory(&mut self, chatId: String) -> Result<bool, String> {
        self.chatHistoryDelegate.requireChatExists(&chatId)?;
        let deleted = self.chatHistoryDelegate.deleteChatHistory(chatId);
        self.syncTokenStatisticsForCurrentChat();
        Ok(deleted)
    }

    /// Deletes one message from an explicit chat by message timestamp.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub async fn deleteMessage(&mut self, chatId: String, messageTimestamp: i64) {
        self.chatHistoryDelegate
            .deleteMessageInChatByTimestamp(chatId, messageTimestamp);
    }

    /// Deletes multiple messages from an explicit chat by message timestamps.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub async fn deleteMessages(&mut self, chatId: String, messageTimestamps: Vec<i64>) -> bool {
        self.chatHistoryDelegate
            .deleteMessagesInChatByTimestamps(chatId, messageTimestamps)
    }

    /// Replaces the content of one message and refreshes the stable context window.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub async fn updateMessage(
        &mut self,
        chatId: String,
        messageTimestamp: i64,
        editedContent: String,
    ) -> bool {
        let currentMessages = self
            .chatHistoryDelegate
            .chatMessagesSnapshotForChat(chatId.clone());
        let Some(message) = currentMessages
            .into_iter()
            .find(|message| message.timestamp == messageTimestamp)
        else {
            return false;
        };
        let editedParts = match message.sender.as_str() {
            "ai" => match MessagePartCodec::parseAssistantMarkup(&editedContent) {
                Ok(parts) => parts,
                Err(error) => {
                    AppLogger::e(
                        "ChatServiceCore",
                        &format!("cannot update assistant message: invalid markup: {error}"),
                    );
                    return false;
                }
            },
            "user" => vec![MessagePart::markdown(
                "part-0".to_string(),
                0,
                editedContent,
            )],
            sender => {
                AppLogger::e(
                    "ChatServiceCore",
                    &format!("cannot update message from unsupported sender: {sender}"),
                );
                return false;
            }
        };
        let editedMessage = ChatMessage {
            parts: editedParts,
            ..message
        };
        self.chatHistoryDelegate
            .addMessageToChat(editedMessage, Some(chatId.clone()));
        if let Some(mut service) = self.newEnhancedAiServiceForChat(&chatId) {
            if let Some(delegate) = self.messageCoordinationDelegate.as_mut() {
                delegate.chatHistoryDelegate = self.chatHistoryDelegate.clone_for_core();
                delegate
                    .refreshStableContextWindow(
                        &mut service,
                        Some(chatId.clone()),
                        None,
                        Some(PromptFunctionType::CHAT),
                        None,
                        None)
                    .await;
                self.chatHistoryDelegate = delegate.chatHistoryDelegate.clone_for_core();
            }
        }
        true
    }

    /// Deletes the selected message and every following message in an explicit chat.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub async fn deleteMessagesFrom(&mut self, chatId: String, messageTimestamp: i64) -> bool {
        self.chatHistoryDelegate
            .deleteMessagesFromTimestamp(chatId, messageTimestamp)
    }

    /// Deletes one alternate response variant from a message timestamp.
    #[allow(non_snake_case)]
    pub fn deleteMessageVariant(&mut self, timestamp: i64, variantIndex: i32) {
        self.chatHistoryDelegate
            .deleteMessageVariant(timestamp, variantIndex);
    }

    /// Selects the displayed response variant for one message timestamp.
    #[allow(non_snake_case)]
    pub fn selectMessageVariant(&mut self, timestamp: i64, selectedVariantIndex: i32) {
        self.chatHistoryDelegate
            .selectMessageVariant(timestamp, selectedVariantIndex);
    }

    /// Creates and resolves a branch with its source's persisted opaque binding before selecting it.
    pub async fn createBranch(
        &mut self,
        upToMessageTimestamp: Option<i64>,
    ) -> Result<String, String> {
        let service = self
            .enhancedAiService
            .as_ref()
            .cloned()
            .ok_or_else(|| "Branch creation requires an initialized AI runtime".to_string())?;
        let chatId = self
            .chatHistoryDelegate
            .createBranch(&service, upToMessageTimestamp)
            .await?;
        self.initialized = true;
        self.syncTokenStatisticsForCurrentChat();
        self.messageProcessingDelegate.scrollToBottom();
        Ok(chatId)
    }

    /// Generates and inserts a summary message around the selected user or AI message.
    #[allow(non_snake_case)]
    pub async fn insertSummary(&mut self, message: ChatMessage) -> bool {
        if message.sender != "user" && message.sender != "ai" {
            return false;
        }
        let Some(currentChatId) = self.chatHistoryDelegate.currentChatIdFlow.value() else {
            return false;
        };
        let Some(mut enhancedAiService) = self.newEnhancedAiServiceForChat(&currentChatId) else {
            return false;
        };
        self.messageProcessingDelegate
            .setInputProcessingStateForChat(
                currentChatId.clone(),
                InputProcessingState::Summarizing {
                    message: "chat_summarizing_generating".to_string(),
                },
            );
        let beforeTimestamp = if message.sender == "ai" {
            Some(message.timestamp)
        } else {
            None
        };
        let afterTimestamp = if message.sender == "user" {
            Some(message.timestamp)
        } else {
            None
        };
        let messagesToSummarize = self
            .chatHistoryDelegate
            .loadMessagesForSummaryInsertion(currentChatId.clone(), afterTimestamp, beforeTimestamp)
            .into_iter()
            .filter(|message| message.sender == "user" || message.sender == "ai")
            .collect::<Vec<_>>();
        if messagesToSummarize.is_empty() {
            self.messageProcessingDelegate
                .setInputProcessingStateForChat(currentChatId, InputProcessingState::Idle);
            return false;
        }
        let configuration = match enhancedAiService.resolveChatConfigurationForOptions(
            &operit_providers::chat::EnhancedAIService::SendMessageOptions {
                chatId: Some(currentChatId.clone()),
                ..operit_providers::chat::EnhancedAIService::SendMessageOptions::new()
            },
        ).await {
            Ok(configuration) => configuration,
            Err(error) => {
                self.messageProcessingDelegate
                    .setInputProcessingStateForChat(
                        currentChatId,
                        InputProcessingState::Error { message: error.to_string() },
                    );
                return false;
            }
        };
        let summaryMessage = match AIMessageManager::summarizeMemory(
            &mut enhancedAiService,
            messagesToSummarize,
            false)
        .await
        {
            Ok(Some(summaryMessage)) => summaryMessage,
            _ => {
                self.messageProcessingDelegate
                    .setInputProcessingStateForChat(currentChatId, InputProcessingState::Idle);
                return false;
            }
        };
        self.chatHistoryDelegate.addSummaryMessage(
            summaryMessage,
            beforeTimestamp,
            afterTimestamp,
            Some(currentChatId.clone()),
        );
        if let Some(delegate) = self.messageCoordinationDelegate.as_mut() {
            delegate.chatHistoryDelegate = self.chatHistoryDelegate.clone_for_core();
            delegate.messageProcessingDelegate = self.messageProcessingDelegate.clone_for_core();
            delegate
                .refreshStableContextWindow(
                    &mut enhancedAiService,
                    Some(currentChatId.clone()),
                    None,
                    None,
                    None,
                    None)
                .await;
            self.chatHistoryDelegate = delegate.chatHistoryDelegate.clone_for_core();
            self.messageProcessingDelegate = delegate.messageProcessingDelegate.clone_for_core();
        }
        self.messageProcessingDelegate
            .setInputProcessingStateForChat(currentChatId, InputProcessingState::Idle);
        true
    }

    /// Returns branch chats that were derived from the requested parent chat.
    pub fn getBranches(&self, parentChatId: String) -> Vec<operit_model::ChatHistory::ChatHistory> {
        self.chatHistoryDelegate.getBranches(parentChatId)
    }

    /// Updates whether a chat is locked against destructive changes.
    pub fn updateChatLocked(&mut self, chatId: String, locked: bool) {
        self.chatHistoryDelegate.updateChatLocked(chatId, locked);
    }

    /// Updates whether a chat is pinned in chat history ordering.
    /// Updates native folder membership independently of plugin role bindings.
    pub fn updateChatGroups(&mut self, chatIds: Vec<String>, groupName: Option<String>) -> Result<(), String> {
        self.chatHistoryDelegate.updateChatGroups(chatIds, groupName)
    }

    pub fn updateChatPinned(&mut self, chatId: String, pinned: bool) {
        self.chatHistoryDelegate.updateChatPinned(chatId, pinned);
    }

    /// Reorders only real chat identifiers; all plugin grouping stays outside Core.
    pub fn updateChatOrder(
        &mut self,
        reorderedHistories: Vec<ChatHistoryListItem>,
        movedItem: ChatHistoryListItem,
    ) -> Result<(), String> {
        self.chatHistoryDelegate
            .updateChatOrder(reorderedHistories, movedItem)
    }

    /// Removes every message from the currently selected chat.
    pub fn clearCurrentChat(&mut self) {
        self.chatHistoryDelegate.clearCurrentChat();
    }

    /// Serializes all chat histories into a JSON archive string.
    #[allow(non_snake_case)]
    pub fn exportChatHistoriesToJson(&self) -> Result<String, String> {
        self.chatHistoryDelegate
            .chatHistoryManager
            .exportChatHistoriesToJson()
            .map_err(|error| error.to_string())
    }

    /// Imports chat histories from a JSON archive string.
    #[allow(non_snake_case)]
    pub fn importChatHistoriesFromJson(
        &mut self,
        jsonString: String,
    ) -> Result<ChatImportResult, String> {
        let result = self
            .chatHistoryDelegate
            .chatHistoryManager
            .importChatHistoriesFromJson(jsonString)
            .map_err(|error| error.to_string())?;
        Ok(result)
    }

    /// Updates the stored title of a chat history.
    pub fn updateChatTitle(&mut self, chatId: String, title: String) {
        self.chatHistoryDelegate.updateChatTitle(chatId, title);
    }

    /// Mounts a selected folder and propagates path validation and persistence errors.
    #[allow(non_snake_case)]
    pub fn bindChatToWorkspace(&mut self, chatId: String, workspace: String) -> Result<(), String> {
        self.chatHistoryDelegate
            .bindChatToFolderPath(chatId, workspace)
            .map(|_| ())
    }

    /// Creates the default workspace directory for a chat and returns its path.
    #[allow(non_snake_case)]
    pub fn createAndGetDefaultWorkspace(
        &mut self,
        chatId: String,
        projectType: Option<String>,
    ) -> String {
        WorkspaceUtils::createAndGetDefaultWorkspace(chatId, projectType)
            .expect("WorkspaceUtils.createAndGetDefaultWorkspace must succeed")
    }

    /// Creates a named workspace for a chat and stores the workspace binding.
    #[allow(non_snake_case)]
    pub fn createAndBindWorkspace(
        &mut self,
        chatId: String,
        name: String,
    ) -> Result<String, String> {
        PathMapper::workspacePath(&name)?;
        let workspacePath =
            WorkspaceUtils::createAndGetDefaultWorkspace(name.clone(), Some("blank".to_string()))?;
        let folderName = operit_model::Workspace::Workspace::folderNameFromPath(&workspacePath)?;
        let workspace = self
            .chatHistoryDelegate
            .chatHistoryManager
            .createWorkspace(
                name.clone(),
                vec![operit_model::Workspace::WorkspaceFolder {
                    name: folderName,
                    path: workspacePath.clone(),
                }],
            )
            .map_err(|error| error.to_string())?;
        self.chatHistoryDelegate
            .chatHistoryManager
            .updateChatWorkspaceId(chatId.clone(), Some(workspace.id))
            .map_err(|error| error.to_string())?;
        self.chatHistoryDelegate.notifyChatWorkspaceChanged(&chatId);
        Ok(workspacePath)
    }

    /// Removes the workspace binding from a chat without deleting workspace files.
    #[allow(non_snake_case)]
    pub fn unbindChatFromWorkspace(&mut self, chatId: String) {
        self.chatHistoryDelegate.unbindChatFromWorkspace(chatId);
    }

    /// Renames the workspace binding and chat title together.
    #[allow(non_snake_case)]
    pub fn renameWorkspaceAndChat(
        &mut self,
        chatId: String,
        newWorkspace: String,
        newTitle: String,
    ) {
        let newWorkspace = PathMapper::normalizeWorkspaceBindingPath(&newWorkspace).expect(
            "ChatServiceCore.renameWorkspaceAndChat requires a workspace path that maps to VFS",
        );
        self.chatHistoryDelegate
            .renameWorkspaceAndChat(chatId, newWorkspace, newTitle);
    }

    /// Shows file changes that would be applied when rewinding before one message timestamp.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub async fn previewWorkspaceChangesForMessage(
        &mut self,
        chatId: String,
        messageTimestamp: i64,
    ) -> Vec<WorkspaceFileChange> {
        let Some((chatId, workspacePath, rewindTimestamp)) =
            self.resolveWorkspaceRewindTarget(chatId, messageTimestamp)
        else {
            return Vec::new();
        };
        WorkspaceBackupManager::getInstance(self.runtimeToolHandler().getContext())
            .previewChangesForRewind(workspacePath, rewindTimestamp, Some(chatId))
    }

    /// Restores the bound workspace to the snapshot before one message timestamp.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub async fn rewindWorkspaceForMessage(
        &mut self,
        chatId: String,
        messageTimestamp: i64,
    ) -> bool {
        self.rewindWorkspaceForMessageTimestamp(chatId, messageTimestamp)
    }

    /// Restores the bound workspace before one timestamp without crossing route again.
    #[allow(non_snake_case)]
    fn rewindWorkspaceForMessageTimestamp(
        &mut self,
        chatId: String,
        messageTimestamp: i64,
    ) -> bool {
        let Some((chatId, workspacePath, rewindTimestamp)) =
            self.resolveWorkspaceRewindTarget(chatId, messageTimestamp)
        else {
            return false;
        };
        WorkspaceBackupManager::getInstance(self.runtimeToolHandler().getContext()).syncState(
            workspacePath,
            rewindTimestamp,
            Some(chatId),
        );
        true
    }

    /// Rolls an explicit chat back to a prior message timestamp.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub async fn rollbackToMessage(
        &mut self,
        chatId: String,
        messageTimestamp: i64,
    ) -> Option<String> {
        let currentMessages = self
            .chatHistoryDelegate
            .chatMessagesSnapshotForChat(chatId.clone());
        let Some(targetMessage) = currentMessages
            .into_iter()
            .find(|message| message.timestamp == messageTimestamp)
        else {
            return None;
        };
        if targetMessage.sender != "user" {
            return None;
        }
        self.rewindWorkspaceForMessageTimestamp(chatId.clone(), messageTimestamp);
        self.chatHistoryDelegate
            .truncateChatHistoryForChat(chatId, Some(messageTimestamp));
        Some(stripXmlLikeTags(&targetMessage.displayText()))
    }

    /// Rewinds a user message and sends edited content as a new turn.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub async fn rewindAndResendMessage(
        &mut self,
        chatId: String,
        messageTimestamp: i64,
        editedContent: String,
    ) -> bool {
        let currentMessages = self
            .chatHistoryDelegate
            .chatMessagesSnapshotForChat(chatId.clone());
        let Some(targetMessage) = currentMessages
            .into_iter()
            .find(|message| message.timestamp == messageTimestamp)
        else {
            return false;
        };
        if targetMessage.sender != "user" {
            return false;
        }
        self.rewindWorkspaceForMessageTimestamp(chatId.clone(), messageTimestamp);
        self.chatHistoryDelegate
            .truncateChatHistoryForChat(chatId.clone(), Some(messageTimestamp));
        self.sendUserMessage(
            PromptFunctionType::CHAT,
            None,
            Some(chatId),
            editedContent,
            None,
            None,
            None,
            Vec::new(),
            None,
            ChatTurnOptions::default(),
        )
        .await;
        true
    }

    /// Regenerates one AI message in place while preserving the surrounding chat history.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub async fn regenerateSingleAiMessage(
        &mut self,
        chatId: String,
        messageTimestamp: i64,
    ) -> Result<(), String> {
        let Some(mut service) = self.newEnhancedAiServiceForChat(&chatId) else {
            return Err("EnhancedAIService is not initialized".to_string());
        };
        let Some(delegate) = self.messageCoordinationDelegate.as_mut() else {
            return Err("MessageCoordinationDelegate is not initialized".to_string());
        };
        delegate.chatHistoryDelegate = self.chatHistoryDelegate.clone_for_core();
        delegate.messageProcessingDelegate = self.messageProcessingDelegate.clone_for_core();
        delegate
            .regenerateSingleAiMessage(&mut service, chatId, messageTimestamp)
            .await?;
        self.chatHistoryDelegate = delegate.chatHistoryDelegate.clone_for_core();
        self.messageProcessingDelegate = delegate.messageProcessingDelegate.clone_for_core();
        self.syncTokenStatisticsForCurrentChat();
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Resolves the workspace rewind boundary for one explicit message timestamp.
    fn resolveWorkspaceRewindTarget(
        &self,
        chatId: String,
        messageTimestamp: i64,
    ) -> Option<(String, String, i64)> {
        let currentMessages = self
            .chatHistoryDelegate
            .chatMessagesSnapshotForChat(chatId.clone());
        let targetIndex = currentMessages
            .iter()
            .position(|message| message.timestamp == messageTimestamp)?;
        let rewindTimestamp = if targetIndex > 0 {
            currentMessages[targetIndex - 1].timestamp
        } else {
            0
        };
        let currentChat = self
            .chatHistoryDelegate
            .chatHistoriesFlow
            .value()
            .into_iter()
            .find(|history| history.id == chatId)?;
        let workspacePath = currentChat
            .workspacePrimaryPath
            .clone()
            .filter(|value| !value.trim().is_empty())?;
        Some((chatId, workspacePath, rewindTimestamp))
    }

    /// Clears the token counters associated with the current chat service.
    pub fn resetTokenStatistics(&mut self) {
        if let Err(error) = UsageStatisticsStore::new().clearAllTokenUsageRecords() {
            AppLogger::e(
                "TokenStatistics",
                &format!("failed to clear token ledger: {error}"),
            );
            return;
        }
        let service = self.enhancedAiService.as_mut();
        if let Some(delegate) = self.messageCoordinationDelegate.as_mut() {
            delegate
                .tokenStatisticsDelegate
                .resetTokenStatistics(service);
        }
    }

    /// Recomputes cumulative token statistics for the current chat and service.
    pub fn updateCumulativeStatistics(&mut self) {
        let chatId = self.chatHistoryDelegate.currentChatIdFlow.value();
        let service = self.enhancedAiService.as_ref();
        if let Some(delegate) = self.messageCoordinationDelegate.as_mut() {
            delegate
                .tokenStatisticsDelegate
                .updateCumulativeStatistics(chatId, service);
        }
    }

    /// Adds a file, pasted text, pasted image, package, screen capture, notification capture, or location capture as an attachment.
    pub async fn handleAttachment(&mut self, _filePath: String) {
        if let Some(content) = _filePath.strip_prefix(PASTED_TEXT_ATTACHMENT_PREFIX) {
            self.attachPastedText(content.to_string());
            return;
        }
        if let Some(payloadJson) = _filePath.strip_prefix(PASTED_IMAGE_ATTACHMENT_PREFIX) {
            self.attachPastedImage(payloadJson);
            return;
        }

        let filePath = _filePath.trim();
        if filePath.is_empty() {
            self.messageProcessingDelegate
                .showToast("无法添加空附件路径".to_string());
            return;
        }

        if filePath == "screen_capture" {
            self.captureScreenContent().await;
            return;
        }
        if filePath == "notifications_capture" {
            self.captureNotifications(10).await;
            return;
        }
        if filePath == "location_capture" {
            self.captureLocation(true).await;
            return;
        }
        if let Some(packageName) = filePath.strip_prefix(PACKAGE_ATTACHMENT_PREFIX) {
            self.attachPackageInternal(packageName.trim()).await;
            return;
        }
        if let Some(relativePath) = filePath.strip_prefix(WORKSPACE_MENTION_ATTACHMENT_PREFIX) {
            match self.attachWorkspaceMentionInternal(relativePath) {
                Ok(fileName) => {
                    self.messageProcessingDelegate
                        .showToast(format!("已添加工作区引用: {fileName}"));
                }
                Err(message) => {
                    self.messageProcessingDelegate.showToast(message);
                }
            }
            return;
        }

        match self.createAttachmentInfo(filePath) {
            Ok(attachmentInfo) => {
                let currentPath = attachmentInfo.filePath.clone();
                if !self
                    .attachments
                    .iter()
                    .any(|attachment| attachment.filePath == currentPath)
                {
                    let fileName = attachmentInfo.fileName.clone();
                    self.attachments.push(attachmentInfo);
                    self.messageProcessingDelegate
                        .showToast(format!("已添加附件: {fileName}"));
                }
            }
            Err(message) => {
                self.messageProcessingDelegate.showToast(message);
            }
        }
    }

    /// Adds the supplied pasted text as an in-memory plain-text attachment.
    #[allow(non_snake_case)]
    fn attachPastedText(&mut self, content: String) {
        let attachmentInfo = AttachmentInfo {
            nodeId: None,
            filePath: format!(
                "pasted_text_{}_{}",
                currentTimeMillis(),
                self.attachments.len()
            ),
            fileName: "pasted_text.txt".to_string(),
            mimeType: "text/plain".to_string(),
            fileSize: content.len() as i64,
            content,
        };
        self.attachments.push(attachmentInfo);
        self.messageProcessingDelegate
            .showToast("已添加粘贴文本附件".to_string());
    }

    /// Adds the supplied pasted image as a clean-on-exit file attachment.
    #[allow(non_snake_case)]
    fn attachPastedImage(&mut self, payloadJson: &str) {
        let payload: PastedImageAttachmentPayload = match serde_json::from_str(payloadJson) {
            Ok(value) => value,
            Err(error) => {
                self.messageProcessingDelegate
                    .showToast(format!("添加粘贴图片失败: {error}"));
                return;
            }
        };
        let fileName = payload.file_name.trim().replace('"', "'");
        if fileName.is_empty() {
            self.messageProcessingDelegate
                .showToast("添加粘贴图片失败: 文件名为空".to_string());
            return;
        }
        let mimeType = payload.mime_type.trim().to_ascii_lowercase();
        if !mimeType.starts_with("image/") {
            self.messageProcessingDelegate
                .showToast(format!("添加粘贴图片失败: 不支持的类型 {mimeType}"));
            return;
        }
        let decoded = match STANDARD.decode(payload.base64_content.trim().as_bytes()) {
            Ok(bytes) if !bytes.is_empty() => bytes,
            Ok(_) => {
                self.messageProcessingDelegate
                    .showToast("添加粘贴图片失败: 图片内容为空".to_string());
                return;
            }
            Err(error) => {
                self.messageProcessingDelegate
                    .showToast(format!("添加粘贴图片失败: {error}"));
                return;
            }
        };
        let fileSize = decoded.len() as i64;
        if payload.file_size > 0 && payload.file_size != fileSize {
            self.messageProcessingDelegate
                .showToast("添加粘贴图片失败: 图片大小不一致".to_string());
            return;
        }
        let tempFile = match createTempFileFromBytes(
            self.fileSystemHost.as_ref(),
            &fileName,
            &decoded,
            self.attachments.len(),
        ) {
            Ok(value) => value,
            Err(message) => {
                self.messageProcessingDelegate.showToast(message);
                return;
            }
        };
        let attachmentInfo = AttachmentInfo {
            nodeId: localAttachmentNodeId(),
            filePath: tempFile.to_string_lossy().into_owned(),
            fileName,
            mimeType,
            fileSize,
            content: String::new(),
        };
        self.attachments.push(attachmentInfo);
        self.messageProcessingDelegate
            .showToast("已添加粘贴图片附件".to_string());
    }

    /// Captures and recognizes screen text through the configured system host.
    #[allow(non_snake_case)]
    async fn captureScreenContent(&mut self) {
        let mut toolHandler = self.runtimeToolHandler();
        let result = toolHandler
            .executeTool(AITool {
                name: "capture_screenshot".to_string(),
                parameters: Vec::new(),
            })
            .await;
        if !result.success {
            self.messageProcessingDelegate
                .showToast(format!("添加屏幕内容失败: {}", toolFailureMessage(&result)));
            return;
        }

        let screenshotPath = result.result.toString().trim().to_string();
        if screenshotPath.is_empty() {
            self.messageProcessingDelegate
                .showToast("添加屏幕内容失败: 截图失败".to_string());
            return;
        }

        let screenshotBytes = match self.fileSystemHost.readFileBytes(&screenshotPath) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.messageProcessingDelegate
                    .showToast(format!("添加屏幕内容失败: {}", error.message));
                return;
            }
        };
        let positionInfo = match image::load_from_memory(&screenshotBytes) {
            Ok(image) if image.width() > 0 && image.height() > 0 => {
                let width = image.width();
                let height = image.height();
                format!("【位置】full_screen; image_px={}x{}", width, height)
            }
            _ => "【位置】full_screen".to_string(),
        };

        let ocrText =
            OCRUtils::recognizeText(&toolHandler.getContext(), &screenshotPath, OCRQuality::HIGH);
        let ocrText = ocrText.trim().to_string();
        if ocrText.is_empty() {
            self.messageProcessingDelegate
                .showToast("添加屏幕内容失败: 未识别到屏幕文字".to_string());
            return;
        }

        let captureId = format!("screen_ocr_{}", currentTimeMillis());
        let content = format!("屏幕内容{positionInfo}\n\n{ocrText}\n\n{OCR_INLINE_INSTRUCTION}");
        self.attachments.push(AttachmentInfo {
            nodeId: None,
            filePath: captureId,
            fileName: "screen_content.txt".to_string(),
            mimeType: "text/plain".to_string(),
            fileSize: content.len() as i64,
            content,
        });
        self.messageProcessingDelegate
            .showToast("已添加屏幕内容".to_string());

        if let Err(error) = self.fileSystemHost.deleteFile(&screenshotPath, false) {
            AppLogger::w(
                "ChatServiceCore",
                &format!("cannot remove captured screenshot: {}", error.message),
            );
        }
    }

    #[allow(non_snake_case)]
    async fn captureNotifications(&mut self, limit: i32) {
        let mut toolHandler = self.runtimeToolHandler();
        let result = toolHandler
            .executeTool(AITool {
                name: "get_notifications".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "limit".to_string(),
                        value: limit.to_string(),
                    },
                    ToolParameter {
                        name: "include_ongoing".to_string(),
                        value: "true".to_string(),
                    },
                ],
            })
            .await;
        if !result.success {
            self.messageProcessingDelegate
                .showToast(format!("添加当前通知失败: {}", toolFailureMessage(&result)));
            return;
        }

        let content = result.result.toString();
        let attachmentInfo = AttachmentInfo {
            nodeId: None,
            filePath: format!("notifications_{}", currentTimeMillis()),
            fileName: "notifications.json".to_string(),
            mimeType: "application/json".to_string(),
            fileSize: content.len() as i64,
            content,
        };
        self.attachments.push(attachmentInfo);
        self.messageProcessingDelegate
            .showToast("已添加当前通知".to_string());
    }

    /// Attaches current coordinates without requesting an implicit reverse-geocoding service.
    #[allow(non_snake_case)]
    async fn captureLocation(&mut self, highAccuracy: bool) {
        let mut toolHandler = self.runtimeToolHandler();
        let result = toolHandler
            .executeTool(AITool {
                name: "get_device_location".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "high_accuracy".to_string(),
                        value: highAccuracy.to_string(),
                    },
                    ToolParameter {
                        name: "timeout".to_string(),
                        value: "10".to_string(),
                    },
                    ToolParameter {
                        name: "include_address".to_string(),
                        value: "false".to_string(),
                    },
                ],
            })
            .await;
        if !result.success {
            self.messageProcessingDelegate
                .showToast(format!("添加当前位置失败: {}", toolFailureMessage(&result)));
            return;
        }

        let content = result.result.toString();
        let attachmentInfo = AttachmentInfo {
            nodeId: None,
            filePath: format!("location_{}", currentTimeMillis()),
            fileName: "location.json".to_string(),
            mimeType: "application/json".to_string(),
            fileSize: content.len() as i64,
            content,
        };
        self.attachments.push(attachmentInfo);
        self.messageProcessingDelegate
            .showToast("已添加当前位置".to_string());
    }

    #[allow(non_snake_case)]
    async fn attachPackageInternal(&mut self, packageName: &str) {
        if packageName.is_empty() {
            self.messageProcessingDelegate
                .showToast(format!("添加包失败: {packageName}"));
            return;
        }

        let toolHandler = self.runtimeToolHandler();
        let packageManager = toolHandler.getOrCreatePackageManager();
        if RuntimePackageManager::prepareMcpPackage(&packageManager, packageName).await.is_err() {
            self.messageProcessingDelegate.showToast(format!("添加包失败: {packageName}"));
            return;
        }
        let isStandardPackage;
        let isSkillPackage;
        let isMcpPackage;
        {
            let packageManagerGuard = packageManager
                .lock()
                .expect("package manager mutex poisoned");
            isStandardPackage = packageManagerGuard
                .getAvailablePackages()
                .contains_key(packageName)
                && !packageManagerGuard.isToolPkgContainer(packageName);
            isMcpPackage = packageManagerGuard
                .getAvailableServerPackages()
                .contains_key(packageName);
        }
        isSkillPackage =
            SkillRepository::getInstance(&toolHandler.getContext(), toolHandler.runtimeSupport())
                .getAiVisibleSkillPackages()
                .contains_key(packageName);

        if !isStandardPackage && !isSkillPackage && !isMcpPackage {
            self.messageProcessingDelegate
                .showToast(format!("添加包失败: {packageName}"));
            return;
        }

        {
            let mut packageManagerGuard = packageManager
                .lock()
                .expect("package manager mutex poisoned");
            if isStandardPackage {
                packageManagerGuard.enablePackage(packageName);
            }
            let packageContent = packageManagerGuard.usePackage(packageName);
            if isPackageAttachmentError(packageName, &packageContent) {
                self.messageProcessingDelegate
                    .showToast(format!("添加包失败: {packageName}"));
                return;
            }

            let attachmentInfo = AttachmentInfo {
                nodeId: None,
                filePath: packageAttachmentPath(packageName),
                fileName: packageAttachmentDisplayName(packageName),
                mimeType: "text/plain".to_string(),
                fileSize: packageContent.len() as i64,
                content: packageContent,
            };
            self.attachments
                .retain(|attachment| attachment.filePath != attachmentInfo.filePath);
            self.attachments.push(attachmentInfo);
        }

        self.messageProcessingDelegate
            .showToast(format!("已添加包: {packageName}"));
    }

    /// Adds a workspace mention as an in-memory plain-text attachment.
    #[allow(non_snake_case)]
    fn attachWorkspaceMentionInternal(&mut self, relativePath: &str) -> Result<String, String> {
        let normalizedRelativePath = PathMapper::normalizeRelativePath(relativePath)?;
        if normalizedRelativePath.is_empty() {
            return Err("无法添加工作区引用: 路径为空".to_string());
        }
        let workspaceRoot = self.currentWorkspaceRoot()?;
        let vfs = self.vfsForWorkspace()?;
        let targetPath = PathMapper::joinVfsPath(&workspaceRoot, &normalizedRelativePath)?;
        let targetInfo = vfs.fileExists(&targetPath)?;
        if !targetInfo.exists {
            return Err("工作区路径不存在".to_string());
        }

        let (mimeType, content) = if targetInfo.isDirectory {
            (
                "application/vnd.workspace-directory+plain".to_string(),
                buildWorkspaceDirectoryMentionContent(&vfs, &targetPath, &normalizedRelativePath)?,
            )
        } else {
            (
                "text/plain".to_string(),
                buildWorkspaceFileMentionContent(&vfs, &targetPath, &normalizedRelativePath)?,
            )
        };
        let attachmentInfo = AttachmentInfo {
            nodeId: None,
            filePath: workspaceMentionAttachmentPath(&normalizedRelativePath),
            fileName: normalizedRelativePath.clone(),
            mimeType,
            fileSize: content.len() as i64,
            content,
        };
        self.attachments
            .retain(|attachment| attachment.filePath != attachmentInfo.filePath);
        self.attachments.push(attachmentInfo);
        Ok(normalizedRelativePath)
    }

    /// Returns the workspace root bound to the currently selected chat.
    #[allow(non_snake_case)]
    fn currentWorkspaceRoot(&self) -> Result<String, String> {
        let chatId = self
            .chatHistoryDelegate
            .currentChatIdFlow
            .value()
            .ok_or_else(|| "无法添加工作区引用: 当前聊天不存在".to_string())?;
        self.chatHistoryDelegate
            .chatHistoriesFlow()
            .value()
            .into_iter()
            .find(|chat| chat.id == chatId)
            .and_then(|chat| chat.workspacePrimaryPath.clone())
            .filter(|workspace| !workspace.trim().is_empty())
            .ok_or_else(|| "当前聊天未绑定工作区".to_string())
    }

    /// Creates a VFS instance for chat workspace attachment reads.
    #[allow(non_snake_case)]
    fn vfsForWorkspace(&self) -> Result<VisualFileSystem, String> {
        let runtimeStorageHost = defaultRuntimeStorageHost();
        let runtimeStoreRoot = runtimeStorageHost.runtimeRootDir().ok_or_else(|| {
            "RuntimeStorageHost runtime root is not configured for workspace mentions".to_string()
        })?;
        let workspaceCollectionRoot = runtimeStorageHost.workspaceRootDir().ok_or_else(|| {
            "RuntimeStorageHost workspace root is not configured for workspace mentions".to_string()
        })?;
        Ok(VisualFileSystem::new(
            self.fileSystemHost.clone(),
            PathMapper::new(runtimeStoreRoot, workspaceCollectionRoot, runtimeStorageHost.clone()),
        ))
    }

    #[allow(non_snake_case)]
    fn createAttachmentInfo(&self, filePath: &str) -> Result<AttachmentInfo, String> {
        let localPath = resolveAttachmentPath(filePath)?;
        let localPathText = localPath.to_string_lossy();
        let source = self
            .fileSystemHost
            .fileExists(&localPathText)
            .map_err(|error| error.message)?;
        if !source.exists {
            return Err("附件文件不存在".to_string());
        }
        if source.isDirectory {
            return Err(format!("无法添加附件: {}", localPath.display()));
        }

        let fileName = localPath
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("无法添加附件: {}", localPath.display()))?
            .to_string();
        let mimeType = getMimeTypeFromPath(&localPath).to_string();
        let tempFile = createTempFileFromPath(self.fileSystemHost.as_ref(), &localPath, &fileName)?;
        let fileSize = self
            .fileSystemHost
            .fileExists(&tempFile.to_string_lossy())
            .map_err(|error| format!("无法读取附件大小: {}", error.message))?
            .size;

        Ok(AttachmentInfo {
            nodeId: localAttachmentNodeId(),
            filePath: tempFile.to_string_lossy().into_owned(),
            fileName,
            mimeType,
            fileSize,
            content: String::new(),
        })
    }

    /// Registers a committed Host upload without reading or copying the file again.
    pub fn attachUploadedFile(
        &mut self,
        attachment: AttachmentInfo,
        expectedChatId: Option<String>,
    ) -> Result<(), String> {
        if self.chatHistoryDelegate.currentChatIdFlow.value() != expectedChatId {
            return Err("Chat changed while uploading the attachment".to_string());
        }
        super::AttachmentTransferManager::validateAttachmentFileName(&attachment.fileName)?;
        let directory = OperitPaths::cleanOnExitDir()?;
        let path = Path::new(&attachment.filePath);
        if !path.starts_with(&directory)
            || path
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
            || attachment.nodeId != localAttachmentNodeId()
            || !attachment.content.is_empty()
        {
            return Err("Uploaded attachment does not belong to this runtime".to_string());
        }
        let stored = self
            .fileSystemHost
            .fileExists(&attachment.filePath)
            .map_err(|error| error.to_string())?;
        if !stored.exists || stored.isDirectory || stored.size != attachment.fileSize {
            return Err(
                "Uploaded attachment is missing or has an unexpected byte length".to_string(),
            );
        }
        if !self
            .attachments
            .iter()
            .any(|item| item.filePath == attachment.filePath)
        {
            let message = format!("已添加附件: {}", attachment.fileName);
            self.attachments.push(attachment);
            self.messageProcessingDelegate.showToast(message);
        }
        Ok(())
    }

    /// Removes one attachment by its stored file path.
    pub fn removeAttachment(&mut self, _filePath: String) {
        self.attachments
            .retain(|attachment| attachment.filePath != _filePath);
    }

    /// Removes every pending attachment from the chat input.
    pub fn clearAttachments(&mut self) {
        self.attachments.clear();
    }

    /// Returns chat ids that currently have active streaming turns.
    pub fn activeStreamingChatIds(&self) -> Vec<String> {
        self.messageProcessingDelegate
            .activeStreamingChatIdsFlow()
            .value()
            .into_iter()
            .collect()
    }

    /// Returns the state flow of chat ids that currently have active streaming turns.
    pub fn activeStreamingChatIdsFlow(&self) -> StateFlow<std::collections::HashSet<String>> {
        self.messageProcessingDelegate.activeStreamingChatIdsFlow()
    }

    /// Returns the state flow of processing states keyed by chat id.
    pub fn inputProcessingStateByChatIdFlow(
        &self,
    ) -> StateFlow<std::collections::HashMap<String, InputProcessingState>> {
        self.messageProcessingDelegate
            .inputProcessingStateByChatIdFlow()
    }

    /// Returns transient toast messages emitted by chat input actions.
    #[allow(non_snake_case)]
    pub fn toastEventFlow(&self) -> StateFlow<Option<String>> {
        self.messageProcessingDelegate.toastEventFlow()
    }

    /// Clears the current transient toast event after the UI has consumed it.
    #[allow(non_snake_case)]
    pub fn clearToastEvent(&mut self) {
        self.messageProcessingDelegate.clearToastEvent();
    }

    /// Returns the processing state for the currently selected chat.
    #[allow(non_snake_case)]
    pub fn currentChatInputProcessingState(&self) -> InputProcessingState {
        let Some(chatId) = self.chatHistoryDelegate.currentChatIdFlow().value() else {
            return InputProcessingState::Idle;
        };
        match self
            .messageProcessingDelegate
            .inputProcessingStateByChatIdFlow()
            .value()
            .get(&chatId)
            .cloned()
        {
            Some(state) => state,
            None => InputProcessingState::Idle,
        }
    }

    /// Returns whether the currently selected chat is actively streaming.
    #[allow(non_snake_case)]
    pub fn currentChatIsLoading(&self) -> bool {
        let Some(chatId) = self.chatHistoryDelegate.currentChatIdFlow().value() else {
            return false;
        };
        self.messageProcessingDelegate
            .activeStreamingChatIdsFlow()
            .value()
            .contains(&chatId)
    }

    /// Returns whether older messages exist beyond the current display window.
    #[allow(non_snake_case)]
    pub fn hasOlderDisplayHistory(&self) -> bool {
        self.chatHistoryDelegate.hasOlderDisplayHistory
    }

    /// Returns whether newer messages exist beyond the current display window.
    #[allow(non_snake_case)]
    pub fn hasNewerDisplayHistory(&self) -> bool {
        self.chatHistoryDelegate.hasNewerDisplayHistory
    }

    /// Returns whether the display-window loader is currently fetching messages.
    #[allow(non_snake_case)]
    pub fn isLoadingDisplayWindow(&self) -> bool {
        self.chatHistoryDelegate.isLoadingDisplayWindow
    }

    /// Returns tool invocation counts for the current turn keyed by chat id.
    pub fn currentTurnToolInvocationCountByChatId(
        &self,
    ) -> &std::collections::HashMap<String, i32> {
        &self
            .messageProcessingDelegate
            .currentTurnToolInvocationCountByChatId
    }

    /// Returns the state flow of tool invocation counts keyed by chat id.
    pub fn currentTurnToolInvocationCountByChatIdFlow(
        &self,
    ) -> StateFlow<std::collections::HashMap<String, i32>> {
        self.messageProcessingDelegate
            .currentTurnToolInvocationCountByChatIdFlow()
    }

    /// Returns the in-memory messages for the current chat.
    pub fn chatHistory(&self) -> Vec<ChatMessage> {
        self.chatHistoryDelegate.currentChatMessagesSnapshot()
    }

    /// Returns the state flow of the currently selected chat id.
    #[allow(non_snake_case)]
    pub async fn currentChatIdFlow(&mut self) -> Result<StateFlow<Option<String>>, String> {
        self.initializeChatConfiguration().await?;
        Ok(self.chatHistoryDelegate.currentChatIdFlow())
    }

    /// Returns a current snapshot of all persisted chat histories.
    pub fn chatHistories(&self) -> Vec<operit_model::ChatHistory::ChatHistory> {
        self.chatHistoryDelegate.chatHistoriesFlow().value()
    }

    /// Returns the state flow of all persisted chat histories.
    #[allow(non_snake_case)]
    pub fn chatHistoriesFlow(&self) -> StateFlow<Vec<operit_model::ChatHistory::ChatHistory>> {
        self.chatHistoryDelegate.chatHistoriesFlow()
    }

    /// Returns chat history list items prepared for grouped history UI.
    #[allow(non_snake_case)]
    pub fn chatHistoryListItemsFlow(&self) -> StateFlow<Vec<ChatHistoryListItem>> {
        self.chatHistoryDelegate.chatHistoryListItemsFlow()
    }

    /// Lists conversation metadata at the executor of the requesting chat.
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.read")]
    pub async fn routedChatListFlow(&self, chatId: String) -> StateFlow<Vec<ChatHistoryListItem>> {
        let _ = chatId;
        self.chatHistoryDelegate
            .chatHistoryListItemsFlow()
            .map(crate::services::core::EdgeChatProjection::compactEdgeHistories)
    }

    /// Creates a conversation from a real routed source without altering another device's current chat.
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.write")]
    pub async fn createRoutedChat(
        &mut self,
        chatId: String,
        input: Option<serde_json::Value>,
    ) -> Result<String, String> {
        self.createNewChat(false, Some(chatId), input).await
    }

    /// Returns messages from the Core selected by Binding for one explicit chat.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.read")]
    pub async fn chatMessagesFlow(&self, chatId: String) -> StateFlow<Vec<ChatMessage>> {
        self.localChatMessagesFlow(chatId)
    }

    /// Watches one bounded display window, addressed by an optional history cursor.
    /// Paging uses the message index; it never transfers complete history or tool payloads.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.read")]
    pub async fn chatMessagesWindowFlow(&self, chatId: String, beforeTimestamp: Option<i64>, beforeTextOffset: Option<u32>, textBytes: u32, textLines: u32) -> StateFlow<operit_model::ChatDisplayWindowState::ChatDisplayWindow> {
        let manager = self.chatHistoryDelegate.chatHistoryManager.clone();
        self.localChatMessagesFlow(chatId.clone()).map(move |live| {
            let messages = if let Some(timestamp) = beforeTimestamp {
                let mut page = match manager.loadChatMessagesDescUpTo(chatId.clone(), timestamp, 14) {
                    Ok(page) => page,
                    Err(error) => return operit_model::ChatDisplayWindowState::ChatDisplayWindow { messages:Vec::new(), older:None, error:Some(error.to_string()) },
                };
                page.reverse(); page
            } else { live };
            crate::services::core::EdgeChatProjection::chatMessageWindow(messages, beforeTimestamp, beforeTextOffset, textBytes, textLines)
        })
    }

    /// Authorizes image access before generic media processing. Display limits belong to callers.
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.read")]
    pub async fn chatImagePreviewChunk(
        &self,
        chatId: String,
        imageId: String,
        offset: u32,
        width: u32,
        height: u32,
        format: String,
        chunkSize: u32,
    ) -> Result<CoreValue, String> {
        use crate::services::media::images::{self, PixelFormat, PreviewOptions};
        let messages = self.localChatMessagesFlow(chatId).value();
        authorizeChatImage(&messages, &imageId)?;
        let pixel_format = match format.as_str() {
            "rgb565le" => PixelFormat::Rgb565Le,
            _ => return Err("Unsupported preview format".into()),
        };
        let chunk = images::preview_chunk(
            &imageId,
            PreviewOptions {
                width,
                height,
                format: pixel_format,
            },
            offset as usize,
            chunkSize as usize,
        )?;
        Ok(CoreValue::Map(std::collections::BTreeMap::from([
            ("format".into(), CoreValue::String(format)),
            ("width".into(), CoreValue::Unsigned(chunk.width as u64)),
            ("height".into(), CoreValue::Unsigned(chunk.height as u64)),
            ("offset".into(), CoreValue::Unsigned(chunk.offset as u64)),
            ("bytes".into(), CoreValue::Bytes(chunk.bytes)),
        ])))
    }

    /// Registers validated image input and projects its pool ID into chat markup.
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.write")]
    pub async fn registerChatImage(
        &self,
        chatId: String,
        imageBytes: Vec<u8>,
        mimeType: String,
    ) -> Result<String, String> {
        let _ = chatId;
        let id = crate::services::media::images::register(&imageBytes, &mimeType)?;
        Ok(format!("<link type=\"image\" id=\"{id}\"></link>"))
    }

    /// Builds a routed diagnostic chat message flow with one embedded response stream.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.read")]
    pub async fn routeProbeChatMessagesFlow(
        &self,
        chatId: String,
        streamText: String,
    ) -> StateFlow<Vec<ChatMessage>> {
        let streamKey = format!("route-probe:{chatId}");
        let streamId = format!("route-probe-stream:{chatId}");
        let source = Arc::new(CoreStreamSource::new({
            let streamKey = streamKey.clone();
            move |request| {
                let (sender, receiver) = CoreEventStream::channel();
                let streamKey = streamKey.clone();
                let chunkOne = format!("{streamText} / chunk-one");
                let chunkTwo = " / chunk-two".to_string();
                let mut markdownStream = MarkdownRenderEventStream::new(streamKey.clone());
                for event in markdownStream.beginSnapshot("") {
                    sender
                        .send(CoreEvent {
                            requestId: Some(request.requestId.clone()),
                            target: request.target.clone(),
                            propertyName: request.propertyName.clone(),
                            kind: CoreEventKind::Changed,
                            value: operit_link::toCoreValue(event)
                                .expect("MarkdownStreamEvent must serialize"),
                        })
                        .expect("route probe stream receiver must be open");
                }
                for chunk in [chunkOne, chunkTwo] {
                    for event in markdownStream.pushChunk(&chunk) {
                        sender
                            .send(CoreEvent {
                                requestId: Some(request.requestId.clone()),
                                target: request.target.clone(),
                                propertyName: request.propertyName.clone(),
                                kind: CoreEventKind::Changed,
                                value: operit_link::toCoreValue(event)
                                    .expect("MarkdownStreamEvent must serialize"),
                            })
                            .expect("route probe stream receiver must be open");
                    }
                }
                sender
                    .send(CoreEvent {
                        requestId: Some(request.requestId.clone()),
                        target: request.target.clone(),
                        propertyName: request.propertyName,
                        kind: CoreEventKind::Completed,
                        value: operit_link::toCoreValue(markdownStream.completed())
                            .expect("MarkdownStreamEvent must serialize"),
                    })
                    .expect("route probe stream receiver must be open");
                Ok(receiver)
            }
        }));
        let mut message = ChatMessage::new("ai".to_string());
        message.contentStream = Some(CoreStream::fromSourceWithId(streamId, source));
        let flow = mutableStateFlow(vec![message]).asStateFlow();
        flow
    }

    fn pluginPageManager(&self) -> operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager {
        let handler = self.runtimeToolHandler();
        ToolPkgBridgeRuntime::new(handler.clone(), handler.getContext()).package_manager()
    }

    /// Uses the existing package runtime; never exposes plugin management to Edge.
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.read")]
    pub fn chatAvailablePlugins(&self, chatId: String, offset: u32, category: Option<String>) -> ChatPluginPage {
        let _ = chatId;
        let manager = self.pluginPageManager();
        let plugins = manager.getToolPkgContainerRuntimes().into_iter()
            .filter(|plugin| manager.isPackageEnabled(&plugin.packageName) && plugin.packageName.len() <= 108)
            .filter(|plugin| match category.as_deref() {
                Some("exclusive") => pluginExclusive(&plugin.manifestExtensions),
                Some("general") => !pluginExclusive(&plugin.manifestExtensions),
                None => true, // Backwards-compatible existing Core readers.
                _ => false,
            })
            .collect::<Vec<_>>();
        ChatPluginPage {
            total: plugins.len().min(u32::MAX as usize) as u32,
            items: plugins.into_iter().skip(offset as usize).take(6).map(|plugin| ChatPluginItem {
                available: plugin.dependencyIssues.is_empty() &&
                    manager.getRegisteredToolPkgMainScript(&plugin.packageName).is_some(),
                id: plugin.packageName,
                name: pluginDisplayText(&plugin.displayName.resolve(false), 42),
            }).collect(),
        }
    }

    /// Bounded metadata only, under the same chat read Binding as the list.
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.read")]
    pub fn chatPluginDetails(&self, chatId: String, packageName: String, toolOffset: u32) -> ChatPluginDetails {
        let _ = chatId;
        let manager = self.pluginPageManager();
        let Some(plugin) = manager.getToolPkgContainerRuntimes().into_iter()
            .find(|p| p.packageName == packageName && p.packageName.len() <= 108 && manager.isPackageEnabled(&p.packageName)) else {
            return ChatPluginDetails { error: "插件已关闭或不可用".into(), ..Default::default() };
        };
        let mut tools = Vec::new();
        for sub in &plugin.subpackages {
            if !manager.isPackageEnabled(&sub.packageName) { continue; }
            if let Some(package) = manager.getEffectivePackageTools(&sub.packageName) {
                tools.extend(package.tools.into_iter().map(|tool| pluginDisplayText(&tool.name, 64)));
            }
        }
        tools.sort(); tools.dedup();
        let total = tools.len().min(u32::MAX as usize) as u32;
        let offset = toolOffset.min(total.saturating_sub(1) / 3 * 3);
        ChatPluginDetails {
            id: plugin.packageName,
            description: pluginDisplayText(&plugin.description.resolve(false), 192),
            tools: tools.into_iter().skip(offset as usize).take(3).collect(),
            toolOffset: offset, toolTotal: total, error: String::new(),
        }
    }

    /// Runs only the plugin-owned diagnostic export in its existing main runtime.
    /// This is execution, not a read-only module-existence check or generic invoke API.
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.write")]
    pub async fn chatPluginStatus(&self, chatId: String, packageName: String, testKind: Option<String>) -> ChatPluginTestResult {
        // Expected diagnostic failures are bounded data, not a service exception
        // which could attach a large JS stack/source to a constrained Link frame.
        let result = self.runChatPluginTest(chatId, packageName, testKind).await;
        ChatPluginTestResult {
            success: result.is_ok(),
            message: result.err().unwrap_or_default().chars().take(80).collect(),
        }
    }

    /// Fixed generic Edge event callback, not an arbitrary tool/JS invocation API.
    /// The Space router authenticates nodeId against the preserved route origin.
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.write")]
    pub async fn chatEdgeEvent(
        &self,
        chatId: String,
        nodeId: String,
        packageName: String,
        payload: serde_json::Value,
    ) -> ChatEdgeEventAck {
        let result = self
            .runEdgeEvent(chatId, nodeId, packageName, payload)
            .await;
        match result {
            Ok(next) => ChatEdgeEventAck {
                success: true,
                next,
                message: String::new(),
            },
            Err(message) => ChatEdgeEventAck {
                success: false,
                next: 0,
                message: message.chars().take(80).collect(),
            },
        }
    }
    async fn runEdgeEvent(
        &self,
        chatId: String,
        nodeId: String,
        packageName: String,
        payload: serde_json::Value,
    ) -> Result<u64, String> {
        use operit_edge_contract::events::{MAX_REQUEST, EdgeEventBatch};
        let batch: EdgeEventBatch = serde_json::from_value(payload.clone())
            .map_err(|_| "Invalid Edge event batch".to_string())?;
        let args = serde_json::json!({"chatId":chatId,"nodeId":nodeId,"packageName":packageName,"payload":payload});
        if chatId.is_empty()
            || chatId.len() > 128
            || nodeId.is_empty()
            || nodeId.len() > 128
            || packageName.is_empty()
            || packageName.len() > 108
            || !batch.valid()
            || serde_json::to_vec(&args)
                .map_err(|_| "Invalid Edge event".to_string())?
                .len()
                > MAX_REQUEST
        {
            return Err("Invalid or oversized Edge event".into());
        }
        let manager = self.pluginPageManager();
        let plugin = manager
            .getToolPkgContainerRuntimes()
            .into_iter()
            .find(|p| p.packageName == packageName)
            .ok_or("Edge receiver plugin is not registered")?;
        if !manager.isPackageEnabled(&packageName)
            || !plugin.dependencyIssues.is_empty()
            || manager
                .getRegisteredToolPkgMainScript(&packageName)
                .is_none()
        {
            return Err("Edge receiver plugin is disabled or unavailable".into());
        }
        // Only this export; enabled checks and nested tool permissions are unchanged.
        // No JS output/stack leaves this bounded ACK. ACK is contingent on persistence.
        let raw = manager.runToolPkgMainHookWithTimeoutMillis(&packageName, "on_edge_event",
            operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_EDGE_EVENT,
            Some("edge_event"), None, None,
            serde_json::json!({"chatId":chatId,"nodeId":nodeId,"batch":payload}),
            None, None, None, 6000).await
            .map_err(|_| "Edge callback failed or timed out".to_string())?
            .ok_or("Edge callback did not acknowledge")?;
        let ack: serde_json::Value =
            serde_json::from_str(&raw).map_err(|_| "Invalid Edge callback ACK")?;
        if ack["accepted"] != true || ack["next"].as_u64() != Some(batch.next) {
            return Err("Edge callback did not acknowledge this batch".into());
        }
        Ok(batch.next)
    }

    async fn runChatPluginTest(&self, chatId: String, packageName: String, testKind: Option<String>) -> Result<(), String> {
        let function = match testKind.as_deref().unwrap_or("connection") {
            "connection" => "test_connection",
            "tool" => "test_tool_call",
            _ => return Err("不支持的插件测试类型".into()),
        };
        let manager = self.pluginPageManager();
        let plugin = manager.getToolPkgContainerRuntimes().into_iter()
            .find(|plugin| plugin.packageName == packageName)
            .ok_or_else(|| "插件未在 Core 注册".to_string())?;
        if !manager.isPackageEnabled(&packageName) { return Err("插件已关闭".into()); }
        if !plugin.dependencyIssues.is_empty() { return Err("插件依赖不可用".into()); }
        if manager.getRegisteredToolPkgMainScript(&packageName).is_none() {
            return Err("插件主模块不可用".into());
        }
        // Fixed export names and no user-supplied tool name/arguments. Existing
        // ToolPkg hook runtime and nested tool permission checks remain in force.
        let raw = manager.runToolPkgMainHookWithTimeoutMillis(
            &packageName, function,
            operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_CORE_COMMAND,
            Some("core_command"), None, None,
            serde_json::json!({"chatId":chatId,"diagnostic":true}),
            None, None, None, 6000,
        ).await.map_err(|_| "插件测试函数不可用、执行失败或超时".to_string())?
            .ok_or_else(|| "插件未提供测试结果".to_string())?;
        let value: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|_| "插件测试结果格式错误".to_string())?;
        // Do not use the JS engine's reserved `success` result-envelope field
        // for a negative diagnostic: it promotes false to an execution exception.
        if value["passed"] != true {
            return Err(value["message"].as_str().unwrap_or("插件测试未通过").chars().take(80).collect());
        }
        Ok(())
    }

    fn inputMenuBridge(&self) -> ToolPkgInputMenuToggleBridge {
        let handler = self.runtimeToolHandler();
        ToolPkgInputMenuToggleBridge::new(ToolPkgBridgeRuntime::new(
            handler.clone(),
            handler.getContext(),
        ))
    }

    /// Reads all composer behavior settings from the chat's execution Core.
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.read")]
    pub async fn chatInputMenuSettings(
        &self,
        chatId: Option<String>,
    ) -> Result<ChatInputMenuSettings, String> {
        let preferences = ApiPreferences::getInstance();
        let bridge = self.inputMenuBridge();
        Ok(ChatInputMenuSettings {
            permissionMode: ToolPermissionSystem::getInstance()
                .getAiPermissionMode()
                .map_err(|e| e.to_string())?,
            disableStreamOutput: preferences
                .disableStreamOutputFlow()
                .first()
                .map_err(|e| e.to_string())?,
            disableUserPreferenceDescription: preferences
                .disableUserPreferenceDescriptionFlow()
                .first()
                .map_err(|e| e.to_string())?,
            pluginChangeVersion: bridge.changeVersion(),
            pluginToggles: bridge.createToggleDefinitionsForFlutter(
                chatId,
                Default::default(),
                Some("main".into()),
            ),
        })
    }

    /// Writes only the supplied fields, preserving concurrent changes to other settings.
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub async fn saveChatInputMenuSettings(
        &self,
        chatId: Option<String>,
        permissionMode: Option<AiPermissionMode>,
        disableStreamOutput: Option<bool>,
        disableUserPreferenceDescription: Option<bool>,
    ) -> Result<(), String> {
        let _ = chatId;
        let preferences = ApiPreferences::getInstance();
        if let Some(value) = permissionMode {
            ToolPermissionSystem::getInstance()
                .saveAiPermissionMode(value)
                .map_err(|e| e.to_string())?;
        }
        if let Some(value) = disableStreamOutput {
            preferences
                .saveDisableStreamOutput(value)
                .map_err(|e| e.to_string())?;
        }
        if let Some(value) = disableUserPreferenceDescription {
            preferences
                .saveDisableUserPreferenceDescription(value)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    #[operit_route_macros::operit_core_route(binding = chatId, permission = "target:runtime.execute")]
    pub async fn triggerChatInputMenuToggle(
        &self,
        chatId: Option<String>,
        toggleId: String,
    ) -> bool {
        self.inputMenuBridge()
            .triggerToggleForFlutter(toggleId, chatId, Some("main".into()))
    }

    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.read")]
    pub async fn chatInputMenuSummary(
        &self,
        chatId: Option<String>,
    ) -> Result<ChatInputMenuSummary, String> {
        let binding = FunctionalConfigManager::default()
            .getModelBindingForFunction(FunctionType::CHAT)
            .map_err(|e| e.to_string())?;
        let config = ModelConfigManager::default()
            .getResolvedModelConfig(&binding.providerId, &binding.modelId)
            .map_err(|e| e.to_string())?;
        let chat = match chatId {
            Some(id) => self
                .chatHistoryDelegate
                .chatHistoryManager
                .loadChatHistory(id)
                .map_err(|e| e.to_string())?,
            None => None,
        };
        Ok(ChatInputMenuSummary {
            currentWindowSize: chat.as_ref().map(|c| c.currentWindowSize).unwrap_or(0),
            inputTokenCount: chat.as_ref().map(|c| c.inputTokens).unwrap_or(0),
            outputTokenCount: chat.as_ref().map(|c| c.outputTokens).unwrap_or(0),
            maxContextLength: f64::from(config.context.maxContextLength),
        })
    }

    /// Responds to a tool permission request through the owning chat route.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.write")]
    pub async fn respondChatToolPermission(
        &self,
        chatId: String,
        requestId: String,
        result: String,
    ) -> Result<(), String> {
        respondChatToolPermission(chatId, requestId, result)
    }

    /// Returns messages from this local Core for one explicit chat.
    #[allow(non_snake_case)]
    pub fn localChatMessagesFlow(&self, chatId: String) -> StateFlow<Vec<ChatMessage>> {
        self.chatHistoryDelegate.chatMessageFlowForChat(chatId)
    }

    /// Returns runtime state from the Core selected by Binding for one explicit chat.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_core_route(binding = chatId, permission = "caller:chat.read")]
    pub async fn chatStateFlow(&self, chatId: String) -> StateFlow<ChatState> {
        self.localChatStateFlow(chatId)
    }

    /// Returns runtime state from this local Core for one explicit chat.
    #[allow(non_snake_case)]
    pub fn localChatStateFlow(&self, chatId: String) -> StateFlow<ChatState> {
        let selectedChatId = chatId;
        let displayWindowStateFlow = self
            .chatHistoryDelegate
            .displayWindowStateFlowForChat(selectedChatId.clone());
        let executionStateFlow = self
            .messageProcessingDelegate
            .executionStateByChatIdFlow()
            .map({
                let selectedChatId = selectedChatId.clone();
                move |states| {
                    states
                        .get(&selectedChatId)
                        .cloned()
                        .unwrap_or_else(ChatExecutionState::idle)
                }
            });
        let chatHistoriesFlow = combine2(
            &self.chatHistoryDelegate.chatHistoriesFlow(),
            &self
                .chatHistoryDelegate
                .chatConfigurationsFlow
                .asStateFlow(),
            |histories, configurations| (histories.clone(), configurations.clone()),
        );
        let pendingQueueStateFlow = self.pendingQueueStateFlow().asStateFlow();
        let toolPermissionRequestsFlow = chatToolPermissionRequestsFlow(selectedChatId.clone());
        combine5(
            &executionStateFlow,
            &displayWindowStateFlow,
            &chatHistoriesFlow,
            &pendingQueueStateFlow,
            &toolPermissionRequestsFlow,
            move |executionState,
                  displayWindowState,
                  chatHistories,
                  pendingQueuesByChatId,
                  toolPermissionRequests| {
                let currentChat = chatHistories
                    .0
                    .iter()
                    .find(|chat| chat.id == selectedChatId);
                let profile = chatHistories
                    .1
                    .get(&selectedChatId)
                    .and_then(|configuration| configuration.identity.as_ref());
                let currentCharacterCardName = profile.map(|identity| identity.title.clone());
                let pendingQueueState = pendingQueuesByChatId.get(&selectedChatId);
                let pendingQueueMessages = pendingQueueState
                    .map(|state| state.messages.clone())
                    .unwrap_or_default();
                let isPendingQueueExpanded = pendingQueueState
                    .map(|state| state.isExpanded)
                    .unwrap_or(true);
                ChatState {
                    currentChatId: selectedChatId.clone(),
                    currentChatTitle: currentChat
                        .map(|chat| chat.title.clone())
                        .unwrap_or_default(),
                    currentCharacterCardAvatarUri: profile
                        .and_then(|profile| profile.avatarUri.clone()),
                    currentCharacterCardName,
                    currentWorkspacePath: currentChat
                        .and_then(|chat| chat.workspacePrimaryPath.clone()),
                    isLoading: executionState.isLoading,
                    inputProcessingState: executionState.inputProcessingState,
                    hasOlderDisplayHistory: displayWindowState.hasOlderDisplayHistory,
                    hasNewerDisplayHistory: displayWindowState.hasNewerDisplayHistory,
                    isLoadingDisplayWindow: displayWindowState.isLoadingDisplayWindow,
                    pendingQueueMessages,
                    isPendingQueueExpanded,
                    toolPermissionRequests,
                }
            },
        )
    }

    /// Returns whether the chat history selector should be visible.
    pub fn showChatHistorySelector(&self) -> bool {
        self.chatHistoryDelegate.showChatHistorySelector
    }

    /// Returns a snapshot of pending input attachments.
    pub fn attachments(&self) -> Vec<AttachmentInfo> {
        self.attachments.clone()
    }

    /// Returns mutable access to chat history operations for host-side integrations.
    pub fn getChatHistoryDelegate(&mut self) -> &mut ChatHistoryDelegate {
        &mut self.chatHistoryDelegate
    }

    /// Returns mutable access to message processing state for host-side integrations.
    pub fn getMessageProcessingDelegate(&mut self) -> &mut MessageProcessingDelegate {
        &mut self.messageProcessingDelegate
    }

    /// Returns mutable access to message coordination state when enhanced AI is initialized.
    pub fn getMessageCoordinationDelegate(&mut self) -> Option<&mut MessageCoordinationDelegate> {
        self.messageCoordinationDelegate.as_mut()
    }

    /// Returns token statistics state owned by the coordination delegate.
    #[allow(non_snake_case)]
    pub fn getTokenStatisticsDelegate(&self) -> Option<&TokenStatisticsDelegate> {
        self.messageCoordinationDelegate
            .as_ref()
            .map(|delegate| &delegate.tokenStatisticsDelegate)
    }

    /// Returns the current context window size state flow.
    #[allow(non_snake_case)]
    pub fn currentWindowSizeFlow(&self) -> StateFlow<i64> {
        self.getTokenStatisticsDelegate()
            .expect("TokenStatisticsDelegate must be initialized")
            .currentWindowSizeFlow()
    }

    /// Returns the cumulative input token count state flow.
    #[allow(non_snake_case)]
    pub fn inputTokenCountFlow(&self) -> StateFlow<i64> {
        self.getTokenStatisticsDelegate()
            .expect("TokenStatisticsDelegate must be initialized")
            .cumulativeInputTokensFlow()
    }

    /// Returns the cumulative output token count state flow.
    #[allow(non_snake_case)]
    pub fn outputTokenCountFlow(&self) -> StateFlow<i64> {
        self.getTokenStatisticsDelegate()
            .expect("TokenStatisticsDelegate must be initialized")
            .cumulativeOutputTokensFlow()
    }

    /// Returns the enhanced AI service used by this chat core.
    pub fn getEnhancedAiService(&self) -> Option<&EnhancedAIService> {
        self.enhancedAiService.as_ref()
    }

    /// Returns whether this chat core has finished delegate initialization.
    pub fn isInitialized(&self) -> bool {
        self.initialized
    }

    /// Registers a callback invoked when the enhanced AI service becomes ready.
    pub fn setOnEnhancedAiServiceReady(&mut self, callback: fn(&EnhancedAIService)) {
        self.onEnhancedAiServiceReady = Some(callback);
    }

    /// Registers an optional callback invoked when a chat turn completes.
    pub fn setAdditionalOnTurnComplete(
        &mut self,
        callback: Option<fn(Option<String>, i32, i32, i32)>,
    ) {
        self.additionalOnTurnComplete = callback;
    }

    /// Replaces the UI bridge used by this chat core.
    pub fn setUiBridge(&mut self, uiBridge: EmptyChatServiceUiBridge) {
        self.uiBridge = uiBridge;
    }

    /// Registers the speech handler used by message playback actions.
    pub fn setSpeakMessageHandler(&mut self, handler: fn(String, bool)) {
        self.messageProcessingDelegate
            .setSpeakMessageHandler(handler);
    }

    /// Reloads chat messages using the display-window strategy for the requested chat.
    pub fn reloadChatMessagesSmart(&mut self, chatId: String) {
        self.chatHistoryDelegate.reloadChatMessagesSmart(chatId);
    }

    /// Loads older messages into the current chat display window.
    pub fn loadOlderMessagesForCurrentChat(&mut self) {
        self.chatHistoryDelegate.loadOlderMessagesForCurrentChat();
    }

    /// Loads newer messages into the current chat display window.
    pub fn loadNewerMessagesForCurrentChat(&mut self) {
        self.chatHistoryDelegate.loadNewerMessagesForCurrentChat();
    }

    /// Moves the current chat display window to the latest messages.
    pub fn showLatestMessagesForCurrentChat(&mut self) {
        self.chatHistoryDelegate.showLatestMessagesForCurrentChat();
    }

    /// Reveals one message inside the current chat display window.
    #[allow(non_snake_case)]
    pub fn revealMessageForCurrentChat(&mut self, targetTimestamp: i64) -> bool {
        self.chatHistoryDelegate
            .revealMessageForCurrentChat(targetTimestamp)
    }

    /// Searches a chat and returns lightweight message previews for navigation.
    #[allow(non_snake_case)]
    pub fn loadChatMessageLocatorPreviews(
        &self,
        chatId: String,
        query: String,
    ) -> Vec<ChatMessageLocatorPreview> {
        self.chatHistoryDelegate
            .loadChatMessageLocatorPreviews(chatId, query)
    }

    /// Marks or unmarks one message as a favorite by message timestamp.
    #[allow(non_snake_case)]
    pub fn setMessageFavorite(&mut self, timestamp: i64, isFavorite: bool) {
        self.chatHistoryDelegate
            .setMessageFavorite(timestamp, isFavorite);
    }
}

#[allow(non_snake_case)]
fn stripXmlLikeTags(text: &str) -> String {
    let mut value = text.to_string();
    for _ in 0..5 {
        let updated = removePairedXmlLikeTags(&value);
        if updated == value {
            break;
        }
        value = updated;
    }
    value = removeSelfClosingXmlLikeTags(&value);
    removeRemainingXmlLikeTags(&value).trim().to_string()
}

#[allow(non_snake_case)]
fn removePairedXmlLikeTags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;

    while let Some(openRelativeStart) = text[cursor..].find('<') {
        let openStart = cursor + openRelativeStart;
        let Some(openEnd) = text[openStart..].find('>').map(|offset| openStart + offset) else {
            break;
        };

        if let Some(tagName) = parseOpeningXmlLikeTag(text, openStart, openEnd) {
            if let Some(closeEnd) = findClosingXmlLikeTagEnd(text, openEnd + 1, tagName) {
                result.push_str(&text[cursor..openStart]);
                cursor = closeEnd;
                continue;
            }
        }

        result.push_str(&text[cursor..openStart + 1]);
        cursor = openStart + 1;
    }

    result.push_str(&text[cursor..]);
    result
}

#[allow(non_snake_case)]
fn removeSelfClosingXmlLikeTags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;

    while let Some(openRelativeStart) = text[cursor..].find('<') {
        let openStart = cursor + openRelativeStart;
        let Some(openEnd) = text[openStart..].find('>').map(|offset| openStart + offset) else {
            break;
        };

        if parseSelfClosingXmlLikeTag(text, openStart, openEnd) {
            result.push_str(&text[cursor..openStart]);
            cursor = openEnd + 1;
            continue;
        }

        result.push_str(&text[cursor..openStart + 1]);
        cursor = openStart + 1;
    }

    result.push_str(&text[cursor..]);
    result
}

#[allow(non_snake_case)]
fn removeRemainingXmlLikeTags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;

    while let Some(openRelativeStart) = text[cursor..].find('<') {
        let openStart = cursor + openRelativeStart;
        let Some(openEnd) = text[openStart..].find('>').map(|offset| openStart + offset) else {
            break;
        };

        result.push_str(&text[cursor..openStart]);
        cursor = openEnd + 1;
    }

    result.push_str(&text[cursor..]);
    result
}

#[allow(non_snake_case)]
fn parseOpeningXmlLikeTag(text: &str, openStart: usize, openEnd: usize) -> Option<&str> {
    let body = text.get(openStart + 1..openEnd)?;
    if body.starts_with('/') || body.trim_end().ends_with('/') {
        return None;
    }
    parseXmlLikeTagName(body)
}

#[allow(non_snake_case)]
fn parseSelfClosingXmlLikeTag(text: &str, openStart: usize, openEnd: usize) -> bool {
    let Some(body) = text.get(openStart + 1..openEnd) else {
        return false;
    };
    if body.starts_with('/') || !body.trim_end().ends_with('/') {
        return false;
    }
    parseXmlLikeTagName(body).is_some()
}

#[allow(non_snake_case)]
fn parseXmlLikeTagName(body: &str) -> Option<&str> {
    let bytes = body.as_bytes();
    let first = *bytes.first()?;
    if !isXmlLikeTagNameStart(first) {
        return None;
    }

    let mut end = 1;
    while end < bytes.len() && isXmlLikeTagNameChar(bytes[end]) {
        end += 1;
    }

    if end < bytes.len() {
        let rest = &body[end..];
        if !rest
            .chars()
            .next()
            .is_some_and(|value| value.is_whitespace())
        {
            return None;
        }
    }

    Some(&body[..end])
}

#[allow(non_snake_case)]
fn findClosingXmlLikeTagEnd(text: &str, from: usize, tagName: &str) -> Option<usize> {
    let mut searchStart = 0;

    while let Some(relativeStart) = text[from + searchStart..].find("</") {
        let closeStart = from + searchStart + relativeStart;
        if let Some(closeEnd) = text[closeStart..]
            .find('>')
            .map(|offset| closeStart + offset)
        {
            let body = &text[closeStart + 2..closeEnd];
            if body.eq_ignore_ascii_case(tagName) {
                return Some(closeEnd + 1);
            }
        }
        searchStart += relativeStart + 2;
    }

    None
}

#[allow(non_snake_case)]
fn isXmlLikeTagNameStart(value: u8) -> bool {
    value.is_ascii_alphabetic()
}

#[allow(non_snake_case)]
fn isXmlLikeTagNameChar(value: u8) -> bool {
    value.is_ascii_alphanumeric() || matches!(value, b':' | b'_' | b'-')
}

/// Reads the identity of the node that actually imported the file, without creating one.
#[allow(non_snake_case)]
fn localAttachmentNodeId() -> Option<String> {
    CoreNodeIdentityStore::localNodeId()
}

#[allow(non_snake_case)]
fn resolveAttachmentPath(filePath: &str) -> Result<PathBuf, String> {
    if filePath.starts_with("file://") {
        let url = Url::parse(filePath).map_err(|_| format!("无法添加附件: {filePath}"))?;
        return fileUrlToPathBuf(&url).map_err(|_| format!("无法添加附件: {filePath}"));
    }
    Ok(PathBuf::from(filePath))
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(non_snake_case)]
fn fileUrlToPathBuf(url: &Url) -> Result<PathBuf, ()> {
    url.to_file_path().map_err(|_| ())
}

#[cfg(target_arch = "wasm32")]
#[allow(non_snake_case)]
fn fileUrlToPathBuf(url: &Url) -> Result<PathBuf, ()> {
    if url.scheme() != "file" {
        return Err(());
    }
    Ok(PathBuf::from(url.path()))
}

#[allow(non_snake_case)]
/// Copies an attachment into clean-on-exit storage through the supplied file-system host.
fn createTempFileFromPath(
    fileSystemHost: &dyn FileSystemHost,
    sourcePath: &Path,
    fileName: &str,
) -> Result<PathBuf, String> {
    let fileExtension = fileName
        .rsplit_once('.')
        .map(|(_, extension)| extension)
        .filter(|extension| !extension.trim().is_empty())
        .unwrap_or("jpg");
    let externalDir = OperitPaths::cleanOnExitDir()?;
    let externalDirText = externalDir.to_string_lossy();
    fileSystemHost
        .makeDirectory(&externalDirText, true)
        .map_err(|error| format!("无法创建附件临时目录: {}", error.message))?;
    let noMediaFile = externalDir.join(".nomedia");
    fileSystemHost
        .writeFile(&noMediaFile.to_string_lossy(), "", false)
        .map_err(|error| format!("无法创建附件媒体标记: {}", error.message))?;
    let tempFile = externalDir.join(format!("img_{}.{}", currentTimeMillis(), fileExtension));
    fileSystemHost
        .copyFile(
            &sourcePath.to_string_lossy(),
            &tempFile.to_string_lossy(),
            false,
        )
        .map_err(|error| format!("无法复制附件: {}", error.message))?;
    let copied = fileSystemHost
        .fileExists(&tempFile.to_string_lossy())
        .map_err(|error| format!("无法读取附件临时文件: {}", error.message))?;
    if !copied.exists || copied.isDirectory || copied.size == 0 {
        return Err(format!("无法添加附件: {}", sourcePath.display()));
    }
    Ok(tempFile)
}

/// Writes pasted image bytes into clean-on-exit storage through the supplied file-system host.
#[allow(non_snake_case)]
fn createTempFileFromBytes(
    fileSystemHost: &dyn FileSystemHost,
    fileName: &str,
    bytes: &[u8],
    slot: usize,
) -> Result<PathBuf, String> {
    let fileExtension = fileName
        .rsplit_once('.')
        .map(|(_, extension)| extension)
        .filter(|extension| !extension.trim().is_empty())
        .ok_or_else(|| format!("无法添加粘贴图片: {fileName}"))?;
    let externalDir = OperitPaths::cleanOnExitDir()?;
    let externalDirText = externalDir.to_string_lossy();
    fileSystemHost
        .makeDirectory(&externalDirText, true)
        .map_err(|error| format!("无法创建附件临时目录: {}", error.message))?;
    let noMediaFile = externalDir.join(".nomedia");
    fileSystemHost
        .writeFile(&noMediaFile.to_string_lossy(), "", false)
        .map_err(|error| format!("无法创建附件媒体标记: {}", error.message))?;
    let tempFile = externalDir.join(format!(
        "pasted_image_{}_{}.{}",
        currentTimeMillis(),
        slot,
        fileExtension
    ));
    fileSystemHost
        .writeFileBytes(&tempFile.to_string_lossy(), bytes)
        .map_err(|error| format!("无法保存粘贴图片: {}", error.message))?;
    let saved = fileSystemHost
        .fileExists(&tempFile.to_string_lossy())
        .map_err(|error| format!("无法读取粘贴图片: {}", error.message))?;
    if !saved.exists || saved.isDirectory || saved.size == 0 {
        return Err(format!("无法添加粘贴图片: {fileName}"));
    }
    Ok(tempFile)
}

#[allow(non_snake_case)]
pub(crate) fn getMimeTypeFromPath(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("heic") => "image/heic",
        Some("txt") => "text/plain",
        Some("json") => "application/json",
        Some("xml") => "application/xml",
        Some("pdf") => "application/pdf",
        Some("doc") | Some("docx") => "application/msword",
        Some("xls") | Some("xlsx") => "application/vnd.ms-excel",
        Some("zip") => "application/zip",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("m4a") => "audio/mp4",
        Some("aac") => "audio/aac",
        Some("ogg") => "audio/ogg",
        Some("flac") => "audio/flac",
        Some("mp4") => "video/mp4",
        Some("mkv") => "video/x-matroska",
        Some("webm") => "video/webm",
        Some("3gp") => "video/3gpp",
        Some("avi") => "video/x-msvideo",
        Some("mov") => "video/quicktime",
        _ => "application/octet-stream",
    }
}

#[allow(non_snake_case)]
fn packageAttachmentPath(packageName: &str) -> String {
    format!("{PACKAGE_ATTACHMENT_PREFIX}{packageName}")
}

#[allow(non_snake_case)]
fn packageAttachmentDisplayName(packageName: &str) -> String {
    format!("包: {packageName}")
}

/// Builds the stored attachment path for a workspace mention.
#[allow(non_snake_case)]
fn workspaceMentionAttachmentPath(relativePath: &str) -> String {
    format!("{WORKSPACE_MENTION_ATTACHMENT_PREFIX}{relativePath}")
}

/// Builds the prompt content for a workspace file mention.
#[allow(non_snake_case)]
fn buildWorkspaceFileMentionContent(
    vfs: &VisualFileSystem,
    fullPath: &str,
    relativePath: &str,
) -> Result<String, String> {
    let content = vfs.readFile(fullPath)?;
    Ok(format!(
        "Selected workspace file: {relativePath}\nThis file was referenced via @ mention.\n\nFile content:\n{content}"
    ))
}

/// Builds the prompt content for a workspace directory mention.
#[allow(non_snake_case)]
fn buildWorkspaceDirectoryMentionContent(
    vfs: &VisualFileSystem,
    fullPath: &str,
    relativePath: &str,
) -> Result<String, String> {
    let mut entries = vfs.listFiles(fullPath)?;
    entries.sort_by(|left, right| {
        (!left.isDirectory)
            .cmp(&(!right.isDirectory))
            .then_with(|| {
                left.name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase())
            })
    });
    let entryText = if entries.is_empty() {
        "(empty)\n".to_string()
    } else {
        entries
            .into_iter()
            .map(|entry| {
                let kind = if entry.isDirectory { "[DIR]" } else { "[FILE]" };
                format!("{kind} {}", entry.name)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    Ok(format!(
        "Selected workspace directory: {relativePath}\nThis directory was referenced via @ mention.\n\nDirectory entries:\n{entryText}"
    ))
}

#[allow(non_snake_case)]
fn isPackageAttachmentError(packageName: &str, packageContent: &str) -> bool {
    if packageContent.trim().is_empty() {
        return true;
    }
    packageContent.starts_with("Package not found: ")
        || packageContent.starts_with("Failed to load package data for: ")
        || packageContent.starts_with("Missing required environment variables for package ")
        || packageContent.starts_with("ToolPkg container '")
        || packageContent.starts_with("MCP server '")
        || packageContent.starts_with("Cannot connect to MCP server")
        || packageContent.starts_with("Cannot get MCP server configuration")
        || packageContent == format!("Skill '{packageName}' is set to not show to AI")
}

#[allow(non_snake_case)]
fn toolFailureMessage(result: &ToolResult) -> String {
    let message = result.error.clone().unwrap_or_default();
    if !message.trim().is_empty() {
        return message;
    }
    result.result.toString()
}

/// Membership is checked on every request, independently of media-cache state.
fn authorizeChatImage(messages: &[ChatMessage], id: &str) -> Result<(), String> {
    use operit_providers::chat::llmprovider::MediaLinkParser::MediaLinkParser;
    if id.is_empty()
        || id.len() > 80
        || !messages.iter().any(|message| {
            MediaLinkParser::extract_image_link_ids(&message.displayText())
                .iter()
                .any(|candidate| candidate == id)
        })
    {
        return Err("Image is not referenced by this chat".into());
    }
    Ok(())
}

#[cfg(test)]
mod image_access_tests {
    use super::*;
    #[test]
    fn image_access_is_scoped_to_the_current_transcript() {
        let messages = vec![ChatMessage::new_with_markdown(
            "user".into(),
            "<link type=\"image\" id=\"image-1\"></link>".into(),
        )];
        assert!(authorizeChatImage(&messages, "image-1").is_ok());
        assert!(authorizeChatImage(&[], "image-1").is_err());
        assert!(authorizeChatImage(&messages, "image-2").is_err());
        assert!(authorizeChatImage(&messages, "").is_err());
    }
}

#[cfg(test)]
mod edge_plugin_metadata_tests {
    use super::*;
    #[test]
    fn explicit_manifest_classification_and_utf8_projection() {
        let mut extensions = std::collections::BTreeMap::new();
        assert!(!pluginExclusive(&extensions));
        extensions.insert("esp32".into(), serde_json::json!({"exclusive":true}));
        assert!(!pluginExclusive(&extensions), "Core must not classify by board type");
        extensions.insert("edge".into(), serde_json::json!({"exclusive":true}));
        assert!(pluginExclusive(&extensions));
        extensions.insert("edge".into(), serde_json::json!({"exclusive":"true"}));
        assert!(!pluginExclusive(&extensions));
        assert_eq!(pluginDisplayText("中".repeat(100).as_str(), 191).len(), 189);
        assert_eq!(pluginDisplayText("a\u{1e}b\n", 42), "ab");
        let details = ChatPluginDetails { id:"a".repeat(108),description:"中".repeat(64),
            tools:vec!["a".repeat(64);3],toolTotal:999, ..Default::default() };
        assert!(serde_json::to_vec(&details).unwrap().len() < 768);
    }
}
