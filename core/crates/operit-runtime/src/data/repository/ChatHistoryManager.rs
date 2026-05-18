use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use operit_store::PreferencesDataStore::{
    stringPreferencesKey, CoroutineScope, PreferencesDataStore, PreferencesDataStoreError,
    PreferencesKey, SharingStarted, StateFlow,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use thiserror::Error;
use uuid::Uuid;

use crate::data::dao::ChatDao::ChatDao;
use crate::data::dao::MessageDao::MessageDao;
use crate::data::dao::MessageVariantDao::MessageVariantDao;
use crate::data::db::AppDatabase::{AppDatabase, AppDatabaseError};
use crate::data::model::CharacterCardChatStats::CharacterCardChatStats;
use crate::data::model::CharacterGroupChatStats::CharacterGroupChatStats;
use crate::data::model::ChatEntity::ChatEntity;
use crate::data::model::ChatHistory::ChatHistory;
use crate::data::model::ChatMessage::ChatMessage;
use crate::data::model::ChatMessageLocatorPreview::ChatMessageLocatorPreview;
use crate::data::model::MessageEntity::MessageEntity;
use crate::data::model::MessageVariantEntity::MessageVariantEntity;

const LOCATOR_PREVIEW_CHAR_COUNT: i32 = 48;

#[derive(Debug, Error)]
pub enum ChatHistoryManagerError {
    #[error(transparent)]
    Database(#[from] AppDatabaseError),
    #[error(transparent)]
    Store(#[from] operit_store::SqliteStore::SqliteStoreError),
    #[error(transparent)]
    Preferences(#[from] PreferencesDataStoreError),
    #[error("{0}")]
    IllegalArgument(String),
    #[error("{0}")]
    IllegalState(String),
}

pub type ChatHistoryManagerResult<T> = Result<T, ChatHistoryManagerError>;

pub struct ChatHistoryManager {
    database: Arc<AppDatabase>,
    chatDao: ChatDao,
    messageDao: MessageDao,
    messageVariantDao: MessageVariantDao,
    currentChatIdDataStore: PreferencesDataStore,
    pub currentChatIdFlow: StateFlow<Option<String>>,
}

pub struct PreferencesKeys;

impl PreferencesKeys {
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
    pub fn getInstance(paths: RuntimeStorePaths) -> ChatHistoryManagerResult<Self> {
        Self::new(paths)
    }

    pub fn default() -> ChatHistoryManagerResult<Self> {
        Self::new(RuntimeStorePaths::default())
    }

    pub fn new(paths: RuntimeStorePaths) -> ChatHistoryManagerResult<Self> {
        let currentChatIdDataStore = PreferencesDataStore::new(
            paths.root_dir().join("current_chat_id.preferences.json"),
        );
        let database = AppDatabase::getDatabase(paths)?;
        let chatDao = database.chatDao();
        let messageDao = database.messageDao();
        let messageVariantDao = database.messageVariantDao();
        let currentChatIdFlow = currentChatIdDataStore
            .dataFlow()
            .catch(|exception| match exception {
                PreferencesDataStoreError::Io(error)
                    if error.kind() == std::io::ErrorKind::NotFound =>
                {
                    Ok(operit_store::PreferencesDataStore::emptyPreferences())
                }
                error => Err(error),
            })
            .map(|preferences| preferences.get(&PreferencesKeys::CURRENT_CHAT_ID()).cloned())
            .stateIn(CoroutineScope, SharingStarted::Lazily, None);
        Ok(Self {
            database,
            chatDao,
            messageDao,
            messageVariantDao,
            currentChatIdDataStore,
            currentChatIdFlow,
        })
    }

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

    fn loadDisplayHistory(&self, chatHistory: ChatHistory) -> ChatHistoryManagerResult<ChatHistory> {
        let messages = self.loadChatMessages(&chatHistory.id)?;
        Ok(ChatHistory { messages, ..chatHistory })
    }

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

    fn toChatHistory(&self, chatEntity: ChatEntity) -> ChatHistory {
        chatEntity.toChatHistory(Vec::new())
    }

    pub fn chatHistoriesFlow(&self) -> ChatHistoryManagerResult<Vec<ChatHistory>> {
        Ok(self
            .chatDao
            .getAllChats()?
            .into_iter()
            .map(|chat| self.toChatHistory(chat))
            .collect())
    }

    pub fn getTotalChatCount(&self) -> ChatHistoryManagerResult<i32> {
        Ok(self.chatDao.getTotalChatCount()?)
    }

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

    pub fn characterCardStatsFlow(&self) -> ChatHistoryManagerResult<Vec<CharacterCardChatStats>> {
        Ok(self.chatDao.getCharacterCardChatStats()?)
    }

    pub fn characterGroupStatsFlow(&self) -> ChatHistoryManagerResult<Vec<CharacterGroupChatStats>> {
        Ok(self.chatDao.getCharacterGroupChatStats()?)
    }

    pub fn getChatHistoriesByCharacterCard(
        &self,
        characterCardName: String,
        isDefault: bool,
    ) -> ChatHistoryManagerResult<Vec<ChatHistory>> {
        let chats = if isDefault {
            self.chatDao.getChatsByCharacterCardOrNull(&characterCardName)?
        } else {
            self.chatDao.getChatsByCharacterCard(&characterCardName)?
        };
        Ok(chats.into_iter().map(|chat| self.toChatHistory(chat)).collect())
    }

    pub fn currentChatIdFlow(&self) -> ChatHistoryManagerResult<Option<String>> {
        Ok(self.currentChatIdFlow.first()?)
    }

    fn saveChatHistoryInternal(&self, history: ChatHistory) -> ChatHistoryManagerResult<()> {
        let chatEntity = ChatEntity::fromChatHistory(&history);
        self.chatDao.insertChat(chatEntity.clone())?;
        self.messageDao.deleteAllMessagesForChat(&chatEntity.id)?;
        self.messageVariantDao.deleteAllVariantsForChat(&chatEntity.id)?;

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

    pub fn saveChatHistory(&self, history: ChatHistory) -> ChatHistoryManagerResult<()> {
        self.saveChatHistoryInternal(history)
    }

    pub fn updateChatLocked(&self, chatId: String, locked: bool) -> ChatHistoryManagerResult<()> {
        self.chatDao
            .updateChatLocked(&chatId, locked, currentTimeMillis())?;
        Ok(())
    }

    fn persistMessageLocked(
        &self,
        chatId: &str,
        messageToPersist: ChatMessage,
    ) -> ChatHistoryManagerResult<ChatMessage> {
        let messageEntity = MessageEntity::fromChatMessage(
            chatId.to_string(),
            messageToPersist.clone(),
            0,
            0,
        );
        self.messageDao.insertMessage(messageEntity)?;
        self.touchChatMetadata(chatId)?;
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
            return if hasAnyMessages { Ok(None) } else { Ok(Some(message)) };
        }

        let beforeMessage = if let Some(beforeTimestamp) = beforeTimestamp {
            self.messageDao.getMessageByTimestamp(chatId, beforeTimestamp)?
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
            self.messageDao.getMessageByTimestamp(chatId, afterTimestamp)?
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
                timestamp: actualBeforeTimestamp + (actualAfterTimestamp - actualBeforeTimestamp) / 2,
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

    pub fn addSummaryMessageBetweenSliceNeighbors(
        &self,
        chatId: String,
        message: ChatMessage,
        beforeTimestamp: Option<i64>,
        afterTimestamp: Option<i64>,
    ) -> ChatHistoryManagerResult<Option<ChatMessage>> {
        let beforeMessage = if let Some(beforeTimestamp) = beforeTimestamp {
            self.messageDao.getMessageByTimestamp(&chatId, beforeTimestamp)?
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
            self.messageDao.getMessageByTimestamp(&chatId, afterTimestamp)?
        } else {
            None
        };

        if beforeMessage.as_ref().map(|message| message.sender.as_str()) == Some("summary")
            || afterMessage.as_ref().map(|message| message.sender.as_str()) == Some("summary")
        {
            return Ok(None);
        }

        let messageToPersist = self.resolveAnchoredMessageLocked(
            &chatId,
            message,
            beforeTimestamp,
            afterTimestamp,
        )?;
        if let Some(messageToPersist) = messageToPersist {
            Ok(Some(self.persistMessageLocked(&chatId, messageToPersist)?))
        } else {
            Ok(None)
        }
    }

    pub fn addMessage(
        &self,
        chatId: String,
        message: ChatMessage,
    ) -> ChatHistoryManagerResult<ChatMessage> {
        self.persistMessageLocked(&chatId, message)
    }

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
        self.chatDao.updateChats(entitiesToUpdate)?;
        Ok(())
    }

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
                    self.chatDao
                        .removeGroupFromChatsForCharacter(&groupName, &characterCardName, timestamp)?;
                }
                None => self.chatDao.removeGroupFromChats(&groupName, timestamp)?,
            }
        }
        Ok(())
    }

    pub fn deleteMessage(&self, chatId: String, timestamp: i64) -> ChatHistoryManagerResult<()> {
        self.messageVariantDao
            .deleteVariantsForMessage(&chatId, timestamp)?;
        self.messageDao.deleteMessageByTimestamp(&chatId, timestamp)?;
        self.touchChatMetadata(&chatId)?;
        Ok(())
    }

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
        Ok(())
    }

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
                self.messageVariantDao.updateVariant(
                    MessageVariantEntity::fromChatMessage(
                        chatId.clone(),
                        message.timestamp,
                        selectedVariantIndex,
                        message,
                        existingVariant.variantId,
                    ),
                )?;
                self.messageDao.updateSelectedVariantIndex(
                    &chatId,
                    existingMessage.timestamp,
                    selectedVariantIndex,
                )?;
                self.touchChatMetadata(&chatId)?;
                return Ok(());
            }

            let updatedMessageEntity = MessageEntity::fromChatMessage(
                chatId.clone(),
                message,
                existingMessage.orderIndex,
                existingMessage.messageId,
            );
            self.messageDao.updateMessage(updatedMessageEntity)?;
            self.touchChatMetadata(&chatId)?;
        } else {
            let messageEntity = MessageEntity::fromChatMessage(chatId.clone(), message, 0, 0);
            self.messageDao.insertMessage(messageEntity)?;
            self.touchChatMetadata(&chatId)?;
        }
        Ok(())
    }

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
            }
        }
        Ok(())
    }

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
        let nextVariantIndex =
            self.messageVariantDao
                .getVariantsForMessage(&chatId, messageTimestamp)?
                .len() as i32
                + 1;
        self.messageVariantDao.insertVariant(
            MessageVariantEntity::fromChatMessage(
                chatId.clone(),
                messageTimestamp,
                nextVariantIndex,
                ChatMessage {
                    selectedVariantIndex: nextVariantIndex,
                    variantCount: 1,
                    ..message
                },
                0,
            ),
        )?;
        self.messageDao
            .updateSelectedVariantIndex(&chatId, messageTimestamp, nextVariantIndex)?;
        self.touchChatMetadata(&chatId)?;
        Ok(nextVariantIndex)
    }

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
        self.messageDao
            .updateSelectedVariantIndex(&chatId, messageTimestamp, selectedVariantIndex)?;
        Ok(())
    }

    pub fn deleteMessagesFrom(
        &self,
        chatId: String,
        timestamp: i64,
    ) -> ChatHistoryManagerResult<()> {
        self.messageVariantDao.deleteVariantsFrom(&chatId, timestamp)?;
        self.messageDao.deleteMessagesFrom(&chatId, timestamp)?;
        self.touchChatMetadata(&chatId)?;
        Ok(())
    }

    pub fn clearChatMessages(&self, chatId: String) -> ChatHistoryManagerResult<()> {
        self.messageVariantDao.deleteAllVariantsForChat(&chatId)?;
        self.messageDao.deleteAllMessagesForChat(&chatId)?;
        self.touchChatMetadata(&chatId)?;
        Ok(())
    }

    pub fn updateChatTitle(&self, chatId: String, title: String) -> ChatHistoryManagerResult<()> {
        self.chatDao
            .updateChatTitle(&chatId, title, currentTimeMillis())?;
        Ok(())
    }

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
        Ok(())
    }

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
        }
        Ok(())
    }

    pub fn setCurrentChatId(&self, chatId: String) -> ChatHistoryManagerResult<()> {
        self.currentChatIdDataStore.edit(|preferences| {
            preferences.set(&PreferencesKeys::CURRENT_CHAT_ID(), chatId);
        })?;
        Ok(())
    }

    pub fn clearCurrentChatId(&self) -> ChatHistoryManagerResult<()> {
        self.currentChatIdDataStore.edit(|preferences| {
            preferences.remove(&PreferencesKeys::CURRENT_CHAT_ID());
        })?;
        Ok(())
    }

    pub fn chatExists(&self, chatId: String) -> ChatHistoryManagerResult<bool> {
        Ok(self.chatDao.getChatById(&chatId)?.is_some())
    }

    pub fn canDeleteChatHistory(&self, chatId: String) -> ChatHistoryManagerResult<bool> {
        Ok(self
            .chatDao
            .getChatById(&chatId)?
            .map(|chat| !chat.locked)
            .unwrap_or(false))
    }

    pub fn deleteChatHistory(&self, chatId: String) -> ChatHistoryManagerResult<bool> {
        let chat = self.chatDao.getChatById(&chatId)?;
        if chat.as_ref().map(|chat| chat.locked).unwrap_or(false) {
            return Ok(false);
        }
        self.chatDao.deleteChat(&chatId)?;
        if self.currentChatIdFlow()?.as_deref() == Some(chatId.as_str()) {
            self.clearCurrentChatId()?;
        }
        Ok(chat.is_some())
    }

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
            workspaceEnv: None,
            parentChatId: None,
            characterCardName,
            characterGroupId,
            locked: false,
        };
        self.chatDao.insertChat(chatEntity.clone())?;
        let history = chatEntity.toChatHistory(Vec::new());
        self.setCurrentChatId(history.id.clone())?;
        Ok(history)
    }

    pub fn updateChatWorkspace(
        &self,
        chatId: String,
        workspace: Option<String>,
        workspaceEnv: Option<String>,
    ) -> ChatHistoryManagerResult<()> {
        self.chatDao
            .updateChatWorkspace(&chatId, workspace, workspaceEnv, currentTimeMillis())?;
        Ok(())
    }

    pub fn updateChatGroup(
        &self,
        chatId: String,
        group: Option<String>,
    ) -> ChatHistoryManagerResult<()> {
        self.chatDao
            .updateChatGroup(&chatId, group, currentTimeMillis())?;
        Ok(())
    }

    pub fn getChatTitle(&self, chatId: String) -> ChatHistoryManagerResult<Option<String>> {
        Ok(self.chatDao.getChatById(&chatId)?.map(|chat| chat.title))
    }

    pub fn loadChatMessages(&self, chatId: &str) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        let messageEntities = self.messageDao.getMessagesForChat(chatId)?;
        self.hydrateMessagesForChat(chatId, messageEntities)
    }

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

    pub fn createBranch(
        &self,
        parentChatId: String,
        upToMessageTimestamp: Option<i64>,
    ) -> ChatHistoryManagerResult<ChatHistory> {
        let parentChat = self
            .chatDao
            .getChatById(&parentChatId)?
            .ok_or_else(|| {
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
            workspaceEnv: parentChat.workspaceEnv,
            parentChatId: Some(parentChatId.clone()),
            characterCardName: parentChat.characterCardName,
            characterGroupId: parentChat.characterGroupId,
            locked: false,
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
        Ok(branchHistory)
    }

    pub fn getBranches(&self, parentChatId: String) -> ChatHistoryManagerResult<Vec<ChatHistory>> {
        Ok(self
            .chatDao
            .getBranchesByParentId(&parentChatId)?
            .into_iter()
            .map(|entity| entity.toChatHistory(Vec::new()))
            .collect())
    }

    pub fn getBranchesFlow(
        &self,
        parentChatId: String,
    ) -> ChatHistoryManagerResult<Vec<ChatHistory>> {
        self.getBranches(parentChatId)
    }

    pub fn clearCharacterCardBinding(
        &self,
        characterCardName: String,
    ) -> ChatHistoryManagerResult<()> {
        self.chatDao
            .clearCharacterCardBinding(&characterCardName, currentTimeMillis())?;
        Ok(())
    }

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

    pub fn getLatestSummaryTimestamp(
        &self,
        chatId: String,
    ) -> ChatHistoryManagerResult<Option<i64>> {
        Ok(self.messageDao.getLatestSummaryTimestamp(&chatId)?)
    }

    pub fn loadMessagesAfterLatestSummaryInRange(
        &self,
        chatId: String,
        beforeTimestampExclusive: Option<i64>,
        upToTimestampInclusive: Option<i64>,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        let latestSummaryTimestamp = if let Some(beforeTimestampExclusive) = beforeTimestampExclusive {
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

    pub fn hasUserMessage(&self, chatId: String) -> ChatHistoryManagerResult<bool> {
        Ok(self.messageDao.existsUserMessage(&chatId)?)
    }

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

    pub fn loadChatMessageLocatorPreviews(
        &self,
        chatId: String,
    ) -> ChatHistoryManagerResult<Vec<ChatMessageLocatorPreview>> {
        Ok(self
            .messageDao
            .getLocatorPreviewsForChat(&chatId, LOCATOR_PREVIEW_CHAR_COUNT)?)
    }

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

    pub fn hasMessagesBefore(
        &self,
        chatId: String,
        beforeTimestampExclusive: i64,
    ) -> ChatHistoryManagerResult<bool> {
        Ok(self
            .messageDao
            .existsMessagesBeforeTimestamp(&chatId, beforeTimestampExclusive)?)
    }

    pub fn hasMessagesAfter(
        &self,
        chatId: String,
        afterTimestampExclusive: i64,
    ) -> ChatHistoryManagerResult<bool> {
        Ok(self
            .messageDao
            .existsMessagesAfterTimestamp(&chatId, afterTimestampExclusive)?)
    }

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

    pub fn updateChatCharacterGroupId(
        &self,
        chatId: String,
        characterGroupId: Option<String>,
    ) -> ChatHistoryManagerResult<()> {
        self.chatDao.updateChatCharacterGroupId(
            &chatId,
            characterGroupId,
            currentTimeMillis(),
        )?;
        Ok(())
    }

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
        Ok(())
    }

    pub fn clearCharacterGroupBinding(
        &self,
        characterGroupId: String,
    ) -> ChatHistoryManagerResult<i32> {
        Ok(self
            .chatDao
            .clearCharacterGroupBinding(&characterGroupId, currentTimeMillis())?)
    }

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

    pub fn renameCharacterCardInChats(
        &self,
        oldName: String,
        newName: String,
    ) -> ChatHistoryManagerResult<i32> {
        Ok(self
            .chatDao
            .renameCharacterCardBinding(&oldName, &newName, currentTimeMillis())?)
    }

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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatImportResult {
    pub new: i32,
    pub updated: i32,
    pub skipped: i32,
}

impl ChatImportResult {
    pub fn total(&self) -> i32 {
        self.new + self.updated
    }
}

fn currentTimeMillis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time must be after UNIX_EPOCH")
        .as_millis() as i64
}
