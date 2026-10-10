use crate::plugins::toolpkg::ToolPkgChatLifecycleHookBridge::{
    ToolPkgChatCreationChat, ToolPkgChatCreationDraft, ToolPkgChatCreationKind,
    ToolPkgChatLifecycleHookBridge,
};
use crate::plugins::toolpkg::ToolPkgChatMessageHookBridge::ToolPkgChatMessageHookBridge;
use crate::plugins::toolpkg::ToolPkgChatViewHookBridge::{
    ChatViewEvent, ChatViewHookParams, ToolPkgChatViewHookBridge,
};
use crate::plugins::toolpkg::ToolPkgInputMenuToggleBridge::ToolPkgInputMenuToggleBridge;
use crate::services::ProviderRuntimeSupportService::ChatConfigurationApi;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_model::ChatDisplayWindowState::ChatDisplayWindowState;
use operit_model::ChatHistory::ChatHistory;
use operit_model::ChatHistoryListItem::ChatHistoryListItem;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatMessageLocatorPreview::ChatMessageLocatorPreview;
use operit_model::FunctionType::FunctionType;
use operit_model::PluginExtensionTarget::PluginExtensionTarget;
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_providers::runtime_support::{ChatConfigurationDisplayResult, ChatConfigurationPurpose, ChatConfigurationRequest};
use operit_store::repository::ChatHistoryManager::ChatHistoryManager;
use operit_store::PreferencesDataStore::{mutableStateFlow, MutableStateFlow, StateFlow};
use operit_store::SyncOperationStore::SyncClock;
use operit_tools::files::PathMapper::PathMapper;
use operit_util::AppLogger::AppLogger;
use operit_util::ChainLogger::{self, MESSAGE_STORE_CHAIN};
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Number of persisted messages loaded per display-window query.
pub const DISPLAY_WINDOW_QUERY_BATCH_SIZE: usize = 80;
/// Number of user or summary messages that close one display page.
const DISPLAY_PAGE_TRIGGER_COUNT: usize = 5;
/// Number of display pages kept visible around locator jumps.
const MAX_DISPLAY_PAGE_COUNT: usize = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
/// Defines one timestamp range used by the chat locator display window.
struct DisplayPageRange {
    startIndex: usize,
    endIndexExclusive: usize,
    startTimestampInclusive: i64,
    endTimestampInclusive: i64,
}

#[derive(Clone, Debug, PartialEq)]
/// Defines whether chat selection follows global persisted state or stays local.
pub enum ChatSelectionMode {
    FOLLOW_GLOBAL,
    LOCAL_ONLY,
}

#[derive(Clone, Debug, PartialEq)]
/// Tracks paging state for the currently displayed chat window.
pub struct CurrentChatWindowController {
    pub hasOlderDisplayHistory: bool,
    pub hasNewerDisplayHistory: bool,
    pub isLoadingDisplayWindow: bool,
}

impl CurrentChatWindowController {
    /// Creates an empty display-window controller.
    pub fn new() -> Self {
        Self {
            hasOlderDisplayHistory: false,
            hasNewerDisplayHistory: false,
            isLoadingDisplayWindow: false,
        }
    }

    /// Resets all display-window paging flags.
    pub fn reset(&mut self) {
        self.hasOlderDisplayHistory = false;
        self.hasNewerDisplayHistory = false;
        self.isLoadingDisplayWindow = false;
    }
}

/// Builds a compact message summary for chat-flow diagnostics.
fn chat_message_trace_summary(message: &ChatMessage) -> String {
    format!(
        "sender={} timestamp={} parts={} displayChars={} streamSet={} completedAt={}",
        message.sender,
        message.timestamp,
        message.parts.len(),
        message.displayText().chars().count(),
        message.contentStream.is_some(),
        message.completedAt
    )
}

/// Counts messages that still carry live embedded streams.
fn chat_flow_stream_count(messages: &[ChatMessage]) -> usize {
    messages
        .iter()
        .filter(|message| message.contentStream.is_some())
        .count()
}

/// Builds one compact snapshot summary for a chat-flow window.
fn chat_flow_trace_summary(messages: &[ChatMessage]) -> String {
    let first = messages
        .first()
        .map(chat_message_trace_summary)
        .unwrap_or_else(|| "none".to_string());
    let last = messages
        .last()
        .map(chat_message_trace_summary)
        .unwrap_or_else(|| "none".to_string());
    format!(
        "count={} streamMessages={} first={} last={}",
        messages.len(),
        chat_flow_stream_count(messages),
        first,
        last
    )
}

/// Preserves unaffected live stream projections while applying a persistence snapshot.
fn preserveLiveMessageStreams(
    previous: &[ChatMessage],
    loaded: &mut [ChatMessage],
    invalidatedMessageTimestamp: Option<i64>,
) {
    let liveMessages = previous
        .iter()
        .filter(|message| {
            message.contentStream.is_some()
                && Some(message.timestamp) != invalidatedMessageTimestamp
        })
        .map(|message| (message.timestamp, message))
        .collect::<HashMap<_, _>>();
    for message in loaded {
        let Some(previousMessage) = liveMessages.get(&message.timestamp) else {
            continue;
        };
        message.contentStream = previousMessage.contentStream.clone();
        message.parts = previousMessage.parts.clone();
        message.completedAt = previousMessage.completedAt;
        message.completedExecutionGeneration = previousMessage.completedExecutionGeneration;
    }
}

/// Splits locator previews into timestamp ranges from oldest to newest.
fn resolveDisplayPageRanges(messages: &[ChatMessageLocatorPreview]) -> Vec<DisplayPageRange> {
    if messages.is_empty() {
        return Vec::new();
    }

    let mut pageStartIndicesNewestFirst = Vec::<usize>::new();
    let mut cursor = messages.len() as isize - 1;

    while cursor >= 0 {
        let mut pageStartIndex = 0usize;
        let mut triggerCountInCurrentPage = 0usize;
        let mut pageClosed = false;

        while cursor >= 0 && !pageClosed {
            let messageIndex = cursor as usize;
            let message = &messages[messageIndex];
            if isDisplayPageTriggerMessage(&message.sender) {
                triggerCountInCurrentPage += 1;

                if message.sender == "summary" {
                    pageStartIndex = messageIndex;
                    cursor -= 1;
                    pageClosed = true;
                }

                if !pageClosed && triggerCountInCurrentPage >= DISPLAY_PAGE_TRIGGER_COUNT {
                    pageStartIndex = messageIndex;
                    cursor -= 1;
                    pageClosed = true;
                }
            }

            if !pageClosed {
                cursor -= 1;
            }
        }

        if !pageClosed {
            pageStartIndex = 0;
            cursor = -1;
        }

        pageStartIndicesNewestFirst.push(pageStartIndex);
    }

    pageStartIndicesNewestFirst.reverse();
    pageStartIndicesNewestFirst
        .iter()
        .enumerate()
        .map(|(index, pageStartIndex)| {
            let pageEndExclusive = pageStartIndicesNewestFirst
                .get(index + 1)
                .copied()
                .unwrap_or(messages.len());
            DisplayPageRange {
                startIndex: *pageStartIndex,
                endIndexExclusive: pageEndExclusive,
                startTimestampInclusive: messages[*pageStartIndex].timestamp,
                endTimestampInclusive: messages[pageEndExclusive - 1].timestamp,
            }
        })
        .collect()
}

/// Returns whether the sender closes display pages while navigating history.
fn isDisplayPageTriggerMessage(sender: &str) -> bool {
    sender == "user" || sender == "summary"
}

/// Coordinates chat history persistence, current-chat state, and display-window updates.
pub struct ChatHistoryDelegate {
    pub chatHistoryManager: ChatHistoryManager,
    pub selectionMode: ChatSelectionMode,
    pub chatMessageFlowsByChatId: Arc<Mutex<HashMap<String, MutableStateFlow<Vec<ChatMessage>>>>>,
    pub currentChatWindow: CurrentChatWindowController,
    pub displayWindowStateFlow: MutableStateFlow<ChatDisplayWindowState>,
    pub displayWindowStateFlowsByChatId:
        Arc<Mutex<HashMap<String, MutableStateFlow<ChatDisplayWindowState>>>>,
    pub hasOlderDisplayHistory: bool,
    pub hasNewerDisplayHistory: bool,
    pub isLoadingDisplayWindow: bool,
    pub showChatHistorySelector: bool,
    pub chatHistoriesFlow: StateFlow<Vec<ChatHistory>>,
    pub chatHistoryListItemsFlow: StateFlow<Vec<ChatHistoryListItem>>,
    pub currentChatIdFlow: MutableStateFlow<Option<String>>,
    pub chatConfigurationsFlow: MutableStateFlow<HashMap<String, ChatConfigurationDisplayResult>>,
    pub isInitialized: bool,
    pub allowAddMessage: bool,
    pub beforeDestructiveHistoryMutation: Option<fn(String)>,
    pub afterDestructiveHistoryMutation: Option<fn(String)>,
    pub pendingPersistChatOrderJob: Option<String>,
}

impl ChatHistoryDelegate {
    /// Creates a chat history delegate for the requested selection mode.
    pub fn new(selectionMode: ChatSelectionMode) -> Self {
        let chatHistoryManager = ChatHistoryManager::default()
            .expect("ChatHistoryManager must initialize for ChatHistoryDelegate");
        let chatHistoriesFlow = chatHistoryManager
            .chatHistoriesFlow()
            .expect("ChatHistoryManager.chatHistoriesFlow must succeed");
        let chatHistoryListItemsFlow = chatHistoriesFlow.map(|histories| {
            histories
                .iter()
                .map(ChatHistoryListItem::fromChatHistory)
                .collect::<Vec<_>>()
        });
        let chatMessageFlowsByChatId = Arc::new(Mutex::new(HashMap::new()));
        let displayWindowStateFlow = mutableStateFlow(ChatDisplayWindowState::default());
        let displayWindowStateFlowsByChatId = Arc::new(Mutex::new(HashMap::new()));
        let currentChatIdFlow = mutableStateFlow(None);
        let mut delegate = Self {
            chatHistoryManager,
            selectionMode,
            chatMessageFlowsByChatId,
            currentChatWindow: CurrentChatWindowController::new(),
            displayWindowStateFlow,
            displayWindowStateFlowsByChatId,
            hasOlderDisplayHistory: false,
            hasNewerDisplayHistory: false,
            isLoadingDisplayWindow: false,
            showChatHistorySelector: false,
            chatHistoriesFlow,
            chatHistoryListItemsFlow,
            currentChatIdFlow,
            chatConfigurationsFlow: mutableStateFlow(HashMap::new()),
            isInitialized: false,
            allowAddMessage: true,
            beforeDestructiveHistoryMutation: None,
            afterDestructiveHistoryMutation: None,
            pendingPersistChatOrderJob: None,
        };

        delegate
    }

    #[allow(non_snake_case)]
    /// Clones the delegate while reusing live state-flow handles for core services.
    pub fn clone_for_core(&self) -> Self {
        Self {
            chatHistoryManager: self.chatHistoryManager.clone(),
            selectionMode: self.selectionMode.clone(),
            chatMessageFlowsByChatId: self.chatMessageFlowsByChatId.clone(),
            currentChatWindow: self.currentChatWindow.clone(),
            displayWindowStateFlow: self.displayWindowStateFlow.clone(),
            displayWindowStateFlowsByChatId: self.displayWindowStateFlowsByChatId.clone(),
            hasOlderDisplayHistory: self.hasOlderDisplayHistory,
            hasNewerDisplayHistory: self.hasNewerDisplayHistory,
            isLoadingDisplayWindow: self.isLoadingDisplayWindow,
            showChatHistorySelector: self.showChatHistorySelector,
            chatHistoriesFlow: self.chatHistoriesFlow.clone(),
            chatHistoryListItemsFlow: self.chatHistoryListItemsFlow.clone(),
            currentChatIdFlow: self.currentChatIdFlow.clone(),
            chatConfigurationsFlow: self.chatConfigurationsFlow.clone(),
            isInitialized: self.isInitialized,
            allowAddMessage: self.allowAddMessage,
            beforeDestructiveHistoryMutation: self.beforeDestructiveHistoryMutation,
            afterDestructiveHistoryMutation: self.afterDestructiveHistoryMutation,
            pendingPersistChatOrderJob: self.pendingPersistChatOrderJob.clone(),
        }
    }

