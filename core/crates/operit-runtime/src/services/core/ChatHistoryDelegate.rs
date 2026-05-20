use crate::data::model::ChatHistory::ChatHistory;
use crate::data::model::ChatMessage::ChatMessage;
use crate::data::repository::ChatHistoryManager::ChatHistoryManager;
use operit_store::PreferencesDataStore::{mutableStateFlow, MutableStateFlow, StateFlow};

pub const DISPLAY_WINDOW_QUERY_BATCH_SIZE: usize = 80;

#[derive(Clone, Debug, PartialEq)]
pub enum ChatSelectionMode {
    FOLLOW_GLOBAL,
    LOCAL_ONLY,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CurrentChatWindowLoadResult {
    pub messages: Vec<ChatMessage>,
    pub hasOlderPersistedHistory: bool,
    pub hasNewerPersistedHistory: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChatMessageLocatorPreview {
    pub timestamp: i64,
    pub sender: String,
    pub contentPreview: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChatDeletionReplacementTarget {
    pub chatId: String,
    pub syncToGlobal: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CurrentChatWindowController {
    pub hasOlderDisplayHistory: bool,
    pub hasNewerDisplayHistory: bool,
    pub isLoadingDisplayWindow: bool,
}

impl CurrentChatWindowController {
    pub fn new() -> Self {
        Self {
            hasOlderDisplayHistory: false,
            hasNewerDisplayHistory: false,
            isLoadingDisplayWindow: false,
        }
    }

    pub fn reset(&mut self) {
        self.hasOlderDisplayHistory = false;
        self.hasNewerDisplayHistory = false;
        self.isLoadingDisplayWindow = false;
    }
}

pub struct ChatHistoryDelegate {
    pub chatHistoryManager: ChatHistoryManager,
    pub selectionMode: ChatSelectionMode,
    pub chatHistory: Vec<ChatMessage>,
    pub chatHistoryFlow: MutableStateFlow<Vec<ChatMessage>>,
    pub currentChatWindow: CurrentChatWindowController,
    pub hasOlderDisplayHistory: bool,
    pub hasNewerDisplayHistory: bool,
    pub isLoadingDisplayWindow: bool,
    pub latestDisplayPageCountByChatId: Vec<(String, i32)>,
    pub showChatHistorySelector: bool,
    pub chatHistories: Vec<ChatHistory>,
    pub chatHistoriesFlow: MutableStateFlow<Vec<ChatHistory>>,
    pub currentChatId: Option<String>,
    pub currentChatIdFlow: MutableStateFlow<Option<String>>,
    pub isInitialized: bool,
    pub allowAddMessage: bool,
    pub beforeDestructiveHistoryMutation: Option<fn(String)>,
    pub afterDestructiveHistoryMutation: Option<fn(String)>,
    pub pendingPersistChatOrderJob: Option<String>,
}

impl ChatHistoryDelegate {
    pub fn new(selectionMode: ChatSelectionMode) -> Self {
        Self {
            chatHistoryManager: ChatHistoryManager::default()
                .expect("ChatHistoryManager must initialize for ChatHistoryDelegate"),
            selectionMode,
            chatHistory: Vec::new(),
            chatHistoryFlow: mutableStateFlow(Vec::new()),
            currentChatWindow: CurrentChatWindowController::new(),
            hasOlderDisplayHistory: false,
            hasNewerDisplayHistory: false,
            isLoadingDisplayWindow: false,
            latestDisplayPageCountByChatId: Vec::new(),
            showChatHistorySelector: false,
            chatHistories: Vec::new(),
            chatHistoriesFlow: mutableStateFlow(Vec::new()),
            currentChatId: None,
            currentChatIdFlow: mutableStateFlow(None),
            isInitialized: false,
            allowAddMessage: true,
            beforeDestructiveHistoryMutation: None,
            afterDestructiveHistoryMutation: None,
            pendingPersistChatOrderJob: None,
        }
    }

    #[allow(non_snake_case)]
    pub fn clone_for_core(&self) -> Self {
        Self {
            chatHistoryManager: ChatHistoryManager::default()
                .expect("ChatHistoryManager must initialize for ChatHistoryDelegate"),
            selectionMode: self.selectionMode.clone(),
            chatHistory: self.chatHistory.clone(),
            chatHistoryFlow: self.chatHistoryFlow.clone(),
            currentChatWindow: self.currentChatWindow.clone(),
            hasOlderDisplayHistory: self.hasOlderDisplayHistory,
            hasNewerDisplayHistory: self.hasNewerDisplayHistory,
            isLoadingDisplayWindow: self.isLoadingDisplayWindow,
            latestDisplayPageCountByChatId: self.latestDisplayPageCountByChatId.clone(),
            showChatHistorySelector: self.showChatHistorySelector,
            chatHistories: self.chatHistories.clone(),
            chatHistoriesFlow: self.chatHistoriesFlow.clone(),
            currentChatId: self.currentChatId.clone(),
            currentChatIdFlow: self.currentChatIdFlow.clone(),
            isInitialized: self.isInitialized,
            allowAddMessage: self.allowAddMessage,
            beforeDestructiveHistoryMutation: self.beforeDestructiveHistoryMutation,
            afterDestructiveHistoryMutation: self.afterDestructiveHistoryMutation,
            pendingPersistChatOrderJob: self.pendingPersistChatOrderJob.clone(),
        }
    }

    #[allow(non_snake_case)]
    pub fn chatHistoryFlow(&self) -> StateFlow<Vec<ChatMessage>> {
        self.chatHistoryFlow.asStateFlow()
    }

    #[allow(non_snake_case)]
    pub fn chatHistoriesFlow(&self) -> StateFlow<Vec<ChatHistory>> {
        self.chatHistoriesFlow.asStateFlow()
    }

    #[allow(non_snake_case)]
    pub fn currentChatIdFlow(&self) -> StateFlow<Option<String>> {
        self.currentChatIdFlow.asStateFlow()
    }

    #[allow(non_snake_case)]
    fn emitChatHistoryState(&mut self) {
        self.chatHistoryFlow.set_value(self.chatHistory.clone());
    }

    #[allow(non_snake_case)]
    fn emitChatHistoriesState(&mut self) {
        self.chatHistoriesFlow.set_value(self.chatHistories.clone());
    }

    #[allow(non_snake_case)]
    fn emitCurrentChatIdState(&mut self) {
        self.currentChatIdFlow.set_value(self.currentChatId.clone());
    }

    #[allow(non_snake_case)]
    fn syncStateFromFlows(&mut self) {
        self.chatHistory = self.chatHistoryFlow.value();
        self.chatHistories = self.chatHistoriesFlow.value();
        self.currentChatId = self.currentChatIdFlow.value();
    }

    #[allow(non_snake_case)]
    pub fn setBeforeDestructiveHistoryMutation(&mut self, handler: fn(String)) {
        self.beforeDestructiveHistoryMutation = Some(handler);
    }

    #[allow(non_snake_case)]
    pub fn setAfterDestructiveHistoryMutation(&mut self, handler: fn(String)) {
        self.afterDestructiveHistoryMutation = Some(handler);
    }

    #[allow(non_snake_case)]
    pub fn prepareChatForDestructiveMutation(&self, chatId: String) {
        if let Some(handler) = self.beforeDestructiveHistoryMutation {
            handler(chatId);
        }
    }

    #[allow(non_snake_case)]
    pub fn finishDestructiveHistoryMutation(&self, chatId: String) {
        if let Some(handler) = self.afterDestructiveHistoryMutation {
            handler(chatId);
        }
    }

    #[allow(non_snake_case)]
    pub fn clearCurrentChatHistoryInMemory(&mut self) {
        self.chatHistory.clear();
        self.currentChatWindow.reset();
        self.hasOlderDisplayHistory = false;
        self.hasNewerDisplayHistory = false;
        self.isLoadingDisplayWindow = false;
        self.emitChatHistoryState();
    }

    #[allow(non_snake_case)]
    pub fn setCurrentChatMessagesInMemory(
        &mut self,
        messages: Vec<ChatMessage>,
        hasOlderPersistedHistory: Option<bool>,
        hasNewerPersistedHistory: Option<bool>,
    ) {
        self.chatHistory = messages;
        self.emitChatHistoryState();
        if let Some(value) = hasOlderPersistedHistory {
            self.currentChatWindow.hasOlderDisplayHistory = value;
            self.hasOlderDisplayHistory = value;
        }
        if let Some(value) = hasNewerPersistedHistory {
            self.currentChatWindow.hasNewerDisplayHistory = value;
            self.hasNewerDisplayHistory = value;
        }
    }

    #[allow(non_snake_case)]
    pub fn refreshCurrentChatDisplayFlags(&mut self, _chatId: String, messages: Vec<ChatMessage>) {
        self.setCurrentChatMessagesInMemory(messages, None, None);
    }

    #[allow(non_snake_case)]
    pub fn buildCurrentChatLoadResult(
        &self,
        _chatId: String,
        messages: Vec<ChatMessage>,
    ) -> CurrentChatWindowLoadResult {
        CurrentChatWindowLoadResult {
            messages,
            hasOlderPersistedHistory: self.currentChatWindow.hasOlderDisplayHistory,
            hasNewerPersistedHistory: self.currentChatWindow.hasNewerDisplayHistory,
        }
    }

    #[allow(non_snake_case)]
    pub fn applyCurrentChatDisplayWindow(
        &mut self,
        chatId: String,
        messages: Vec<ChatMessage>,
    ) -> Vec<ChatMessage> {
        let loadResult = self.buildCurrentChatLoadResult(chatId, messages);
        self.setCurrentChatMessagesInMemory(
            loadResult.messages.clone(),
            Some(loadResult.hasOlderPersistedHistory),
            Some(loadResult.hasNewerPersistedHistory),
        );
        loadResult.messages
    }

    #[allow(non_snake_case)]
    pub fn currentDisplayPageCount(&self) -> i32 {
        if self.chatHistory.is_empty() { 1 } else { 1 }
    }

    #[allow(non_snake_case)]
    pub fn collectNewestDisplayPages(
        &self,
        chatId: String,
        _pageCount: i32,
        _endTimestampInclusive: Option<i64>,
    ) -> Vec<ChatMessage> {
        self.getChatHistory(chatId)
    }

    #[allow(non_snake_case)]
    pub fn collectOlderDisplayPagesBefore(
        &self,
        chatId: String,
        _pageCount: i32,
        beforeTimestampExclusive: i64,
    ) -> Vec<ChatMessage> {
        self.getChatHistory(chatId)
            .into_iter()
            .filter(|message| message.timestamp < beforeTimestampExclusive)
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn collectNewerDisplayPagesAfter(
        &self,
        chatId: String,
        _pageCount: i32,
        afterTimestampExclusive: i64,
    ) -> Vec<ChatMessage> {
        self.getChatHistory(chatId)
            .into_iter()
            .filter(|message| message.timestamp > afterTimestampExclusive)
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn loadLatestCurrentChatDisplayWindow(&mut self) -> Vec<ChatMessage> {
        let Some(chatId) = self.currentChatId.clone() else {
            self.clearCurrentChatHistoryInMemory();
            return Vec::new();
        };
        let messages = self.getChatHistory(chatId.clone());
        self.applyCurrentChatDisplayWindow(chatId, messages)
    }

    #[allow(non_snake_case)]
    pub fn reloadCurrentChatDisplayHistory(&mut self, chatId: String) -> Vec<ChatMessage> {
        let messages = self.getChatHistory(chatId.clone());
        self.applyCurrentChatDisplayWindow(chatId, messages)
    }

    #[allow(non_snake_case)]
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
    pub fn runCurrentChatDestructiveHistoryMutation<F>(&mut self, _staleMessage: String, mutation: F) -> bool
    where
        F: FnOnce(&mut Self, String) -> bool,
    {
        let Some(chatId) = self.currentChatId.clone() else {
            return false;
        };
        self.runDestructiveHistoryMutation(chatId, mutation)
    }

    #[allow(non_snake_case)]
    pub fn getChatHistory(&self, chatId: String) -> Vec<ChatMessage> {
        self.chatHistoryManager
            .loadChatMessages(&chatId)
            .expect("ChatHistoryManager.loadChatMessages must succeed")
    }

    #[allow(non_snake_case)]
    pub fn getRuntimeChatHistory(&self, chatId: String) -> Vec<ChatMessage> {
        self.getChatHistory(chatId)
            .into_iter()
            .filter(|message| message.displayMode != crate::data::model::ChatMessageDisplayMode::ChatMessageDisplayMode::HIDDEN_PLACEHOLDER)
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn getCurrentRuntimeChatHistorySnapshot(&self) -> Vec<ChatMessage> {
        let Some(chatId) = self.currentChatId.clone() else {
            return Vec::new();
        };
        self.getRuntimeChatHistory(chatId)
    }

    #[allow(non_snake_case)]
    pub fn loadMessagesForSummaryInsertion(
        &self,
        chatId: String,
        beforeTimestampExclusive: Option<i64>,
        upToTimestampInclusive: Option<i64>,
    ) -> Vec<ChatMessage> {
        self.getRuntimeChatHistory(chatId)
            .into_iter()
            .filter(|message| beforeTimestampExclusive.map(|ts| message.timestamp < ts).unwrap_or(true))
            .filter(|message| upToTimestampInclusive.map(|ts| message.timestamp <= ts).unwrap_or(true))
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn loadChatMessageLocatorPreviews(&self, chatId: String) -> Vec<ChatMessageLocatorPreview> {
        self.getChatHistory(chatId)
            .into_iter()
            .map(|message| ChatMessageLocatorPreview {
                timestamp: message.timestamp,
                sender: message.sender,
                contentPreview: message.content.chars().take(80).collect(),
            })
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn hasUserMessage(&self, chatId: String) -> bool {
        self.getChatHistory(chatId)
            .iter()
            .any(|message| message.sender == "user")
    }

    #[allow(non_snake_case)]
    pub fn revealMessageForCurrentChat(&mut self, targetTimestamp: i64) -> bool {
        self.chatHistory
            .iter()
            .any(|message| message.timestamp == targetTimestamp)
    }

    #[allow(non_snake_case)]
    pub fn loadOlderMessagesForCurrentChat(&mut self) -> bool {
        let Some(chatId) = self.currentChatId.clone() else {
            return false;
        };
        let Some(first) = self.chatHistory.first() else {
            return false;
        };
        let messages = self.collectOlderDisplayPagesBefore(chatId.clone(), self.currentDisplayPageCount(), first.timestamp);
        if messages.is_empty() {
            return false;
        }
        let mut merged = messages;
        merged.extend(self.chatHistory.clone());
        self.applyCurrentChatDisplayWindow(chatId, merged);
        true
    }

    #[allow(non_snake_case)]
    pub fn loadNewerMessagesForCurrentChat(&mut self) -> bool {
        let Some(chatId) = self.currentChatId.clone() else {
            return false;
        };
        let Some(last) = self.chatHistory.last() else {
            return false;
        };
        let messages = self.collectNewerDisplayPagesAfter(chatId.clone(), self.currentDisplayPageCount(), last.timestamp);
        if messages.is_empty() {
            return false;
        }
        let mut merged = self.chatHistory.clone();
        merged.extend(messages);
        self.applyCurrentChatDisplayWindow(chatId, merged);
        true
    }

    #[allow(non_snake_case)]
    pub fn showLatestMessagesForCurrentChat(&mut self) -> bool {
        !self.loadLatestCurrentChatDisplayWindow().is_empty()
    }

    pub fn initialize(&mut self) {
        self.chatHistories = self
            .chatHistoryManager
            .chatHistoriesFlow()
            .expect("ChatHistoryManager.chatHistoriesFlow must succeed");
        self.emitChatHistoriesState();
        if let Some(chatId) = self
            .chatHistoryManager
            .currentChatIdFlow()
            .expect("ChatHistoryManager.currentChatIdFlow must succeed")
        {
            self.currentChatId = Some(chatId.clone());
            self.emitCurrentChatIdState();
            self.loadChatMessages(chatId);
        }
        self.isInitialized = true;
    }

    #[allow(non_snake_case)]
    pub fn loadChatMessages(&mut self, chatId: String) {
        self.allowAddMessage = false;
        let messages = self.getChatHistory(chatId.clone());
        self.currentChatId = Some(chatId.clone());
        self.emitCurrentChatIdState();
        self.applyCurrentChatDisplayWindow(chatId, messages);
        self.allowAddMessage = true;
    }

    #[allow(non_snake_case)]
    pub fn reloadChatMessagesSmart(&mut self, chatId: String) {
        self.reloadCurrentChatDisplayHistory(chatId);
    }

    #[allow(non_snake_case)]
    pub fn syncOpeningStatementIfNoUserMessage(&mut self, _chatId: String) {}

    #[allow(non_snake_case)]
    pub fn checkIfShouldCreateNewChat(&self) -> bool {
        self.currentChatId.is_none()
    }

    #[allow(non_snake_case)]
    pub fn createNewChat(
        &mut self,
        characterCardName: Option<String>,
        characterGroupId: Option<String>,
        group: Option<String>,
        _inheritGroupFromCurrent: bool,
        setAsCurrentChat: bool,
        _characterCardId: Option<String>,
    ) {
        let newChat = self
            .chatHistoryManager
            .createNewChat(None, group, characterCardName, characterGroupId)
            .expect("ChatHistoryManager.createNewChat must succeed");
        self.chatHistories = self
            .chatHistoryManager
            .chatHistoriesFlow()
            .expect("ChatHistoryManager.chatHistoriesFlow must succeed");
        self.emitChatHistoriesState();
        if setAsCurrentChat {
            self.currentChatId = Some(newChat.id.clone());
            self.emitCurrentChatIdState();
            self.loadChatMessages(newChat.id);
        }
    }

    #[allow(non_snake_case)]
    pub fn switchChat(&mut self, chatId: String, _syncToGlobal: bool) {
        self.chatHistoryManager
            .setCurrentChatId(chatId.clone())
            .expect("ChatHistoryManager.setCurrentChatId must succeed");
        self.allowAddMessage = false;
        self.loadChatMessages(chatId);
        self.allowAddMessage = true;
    }

    #[allow(non_snake_case)]
    pub fn createBranch(&mut self, upToMessageTimestamp: Option<i64>) {
        let Some(currentChatId) = self.currentChatId.clone() else {
            return;
        };
        let mut branchMessages = self.getChatHistory(currentChatId);
        if let Some(timestamp) = upToMessageTimestamp {
            branchMessages.retain(|message| message.timestamp <= timestamp);
        }
        self.createNewChat(None, None, None, true, true, None);
        if let Some(branchId) = self.currentChatId.clone() {
            if let Some(chat) = self.chatHistories.iter_mut().find(|chat| chat.id == branchId) {
                chat.messages = branchMessages;
            }
            self.loadChatMessages(branchId);
        }
    }

    #[allow(non_snake_case)]
    pub fn resolveDeletionReplacementTarget(&self, chat: ChatHistory) -> Option<ChatDeletionReplacementTarget> {
        self.chatHistories
            .iter()
            .find(|candidate| candidate.id != chat.id)
            .map(|candidate| ChatDeletionReplacementTarget {
                chatId: candidate.id.clone(),
                syncToGlobal: self.selectionMode == ChatSelectionMode::FOLLOW_GLOBAL,
            })
    }

    #[allow(non_snake_case)]
    pub fn matchesDeletionReplacementTarget(&self, chat: &ChatHistory, target: &ChatDeletionReplacementTarget) -> bool {
        chat.id == target.chatId
    }

    #[allow(non_snake_case)]
    pub fn findLatestDeletionReplacementChat(&self, chat: ChatHistory) -> Option<ChatHistory> {
        self.chatHistories.iter().find(|candidate| candidate.id != chat.id).cloned()
    }

    #[allow(non_snake_case)]
    pub fn awaitCurrentChatSelection(&self, chatId: String, _timeoutMs: i64) -> bool {
        self.currentChatId.as_ref() == Some(&chatId)
    }

    #[allow(non_snake_case)]
    pub fn awaitCurrentChatChangeFrom(&self, previousChatId: String, _timeoutMs: i64) -> bool {
        self.currentChatId.as_ref() != Some(&previousChatId)
    }

    #[allow(non_snake_case)]
    pub fn moveCurrentChatAwayBeforeDeletion(&mut self, currentChat: ChatHistory) -> bool {
        let Some(target) = self.resolveDeletionReplacementTarget(currentChat) else {
            return false;
        };
        self.switchChat(target.chatId, target.syncToGlobal);
        true
    }

    #[allow(non_snake_case)]
    pub fn deleteChatHistory(&mut self, chatId: String) -> bool {
        self.prepareChatForDestructiveMutation(chatId.clone());
        if self.currentChatId.as_ref() == Some(&chatId) {
            if let Some(currentChat) = self.chatHistories.iter().find(|chat| chat.id == chatId).cloned() {
                self.moveCurrentChatAwayBeforeDeletion(currentChat);
            }
        }
        let before = self.chatHistories.len();
        self.chatHistories.retain(|chat| chat.id != chatId);
        let deleted = self.chatHistories.len() != before;
        if deleted {
            self.emitChatHistoriesState();
        }
        if deleted {
            self.finishDestructiveHistoryMutation(chatId);
        }
        deleted
    }

    #[allow(non_snake_case)]
    pub fn deleteMessage(&mut self, index: usize) -> bool {
        let Some(chatId) = self.currentChatId.clone() else {
            return false;
        };
        self.runDestructiveHistoryMutation(chatId.clone(), |delegate, _| {
            if index >= delegate.chatHistory.len() {
                return false;
            }
            let timestamp = delegate.chatHistory[index].timestamp;
            delegate.deleteMessageByTimestamp(chatId, timestamp)
        })
    }

    #[allow(non_snake_case)]
    pub fn deleteMessageByTimestamp(&mut self, chatId: String, timestamp: i64) -> bool {
        if let Some(chat) = self.chatHistories.iter_mut().find(|chat| chat.id == chatId) {
            chat.messages.retain(|message| message.timestamp != timestamp);
        }
        if self.currentChatId.as_ref() == Some(&chatId) {
            self.reloadCurrentChatDisplayHistory(chatId);
        }
        true
    }

    #[allow(non_snake_case)]
    pub fn deleteMessagesByTimestamps(&mut self, chatId: String, timestamps: Vec<i64>) {
        for timestamp in timestamps {
            self.deleteMessageByTimestamp(chatId.clone(), timestamp);
        }
    }

    #[allow(non_snake_case)]
    pub fn setMessageFavorite(&mut self, timestamp: i64, isFavorite: bool) {
        let Some(chatId) = self.currentChatId.clone() else {
            return;
        };
        if let Some(chat) = self.chatHistories.iter_mut().find(|chat| chat.id == chatId) {
            if let Some(message) = chat.messages.iter_mut().find(|message| message.timestamp == timestamp) {
                message.isFavorite = isFavorite;
            }
        }
        self.reloadCurrentChatDisplayHistory(chatId);
    }

    #[allow(non_snake_case)]
    pub fn deleteMessageVariant(&mut self, timestamp: i64, _variantIndex: i32) {
        self.deleteMessageByTimestamp(self.currentChatId.clone().unwrap_or_default(), timestamp);
    }

    #[allow(non_snake_case)]
    pub fn deleteMessagesFrom(&mut self, index: usize) -> bool {
        let Some(chatId) = self.currentChatId.clone() else {
            return false;
        };
        if index >= self.chatHistory.len() {
            return false;
        }
        let timestamp = self.chatHistory[index].timestamp;
        if let Some(chat) = self.chatHistories.iter_mut().find(|chat| chat.id == chatId) {
            chat.messages.retain(|message| message.timestamp < timestamp);
        }
        self.reloadCurrentChatDisplayHistory(chatId);
        true
    }

    #[allow(non_snake_case)]
    pub fn selectMessageVariant(&mut self, timestamp: i64, selectedVariantIndex: i32) {
        let Some(chatId) = self.currentChatId.clone() else {
            return;
        };
        if let Some(chat) = self.chatHistories.iter_mut().find(|chat| chat.id == chatId) {
            if let Some(message) = chat.messages.iter_mut().find(|message| message.timestamp == timestamp) {
                message.selectedVariantIndex = selectedVariantIndex;
            }
        }
        self.reloadCurrentChatDisplayHistory(chatId);
    }

    #[allow(non_snake_case)]
    pub fn addMessageVariant(&mut self, timestamp: i64, message: ChatMessage, chatIdOverride: Option<String>) -> i32 {
        let chatId = chatIdOverride.or_else(|| self.currentChatId.clone()).expect("No active chat");
        let mut selectedVariantIndex = 0;
        if let Some(chat) = self.chatHistories.iter_mut().find(|chat| chat.id == chatId) {
            if let Some(existing) = chat.messages.iter_mut().find(|existing| existing.timestamp == timestamp) {
                existing.variantCount += 1;
                selectedVariantIndex = existing.variantCount - 1;
            } else {
                selectedVariantIndex = message.selectedVariantIndex;
                chat.messages.push(message);
            }
        }
        if self.currentChatId.as_ref() == Some(&chatId) {
            self.reloadCurrentChatDisplayHistory(chatId);
        }
        selectedVariantIndex
    }

    #[allow(non_snake_case)]
    pub fn clearCurrentChat(&mut self) -> bool {
        let Some(chatId) = self.currentChatId.clone() else {
            self.createNewChat(None, None, None, true, true, None);
            return false;
        };
        self.prepareChatForDestructiveMutation(chatId.clone());
        if let Some(chat) = self.chatHistories.iter_mut().find(|chat| chat.id == chatId) {
            chat.messages.clear();
        }
        self.clearCurrentChatHistoryInMemory();
        self.finishDestructiveHistoryMutation(chatId);
        true
    }

    #[allow(non_snake_case)]
    pub fn saveCurrentChat(
        &mut self,
        inputTokens: i32,
        outputTokens: i32,
        actualContextWindowSize: i32,
        chatIdOverride: Option<String>,
    ) {
        let chatId = chatIdOverride.or_else(|| self.currentChatId.clone());
        if let Some(chatId) = chatId {
            let shouldSave = !self.chatHistory.is_empty()
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
                if let Some(chat) = self.chatHistories.iter_mut().find(|chat| chat.id == chatId) {
                    chat.inputTokens = inputTokens;
                    chat.outputTokens = outputTokens;
                    chat.currentWindowSize = actualContextWindowSize;
                }
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn bindChatToWorkspace(&mut self, chatId: String, workspace: String, workspaceEnv: Option<String>) {
        if let Some(chat) = self.chatHistories.iter_mut().find(|chat| chat.id == chatId) {
            chat.workspace = Some(workspace);
            chat.workspaceEnv = workspaceEnv;
        }
    }

    #[allow(non_snake_case)]
    pub fn updateChatCharacterCard(&mut self, chatId: String, characterCardName: Option<String>) {
        self.updateChatCharacterBinding(chatId, characterCardName, None);
    }

    #[allow(non_snake_case)]
    pub fn updateChatCharacterGroup(&mut self, chatId: String, characterGroupId: Option<String>) {
        self.updateChatCharacterBinding(chatId, None, characterGroupId);
    }

    #[allow(non_snake_case)]
    pub fn updateChatCharacterBinding(
        &mut self,
        chatId: String,
        characterCardName: Option<String>,
        characterGroupId: Option<String>,
    ) {
        if let Some(chat) = self.chatHistories.iter_mut().find(|chat| chat.id == chatId) {
            chat.characterCardName = characterCardName;
            chat.characterGroupId = characterGroupId;
        }
    }

    #[allow(non_snake_case)]
    pub fn unbindChatFromWorkspace(&mut self, chatId: String) {
        if let Some(chat) = self.chatHistories.iter_mut().find(|chat| chat.id == chatId) {
            chat.workspace = None;
            chat.workspaceEnv = None;
        }
    }

    #[allow(non_snake_case)]
    pub fn updateChatTitle(&mut self, chatId: String, title: String) {
        if let Some(chat) = self.chatHistories.iter_mut().find(|chat| chat.id == chatId) {
            chat.title = title;
            self.emitChatHistoriesState();
        }
    }

    #[allow(non_snake_case)]
    pub fn renameWorkspaceAndChat(&mut self, chatId: String, newWorkspace: String, newTitle: String) {
        self.bindChatToWorkspace(chatId.clone(), newWorkspace, None);
        self.updateChatTitle(chatId, newTitle);
    }

    #[allow(non_snake_case)]
    pub fn upsertCurrentChatMessageInMemory(&mut self, message: ChatMessage) -> bool {
        self.chatHistory = self.chatHistoryFlow.value();
        if let Some(existingIndex) = self.chatHistory.iter().position(|existing| existing.timestamp == message.timestamp) {
            if message.contentStream.is_none() || self.chatHistory[existingIndex].contentStream.is_none() {
                self.chatHistory[existingIndex] = message;
                self.emitChatHistoryState();
            }
            return true;
        }
        self.chatHistory.push(message);
        self.emitChatHistoryState();
        false
    }

    #[allow(non_snake_case)]
    pub fn addMessageToChat(&mut self, message: ChatMessage, chatIdOverride: Option<String>) {
        self.syncStateFromFlows();
        let Some(targetChatId) = chatIdOverride.or_else(|| self.currentChatId.clone()) else {
            return;
        };
        let isCurrentChat = self.currentChatId.as_ref() == Some(&targetChatId);
        if message.isVariantPreview {
            if isCurrentChat {
                self.upsertCurrentChatMessageInMemory(message);
            }
            return;
        }

        if isCurrentChat && !self.allowAddMessage {
            self.chatHistoryManager
                .updateMessage(targetChatId, message)
                .expect("ChatHistoryManager.updateMessage must succeed");
            return;
        }

        if !isCurrentChat {
            self.chatHistoryManager
                .updateMessage(targetChatId, message)
                .expect("ChatHistoryManager.updateMessage must succeed");
            return;
        }

        let didUpdateVisibleMessage = self.upsertCurrentChatMessageInMemory(message.clone());
        let isVisibleNewMessage = !self.currentChatWindow.hasNewerDisplayHistory
            && self.chatHistory.iter().any(|existing| existing.timestamp == message.timestamp);

        if didUpdateVisibleMessage {
            self.chatHistoryManager
                .updateMessage(targetChatId.clone(), message)
                .expect("ChatHistoryManager.updateMessage must succeed");
        } else if isVisibleNewMessage {
            self.chatHistoryManager
                .addMessage(targetChatId.clone(), message)
                .expect("ChatHistoryManager.addMessage must succeed");
            self.refreshCurrentChatDisplayFlags(targetChatId.clone(), self.chatHistory.clone());
        } else {
            self.chatHistoryManager
                .updateMessage(targetChatId.clone(), message)
                .expect("ChatHistoryManager.updateMessage must succeed");
        }

        self.chatHistories = self
            .chatHistoryManager
            .chatHistoriesFlow()
            .expect("ChatHistoryManager.chatHistoriesFlow must succeed");
        self.emitChatHistoriesState();
    }

    #[allow(non_snake_case)]
    pub fn addMessageToChatAsync(&mut self, message: ChatMessage, chatIdOverride: Option<String>) {
        self.addMessageToChat(message, chatIdOverride);
    }

    #[allow(non_snake_case)]
    pub fn truncateChatHistory(&mut self, timestampOfFirstDeletedMessage: Option<i64>) {
        let Some(chatId) = self.currentChatId.clone() else {
            return;
        };
        if let Some(chat) = self.chatHistories.iter_mut().find(|chat| chat.id == chatId) {
            if let Some(timestamp) = timestampOfFirstDeletedMessage {
                chat.messages.retain(|message| message.timestamp < timestamp);
            } else {
                chat.messages.clear();
            }
            self.emitChatHistoriesState();
        }
        self.reloadCurrentChatDisplayHistory(chatId);
    }

    #[allow(non_snake_case)]
    pub fn updateChatOrderAndGroup(&mut self, chatId: String, displayOrder: i64, group: Option<String>) {
        if let Some(chat) = self.chatHistories.iter_mut().find(|chat| chat.id == chatId) {
            chat.displayOrder = displayOrder;
            chat.group = group;
            self.emitChatHistoriesState();
        }
    }

    #[allow(non_snake_case)]
    pub fn updateGroupName(&mut self, oldName: String, newName: String, characterCardName: Option<String>) {
        for chat in self.chatHistories.iter_mut() {
            if chat.group.as_ref() == Some(&oldName) && chat.characterCardName == characterCardName {
                chat.group = Some(newName.clone());
            }
        }
        self.emitChatHistoriesState();
    }

    #[allow(non_snake_case)]
    pub fn deleteGroup(&mut self, groupName: String, deleteChats: bool, characterCardName: Option<String>) {
        if deleteChats {
            self.chatHistories.retain(|chat| !(chat.group.as_ref() == Some(&groupName) && chat.characterCardName == characterCardName));
        } else {
            for chat in self.chatHistories.iter_mut() {
                if chat.group.as_ref() == Some(&groupName) && chat.characterCardName == characterCardName {
                    chat.group = None;
                }
            }
        }
        self.emitChatHistoriesState();
    }

    #[allow(non_snake_case)]
    pub fn createGroup(&mut self, _groupName: String, _characterCardName: Option<String>, _characterGroupId: Option<String>) {}

    #[allow(non_snake_case)]
    pub fn addSummaryMessage(
        &mut self,
        summaryMessage: ChatMessage,
        beforeTimestamp: Option<i64>,
        afterTimestamp: Option<i64>,
        chatIdOverride: Option<String>,
    ) {
        let Some(chatId) = chatIdOverride.or_else(|| self.currentChatId.clone()) else {
            return;
        };
        let summaryFallbackPosition = self
            .chatHistories
            .iter()
            .find(|chat| chat.id == chatId)
            .map(|chat| self.findProperSummaryPosition(chat.messages.clone()));
        if let Some(chat) = self.chatHistories.iter_mut().find(|chat| chat.id == chatId) {
            let insertPosition = chat
                .messages
                .iter()
                .position(|message| afterTimestamp.map(|ts| message.timestamp == ts).unwrap_or(false))
                .or_else(|| {
                    beforeTimestamp.and_then(|ts| {
                        chat.messages
                            .iter()
                            .position(|message| message.timestamp == ts)
                            .map(|index| index + 1)
                    })
                })
                .or(summaryFallbackPosition)
                .unwrap_or(0);
            chat.messages.insert(insertPosition, summaryMessage);
        }
        if self.currentChatId.as_ref() == Some(&chatId) {
            self.reloadCurrentChatDisplayHistory(chatId);
        }
    }

    #[allow(non_snake_case)]
    pub fn shouldGenerateSummary(&self, messages: Vec<ChatMessage>, currentTokens: i32, maxTokens: i32) -> bool {
        !messages.is_empty() && currentTokens >= maxTokens
    }

    #[allow(non_snake_case)]
    pub fn summarizeMemory(&self, _messages: Vec<ChatMessage>) {}

    #[allow(non_snake_case)]
    pub fn findProperSummaryPosition(&self, messages: Vec<ChatMessage>) -> usize {
        messages
            .iter()
            .rposition(|message| message.sender == "ai")
            .map(|index| index + 1)
            .unwrap_or(0)
    }

    #[allow(non_snake_case)]
    pub fn toggleChatHistorySelector(&mut self) {
        self.showChatHistorySelector = !self.showChatHistorySelector;
    }

    #[allow(non_snake_case)]
    pub fn showChatHistorySelector(&mut self, show: bool) {
        self.showChatHistorySelector = show;
    }

    #[allow(non_snake_case)]
    pub fn getMemory(&self, _includePlanInfo: bool) -> Vec<(String, String)> {
        Vec::new()
    }

    #[allow(non_snake_case)]
    pub fn getEnhancedAiService(&self) -> Option<String> {
        None
    }

    #[allow(non_snake_case)]
    pub fn getCurrentTokenCounts(&self) -> (i32, i32) {
        let Some(chatId) = self.currentChatId.clone() else {
            return (0, 0);
        };
        self.chatHistories
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
