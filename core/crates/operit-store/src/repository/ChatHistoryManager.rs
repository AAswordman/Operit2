use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::PreferencesDataStore::{
    stringPreferencesKey, CoroutineScope, PreferencesDataStore, PreferencesDataStoreError,
    PreferencesKey, SharingStarted, StateFlow,
};
use crate::RuntimeStorePaths::RuntimeStorePaths;
use thiserror::Error;
use uuid::Uuid;

use crate::dao::ChatDao::ChatDao;
use crate::dao::MessageDao::MessageDao;
use crate::dao::MessageVariantDao::MessageVariantDao;
use crate::db::AppDatabase::{AppDatabase, AppDatabaseError};
use crate::sync::SqlChatSyncStore::{SqlChatSyncStore, SqlChatSyncStoreError};
use operit_model::CharacterCardChatStats::CharacterCardChatStats;
use operit_model::CharacterGroupChatStats::CharacterGroupChatStats;
use operit_model::ChatEntity::ChatEntity;
use operit_model::ChatHistory::ChatHistory;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatMessageLocatorPreview::ChatMessageLocatorPreview;
use operit_model::MessageEntity::MessageEntity;
use operit_model::MessageVariantEntity::MessageVariantEntity;
use operit_model::OperitChatArchive::{
    OperitArchivedChat, OperitArchivedMessage, OperitArchivedMessageVariant, OperitChatArchive,
    ARCHIVE_TYPE, CURRENT_FORMAT_VERSION,
};
use serde::{Deserialize, Serialize};

const LOCATOR_PREVIEW_CHAR_COUNT: i32 = 48;