    /// Returns the mutable message flow for one chat, creating its indexed display window once.
    fn mutableChatMessageFlowForChat(&self, chatId: String) -> MutableStateFlow<Vec<ChatMessage>> {
        if let Some(flow) = self
            .chatMessageFlowsByChatId
            .lock()
            .expect("chat message flow registry mutex must not be poisoned")
            .get(&chatId)
            .cloned()
        {
            return flow;
        }
        let messages =
            self.collectNewestDisplayPages(chatId.clone(), self.displayWindowQueryLimit(), None);
        AppLogger::trace(
            "ChatFlowTrace",
            &format!(
                "flow.create chatId={} {}",
                chatId,
                chat_flow_trace_summary(&messages)
            ),
        );
        let flow = mutableStateFlow(messages);
        let mut flows = self
            .chatMessageFlowsByChatId
            .lock()
            .expect("chat message flow registry mutex must not be poisoned");
        let flow = flows.entry(chatId).or_insert_with(|| flow).clone();
        flow
    }

    /// Returns the independently addressable message-window flow for one chat id.
    pub fn chatMessageFlowForChat(&self, chatId: String) -> StateFlow<Vec<ChatMessage>> {
        self.mutableChatMessageFlowForChat(chatId).asStateFlow()
    }

    /// Returns the current chat message snapshot from its shared in-memory flow.
    pub fn currentChatMessagesSnapshot(&self) -> Vec<ChatMessage> {
        let Some(chatId) = self.currentChatIdFlow.value() else {
            return Vec::new();
        };
        self.mutableChatMessageFlowForChat(chatId).value()
    }

    /// Returns the message snapshot for one explicit chat id.
    #[allow(non_snake_case)]
    pub fn chatMessagesSnapshotForChat(&self, chatId: String) -> Vec<ChatMessage> {
        self.mutableChatMessageFlowForChat(chatId).value()
    }

    /// Returns the independently addressable display-window flow for one chat id.
    pub fn displayWindowStateFlowForChat(
        &self,
        chatId: String,
    ) -> StateFlow<ChatDisplayWindowState> {
        if let Some(flow) = self
            .displayWindowStateFlowsByChatId
            .lock()
            .expect("display-window flow registry mutex must not be poisoned")
            .get(&chatId)
            .cloned()
        {
            return flow.asStateFlow();
        }
        let messages =
            self.collectNewestDisplayPages(chatId.clone(), self.displayWindowQueryLimit(), None);
        let hasOlder = messages
            .first()
            .map(|message| {
                self.chatHistoryManager
                    .hasMessagesBefore(chatId.clone(), message.timestamp)
                    .expect("ChatHistoryManager.hasMessagesBefore must succeed")
            })
            .unwrap_or(false);
        let hasNewer = messages
            .last()
            .map(|message| {
                self.chatHistoryManager
                    .hasMessagesAfter(chatId.clone(), message.timestamp)
                    .expect("ChatHistoryManager.hasMessagesAfter must succeed")
            })
            .unwrap_or(false);
        let flow = mutableStateFlow(ChatDisplayWindowState {
            hasOlderDisplayHistory: hasOlder,
            hasNewerDisplayHistory: hasNewer,
            isLoadingDisplayWindow: false,
        });
        let mut flows = self
            .displayWindowStateFlowsByChatId
            .lock()
            .expect("display-window flow registry mutex must not be poisoned");
        let flow = flows.entry(chatId).or_insert_with(|| flow).clone();
        flow.asStateFlow()
    }

    #[allow(non_snake_case)]
    /// Returns the flow for persisted chat metadata.
    pub fn chatHistoriesFlow(&self) -> StateFlow<Vec<ChatHistory>> {
        self.chatHistoriesFlow.clone()
    }

    #[allow(non_snake_case)]
    /// Returns the flow for chat list rows derived from persisted metadata.
    pub fn chatHistoryListItemsFlow(&self) -> StateFlow<Vec<ChatHistoryListItem>> {
        self.chatHistoryListItemsFlow.clone()
    }

    #[allow(non_snake_case)]
    /// Returns the flow for the selected chat id.
    pub fn currentChatIdFlow(&self) -> StateFlow<Option<String>> {
        self.currentChatIdFlow.asStateFlow()
    }

    #[allow(non_snake_case)]
    /// Returns the flow for display-window paging state.
    pub fn displayWindowStateFlow(&self) -> StateFlow<ChatDisplayWindowState> {
        self.displayWindowStateFlow.asStateFlow()
    }

    #[allow(non_snake_case)]
    fn currentDisplayWindowState(&self) -> ChatDisplayWindowState {
        ChatDisplayWindowState {
            hasOlderDisplayHistory: self.hasOlderDisplayHistory,
            hasNewerDisplayHistory: self.hasNewerDisplayHistory,
            isLoadingDisplayWindow: self.isLoadingDisplayWindow,
        }
    }

    #[allow(non_snake_case)]
    fn emitChatHistoryState(&mut self) {
        if let Some(chatId) = self.currentChatIdFlow.value() {
            self.dispatchChatViewEvent(ChatViewEvent::ViewUpdated, &chatId);
        }
    }

    #[allow(non_snake_case)]
    fn emitDisplayWindowState(&mut self) {
        let state = self.currentDisplayWindowState();
        self.displayWindowStateFlow.set_value(state.clone());
        if let Some(chatId) = self.currentChatIdFlow.value() {
            if let Some(flow) = self
                .displayWindowStateFlowsByChatId
                .lock()
                .expect("display-window flow registry mutex must not be poisoned")
                .get(&chatId)
                .cloned()
            {
                flow.set_value(state);
            }
        }
    }

    /// Reads one already opened chat-flow snapshot without materializing a new flow.
    fn openedChatMessageFlowSnapshot(&self, chatId: &str) -> Option<Vec<ChatMessage>> {
        self.chatMessageFlowsByChatId
            .lock()
            .expect("chat message flow registry mutex must not be poisoned")
            .get(chatId)
            .cloned()
            .map(|flow| flow.value())
    }

    /// Publishes one message replacement to an already opened chat-scoped flow.
    pub fn publishChatMessage(&self, chatId: &str, message: ChatMessage) {
        let messageSummary = chat_message_trace_summary(&message);
        let flow = self
            .chatMessageFlowsByChatId
            .lock()
            .expect("chat message flow registry mutex must not be poisoned")
            .get(chatId)
            .cloned();
        let Some(flow) = flow else {
            AppLogger::trace(
                "ChatFlowTrace",
                &format!(
                    "publish.skipped_no_flow chatId={} {}",
                    chatId, messageSummary
                ),
            );
            return;
        };
        let beforeMessages = flow.value();
        let beforeSummary = chat_flow_trace_summary(&beforeMessages);
        let mut messages = beforeMessages;
        let action = if let Some(index) = messages
            .iter()
            .position(|existing| existing.timestamp == message.timestamp)
        {
            messages[index] = message;
            "replace"
        } else {
            messages.push(message);
            messages.sort_by_key(|item| item.timestamp);
            "insert"
        };
        AppLogger::trace(
            "ChatFlowTrace",
            &format!(
                "publish chatId={} action={} count={} streamMessages={} {}",
                chatId,
                action,
                messages.len(),
                chat_flow_stream_count(&messages),
                messageSummary
            ),
        );
        flow.set_value(messages);
        AppLogger::trace(
            "ChatFlowTrace",
            &format!(
                "publish.done chatId={} action={} before={} after={}",
                chatId,
                action,
                beforeSummary,
                chat_flow_trace_summary(&flow.value())
            ),
        );
    }

    /// Returns a storage-safe copy of a chat message without a live stream handle.
    fn persistentChatMessage(mut message: ChatMessage) -> ChatMessage {
        message.contentStream = None;
        message
    }

    /// Applies one runtime mutation to a message in an already opened chat-scoped flow.
    pub fn updateOpenedChatMessage<F>(&self, chatId: &str, timestamp: i64, update: F)
    where
        F: FnOnce(&mut ChatMessage),
    {
        let flow = self
            .chatMessageFlowsByChatId
            .lock()
            .expect("chat message flow registry mutex must not be poisoned")
            .get(chatId)
            .cloned();
        let Some(flow) = flow else {
            return;
        };
        let mut messages = flow.value();
        if let Some(message) = messages
            .iter_mut()
            .find(|message| message.timestamp == timestamp)
        {
            update(message);
            flow.set_value(messages);
        }
    }

    /// Removes one message from an already opened chat-scoped flow.
    pub fn removeChatMessage(&self, chatId: &str, timestamp: i64) {
        self.retainChatFlowMessages(chatId, |message| message.timestamp != timestamp);
    }

    /// Removes every opened chat-scoped message at or after one timestamp.
    pub fn removeChatMessagesFrom(&self, chatId: &str, timestamp: i64) {
        self.retainChatFlowMessages(chatId, |message| message.timestamp < timestamp);
    }

    /// Applies one in-memory filter to an already opened chat-scoped flow.
    fn retainChatFlowMessages<F>(&self, chatId: &str, retain: F)
    where
        F: FnMut(&ChatMessage) -> bool,
    {
        let flow = self
            .chatMessageFlowsByChatId
            .lock()
            .expect("chat message flow registry mutex must not be poisoned")
            .get(chatId)
            .cloned();
        let Some(flow) = flow else {
            return;
        };
        let mut retain = retain;
        let beforeMessages = flow.value();
        let beforeSummary = chat_flow_trace_summary(&beforeMessages);
        let mut messages = beforeMessages;
        messages.retain(|message| retain(message));
        AppLogger::trace(
            "ChatFlowTrace",
            &format!(
                "retain chatId={} before={} after={}",
                chatId,
                beforeSummary,
                chat_flow_trace_summary(&messages)
            ),
        );
        flow.set_value(messages);
    }

    /// Clears one chat-scoped flow after the chat itself is deleted.
    pub fn clearChatFlow(&self, chatId: &str) {
        let flow = self
            .chatMessageFlowsByChatId
            .lock()
            .expect("chat message flow registry mutex must not be poisoned")
            .get(chatId)
            .cloned();
        if let Some(flow) = flow {
            let beforeMessages = flow.value();
            AppLogger::trace(
                "ChatFlowTrace",
                &format!(
                    "clear chatId={} before={}",
                    chatId,
                    chat_flow_trace_summary(&beforeMessages)
                ),
            );
            flow.set_value(Vec::new());
            AppLogger::trace(
                "ChatFlowTrace",
                &format!("clear.done chatId={} after=count=0", chatId),
            );
        }
    }

    #[allow(non_snake_case)]
    fn dispatchChatViewEvent(&self, event: ChatViewEvent, chatId: &str) {
        ToolPkgChatViewHookBridge::dispatchRegisteredChatViewEvent(
            event,
            self.buildChatViewHookParams(chatId),
        );
    }

    #[allow(non_snake_case)]
    /// Builds the main chat surface event using the shared chat runtime identifier.
    fn buildChatViewHookParams(&self, chatId: &str) -> ChatViewHookParams {
        let chat = self
            .chatHistoryManager
            .loadChatHistory(chatId.to_string())
            .expect("ChatHistoryManager.loadChatHistory must succeed");
        let (workspacePath, title) = match chat.as_ref() {
            Some(chat) => (
                self.primaryWorkspacePathForChat(chat),
                Some(chat.title.clone()),
            ),
            None => (None, None),
        };
        ChatViewHookParams {
            viewId: format!("chat:{chatId}"),
            chatId: chatId.to_string(),
            workspacePath,
            workspaceEnv: serde_json::json!({}),
            runtime: "main".to_string(),
            title,
        }
    }

