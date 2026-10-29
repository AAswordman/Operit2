use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::CoreNodeBindingStore::CoreNodeBindingStore;
use crate::PreferencesDataStore::{
    combine2, stringPreferencesKey, CoroutineScope, PreferencesDataStore, PreferencesDataStoreError,
    PreferencesKey, SharingStarted, StateFlow,
};
use crate::RuntimeStorePaths::RuntimeStorePaths;
use thiserror::Error;
use uuid::Uuid;

use crate::dao::ChatDao::ChatDao;
use crate::dao::MessageDao::{insertMessageParams, insertMessageSql, mapMessageEntity, MessageDao};
use crate::dao::MessagePartDao::MessagePartDao;
use crate::dao::MessageVariantDao::{insertVariantParams, insertVariantSql, MessageVariantDao};
use crate::db::AppDatabase::{AppDatabase, AppDatabaseError};
use crate::repository::WorkspacePreferenceStore::WorkspacePreferenceStore;
use crate::sqliteParams;
use crate::sync::SqlChatSyncStore::{SqlChatSyncStore, SqlChatSyncStoreError};
use crate::SqliteStore::{SqliteRowGet, SqliteStore, SqliteStoreError, SqliteValue};
use operit_model::ChatEntity::ChatEntity;
use operit_model::ChatHistory::ChatHistory;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatMessageLocatorPreview::ChatMessageLocatorPreview;
use operit_model::MessageEntity::MessageEntity;
use operit_model::MessagePart::MessagePart;
use operit_model::MessagePartEntity::MessagePartEntity;
use operit_model::MessageVariantEntity::MessageVariantEntity;
use operit_model::OperitChatArchive::{
    OperitArchivedChat, OperitArchivedMessage, OperitArchivedMessageVariant, OperitChatArchive,
    ARCHIVE_TYPE, CURRENT_FORMAT_VERSION,
};
use operit_model::Workspace::{Workspace, WorkspaceFolder};
use serde::{Deserialize, Serialize};

const LOCATOR_PREVIEW_CHAR_COUNT: i32 = 48;

/// Builds persistence rows for every part in one message revision.
fn messagePartEntities(
    chatId: &str,
    messageTimestamp: i64,
    variantIndex: i32,
    parts: &[MessagePart],
) -> Vec<MessagePartEntity> {
    parts
        .iter()
        .cloned()
        .map(|part| {
            MessagePartEntity::fromMessagePart(
                chatId.to_string(),
                messageTimestamp,
                variantIndex,
                part,
            )
        })
        .collect()
}

/// Groups part rows by their owning message revision for repository persistence.
fn groupMessagePartEntities(
    parts: Vec<MessagePartEntity>,
) -> BTreeMap<(i64, i32), Vec<MessagePartEntity>> {
    let mut groups = BTreeMap::new();
    for part in parts {
        groups
            .entry((part.messageTimestamp, part.variantIndex))
            .or_insert_with(Vec::new)
            .push(part);
    }
    groups
}

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

/// Deletes a revision atomically using the Kotlin promotion and adjacent-selection rules.
fn deleteMessageVariantRevision(
    store: &SqliteStore,
    baseMessage: &MessageEntity,
    variants: &[MessageVariantEntity],
    variantIndex: i32,
) -> ChatHistoryManagerResult<()> {
    if baseMessage.sender != "ai" {
        return Err(ChatHistoryManagerError::IllegalArgument(
            "Only AI messages can delete variants".to_string(),
        ));
    }
    let firstVariant = variants.first().ok_or_else(|| {
        ChatHistoryManagerError::IllegalState(format!(
            "Message {} has no deletable variants",
            baseMessage.timestamp
        ))
    })?;
    let targetVariant = if variantIndex == 0 {
        firstVariant
    } else {
        variants
            .iter()
            .find(|variant| variant.variantIndex == variantIndex)
            .ok_or_else(|| {
                ChatHistoryManagerError::IllegalArgument(format!(
                    "Variant {variantIndex} does not exist for message {}",
                    baseMessage.timestamp
                ))
            })?
    };
    let selectedVariantIndex = if variantIndex == 0 {
        0
    } else if variants
        .iter()
        .any(|variant| variant.variantIndex > targetVariant.variantIndex)
    {
        targetVariant.variantIndex
    } else {
        (targetVariant.variantIndex - 1).max(0)
    };
    let chatId = &baseMessage.chatId;
    let messageTimestamp = baseMessage.timestamp;
    store.transaction(|transaction| {
        if variantIndex == 0 {
            let mut promotedMessage = targetVariant.applyTo(
                baseMessage.toChatMessage(Vec::new()),
                Vec::new(),
                variants.len() as i32,
            );
            let extensionRow = transaction.queryOne("SELECT pluginExtensions FROM message_variants WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3", sqliteParams![chatId, messageTimestamp, targetVariant.variantIndex])?.ok_or_else(|| crate::SqliteStore::SqliteStoreError::Message("Promoted variant no longer exists".to_string()))?;
            promotedMessage.pluginExtensions = crate::PluginExtensions::decodePluginExtensions(&crate::SqliteStore::SqliteRowGet::get::<_, String>(&extensionRow, 0)?)?;
            transaction.execute(
                r#"
                UPDATE messages
                SET roleName = ?3, provider = ?4, modelName = ?5,
                    inputTokens = ?6, outputTokens = ?7, cachedInputTokens = ?8,
                    sentAt = ?9, outputDurationMs = ?10, waitDurationMs = ?11,
                    completedAt = ?12, pluginExtensions = ?13
                WHERE chatId = ?1 AND timestamp = ?2
                "#,
                sqliteParams![
                    chatId, messageTimestamp, promotedMessage.roleName,
                    promotedMessage.provider, promotedMessage.modelName,
                    promotedMessage.inputTokens, promotedMessage.outputTokens,
                    promotedMessage.cachedInputTokens, promotedMessage.sentAt,
                    promotedMessage.outputDurationMs, promotedMessage.waitDurationMs,
                    promotedMessage.completedAt,
                    crate::PluginExtensions::encodePluginExtensions(&promotedMessage.pluginExtensions)?,
                ],
            )?;
            transaction.execute(
                "DELETE FROM message_parts WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = 0",
                sqliteParams![chatId, messageTimestamp],
            )?;
            transaction.execute(
                "UPDATE message_parts SET variantIndex = 0 WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
                sqliteParams![chatId, messageTimestamp, targetVariant.variantIndex],
            )?;
        } else {
            transaction.execute(
                "DELETE FROM message_parts WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
                sqliteParams![chatId, messageTimestamp, targetVariant.variantIndex],
            )?;
        }
        transaction.execute(
            "DELETE FROM message_variants WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
            sqliteParams![chatId, messageTimestamp, targetVariant.variantIndex],
        )?;
        // Move revisions in ascending order so each unique destination has been vacated.
        for variant in variants
            .iter()
            .filter(|variant| variant.variantIndex > targetVariant.variantIndex)
        {
            transaction.execute(
                "UPDATE message_variants SET variantIndex = ?4 WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
                sqliteParams![chatId, messageTimestamp, variant.variantIndex, variant.variantIndex - 1],
            )?;
            transaction.execute(
                "UPDATE message_parts SET variantIndex = ?4 WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
                sqliteParams![chatId, messageTimestamp, variant.variantIndex, variant.variantIndex - 1],
            )?;
        }
        transaction.execute(
            "UPDATE messages SET selectedVariantIndex = ?3 WHERE chatId = ?1 AND timestamp = ?2",
            sqliteParams![chatId, messageTimestamp, selectedVariantIndex],
        )?;
        Ok(())
    })?;
    Ok(())
}