/// Error surface for chat history persistence and import/export operations.
#[derive(Debug, Error)]
pub enum ChatHistoryManagerError {
    #[error(transparent)]
    Database(#[from] AppDatabaseError),
    #[error(transparent)]
    Store(#[from] crate::SqliteStore::SqliteStoreError),
    #[error(transparent)]
    Preferences(#[from] PreferencesDataStoreError),
    #[error(transparent)]
    Sync(#[from] SqlChatSyncStoreError),
    #[error("{0}")]
    IllegalArgument(String),
    #[error("{0}")]
    IllegalState(String),
}

/// Result alias used by chat history repository operations.
pub type ChatHistoryManagerResult<T> = Result<T, ChatHistoryManagerError>;

/// Repository for chats, messages, variants, branches, bindings, and sync recording.
#[derive(Clone)]
pub struct ChatHistoryManager {
    database: Arc<AppDatabase>,
    chatDao: ChatDao,
    messageDao: MessageDao,
    messageVariantDao: MessageVariantDao,
    syncStore: SqlChatSyncStore,
    currentChatIdDataStore: PreferencesDataStore,
    pub currentChatIdFlow: StateFlow<Option<String>>,
    _chatHistoriesFlow: StateFlow<Vec<ChatHistory>>,
    pub chatHistoriesFlow: StateFlow<Vec<ChatHistory>>,
}

static INSTANCE: Mutex<Option<ChatHistoryManager>> = Mutex::new(None);

/// Preference keys used by chat history state.
pub struct PreferencesKeys;

impl PreferencesKeys {
    /// Returns the key storing the current chat id.
    pub fn CURRENT_CHAT_ID() -> PreferencesKey {
        stringPreferencesKey("current_chat_id")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ImportCounters {
    newCount: i32,
    updatedCount: i32,
    skippedCount: i32,
}

impl ChatHistoryManager {
    /// Returns the shared chat history manager for the supplied runtime paths.
    pub fn getInstance(paths: RuntimeStorePaths) -> ChatHistoryManagerResult<Self> {
        let mut instance = INSTANCE
            .lock()
            .expect("ChatHistoryManager.INSTANCE mutex must not be poisoned");
        if let Some(manager) = instance.as_ref() {
            return Ok(manager.clone());
        }
        let manager = Self::create(paths)?;
        *instance = Some(manager.clone());
        Ok(manager)
    }

    /// Returns the shared chat history manager for default runtime paths.
    pub fn default() -> ChatHistoryManagerResult<Self> {
        Self::getInstance(RuntimeStorePaths::default())
    }

    /// Creates or returns the shared manager for the supplied runtime paths.
    pub fn new(paths: RuntimeStorePaths) -> ChatHistoryManagerResult<Self> {
        Self::getInstance(paths)
    }

    /// Opens database handles and observable flows for chat history state.
    fn create(paths: RuntimeStorePaths) -> ChatHistoryManagerResult<Self> {
        let currentChatIdDataStore =
            PreferencesDataStore::new(paths.current_chat_id_preferences_path());
        let database = AppDatabase::getDatabase(paths.clone())?;
        let chatDao = database.chatDao();
        let messageDao = database.messageDao();
        let messageVariantDao = database.messageVariantDao();
        let syncStore = SqlChatSyncStore::new(paths.clone(), &database)?;
        let currentChatIdFlow = currentChatIdDataStore
            .dataFlow()
            .catch(|exception| match exception {
                PreferencesDataStoreError::Io(error)
                    if error.kind() == std::io::ErrorKind::NotFound =>
                {
                    Ok(crate::PreferencesDataStore::emptyPreferences())
                }
                error => Err(error),
            })
            .map(|preferences| {
                preferences
                    .get(&PreferencesKeys::CURRENT_CHAT_ID())
                    .cloned()
            })
            .stateIn(CoroutineScope, SharingStarted::Lazily, None);
        let _chatHistoriesFlow = chatDao.getAllChats()?.map(|chatEntities| {
            chatEntities
                .into_iter()
                .map(|chatEntity| chatEntity.toChatHistory(Vec::new()))
                .collect()
        });
        let chatHistoriesFlow = _chatHistoriesFlow.clone();
        Ok(Self {
            database,
            chatDao,
            messageDao,
            messageVariantDao,
            syncStore,
            currentChatIdDataStore,
            currentChatIdFlow,
            _chatHistoriesFlow,
            chatHistoriesFlow,
        })
    }

    /// Hydrates message entities with selected variants into chat messages.
    fn hydrateMessages(
        &self,
        messageEntities: Vec<MessageEntity>,
        variants: Vec<MessageVariantEntity>,
    ) -> Vec<ChatMessage> {
        let mut variantsByTimestamp: HashMap<i64, Vec<MessageVariantEntity>> = HashMap::new();
        for variant in variants {
            variantsByTimestamp
                .entry(variant.messageTimestamp)
                .or_default()
                .push(variant);
        }
        for variants in variantsByTimestamp.values_mut() {
            variants.sort_by_key(|variant| variant.variantIndex);
        }

        messageEntities
            .into_iter()
            .map(|messageEntity| {
                let baseMessage = messageEntity.toChatMessage();
                let messageVariants = variantsByTimestamp
                    .get(&messageEntity.timestamp)
                    .cloned()
                    .unwrap_or_default();
                let variantCount = messageVariants.len() as i32 + 1;
                if messageEntity.selectedVariantIndex == 0 {
                    ChatMessage {
                        selectedVariantIndex: 0,
                        variantCount,
                        ..baseMessage
                    }
                } else {
                    let selectedVariant = messageVariants
                        .iter()
                        .find(|variant| variant.variantIndex == messageEntity.selectedVariantIndex)
                        .expect("selected variant must exist for message");
                    selectedVariant.applyTo(baseMessage, variantCount)
                }
            })
            .collect()
    }

    /// Loads and hydrates all messages and variants for one chat.
    fn hydrateMessagesForChat(
        &self,
        chatId: &str,
        messageEntities: Vec<MessageEntity>,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        if messageEntities.is_empty() {
            return Ok(Vec::new());
        }
        let visibleTimestamps = messageEntities
            .iter()
            .map(|message| message.timestamp)
            .collect::<Vec<_>>();
        let variants = self
            .messageVariantDao
            .getVariantsForMessages(chatId, visibleTimestamps)?;
        Ok(self.hydrateMessages(messageEntities, variants))
    }

    /// Loads a chat history row with display-ready messages.
    fn loadDisplayHistory(
        &self,
        chatHistory: ChatHistory,
    ) -> ChatHistoryManagerResult<ChatHistory> {
        let messages = self.loadChatMessages(&chatHistory.id)?;
        Ok(ChatHistory {
            messages,
            ..chatHistory
        })
    }

    /// Loads display-ready histories for a list of chat entities.
    fn loadDisplayHistories(
        &self,
        chatHistories: Vec<ChatHistory>,
    ) -> ChatHistoryManagerResult<Vec<ChatHistory>> {
        let mut completeHistories = Vec::new();
        for chatHistory in chatHistories {
            completeHistories.push(self.loadDisplayHistory(chatHistory)?);
        }
        Ok(completeHistories)
    }

    /// Converts one chat and its messages into an archive entry.
    fn buildOperitArchivedChat(
        &self,
        chatHistory: ChatHistory,
    ) -> ChatHistoryManagerResult<OperitArchivedChat> {
        let messageEntities = self.messageDao.getMessagesForChat(&chatHistory.id)?;
        let mut variantsByTimestamp: HashMap<i64, Vec<MessageVariantEntity>> = HashMap::new();
        for variant in self.messageVariantDao.getVariantsForChat(&chatHistory.id)? {
            variantsByTimestamp
                .entry(variant.messageTimestamp)
                .or_default()
                .push(variant);
        }
        for variants in variantsByTimestamp.values_mut() {
            variants.sort_by_key(|variant| variant.variantIndex);
        }
        let archivedMessages = messageEntities
            .into_iter()
            .map(|messageEntity| {
                let messageVariants = variantsByTimestamp
                    .remove(&messageEntity.timestamp)
                    .unwrap_or_default();
                OperitArchivedMessage {
                    baseMessage: ChatMessage {
                        variantCount: messageVariants.len() as i32 + 1,
                        ..messageEntity.toChatMessage()
                    },
                    variants: messageVariants
                        .into_iter()
                        .map(OperitArchivedMessageVariant::fromEntity)
                        .collect(),
                }
            })
            .collect();
        OperitArchivedChat::fromChatHistory(chatHistory, archivedMessages)
            .map_err(ChatHistoryManagerError::IllegalState)
    }

    fn toChatHistory(&self, chatEntity: ChatEntity) -> ChatHistory {
        chatEntity.toChatHistory(Vec::new())
    }

    /// Reads the current chat history list.
    pub fn chatHistoriesFlow(&self) -> ChatHistoryManagerResult<Vec<ChatHistory>> {
        Ok(self.chatHistoriesFlow.value())
    }

    /// Counts all stored chats.
    pub fn getTotalChatCount(&self) -> ChatHistoryManagerResult<i32> {
        Ok(self.chatDao.getTotalChatCount()?)
    }

    /// Counts all stored messages.
    pub fn getTotalMessageCount(&self) -> ChatHistoryManagerResult<i32> {
        Ok(self.messageDao.getTotalMessageCount()?)
    }

    pub fn getMessageCountsByChatId(&self) -> ChatHistoryManagerResult<HashMap<String, i32>> {
        Ok(self
            .messageDao
            .getMessageCountsByChatId()?
            .into_iter()
            .map(|count| (count.chatId, count.count))
            .collect())
    }

    /// Reads chat counts grouped by character card.
    pub fn characterCardStatsFlow(&self) -> ChatHistoryManagerResult<Vec<CharacterCardChatStats>> {
        Ok(self.chatDao.getCharacterCardChatStats()?)
    }

    /// Reads chat counts grouped by character group.
    pub fn characterGroupStatsFlow(
        &self,
    ) -> ChatHistoryManagerResult<Vec<CharacterGroupChatStats>> {
        Ok(self.chatDao.getCharacterGroupChatStats()?)
    }

    /// Loads chat histories bound to a character card.
    pub fn getChatHistoriesByCharacterCard(
        &self,
        characterCardName: String,
        isDefault: bool,
    ) -> ChatHistoryManagerResult<Vec<ChatHistory>> {
        let chats = if isDefault {
            self.chatDao
                .getChatsByCharacterCardOrNull(&characterCardName)?
        } else {
            self.chatDao.getChatsByCharacterCard(&characterCardName)?
        };
        Ok(chats
            .into_iter()
            .map(|chat| self.toChatHistory(chat))
            .collect())
    }

    /// Reads the persisted current chat id.
    pub fn currentChatIdFlow(&self) -> ChatHistoryManagerResult<Option<String>> {
        Ok(self.currentChatIdFlow.first()?)
    }

    fn saveChatHistoryInternal(&self, history: ChatHistory) -> ChatHistoryManagerResult<()> {
        let chatEntity = ChatEntity::fromChatHistory(&history);
        self.chatDao.insertChat(chatEntity.clone())?;
        self.messageDao.deleteAllMessagesForChat(&chatEntity.id)?;
        self.messageVariantDao
            .deleteAllVariantsForChat(&chatEntity.id)?;

        let messageEntities = history
            .messages
            .into_iter()
            .enumerate()
            .map(|(index, message)| {
                MessageEntity::fromChatMessage(
                    chatEntity.id.clone(),
                    ChatMessage {
                        selectedVariantIndex: 0,
                        variantCount: 1,
                        ..message
                    },
                    index as i32,
                    0,
                )
            })
            .collect::<Vec<_>>();
        self.messageDao.insertMessages(messageEntities)?;
        Ok(())
    }

    /// Saves chat metadata and records a chat snapshot for sync.
    pub fn saveChatHistory(&self, history: ChatHistory) -> ChatHistoryManagerResult<()> {
        let chatId = history.id.clone();
        self.saveChatHistoryInternal(history)?;
        self.recordChatSnapshot(&chatId)?;
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Exports all chats and messages as an Operit chat archive.
    pub fn exportChatHistoriesToJson(&self) -> ChatHistoryManagerResult<String> {
        let chats = self
            .chatHistoriesFlow
            .value()
            .into_iter()
            .map(|chatHistory| self.buildOperitArchivedChat(chatHistory))
            .collect::<ChatHistoryManagerResult<Vec<_>>>()?;
        let archive = OperitChatArchive {
            archiveType: ARCHIVE_TYPE.to_string(),
            formatVersion: CURRENT_FORMAT_VERSION,
            exportedAt: currentTimeMillis(),
            chats,
        };
        serde_json::to_string_pretty(&archive)
            .map_err(|error| ChatHistoryManagerError::IllegalState(error.to_string()))
    }

    #[allow(non_snake_case)]
    /// Imports chats and messages from an Operit chat archive.
    pub fn importChatHistoriesFromJson(
        &self,
        jsonString: String,
    ) -> ChatHistoryManagerResult<ChatImportResult> {
        let archive: OperitChatArchive = serde_json::from_str(&jsonString)
            .map_err(|error| ChatHistoryManagerError::IllegalArgument(error.to_string()))?;
        if archive.archiveType != ARCHIVE_TYPE {
            return Err(ChatHistoryManagerError::IllegalArgument(format!(
                "invalid archiveType: {}",
                archive.archiveType
            )));
        }
        if archive.formatVersion != CURRENT_FORMAT_VERSION {
            return Err(ChatHistoryManagerError::IllegalArgument(format!(
                "unsupported formatVersion: {}",
                archive.formatVersion
            )));
        }

        let mut existingIds = self
            .chatHistoriesFlow
            .value()
            .into_iter()
            .map(|chat| chat.id)
            .collect::<HashSet<_>>();
        let mut counters = ImportCounters {
            newCount: 0,
            updatedCount: 0,
            skippedCount: 0,
        };
        for archivedChat in archive.chats {
            if archivedChat.messages.is_empty() {
                counters.skippedCount += 1;
                continue;
            }
            if existingIds.contains(&archivedChat.id) {
                counters.updatedCount += 1;
            } else {
                existingIds.insert(archivedChat.id.clone());
                counters.newCount += 1;
            }
            self.saveArchivedChat(archivedChat)?;
        }
        Ok(ChatImportResult {
            new: counters.newCount,
            updated: counters.updatedCount,
            skipped: counters.skippedCount,
        })
    }

    #[allow(non_snake_case)]
    fn saveArchivedChat(&self, archivedChat: OperitArchivedChat) -> ChatHistoryManagerResult<()> {
        let chatId = archivedChat.id.clone();
        let history = archivedChat
            .toChatHistory()
            .map_err(ChatHistoryManagerError::IllegalArgument)?;
        self.chatDao
            .insertChat(ChatEntity::fromChatHistory(&history))?;
        self.messageDao.deleteAllMessagesForChat(&chatId)?;
        self.messageVariantDao.deleteAllVariantsForChat(&chatId)?;

        let mut variants = Vec::new();
        let messages = archivedChat
            .messages
            .into_iter()
            .enumerate()
            .map(|(index, archivedMessage)| {
                let baseMessage = archivedMessage.baseMessage;
                for variant in archivedMessage.variants {
                    variants.push(variant.toEntity(chatId.clone(), baseMessage.timestamp));
                }
                MessageEntity::fromChatMessage(chatId.clone(), baseMessage, index as i32, 0)
            })
            .collect::<Vec<_>>();
        self.messageDao.insertMessages(messages)?;
        self.messageVariantDao.insertVariants(variants)?;
        self.recordChatSnapshot(&chatId)?;
        Ok(())
    }

    /// Updates the locked state of a chat and records sync metadata.
    pub fn updateChatLocked(&self, chatId: String, locked: bool) -> ChatHistoryManagerResult<()> {
        self.chatDao
            .updateChatLocked(&chatId, locked, currentTimeMillis())?;
        self.recordChatMetadata(&chatId)?;
        Ok(())
    }

    /// Updates the pinned state of a chat and records sync metadata.
    pub fn updateChatPinned(&self, chatId: String, pinned: bool) -> ChatHistoryManagerResult<()> {
        self.chatDao
            .updateChatPinned(&chatId, pinned, currentTimeMillis())?;
        self.recordChatMetadata(&chatId)?;
        Ok(())
    }

    fn persistMessageLocked(
        &self,
        chatId: &str,
        messageToPersist: ChatMessage,
    ) -> ChatHistoryManagerResult<ChatMessage> {
        let messageEntity =
            MessageEntity::fromChatMessage(chatId.to_string(), messageToPersist.clone(), 0, 0);
        self.messageDao.insertMessage(messageEntity)?;
        self.touchChatMetadata(chatId)?;
        self.recordMessageSnapshot(chatId, messageToPersist.timestamp)?;
        Ok(messageToPersist)
    }

    fn resolveAnchoredMessageLocked(
        &self,
        chatId: &str,
        message: ChatMessage,
        beforeTimestamp: Option<i64>,
        afterTimestamp: Option<i64>,
    ) -> ChatHistoryManagerResult<Option<ChatMessage>> {
        if beforeTimestamp.is_none() && afterTimestamp.is_none() {
            let hasAnyMessages = !self.messageDao.getMessagesForChatAsc(chatId, 1)?.is_empty();
            return if hasAnyMessages {
                Ok(None)
            } else {
                Ok(Some(message))
            };
        }

        let beforeMessage = if let Some(beforeTimestamp) = beforeTimestamp {
            self.messageDao
                .getMessageByTimestamp(chatId, beforeTimestamp)?
        } else if let Some(afterTimestamp) = afterTimestamp {
            self.messageDao
                .getMessagesForChatBeforeTimestampExclusiveDesc(chatId, afterTimestamp, 1)?
                .into_iter()
                .next()
        } else {
            None
        };

        let afterMessage = if beforeTimestamp.is_some() && afterTimestamp.is_none() {
            self.messageDao
                .getMessagesForChatAfterTimestampExclusiveAsc(
                    chatId,
                    beforeTimestamp.expect("beforeTimestamp checked"),
                    1,
                )?
                .into_iter()
                .next()
        } else if let Some(afterTimestamp) = afterTimestamp {
            self.messageDao
                .getMessageByTimestamp(chatId, afterTimestamp)?
        } else {
            None
        };

        if beforeTimestamp.is_some() && beforeMessage.is_none() {
            return Ok(None);
        }
        if afterTimestamp.is_some() && afterMessage.is_none() {
            return Ok(None);
        }

        let actualBeforeTimestamp = beforeMessage.as_ref().map(|message| message.timestamp);
        let actualAfterTimestamp = afterMessage.as_ref().map(|message| message.timestamp);

        if let (Some(actualBeforeTimestamp), Some(actualAfterTimestamp)) =
            (actualBeforeTimestamp, actualAfterTimestamp)
        {
            if actualBeforeTimestamp >= actualAfterTimestamp {
                return Ok(None);
            }
            if actualAfterTimestamp - actualBeforeTimestamp <= 1 {
                return Ok(None);
            }
            return Ok(Some(ChatMessage {
                timestamp: actualBeforeTimestamp
                    + (actualAfterTimestamp - actualBeforeTimestamp) / 2,
                ..message
            }));
        }

        if let Some(actualBeforeTimestamp) = actualBeforeTimestamp {
            return Ok(Some(ChatMessage {
                timestamp: actualBeforeTimestamp + 1,
                ..message
            }));
        }
        if let Some(actualAfterTimestamp) = actualAfterTimestamp {
            return Ok(Some(ChatMessage {
                timestamp: actualAfterTimestamp - 1,
                ..message
            }));
        }
        Ok(Some(message))
    }

    /// Inserts a summary message between two neighboring slice anchors.
    pub fn addSummaryMessageBetweenSliceNeighbors(
        &self,
        chatId: String,
        message: ChatMessage,
        beforeTimestamp: Option<i64>,
        afterTimestamp: Option<i64>,
    ) -> ChatHistoryManagerResult<Option<ChatMessage>> {
        let beforeMessage = if let Some(beforeTimestamp) = beforeTimestamp {
            self.messageDao
                .getMessageByTimestamp(&chatId, beforeTimestamp)?
        } else if let Some(afterTimestamp) = afterTimestamp {
            self.messageDao
                .getMessagesForChatBeforeTimestampExclusiveDesc(&chatId, afterTimestamp, 1)?
                .into_iter()
                .next()
        } else {
            None
        };
        let afterMessage = if beforeTimestamp.is_some() && afterTimestamp.is_none() {
            self.messageDao
                .getMessagesForChatAfterTimestampExclusiveAsc(
                    &chatId,
                    beforeTimestamp.expect("beforeTimestamp checked"),
                    1,
                )?
                .into_iter()
                .next()
        } else if let Some(afterTimestamp) = afterTimestamp {
            self.messageDao
                .getMessageByTimestamp(&chatId, afterTimestamp)?
        } else {
            None
        };

        if beforeMessage
            .as_ref()
            .map(|message| message.sender.as_str())
            == Some("summary")
            || afterMessage.as_ref().map(|message| message.sender.as_str()) == Some("summary")
        {
            return Ok(None);
        }

        let messageToPersist =
            self.resolveAnchoredMessageLocked(&chatId, message, beforeTimestamp, afterTimestamp)?;
        if let Some(messageToPersist) = messageToPersist {
            Ok(Some(self.persistMessageLocked(&chatId, messageToPersist)?))
        } else {
            Ok(None)
        }
    }

    /// Adds or replaces a message in a chat and records message sync state.
    pub fn addMessage(
        &self,
        chatId: String,
        message: ChatMessage,
    ) -> ChatHistoryManagerResult<ChatMessage> {
        self.persistMessageLocked(&chatId, message)
    }

    /// Persists reordered chats and their group assignments.
    pub fn updateChatOrderAndGroup(
        &self,
        updatedHistories: Vec<ChatHistory>,
    ) -> ChatHistoryManagerResult<()> {
        let timestamp = currentTimeMillis();
        let entitiesToUpdate = updatedHistories
            .into_iter()
            .map(|history| {
                if let Some(mut originalEntity) = self.chatDao.getChatById(&history.id)? {
                    originalEntity.displayOrder = history.displayOrder;
                    originalEntity.group = history.group;
                    originalEntity.updatedAt = timestamp;
                    Ok(originalEntity)
                } else {
                    Ok(ChatEntity::fromChatHistory(&ChatHistory {
                        updatedAt: timestamp.to_string(),
                        ..history
                    }))
                }
            })
            .collect::<Result<Vec<_>, ChatHistoryManagerError>>()?;
        let updatedIds = entitiesToUpdate
            .iter()
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        self.chatDao.updateChats(entitiesToUpdate)?;
        for chatId in updatedIds {
            self.recordChatMetadata(&chatId)?;
        }
        Ok(())
    }

    /// Renames a chat group within an optional character-card scope.
    pub fn updateGroupName(
        &self,
        oldName: String,
        newName: String,
        characterCardName: Option<String>,
    ) -> ChatHistoryManagerResult<()> {
        match characterCardName {
            Some(characterCardName) => {
                self.chatDao
                    .updateGroupNameForCharacter(&oldName, &newName, &characterCardName)?;
            }
            None => self.chatDao.updateGroupName(&oldName, &newName)?,
        }
        Ok(())
    }

    /// Deletes a chat group or clears matching group assignments.
    pub fn deleteGroup(
        &self,
        groupName: String,
        deleteChats: bool,
        characterCardName: Option<String>,
    ) -> ChatHistoryManagerResult<()> {
        let timestamp = currentTimeMillis();
        if deleteChats {
            match characterCardName {
                Some(characterCardName) => {
                    self.chatDao
                        .deleteChatsInGroupForCharacter(&groupName, &characterCardName)?;
                }
                None => self.chatDao.deleteChatsInGroup(&groupName)?,
            }
        } else {
            match characterCardName {
                Some(characterCardName) => {
                    self.chatDao.removeGroupFromChatsForCharacter(
                        &groupName,
                        &characterCardName,
                        timestamp,
                    )?;
                }
                None => self.chatDao.removeGroupFromChats(&groupName, timestamp)?,
            }
        }
        Ok(())
    }

    /// Deletes one message and records sync deletion state.
    pub fn deleteMessage(&self, chatId: String, timestamp: i64) -> ChatHistoryManagerResult<()> {
        self.messageVariantDao
            .deleteVariantsForMessage(&chatId, timestamp)?;
        self.messageDao
            .deleteMessageByTimestamp(&chatId, timestamp)?;
        self.touchChatMetadata(&chatId)?;
        self.recordMessageDeletion(&chatId, timestamp)?;
        Ok(())
    }

    /// Deletes one message variant and keeps selected variant state valid.
    pub fn deleteMessageVariant(
        &self,
        chatId: String,
        messageTimestamp: i64,
        variantIndex: i32,
    ) -> ChatHistoryManagerResult<()> {
        let baseMessage = self
            .messageDao
            .getMessageByTimestamp(&chatId, messageTimestamp)?
            .ok_or_else(|| {
                ChatHistoryManagerError::IllegalArgument(format!(
                    "Message {messageTimestamp} does not exist in chat {chatId}"
                ))
            })?;
        if baseMessage.sender != "ai" {
            return Err(ChatHistoryManagerError::IllegalArgument(
                "Only AI messages can have variants".to_string(),
            ));
        }
        if variantIndex == 0 {
            return Err(ChatHistoryManagerError::IllegalArgument(
                "Cannot delete base variant with deleteMessageVariant".to_string(),
            ));
        }
        self.messageVariantDao
            .deleteVariant(&chatId, messageTimestamp, variantIndex)?;
        if baseMessage.selectedVariantIndex == variantIndex {
            self.messageDao
                .updateSelectedVariantIndex(&chatId, messageTimestamp, 0)?;
        }
        self.touchChatMetadata(&chatId)?;
        self.recordMessageSnapshot(&chatId, messageTimestamp)?;
        Ok(())
    }

    /// Updates message content and selected variant metadata.
    pub fn updateMessage(
        &self,
        chatId: String,
        message: ChatMessage,
    ) -> ChatHistoryManagerResult<()> {
        let existingMessage = self
            .messageDao
            .getMessageByTimestamp(&chatId, message.timestamp)?;

        if let Some(existingMessage) = existingMessage {
            if message.selectedVariantIndex > 0 {
                let messageTimestamp = message.timestamp;
                let existingVariant = self
                    .messageVariantDao
                    .getVariantForMessage(&chatId, message.timestamp, message.selectedVariantIndex)?
                    .ok_or_else(|| {
                        ChatHistoryManagerError::IllegalState(format!(
                            "Missing variant {} for message {}",
                            message.selectedVariantIndex, message.timestamp
                        ))
                    })?;
                let selectedVariantIndex = message.selectedVariantIndex;
                self.messageVariantDao
                    .updateVariant(MessageVariantEntity::fromChatMessage(
                        chatId.clone(),
                        message.timestamp,
                        selectedVariantIndex,
                        message,
                        existingVariant.variantId,
                    ))?;
                self.messageDao.updateSelectedVariantIndex(
                    &chatId,
                    existingMessage.timestamp,
                    selectedVariantIndex,
                )?;
                self.touchChatMetadata(&chatId)?;
                self.recordMessageSnapshot(&chatId, messageTimestamp)?;
                return Ok(());
            }

            let messageTimestamp = message.timestamp;
            let shouldUpdateChatMetadata = message.contentStream.is_none()
                || (existingMessage.content.is_empty() && !message.content.is_empty());
            let updatedMessageEntity = MessageEntity::fromChatMessage(
                chatId.clone(),
                message,
                existingMessage.orderIndex,
                existingMessage.messageId,
            );
            self.messageDao.updateMessage(updatedMessageEntity)?;
            if shouldUpdateChatMetadata {
                self.touchChatMetadata(&chatId)?;
            }
            self.recordMessageSnapshot(&chatId, messageTimestamp)?;
        } else {
            let messageTimestamp = message.timestamp;
            let messageEntity = MessageEntity::fromChatMessage(chatId.clone(), message, 0, 0);
            self.messageDao.insertMessage(messageEntity)?;
            self.touchChatMetadata(&chatId)?;
            self.recordMessageSnapshot(&chatId, messageTimestamp)?;
        }
        Ok(())
    }

    /// Updates favorite state for a message and its selected variant.
    pub fn setMessageFavorite(
        &self,
        chatId: String,
        timestamp: i64,
        isFavorite: bool,
    ) -> ChatHistoryManagerResult<()> {
        let existingMessage = self.messageDao.getMessageByTimestamp(&chatId, timestamp)?;
        if let Some(existingMessage) = existingMessage {
            if existingMessage.isFavorite != isFavorite {
                self.messageDao
                    .updateMessageFavorite(&chatId, timestamp, isFavorite)?;
                self.recordMessageSnapshot(&chatId, timestamp)?;
            }
        }
        Ok(())
    }

    /// Adds an alternate AI message variant.
    pub fn addMessageVariant(
        &self,
        chatId: String,
        messageTimestamp: i64,
        message: ChatMessage,
    ) -> ChatHistoryManagerResult<i32> {
        let baseMessage = self
            .messageDao
            .getMessageByTimestamp(&chatId, messageTimestamp)?
            .ok_or_else(|| {
                ChatHistoryManagerError::IllegalArgument(format!(
                    "Message {messageTimestamp} does not exist in chat {chatId}"
                ))
            })?;
        if baseMessage.sender != "ai" {
            return Err(ChatHistoryManagerError::IllegalArgument(
                "Only AI messages can have regenerated variants".to_string(),
            ));
        }
        let nextVariantIndex = self
            .messageVariantDao
            .getVariantsForMessage(&chatId, messageTimestamp)?
            .len() as i32
            + 1;
        self.messageVariantDao
            .insertVariant(MessageVariantEntity::fromChatMessage(
                chatId.clone(),
                messageTimestamp,
                nextVariantIndex,
                ChatMessage {
                    selectedVariantIndex: nextVariantIndex,
                    variantCount: 1,
                    ..message
                },
                0,
            ))?;
        self.messageDao
            .updateSelectedVariantIndex(&chatId, messageTimestamp, nextVariantIndex)?;
        self.touchChatMetadata(&chatId)?;
        self.recordMessageSnapshot(&chatId, messageTimestamp)?;
        Ok(nextVariantIndex)
    }

    /// Selects the active variant for one message.
    pub fn selectMessageVariant(
        &self,
        chatId: String,
        messageTimestamp: i64,
        selectedVariantIndex: i32,
    ) -> ChatHistoryManagerResult<()> {
        self.messageDao
            .getMessageByTimestamp(&chatId, messageTimestamp)?
            .ok_or_else(|| {
                ChatHistoryManagerError::IllegalArgument(format!(
                    "Message {messageTimestamp} does not exist in chat {chatId}"
                ))
            })?;
        if selectedVariantIndex > 0 {
            self.messageVariantDao
                .getVariantForMessage(&chatId, messageTimestamp, selectedVariantIndex)?
                .ok_or_else(|| {
                    ChatHistoryManagerError::IllegalArgument(format!(
                        "Variant {selectedVariantIndex} does not exist for message {messageTimestamp}"
                    ))
                })?;
        }
        self.messageDao.updateSelectedVariantIndex(
            &chatId,
            messageTimestamp,
            selectedVariantIndex,
        )?;
        self.recordMessageSnapshot(&chatId, messageTimestamp)?;
        Ok(())
    }

    /// Deletes messages from a timestamp onward and records sync deletions.
    pub fn deleteMessagesFrom(
        &self,
        chatId: String,
        timestamp: i64,
    ) -> ChatHistoryManagerResult<()> {
        self.messageVariantDao
            .deleteVariantsFrom(&chatId, timestamp)?;
        self.messageDao.deleteMessagesFrom(&chatId, timestamp)?;
        self.touchChatMetadata(&chatId)?;
        self.recordMessagesFromDeletion(&chatId, timestamp)?;
        Ok(())
    }

    /// Deletes every message and variant in a chat.
    pub fn clearChatMessages(&self, chatId: String) -> ChatHistoryManagerResult<()> {
        self.messageVariantDao.deleteAllVariantsForChat(&chatId)?;
        self.messageDao.deleteAllMessagesForChat(&chatId)?;
        self.touchChatMetadata(&chatId)?;
        self.recordAllMessagesForChatDeletion(&chatId)?;
        Ok(())
    }

    /// Updates a chat title and records chat metadata for sync.
    pub fn updateChatTitle(&self, chatId: String, title: String) -> ChatHistoryManagerResult<()> {
        self.chatDao
            .updateChatTitle(&chatId, title, currentTimeMillis())?;
        self.recordChatMetadata(&chatId)?;
        Ok(())
    }

    /// Updates the character-card binding stored on a chat.
    pub fn updateChatCharacterCardName(
        &self,
        chatId: String,
        characterCardName: Option<String>,
    ) -> ChatHistoryManagerResult<()> {
        self.chatDao.updateChatCharacterCardName(
            &chatId,
            characterCardName,
            currentTimeMillis(),
        )?;
        self.recordChatMetadata(&chatId)?;
        Ok(())
    }

    /// Updates cached token counts for a chat.
    pub fn updateChatTokenCounts(
        &self,
        chatId: String,
        inputTokens: i32,
        outputTokens: i32,
        currentWindowSize: i32,
    ) -> ChatHistoryManagerResult<()> {
        if let Some(chat) = self.chatDao.getChatById(&chatId)? {
            self.chatDao.updateChatMetadata(
                &chatId,
                chat.title,
                currentTimeMillis(),
                inputTokens,
                outputTokens,
                currentWindowSize,
            )?;
            self.recordChatMetadata(&chatId)?;
        }
        Ok(())
    }

    /// Persists the active chat id.
    pub fn setCurrentChatId(&self, chatId: String) -> ChatHistoryManagerResult<()> {
        self.currentChatIdDataStore.edit(|preferences| {
            preferences.set(&PreferencesKeys::CURRENT_CHAT_ID(), chatId);
        })?;
        Ok(())
    }

    /// Clears the persisted active chat id.
    pub fn clearCurrentChatId(&self) -> ChatHistoryManagerResult<()> {
        self.currentChatIdDataStore.edit(|preferences| {
            preferences.remove(&PreferencesKeys::CURRENT_CHAT_ID());
        })?;
        Ok(())
    }

    /// Returns whether a chat id exists.
    pub fn chatExists(&self, chatId: String) -> ChatHistoryManagerResult<bool> {
        Ok(self.chatDao.getChatById(&chatId)?.is_some())
    }

    /// Returns whether chat deletion is currently allowed.
    pub fn canDeleteChatHistory(&self, chatId: String) -> ChatHistoryManagerResult<bool> {
        Ok(self
            .chatDao
            .getChatById(&chatId)?
            .map(|chat| !chat.locked)
            .unwrap_or(false))
    }

    /// Deletes a chat when deletion policy allows it.
    pub fn deleteChatHistory(&self, chatId: String) -> ChatHistoryManagerResult<bool> {
        let chat = self.chatDao.getChatById(&chatId)?;
        if chat.as_ref().map(|chat| chat.locked).unwrap_or(false) {
            return Ok(false);
        }
        self.chatDao.deleteChat(&chatId)?;
        if chat.is_some() {
            self.recordChatDeletion(&chatId)?;
        }
        if self.currentChatIdFlow()?.as_deref() == Some(chatId.as_str()) {
            self.clearCurrentChatId()?;
        }
        Ok(chat.is_some())
    }

    /// Creates a new chat metadata row and optionally selects it.
    pub fn createNewChat(
        &self,
        title: Option<String>,
        group: Option<String>,
        characterCardName: Option<String>,
        characterGroupId: Option<String>,
    ) -> ChatHistoryManagerResult<ChatHistory> {
        let timestamp = currentTimeMillis();
        let finalTitle = title.unwrap_or_else(|| "New Chat".to_string());
        let chatEntity = ChatEntity {
            id: Uuid::new_v4().to_string(),
            title: finalTitle,
            createdAt: timestamp,
            updatedAt: timestamp,
            inputTokens: 0,
            outputTokens: 0,
            currentWindowSize: 0,
            group,
            displayOrder: -timestamp,
            workspace: None,
            parentChatId: None,
            characterCardName,
            characterGroupId,
            locked: false,
            pinned: false,
        };
        self.chatDao.insertChat(chatEntity.clone())?;
        let history = chatEntity.toChatHistory(Vec::new());
        self.setCurrentChatId(history.id.clone())?;
        self.recordChatMetadata(&history.id)?;
        Ok(history)
    }

    /// Updates the workspace binding for a chat.
    pub fn updateChatWorkspace(
        &self,
        chatId: String,
        workspace: Option<String>,
    ) -> ChatHistoryManagerResult<()> {
        self.chatDao
            .updateChatWorkspace(&chatId, workspace, currentTimeMillis())?;
        self.recordChatMetadata(&chatId)?;
        Ok(())
    }

    /// Updates the group name assigned to a chat.
    pub fn updateChatGroup(
        &self,
        chatId: String,
        group: Option<String>,
    ) -> ChatHistoryManagerResult<()> {
        self.chatDao
            .updateChatGroup(&chatId, group, currentTimeMillis())?;
        self.recordChatMetadata(&chatId)?;
        Ok(())
    }

    /// Loads the title for a chat id.
    pub fn getChatTitle(&self, chatId: String) -> ChatHistoryManagerResult<Option<String>> {
        Ok(self.chatDao.getChatById(&chatId)?.map(|chat| chat.title))
    }

    /// Loads all hydrated messages for one chat.
    pub fn loadChatMessages(&self, chatId: &str) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        let messageEntities = self.messageDao.getMessagesForChat(chatId)?;
        self.hydrateMessagesForChat(chatId, messageEntities)
    }

    /// Loads hydrated messages with optional summary-message filtering.
    pub fn loadChatMessagesWithOptions(
        &self,
        chatId: String,
        order: Option<String>,
        limit: Option<i32>,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        let normalizedOrder = order.map(|order| order.trim().to_lowercase());
        let effectiveLimit = limit.map(|limit| limit.max(1));
        let messageEntities = match normalizedOrder.as_deref() {
            Some("desc") => {
                if let Some(limit) = effectiveLimit {
                    self.messageDao.getMessagesForChatDesc(&chatId, limit)?
                } else {
                    let mut messages = self.messageDao.getMessagesForChat(&chatId)?;
                    messages.reverse();
                    messages
                }
            }
            _ => {
                if let Some(limit) = effectiveLimit {
                    self.messageDao.getMessagesForChatAsc(&chatId, limit)?
                } else {
                    self.messageDao.getMessagesForChat(&chatId)?
                }
            }
        };
        self.hydrateMessagesForChat(&chatId, messageEntities)
    }

    /// Searches message content and returns matching chat ids.
    pub fn searchChatIdsByContent(
        &self,
        query: String,
    ) -> ChatHistoryManagerResult<HashSet<String>> {
        if query.trim().is_empty() {
            return Ok(HashSet::new());
        }
        let escapedQuery = query
            .trim()
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        Ok(self
            .messageDao
            .searchChatIdsByContent(&escapedQuery)?
            .into_iter()
            .collect())
    }

    /// Creates a branch chat from a parent chat at a message timestamp.
    pub fn createBranch(
        &self,
        parentChatId: String,
        upToMessageTimestamp: Option<i64>,
    ) -> ChatHistoryManagerResult<ChatHistory> {
        let parentChat = self.chatDao.getChatById(&parentChatId)?.ok_or_else(|| {
            ChatHistoryManagerError::IllegalArgument(format!(
                "Parent chat {parentChatId} does not exist"
            ))
        })?;
        let timestamp = currentTimeMillis();
        let branchEntity = ChatEntity {
            id: Uuid::new_v4().to_string(),
            title: parentChat.title,
            createdAt: timestamp,
            updatedAt: timestamp,
            inputTokens: parentChat.inputTokens,
            outputTokens: parentChat.outputTokens,
            currentWindowSize: parentChat.currentWindowSize,
            group: parentChat.group,
            displayOrder: -timestamp,
            workspace: parentChat.workspace,
            parentChatId: Some(parentChatId.clone()),
            characterCardName: parentChat.characterCardName,
            characterGroupId: parentChat.characterGroupId,
            locked: false,
            pinned: false,
        };
        self.chatDao.insertChat(branchEntity.clone())?;
        let copiedMessageCount = self
            .messageDao
            .countMessagesForChatUpToTimestamp(&parentChatId, upToMessageTimestamp)?;
        if copiedMessageCount > 0 {
            self.messageDao.copyMessagesToChat(
                &parentChatId,
                &branchEntity.id,
                upToMessageTimestamp,
            )?;
            self.messageVariantDao.copyVariantsToChat(
                &parentChatId,
                &branchEntity.id,
                upToMessageTimestamp,
            )?;
        }
        let branchHistory = branchEntity.toChatHistory(Vec::new());
        self.setCurrentChatId(branchHistory.id.clone())?;
        self.recordChatSnapshot(&branchHistory.id)?;
        Ok(branchHistory)
    }

    /// Loads branch chats for a parent chat id.
    pub fn getBranches(&self, parentChatId: String) -> ChatHistoryManagerResult<Vec<ChatHistory>> {
        Ok(self
            .chatDao
            .getBranchesByParentId(&parentChatId)?
            .into_iter()
            .map(|entity| entity.toChatHistory(Vec::new()))
            .collect())
    }

    /// Reads branch chat histories for a parent chat id.
    pub fn getBranchesFlow(
        &self,
        parentChatId: String,
    ) -> ChatHistoryManagerResult<Vec<ChatHistory>> {
        self.getBranches(parentChatId)
    }

    /// Clears a character-card binding from matching chats.
    pub fn clearCharacterCardBinding(
        &self,
        characterCardName: String,
    ) -> ChatHistoryManagerResult<()> {
        self.chatDao
            .clearCharacterCardBinding(&characterCardName, currentTimeMillis())?;
        Ok(())
    }

    /// Reassigns chats from one character card binding to another.
    pub fn reassignChatsToCharacterCard(
        &self,
        sourceCharacterCardName: Option<String>,
        targetCharacterCardName: String,
    ) -> ChatHistoryManagerResult<i32> {
        let updated = if let Some(sourceCharacterCardName) = sourceCharacterCardName {
            self.chatDao.renameCharacterCardBinding(
                &sourceCharacterCardName,
                &targetCharacterCardName,
                currentTimeMillis(),
            )?
        } else {
            self.chatDao
                .assignCharacterCardToUnbound(&targetCharacterCardName, currentTimeMillis())?
        };
        Ok(updated)
    }

    /// Returns the latest summary-message timestamp in a chat.
    pub fn getLatestSummaryTimestamp(
        &self,
        chatId: String,
    ) -> ChatHistoryManagerResult<Option<i64>> {
        Ok(self.messageDao.getLatestSummaryTimestamp(&chatId)?)
    }

    /// Loads non-summary messages after the latest summary in a timestamp range.
    pub fn loadMessagesAfterLatestSummaryInRange(
        &self,
        chatId: String,
        beforeTimestampExclusive: Option<i64>,
        upToTimestampInclusive: Option<i64>,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        let latestSummaryTimestamp =
            if let Some(beforeTimestampExclusive) = beforeTimestampExclusive {
                self.messageDao
                    .getLatestSummaryTimestampBefore(&chatId, beforeTimestampExclusive)?
            } else if let Some(upToTimestampInclusive) = upToTimestampInclusive {
                self.messageDao
                    .getLatestSummaryTimestampUpTo(&chatId, upToTimestampInclusive)?
            } else {
                self.messageDao.getLatestSummaryTimestamp(&chatId)?
            };
        let messageEntities = self.messageDao.getMessagesForChatInRangeAsc(
            &chatId,
            latestSummaryTimestamp,
            beforeTimestampExclusive,
            upToTimestampInclusive,
        )?;
        self.hydrateMessagesForChat(&chatId, messageEntities)
    }

    /// Returns whether a chat contains at least one user message.
    pub fn hasUserMessage(&self, chatId: String) -> ChatHistoryManagerResult<bool> {
        Ok(self.messageDao.existsUserMessage(&chatId)?)
    }

    /// Loads runtime-visible messages for model context.
    pub fn loadRuntimeChatMessages(
        &self,
        chatId: String,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        let latestSummaryTimestamp = self.messageDao.getLatestSummaryTimestamp(&chatId)?;
        let messageEntities = if let Some(latestSummaryTimestamp) = latestSummaryTimestamp {
            self.messageDao
                .getMessagesForChatFromTimestampAsc(&chatId, latestSummaryTimestamp)?
        } else {
            self.messageDao.getMessagesForChat(&chatId)?
        };
        self.hydrateMessagesForChat(&chatId, messageEntities)
    }

    /// Loads lightweight message previews for locator search UI.
    pub fn loadChatMessageLocatorPreviews(
        &self,
        chatId: String,
        query: String,
    ) -> ChatHistoryManagerResult<Vec<ChatMessageLocatorPreview>> {
        if !query.trim().is_empty() {
            return Ok(self.messageDao.searchLocatorPreviewsForChat(
                &chatId,
                query.trim(),
                LOCATOR_PREVIEW_CHAR_COUNT,
            )?);
        }

        Ok(self
            .messageDao
            .getLocatorPreviewsForChat(&chatId, LOCATOR_PREVIEW_CHAR_COUNT)?)
    }

    /// Loads hydrated messages starting at a timestamp.
    pub fn loadChatMessagesFromTimestamp(
        &self,
        chatId: String,
        startTimestampInclusive: i64,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        let messageEntities = self
            .messageDao
            .getMessagesForChatFromTimestampAsc(&chatId, startTimestampInclusive)?;
        self.hydrateMessagesForChat(&chatId, messageEntities)
    }

    /// Loads hydrated messages inside an inclusive timestamp window.
    pub fn loadChatMessagesWindow(
        &self,
        chatId: String,
        startTimestampInclusive: i64,
        endTimestampInclusive: i64,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        let messageEntities = self.messageDao.getMessagesForChatWindowAsc(
            &chatId,
            startTimestampInclusive,
            endTimestampInclusive,
        )?;
        self.hydrateMessagesForChat(&chatId, messageEntities)
    }

    /// Loads hydrated messages after a timestamp in ascending order.
    pub fn loadChatMessagesAscAfter(
        &self,
        chatId: String,
        afterTimestampExclusive: i64,
        limit: i32,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        let messageEntities = self
            .messageDao
            .getMessagesForChatAfterTimestampExclusiveAsc(
                &chatId,
                afterTimestampExclusive,
                limit,
            )?;
        self.hydrateMessagesForChat(&chatId, messageEntities)
    }

    /// Loads older messages before a timestamp for paged history views.
    pub fn loadOlderChatMessages(
        &self,
        chatId: String,
        beforeTimestampExclusive: i64,
        limit: i32,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        let mut messageEntities = self
            .messageDao
            .getMessagesForChatBeforeTimestampExclusiveDesc(
                &chatId,
                beforeTimestampExclusive,
                limit,
            )?;
        messageEntities.reverse();
        self.hydrateMessagesForChat(&chatId, messageEntities)
    }

    /// Returns whether a chat has messages before a timestamp.
    pub fn hasMessagesBefore(
        &self,
        chatId: String,
        beforeTimestampExclusive: i64,
    ) -> ChatHistoryManagerResult<bool> {
        Ok(self
            .messageDao
            .existsMessagesBeforeTimestamp(&chatId, beforeTimestampExclusive)?)
    }

    /// Returns whether a chat has messages after a timestamp.
    pub fn hasMessagesAfter(
        &self,
        chatId: String,
        afterTimestampExclusive: i64,
    ) -> ChatHistoryManagerResult<bool> {
        Ok(self
            .messageDao
            .existsMessagesAfterTimestamp(&chatId, afterTimestampExclusive)?)
    }

    /// Loads messages in descending timestamp order.
    pub fn loadChatMessagesDesc(
        &self,
        chatId: String,
        limit: i32,
        beforeTimestampExclusive: Option<i64>,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        let messageEntities = if let Some(beforeTimestampExclusive) = beforeTimestampExclusive {
            self.messageDao
                .getMessagesForChatBeforeTimestampExclusiveDesc(
                    &chatId,
                    beforeTimestampExclusive,
                    limit,
                )?
        } else {
            self.messageDao.getMessagesForChatDesc(&chatId, limit)?
        };
        self.hydrateMessagesForChat(&chatId, messageEntities)
    }

    /// Loads messages in descending order up to an inclusive timestamp.
    pub fn loadChatMessagesDescUpTo(
        &self,
        chatId: String,
        maxTimestampInclusive: i64,
        limit: i32,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        let messageEntities = self.messageDao.getMessagesForChatBeforeTimestampDesc(
            &chatId,
            maxTimestampInclusive,
            limit,
        )?;
        self.hydrateMessagesForChat(&chatId, messageEntities)
    }

    /// Reassigns chats from one character group to another.
    pub fn reassignChatsToCharacterGroup(
        &self,
        sourceCharacterGroupId: Option<String>,
        targetCharacterGroupId: String,
    ) -> ChatHistoryManagerResult<i32> {
        let updated = if let Some(sourceCharacterGroupId) = sourceCharacterGroupId {
            self.chatDao.renameCharacterGroupBinding(
                &sourceCharacterGroupId,
                &targetCharacterGroupId,
                currentTimeMillis(),
            )?
        } else {
            self.chatDao
                .assignCharacterGroupToUnbound(&targetCharacterGroupId, currentTimeMillis())?
        };
        Ok(updated)
    }

    /// Updates the character group binding stored on a chat.
    pub fn updateChatCharacterGroupId(
        &self,
        chatId: String,
        characterGroupId: Option<String>,
    ) -> ChatHistoryManagerResult<()> {
        self.chatDao
            .updateChatCharacterGroupId(&chatId, characterGroupId, currentTimeMillis())?;
        self.recordChatMetadata(&chatId)?;
        Ok(())
    }

    /// Updates character card and group bindings for a chat together.
    pub fn updateChatCharacterBinding(
        &self,
        chatId: String,
        characterCardName: Option<String>,
        characterGroupId: Option<String>,
    ) -> ChatHistoryManagerResult<()> {
        self.chatDao.updateChatCharacterBinding(
            &chatId,
            characterCardName,
            characterGroupId,
            currentTimeMillis(),
        )?;
        self.recordChatMetadata(&chatId)?;
        Ok(())
    }

    /// Clears a character group binding from matching chats.
    pub fn clearCharacterGroupBinding(
        &self,
        characterGroupId: String,
    ) -> ChatHistoryManagerResult<i32> {
        Ok(self
            .chatDao
            .clearCharacterGroupBinding(&characterGroupId, currentTimeMillis())?)
    }

    /// Deletes chats bound to a character card and updates current chat state.
    pub fn deleteChatsByCharacterCardBinding(
        &self,
        sourceCharacterCardName: Option<String>,
    ) -> ChatHistoryManagerResult<i32> {
        let currentChatId = self.currentChatIdFlow()?;
        let currentChat = currentChatId
            .as_ref()
            .map(|chatId| self.chatDao.getChatById(chatId))
            .transpose()?
            .flatten();
        let deletedCount = if let Some(sourceCharacterCardName) = sourceCharacterCardName.clone() {
            self.chatDao
                .deleteUnlockedChatsByCharacterCardName(&sourceCharacterCardName)?
        } else {
            self.chatDao.deleteUnlockedUnboundChats()?
        };
        let currentChatShouldBeCleared = currentChat
            .map(|chat| {
                !chat.locked
                    && if let Some(sourceCharacterCardName) = sourceCharacterCardName {
                        chat.characterCardName == Some(sourceCharacterCardName)
                    } else {
                        chat.characterCardName.is_none() && chat.characterGroupId.is_none()
                    }
            })
            .unwrap_or(false);
        if currentChatShouldBeCleared {
            self.clearCurrentChatId()?;
        }
        Ok(deletedCount)
    }

    /// Assigns a character-card binding to a set of chats.
    pub fn assignCharacterCardToChats(
        &self,
        chatIds: Vec<String>,
        targetCharacterCardName: Option<String>,
    ) -> ChatHistoryManagerResult<i32> {
        if chatIds.is_empty() {
            return Ok(0);
        }
        Ok(self.chatDao.updateCharacterCardForChats(
            chatIds,
            targetCharacterCardName,
            currentTimeMillis(),
        )?)
    }

    /// Assigns a character-group binding to a set of chats.
    pub fn assignCharacterGroupToChats(
        &self,
        chatIds: Vec<String>,
        targetCharacterGroupId: Option<String>,
    ) -> ChatHistoryManagerResult<i32> {
        if chatIds.is_empty() {
            return Ok(0);
        }
        Ok(self.chatDao.updateCharacterGroupForChats(
            chatIds,
            targetCharacterGroupId,
            currentTimeMillis(),
        )?)
    }

    /// Clears character-group bindings from a set of chats.
    pub fn clearCharacterGroupBindingForChats(
        &self,
        chatIds: Vec<String>,
    ) -> ChatHistoryManagerResult<i32> {
        if chatIds.is_empty() {
            return Ok(0);
        }
        Ok(self
            .chatDao
            .clearCharacterGroupForChats(chatIds, currentTimeMillis())?)
    }

    /// Assigns a chat-list group name to a set of chats.
    pub fn assignGroupToChats(
        &self,
        chatIds: Vec<String>,
        groupName: Option<String>,
    ) -> ChatHistoryManagerResult<i32> {
        if chatIds.is_empty() {
            return Ok(0);
        }
        Ok(self
            .chatDao
            .updateGroupForChats(chatIds, groupName, currentTimeMillis())?)
    }

    /// Renames character-card references stored on chats.
    pub fn renameCharacterCardInChats(
        &self,
        oldName: String,
        newName: String,
    ) -> ChatHistoryManagerResult<i32> {
        Ok(self
            .chatDao
            .renameCharacterCardBinding(&oldName, &newName, currentTimeMillis())?)
    }

    /// Renames role names stored in message rows.
    pub fn renameRoleNameInMessages(
        &self,
        oldName: String,
        newName: String,
    ) -> ChatHistoryManagerResult<i32> {
        Ok(self.messageDao.renameRoleName(&oldName, &newName)?)
    }

    fn touchChatMetadata(&self, chatId: &str) -> ChatHistoryManagerResult<()> {
        if let Some(chat) = self.chatDao.getChatById(chatId)? {
            self.chatDao.updateChatMetadata(
                chatId,
                chat.title,
                currentTimeMillis(),
                chat.inputTokens,
                chat.outputTokens,
                chat.currentWindowSize,
            )?;
        }
        Ok(())
    }

    fn recordChatMetadata(&self, chatId: &str) -> ChatHistoryManagerResult<()> {
        self.syncStore.recordChatMetadata(chatId)?;
        Ok(())
    }

    fn recordChatSnapshot(&self, chatId: &str) -> ChatHistoryManagerResult<()> {
        self.syncStore.recordChatSnapshot(chatId)?;
        Ok(())
    }

    fn recordMessageSnapshot(&self, chatId: &str, timestamp: i64) -> ChatHistoryManagerResult<()> {
        self.syncStore.recordMessageSnapshot(chatId, timestamp)?;
        Ok(())
    }

    fn recordChatDeletion(&self, chatId: &str) -> ChatHistoryManagerResult<()> {
        self.syncStore.recordChatDeletion(chatId)?;
        Ok(())
    }

    fn recordMessageDeletion(&self, chatId: &str, timestamp: i64) -> ChatHistoryManagerResult<()> {
        self.syncStore.recordMessageDeletion(chatId, timestamp)?;
        Ok(())
    }

    fn recordMessagesFromDeletion(
        &self,
        chatId: &str,
        timestamp: i64,
    ) -> ChatHistoryManagerResult<()> {
        self.syncStore
            .recordMessagesFromDeletion(chatId, timestamp)?;
        Ok(())
    }

    fn recordAllMessagesForChatDeletion(&self, chatId: &str) -> ChatHistoryManagerResult<()> {
        self.syncStore.recordAllMessagesForChatDeletion(chatId)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
/// Counts chat import outcomes by operation type.
pub struct ChatImportResult {
    pub new: i32,
    pub updated: i32,
    pub skipped: i32,
}

impl ChatImportResult {
    /// Returns the total number of imported, updated, and skipped chats.
    pub fn total(&self) -> i32 {
        self.new + self.updated
    }
}

fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}