    #[allow(non_snake_case)]
    /// Registers a hook invoked before destructive history mutations.
    pub fn setBeforeDestructiveHistoryMutation(&mut self, handler: fn(String)) {
        self.beforeDestructiveHistoryMutation = Some(handler);
    }

    #[allow(non_snake_case)]
    /// Registers a hook invoked after destructive history mutations.
    pub fn setAfterDestructiveHistoryMutation(&mut self, handler: fn(String)) {
        self.afterDestructiveHistoryMutation = Some(handler);
    }

    #[allow(non_snake_case)]
    /// Invokes the pre-mutation hook for a chat id.
    pub fn prepareChatForDestructiveMutation(&self, chatId: String) {
        if let Some(handler) = self.beforeDestructiveHistoryMutation {
            handler(chatId);
        }
    }

    #[allow(non_snake_case)]
    /// Invokes the post-mutation hook for a chat id.
    pub fn finishDestructiveHistoryMutation(&self, chatId: String) {
        if let Some(handler) = self.afterDestructiveHistoryMutation {
            handler(chatId);
        }
    }

    #[allow(non_snake_case)]
    /// Clears active chat messages and resets display-window state in memory.
    pub fn clearCurrentChatHistoryInMemory(&mut self) {
        if let Some(chatId) = self.currentChatIdFlow.value() {
            let beforeSummary = self
                .openedChatMessageFlowSnapshot(&chatId)
                .map(|messages| chat_flow_trace_summary(&messages))
                .unwrap_or_else(|| "none".to_string());
            AppLogger::trace(
                "ChatFlowTrace",
                &format!("clear_current chatId={} snapshot={}", chatId, beforeSummary),
            );
            self.clearChatFlow(&chatId);
        }
        self.currentChatWindow.reset();
        self.hasOlderDisplayHistory = false;
        self.hasNewerDisplayHistory = false;
        self.isLoadingDisplayWindow = false;
        self.emitDisplayWindowState();
        self.emitChatHistoryState();
        AppLogger::v_with_level(
            "ChatFlowTrace",
            "clear_current.done",
            operit_util::AppLogger::VERBOSE_LEVEL_5,
        );
    }

    #[allow(non_snake_case)]
    /// Replaces active chat messages in memory and optionally updates paging flags.
    pub fn setCurrentChatMessagesInMemory(
        &mut self,
        messages: Vec<ChatMessage>,
        hasOlderPersistedHistory: Option<bool>,
        hasNewerPersistedHistory: Option<bool>,
    ) {
        if let Some(chatId) = self.currentChatIdFlow.value() {
            let flow = self.mutableChatMessageFlowForChat(chatId.clone());
            let beforeMessages = flow.value();
            let beforeSummary = chat_flow_trace_summary(&beforeMessages);
            let incomingSummary = chat_flow_trace_summary(&messages);
            AppLogger::trace(
                "ChatFlowTrace",
                &format!(
                    "set_current chatId={} before={} incoming={} hasOlder={} hasNewer={}",
                    chatId,
                    beforeSummary,
                    incomingSummary,
                    hasOlderPersistedHistory.unwrap_or(false),
                    hasNewerPersistedHistory.unwrap_or(false)
                ),
            );
            flow.set_value(messages);
            AppLogger::trace(
                "ChatFlowTrace",
                &format!(
                    "set_current.done chatId={} after={}",
                    chatId,
                    chat_flow_trace_summary(&flow.value())
                ),
            );
        }
        self.emitChatHistoryState();
        if let Some(value) = hasOlderPersistedHistory {
            self.currentChatWindow.hasOlderDisplayHistory = value;
            self.hasOlderDisplayHistory = value;
        }
        if let Some(value) = hasNewerPersistedHistory {
            self.currentChatWindow.hasNewerDisplayHistory = value;
            self.hasNewerDisplayHistory = value;
        }
        self.emitDisplayWindowState();
    }

    #[allow(non_snake_case)]
    /// Refreshes active chat display flags while preserving existing paging metadata.
    pub fn refreshCurrentChatDisplayFlags(&mut self, _chatId: String, messages: Vec<ChatMessage>) {
        self.setCurrentChatMessagesInMemory(messages, None, None);
    }

    #[allow(non_snake_case)]
    /// Returns the fixed query limit used by one display-window load.
    pub fn displayWindowQueryLimit(&self) -> i32 {
        DISPLAY_WINDOW_QUERY_BATCH_SIZE as i32
    }

    #[allow(non_snake_case)]
    /// Loads the newest display window without hydrating the entire conversation.
    pub fn collectNewestDisplayPages(
        &self,
        chatId: String,
        _pageCount: i32,
        endTimestampInclusive: Option<i64>,
    ) -> Vec<ChatMessage> {
        let messages = match endTimestampInclusive {
            Some(endTimestamp) => self
                .chatHistoryManager
                .loadChatMessagesDescUpTo(
                    chatId,
                    endTimestamp,
                    DISPLAY_WINDOW_QUERY_BATCH_SIZE as i32,
                )
                .expect("ChatHistoryManager.loadChatMessagesDescUpTo must succeed"),
            None => self
                .chatHistoryManager
                .loadChatMessagesDesc(chatId, DISPLAY_WINDOW_QUERY_BATCH_SIZE as i32, None)
                .expect("ChatHistoryManager.loadChatMessagesDesc must succeed"),
        };
        let mut ordered = messages;
        ordered.reverse();
        ordered
    }

    #[allow(non_snake_case)]
    /// Loads one older display window directly from the message index.
    pub fn collectOlderDisplayPagesBefore(
        &self,
        chatId: String,
        _pageCount: i32,
        beforeTimestampExclusive: i64,
    ) -> Vec<ChatMessage> {
        self.chatHistoryManager
            .loadOlderChatMessages(
                chatId,
                beforeTimestampExclusive,
                DISPLAY_WINDOW_QUERY_BATCH_SIZE as i32,
            )
            .expect("ChatHistoryManager.loadOlderChatMessages must succeed")
    }

    #[allow(non_snake_case)]
    /// Loads one newer display window directly from the message index.
    pub fn collectNewerDisplayPagesAfter(
        &self,
        chatId: String,
        _pageCount: i32,
        afterTimestampExclusive: i64,
    ) -> Vec<ChatMessage> {
        self.chatHistoryManager
            .loadChatMessagesAscAfter(
                chatId,
                afterTimestampExclusive,
                DISPLAY_WINDOW_QUERY_BATCH_SIZE as i32,
            )
            .expect("ChatHistoryManager.loadChatMessagesAscAfter must succeed")
    }

    #[allow(non_snake_case)]
    /// Loads and applies the newest indexed display window for the active chat.
    pub fn loadLatestCurrentChatDisplayWindow(&mut self) -> Vec<ChatMessage> {
        let Some(chatId) = self.currentChatIdFlow.value() else {
            self.clearCurrentChatHistoryInMemory();
            return Vec::new();
        };
        let previousMessages = self
            .openedChatMessageFlowSnapshot(&chatId)
            .unwrap_or_default();
        let currentSummary = chat_flow_trace_summary(&previousMessages);
        let mut messages =
            self.collectNewestDisplayPages(chatId.clone(), self.displayWindowQueryLimit(), None);
        preserveLiveMessageStreams(&previousMessages, &mut messages, None);
        let loadedSummary = chat_flow_trace_summary(&messages);
        let hasOlder = messages
            .first()
            .map(|message| {
                self.chatHistoryManager
                    .hasMessagesBefore(chatId.clone(), message.timestamp)
                    .expect("ChatHistoryManager.hasMessagesBefore must succeed")
            })
            .unwrap_or(false);
        let hasNewer = messages
            .last()
            .map(|message| {
                self.chatHistoryManager
                    .hasMessagesAfter(chatId.clone(), message.timestamp)
                    .expect("ChatHistoryManager.hasMessagesAfter must succeed")
            })
            .unwrap_or(false);
        AppLogger::trace(
            "ChatFlowTrace",
            &format!(
                "load_latest chatId={} current={} loaded={} hasOlder={} hasNewer={}",
                chatId, currentSummary, loadedSummary, hasOlder, hasNewer
            ),
        );
        self.setCurrentChatMessagesInMemory(messages.clone(), Some(hasOlder), Some(hasNewer));
        messages
    }

    #[allow(non_snake_case)]
    /// Reloads the current indexed display window for the supplied chat id.
    pub fn reloadCurrentChatDisplayHistory(&mut self, chatId: String) -> Vec<ChatMessage> {
        self.reloadCurrentChatDisplayHistoryWithInvalidatedRevision(chatId, None)
    }

    /// Reloads persisted revisions without retaining a mutated message's obsolete stream projection.
    fn reloadCurrentChatDisplayHistoryWithInvalidatedRevision(
        &mut self,
        chatId: String,
        invalidatedMessageTimestamp: Option<i64>,
    ) -> Vec<ChatMessage> {
        let previousMessages = self
            .openedChatMessageFlowSnapshot(&chatId)
            .unwrap_or_default();
        let currentSummary = chat_flow_trace_summary(&previousMessages);
        let displayEndTimestamp = previousMessages
            .last()
            .filter(|_| self.currentChatWindow.hasNewerDisplayHistory)
            .map(|message| message.timestamp);
        let mut messages = self.collectNewestDisplayPages(
            chatId.clone(),
            self.displayWindowQueryLimit(),
            displayEndTimestamp,
        );
        preserveLiveMessageStreams(
            &previousMessages,
            &mut messages,
            invalidatedMessageTimestamp,
        );
        let loadedSummary = chat_flow_trace_summary(&messages);
        let hasOlder = messages
            .first()
            .map(|message| {
                self.chatHistoryManager
                    .hasMessagesBefore(chatId.clone(), message.timestamp)
                    .expect("ChatHistoryManager.hasMessagesBefore must succeed")
            })
            .unwrap_or(false);
        let hasNewer = messages
            .last()
            .map(|message| {
                self.chatHistoryManager
                    .hasMessagesAfter(chatId.clone(), message.timestamp)
                    .expect("ChatHistoryManager.hasMessagesAfter must succeed")
            })
            .unwrap_or(false);
        AppLogger::trace(
            "ChatFlowTrace",
            &format!(
                "reload_display chatId={} current={} loaded={} hasOlder={} hasNewer={}",
                chatId, currentSummary, loadedSummary, hasOlder, hasNewer
            ),
        );
        self.applyCurrentChatDisplayWindowWithFlags(chatId, messages, hasOlder, hasNewer)
    }

    #[allow(non_snake_case)]
    /// Applies a display window together with its persisted boundary flags.
    fn applyCurrentChatDisplayWindowWithFlags(
        &mut self,
        _chatId: String,
        messages: Vec<ChatMessage>,
        hasOlder: bool,
        hasNewer: bool,
    ) -> Vec<ChatMessage> {
        self.setCurrentChatMessagesInMemory(messages.clone(), Some(hasOlder), Some(hasNewer));
        messages
    }

    #[allow(non_snake_case)]
    /// Runs a destructive history mutation with before and after hooks.
    pub fn runDestructiveHistoryMutation<F>(&mut self, chatId: String, mutation: F) -> bool
    where
        F: FnOnce(&mut Self, String) -> bool,
    {
        self.prepareChatForDestructiveMutation(chatId.clone());
        let changed = mutation(self, chatId.clone());
        if changed {
            self.finishDestructiveHistoryMutation(chatId);
        }
        changed
    }

    #[allow(non_snake_case)]
    /// Runs a destructive history mutation for the currently selected chat.
    pub fn runCurrentChatDestructiveHistoryMutation<F>(
        &mut self,
        _staleMessage: String,
        mutation: F,
    ) -> bool
    where
        F: FnOnce(&mut Self, String) -> bool,
    {
        let Some(chatId) = self.currentChatIdFlow.value() else {
            return false;
        };
        self.runDestructiveHistoryMutation(chatId, mutation)
    }