/// Repository for chats, messages, variants, branches, and sync recording.
#[derive(Clone)]
pub struct ChatHistoryManager {
    database: Arc<AppDatabase>,
    chatDao: ChatDao,
    workspaceStore: WorkspacePreferenceStore,
    messageDao: MessageDao,
    messageVariantDao: MessageVariantDao,
    syncStore: SqlChatSyncStore,
    bindingStore: CoreNodeBindingStore,
    chatHistoriesFlow: StateFlow<Vec<ChatHistory>>,
    currentChatIdDataStore: PreferencesDataStore,
    pub currentChatIdFlow: StateFlow<Option<String>>,
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

    /// Opens database handles and the persisted current-chat selection flow.
    fn create(paths: RuntimeStorePaths) -> ChatHistoryManagerResult<Self> {
        let currentChatIdDataStore =
            PreferencesDataStore::new(paths.current_chat_id_preferences_path());
        let database = AppDatabase::getDatabase(paths.clone())?;
        let chatDao = database.chatDao();
        let workspaceStore = WorkspacePreferenceStore::new(paths.clone());
        let messageDao = database.messageDao();
        let messageVariantDao = database.messageVariantDao();
        let syncStore = SqlChatSyncStore::new(paths.clone(), &database)?;
        let bindingStore =
            CoreNodeBindingStore::default().map_err(ChatHistoryManagerError::IllegalState)?;
        let workspacesFlow = workspaceStore.allWorkspacesFlow();
        let workspaceStateFlow = workspacesFlow.stateIn(
            CoroutineScope,
            SharingStarted::Lazily,
            workspacesFlow.first()?,
        );
        let chatHistoriesFlow = combine2(
            &chatDao.getAllChats()?,
            &workspaceStateFlow,
            |chatEntities, workspaces| attachWorkspaceNames(workspaces, chatEntities),
        );
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
            .mapResult(|preferences| preferences.getTyped::<String>(&PreferencesKeys::CURRENT_CHAT_ID()))
            .stateIn(CoroutineScope, SharingStarted::Lazily, None);
        Ok(Self {
            database,
            chatDao,
            workspaceStore,
            messageDao,
            messageVariantDao,
            syncStore,
            bindingStore,
            chatHistoriesFlow,
            currentChatIdDataStore,
            currentChatIdFlow,
        })
    }

    /// Hydrates message entities with selected variants into chat messages.
    fn hydrateMessages(
        &self,
        messageEntities: Vec<MessageEntity>,
        variants: Vec<MessageVariantEntity>,
        partEntities: Vec<MessagePartEntity>,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        let mut variantsByTimestamp = messageEntities
            .iter()
            .map(|message| (message.timestamp, Vec::<MessageVariantEntity>::new()))
            .collect::<HashMap<_, _>>();
        for variant in variants {
            variantsByTimestamp
                .entry(variant.messageTimestamp)
                .or_insert_with(Vec::new)
                .push(variant);
        }
        for variants in variantsByTimestamp.values_mut() {
            variants.sort_by_key(|variant| variant.variantIndex);
        }

        let mut partsByRevision = HashMap::<(i64, i32), Vec<MessagePart>>::new();
        for part in partEntities {
            partsByRevision
                .entry((part.messageTimestamp, part.variantIndex))
                .or_insert_with(Vec::new)
                .push(part.toMessagePart());
        }
        for parts in partsByRevision.values_mut() {
            parts.sort_by_key(|part| part.sequence);
        }

        messageEntities
            .into_iter()
            .map(|messageEntity| {
                let baseParts = partsByRevision
                    .remove(&(messageEntity.timestamp, 0))
                    .ok_or_else(|| {
                        ChatHistoryManagerError::IllegalState(format!(
                            "Missing base parts for message {}",
                            messageEntity.timestamp
                        ))
                    })?;
                let baseMessage = messageEntity.toChatMessage(baseParts);
                let messageVariants = variantsByTimestamp
                    .remove(&messageEntity.timestamp)
                    .expect("every message must initialize a variant list");
                let variantCount = messageVariants.len() as i32 + 1;
                if messageEntity.selectedVariantIndex == 0 {
                    Ok(ChatMessage {
                        selectedVariantIndex: 0,
                        variantCount,
                        ..baseMessage
                    })
                } else {
                    let selectedVariant = messageVariants
                        .iter()
                        .find(|variant| variant.variantIndex == messageEntity.selectedVariantIndex)
                        .ok_or_else(|| {
                            ChatHistoryManagerError::IllegalState(format!(
                                "Selected variant {} does not exist for message {}",
                                messageEntity.selectedVariantIndex, messageEntity.timestamp
                            ))
                        })?;
                    let selectedParts = partsByRevision
                        .remove(&(messageEntity.timestamp, selectedVariant.variantIndex))
                        .ok_or_else(|| {
                            ChatHistoryManagerError::IllegalState(format!(
                                "Missing parts for message {} variant {}",
                                messageEntity.timestamp, selectedVariant.variantIndex
                            ))
                        })?;
                    Ok(selectedVariant.applyTo(baseMessage, selectedParts, variantCount))
                }
            })
            .collect()
    }

