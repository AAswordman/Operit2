use crate::data::preferences::ActivePromptManager::ActivePromptManager;
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::CharacterGroupCardManager::CharacterGroupCardManager;
use crate::plugins::toolpkg::ToolPkgChatMessageHookBridge::ToolPkgChatMessageHookBridge;
use crate::plugins::toolpkg::ToolPkgChatViewHookBridge::{
    ChatViewEvent, ChatViewHookParams, ToolPkgChatViewHookBridge,
};
use operit_model::ActivePrompt::ActivePrompt;
use operit_model::ChatDisplayWindowState::ChatDisplayWindowState;
use operit_model::ChatHistory::ChatHistory;
use operit_model::ChatHistoryListItem::ChatHistoryListItem;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatMessageLocatorPreview::ChatMessageLocatorPreview;
use operit_store::repository::ChatHistoryManager::ChatHistoryManager;
use operit_store::PreferencesDataStore::{mutableStateFlow, MutableStateFlow, StateFlow};
use operit_store::SyncOperationStore::SyncClock;
use operit_util::AppLogger::AppLogger;
use operit_util::ChainLogger::{self, MESSAGE_STORE_CHAIN};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Number of persisted messages loaded per display-window query.
pub const DISPLAY_WINDOW_QUERY_BATCH_SIZE: usize = 80;

#[derive(Clone, Debug, PartialEq)]
/// Defines whether chat selection follows global persisted state or stays local.
pub enum ChatSelectionMode {
    FOLLOW_GLOBAL,
    LOCAL_ONLY,
}