    #[allow(non_snake_case)]
    /// Loads all persisted messages for a chat.
    pub fn getChatHistory(&self, chatId: String) -> Vec<ChatMessage> {
        self.chatHistoryManager
            .loadChatMessages(&chatId)
            .expect("ChatHistoryManager.loadChatMessages must succeed")
    }

    #[allow(non_snake_case)]
    /// Loads persisted messages that should participate in runtime model context.
    pub fn getRuntimeChatHistory(&self, chatId: String) -> Vec<ChatMessage> {
        self.getChatHistory(chatId)
            .into_iter()
            .filter(|message| message.displayMode != operit_model::ChatMessageDisplayMode::ChatMessageDisplayMode::HIDDEN_PLACEHOLDER)
            .collect()
    }

    /// Loads runtime chat messages through one inclusive message timestamp.
    #[allow(non_snake_case)]
    pub fn getRuntimeChatHistoryUpTo(
        &self,
        chatId: String,
        upToTimestampInclusive: i64,
    ) -> Vec<ChatMessage> {
        self.chatHistoryManager
            .loadMessagesAfterLatestSummaryInRange(chatId, None, Some(upToTimestampInclusive))
            .expect("ChatHistoryManager.loadMessagesAfterLatestSummaryInRange must succeed")
    }

    #[allow(non_snake_case)]
    /// Loads messages used when inserting or refreshing conversation summaries.
    pub fn loadMessagesForSummaryInsertion(
        &self,
        chatId: String,
        beforeTimestampExclusive: Option<i64>,
        upToTimestampInclusive: Option<i64>,
    ) -> Vec<ChatMessage> {
        self.getRuntimeChatHistory(chatId)
            .into_iter()
            .filter(|message| {
                beforeTimestampExclusive
                    .map(|ts| message.timestamp < ts)
                    .unwrap_or(true)
            })
            .filter(|message| {
                upToTimestampInclusive
                    .map(|ts| message.timestamp <= ts)
                    .unwrap_or(true)
            })
            .collect()
    }

    #[allow(non_snake_case)]
    /// Loads compact locator previews for messages matching a query.
    pub fn loadChatMessageLocatorPreviews(
        &self,
        chatId: String,
        query: String,
    ) -> Vec<ChatMessageLocatorPreview> {
        self.chatHistoryManager
            .loadChatMessageLocatorPreviews(chatId, query)
            .expect("load chat message locator previews")
    }

    #[allow(non_snake_case)]
    /// Returns whether a chat contains at least one user-authored message.
    pub fn hasUserMessage(&self, chatId: String) -> bool {
        self.getChatHistory(chatId)
            .iter()
            .any(|message| message.sender == "user")
    }

    #[allow(non_snake_case)]
    /// Reveals the target message inside the active chat display window.
    pub fn revealMessageForCurrentChat(&mut self, targetTimestamp: i64) -> bool {
        if self
            .currentChatMessagesSnapshot()
            .iter()
            .any(|message| message.timestamp == targetTimestamp)
        {
            return true;
        }

        let Some(chatId) = self.currentChatIdFlow.value() else {
            return false;
        };
        if self.isLoadingDisplayWindow {
            return false;
        }

        self.isLoadingDisplayWindow = true;
        self.currentChatWindow.isLoadingDisplayWindow = true;
        self.emitDisplayWindowState();

        let locatorEntries = self
            .chatHistoryManager
            .loadChatMessageLocatorPreviews(chatId.clone(), String::new())
            .expect("ChatHistoryManager.loadChatMessageLocatorPreviews must succeed");
        let pageRanges = resolveDisplayPageRanges(&locatorEntries);
        let Some(targetPageIndex) = pageRanges.iter().position(|range| {
            targetTimestamp >= range.startTimestampInclusive
                && targetTimestamp <= range.endTimestampInclusive
        }) else {
            self.isLoadingDisplayWindow = false;
            self.currentChatWindow.isLoadingDisplayWindow = false;
            self.emitDisplayWindowState();
            return false;
        };

        let windowStartPageIndex = if targetPageIndex + 1 < pageRanges.len() {
            targetPageIndex
        } else {
            targetPageIndex.saturating_sub(MAX_DISPLAY_PAGE_COUNT - 1)
        };
        let windowEndPageIndex =
            (windowStartPageIndex + MAX_DISPLAY_PAGE_COUNT - 1).min(pageRanges.len() - 1);
        let startTimestamp = pageRanges[windowStartPageIndex].startTimestampInclusive;
        let endTimestamp = pageRanges[windowEndPageIndex].endTimestampInclusive;
        let mut revealedMessages = self
            .chatHistoryManager
            .loadChatMessagesWindow(chatId.clone(), startTimestamp, endTimestamp)
            .expect("ChatHistoryManager.loadChatMessagesWindow must succeed");
        let previousMessages = self
            .openedChatMessageFlowSnapshot(&chatId)
            .unwrap_or_default();
        preserveLiveMessageStreams(&previousMessages, &mut revealedMessages, None);
        let hasOlder = self
            .chatHistoryManager
            .hasMessagesBefore(chatId.clone(), startTimestamp)
            .expect("ChatHistoryManager.hasMessagesBefore must succeed");
        let hasNewer = self
            .chatHistoryManager
            .hasMessagesAfter(chatId.clone(), endTimestamp)
            .expect("ChatHistoryManager.hasMessagesAfter must succeed");
        AppLogger::trace(
            "ChatFlowTrace",
            &format!(
                "reveal_locator chatId={} target={} pages={}..{} loaded={} hasOlder={} hasNewer={}",
                chatId,
                targetTimestamp,
                windowStartPageIndex,
                windowEndPageIndex,
                chat_flow_trace_summary(&revealedMessages),
                hasOlder,
                hasNewer
            ),
        );
        self.isLoadingDisplayWindow = false;
        self.currentChatWindow.isLoadingDisplayWindow = false;
        self.applyCurrentChatDisplayWindowWithFlags(chatId, revealedMessages, hasOlder, hasNewer);
        self.currentChatMessagesSnapshot()
            .iter()
            .any(|message| message.timestamp == targetTimestamp)
    }

    #[allow(non_snake_case)]
    /// Loads one older indexed batch into the active display window.
    pub fn loadOlderMessagesForCurrentChat(&mut self) -> bool {
        let Some(chatId) = self.currentChatIdFlow.value() else {
            return false;
        };
        let currentMessages = self.currentChatMessagesSnapshot();
        let Some(first) = currentMessages.first() else {
            return false;
        };
        let olderMessages = self.collectOlderDisplayPagesBefore(
            chatId.clone(),
            self.displayWindowQueryLimit(),
            first.timestamp,
        );
        if olderMessages.is_empty() {
            return false;
        }
        let mut messages = olderMessages;
        messages.extend(currentMessages);
        let hasOlder = self
            .chatHistoryManager
            .hasMessagesBefore(chatId.clone(), messages[0].timestamp)
            .expect("ChatHistoryManager.hasMessagesBefore must succeed");
        let hasNewer = self
            .chatHistoryManager
            .hasMessagesAfter(
                chatId.clone(),
                messages
                    .last()
                    .expect("merged display window must contain messages")
                    .timestamp,
            )
            .expect("ChatHistoryManager.hasMessagesAfter must succeed");
        AppLogger::trace(
            "ChatFlowTrace",
            &format!(
                "load_older chatId={} merged={}",
                chatId,
                chat_flow_trace_summary(&messages)
            ),
        );
        self.applyCurrentChatDisplayWindowWithFlags(chatId, messages, hasOlder, hasNewer);
        true
    }

    #[allow(non_snake_case)]
    /// Loads one newer indexed batch into the active display window.
    pub fn loadNewerMessagesForCurrentChat(&mut self) -> bool {
        let Some(chatId) = self.currentChatIdFlow.value() else {
            return false;
        };
        let currentMessages = self.currentChatMessagesSnapshot();
        let Some(last) = currentMessages.last() else {
            return false;
        };
        let newerMessages = self.collectNewerDisplayPagesAfter(
            chatId.clone(),
            self.displayWindowQueryLimit(),
            last.timestamp,
        );
        if newerMessages.is_empty() {
            return false;
        }
        let mut messages = currentMessages;
        messages.extend(newerMessages);
        let hasOlder = self
            .chatHistoryManager
            .hasMessagesBefore(chatId.clone(), messages[0].timestamp)
            .expect("ChatHistoryManager.hasMessagesBefore must succeed");
        let hasNewer = self
            .chatHistoryManager
            .hasMessagesAfter(
                chatId.clone(),
                messages
                    .last()
                    .expect("merged display window must contain messages")
                    .timestamp,
            )
            .expect("ChatHistoryManager.hasMessagesAfter must succeed");
        AppLogger::trace(
            "ChatFlowTrace",
            &format!(
                "load_newer chatId={} merged={}",
                chatId,
                chat_flow_trace_summary(&messages)
            ),
        );
        self.applyCurrentChatDisplayWindowWithFlags(chatId, messages, hasOlder, hasNewer);
        true
    }

    #[allow(non_snake_case)]
    /// Shows the latest messages for the active chat.
    pub fn showLatestMessagesForCurrentChat(&mut self) -> bool {
        !self.loadLatestCurrentChatDisplayWindow().is_empty()
    }