    /// Loads and hydrates a message query from one consistent SQLite snapshot.
    fn loadMessagesSnapshot(
        &self,
        chatId: &str,
        sql: &str,
        params: Vec<SqliteValue>,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        let snapshot = self.messageDao.getSnapshot(chatId, sql, params)?;
        self.hydrateMessages(snapshot.messages, snapshot.variants, snapshot.parts)
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
        let snapshot = self.messageDao.getSnapshot(&chatHistory.id,
            "SELECT * FROM messages WHERE chatId = ?1 ORDER BY timestamp ASC",
            sqliteParams![chatHistory.id])?;
        let messageEntities = snapshot.messages;
        let mut partsByRevision = snapshot.parts
            .into_iter()
            .fold(
                HashMap::<(i64, i32), Vec<MessagePart>>::new(),
                |mut groups, part| {
                    groups
                        .entry((part.messageTimestamp, part.variantIndex))
                        .or_insert_with(Vec::new)
                        .push(part.toMessagePart());
                    groups
                },
            );
        for parts in partsByRevision.values_mut() {
            parts.sort_by_key(|part| part.sequence);
        }
        let mut variantsByTimestamp = messageEntities
            .iter()
            .map(|message| (message.timestamp, Vec::<MessageVariantEntity>::new()))
            .collect::<HashMap<_, _>>();
        for variant in snapshot.variants {
            variantsByTimestamp
                .get_mut(&variant.messageTimestamp)
                .expect("every variant must reference a base message")
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
                    .expect("every archived message must initialize a variant list");
                let baseParts = partsByRevision
                    .remove(&(messageEntity.timestamp, 0))
                    .expect("every archived message must have base parts");
                OperitArchivedMessage {
                    baseMessage: ChatMessage {
                        variantCount: messageVariants.len() as i32 + 1,
                        ..messageEntity.toChatMessage(baseParts)
                    },
                    variants: messageVariants
                        .into_iter()
                        .map(|variant| {
                            let parts = partsByRevision
                                .remove(&(messageEntity.timestamp, variant.variantIndex))
                                .expect("every archived variant must have parts");
                            OperitArchivedMessageVariant::fromEntity(variant, parts)
                        })
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

    /// Loads the current chat history list directly from persistent storage.
    pub fn loadChatHistories(&self) -> ChatHistoryManagerResult<Vec<ChatHistory>> {
        Ok(attachWorkspaceNames(
            self.workspaceStore.getAll()?,
            self.chatDao.getAllChatsDirectly()?,
        ))
    }

    #[allow(non_snake_case)]
    /// Returns the observable chat metadata list from persistent storage.
    pub fn chatHistoriesFlow(&self) -> ChatHistoryManagerResult<StateFlow<Vec<ChatHistory>>> {
        Ok(self.chatHistoriesFlow.clone())
    }

    /// Loads one persisted chat history metadata row.
    #[allow(non_snake_case)]
    pub fn loadChatHistory(&self, chatId: String) -> ChatHistoryManagerResult<Option<ChatHistory>> {
        Ok(self
            .chatDao
            .getChatById(&chatId)?
            .map(|chatEntity| chatEntity.toChatHistory(Vec::new())))
    }

    /// Counts all stored chats.
    pub fn getTotalChatCount(&self) -> ChatHistoryManagerResult<i32> {
        Ok(self.chatDao.getTotalChatCount()?)
    }

    /// Counts all stored messages.
    pub fn getTotalMessageCount(&self) -> ChatHistoryManagerResult<i32> {
        Ok(self.messageDao.getTotalMessageCount()?)
    }

    /// Counts messages across the complete chat directory.
    pub fn getMessageCountsByChatId(&self) -> ChatHistoryManagerResult<HashMap<String, i32>> {
        Ok(self
            .messageDao
            .getMessageCountsByChatId()?
            .into_iter()
            .map(|count| (count.chatId, count.count))
            .collect())
    }

    /// Loads only explicitly selected metadata records without materializing message histories.
    pub fn getMessageCountsForChats(
        &self,
        ids: &[String],
    ) -> ChatHistoryManagerResult<HashMap<String, i32>> {
        Ok(self
            .messageDao
            .getMessageCountsForChats(ids)?
            .into_iter()
            .map(|count| (count.chatId, count.count))
            .collect())
    }

    /// Loads only explicitly selected metadata records without materializing message histories.
    pub fn loadChatHistoriesByIds(
        &self,
        ids: &[String],
    ) -> ChatHistoryManagerResult<Vec<ChatHistory>> {
        Ok(self
            .chatDao
            .getChatsByIds(ids)?
            .into_iter()
            .map(|chat| chat.toChatHistory(Vec::new()))
            .collect())
    }

    /// Reads the persisted current chat id.
    pub fn currentChatIdFlow(&self) -> ChatHistoryManagerResult<Option<String>> {
        Ok(self.currentChatIdFlow.first()?)
    }

    /// Saves host metadata and message edits without overwriting live namespaces or deleting independent variants.
    fn saveChatHistoryInternal(&self, history: ChatHistory) -> ChatHistoryManagerResult<()> {
        if self.chatDao.getChatById(&history.id)?.is_some() {
            self.chatDao
                .updateChats(vec![ChatEntity::fromChatHistory(&history)])?;
            for message in history.messages {
                self.updateMessage(history.id.clone(), message)?;
            }
        } else {
            self.syncStore.commitChatDraft(&history, None)?;
        }
        Ok(())
    }

    /// Saves chat metadata and records a chat snapshot for sync.
    pub fn saveChatHistory(&self, history: ChatHistory) -> ChatHistoryManagerResult<()> {
        let chatId = history.id.clone();
        let isNewChat = self.chatDao.getChatById(&chatId)?.is_none();
        self.saveChatHistoryInternal(history)?;
        self.recordChatSnapshot(&chatId)?;
        if isNewChat {
            self.createBinding(&chatId)?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Exports all chats and messages as an Operit chat archive.
    pub fn exportChatHistoriesToJson(&self) -> ChatHistoryManagerResult<String> {
        let chats = self
            .loadChatHistories()?
            .into_iter()
            .map(|chatHistory| self.buildOperitArchivedChat(chatHistory))
            .collect::<ChatHistoryManagerResult<Vec<_>>>()?;
        let workspaceIds = chats
            .iter()
            .filter_map(|chat| chat.workspaceId.clone())
            .collect::<HashSet<_>>();
        let workspaces = self
            .workspaceStore
            .getAll()?
            .into_iter()
            .filter(|workspace| workspaceIds.contains(&workspace.id))
            .collect::<Vec<_>>();
        let archive = OperitChatArchive {
            archiveType: ARCHIVE_TYPE.to_string(),
            formatVersion: CURRENT_FORMAT_VERSION,
            exportedAt: currentTimeMillis(),
            workspaces,
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
        self.importChatArchive(archive, |_, _, _| {})
    }

    /// Imports one validated chat archive and reports each persisted chat.
    pub fn importChatArchiveWithProgress<F>(
        &self,
        archive: OperitChatArchive,
        onChatPersisted: F,
    ) -> ChatHistoryManagerResult<ChatImportResult>
    where
        F: FnMut(usize, usize, usize),
    {
        self.importChatArchive(archive, onChatPersisted)
    }

    /// Validates and imports a chat archive while invoking its persistence callback.
    fn importChatArchive<F>(
        &self,
        archive: OperitChatArchive,
        mut onChatPersisted: F,
    ) -> ChatHistoryManagerResult<ChatImportResult>
    where
        F: FnMut(usize, usize, usize),
    {
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
            .loadChatHistories()?
            .into_iter()
            .map(|chat| chat.id)
            .collect::<HashSet<_>>();
        let mut counters = ImportCounters {
            newCount: 0,
            updatedCount: 0,
            skippedCount: 0,
        };
        for workspace in archive.workspaces {
            self.upsertWorkspace(workspace)?;
        }
        let totalChatCount = archive.chats.len();
        let mut processedChatCount = 0;
        for archivedChat in archive.chats {
            let messageCount = archivedChat.messages.len();
            processedChatCount += 1;
            let isNewChat = existingIds.insert(archivedChat.id.clone());
            if !isNewChat {
                counters.updatedCount += 1;
            } else {
                counters.newCount += 1;
            }
            self.saveArchivedChat(archivedChat, isNewChat)?;
            onChatPersisted(processedChatCount, totalChatCount, messageCount);
        }
        Ok(ChatImportResult {
            new: counters.newCount,
            updated: counters.updatedCount,
            skipped: counters.skippedCount,
        })
    }

    /// Persists one imported chat and optionally creates its initial Binding.
    #[allow(non_snake_case)]
    fn saveArchivedChat(
        &self,
        archivedChat: OperitArchivedChat,
        createBinding: bool,
    ) -> ChatHistoryManagerResult<()> {
        let chatId = archivedChat.id.clone();
        let history = archivedChat
            .toChatHistory()
            .map_err(ChatHistoryManagerError::IllegalArgument)?;
        let chatEntity = ChatEntity::fromChatHistory(&history);

        let mut variants = Vec::new();
        let mut partEntities = Vec::new();
        let messages = archivedChat
            .messages
            .into_iter()
            .enumerate()
            .map(|(index, archivedMessage)| {
                let baseMessage = archivedMessage.baseMessage;
                for variant in archivedMessage.variants {
                    partEntities.extend(messagePartEntities(
                        &chatId,
                        baseMessage.timestamp,
                        variant.variantIndex,
                        &variant.parts,
                    ));
                    variants.push(variant.toEntity(chatId.clone(), baseMessage.timestamp));
                }
                partEntities.extend(messagePartEntities(
                    &chatId,
                    baseMessage.timestamp,
                    0,
                    &baseMessage.parts,
                ));
                MessageEntity::fromChatMessage(chatId.clone(), baseMessage, index as i32, 0)
            })
            .collect::<Vec<_>>();
        self.syncStore
            .commitImportedChatRecords(chatEntity, messages, variants, partEntities)?;
        if createBinding {
            self.createBinding(&chatId)?;
        }
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

    /// Persists message metadata, revision parts, chat ordering, and sync state atomically.
    fn persistMessageLocked(
        &self,
        chatId: &str,
        messageToPersist: ChatMessage,
    ) -> ChatHistoryManagerResult<ChatMessage> {
        if messageToPersist.selectedVariantIndex < 0 {
            return Err(ChatHistoryManagerError::IllegalArgument(
                "Variant index must be nonnegative".to_string(),
            ));
        }
        let timestamp = messageToPersist.timestamp;
        let variantIndex = messageToPersist.selectedVariantIndex;
        let updatedAt = currentTimeMillis();
        self.syncStore.mutateMessage(chatId, timestamp, |transaction| {
            let existing = transaction.queryOne(
                "SELECT * FROM messages WHERE chatId = ?1 AND timestamp = ?2",
                sqliteParams![chatId, timestamp],
            )?.as_ref().map(mapMessageEntity).transpose()?;
            if variantIndex > 0 {
                transaction.queryOne(
                    "SELECT variantId FROM message_variants WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
                    sqliteParams![chatId, timestamp, variantIndex],
                )?.ok_or_else(|| SqliteStoreError::Message(format!("Variant {variantIndex} does not exist for message {chatId}:{timestamp}")))?;
                let variant = MessageVariantEntity::fromChatMessage(
                    chatId.to_string(), timestamp, variantIndex, messageToPersist.clone(), 0,
                );
                transaction.execute(insertVariantSql(false), insertVariantParams(&variant, false)?)?;
                let changed = transaction.execute(
                    "UPDATE messages SET selectedVariantIndex = ?3 WHERE chatId = ?1 AND timestamp = ?2",
                    sqliteParams![chatId, timestamp, variantIndex],
                )?;
                if changed != 1 {
                    return Err(SqliteStoreError::Message(format!("Message {chatId}:{timestamp} does not exist")));
                }
            } else {
                let orderIndex = existing.as_ref().map_or(0, |message| message.orderIndex);
                let entity = MessageEntity::fromChatMessage(chatId.to_string(), messageToPersist.clone(), orderIndex, 0);
                transaction.execute(insertMessageSql(false), insertMessageParams(&entity, false)?)?;
            }
            MessagePartDao::replacePartsInTransaction(
                transaction, chatId, timestamp, variantIndex,
                messagePartEntities(chatId, timestamp, variantIndex, &messageToPersist.parts),
            )?;
            if existing.is_none() && messageToPersist.sender == "user" {
                transaction.execute(
                    "UPDATE chats SET displayOrder = MIN(-?2, (SELECT MIN(displayOrder) FROM chats) - 1), updatedAt = ?2 WHERE id = ?1",
                    sqliteParams![chatId, updatedAt],
                )?;
            } else if existing.is_none() || messageToPersist.sender != "ai" || messageToPersist.completedAt > 0 || variantIndex > 0 {
                transaction.execute("UPDATE chats SET updatedAt = ?2 WHERE id = ?1", sqliteParams![chatId, updatedAt])?;
            }
            Ok(())
        })?;
        Ok(messageToPersist)
    }

    /// Resolves an anchored message timestamp against the current stored neighbors.
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

    /// Reorders only real chat identifiers and rejects duplicates or nonexistent rows before any write.
    pub fn updateChatOrder(&self, chatIds: Vec<String>) -> ChatHistoryManagerResult<()> {
        let mut seen = HashSet::new();
        let mut entities = Vec::new();
        for (index, chatId) in chatIds.iter().enumerate() {
            if !seen.insert(chatId.clone()) {
                return Err(ChatHistoryManagerError::IllegalArgument(format!(
                    "Duplicate chat id in order: {chatId}"
                )));
            }
            let mut entity = self.chatDao.getChatById(chatId)?.ok_or_else(|| {
                ChatHistoryManagerError::IllegalArgument(format!("Chat does not exist: {chatId}"))
            })?;
            entity.displayOrder = index as i64;
            entities.push(entity);
        }
        self.chatDao.updateChats(entities)?;
        for chatId in chatIds {
            self.recordChatMetadata(&chatId)?;
        }
        Ok(())
    }

    /// Changes native folder membership atomically without touching plugin namespaces.
    pub fn updateChatGroups(&self, chatIds: Vec<String>, groupName: Option<String>) -> ChatHistoryManagerResult<()> {
        let name = groupName.map(|name| name.trim().to_string()).filter(|name| !name.is_empty());
        let mut chats = Vec::new();
        for id in &chatIds {
            let mut chat = self.chatDao.getChatById(id)?.ok_or_else(|| ChatHistoryManagerError::IllegalArgument(format!("Chat does not exist: {id}")))?;
            chat.group = name.clone();
            chat.updatedAt = currentTimeMillis();
            chats.push(chat);
        }
        self.chatDao.updateChats(chats)?;
        for id in chatIds { self.recordChatMetadata(&id)?; }
        Ok(())
    }

    /// Returns the persistent synchronization clock observed by this chat store.
    #[allow(non_snake_case)]
    pub fn syncClock(&self) -> ChatHistoryManagerResult<crate::SyncOperationStore::SyncClock> {
        Ok(self.syncStore.localClock()?)
    }

    /// Deletes one message and records sync deletion state.
    pub fn deleteMessage(&self, chatId: String, timestamp: i64) -> ChatHistoryManagerResult<()> {
        self.syncStore.recordMessageDeletion(&chatId, timestamp)?;
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
        let variants = self
            .messageVariantDao
            .getVariantsForMessage(&chatId, messageTimestamp)?;
        deleteMessageVariantRevision(self.database.store(), &baseMessage, &variants, variantIndex)?;
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
        self.persistMessageLocked(&chatId, message)?;
        Ok(())
    }

    /// Atomically commits one assistant segment, chat metrics, and its sync operation.
    pub fn commitAssistantMessageSegment(
        &self,
        chatId: String,
        message: ChatMessage,
        chatMetrics: Option<(i64, i64, i64)>,
    ) -> ChatHistoryManagerResult<crate::SyncOperationStore::SyncClock> {
        if message.sender != "ai" {
            return Err(ChatHistoryManagerError::IllegalArgument(format!(
                "assistant segment requires an ai message for chat {chatId}"
            )));
        }
        if message.selectedVariantIndex != 0 {
            return Err(ChatHistoryManagerError::IllegalArgument(format!(
                "assistant segment requires the base revision for chat {chatId}"
            )));
        }
        let existingMessage = self
            .messageDao
            .getMessageByTimestamp(&chatId, message.timestamp)?
            .ok_or_else(|| {
                ChatHistoryManagerError::IllegalState(format!(
                    "Assistant placeholder {} does not exist in chat {chatId}",
                    message.timestamp
                ))
            })?;
        let mut chat = self.chatDao.getChatById(&chatId)?.ok_or_else(|| {
            ChatHistoryManagerError::IllegalArgument(format!("Chat does not exist: {chatId}"))
        })?;
        chat.updatedAt = currentTimeMillis();
        if let Some((inputTokens, outputTokens, currentWindowSize)) = chatMetrics {
            chat.inputTokens = inputTokens;
            chat.outputTokens = outputTokens;
            chat.currentWindowSize = currentWindowSize;
        }
        let timestamp = message.timestamp;
        let parts = message.parts.clone();
        let messageEntity = MessageEntity::fromChatMessage(
            chatId.clone(),
            message,
            existingMessage.orderIndex,
            existingMessage.messageId,
        );
        let partEntities = messagePartEntities(&chatId, timestamp, 0, &parts);
        Ok(self
            .syncStore
            .commitAssistantMessageSegment(chat, messageEntity, partEntities)?)
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
        let updatedAt = currentTimeMillis();
        Ok(self.syncStore.mutateMessage(&chatId, messageTimestamp, |transaction| {
            let row = transaction.queryOne(
                "SELECT sender FROM messages WHERE chatId = ?1 AND timestamp = ?2",
                sqliteParams![chatId, messageTimestamp],
            )?.ok_or_else(|| SqliteStoreError::Message(format!("Message {chatId}:{messageTimestamp} does not exist")))?;
            if row.get::<_, String>(0)? != "ai" {
                return Err(SqliteStoreError::Message("Only AI messages can have regenerated variants".to_string()));
            }
            let row = transaction.queryOne(
                "SELECT COUNT(*) + 1 FROM message_variants WHERE chatId = ?1 AND messageTimestamp = ?2",
                sqliteParams![chatId, messageTimestamp],
            )?.expect("aggregate variant count must return one row");
            let nextVariantIndex: i32 = row.get(0)?;
            let parts = messagePartEntities(&chatId, messageTimestamp, nextVariantIndex, &message.parts);
            let variant = MessageVariantEntity::fromChatMessage(chatId.clone(), messageTimestamp, nextVariantIndex, message, 0);
            transaction.execute(insertVariantSql(false), insertVariantParams(&variant, false)?)?;
            MessagePartDao::replacePartsInTransaction(transaction, &chatId, messageTimestamp, nextVariantIndex, parts)?;
            transaction.execute(
                "UPDATE messages SET selectedVariantIndex = ?3 WHERE chatId = ?1 AND timestamp = ?2",
                sqliteParams![chatId, messageTimestamp, nextVariantIndex],
            )?;
            transaction.execute("UPDATE chats SET updatedAt = ?2 WHERE id = ?1", sqliteParams![chatId, updatedAt])?;
            Ok(nextVariantIndex)
        })?)
    }

    /// Selects the active variant for one message.
    pub fn selectMessageVariant(
        &self,
        chatId: String,
        messageTimestamp: i64,
        selectedVariantIndex: i32,
    ) -> ChatHistoryManagerResult<()> {
        if selectedVariantIndex < 0 {
            return Err(ChatHistoryManagerError::IllegalArgument(
                "Variant index must be nonnegative".to_string(),
            ));
        }
        self.syncStore.mutateMessage(&chatId, messageTimestamp, |transaction| {
            if selectedVariantIndex > 0 {
                transaction.queryOne(
                    "SELECT variantId FROM message_variants WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
                    sqliteParams![chatId, messageTimestamp, selectedVariantIndex],
                )?.ok_or_else(|| SqliteStoreError::Message(format!("Variant {selectedVariantIndex} does not exist for message {chatId}:{messageTimestamp}")))?;
            }
            let changed = transaction.execute(
                "UPDATE messages SET selectedVariantIndex = ?3 WHERE chatId = ?1 AND timestamp = ?2",
                sqliteParams![chatId, messageTimestamp, selectedVariantIndex],
            )?;
            if changed != 1 {
                return Err(SqliteStoreError::Message(format!("Message {chatId}:{messageTimestamp} does not exist")));
            }
            Ok(())
        })?;
        Ok(())
    }

    /// Deletes messages from a timestamp onward and records sync deletions.
    pub fn deleteMessagesFrom(
        &self,
        chatId: String,
        timestamp: i64,
    ) -> ChatHistoryManagerResult<()> {
        self.syncStore.recordMessagesFromDeletion(&chatId, timestamp)?;
        Ok(())
    }

    /// Deletes every message and variant in a chat.
    pub fn clearChatMessages(&self, chatId: String) -> ChatHistoryManagerResult<()> {
        self.syncStore.recordAllMessagesForChatDeletion(&chatId)?;
        Ok(())
    }

    /// Updates a chat title and records chat metadata for sync.
    pub fn updateChatTitle(&self, chatId: String, title: String) -> ChatHistoryManagerResult<()> {
        self.chatDao
            .updateChatTitle(&chatId, title, currentTimeMillis())?;
        self.recordChatMetadata(&chatId)?;
        Ok(())
    }

    /// Updates cached token counts for a chat.
    pub fn updateChatTokenCounts(
        &self,
        chatId: String,
        inputTokens: i64,
        outputTokens: i64,
        currentWindowSize: i64,
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

    /// Persists summary compaction without overwriting totals recorded by a concurrent reply.
    pub fn updateChatContextWindow(
        &self,
        chatId: String,
        currentWindowSize: i64,
    ) -> ChatHistoryManagerResult<ChatHistory> {
        self.chatDao.updateChatContextWindow(&chatId, currentWindowSize, currentTimeMillis())?;
        self.recordChatMetadata(&chatId)?;
        self.loadChatHistory(chatId.clone())?.ok_or_else(|| {
            ChatHistoryManagerError::IllegalArgument(format!("Chat does not exist: {chatId}"))
        })
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

    /// Checks one conversation's committed deletion fact through its indexed lifecycle key.
    pub fn chatIsDeleted(&self, chatId: &str) -> ChatHistoryManagerResult<bool> {
        Ok(self.syncStore.chatIsDeleted(chatId)?)
    }

    /// Returns whether chat deletion is currently allowed.
    pub fn canDeleteChatHistory(&self, chatId: String) -> ChatHistoryManagerResult<bool> {
        match self.chatDao.getChatById(&chatId)? {
            Some(chat) => Ok(!chat.locked),
            None => Ok(self.syncStore.chatIsDeleted(&chatId)?),
        }
    }

    /// Deletes a chat when deletion policy allows it.
    pub fn deleteChatHistory(&self, chatId: String) -> ChatHistoryManagerResult<bool> {
        if !self.syncStore.deleteChat(&chatId)? {
            return Ok(false);
        }
        self.deleteBinding(&chatId)?;
        if self.currentChatIdFlow()?.as_deref() == Some(chatId.as_str()) {
            self.clearCurrentChatId()?;
        }
        Ok(true)
    }

    /// Builds an unpersisted chat draft with explicit plugin namespace objects.
    pub fn newChatDraft(
        &self,
        title: String,
        pluginExtensions: BTreeMap<String, serde_json::Value>,
        workspaceId: Option<String>,
        parentChatId: Option<String>,
    ) -> ChatHistoryManagerResult<ChatHistory> {
        crate::PluginExtensions::validatePluginExtensions(&pluginExtensions)?;
        if let Some(workspaceId) = &workspaceId {
            if self.workspaceStore.getById(workspaceId)?.is_none() {
                return Err(ChatHistoryManagerError::IllegalArgument(format!(
                    "Workspace does not exist: {workspaceId}"
                )));
            }
        }
        let mut entity = ChatEntity::create(title);
        entity.pluginExtensions = pluginExtensions;
        entity.workspaceId = workspaceId;
        entity.parentChatId = parentChatId;
        Ok(entity.toChatHistory(Vec::new()))
    }

    /// Commits a resolved draft and initial messages or a true source branch in one SQLite transaction.
    pub fn commitChatDraft(
        &self,
        draft: ChatHistory,
        source: Option<(&str, Option<i64>)>,
    ) -> ChatHistoryManagerResult<ChatHistory> {
        self.syncStore.commitChatDraft(&draft, source)?;
        self.createBinding(&draft.id)?;
        self.loadChatHistory(draft.id.clone())?.ok_or_else(|| {
            ChatHistoryManagerError::IllegalState("Committed draft chat was not found".to_string())
        })
    }

    /// Materializes metadata on the executor after Space has installed the
    /// Binding. Reconnects reuse the same chat without changing UI selection.
    pub fn ensureRoutedChat(&self, chatId: String) -> ChatHistoryManagerResult<()> {
        self.bindingStore
            .binding(&chatId)
            .map_err(ChatHistoryManagerError::IllegalState)?;
        if self.chatDao.getChatById(&chatId)?.is_none() {
            self.createChatWithId(chatId, "Edge Chat".into(), false)?;
        }
        Ok(())
    }

    /// Creates only executor routing metadata using a real host-supplied chat id.
    fn createChatWithId(
        &self,
        chatId: String,
        title: String,
        createBinding: bool,
    ) -> ChatHistoryManagerResult<ChatHistory> {
        let draft = ChatEntity::new(chatId, title, currentTimeMillis()).toChatHistory(Vec::new());
        self.syncStore.commitChatDraft(&draft, None)?;
        if createBinding {
            self.createBinding(&draft.id)?;
        }
        Ok(draft)
    }

    /// Updates the workspace id bound to a chat.
    pub fn updateChatWorkspaceId(
        &self,
        chatId: String,
        workspaceId: Option<String>,
    ) -> ChatHistoryManagerResult<()> {
        if let Some(workspaceId) = workspaceId.as_ref() {
            let workspace = self.workspaceStore.getById(workspaceId)?.ok_or_else(|| {
                ChatHistoryManagerError::IllegalArgument(format!(
                    "workspace does not exist: {workspaceId}"
                ))
            })?;
            workspace
                .validate()
                .map_err(ChatHistoryManagerError::IllegalArgument)?;
        }
        self.chatDao
            .updateChatWorkspaceId(&chatId, workspaceId, currentTimeMillis())?;
        self.recordChatMetadata(&chatId)?;
        Ok(())
    }

    /// Loads every persisted workspace.
    pub fn listWorkspaces(&self) -> ChatHistoryManagerResult<Vec<Workspace>> {
        Ok(self.workspaceStore.getAll()?)
    }

    /// Loads one workspace by id.
    pub fn getWorkspace(&self, workspaceId: &str) -> ChatHistoryManagerResult<Option<Workspace>> {
        Ok(self.workspaceStore.getById(workspaceId)?)
    }

    /// Loads the workspace bound to a chat.
    pub fn getWorkspaceForChat(&self, chatId: &str) -> ChatHistoryManagerResult<Option<Workspace>> {
        let chat = self.chatDao.getChatById(chatId)?.ok_or_else(|| {
            ChatHistoryManagerError::IllegalArgument(format!("Chat does not exist: {chatId}"))
        })?;
        let Some(workspaceId) = chat.workspaceId else {
            return Ok(None);
        };
        Ok(self.workspaceStore.getById(&workspaceId)?)
    }

    /// Creates a named workspace with the supplied folders.
    pub fn createWorkspace(
        &self,
        name: String,
        folders: Vec<WorkspaceFolder>,
    ) -> ChatHistoryManagerResult<Workspace> {
        let timestamp = currentTimeMillis();
        let workspace = Workspace {
            id: Uuid::new_v4().to_string(),
            name,
            folders,
            createdAt: timestamp,
            updatedAt: timestamp,
        };
        workspace
            .validate()
            .map_err(ChatHistoryManagerError::IllegalArgument)?;
        Ok(self.workspaceStore.upsert(workspace)?)
    }

    /// Persists an existing workspace record.
    pub fn upsertWorkspace(&self, workspace: Workspace) -> ChatHistoryManagerResult<Workspace> {
        workspace
            .validate()
            .map_err(ChatHistoryManagerError::IllegalArgument)?;
        Ok(self.workspaceStore.upsert(workspace)?)
    }

    /// Renames a workspace.
    pub fn renameWorkspace(
        &self,
        workspaceId: String,
        name: String,
    ) -> ChatHistoryManagerResult<Workspace> {
        let mut workspace = self.workspaceStore.getById(&workspaceId)?.ok_or_else(|| {
            ChatHistoryManagerError::IllegalArgument(format!(
                "workspace does not exist: {workspaceId}"
            ))
        })?;
        workspace.name = name;
        workspace.updatedAt = currentTimeMillis();
        workspace
            .validate()
            .map_err(ChatHistoryManagerError::IllegalArgument)?;
        Ok(self.workspaceStore.upsert(workspace)?)
    }

    /// Adds a folder to an existing workspace.
    ///
    /// Mounting stays idempotent: a path that is already mounted under another
    /// name leaves the workspace untouched, and a known folder name whose path
    /// changed is rebound instead of failing validation with a duplicate name.
    pub fn addWorkspaceFolder(
        &self,
        workspaceId: String,
        folder: WorkspaceFolder,
    ) -> ChatHistoryManagerResult<Workspace> {
        let mut workspace = self.workspaceStore.getById(&workspaceId)?.ok_or_else(|| {
            ChatHistoryManagerError::IllegalArgument(format!(
                "workspace does not exist: {workspaceId}"
            ))
        })?;
        if !workspace.mountFolder(folder) {
            return Ok(workspace);
        }
        workspace.updatedAt = currentTimeMillis();
        workspace
            .validate()
            .map_err(ChatHistoryManagerError::IllegalArgument)?;
        Ok(self.workspaceStore.upsert(workspace)?)
    }

    /// Removes a folder from an existing workspace.
    pub fn removeWorkspaceFolder(
        &self,
        workspaceId: String,
        folderName: String,
    ) -> ChatHistoryManagerResult<Workspace> {
        let mut workspace = self.workspaceStore.getById(&workspaceId)?.ok_or_else(|| {
            ChatHistoryManagerError::IllegalArgument(format!(
                "workspace does not exist: {workspaceId}"
            ))
        })?;
        let originalLen = workspace.folders.len();
        workspace.folders.retain(|folder| folder.name != folderName);
        if workspace.folders.len() == originalLen {
            return Err(ChatHistoryManagerError::IllegalArgument(format!(
                "workspace folder not found: {folderName}"
            )));
        }
        workspace.updatedAt = currentTimeMillis();
        workspace
            .validate()
            .map_err(ChatHistoryManagerError::IllegalArgument)?;
        Ok(self.workspaceStore.upsert(workspace)?)
    }

    /// Deletes a workspace that is not bound to any chat.
    pub fn deleteWorkspace(&self, workspaceId: String) -> ChatHistoryManagerResult<()> {
        let bound = self
            .chatDao
            .getAllChatsDirectly()?
            .into_iter()
            .any(|chat| chat.workspaceId.as_deref() == Some(workspaceId.as_str()));
        if bound {
            return Err(ChatHistoryManagerError::IllegalArgument(format!(
                "workspace is bound to a chat: {workspaceId}"
            )));
        }
        self.workspaceStore.delete(&workspaceId)?;
        Ok(())
    }

    /// Loads the title for a chat id.
    pub fn getChatTitle(&self, chatId: String) -> ChatHistoryManagerResult<Option<String>> {
        Ok(self.chatDao.getChatById(&chatId)?.map(|chat| chat.title))
    }

    /// Loads all hydrated messages for one chat.
    pub fn loadChatMessages(&self, chatId: &str) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        self.loadMessagesSnapshot(
            chatId,
            "SELECT * FROM messages WHERE chatId = ?1 ORDER BY timestamp ASC",
            sqliteParams![chatId],
        )
    }

    /// Loads hydrated messages with optional summary-message filtering.
    pub fn loadChatMessagesWithOptions(
        &self,
        chatId: String,
        order: Option<String>,
        limit: Option<i32>,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        let normalizedOrder = order.map(|order| order.trim().to_lowercase());
        let direction = match normalizedOrder.as_deref() {
            Some("desc") => "DESC",
            _ => "ASC",
        };
        let (sql, params) = match limit {
            Some(limit) => (format!("SELECT * FROM messages WHERE chatId = ?1 ORDER BY timestamp {direction} LIMIT ?2"), sqliteParams![chatId, limit.max(1)]),
            None => (format!("SELECT * FROM messages WHERE chatId = ?1 ORDER BY timestamp {direction}"), sqliteParams![chatId]),
        };
        self.loadMessagesSnapshot(&chatId, &sql, params)
    }

    /// Loads hydrated messages inside an inclusive zero-based index range.
    pub fn loadChatMessagesRange(
        &self,
        chatId: String,
        order: String,
        start: i32,
        end: i32,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        if start < 0 || end < start {
            return Err(ChatHistoryManagerError::IllegalArgument(
                "range requires 0 <= start <= end".to_string(),
            ));
        }
        let limit = end
            .checked_sub(start)
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| {
                ChatHistoryManagerError::IllegalArgument(
                    "range length exceeds supported integer limits".to_string(),
                )
            })?;
        let direction = match order.trim().to_lowercase().as_str() {
            "asc" => "ASC",
            "desc" => "DESC",
            value => {
                return Err(ChatHistoryManagerError::IllegalArgument(format!(
                    "order must be asc/desc, got {value}"
                )))
            }
        };
        let sql = format!("SELECT * FROM messages WHERE chatId = ?1 ORDER BY timestamp {direction} LIMIT ?2 OFFSET ?3");
        self.loadMessagesSnapshot(&chatId, &sql, sqliteParams![chatId, limit, start])
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
        self.loadMessagesSnapshot(
            &chatId,
            r#"
            WITH latest AS (
                SELECT MAX(timestamp) AS timestamp FROM messages
                WHERE chatId = ?1 AND sender = 'summary'
                    AND (?2 IS NULL OR timestamp < ?2)
                    AND (?2 IS NOT NULL OR ?3 IS NULL OR timestamp <= ?3)
            )
            SELECT messages.* FROM messages, latest
            WHERE chatId = ?1
                AND (latest.timestamp IS NULL OR messages.timestamp > latest.timestamp)
                AND (?2 IS NULL OR messages.timestamp < ?2)
                AND (?3 IS NULL OR messages.timestamp <= ?3)
            ORDER BY messages.timestamp ASC
            "#,
            sqliteParams![chatId, beforeTimestampExclusive, upToTimestampInclusive],
        )
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
        self.loadMessagesSnapshot(&chatId,
            r#"
            WITH latest AS (
                SELECT MAX(timestamp) AS timestamp FROM messages WHERE chatId = ?1 AND sender = 'summary'
            )
            SELECT messages.* FROM messages, latest
            WHERE chatId = ?1 AND (latest.timestamp IS NULL OR messages.timestamp >= latest.timestamp)
            ORDER BY messages.timestamp ASC
            "#,
            sqliteParams![chatId])
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
        self.loadMessagesSnapshot(
            &chatId,
            "SELECT * FROM messages WHERE chatId = ?1 AND timestamp >= ?2 ORDER BY timestamp ASC",
            sqliteParams![chatId, startTimestampInclusive],
        )
    }

    /// Loads hydrated messages inside an inclusive timestamp window.
    pub fn loadChatMessagesWindow(
        &self,
        chatId: String,
        startTimestampInclusive: i64,
        endTimestampInclusive: i64,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        self.loadMessagesSnapshot(&chatId,
            "SELECT * FROM messages WHERE chatId = ?1 AND timestamp >= ?2 AND timestamp <= ?3 ORDER BY timestamp ASC",
            sqliteParams![chatId, startTimestampInclusive, endTimestampInclusive])
    }

    /// Loads hydrated messages after a timestamp in ascending order.
    pub fn loadChatMessagesAscAfter(
        &self,
        chatId: String,
        afterTimestampExclusive: i64,
        limit: i32,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        self.loadMessagesSnapshot(&chatId,
            "SELECT * FROM messages WHERE chatId = ?1 AND timestamp > ?2 ORDER BY timestamp ASC LIMIT ?3",
            sqliteParams![chatId, afterTimestampExclusive, limit])
    }

    /// Loads older messages before a timestamp for paged history views.
    pub fn loadOlderChatMessages(
        &self,
        chatId: String,
        beforeTimestampExclusive: i64,
        limit: i32,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        let mut messages = self.loadMessagesSnapshot(&chatId,
            "SELECT * FROM messages WHERE chatId = ?1 AND timestamp < ?2 ORDER BY timestamp DESC LIMIT ?3",
            sqliteParams![chatId, beforeTimestampExclusive, limit])?;
        messages.reverse();
        Ok(messages)
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
        match beforeTimestampExclusive {
            Some(timestamp) => self.loadMessagesSnapshot(&chatId,
                "SELECT * FROM messages WHERE chatId = ?1 AND timestamp < ?2 ORDER BY timestamp DESC LIMIT ?3",
                sqliteParams![chatId, timestamp, limit]),
            None => self.loadMessagesSnapshot(&chatId,
                "SELECT * FROM messages WHERE chatId = ?1 ORDER BY timestamp DESC LIMIT ?2",
                sqliteParams![chatId, limit]),
        }
    }

    /// Loads messages in descending order up to an inclusive timestamp.
    pub fn loadChatMessagesDescUpTo(
        &self,
        chatId: String,
        maxTimestampInclusive: i64,
        limit: i32,
    ) -> ChatHistoryManagerResult<Vec<ChatMessage>> {
        self.loadMessagesSnapshot(&chatId,
            "SELECT * FROM messages WHERE chatId = ?1 AND timestamp <= ?2 ORDER BY timestamp DESC LIMIT ?3",
            sqliteParams![chatId, maxTimestampInclusive, limit])
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

    /// Creates the local initial Binding for one newly created chat key.
    fn createBinding(&self, chatId: &str) -> ChatHistoryManagerResult<()> {
        self.bindingStore
            .createLocal(chatId)
            .map_err(ChatHistoryManagerError::IllegalState)?;
        Ok(())
    }

    /// Deletes the Binding associated with one deleted chat key.
    fn deleteBinding(&self, chatId: &str) -> ChatHistoryManagerResult<()> {
        self.bindingStore
            .delete(chatId)
            .map_err(ChatHistoryManagerError::IllegalState)?;
        Ok(())
    }

    fn recordMessageSnapshot(&self, chatId: &str, timestamp: i64) -> ChatHistoryManagerResult<()> {
        self.syncStore.recordMessageSnapshot(chatId, timestamp)?;
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

/// Projects workspace names and primary paths from the current workspace records.
fn attachWorkspaceNames(
    workspaces: Vec<Workspace>,
    chatEntities: Vec<ChatEntity>,
) -> Vec<ChatHistory> {
    let workspaces = workspaces
        .into_iter()
        .map(|workspace| (workspace.id.clone(), workspace))
        .collect::<HashMap<_, _>>();
    chatEntities
        .into_iter()
        .map(|chatEntity| {
            let workspace = chatEntity
                .workspaceId
                .as_ref()
                .and_then(|workspaceId| workspaces.get(workspaceId));
            let mut history = chatEntity.toChatHistory(Vec::new());
            history.workspaceName = workspace.map(|item| item.name.clone());
            history.workspacePrimaryPath = workspace.map(|item| item.primaryFolder().path.clone());
            history
        })
        .collect()
}

#[cfg(test)]
#[path = "ChatHistoryManagerTests.rs"]
mod tests;

#[path = "ChatHistoryExtensions.rs"]
mod plugin_extensions;

#[cfg(test)]
#[path = "ChatHistoryExtensionTests.rs"]
mod extension_tests;