#[derive(Clone, Debug, PartialEq)]
/// Target chat selected after deleting the current chat.
pub struct ChatDeletionReplacementTarget {
    pub characterCardName: Option<String>,
    pub characterCardId: Option<String>,
    pub characterGroupId: Option<String>,
    pub includeUnboundChats: bool,
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

/// Coordinates chat history persistence, current-chat state, and display-window updates.
pub struct ChatHistoryDelegate {
    pub chatHistoryManager: ChatHistoryManager,
    pub characterCardManager: CharacterCardManager,
    pub activePromptManager: ActivePromptManager,
    pub characterGroupCardManager: CharacterGroupCardManager,
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
            characterCardManager: CharacterCardManager::getInstance(),
            activePromptManager: ActivePromptManager::getInstance(),
            characterGroupCardManager: CharacterGroupCardManager::getInstance(),
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
            characterCardManager: CharacterCardManager::getInstance(),
            activePromptManager: ActivePromptManager::getInstance(),
            characterGroupCardManager: CharacterGroupCardManager::getInstance(),
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
        AppLogger::i(
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
            AppLogger::i(
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
        AppLogger::i(
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
        AppLogger::i(
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
        AppLogger::i(
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
            AppLogger::i(
                "ChatFlowTrace",
                &format!(
                    "clear chatId={} before={}",
                    chatId,
                    chat_flow_trace_summary(&beforeMessages)
                ),
            );
            flow.set_value(Vec::new());
            AppLogger::i(
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
    fn buildChatViewHookParams(&self, chatId: &str) -> ChatViewHookParams {
        let histories = self.chatHistoriesFlow.value();
        let (workspacePath, title) = match histories.iter().find(|chat| chat.id == chatId) {
            Some(chat) => (chat.workspace.clone(), Some(chat.title.clone())),
            None => (None, None),
        };
        ChatViewHookParams {
            viewId: format!("chat:{chatId}"),
            chatId: chatId.to_string(),
            workspacePath,
            workspaceEnv: serde_json::json!({}),
            runtime: "rust".to_string(),
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
            AppLogger::i(
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
        AppLogger::i("ChatFlowTrace", "clear_current.done");
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
            AppLogger::i(
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
            AppLogger::i(
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
        let currentSummary = self
            .openedChatMessageFlowSnapshot(&chatId)
            .map(|messages| chat_flow_trace_summary(&messages))
            .unwrap_or_else(|| "none".to_string());
        let messages =
            self.collectNewestDisplayPages(chatId.clone(), self.displayWindowQueryLimit(), None);
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
        AppLogger::i(
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
    /// Reloads the newest indexed display window for the supplied chat id.
    pub fn reloadCurrentChatDisplayHistory(&mut self, chatId: String) -> Vec<ChatMessage> {
        let currentSummary = self
            .openedChatMessageFlowSnapshot(&chatId)
            .map(|messages| chat_flow_trace_summary(&messages))
            .unwrap_or_else(|| "none".to_string());
        let messages =
            self.collectNewestDisplayPages(chatId.clone(), self.displayWindowQueryLimit(), None);
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
        AppLogger::i(
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
    /// Returns whether the target timestamp is already visible in the active chat.
    pub fn revealMessageForCurrentChat(&mut self, targetTimestamp: i64) -> bool {
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
        AppLogger::i(
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
        AppLogger::i(
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

    /// Initializes flows and active-chat state from persisted chat storage.
    pub fn initialize(&mut self) {
        if let Some(chatId) = self
            .chatHistoryManager
            .currentChatIdFlow()
            .expect("ChatHistoryManager.currentChatIdFlow must succeed")
        {
            let exists = self
                .chatHistoryManager
                .chatExists(chatId.clone())
                .expect("ChatHistoryManager.chatExists must succeed");
            if exists {
                self.currentChatIdFlow.set_value(Some(chatId.clone()));
                self.loadChatMessages(chatId);
            } else {
                if self.selectionMode == ChatSelectionMode::FOLLOW_GLOBAL {
                    self.chatHistoryManager
                        .clearCurrentChatId()
                        .expect("ChatHistoryManager.clearCurrentChatId must succeed");
                }
                self.clearCurrentChatHistoryInMemory();
                self.currentChatIdFlow.set_value(None);
            }
        }
        self.isInitialized = true;
    }

    #[allow(non_snake_case)]
    /// Loads the newest indexed display window for one active chat.
    pub fn loadChatMessages(&mut self, chatId: String) {
        self.allowAddMessage = false;
        let currentSummary = self
            .openedChatMessageFlowSnapshot(&chatId)
            .map(|messages| chat_flow_trace_summary(&messages))
            .unwrap_or_else(|| "none".to_string());
        let messages =
            self.collectNewestDisplayPages(chatId.clone(), self.displayWindowQueryLimit(), None);
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
        AppLogger::i(
            "ChatFlowTrace",
            &format!(
                "load_chat.start chatId={} current={} loaded={} allowAddMessage=false",
                chatId, currentSummary, loadedSummary
            ),
        );
        self.activatePromptForChat(chatId.clone());
        self.currentChatIdFlow.set_value(Some(chatId.clone()));
        self.setCurrentChatMessagesInMemory(messages, Some(hasOlder), Some(false));
        self.dispatchChatViewEvent(ChatViewEvent::ViewOpened, &chatId);
        self.emitDisplayWindowState();
        self.allowAddMessage = true;
        AppLogger::i(
            "ChatFlowTrace",
            &format!("load_chat.done chatId={} hasOlder={}", chatId, hasOlder),
        );
    }

    #[allow(non_snake_case)]
    /// Reloads the active display window for a chat.
    pub fn reloadChatMessagesSmart(&mut self, chatId: String) {
        self.reloadCurrentChatDisplayHistory(chatId);
    }

    #[allow(non_snake_case)]
    fn activatePromptForChat(&self, chatId: String) {
        let histories = self.chatHistoriesFlow.value();
        if let Some(chat) = histories.iter().find(|chat| chat.id == chatId) {
            self.activePromptManager
                .activateForChatBinding(
                    chat.characterCardName.clone(),
                    chat.characterGroupId.clone(),
                )
                .expect("ActivePromptManager.activateForChatBinding must succeed");
        }
    }

    #[allow(non_snake_case)]
    /// Switches active state to the latest chat for a character card or creates one.
    pub fn switchActiveCharacterCardTarget(&mut self, characterCardId: String) {
        let targetCard = self
            .characterCardManager
            .getCharacterCard(&characterCardId)
            .expect("CharacterCardManager.getCharacterCard must succeed");
        self.activePromptManager
            .setActivePrompt(ActivePrompt::CharacterCard {
                id: targetCard.id.clone(),
            })
            .expect("ActivePromptManager.setActivePrompt must succeed");
        if let Some(chatId) =
            self.findLatestChatForCharacterCard(targetCard.name.clone(), targetCard.isDefault)
        {
            self.switchChat(chatId, true);
        } else {
            self.createNewChat(None, None, None, true, true, Some(targetCard.id));
        }
    }

    #[allow(non_snake_case)]
    /// Switches active state to the latest chat for a character group or creates one.
    pub fn switchActiveCharacterGroupTarget(&mut self, characterGroupId: String) {
        let targetGroup = self
            .characterGroupCardManager
            .getCharacterGroupCard(&characterGroupId)
            .expect("CharacterGroupCardManager.getCharacterGroupCard must succeed")
            .expect("Character group card must exist");
        self.activePromptManager
            .setActivePrompt(ActivePrompt::CharacterGroup {
                id: targetGroup.id.clone(),
            })
            .expect("ActivePromptManager.setActivePrompt must succeed");
        if let Some(chatId) = self.findLatestChatForCharacterGroup(targetGroup.id.clone()) {
            self.switchChat(chatId, true);
        } else {
            self.createNewChat(None, Some(targetGroup.id.clone()), None, true, true, None);
        }
    }

    #[allow(non_snake_case)]
    fn findLatestChatForCharacterCard(
        &self,
        targetCardName: String,
        targetCardIsDefault: bool,
    ) -> Option<String> {
        self.chatHistoriesFlow
            .value()
            .iter()
            .filter(|history| {
                history
                    .characterGroupId
                    .as_ref()
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false)
                    == false
            })
            .filter(|history| {
                let historyCardName = history
                    .characterCardName
                    .as_ref()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty());
                if targetCardIsDefault {
                    historyCardName
                        .as_ref()
                        .map(|value| value == &targetCardName)
                        .unwrap_or(true)
                } else {
                    historyCardName
                        .as_ref()
                        .map(|value| value == &targetCardName)
                        .unwrap_or(false)
                }
            })
            .max_by_key(|history| {
                history
                    .updatedAt
                    .parse::<i64>()
                    .expect("ChatHistory.updatedAt must be an epoch millis string")
            })
            .map(|history| history.id.clone())
    }

    #[allow(non_snake_case)]
    fn findLatestChatForCharacterGroup(&self, targetGroupId: String) -> Option<String> {
        self.chatHistoriesFlow
            .value()
            .iter()
            .filter(|history| {
                history
                    .characterGroupId
                    .as_ref()
                    .map(|value| value.trim() == targetGroupId)
                    .unwrap_or(false)
            })
            .max_by_key(|history| {
                history
                    .updatedAt
                    .parse::<i64>()
                    .expect("ChatHistory.updatedAt must be an epoch millis string")
            })
            .map(|history| history.id.clone())
    }

    #[allow(non_snake_case)]
    /// Synchronizes an opening statement when the chat has no user message.
    pub fn syncOpeningStatementIfNoUserMessage(&mut self, _chatId: String) {}

    #[allow(non_snake_case)]
    /// Returns whether the current state requires creating a new chat.
    pub fn checkIfShouldCreateNewChat(&self) -> bool {
        self.currentChatIdFlow.value().is_none()
    }

    #[allow(non_snake_case)]
    /// Creates a new chat and optionally makes it the active chat.
    pub fn createNewChat(
        &mut self,
        characterCardName: Option<String>,
        characterGroupId: Option<String>,
        group: Option<String>,
        inheritGroupFromCurrent: bool,
        setAsCurrentChat: bool,
        characterCardId: Option<String>,
    ) {
        let inheritGroupFromChatId = if inheritGroupFromCurrent {
            self.currentChatIdFlow.value()
        } else {
            None
        };
        let effectiveGroup = match group {
            Some(value) => Some(value),
            None => inheritGroupFromChatId.and_then(|chatId| {
                self.chatHistoriesFlow
                    .value()
                    .iter()
                    .find(|chat| chat.id == chatId)
                    .and_then(|chat| chat.group.clone())
            }),
        };
        let normalizedCharacterGroupId =
            characterGroupId.and_then(|value| normalizedNonBlank(value));
        let activeCard = match self.activePromptManager.getActivePrompt() {
            Ok(ActivePrompt::CharacterCard { id }) => {
                self.characterCardManager.getCharacterCard(&id).ok()
            }
            Ok(ActivePrompt::CharacterGroup { .. }) | Err(_) => None,
        };
        let resolvedCard = if normalizedCharacterGroupId.is_none() {
            characterCardId
                .and_then(normalizedNonBlank)
                .and_then(|id| self.characterCardManager.getCharacterCard(&id).ok())
                .or(activeCard)
        } else {
            None
        };
        let explicitCharacterCardName = characterCardName.clone();
        let effectiveCharacterCardName = if normalizedCharacterGroupId.is_none() {
            characterCardName.or_else(|| resolvedCard.as_ref().map(|card| card.name.clone()))
        } else {
            None
        };
        let newChat = self
            .chatHistoryManager
            .createNewChat(
                None,
                effectiveGroup,
                effectiveCharacterCardName,
                normalizedCharacterGroupId.clone(),
            )
            .expect("ChatHistoryManager.createNewChat must succeed");
        if normalizedCharacterGroupId.is_none()
            && explicitCharacterCardName.is_none()
            && resolvedCard
                .as_ref()
                .map(|card| !card.openingStatement.is_empty())
                .unwrap_or(false)
        {
            if let Some(card) = resolvedCard {
                let mut openingMessage =
                    ChatMessage::new_with_markdown("ai".to_string(), card.openingStatement);
                openingMessage.roleName = card.name;
                let persistedOpeningMessage = openingMessage.clone();
                self.chatHistoryManager
                    .addMessage(newChat.id.clone(), openingMessage)
                    .expect("ChatHistoryManager.addMessage must succeed");
                ToolPkgChatMessageHookBridge::dispatchMessagePersisted(
                    &newChat.id,
                    &persistedOpeningMessage,
                );
            }
        }
        if setAsCurrentChat {
            if self.selectionMode == ChatSelectionMode::FOLLOW_GLOBAL {
                self.chatHistoryManager
                    .setCurrentChatId(newChat.id.clone())
                    .expect("ChatHistoryManager.setCurrentChatId must succeed");
            }
            self.loadChatMessages(newChat.id);
        }
    }

    #[allow(non_snake_case)]
    /// Switches the selected chat and refreshes in-memory message state.
    pub fn switchChat(&mut self, chatId: String, syncToGlobal: bool) {
        let previousChatId = self.currentChatIdFlow.value();
        AppLogger::i(
            "ChatFlowTrace",
            &format!(
                "switch.start from={} to={} syncToGlobal={}",
                previousChatId.as_deref().unwrap_or("none"),
                chatId,
                syncToGlobal
            ),
        );
        let exists = self
            .chatHistoryManager
            .chatExists(chatId.clone())
            .expect("ChatHistoryManager.chatExists must succeed");
        if !exists {
            if let Some(previousChatId) = self.currentChatIdFlow.value() {
                self.dispatchChatViewEvent(ChatViewEvent::ViewClosed, &previousChatId);
            }
            if syncToGlobal && self.selectionMode == ChatSelectionMode::FOLLOW_GLOBAL {
                self.chatHistoryManager
                    .clearCurrentChatId()
                    .expect("ChatHistoryManager.clearCurrentChatId must succeed");
            }
            self.clearCurrentChatHistoryInMemory();
            self.currentChatIdFlow.set_value(None);
            AppLogger::i(
                "ChatFlowTrace",
                &format!("switch.done missing chatId={}", chatId),
            );
            return;
        }
        if syncToGlobal {
            self.chatHistoryManager
                .setCurrentChatId(chatId.clone())
                .expect("ChatHistoryManager.setCurrentChatId must succeed");
        }
        self.allowAddMessage = false;
        self.loadChatMessages(chatId);
        self.allowAddMessage = true;
        AppLogger::i("ChatFlowTrace", "switch.done");
    }

    #[allow(non_snake_case)]
    /// Creates a branch from the active chat up to an optional message timestamp.
    pub fn createBranch(&mut self, upToMessageTimestamp: Option<i64>) {
        let Some(currentChatId) = self.currentChatIdFlow.value() else {
            return;
        };
        let (inputTokens, outputTokens, windowSize) = self
            .chatHistoriesFlow
            .value()
            .iter()
            .find(|chat| chat.id == currentChatId)
            .map(|chat| (chat.inputTokens, chat.outputTokens, chat.currentWindowSize))
            .unwrap_or((0, 0, 0));
        self.saveCurrentChat(
            inputTokens,
            outputTokens,
            windowSize,
            Some(currentChatId.clone()),
        );
        let branchChat = self
            .chatHistoryManager
            .createBranch(currentChatId, upToMessageTimestamp)
            .expect("ChatHistoryManager.createBranch must succeed");
        if self.selectionMode == ChatSelectionMode::FOLLOW_GLOBAL {
            self.chatHistoryManager
                .setCurrentChatId(branchChat.id.clone())
                .expect("ChatHistoryManager.setCurrentChatId must succeed");
        }
        self.loadChatMessages(branchChat.id);
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
    /// Resolves the character target that must remain selected after deletion.
    pub fn resolveDeletionReplacementTarget(
        &self,
        chat: ChatHistory,
    ) -> ChatDeletionReplacementTarget {
        if let Some(characterGroupId) = chat.characterGroupId.and_then(normalizedNonBlank) {
            return ChatDeletionReplacementTarget {
                characterCardName: None,
                characterCardId: None,
                characterGroupId: Some(characterGroupId),
                includeUnboundChats: false,
            };
        }

        if let Some(characterCardName) = chat.characterCardName.and_then(normalizedNonBlank) {
            let characterCard = self
                .characterCardManager
                .findCharacterCardByName(&characterCardName)
                .expect("CharacterCardManager.findCharacterCardByName must succeed");
            return ChatDeletionReplacementTarget {
                characterCardId: characterCard.as_ref().map(|card| card.id.clone()),
                includeUnboundChats: characterCard.as_ref().is_some_and(|card| card.isDefault),
                characterCardName: Some(characterCardName),
                characterGroupId: None,
            };
        }

        match self
            .activePromptManager
            .getActivePrompt()
            .expect("ActivePromptManager.getActivePrompt must succeed")
        {
            ActivePrompt::CharacterGroup { id } => ChatDeletionReplacementTarget {
                characterCardName: None,
                characterCardId: None,
                characterGroupId: normalizedNonBlank(id),
                includeUnboundChats: false,
            },
            ActivePrompt::CharacterCard { id } => {
                let characterCard = self
                    .characterCardManager
                    .getCharacterCard(&id)
                    .expect("CharacterCardManager.getCharacterCard must succeed");
                ChatDeletionReplacementTarget {
                    characterCardName: Some(characterCard.name),
                    characterCardId: Some(characterCard.id),
                    characterGroupId: None,
                    includeUnboundChats: characterCard.isDefault,
                }
            }
        }
    }

    #[allow(non_snake_case)]
    /// Checks whether a chat belongs to the selected deletion target.
    pub fn matchesDeletionReplacementTarget(
        &self,
        chat: &ChatHistory,
        target: &ChatDeletionReplacementTarget,
    ) -> bool {
        let chatCharacterGroupId = chat.characterGroupId.clone().and_then(normalizedNonBlank);
        let chatCharacterCardName = chat.characterCardName.clone().and_then(normalizedNonBlank);

        if let Some(targetCharacterGroupId) = target.characterGroupId.as_ref() {
            return chatCharacterGroupId.as_ref() == Some(targetCharacterGroupId);
        }

        if let Some(targetCharacterCardName) = target.characterCardName.as_ref() {
            if chatCharacterGroupId.is_some() {
                return false;
            }
            return if target.includeUnboundChats {
                chatCharacterCardName.is_none()
                    || chatCharacterCardName.as_ref() == Some(targetCharacterCardName)
            } else {
                chatCharacterCardName.as_ref() == Some(targetCharacterCardName)
            };
        }

        chatCharacterGroupId.is_none() && chatCharacterCardName.is_none()
    }

    #[allow(non_snake_case)]
    /// Finds the newest existing chat that belongs to the deletion target.
    pub fn findLatestDeletionReplacementChat(
        &self,
        deletingChatId: String,
        target: &ChatDeletionReplacementTarget,
    ) -> Option<ChatHistory> {
        self.chatHistoriesFlow
            .value()
            .iter()
            .filter(|chat| chat.id != deletingChatId)
            .filter(|chat| self.matchesDeletionReplacementTarget(chat, target))
            .max_by_key(|chat| {
                chat.updatedAt
                    .parse::<i64>()
                    .expect("ChatHistory.updatedAt must be an epoch millis string")
            })
            .cloned()
    }

    #[allow(non_snake_case)]
    /// Moves selection away from the active chat before deleting it.
    pub fn moveCurrentChatAwayBeforeDeletion(&mut self, currentChat: ChatHistory) -> bool {
        let deletingChatId = currentChat.id.clone();
        let target = self.resolveDeletionReplacementTarget(currentChat);
        if let Some(replacementChat) =
            self.findLatestDeletionReplacementChat(deletingChatId.clone(), &target)
        {
            self.switchChat(
                replacementChat.id.clone(),
                self.selectionMode == ChatSelectionMode::FOLLOW_GLOBAL,
            );
            return self.currentChatIdFlow.value().as_ref() == Some(&replacementChat.id);
        }

        self.createNewChat(
            target.characterCardName,
            target.characterGroupId,
            None,
            true,
            true,
            target.characterCardId,
        );
        self.currentChatIdFlow.value().as_ref() != Some(&deletingChatId)
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
        let deleted = if self.currentChatIdFlow.value().as_ref() == Some(&chatId) {
            match self
                .chatHistoriesFlow
                .value()
                .iter()
                .find(|chat| chat.id == chatId)
                .cloned()
            {
                Some(currentChat) => {
                    self.moveCurrentChatAwayBeforeDeletion(currentChat)
                        && self
                            .chatHistoryManager
                            .deleteChatHistory(chatId.clone())
                            .expect("ChatHistoryManager.deleteChatHistory must succeed")
                }
                None => false,
            }
        } else {
            self.chatHistoryManager
                .deleteChatHistory(chatId.clone())
                .expect("ChatHistoryManager.deleteChatHistory must succeed")
        };
        if deleted {
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
    /// Deletes one alternate message variant by timestamp and variant index.
    pub fn deleteMessageVariant(&mut self, timestamp: i64, variantIndex: i32) {
        let Some(chatId) = self.currentChatIdFlow.value() else {
            return;
        };
        self.chatHistoryManager
            .deleteMessageVariant(chatId.clone(), timestamp, variantIndex)
            .expect("ChatHistoryManager.deleteMessageVariant must remove the requested variant");
        self.updateOpenedChatMessage(&chatId, timestamp, |message| {
            message.variantCount = (message.variantCount - 1).max(1);
            if message.selectedVariantIndex == variantIndex {
                message.selectedVariantIndex = 0;
            }
        });
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
        self.updateOpenedChatMessage(&chatId, timestamp, |message| {
            message.selectedVariantIndex = selectedVariantIndex;
        });
    }

    #[allow(non_snake_case)]
    /// Adds a variant to a message and refreshes the active display when applicable.
    pub fn addMessageVariant(
        &mut self,
        timestamp: i64,
        message: ChatMessage,
        chatIdOverride: Option<String>,
    ) -> i32 {
        let chatId = chatIdOverride
            .or_else(|| self.currentChatIdFlow.value())
            .expect("No active chat");
        let mut publishedMessage = message;
        let selectedVariantIndex = self
            .chatHistoryManager
            .addMessageVariant(chatId.clone(), timestamp, publishedMessage.clone())
            .expect("ChatHistoryManager.addMessageVariant must succeed");
        publishedMessage.timestamp = timestamp;
        publishedMessage.selectedVariantIndex = selectedVariantIndex;
        self.updateOpenedChatMessage(&chatId, timestamp, |message| {
            publishedMessage.variantCount = message.variantCount + 1;
            *message = publishedMessage.clone();
        });
        ChainLogger::info(
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
            self.createNewChat(None, None, None, true, true, None);
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
                ChainLogger::info(
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

    #[allow(non_snake_case)]
    /// Binds a chat to a workspace.
    pub fn bindChatToWorkspace(&mut self, chatId: String, workspace: String) {
        self.chatHistoryManager
            .updateChatWorkspace(chatId.clone(), Some(workspace.clone()))
            .expect("ChatHistoryManager.updateChatWorkspace must succeed");
        if self.currentChatIdFlow.value().as_ref() == Some(&chatId) {
            self.dispatchChatViewEvent(ChatViewEvent::ViewUpdated, &chatId);
        }
    }

    #[allow(non_snake_case)]
    /// Updates the character-card binding for a chat.
    pub fn updateChatCharacterCard(&mut self, chatId: String, characterCardName: Option<String>) {
        self.updateChatCharacterBinding(chatId, characterCardName, None);
    }

    #[allow(non_snake_case)]
    /// Updates the character-group binding for a chat.
    pub fn updateChatCharacterGroup(&mut self, chatId: String, characterGroupId: Option<String>) {
        self.updateChatCharacterBinding(chatId, None, characterGroupId);
    }

    #[allow(non_snake_case)]
    /// Updates character-card and character-group bindings for a chat.
    pub fn updateChatCharacterBinding(
        &mut self,
        chatId: String,
        characterCardName: Option<String>,
        characterGroupId: Option<String>,
    ) {
        self.chatHistoryManager
            .updateChatCharacterBinding(
                chatId.clone(),
                characterCardName.clone(),
                characterGroupId.clone(),
            )
            .expect("ChatHistoryManager.updateChatCharacterBinding must succeed");
        if self.currentChatIdFlow.value().as_ref() == Some(&chatId) {
            self.activatePromptForChat(chatId.clone());
            self.dispatchChatViewEvent(ChatViewEvent::ViewUpdated, &chatId);
        }
    }

    #[allow(non_snake_case)]
    /// Removes the workspace binding from a chat.
    pub fn unbindChatFromWorkspace(&mut self, chatId: String) {
        self.chatHistoryManager
            .updateChatWorkspace(chatId.clone(), None)
            .expect("ChatHistoryManager.updateChatWorkspace must succeed");
        if self.currentChatIdFlow.value().as_ref() == Some(&chatId) {
            self.dispatchChatViewEvent(ChatViewEvent::ViewUpdated, &chatId);
        }
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
            AppLogger::i(
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
        AppLogger::i(
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
        AppLogger::i(
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
                ChainLogger::info(
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
            ChainLogger::info(
                MESSAGE_STORE_CHAIN,
                "message.store.hidden.update",
                &[
                    ("chatId", targetChatId.clone()),
                    ("sender", messageSender),
                    ("timestamp", messageTimestamp.to_string()),
                    ("messageChars", messageChars),
                ],
            );
            AppLogger::i(
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
            ChainLogger::info(
                MESSAGE_STORE_CHAIN,
                "message.store.background.update",
                &[
                    ("chatId", targetChatId.clone()),
                    ("sender", messageSender),
                    ("timestamp", messageTimestamp.to_string()),
                    ("messageChars", messageChars),
                ],
            );
            AppLogger::i(
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
            ChainLogger::info(
                MESSAGE_STORE_CHAIN,
                "message.store.visible.update",
                &[
                    ("chatId", targetChatId.clone()),
                    ("sender", messageSender),
                    ("timestamp", messageTimestamp.to_string()),
                    ("messageChars", messageChars),
                ],
            );
            AppLogger::i(
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
            ChainLogger::info(
                MESSAGE_STORE_CHAIN,
                "message.store.visible.insert",
                &[
                    ("chatId", targetChatId.clone()),
                    ("sender", messageSender),
                    ("timestamp", messageTimestamp.to_string()),
                    ("messageChars", messageChars),
                ],
            );
            AppLogger::i(
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
            ChainLogger::info(
                MESSAGE_STORE_CHAIN,
                "message.store.window.update",
                &[
                    ("chatId", targetChatId.clone()),
                    ("sender", messageSender),
                    ("timestamp", messageTimestamp.to_string()),
                    ("messageChars", messageChars),
                ],
            );
            AppLogger::i(
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

    /// Commits one assistant segment atomically and publishes its in-memory state.
    #[allow(non_snake_case)]
    pub fn commitAssistantMessageSegment(
        &mut self,
        chatId: String,
        message: ChatMessage,
        chatMetrics: Option<(i64, i64, i64)>,
    ) -> Result<SyncClock, String> {
        AppLogger::i(
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
        ToolPkgChatMessageHookBridge::dispatchMessagePersisted(&chatId, &message);
        if self.currentChatIdFlow.value().as_ref() == Some(&chatId) {
            self.upsertCurrentChatMessageInMemory(message.clone());
        } else {
            self.publishChatMessage(&chatId, message.clone());
        }
        AppLogger::i(
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
        AppLogger::i(
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

    #[allow(non_snake_case)]
    /// Persists a reordered chat list and optionally moves one chat to a group.
    pub fn updateChatOrderAndGroup(
        &mut self,
        reorderedHistories: Vec<ChatHistoryListItem>,
        movedItem: ChatHistoryListItem,
        targetGroup: Option<String>,
    ) {
        let updatedList = reorderedHistories
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let mut history = self
                    .chatHistoriesFlow
                    .value()
                    .iter()
                    .find(|history| history.id == item.id)
                    .cloned()
                    .expect("Chat history list item id must exist in chat histories");
                history.displayOrder = index as i64;
                if history.id == movedItem.id {
                    history.group = targetGroup.clone();
                }
                history
            })
            .collect::<Vec<_>>();

        self.chatHistoryManager
            .updateChatOrderAndGroup(updatedList)
            .expect("ChatHistoryManager.updateChatOrderAndGroup must succeed");
    }

    #[allow(non_snake_case)]
    /// Renames a chat group, optionally scoped to a character card.
    pub fn updateGroupName(
        &mut self,
        oldName: String,
        newName: String,
        characterCardName: Option<String>,
    ) {
        self.chatHistoryManager
            .updateGroupName(oldName.clone(), newName.clone(), characterCardName.clone())
            .expect("ChatHistoryManager.updateGroupName must succeed");
    }

    #[allow(non_snake_case)]
    /// Deletes a chat group and optionally deletes the chats inside it.
    pub fn deleteGroup(
        &mut self,
        groupName: String,
        deleteChats: bool,
        characterCardName: Option<String>,
    ) {
        let matchesGroup = |chat: &ChatHistory| {
            chat.group.as_deref() == Some(groupName.as_str())
                && characterCardName
                    .as_ref()
                    .map(|name| chat.characterCardName.as_ref() == Some(name))
                    .unwrap_or(true)
        };
        let deletedChatIds = deleteChats
            .then(|| {
                self.chatHistoriesFlow
                    .value()
                    .iter()
                    .filter(|chat| matchesGroup(chat))
                    .map(|chat| chat.id.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.chatHistoryManager
            .deleteGroup(groupName.clone(), deleteChats, characterCardName.clone())
            .expect("ChatHistoryManager.deleteGroup must succeed");
        if deleteChats {
            for chatId in &deletedChatIds {
                self.clearChatFlow(chatId);
            }
            if self
                .currentChatIdFlow
                .value()
                .as_ref()
                .map(|chatId| deletedChatIds.iter().any(|deletedId| deletedId == chatId))
                .unwrap_or(false)
            {
                self.currentChatIdFlow.set_value(None);
                self.clearCurrentChatHistoryInMemory();
            }
        }
    }

    #[allow(non_snake_case)]
    /// Creates a chat group and switches into its new backing chat.
    pub fn createGroup(
        &mut self,
        groupName: String,
        characterCardName: Option<String>,
        characterGroupId: Option<String>,
    ) {
        if let Some(currentChatId) = self.currentChatIdFlow.value() {
            let statistics = self
                .chatHistoriesFlow
                .value()
                .iter()
                .find(|chat| chat.id == currentChatId)
                .map(|chat| (chat.inputTokens, chat.outputTokens, chat.currentWindowSize));
            if let Some((inputTokens, outputTokens, windowSize)) = statistics {
                self.saveCurrentChat(inputTokens, outputTokens, windowSize, Some(currentChatId));
            }
        }

        let newChat = self
            .chatHistoryManager
            .createNewChat(None, Some(groupName), characterCardName, characterGroupId)
            .expect("ChatHistoryManager.createNewChat must succeed");
        if self.selectionMode == ChatSelectionMode::FOLLOW_GLOBAL {
            self.chatHistoryManager
                .setCurrentChatId(newChat.id.clone())
                .expect("ChatHistoryManager.setCurrentChatId must succeed");
        }
        self.loadChatMessages(newChat.id);
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

fn normalizedNonBlank(value: String) -> Option<String> {
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}