    /// Restores the persisted history selection without requiring a plugin execution binding or AI runtime.
    pub fn initialize(&mut self) -> Result<(), String> {
        if self.isInitialized {
            return Ok(());
        }
        if self.selectionMode == ChatSelectionMode::FOLLOW_GLOBAL {
            let selected = self
                .chatHistoryManager
                .currentChatIdFlow()
                .map_err(|error| error.to_string())?;
            if let Some(chatId) = selected {
                self.openChatHistory(chatId, false)?;
            }
        }
        self.isInitialized = true;
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Loads the newest indexed display window for one active chat.
    pub fn loadChatMessages(&mut self, chatId: String) {
        self.allowAddMessage = false;
        let previousMessages = self
            .openedChatMessageFlowSnapshot(&chatId)
            .unwrap_or_default();
        let currentSummary = chat_flow_trace_summary(&previousMessages);
        let mut messages =
            self.collectNewestDisplayPages(chatId.clone(), self.displayWindowQueryLimit(), None);
        preserveLiveMessageStreams(&previousMessages, &mut messages, None);
        let loadedSummary = chat_flow_trace_summary(&messages);
        let hasOlder = messages
            .first()
            .map(|message| {
                self.chatHistoryManager
                    .hasMessagesBefore(chatId.clone(), message.timestamp)
                    .expect("ChatHistoryManager.hasMessagesBefore must succeed")
            })
            .unwrap_or(false);

        self.currentChatWindow.reset();
        self.currentChatWindow.hasOlderDisplayHistory = hasOlder;
        self.hasOlderDisplayHistory = hasOlder;
        self.hasNewerDisplayHistory = false;
        self.isLoadingDisplayWindow = false;
        AppLogger::trace(
            "ChatFlowTrace",
            &format!(
                "load_chat.start chatId={} current={} loaded={} allowAddMessage=false",
                chatId, currentSummary, loadedSummary
            ),
        );
        self.currentChatIdFlow.set_value(Some(chatId.clone()));
        self.setCurrentChatMessagesInMemory(messages, Some(hasOlder), Some(false));
        self.dispatchChatViewEvent(ChatViewEvent::ViewOpened, &chatId);
        self.emitDisplayWindowState();
        self.allowAddMessage = true;
        AppLogger::trace(
            "ChatFlowTrace",
            &format!("load_chat.done chatId={} hasOlder={}", chatId, hasOlder),
        );
    }

    #[allow(non_snake_case)]
    /// Reloads the active display window for a chat.
    pub fn reloadChatMessagesSmart(&mut self, chatId: String) {
        self.reloadCurrentChatDisplayHistory(chatId);
    }

    /// Refreshes one opened chat flow from the persisted database snapshot.
    pub fn refreshChatMessagesFromPersistence(&mut self, chatId: String) {
        let currentChatId = self.currentChatIdFlow.value();
        if currentChatId.as_ref() == Some(&chatId) {
            self.reloadCurrentChatDisplayHistory(chatId);
            return;
        }
        let flow = self
            .chatMessageFlowsByChatId
            .lock()
            .expect("chat message flow registry mutex must not be poisoned")
            .get(&chatId)
            .cloned();
        let Some(flow) = flow else {
            return;
        };
        let beforeSummary = chat_flow_trace_summary(&flow.value());
        let previousMessages = flow.value();
        let messages =
            self.collectNewestDisplayPages(chatId.clone(), self.displayWindowQueryLimit(), None);
        let mut messages = messages;
        preserveLiveMessageStreams(&previousMessages, &mut messages, None);
        let loadedSummary = chat_flow_trace_summary(&messages);
        AppLogger::trace(
            "ChatFlowTrace",
            &format!(
                "sync_refresh chatId={} before={} loaded={}",
                chatId, beforeSummary, loadedSummary
            ),
        );
        flow.set_value(messages);
    }

    /// Publishes only a successfully validated display descriptor, never an execution identity, to UI state.
    pub fn publishChatConfiguration(&self, chatId: &str, configuration: ChatConfigurationDisplayResult) {
        let mut configurations = self.chatConfigurationsFlow.value();
        configurations.insert(chatId.to_string(), configuration);
        self.chatConfigurationsFlow.set_value(configurations);
    }

    /// Initializes a new draft through awaited plugin hooks before any record or configuration is published.
    pub async fn createNewChat(
        &mut self,
        service: &EnhancedAIService,
        setAsCurrentChat: bool,
        sourceChatId: Option<String>,
        input: Option<Value>,
    ) -> Result<String, String> {
        self.createChatDraft(
            service,
            ToolPkgChatCreationKind::New,
            setAsCurrentChat,
            sourceChatId,
            None,
            input,
        )
        .await
    }

    /// Resolves a complete unpersisted draft, commits all initial rows atomically, and only then opens it.
    async fn createChatDraft(
        &mut self,
        service: &EnhancedAIService,
        creationKind: ToolPkgChatCreationKind,
        setAsCurrentChat: bool,
        sourceChatId: Option<String>,
        sourceMessageTimestamp: Option<i64>,
        input: Option<Value>,
    ) -> Result<String, String> {
        let source = match &sourceChatId {
            Some(id) => Some(
                self.chatHistoryManager
                    .loadChatHistory(id.clone())
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| format!("Source chat does not exist: {id}"))?,
            ),
            None => None,
        };
        if let Some(timestamp) = sourceMessageTimestamp {
            let sourceId = sourceChatId
                .as_ref()
                .ok_or_else(|| "Branch timestamp requires an explicit source chat".to_string())?;
            self.chatHistoryManager
                .loadChatMessageVariant(sourceId, timestamp, 0)
                .map_err(|error| error.to_string())?;
        }
        let isBranch = matches!(creationKind, ToolPkgChatCreationKind::Branch);
        if isBranch && source.is_none() {
            return Err("Branch creation requires a source chat".to_string());
        }
        let workspaceId = source.as_ref().and_then(|chat| chat.workspaceId.clone());
        let parentChatId = if isBranch { sourceChatId.clone() } else { None };
        let title = if isBranch {
            source
                .as_ref()
                .ok_or_else(|| "Branch source is missing".to_string())?
                .title
                .clone()
        } else {
            "New Chat".to_string()
        };
        let mut draft = self
            .chatHistoryManager
            .newChatDraft(title, BTreeMap::new(), workspaceId, parentChatId)
            .map_err(|error| error.to_string())?;
        let api = ChatConfigurationApi::ready(&service.tool_handler).await?;
        let hooks = ToolPkgChatLifecycleHookBridge::snapshot().await?;
        let hookDraft = ToolPkgChatCreationDraft {
            creationKind,
            chat: ToolPkgChatCreationChat {
                id: draft.id.clone(),
                title: draft.title.clone(),
                workspaceId: draft.workspaceId.clone(),
                parentChatId: draft.parentChatId.clone(),
            },
            sourceChatId: sourceChatId.clone(),
            sourceMessageTimestamp,
            input: input
                .map(|value| match value {
                    Value::Object(object) => Ok(object),
                    _ => Err("Chat creation input must be a JSON object".to_string()),
                })
                .transpose()?,
        };
        let sourceExtensions = match &sourceChatId {
            Some(id) => self
                .chatHistoryManager
                .readPluginExtensions(&PluginExtensionTarget::Chat { chatId: id.clone() })
                .map_err(|error| error.to_string())?,
            None => BTreeMap::new(),
        };
        draft.pluginExtensions = ToolPkgChatLifecycleHookBridge::dispatchBeforeCreate(
            &hooks,
            &hookDraft,
            &sourceExtensions,
        )
        .await?;
        let support = service.provider_runtime_context.support();
        let request = ChatConfigurationRequest {
            purpose: ChatConfigurationPurpose::Display,
            chatId: Some(draft.id.clone()),
            chatExtension: None,
            messageExtension: None,
            participantId: None,
            promptFunctionType: PromptFunctionType::CHAT,
            defaultModelBinding: support
                .modelBindingForFunction(support.dataDir()?, FunctionType::CHAT)?,
            defaultTtsConfigId: support.defaultTtsConfigId()?,
        };
        let ownerExtension = draft
            .pluginExtensions
            .get(api.extensionOwner())
            .map(|value| {
                value.as_object().cloned().ok_or_else(|| {
                    "Draft configuration owner extension must be a JSON object".to_string()
                })
            })
            .transpose()?;
        let configuration = api.resolveDraft(request.clone(), ownerExtension).await?;
        if !isBranch && setAsCurrentChat {
            if let Some(currentId) = self.currentChatIdFlow.value() {
                if sourceChatId.as_ref() == Some(&currentId) {
                    let current = self
                        .chatHistoryManager
                        .loadChatHistory(currentId.clone())
                        .map_err(|error| error.to_string())?
                        .ok_or_else(|| format!("Current chat does not exist: {currentId}"))?;
                    if current.pluginExtensions == draft.pluginExtensions
                        && current.workspaceId == draft.workspaceId
                        && !self
                            .chatHistoryManager
                            .hasUserMessage(currentId.clone())
                            .map_err(|error| error.to_string())?
                    {
                        let existingConfiguration = api
                            .resolveDisplay(ChatConfigurationRequest {
                                chatId: Some(currentId.clone()),
                                ..request
                            })
                            .await?;
                        self.publishChatConfiguration(&currentId, existingConfiguration);
                        self.openChatRecord(
                            currentId.clone(),
                            self.selectionMode == ChatSelectionMode::FOLLOW_GLOBAL,
                        )?;
                        return Ok(currentId);
                    }
                }
            }
        }
        if isBranch {
            let source = source
                .as_ref()
                .ok_or_else(|| "Branch source is missing".to_string())?;
            draft.inputTokens = source.inputTokens;
            draft.outputTokens = source.outputTokens;
            draft.currentWindowSize = source.currentWindowSize;
        } else {
            for initial in &configuration.initialMessages {
                let mut opening = ChatMessage::new_with_markdown("ai".to_string(), initial.content.clone());
                opening.roleName = initial.displayName.clone();
                if let Some(snapshot) = &initial.messageExtension {
                    opening.pluginExtensions.insert(configuration.extensionOwner.clone(), Value::Object(snapshot.clone()));
                }
                draft.messages.push(opening);
            }
        }
        draft.group = source.as_ref().and_then(|chat| chat.group.clone());
        let initialMessages = draft.messages.clone();
        let cloneSource = if isBranch {
            Some((
                sourceChatId
                    .as_deref()
                    .ok_or_else(|| "Branch source is missing".to_string())?,
                sourceMessageTimestamp,
            ))
        } else {
            None
        };
        let committed = self
            .chatHistoryManager
            .commitChatDraft(draft, cloneSource)
            .map_err(|error| error.to_string())?;
        self.publishChatConfiguration(&committed.id, configuration);
        for message in initialMessages {
            ToolPkgChatMessageHookBridge::dispatchMessagePersisted(&committed.id, &message);
        }
        if setAsCurrentChat {
            self.openChatRecord(
                committed.id.clone(),
                self.selectionMode == ChatSelectionMode::FOLLOW_GLOBAL,
            )?;
        }
        Ok(committed.id)
    }

    /// Opens a real history and marks its execution descriptor unresolved without inventing a configuration.
    pub fn openChatHistory(&mut self, chatId: String, syncToGlobal: bool) -> Result<(), String> {
        self.requireChatExists(&chatId)?;
        let mut configurations = self.chatConfigurationsFlow.value();
        configurations.remove(&chatId);
        self.chatConfigurationsFlow.set_value(configurations);
        self.openChatRecord(chatId, syncToGlobal)
    }

    /// Requires an actual Core chat record before any configuration or history operation.
    pub fn requireChatExists(&self, chatId: &str) -> Result<(), String> {
        if chatId.trim().is_empty() {
            return Err("Chat id is empty".to_string());
        }
        if !self
            .chatHistoryManager
            .chatExists(chatId.to_string())
            .map_err(|error| error.to_string())?
        {
            return Err(format!("Chat does not exist: {chatId}"));
        }
        Ok(())
    }

    /// Opens canonical persisted records independently of any optional runtime execution descriptor.
    #[allow(non_snake_case)]
    fn openChatRecord(&mut self, chatId: String, syncToGlobal: bool) -> Result<(), String> {
        self.requireChatExists(&chatId)?;
        if syncToGlobal && self.selectionMode == ChatSelectionMode::FOLLOW_GLOBAL {
            self.chatHistoryManager
                .setCurrentChatId(chatId.clone())
                .map_err(|error| error.to_string())?;
        }
        if let Some(previousChatId) = self.currentChatIdFlow.value() {
            if previousChatId != chatId {
                self.dispatchChatViewEvent(ChatViewEvent::ViewClosed, &previousChatId);
            }
        }
        self.loadChatMessages(chatId);
        self.isInitialized = true;
        Ok(())
    }

    /// Initializes a true source branch through the same hook and draft transaction as a new conversation.
    pub async fn createBranch(
        &mut self,
        service: &EnhancedAIService,
        upToMessageTimestamp: Option<i64>,
    ) -> Result<String, String> {
        let sourceChatId = self
            .currentChatIdFlow
            .value()
            .ok_or_else(|| "Branch creation requires a selected chat".to_string())?;
        self.createChatDraft(
            service,
            ToolPkgChatCreationKind::Branch,
            true,
            Some(sourceChatId),
            upToMessageTimestamp,
            None,
        )
        .await
    }

    #[allow(non_snake_case)]
    /// Loads branches whose parent is the supplied chat id.
    pub fn getBranches(&self, parentChatId: String) -> Vec<ChatHistory> {
        self.chatHistoryManager
            .getBranches(parentChatId)
            .expect("ChatHistoryManager.getBranches must succeed")
    }

    #[allow(non_snake_case)]
    /// Updates the locked state for a chat and emits metadata changes.
    pub fn updateChatLocked(&mut self, chatId: String, locked: bool) {
        self.chatHistoryManager
            .updateChatLocked(chatId, locked)
            .expect("ChatHistoryManager.updateChatLocked must succeed");
    }

    #[allow(non_snake_case)]
    /// Updates the pinned state for a chat and emits metadata changes.
    pub fn updateChatPinned(&mut self, chatId: String, pinned: bool) {
        self.chatHistoryManager
            .updateChatPinned(chatId, pinned)
            .expect("ChatHistoryManager.updateChatPinned must succeed");
    }

    #[allow(non_snake_case)]
    /// Deletes a chat after the active selection has moved to a valid chat.
    pub fn deleteChatHistory(&mut self, chatId: String) -> bool {
        let canDelete = self
            .chatHistoryManager
            .canDeleteChatHistory(chatId.clone())
            .expect("ChatHistoryManager.canDeleteChatHistory must succeed");
        if !canDelete {
            return false;
        }
        self.prepareChatForDestructiveMutation(chatId.clone());
        let deleted = self
            .chatHistoryManager
            .deleteChatHistory(chatId.clone())
            .expect("ChatHistoryManager.deleteChatHistory must succeed");
        if deleted && self.currentChatIdFlow.value().as_ref() == Some(&chatId) {
            self.currentChatIdFlow.set_value(None);
            self.clearCurrentChatHistoryInMemory();
        }
        if deleted {
            let mut configurations = self.chatConfigurationsFlow.value();
            configurations.remove(&chatId);
            self.chatConfigurationsFlow.set_value(configurations);
            self.clearChatFlow(&chatId);
            self.finishDestructiveHistoryMutation(chatId);
        }
        deleted
    }

    #[allow(non_snake_case)]
    /// Removes the provisional assistant message for a failed response.
    pub fn discardFailedAssistantMessage(&mut self, chatId: String, timestamp: i64) {
        self.chatHistoryManager
            .deleteMessage(chatId.clone(), timestamp)
            .expect("ChatHistoryManager.deleteMessage must remove failed assistant message");
        self.removeChatMessage(&chatId, timestamp);
        if self.currentChatIdFlow.value().as_ref() == Some(&chatId) {
            self.emitChatHistoryState();
        }
    }

    #[allow(non_snake_case)]
    /// Deletes a message from a chat by timestamp.
    pub fn deleteMessageByTimestamp(&mut self, chatId: String, timestamp: i64) -> bool {
        self.chatHistoryManager
            .deleteMessage(chatId.clone(), timestamp)
            .expect("ChatHistoryManager.deleteMessage must remove message");
        self.removeChatMessage(&chatId, timestamp);
        if self.currentChatIdFlow.value().as_ref() == Some(&chatId) {
            self.emitChatHistoryState();
        }
        true
    }

    #[allow(non_snake_case)]
    /// Deletes a message from an explicit chat by timestamp with mutation hooks.
    pub fn deleteMessageInChatByTimestamp(&mut self, chatId: String, timestamp: i64) -> bool {
        self.runDestructiveHistoryMutation(chatId, |delegate, chatId| {
            delegate.deleteMessageByTimestamp(chatId, timestamp)
        })
    }

    #[allow(non_snake_case)]
    /// Deletes multiple messages from a chat by timestamp.
    pub fn deleteMessagesByTimestamps(&mut self, chatId: String, timestamps: Vec<i64>) {
        for timestamp in timestamps {
            self.deleteMessageByTimestamp(chatId.clone(), timestamp);
        }
    }

    #[allow(non_snake_case)]
    /// Deletes multiple messages from an explicit chat by timestamp with mutation hooks.
    pub fn deleteMessagesInChatByTimestamps(
        &mut self,
        chatId: String,
        timestamps: Vec<i64>,
    ) -> bool {
        if timestamps.is_empty() {
            return false;
        }
        self.runDestructiveHistoryMutation(chatId.clone(), |delegate, chatId| {
            for timestamp in timestamps {
                delegate
                    .chatHistoryManager
                    .deleteMessage(chatId.clone(), timestamp)
                    .expect("ChatHistoryManager.deleteMessage must remove message");
                delegate.removeChatMessage(&chatId, timestamp);
            }
            if delegate.currentChatIdFlow.value().as_ref() == Some(&chatId) {
                delegate.emitChatHistoryState();
            }
            true
        })
    }

    #[allow(non_snake_case)]
    /// Updates the favorite marker on the active chat message with the timestamp.
    pub fn setMessageFavorite(&mut self, timestamp: i64, isFavorite: bool) {
        let Some(chatId) = self.currentChatIdFlow.value() else {
            return;
        };
        self.chatHistoryManager
            .setMessageFavorite(chatId.clone(), timestamp, isFavorite)
            .expect("ChatHistoryManager.setMessageFavorite must update the message");
        self.updateOpenedChatMessage(&chatId, timestamp, |message| {
            message.isFavorite = isFavorite;
        });
        if self.currentChatIdFlow.value().as_deref() == Some(chatId.as_str()) {
            self.emitChatHistoryState();
        }
    }

    #[allow(non_snake_case)]
    /// Deletes one message revision and reloads its persisted display, matching Kotlin.
    pub fn deleteMessageVariant(&mut self, timestamp: i64, variantIndex: i32) {
        let Some(chatId) = self.currentChatIdFlow.value() else {
            return;
        };
        self.chatHistoryManager
            .deleteMessageVariant(chatId.clone(), timestamp, variantIndex)
            .expect("ChatHistoryManager.deleteMessageVariant must remove the requested variant");
        self.reloadCurrentChatDisplayHistoryWithInvalidatedRevision(chatId, Some(timestamp));
    }

    #[allow(non_snake_case)]
    /// Deletes all messages from an explicit chat starting at one timestamp.
    pub fn deleteMessagesFromTimestamp(&mut self, chatId: String, timestamp: i64) -> bool {
        self.runDestructiveHistoryMutation(chatId.clone(), |delegate, chatId| {
            delegate
                .chatHistoryManager
                .deleteMessagesFrom(chatId.clone(), timestamp)
                .expect("ChatHistoryManager.deleteMessagesFrom must remove messages");
            delegate.removeChatMessagesFrom(&chatId, timestamp);
            if delegate.currentChatIdFlow.value().as_ref() == Some(&chatId) {
                delegate.emitChatHistoryState();
            }
            true
        })
    }

    #[allow(non_snake_case)]
    /// Selects the active variant for a message by timestamp.
    pub fn selectMessageVariant(&mut self, timestamp: i64, selectedVariantIndex: i32) {
        let Some(chatId) = self.currentChatIdFlow.value() else {
            return;
        };
        self.chatHistoryManager
            .selectMessageVariant(chatId.clone(), timestamp, selectedVariantIndex)
            .expect("ChatHistoryManager.selectMessageVariant must select the requested variant");
        self.reloadCurrentChatDisplayHistory(chatId);
    }

    #[allow(non_snake_case)]
    /// Adds a variant to a message and refreshes the active display.
    pub fn addMessageVariant(
        &mut self,
        timestamp: i64,
        message: ChatMessage,
        chatIdOverride: Option<String>,
    ) -> i32 {
        let chatId = chatIdOverride
            .or_else(|| self.currentChatIdFlow.value())
            .expect("No active chat");
        let selectedVariantIndex = self
            .chatHistoryManager
            .addMessageVariant(chatId.clone(), timestamp, message)
            .expect("ChatHistoryManager.addMessageVariant must succeed");
        let persistedVariant = self
            .chatHistoryManager
            .loadChatMessageVariant(&chatId, timestamp, selectedVariantIndex)
            .expect("The newly persisted exact variant must be readable");
        ToolPkgChatMessageHookBridge::dispatchMessagePersisted(&chatId, &persistedVariant);
        if self.currentChatIdFlow.value().as_ref() == Some(&chatId) {
            self.reloadCurrentChatDisplayHistory(chatId.clone());
        }
        ChainLogger::verbose(
            MESSAGE_STORE_CHAIN,
            "message.store.variant",
            &[
                ("chatId", chatId.clone()),
                ("timestamp", timestamp.to_string()),
                ("selectedVariantIndex", selectedVariantIndex.to_string()),
            ],
        );
        selectedVariantIndex
    }

    #[allow(non_snake_case)]
    /// Clears messages from the current chat.
    pub fn clearCurrentChat(&mut self) -> bool {
        let Some(chatId) = self.currentChatIdFlow.value() else {
            return false;
        };
        self.prepareChatForDestructiveMutation(chatId.clone());
        self.chatHistoryManager
            .clearChatMessages(chatId.clone())
            .expect("ChatHistoryManager.clearChatMessages must succeed");
        self.clearCurrentChatHistoryInMemory();
        self.finishDestructiveHistoryMutation(chatId);
        true
    }

    #[allow(non_snake_case)]
    /// Persists token metrics for the current or supplied chat.
    pub fn saveCurrentChat(
        &mut self,
        inputTokens: i64,
        outputTokens: i64,
        actualContextWindowSize: i64,
        chatIdOverride: Option<String>,
    ) {
        let chatId = chatIdOverride.or_else(|| self.currentChatIdFlow.value());
        if let Some(chatId) = chatId {
            let shouldSave = !self.currentChatMessagesSnapshot().is_empty()
                || inputTokens != 0
                || outputTokens != 0
                || actualContextWindowSize != 0;
            if shouldSave {
                self.chatHistoryManager
                    .updateChatTokenCounts(
                        chatId.clone(),
                        inputTokens,
                        outputTokens,
                        actualContextWindowSize,
                    )
                    .expect("ChatHistoryManager.updateChatTokenCounts must succeed");
                ChainLogger::verbose(
                    MESSAGE_STORE_CHAIN,
                    "chat.store.metrics",
                    &[
                        ("chatId", chatId.clone()),
                        ("inputTokens", inputTokens.to_string()),
                        ("outputTokens", outputTokens.to_string()),
                        (
                            "actualContextWindowSize",
                            actualContextWindowSize.to_string(),
                        ),
                    ],
                );
            }
        }
    }

    /// Synchronizes plugin view state and menu definitions after a workspace mutation.
    #[allow(non_snake_case)]
    pub fn notifyChatWorkspaceChanged(&self, chatId: &str) {
        if self.currentChatIdFlow.value().as_deref() == Some(chatId) {
            self.dispatchChatViewEvent(ChatViewEvent::ViewUpdated, chatId);
        }
        ToolPkgInputMenuToggleBridge::invalidateToggleDefinitions();
    }

    #[allow(non_snake_case)]
    /// Binds a chat to a workspace.
    pub fn bindChatToWorkspace(&mut self, chatId: String, workspace: String) {
        self.bindChatToFolderPath(chatId, workspace)
            .expect("ChatHistoryDelegate.bindChatToFolderPath must succeed");
    }

    /// Mounts a VFS folder onto the chat's workspace, creating the workspace when needed.
    pub fn bindChatToFolderPath(
        &mut self,
        chatId: String,
        folderPath: String,
    ) -> Result<operit_model::Workspace::Workspace, String> {
        let selection = folderPath.trim();
        let source = if let Some(json) =
            selection.strip_prefix(operit_tools::files::MountRegistry::MOUNT_SOURCE_PREFIX)
        {
            Some(
                serde_json::from_str::<operit_tools::files::MountRegistry::MountSource>(json)
                    .map_err(|e| format!("Invalid workspace mount source: {e}"))?,
            )
        } else if selection.starts_with("content://") {
            // Keep compatibility with callers that submit a raw authorized tree URI.
            Some(operit_tools::files::MountRegistry::MountSource {
                namespace: "/mnt/android/documents".into(),
                backend: "android_documents".into(),
                root: selection.into(),
                name: "Documents".into(),
            })
        } else {
            None
        };
        let (folderPath, mountedName) = match source {
            Some(source) => {
                let host = operit_store::RuntimeStorageHost::defaultRuntimeStorageHost();
                let mount = operit_tools::files::MountRegistry::MountRegistry::withStorage(host).register(
                    &source.namespace, &source.backend, &source.root, &source.name,
                )?;
                (mount.vfsPath(), Some(mount.name))
            }
            None => (folderPath, None),
        };
        let folderPath = PathMapper::normalizeWorkspaceBindingPath(&folderPath)?;
        let folderName = match mountedName {
            Some(name) => name,
            None => operit_model::Workspace::Workspace::folderNameFromPath(&folderPath)?,
        };
        let workspace = match self
            .chatHistoryManager
            .getWorkspaceForChat(&chatId)
            .map_err(|error| error.to_string())?
        {
            Some(existing) => {
                if existing
                    .folders
                    .iter()
                    .any(|folder| folder.path == folderPath)
                {
                    existing
                } else {
                    self.chatHistoryManager
                        .addWorkspaceFolder(
                            existing.id,
                            operit_model::Workspace::WorkspaceFolder {
                                name: folderName,
                                path: folderPath,
                            },
                        )
                        .map_err(|error| error.to_string())?
                }
            }
            None => {
                let created = self
                    .chatHistoryManager
                    .createWorkspace(
                        folderName.clone(),
                        vec![operit_model::Workspace::WorkspaceFolder {
                            name: folderName,
                            path: folderPath,
                        }],
                    )
                    .map_err(|error| error.to_string())?;
                self.chatHistoryManager
                    .updateChatWorkspaceId(chatId.clone(), Some(created.id.clone()))
                    .map_err(|error| error.to_string())?;
                created
            }
        };
        self.notifyChatWorkspaceChanged(&chatId);
        Ok(workspace)
    }

    /// Returns the primary folder path for a chat history row.
    pub fn primaryWorkspacePathForChat(&self, chat: &ChatHistory) -> Option<String> {
        let workspaceId = chat.workspaceId.as_ref()?;
        self.chatHistoryManager
            .getWorkspace(workspaceId)
            .ok()
            .flatten()
            .map(|workspace| workspace.primaryFolder().path.clone())
    }

    /// Returns every mounted folder path for a chat.
    pub fn workspaceFolderPathsForChat(&self, chat: &ChatHistory) -> Vec<String> {
        let Some(workspaceId) = chat.workspaceId.as_ref() else {
            return Vec::new();
        };
        self.chatHistoryManager
            .getWorkspace(workspaceId)
            .ok()
            .flatten()
            .map(|workspace| workspace.folderPaths())
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    /// Removes the workspace binding from a chat.
    pub fn unbindChatFromWorkspace(&mut self, chatId: String) {
        self.chatHistoryManager
            .updateChatWorkspaceId(chatId.clone(), None)
            .expect("ChatHistoryManager.updateChatWorkspaceId must succeed");
        self.notifyChatWorkspaceChanged(&chatId);
    }

    #[allow(non_snake_case)]
    /// Updates a chat title and emits metadata changes.
    pub fn updateChatTitle(&mut self, chatId: String, title: String) {
        self.chatHistoryManager
            .updateChatTitle(chatId.clone(), title.clone())
            .expect("ChatHistoryManager.updateChatTitle must succeed");
        if self.currentChatIdFlow.value().as_ref() == Some(&chatId) {
            self.dispatchChatViewEvent(ChatViewEvent::ViewUpdated, &chatId);
        }
    }

    #[allow(non_snake_case)]
    /// Updates both workspace binding and title for a chat.
    pub fn renameWorkspaceAndChat(
        &mut self,
        chatId: String,
        newWorkspace: String,
        newTitle: String,
    ) {
        self.bindChatToWorkspace(chatId.clone(), newWorkspace);
        self.updateChatTitle(chatId, newTitle);
    }

    #[allow(non_snake_case)]
    /// Inserts or replaces an in-memory message in the current chat.
    pub fn upsertCurrentChatMessageInMemory(&mut self, message: ChatMessage) -> bool {
        let Some(chatId) = self.currentChatIdFlow.value() else {
            AppLogger::trace(
                "ChatFlowTrace",
                &format!(
                    "upsert_current.skipped_no_chat {}",
                    chat_message_trace_summary(&message)
                ),
            );
            return false;
        };
        let messageSummary = chat_message_trace_summary(&message);
        let flow = self.mutableChatMessageFlowForChat(chatId.clone());
        let mut messages = flow.value();
        let didUpdate = if let Some(existingIndex) = messages
            .iter()
            .position(|existing| existing.timestamp == message.timestamp)
        {
            messages[existingIndex] = message;
            true
        } else {
            messages.push(message);
            messages.sort_by_key(|item| item.timestamp);
            false
        };
        AppLogger::trace(
            "ChatFlowTrace",
            &format!(
                "upsert_current chatId={} action={} count={} streamMessages={} {}",
                chatId,
                if didUpdate { "replace" } else { "insert" },
                messages.len(),
                chat_flow_stream_count(&messages),
                messageSummary
            ),
        );
        flow.set_value(messages);
        self.emitChatHistoryState();
        didUpdate
    }

    #[allow(non_snake_case)]
    /// Adds, updates, or persists a message for the current or supplied chat.
    pub fn addMessageToChat(&mut self, message: ChatMessage, chatIdOverride: Option<String>) {
        let Some(targetChatId) = chatIdOverride.or_else(|| self.currentChatIdFlow.value()) else {
            return;
        };
        let messageSender = message.sender.clone();
        let messageTimestamp = message.timestamp;
        let messageChars = ChainLogger::lenField(&message.displayText());
        let isCurrentChat = self.currentChatIdFlow.value().as_ref() == Some(&targetChatId);
        let currentSummary = self
            .openedChatMessageFlowSnapshot(&targetChatId)
            .map(|messages| chat_flow_trace_summary(&messages))
            .unwrap_or_else(|| "none".to_string());
        AppLogger::trace(
            "ChatFlowTrace",
            &format!(
                "add.start chatId={} current={} sender={} timestamp={} chars={} currentChat={}",
                targetChatId,
                currentSummary,
                messageSender,
                messageTimestamp,
                messageChars,
                isCurrentChat
            ),
        );
        if message.isVariantPreview {
            if isCurrentChat {
                self.upsertCurrentChatMessageInMemory(message);
                ChainLogger::verbose(
                    MESSAGE_STORE_CHAIN,
                    "message.store.preview.memory",
                    &[
                        ("chatId", targetChatId.clone()),
                        ("sender", messageSender),
                        ("timestamp", messageTimestamp.to_string()),
                        ("messageChars", messageChars),
                    ],
                );
            }
            return;
        }

        if isCurrentChat && !self.allowAddMessage {
            let persistedMessage = Self::persistentChatMessage(message.clone());
            self.chatHistoryManager
                .updateMessage(targetChatId.clone(), persistedMessage.clone())
                .expect("ChatHistoryManager.updateMessage must succeed");
            ToolPkgChatMessageHookBridge::dispatchMessagePersisted(
                &targetChatId,
                &persistedMessage,
            );
            self.publishChatMessage(&targetChatId, message);
            ChainLogger::verbose(
                MESSAGE_STORE_CHAIN,
                "message.store.hidden.update",
                &[
                    ("chatId", targetChatId.clone()),
                    ("sender", messageSender),
                    ("timestamp", messageTimestamp.to_string()),
                    ("messageChars", messageChars),
                ],
            );
            AppLogger::trace(
                "ChatFlowTrace",
                &format!(
                    "add.hidden_update.done chatId={} current={}",
                    targetChatId,
                    self.openedChatMessageFlowSnapshot(&targetChatId)
                        .map(|messages| chat_flow_trace_summary(&messages))
                        .unwrap_or_else(|| "none".to_string())
                ),
            );
            return;
        }

        if !isCurrentChat {
            let persistedMessage = Self::persistentChatMessage(message.clone());
            self.chatHistoryManager
                .updateMessage(targetChatId.clone(), persistedMessage.clone())
                .expect("ChatHistoryManager.updateMessage must succeed");
            ToolPkgChatMessageHookBridge::dispatchMessagePersisted(
                &targetChatId,
                &persistedMessage,
            );
            self.publishChatMessage(&targetChatId, message);
            ChainLogger::verbose(
                MESSAGE_STORE_CHAIN,
                "message.store.background.update",
                &[
                    ("chatId", targetChatId.clone()),
                    ("sender", messageSender),
                    ("timestamp", messageTimestamp.to_string()),
                    ("messageChars", messageChars),
                ],
            );
            AppLogger::trace(
                "ChatFlowTrace",
                &format!(
                    "add.background.done chatId={} current={}",
                    targetChatId,
                    self.openedChatMessageFlowSnapshot(&targetChatId)
                        .map(|messages| chat_flow_trace_summary(&messages))
                        .unwrap_or_else(|| "none".to_string())
                ),
            );
            return;
        }

        let didUpdateVisibleMessage = self
            .currentChatMessagesSnapshot()
            .iter()
            .any(|existing| existing.timestamp == message.timestamp);
        let isVisibleNewMessage =
            !didUpdateVisibleMessage && !self.currentChatWindow.hasNewerDisplayHistory;

        if didUpdateVisibleMessage {
            let persistedMessage = Self::persistentChatMessage(message.clone());
            self.chatHistoryManager
                .updateMessage(targetChatId.clone(), persistedMessage.clone())
                .expect("ChatHistoryManager.updateMessage must succeed");
            ToolPkgChatMessageHookBridge::dispatchMessagePersisted(
                &targetChatId,
                &persistedMessage,
            );
            ChainLogger::verbose(
                MESSAGE_STORE_CHAIN,
                "message.store.visible.update",
                &[
                    ("chatId", targetChatId.clone()),
                    ("sender", messageSender),
                    ("timestamp", messageTimestamp.to_string()),
                    ("messageChars", messageChars),
                ],
            );
            AppLogger::trace(
                "ChatFlowTrace",
                &format!(
                    "add.visible_update chatId={} current={}",
                    targetChatId,
                    self.openedChatMessageFlowSnapshot(&targetChatId)
                        .map(|messages| chat_flow_trace_summary(&messages))
                        .unwrap_or_else(|| "none".to_string())
                ),
            );
        } else if isVisibleNewMessage {
            let persistedMessage = Self::persistentChatMessage(message.clone());
            self.chatHistoryManager
                .addMessage(targetChatId.clone(), persistedMessage.clone())
                .expect("ChatHistoryManager.addMessage must succeed");
            ToolPkgChatMessageHookBridge::dispatchMessagePersisted(
                &targetChatId,
                &persistedMessage,
            );
            ChainLogger::verbose(
                MESSAGE_STORE_CHAIN,
                "message.store.visible.insert",
                &[
                    ("chatId", targetChatId.clone()),
                    ("sender", messageSender),
                    ("timestamp", messageTimestamp.to_string()),
                    ("messageChars", messageChars),
                ],
            );
            AppLogger::trace(
                "ChatFlowTrace",
                &format!(
                    "add.visible_insert chatId={} current={}",
                    targetChatId,
                    self.openedChatMessageFlowSnapshot(&targetChatId)
                        .map(|messages| chat_flow_trace_summary(&messages))
                        .unwrap_or_else(|| "none".to_string())
                ),
            );
        } else {
            let persistedMessage = Self::persistentChatMessage(message.clone());
            self.chatHistoryManager
                .updateMessage(targetChatId.clone(), persistedMessage.clone())
                .expect("ChatHistoryManager.updateMessage must succeed");
            ToolPkgChatMessageHookBridge::dispatchMessagePersisted(
                &targetChatId,
                &persistedMessage,
            );
            ChainLogger::verbose(
                MESSAGE_STORE_CHAIN,
                "message.store.window.update",
                &[
                    ("chatId", targetChatId.clone()),
                    ("sender", messageSender),
                    ("timestamp", messageTimestamp.to_string()),
                    ("messageChars", messageChars),
                ],
            );
            AppLogger::trace(
                "ChatFlowTrace",
                &format!(
                    "add.window_update chatId={} current={}",
                    targetChatId,
                    self.openedChatMessageFlowSnapshot(&targetChatId)
                        .map(|messages| chat_flow_trace_summary(&messages))
                        .unwrap_or_else(|| "none".to_string())
                ),
            );
        }
        self.upsertCurrentChatMessageInMemory(message);
        if isVisibleNewMessage {
            self.emitDisplayWindowState();
        }
    }

    /// Persists one original user row, propagates storage errors, and returns its exact canonical committed identity.
    pub(crate) fn commitUserMessage(&mut self, chatId: &str, message: ChatMessage) -> Result<ChatMessage, String> {
        self.requireChatExists(chatId)?;
        if message.sender != "user" || message.isVariantPreview || message.selectedVariantIndex != 0 {
            return Err("User submission must create an ordinary base user-message record".to_string());
        }
        let persisted = Self::persistentChatMessage(message);
        self.chatHistoryManager.updateMessage(chatId.to_string(), persisted.clone()).map_err(|error| error.to_string())?;
        let committed = self.chatHistoryManager.loadChatMessageVariant(chatId, persisted.timestamp, 0).map_err(|error| error.to_string())?;
        ToolPkgChatMessageHookBridge::dispatchMessagePersisted(chatId, &committed);
        self.publishChatMessage(chatId, committed.clone());
        Ok(committed)
    }

    /// Commits one assistant segment atomically and publishes its in-memory state.
    #[allow(non_snake_case)]
    pub fn commitAssistantMessageSegment(
        &mut self,
        chatId: String,
        message: ChatMessage,
        chatMetrics: Option<(i64, i64, i64)>,
    ) -> Result<SyncClock, String> {
        AppLogger::trace(
            "ChatFlowTrace",
            &format!(
                "commit_assistant.start chatId={} metricsSet={} {}",
                chatId,
                chatMetrics.is_some(),
                chat_message_trace_summary(&message)
            ),
        );
        let clock = self
            .chatHistoryManager
            .commitAssistantMessageSegment(chatId.clone(), message.clone(), chatMetrics)
            .map_err(|error| error.to_string())?;
        let persistedMessage = self
            .chatHistoryManager
            .loadChatMessageVariant(&chatId, message.timestamp, 0)
            .map_err(|error| error.to_string())?;
        ToolPkgChatMessageHookBridge::dispatchMessagePersisted(&chatId, &persistedMessage);
        if self.currentChatIdFlow.value().as_ref() == Some(&chatId) {
            self.upsertCurrentChatMessageInMemory(persistedMessage.clone());
        } else {
            self.publishChatMessage(&chatId, persistedMessage.clone());
        }
        AppLogger::trace(
            "ChatFlowTrace",
            &format!(
                "commit_assistant.done chatId={} clockEntries={} current={}",
                chatId,
                clock.sequences.len(),
                self.openedChatMessageFlowSnapshot(&chatId)
                    .map(|messages| chat_flow_trace_summary(&messages))
                    .unwrap_or_else(|| "none".to_string())
            ),
        );
        AppLogger::trace(
            "ChatFlowTrace",
            &format!(
                "commit_assistant.persisted chatId={} {}",
                chatId,
                chat_message_trace_summary(&message)
            ),
        );
        Ok(clock)
    }

    #[allow(non_snake_case)]
    /// Async-compatible wrapper for adding a message to a chat.
    pub fn addMessageToChatAsync(&mut self, message: ChatMessage, chatIdOverride: Option<String>) {
        self.addMessageToChat(message, chatIdOverride);
    }

    #[allow(non_snake_case)]
    /// Removes messages from the active chat starting at an optional timestamp.
    pub fn truncateChatHistory(&mut self, timestampOfFirstDeletedMessage: Option<i64>) {
        let Some(chatId) = self.currentChatIdFlow.value() else {
            return;
        };
        match timestampOfFirstDeletedMessage {
            Some(timestamp) => {
                self.chatHistoryManager
                    .deleteMessagesFrom(chatId.clone(), timestamp)
                    .expect("ChatHistoryManager.deleteMessagesFrom must succeed");
                self.removeChatMessagesFrom(&chatId, timestamp);
            }
            None => {
                self.chatHistoryManager
                    .clearChatMessages(chatId.clone())
                    .expect("ChatHistoryManager.clearChatMessages must succeed");
                self.clearChatFlow(&chatId);
            }
        }
        self.emitChatHistoryState();
    }

    #[allow(non_snake_case)]
    /// Removes messages from an explicit chat starting at an optional timestamp.
    pub fn truncateChatHistoryForChat(
        &mut self,
        chatId: String,
        timestampOfFirstDeletedMessage: Option<i64>,
    ) -> bool {
        self.runDestructiveHistoryMutation(chatId.clone(), |delegate, chatId| {
            match timestampOfFirstDeletedMessage {
                Some(timestamp) => {
                    delegate
                        .chatHistoryManager
                        .deleteMessagesFrom(chatId.clone(), timestamp)
                        .expect("ChatHistoryManager.deleteMessagesFrom must succeed");
                    delegate.removeChatMessagesFrom(&chatId, timestamp);
                }
                None => {
                    delegate
                        .chatHistoryManager
                        .clearChatMessages(chatId.clone())
                        .expect("ChatHistoryManager.clearChatMessages must succeed");
                    delegate.clearChatFlow(&chatId);
                }
            }
            if delegate.currentChatIdFlow.value().as_ref() == Some(&chatId) {
                delegate.emitChatHistoryState();
            }
            true
        })
    }

    /// Restores native conversation folders independently of role-card plugins.
    pub fn updateChatGroups(&mut self, chatIds: Vec<String>, groupName: Option<String>) -> Result<(), String> {
        self.chatHistoryManager.updateChatGroups(chatIds, groupName).map_err(|error| error.to_string())
    }

    /// Applies a neutral order to existing chats without interpreting plugin sidebar groups.
    pub fn updateChatOrder(
        &mut self,
        reorderedHistories: Vec<ChatHistoryListItem>,
        movedItem: ChatHistoryListItem,
    ) -> Result<(), String> {
        let chatIds = reorderedHistories
            .into_iter()
            .map(|item| item.id)
            .collect::<Vec<_>>();
        if !chatIds.iter().any(|id| id == &movedItem.id) {
            return Err("Moved chat is absent from the reordered canonical chat IDs".to_string());
        }
        self.chatHistoryManager
            .updateChatOrder(chatIds)
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    /// Inserts a persisted summary message between neighboring message timestamps.
    pub fn addSummaryMessage(
        &mut self,
        summaryMessage: ChatMessage,
        beforeTimestamp: Option<i64>,
        afterTimestamp: Option<i64>,
        chatIdOverride: Option<String>,
    ) {
        let Some(chatId) = chatIdOverride.or_else(|| self.currentChatIdFlow.value()) else {
            return;
        };
        let isCurrentChat = self.currentChatIdFlow.value().as_ref() == Some(&chatId);
        let persistedSummaryMessage = self
            .chatHistoryManager
            .addSummaryMessageBetweenSliceNeighbors(
                chatId.clone(),
                summaryMessage,
                beforeTimestamp,
                afterTimestamp,
            )
            .expect("ChatHistoryManager.addSummaryMessageBetweenSliceNeighbors must succeed");
        let Some(_) = persistedSummaryMessage else {
            return;
        };
        if isCurrentChat {
            self.reloadCurrentChatDisplayHistory(chatId);
        }
    }

    #[allow(non_snake_case)]
    /// Returns whether the current token pressure requires summarization.
    pub fn shouldGenerateSummary(
        &self,
        messages: Vec<ChatMessage>,
        currentTokens: i64,
        maxTokens: i32,
    ) -> bool {
        !messages.is_empty() && currentTokens >= i64::from(maxTokens)
    }

    #[allow(non_snake_case)]
    /// Placeholder hook for summarizing chat memory.
    pub fn summarizeMemory(&self, _messages: Vec<ChatMessage>) {}

    #[allow(non_snake_case)]
    /// Finds the insertion position after the latest AI message.
    pub fn findProperSummaryPosition(&self, messages: Vec<ChatMessage>) -> usize {
        messages
            .iter()
            .rposition(|message| message.sender == "ai")
            .map(|index| index + 1)
            .unwrap_or(0)
    }

    #[allow(non_snake_case)]
    /// Toggles the chat history selector visibility.
    pub fn toggleChatHistorySelector(&mut self) {
        self.showChatHistorySelector = !self.showChatHistorySelector;
    }

    #[allow(non_snake_case)]
    /// Sets the chat history selector visibility.
    pub fn showChatHistorySelector(&mut self, show: bool) {
        self.showChatHistorySelector = show;
    }

    #[allow(non_snake_case)]
    /// Returns memory entries associated with the active chat context.
    pub fn getMemory(&self, _includePlanInfo: bool) -> Vec<(String, String)> {
        Vec::new()
    }

    #[allow(non_snake_case)]
    /// Returns the enhanced AI service identifier associated with this delegate.
    pub fn getEnhancedAiService(&self) -> Option<String> {
        None
    }

    #[allow(non_snake_case)]
    /// Returns the current chat's input and output token counts.
    pub fn getCurrentTokenCounts(&self) -> (i64, i64) {
        let Some(chatId) = self.currentChatIdFlow.value() else {
            return (0, 0);
        };
        self.chatHistoriesFlow
            .value()
            .iter()
            .find(|chat| chat.id == chatId)
            .map(|chat| (chat.inputTokens, chat.outputTokens))
            .unwrap_or((0, 0))
    }
}

impl Default for ChatHistoryDelegate {
    fn default() -> Self {
        Self::new(ChatSelectionMode::FOLLOW_GLOBAL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_link::{CoreEventStream, CoreStream, CoreStreamSource};
    use operit_model::MessagePart::MessagePart;
    use operit_util::MarkdownRenderStream::MarkdownStreamEvent;
    use std::sync::Arc;

    /// Verifies that a persistence refresh keeps the active stream projection intact.
    #[test]
    fn persistence_refresh_preserves_live_message_stream() {
        let source = Arc::new(CoreStreamSource::new(|_request| {
            let (_sender, receiver) = CoreEventStream::channel();
            Ok(receiver)
        }));
        let mut previous = ChatMessage::new_with_markdown_timestamp(
            "ai".to_string(),
            "live content".to_string(),
            123,
        );
        previous.contentStream = Some(CoreStream::<MarkdownStreamEvent>::fromSourceWithId(
            "live-stream".to_string(),
            source,
        ));
        let mut loaded =
            ChatMessage::new_with_markdown_timestamp("ai".to_string(), String::new(), 123);
        loaded.parts = vec![MessagePart::markdown(
            "part-0".to_string(),
            0,
            String::new(),
        )];

        preserveLiveMessageStreams(&[previous.clone()], std::slice::from_mut(&mut loaded), None);

        assert_eq!(loaded.contentStream, previous.contentStream);
        assert_eq!(loaded.parts, previous.parts);
    }

    /// Keeps the reloaded survivor's parts even when its index matches the deleted live revision.
    #[test]
    fn revision_deletion_does_not_preserve_obsolete_live_parts() {
        let source = Arc::new(CoreStreamSource::new(|_request| {
            let (_sender, receiver) = CoreEventStream::channel();
            Ok(receiver)
        }));
        let mut deleted = ChatMessage::new_with_markdown_timestamp(
            "ai".to_string(),
            "deleted live revision".to_string(),
            123,
        );
        deleted.selectedVariantIndex = 1;
        deleted.contentStream = Some(CoreStream::<MarkdownStreamEvent>::fromSourceWithId(
            "deleted-stream".to_string(),
            source.clone(),
        ));
        let mut other = ChatMessage::new_with_markdown_timestamp(
            "ai".to_string(),
            "unrelated live output".to_string(),
            456,
        );
        other.contentStream = Some(CoreStream::<MarkdownStreamEvent>::fromSourceWithId(
            "other-stream".to_string(),
            source,
        ));
        let mut survivor = ChatMessage::new_with_markdown_timestamp(
            "ai".to_string(),
            "surviving revision".to_string(),
            123,
        );
        survivor.selectedVariantIndex = 1;
        let mut loaded = vec![
            survivor.clone(),
            ChatMessage::new_with_markdown_timestamp("ai".to_string(), String::new(), 456),
        ];

        preserveLiveMessageStreams(&[deleted, other.clone()], &mut loaded, Some(123));

        assert_eq!(loaded[0].parts, survivor.parts);
        assert!(loaded[0].contentStream.is_none());
        assert_eq!(loaded[1].parts, other.parts);
        assert_eq!(loaded[1].contentStream, other.contentStream);
    }
}
