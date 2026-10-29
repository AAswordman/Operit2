use crate::PluginExtensions::{decodePluginExtensions, encodePluginExtensions};
use std::collections::{BTreeMap, BTreeSet};

use crate::sqliteParams;
use crate::RuntimeStorePaths::RuntimeStorePaths;
use crate::SqliteStore::{
    SqliteRow, SqliteRowGet, SqliteStore, SqliteStoreError, SqliteTransaction, SqliteValue,
};
use crate::SyncOperationStore::{
    compactSyncOperations, incomingSyncOperationWins, publishSyncMutation, SyncClock, SyncOperation, SyncOperationSemantics,
    SyncOperationStore, SyncOperationStoreError, SyncOperationKnowledge, SyncReceipt,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::dao::ChatDao::ChatDao;
use crate::dao::MessageDao::MessageDao;
use crate::db::AppDatabase::{AppDatabase, AppDatabaseError};
use operit_model::ChatEntity::ChatEntity;
use operit_model::ChatMessagePresentation::{ChatMessagePresentation, PRESENTATION_EXTENSION};
use operit_model::MessageEntity::MessageEntity;
use operit_model::MessagePart::MessagePartKind;
use operit_model::MessagePartEntity::MessagePartEntity;
use operit_model::MessageVariantEntity::MessageVariantEntity;

/// Sync domain used for SQL-backed chat history operations.
pub const CHAT_SYNC_DOMAIN: &str = "chat";

const CHAT_SYNC_OPERATION_SCHEMA_VERSION: i32 = 8;

const DELETE_CHAT: &str = "chats";
const DELETE_MESSAGE: &str = "messages";
const DELETE_MESSAGES_FROM: &str = "messages_from";
const DELETE_MESSAGES_FOR_CHAT: &str = "messages_for_chat";
const DELETE_VARIANT: &str = "message_variants";
const DELETE_VARIANTS_FROM: &str = "message_variants_from";
const DELETE_VARIANTS_FOR_MESSAGE: &str = "message_variants_for_message";
const DELETE_VARIANTS_FOR_CHAT: &str = "message_variants_for_chat";

/// Error type for SQL chat sync recording and replay.
#[derive(Debug, Error)]
pub enum SqlChatSyncStoreError {
    #[error(transparent)]
    Database(#[from] AppDatabaseError),
    #[error(transparent)]
    Store(#[from] SqliteStoreError),
    #[error(transparent)]
    Sync(#[from] SyncOperationStoreError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Message(String),
}

/// Describes one deletion operation inside a chat sync payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatSyncDeletion {
    pub tableName: String,
    pub chatId: String,
    pub messageTimestamp: Option<i64>,
    pub variantIndex: Option<i32>,
}

/// Carries chat, message, part, variant, and deletion rows for one sync operation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatSyncPayload {
    pub chatRows: Vec<ChatEntity>,
    pub messageRows: Vec<MessageEntity>,
    pub partRows: Vec<MessagePartEntity>,
    pub variantRows: Vec<MessageVariantEntity>,
    pub deletions: Vec<ChatSyncDeletion>,
}

/// Records and applies sync operations for SQL-backed chat data.
#[derive(Clone)]
pub struct SqlChatSyncStore {
    store: SqliteStore,
    syncOperationStore: SyncOperationStore,
    originDeviceId: String,
}

impl SqlChatSyncStore {
    /// Creates a SQL chat sync store for a database and runtime path set.
    pub fn new(
        paths: RuntimeStorePaths,
        database: &AppDatabase,
    ) -> Result<Self, SqlChatSyncStoreError> {
        let syncOperationStore = SyncOperationStore::native(paths);
        let deviceId = syncOperationStore.localDeviceId()?;
        Ok(Self {
            store: database.store().clone(),
            syncOperationStore,
            originDeviceId: format!("{deviceId}:sql"),
        })
    }

    /// Creates a sync store from the default database and runtime paths.
    pub fn default() -> Result<Self, SqlChatSyncStoreError> {
        let database = AppDatabase::default()?;
        Self::new(RuntimeStorePaths::default(), &database)
    }

    /// Records a metadata-only upsert for a chat.
    pub fn recordChatMetadata(&self, chatId: &str) -> Result<(), SqlChatSyncStoreError> {
        let payload = self.payloadForChatMetadata(chatId)?;
        if payload.chatRows.is_empty() {
            return Ok(());
        }
        self.appendLocalOperation(
            "chat",
            chatId,
            "upsert",
            SyncOperationSemantics::EntityState,
            payload,
        )?;
        Ok(())
    }

    /// Records a full chat snapshot including messages and variants.
    pub fn recordChatSnapshot(&self, chatId: &str) -> Result<(), SqlChatSyncStoreError> {
        let payload = self.payloadForChatSnapshot(chatId)?;
        if payload.chatRows.is_empty() {
            return Ok(());
        }
        self.appendLocalOperation(
            "chat",
            chatId,
            "upsert",
            SyncOperationSemantics::EntityState,
            payload,
        )?;
        Ok(())
    }

    /// Records a snapshot for one message and its variants.
    pub fn recordMessageSnapshot(
        &self,
        chatId: &str,
        timestamp: i64,
    ) -> Result<(), SqlChatSyncStoreError> {
        let payload = self.payloadForMessageSnapshot(chatId, timestamp)?;
        if payload.chatRows.is_empty()
            && payload.messageRows.is_empty()
            && payload.partRows.is_empty()
            && payload.variantRows.is_empty()
        {
            return Ok(());
        }
        self.appendLocalOperation(
            "message",
            &format!("{chatId}:{timestamp}"),
            "upsert",
            SyncOperationSemantics::EntityState,
            payload,
        )?;
        Ok(())
    }

    /// Atomically commits one assistant segment together with chat metadata and sync state.
    pub fn commitAssistantMessageSegment(
        &self,
        chat: ChatEntity,
        message: MessageEntity,
        baseParts: Vec<MessagePartEntity>,
    ) -> Result<SyncClock, SqlChatSyncStoreError> {
        if message.chatId != chat.id {
            return Err(SqlChatSyncStoreError::Message(format!(
                "assistant message chat {} does not match metadata chat {}",
                message.chatId, chat.id
            )));
        }
        if message.sender != "ai" {
            return Err(SqlChatSyncStoreError::Message(format!(
                "assistant segment requires an ai message for chat {}",
                chat.id
            )));
        }
        if baseParts.iter().any(|part| {
            part.chatId != chat.id
                || part.messageTimestamp != message.timestamp
                || part.variantIndex != 0
        }) {
            return Err(SqlChatSyncStoreError::Message(format!(
                "assistant segment contains parts outside {}:{} base revision",
                chat.id, message.timestamp
            )));
        }

        if baseParts.is_empty() {
            return Err(SqlChatSyncStoreError::Message("Assistant segment requires at least one base part".to_string()));
        }
        let presentation = message.pluginExtensions.get(PRESENTATION_EXTENSION)
            .map(|value| serde_json::from_value::<ChatMessagePresentation>(value.clone()))
            .transpose()?;
        let createdAt = currentTimeMillis()?;
        self.store.transaction(|transaction| {
            let current = extension_transactions::payloadInTransaction(transaction, &chat.id, Some(message.timestamp))?;
            let variantRows = current.variantRows;
            let mut partRows = current.partRows.into_iter().filter(|part| part.variantIndex != 0).collect::<Vec<_>>();
            partRows.extend(baseParts);

            let mut chat = chat.clone();
            let chatRow = transaction.queryOne("SELECT pluginExtensions FROM chats WHERE id = ?1", sqliteParams![chat.id])?.ok_or_else(|| SqliteStoreError::Message(format!("Chat does not exist: {}", chat.id)))?;
            chat.pluginExtensions = decodePluginExtensions(&chatRow.get::<_, String>(0)?)?;
            let mut message = message.clone();
            let messageRow = transaction.queryOne("SELECT pluginExtensions FROM messages WHERE chatId = ?1 AND timestamp = ?2", sqliteParams![message.chatId, message.timestamp])?.ok_or_else(|| SqliteStoreError::Message("Assistant base record does not exist".to_string()))?;
            message.pluginExtensions = decodePluginExtensions(&messageRow.get::<_, String>(0)?)?;
            if let Some(presentation) = &presentation {
                message.pluginExtensions.insert(PRESENTATION_EXTENSION.to_string(),
                    serde_json::to_value(presentation).map_err(|error| SqliteStoreError::Message(error.to_string()))?);
            }
            let mut variantRows = variantRows.clone();
            for variant in &mut variantRows {
                let row = transaction.queryOne("SELECT pluginExtensions FROM message_variants WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3", sqliteParams![variant.chatId, variant.messageTimestamp, variant.variantIndex])?.ok_or_else(|| SqliteStoreError::Message("Assistant variant record does not exist".to_string()))?;
                variant.pluginExtensions = decodePluginExtensions(&row.get::<_, String>(0)?)?;
            }
            let payload = ChatSyncPayload {
                chatRows: vec![chat.clone()],
                messageRows: vec![message.clone()],
                partRows: partRows.clone(),
                variantRows,
                deletions: Vec::new(),
            };
            let payloadValue = serde_json::to_value(&payload)
                .map_err(|error| SqliteStoreError::Message(error.to_string()))?;
            upsertChat(transaction, &chat)?;
            upsertMessage(transaction, &message)?;
            for variant in &payload.variantRows {
                upsertVariant(transaction, variant)?;
            }
            replacePartRows(transaction, &payload.partRows)?;

            let sequence = sequenceFor(transaction, &self.originDeviceId)? + 1;
            let operation = SyncOperation {
                opId: format!("{}:{sequence}", self.originDeviceId),
                originDeviceId: self.originDeviceId.clone(),
                sequence,
                domain: CHAT_SYNC_DOMAIN.to_string(),
                entityType: "message".to_string(),
                entityId: format!("{}:{}", chat.id, message.timestamp),
                operation: "upsert".to_string(),
                semantics: SyncOperationSemantics::EntityState,
                payload: payloadValue,
                createdAt,
                schemaVersion: CHAT_SYNC_OPERATION_SCHEMA_VERSION,
            };
            insertOperation(transaction, &operation, &payload)?;
            observeOperation(transaction, &operation)?;
            Ok(())
        })?;
        self.store.notifyInvalidated()?;
        publishSyncMutation();
        self.localClock()
    }

    /// Records deletion of one chat row.
    pub fn recordChatDeletion(&self, chatId: &str) -> Result<(), SqlChatSyncStoreError> {
        let payload = ChatSyncPayload {
            deletions: vec![ChatSyncDeletion {
                tableName: DELETE_CHAT.to_string(),
                chatId: chatId.to_string(),
                messageTimestamp: None,
                variantIndex: None,
            }],
            ..ChatSyncPayload::default()
        };
        self.appendLocalOperation(
            "chat",
            chatId,
            "delete",
            SyncOperationSemantics::Transaction,
            payload,
        )?;
        self.store.notifyInvalidated()?;
        Ok(())
    }

    /// Deletes one message and its variants together with their sync deletion record.
    pub fn recordMessageDeletion(
        &self,
        chatId: &str,
        timestamp: i64,
    ) -> Result<(), SqlChatSyncStoreError> {
        let mut deletions = Vec::new();
        deletions.push(ChatSyncDeletion {
            tableName: DELETE_VARIANTS_FOR_MESSAGE.to_string(),
            chatId: chatId.to_string(),
            messageTimestamp: Some(timestamp),
            variantIndex: None,
        });
        deletions.push(ChatSyncDeletion {
            tableName: DELETE_MESSAGE.to_string(),
            chatId: chatId.to_string(),
            messageTimestamp: Some(timestamp),
            variantIndex: None,
        });
        self.commitMessageDeletion(
            chatId,
            "message",
            &format!("{chatId}:{timestamp}"),
            deletions,
        )
    }

    /// Deletes messages from a timestamp and records their sync deletions atomically.
    pub fn recordMessagesFromDeletion(
        &self,
        chatId: &str,
        timestamp: i64,
    ) -> Result<(), SqlChatSyncStoreError> {
        let mut deletions = Vec::new();
        deletions.push(ChatSyncDeletion {
            tableName: DELETE_VARIANTS_FROM.to_string(),
            chatId: chatId.to_string(),
            messageTimestamp: Some(timestamp),
            variantIndex: None,
        });
        deletions.push(ChatSyncDeletion {
            tableName: DELETE_MESSAGES_FROM.to_string(),
            chatId: chatId.to_string(),
            messageTimestamp: Some(timestamp),
            variantIndex: None,
        });
        self.commitMessageDeletion(
            chatId,
            "messages",
            &format!("{chatId}:{timestamp}"),
            deletions,
        )
    }

    /// Clears message records and records their sync deletions atomically.
    pub fn recordAllMessagesForChatDeletion(
        &self,
        chatId: &str,
    ) -> Result<(), SqlChatSyncStoreError> {
        let mut deletions = Vec::new();
        deletions.push(ChatSyncDeletion {
            tableName: DELETE_VARIANTS_FOR_CHAT.to_string(),
            chatId: chatId.to_string(),
            messageTimestamp: None,
            variantIndex: None,
        });
        deletions.push(ChatSyncDeletion {
            tableName: DELETE_MESSAGES_FOR_CHAT.to_string(),
            chatId: chatId.to_string(),
            messageTimestamp: None,
            variantIndex: None,
        });
        self.commitMessageDeletion(chatId, "messages", chatId, deletions)
    }

    /// Reads the latest observed SQL chat sync clock.
    pub fn localClock(&self) -> Result<SyncClock, SqlChatSyncStoreError> {
        let sequences = self
            .store
            .queryRows(
                "SELECT originDeviceId, sequence FROM sync_sql_clocks ORDER BY originDeviceId",
                sqliteParams![],
            )?
            .into_iter()
            .map(|row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
            .collect::<Result<BTreeMap<_, _>, SqliteStoreError>>()?;
        Ok(SyncClock { sequences })
    }

    /// Reads actual committed operation sequences without inferring missing SQL history.
    pub fn receipt(&self) -> Result<SyncReceipt, SqlChatSyncStoreError> {
        let mut receipt = SyncReceipt::default();
        for row in self.store.queryRows(
            "SELECT originDeviceId,sequence FROM sync_sql_operations ORDER BY originDeviceId,sequence",
            Vec::new(),
        )? {
            receipt.observe(&row.get::<_, String>(0)?, row.get(1)?);
        }
        Ok(receipt)
    }

    /// Lists compacted SQL chat operations newer than the supplied clock.
    pub fn operationsSince<K: SyncOperationKnowledge>(
        &self,
        clock: &K,
        domains: &[String],
        limit: usize,
    ) -> Result<Vec<SyncOperation>, SqlChatSyncStoreError> {
        let domainSet = domains.iter().cloned().collect::<BTreeSet<_>>();
        if !domainSet.is_empty() && !domainSet.contains(CHAT_SYNC_DOMAIN) {
            return Ok(Vec::new());
        }
        let mut operations = Vec::new();
        for originRow in self.store.queryRows(
            "SELECT DISTINCT originDeviceId FROM sync_sql_operations WHERE domain=?1",
            sqliteParams![CHAT_SYNC_DOMAIN],
        )? {
            let origin: String = originRow.get(0)?;
            let mut cursor = clock
                .sequenceFor(&origin)
                .max(self.syncOperationStore.exportFloorFor(&origin)?);
            let mut missing = 0usize;
            while missing < limit {
                let Some(next) = clock.nextMissingSequence(&origin, cursor) else {
                    break;
                };
                let rows = self.store.queryRows(
                    "SELECT opId, originDeviceId, sequence, domain, entityType, entityId, operation, semantics, createdAt, schemaVersion FROM sync_sql_operations WHERE domain=?1 AND originDeviceId=?2 AND sequence>=?3 ORDER BY sequence LIMIT ?4",
                    sqliteParams![CHAT_SYNC_DOMAIN, origin, next, limit.min(256) as i64],
                )?;
                if rows.is_empty() {
                    break;
                }
                for row in rows {
                    let operation = mapOperationMetadata(&row)?;
                    cursor = operation.sequence;
                    if clock.containsSequence(&origin, cursor) {
                        continue;
                    }
                    operations.push(operation);
                    missing += 1;
                    if missing == limit {
                        break;
                    }
                }
            }
        }
        operations.sort_by(|left, right| {
            (left.createdAt, &left.originDeviceId, left.sequence).cmp(&(
                right.createdAt,
                &right.originDeviceId,
                right.sequence,
            ))
        });
        let mut operations = compactSyncOperations(operations);
        operations.truncate(limit);
        for operation in &mut operations {
            operation.payload = serde_json::to_value(readPayload(
                &mut |sql, params| self.store.queryRows(sql, params), &operation.opId,
            )?)?;
        }
        Ok(operations)
    }

    /// Applies a remote SQL chat sync operation to the local database.
    pub fn applyOperation(&self, operation: &SyncOperation) -> Result<(), SqlChatSyncStoreError> {
        self.applyOperationInternal(operation, false)
    }

    /// Applies one Space bootstrap operation without local entity conflict filtering.
    #[allow(non_snake_case)]
    pub fn applyBootstrapOperation(
        &self,
        operation: &SyncOperation,
    ) -> Result<(), SqlChatSyncStoreError> {
        self.applyOperationInternal(operation, true)
    }

    /// Applies one SQL operation with the selected conflict policy.
    #[allow(non_snake_case)]
    fn applyOperationInternal(
        &self,
        operation: &SyncOperation,
        ignoreNewerEntityState: bool,
    ) -> Result<(), SqlChatSyncStoreError> {
        let didApply = self.store.transaction(|transaction| {
            let mut replaced = None;
            if let Some(row) = transaction.queryOne(
                "SELECT * FROM sync_sql_operations WHERE opId = ?1", sqliteParams![operation.opId],
            )? {
                let mut existing = mapOperationMetadata(&row)?;
                existing.payload = serde_json::to_value(readPayload(
                    &mut |sql, params| transaction.queryRows(sql, params), &operation.opId,
                )?).map_err(|error| SqliteStoreError::Message(error.to_string()))?;
                if !incomingSyncOperationWins(&existing, operation)
                    .map_err(|error| SqliteStoreError::Message(error.to_string()))? {
                    observeOperation(transaction, operation)?;
                    return Ok(false);
                }
                replaced = Some(existing);
                transaction.execute("DELETE FROM sync_sql_operations WHERE opId = ?1", sqliteParams![operation.opId])?;
            }
            if !ignoreNewerEntityState && hasNewerMergedEntityState(transaction, operation)? {
                observeOperation(transaction, operation)?;
                return Ok(false);
            }
            let payload = decodePayload(operation)
                .map_err(|error| SqliteStoreError::Message(error.to_string()))?;
            let later = match replaced {
                Some(existing) => laterChatOperations(transaction, &existing, &payload)?,
                None => Vec::new(),
            };
            applyPayload(transaction, operation, &payload)?;
            insertOperation(transaction, operation, &payload)?;
            for laterOperation in later {
                let laterPayload = readPayload(
                    &mut |sql, params| transaction.queryRows(sql, params), &laterOperation.opId,
                )?;
                applyPayload(transaction, &laterOperation, &laterPayload)?;
            }
            observeOperation(transaction, operation)?;
            Ok(true)
        })?;
        if didApply {
            self.store.notifyInvalidated()?;
            publishSyncMutation();
        }
        Ok(())
    }

    /// Marks local SQL operations before the Space join as unexportable.
    #[allow(non_snake_case)]
    pub fn markLocalOperationsUnexportable(&self) -> Result<(), SqlChatSyncStoreError> {
        let sequence = self.localClock()?.sequenceFor(&self.originDeviceId);
        self.syncOperationStore
            .setExportFloor(&self.originDeviceId, sequence)?;
        Ok(())
    }

    /// Appends a locally generated operation and updates the local clock.
    fn appendLocalOperation(
        &self,
        entityType: &str,
        entityId: &str,
        operationName: &str,
        semantics: SyncOperationSemantics,
        payload: ChatSyncPayload,
    ) -> Result<SyncOperation, SqlChatSyncStoreError> {
        let (_, operation) =
            self.commitLocalOperation(entityType, entityId, operationName, semantics, |_| {
                Ok(((), payload))
            })?;
        Ok(operation)
    }

    /// Commits an authoritative mutation and its sync payload in the same transaction.
    fn commitLocalOperation<T, F>(
        &self,
        entityType: &str,
        entityId: &str,
        operationName: &str,
        semantics: SyncOperationSemantics,
        action: F,
    ) -> Result<(T, SyncOperation), SqlChatSyncStoreError>
    where
        F: FnOnce(&mut SqliteTransaction<'_>) -> Result<(T, ChatSyncPayload), SqliteStoreError>,
    {
        let createdAt = currentTimeMillis()?;
        let result = self.store.transaction(|transaction| {
            let (result, payload) = action(transaction)?;
            let sequence = sequenceFor(transaction, &self.originDeviceId)? + 1;
            let operation = SyncOperation {
                opId: format!("{}:{sequence}", self.originDeviceId),
                originDeviceId: self.originDeviceId.clone(),
                sequence,
                domain: CHAT_SYNC_DOMAIN.to_string(),
                entityType: entityType.to_string(),
                entityId: entityId.to_string(),
                operation: operationName.to_string(),
                semantics,
                payload: serde_json::to_value(&payload)
                    .map_err(|error| SqliteStoreError::Message(error.to_string()))?,
                createdAt,
                schemaVersion: CHAT_SYNC_OPERATION_SCHEMA_VERSION,
            };
            insertOperation(transaction, &operation, &payload)?;
            observeOperation(transaction, &operation)?;
            Ok((result, operation))
        })?;
        publishSyncMutation();
        Ok(result)
    }

    /// Saves a complete message revision and its sync snapshot before releasing the transaction.
    pub(crate) fn mutateMessage<T, F>(
        &self,
        chatId: &str,
        timestamp: i64,
        action: F,
    ) -> Result<T, SqlChatSyncStoreError>
    where
        F: FnOnce(&mut SqliteTransaction<'_>) -> Result<T, SqliteStoreError>,
    {
        let (result, _) = self.commitLocalOperation(
            "message",
            &format!("{chatId}:{timestamp}"),
            "upsert",
            SyncOperationSemantics::EntityState,
            |transaction| {
                let result = action(transaction)?;
                let payload = extension_transactions::payloadInTransaction(
                    transaction,
                    chatId,
                    Some(timestamp),
                )?;
                Ok((result, payload))
            },
        )?;
        self.store.notifyInvalidated()?;
        Ok(result)
    }

    /// Applies deletions and records their metadata and sync operation atomically.
    fn commitMessageDeletion(
        &self,
        chatId: &str,
        entityType: &str,
        entityId: &str,
        deletions: Vec<ChatSyncDeletion>,
    ) -> Result<(), SqlChatSyncStoreError> {
        let updatedAt = currentTimeMillis()?;
        self.commitLocalOperation(
            entityType,
            entityId,
            "delete",
            SyncOperationSemantics::Transaction,
            |transaction| {
                for deletion in &deletions {
                    applyDeletion(transaction, deletion)?;
                }
                transaction.execute(
                    "UPDATE chats SET updatedAt = ?2 WHERE id = ?1",
                    sqliteParams![chatId, updatedAt],
                )?;
                let chatRows = transaction
                    .queryRows("SELECT * FROM chats WHERE id = ?1", sqliteParams![chatId])?
                    .iter()
                    .map(crate::dao::ChatDao::mapChatEntity)
                    .collect::<Result<_, _>>()?;
                Ok((
                    (),
                    ChatSyncPayload {
                        chatRows,
                        deletions,
                        ..ChatSyncPayload::default()
                    },
                ))
            },
        )?;
        self.store.notifyInvalidated()?;
        Ok(())
    }

    /// Builds a payload containing only chat metadata.
    fn payloadForChatMetadata(
        &self,
        chatId: &str,
    ) -> Result<ChatSyncPayload, SqlChatSyncStoreError> {
        let chatDao = ChatDao::new(self.store.clone());
        let chatRows = chatDao.getChatById(chatId)?.into_iter().collect::<Vec<_>>();
        Ok(ChatSyncPayload {
            chatRows,
            ..ChatSyncPayload::default()
        })
    }

    /// Builds a payload containing a complete chat, message, and variant snapshot.
    fn payloadForChatSnapshot(
        &self,
        chatId: &str,
    ) -> Result<ChatSyncPayload, SqlChatSyncStoreError> {
        let chatDao = ChatDao::new(self.store.clone());
        let messageDao = MessageDao::new(self.store.clone());
        let chatRows = chatDao.getChatById(chatId)?.into_iter().collect::<Vec<_>>();
        let snapshot = messageDao.getSnapshot(
            chatId,
            "SELECT * FROM messages WHERE chatId = ?1 ORDER BY timestamp ASC",
            sqliteParams![chatId],
        )?;
        let messageRows = snapshot.messages;
        let partRows = snapshot.parts;
        let variantRows = snapshot.variants;
        Ok(ChatSyncPayload {
            chatRows,
            messageRows,
            partRows,
            variantRows,
            deletions: Vec::new(),
        })
    }

    /// Builds a payload containing one message snapshot and its variants.
    fn payloadForMessageSnapshot(
        &self,
        chatId: &str,
        timestamp: i64,
    ) -> Result<ChatSyncPayload, SqlChatSyncStoreError> {
        let chatDao = ChatDao::new(self.store.clone());
        let messageDao = MessageDao::new(self.store.clone());
        let chatRows = chatDao.getChatById(chatId)?.into_iter().collect::<Vec<_>>();
        let snapshot = messageDao.getSnapshot(
            chatId,
            "SELECT * FROM messages WHERE chatId = ?1 AND timestamp = ?2",
            sqliteParams![chatId, timestamp],
        )?;
        let messageRows = snapshot.messages;
        let partRows = snapshot.parts;
        let variantRows = snapshot.variants;
        Ok(ChatSyncPayload {
            chatRows,
            messageRows,
            partRows,
            variantRows,
            deletions: Vec::new(),
        })
    }
}

/// Decodes one versioned chat sync payload into the canonical structured representation.
fn decodePayload(operation: &SyncOperation) -> Result<ChatSyncPayload, SqlChatSyncStoreError> {
    match operation.schemaVersion {
        CHAT_SYNC_OPERATION_SCHEMA_VERSION => {
            Ok(serde_json::from_value(operation.payload.clone())?)
        }
        unsupported => Err(SqlChatSyncStoreError::Message(format!(
            "unsupported chat sync operation schemaVersion: {unsupported}"
        ))),
    }
}

/// Maps operation metadata columns into a sync operation without payload rows.
fn mapOperationMetadata(row: &SqliteRow) -> Result<SyncOperation, SqliteStoreError> {
    Ok(SyncOperation {
        opId: row.get("opId")?,
        originDeviceId: row.get("originDeviceId")?,
        sequence: row.get("sequence")?,
        domain: row.get("domain")?,
        entityType: row.get("entityType")?,
        entityId: row.get("entityId")?,
        operation: row.get("operation")?,
        semantics: SyncOperationSemantics::fromStorageValue(&row.get::<_, String>("semantics")?)
            .map_err(SqliteStoreError::Message)?,
        payload: serde_json::Value::Null,
        createdAt: row.get("createdAt")?,
        schemaVersion: row.get("schemaVersion")?,
    })
}

type ChatPayloadQuery<'a> = dyn FnMut(&str, Vec<SqliteValue>) -> Result<Vec<SqliteRow>, SqliteStoreError> + 'a;

/// Reads one complete operation through the same query path inside or outside a transaction.
fn readPayload(query: &mut ChatPayloadQuery<'_>, opId: &str) -> Result<ChatSyncPayload, SqliteStoreError> {
    Ok(ChatSyncPayload {
        chatRows: readChatRows(query, opId)?,
        messageRows: readMessageRows(query, opId)?,
        partRows: readPartRows(query, opId)?,
        variantRows: readVariantRows(query, opId)?,
        deletions: readDeletions(query, opId)?,
    })
}

/// Reads complete chat sync records including independently persisted extension maps.
fn readChatRows(query: &mut ChatPayloadQuery<'_>, opId: &str) -> Result<Vec<ChatEntity>, SqliteStoreError> {
    query("SELECT id, title, createdAt, updatedAt, inputTokens, outputTokens, currentWindowSize, displayOrder, workspaceId, parentChatId, locked, pinned, pluginExtensions, \"group\" FROM sync_sql_chat_rows WHERE opId = ?1 ORDER BY id", sqliteParams![opId])?
        .into_iter().map(|row| Ok(ChatEntity {
                id: row.get(0)?,
                title: row.get(1)?,
                createdAt: row.get(2)?,
                updatedAt: row.get(3)?,
                inputTokens: row.get(4)?,
                outputTokens: row.get(5)?,
                currentWindowSize: row.get(6)?,
                displayOrder: row.get(7)?,
                workspaceId: row.get(8)?,
                parentChatId: row.get(9)?,
                locked: row.get(10)?,
                pinned: row.get(11)?,
                pluginExtensions: decodePluginExtensions(&row.get::<_, String>(12)?)?,
                group: row.get(13)?,
            })).collect()
}

/// Reads complete message sync records including independently persisted extension maps.
fn readMessageRows(
    query: &mut ChatPayloadQuery<'_>,
    opId: &str,
) -> Result<Vec<MessageEntity>, SqliteStoreError> {
    query("SELECT chatId, sender, timestamp, orderIndex, roleName, selectedVariantIndex, provider, modelName, inputTokens, outputTokens, cachedInputTokens, sentAt, outputDurationMs, waitDurationMs, completedAt, completedExecutionGeneration, displayMode, isFavorite, pluginExtensions FROM sync_sql_message_rows WHERE opId = ?1 ORDER BY chatId, timestamp", sqliteParams![opId])?
        .into_iter().map(|row| Ok(MessageEntity {
                messageId: 0,
                chatId: row.get(0)?,
                sender: row.get(1)?,
                timestamp: row.get(2)?,
                orderIndex: row.get(3)?,
                roleName: row.get(4)?,
                selectedVariantIndex: row.get(5)?,
                provider: row.get(6)?,
                modelName: row.get(7)?,
                inputTokens: row.get(8)?,
                outputTokens: row.get(9)?,
                cachedInputTokens: row.get(10)?,
                sentAt: row.get(11)?,
                outputDurationMs: row.get(12)?,
                waitDurationMs: row.get(13)?,
                completedAt: row.get(14)?,
                completedExecutionGeneration: row.get(15)?,
                displayMode: row.get(16)?,
                isFavorite: row.get(17)?,
                pluginExtensions: decodePluginExtensions(&row.get::<_, String>(18)?)?,
            })).collect()
}

/// Reads complete variant sync records including independently persisted extension maps.
fn readVariantRows(
    query: &mut ChatPayloadQuery<'_>,
    opId: &str,
) -> Result<Vec<MessageVariantEntity>, SqliteStoreError> {
    query("SELECT chatId, messageTimestamp, variantIndex, roleName, provider, modelName, inputTokens, outputTokens, cachedInputTokens, sentAt, outputDurationMs, waitDurationMs, completedAt, pluginExtensions FROM sync_sql_message_variant_rows WHERE opId = ?1 ORDER BY chatId, messageTimestamp, variantIndex", sqliteParams![opId])?
        .into_iter().map(|row| Ok(MessageVariantEntity {
                variantId: 0,
                chatId: row.get(0)?,
                messageTimestamp: row.get(1)?,
                variantIndex: row.get(2)?,
                roleName: row.get(3)?,
                provider: row.get(4)?,
                modelName: row.get(5)?,
                inputTokens: row.get(6)?,
                outputTokens: row.get(7)?,
                cachedInputTokens: row.get(8)?,
                sentAt: row.get(9)?,
                outputDurationMs: row.get(10)?,
                waitDurationMs: row.get(11)?,
                completedAt: row.get(12)?,
                pluginExtensions: decodePluginExtensions(&row.get::<_, String>(13)?)?,
            })).collect()
}

/// Reads structured part rows stored for one pending sync operation.
fn readPartRows(
    query: &mut ChatPayloadQuery<'_>,
    opId: &str,
) -> Result<Vec<MessagePartEntity>, SqliteStoreError> {
    query(
            r#"
            SELECT chatId, messageTimestamp, variantIndex, partId, sequence, kind, content,
                toolCallId, toolName, attributesJson
            FROM sync_sql_message_part_rows
            WHERE opId = ?1
            ORDER BY chatId, messageTimestamp, variantIndex, sequence
            "#,
            sqliteParams![opId],
        )?
        .into_iter()
        .map(|row| {
            let attributesJson: String = row.get(9)?;
            let kind: String = row.get(5)?;
            Ok(MessagePartEntity {
                chatId: row.get(0)?,
                messageTimestamp: row.get(1)?,
                variantIndex: row.get(2)?,
                partId: row.get(3)?,
                sequence: row.get(4)?,
                kind: messagePartKindFromLabel(&kind)?,
                content: row.get(6)?,
                toolCallId: row.get(7)?,
                toolName: row.get(8)?,
                attributes: serde_json::from_str(&attributesJson).map_err(|error| {
                    SqliteStoreError::Message(format!(
                        "invalid sync message-part attributes JSON: {error}"
                    ))
                })?,
            })
        })
        .collect()
}

/// Reads the ordered deletion effects from the selected SQL operation.
fn readDeletions(
    query: &mut ChatPayloadQuery<'_>,
    opId: &str,
) -> Result<Vec<ChatSyncDeletion>, SqliteStoreError> {
    query(
            r#"
            SELECT tableName, chatId, messageTimestamp, variantIndex
            FROM sync_sql_deletions
            WHERE opId = ?1
            ORDER BY ordinal
            "#,
            sqliteParams![opId],
        )?
        .into_iter()
        .map(|row| {
            Ok(ChatSyncDeletion {
                tableName: row.get(0)?,
                chatId: row.get(1)?,
                messageTimestamp: row.get(2)?,
                variantIndex: row.get(3)?,
            })
        })
        .collect()
}

/// Records only complete message revisions, rejecting malformed writes within their transaction.
fn insertOperation(
    transaction: &mut SqliteTransaction<'_>,
    operation: &SyncOperation,
    payload: &ChatSyncPayload,
) -> Result<(), SqliteStoreError> {
    let revisions = payload
        .partRows
        .iter()
        .map(|part| {
            (
                part.chatId.as_str(),
                part.messageTimestamp,
                part.variantIndex,
            )
        })
        .collect::<BTreeSet<_>>();
    let required = payload
        .messageRows
        .iter()
        .flat_map(|message| {
            [
                (message.chatId.as_str(), message.timestamp, 0),
                (
                    message.chatId.as_str(),
                    message.timestamp,
                    message.selectedVariantIndex,
                ),
            ]
        })
        .chain(payload.variantRows.iter().map(|variant| {
            (
                variant.chatId.as_str(),
                variant.messageTimestamp,
                variant.variantIndex,
            )
        }));
    for (chatId, timestamp, variantIndex) in required {
        if !revisions.contains(&(chatId, timestamp, variantIndex)) {
            return Err(SqliteStoreError::Message(format!(
                "Missing parts for message {chatId}:{timestamp} variant {variantIndex}"
            )));
        }
    }
    mergeOlderEntityStates(transaction, operation)?;
    transaction.execute(
        r#"
        INSERT INTO sync_sql_operations (
            opId, originDeviceId, sequence, domain, entityType, entityId,
            operation, semantics, createdAt, schemaVersion
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#,
        sqliteParams![
            operation.opId,
            operation.originDeviceId,
            operation.sequence,
            operation.domain,
            operation.entityType,
            operation.entityId,
            operation.operation,
            operation.semantics.storageValue(),
            operation.createdAt,
            operation.schemaVersion,
        ],
    )?;
    for chat in &payload.chatRows {
        insertChatSyncRow(transaction, &operation.opId, chat)?;
    }
    for message in &payload.messageRows {
        insertMessageSyncRow(transaction, &operation.opId, message)?;
    }
    for part in &payload.partRows {
        insertPartSyncRow(transaction, &operation.opId, part)?;
    }
    for variant in &payload.variantRows {
        insertVariantSyncRow(transaction, &operation.opId, variant)?;
    }
    for (index, deletion) in payload.deletions.iter().enumerate() {
        transaction.execute(
            r#"
            INSERT INTO sync_sql_deletions (
                opId, ordinal, tableName, chatId, messageTimestamp, variantIndex
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            sqliteParams![
                operation.opId,
                index as i32,
                deletion.tableName,
                deletion.chatId,
                deletion.messageTimestamp,
                deletion.variantIndex,
            ],
        )?;
    }
    Ok(())
}

/// Stores a complete chat record in the real sync operation payload table.
fn insertChatSyncRow(
    transaction: &mut SqliteTransaction<'_>,
    opId: &str,
    chat: &ChatEntity,
) -> Result<(), SqliteStoreError> {
    transaction.execute("INSERT INTO sync_sql_chat_rows (opId, id, title, createdAt, updatedAt, inputTokens, outputTokens, currentWindowSize, displayOrder, workspaceId, parentChatId, locked, pinned, pluginExtensions, \"group\") VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)", sqliteParams![opId, chat.id, chat.title, chat.createdAt, chat.updatedAt, chat.inputTokens, chat.outputTokens, chat.currentWindowSize, chat.displayOrder, chat.workspaceId, chat.parentChatId, chat.locked, chat.pinned, encodePluginExtensions(&chat.pluginExtensions)?, chat.group])?;
    Ok(())
}

/// Stores a complete message record in the real sync operation payload table.
fn insertMessageSyncRow(
    transaction: &mut SqliteTransaction<'_>,
    opId: &str,
    message: &MessageEntity,
) -> Result<(), SqliteStoreError> {
    transaction.execute("INSERT INTO sync_sql_message_rows (opId, chatId, sender, timestamp, orderIndex, roleName, selectedVariantIndex, provider, modelName, inputTokens, outputTokens, cachedInputTokens, sentAt, outputDurationMs, waitDurationMs, completedAt, completedExecutionGeneration, displayMode, isFavorite, pluginExtensions) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)", sqliteParams![opId, message.chatId, message.sender, message.timestamp, message.orderIndex, message.roleName, message.selectedVariantIndex, message.provider, message.modelName, message.inputTokens, message.outputTokens, message.cachedInputTokens, message.sentAt, message.outputDurationMs, message.waitDurationMs, message.completedAt, message.completedExecutionGeneration, message.displayMode, message.isFavorite, encodePluginExtensions(&message.pluginExtensions)?])?;
    Ok(())
}

/// Inserts one canonical message part into a pending sync operation.
fn insertPartSyncRow(
    transaction: &mut SqliteTransaction<'_>,
    opId: &str,
    part: &MessagePartEntity,
) -> Result<(), SqliteStoreError> {
    let attributesJson = serde_json::to_string(&part.attributes).map_err(|error| {
        SqliteStoreError::Message(format!("message-part attributes cannot serialize: {error}"))
    })?;
    transaction.execute(
        r#"
        INSERT INTO sync_sql_message_part_rows (
            opId, chatId, messageTimestamp, variantIndex, partId, sequence, kind,
            content, toolCallId, toolName, attributesJson
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#,
        sqliteParams![
            opId,
            part.chatId,
            part.messageTimestamp,
            part.variantIndex,
            part.partId,
            part.sequence,
            messagePartKindLabel(&part.kind),
            part.content,
            part.toolCallId,
            part.toolName,
            attributesJson,
        ],
    )?;
    Ok(())
}

/// Stores a complete variant record in the real sync operation payload table.
fn insertVariantSyncRow(
    transaction: &mut SqliteTransaction<'_>,
    opId: &str,
    variant: &MessageVariantEntity,
) -> Result<(), SqliteStoreError> {
    transaction.execute("INSERT INTO sync_sql_message_variant_rows (opId, chatId, messageTimestamp, variantIndex, roleName, provider, modelName, inputTokens, outputTokens, cachedInputTokens, sentAt, outputDurationMs, waitDurationMs, completedAt, pluginExtensions) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)", sqliteParams![opId, variant.chatId, variant.messageTimestamp, variant.variantIndex, variant.roleName, variant.provider, variant.modelName, variant.inputTokens, variant.outputTokens, variant.cachedInputTokens, variant.sentAt, variant.outputDurationMs, variant.waitDurationMs, variant.completedAt, encodePluginExtensions(&variant.pluginExtensions)?])?;
    Ok(())
}

/// Restores subsequent chat mutations after correcting an already applied historical operation.
fn laterChatOperations(
    transaction: &mut SqliteTransaction<'_>,
    replaced: &SyncOperation,
    payload: &ChatSyncPayload,
) -> Result<Vec<SyncOperation>, SqliteStoreError> {
    let chats = payload.chatRows.iter().map(|row| &row.id)
        .chain(payload.messageRows.iter().map(|row| &row.chatId))
        .chain(payload.partRows.iter().map(|row| &row.chatId))
        .chain(payload.variantRows.iter().map(|row| &row.chatId))
        .chain(payload.deletions.iter().map(|row| &row.chatId))
        .collect::<BTreeSet<_>>();
    let mut later = BTreeMap::new();
    for chatId in chats {
        for row in transaction.queryRows(
            r#"SELECT DISTINCT o.* FROM sync_sql_operations o
            WHERE o.opId <> ?1 AND o.domain = 'chat'
              AND ((o.originDeviceId = ?2 AND o.sequence > ?3)
                OR (o.originDeviceId <> ?2 AND (o.createdAt, o.originDeviceId, o.sequence) > (?4, ?2, ?3)))
              AND o.opId IN (
                SELECT opId FROM sync_sql_chat_rows WHERE id = ?5
                UNION SELECT opId FROM sync_sql_message_rows WHERE chatId = ?5
                UNION SELECT opId FROM sync_sql_message_part_rows WHERE chatId = ?5
                UNION SELECT opId FROM sync_sql_message_variant_rows WHERE chatId = ?5
                UNION SELECT opId FROM sync_sql_deletions WHERE chatId = ?5)"#,
            sqliteParams![replaced.opId, replaced.originDeviceId, replaced.sequence, replaced.createdAt, chatId],
        )? {
            let operation = mapOperationMetadata(&row)?;
            later.insert(operation.opId.clone(), operation);
        }
    }
    let mut origins = BTreeMap::<String, BTreeMap<i64, SyncOperation>>::new();
    for operation in later.into_values() {
        origins.entry(operation.originDeviceId.clone()).or_default().insert(operation.sequence, operation);
    }
    let mut ordered = Vec::new();
    while !origins.is_empty() {
        let origin = origins.iter().min_by_key(|(_, operations)| {
            let operation = operations.first_key_value().expect("nonempty origin").1;
            (operation.createdAt, &operation.originDeviceId)
        }).expect("pending origins").0.clone();
        let operations = origins.get_mut(&origin).expect("selected origin");
        ordered.push(operations.pop_first().expect("nonempty origin").1);
        if operations.is_empty() { origins.remove(&origin); }
    }
    Ok(ordered)
}

/// Applies one complete chat payload within the caller's SQL transaction.
fn applyPayload(
    transaction: &mut SqliteTransaction<'_>,
    operation: &SyncOperation,
    payload: &ChatSyncPayload,
) -> Result<(), SqliteStoreError> {
    for deletion in &payload.deletions {
        applyDeletion(transaction, deletion)?;
    }
    for chat in &payload.chatRows {
        upsertChat(transaction, chat)?;
    }
    for message in &payload.messageRows {
        upsertMessage(transaction, message)?;
    }
    for variant in &payload.variantRows {
        upsertVariant(transaction, variant)?;
    }
    replacePartRows(transaction, &payload.partRows)?;
    Ok(())
}

fn applyDeletion(
    transaction: &mut SqliteTransaction<'_>,
    deletion: &ChatSyncDeletion,
) -> Result<(), SqliteStoreError> {
    match deletion.tableName.as_str() {
        DELETE_CHAT => {
            transaction.execute(
                "DELETE FROM chats WHERE id = ?1",
                sqliteParams![deletion.chatId],
            )?;
        }
        DELETE_MESSAGE => {
            let timestamp = requiredTimestamp(deletion)?;
            transaction.execute(
                "DELETE FROM message_parts WHERE chatId = ?1 AND messageTimestamp = ?2",
                sqliteParams![deletion.chatId, timestamp],
            )?;
            transaction.execute(
                "DELETE FROM messages WHERE chatId = ?1 AND timestamp = ?2",
                sqliteParams![deletion.chatId, timestamp],
            )?;
        }
        DELETE_MESSAGES_FROM => {
            let timestamp = requiredTimestamp(deletion)?;
            transaction.execute(
                "DELETE FROM message_parts WHERE chatId = ?1 AND messageTimestamp >= ?2",
                sqliteParams![deletion.chatId, timestamp],
            )?;
            transaction.execute(
                "DELETE FROM messages WHERE chatId = ?1 AND timestamp >= ?2",
                sqliteParams![deletion.chatId, timestamp],
            )?;
        }
        DELETE_MESSAGES_FOR_CHAT => {
            transaction.execute(
                "DELETE FROM message_parts WHERE chatId = ?1",
                sqliteParams![deletion.chatId],
            )?;
            transaction.execute(
                "DELETE FROM messages WHERE chatId = ?1",
                sqliteParams![deletion.chatId],
            )?;
        }
        DELETE_VARIANT => {
            let timestamp = requiredTimestamp(deletion)?;
            let variantIndex = requiredVariantIndex(deletion)?;
            transaction.execute(
                "DELETE FROM message_parts WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
                sqliteParams![deletion.chatId, timestamp, variantIndex],
            )?;
            transaction.execute(
                "DELETE FROM message_variants WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
                sqliteParams![deletion.chatId, timestamp, variantIndex],
            )?;
        }
        DELETE_VARIANTS_FROM => {
            let timestamp = requiredTimestamp(deletion)?;
            transaction.execute(
                "DELETE FROM message_parts WHERE chatId = ?1 AND messageTimestamp >= ?2 AND variantIndex > 0",
                sqliteParams![deletion.chatId, timestamp],
            )?;
            transaction.execute(
                "DELETE FROM message_variants WHERE chatId = ?1 AND messageTimestamp >= ?2",
                sqliteParams![deletion.chatId, timestamp],
            )?;
        }
        DELETE_VARIANTS_FOR_MESSAGE => {
            let timestamp = requiredTimestamp(deletion)?;
            transaction.execute(
                "DELETE FROM message_parts WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex > 0",
                sqliteParams![deletion.chatId, timestamp],
            )?;
            transaction.execute(
                "DELETE FROM message_variants WHERE chatId = ?1 AND messageTimestamp = ?2",
                sqliteParams![deletion.chatId, timestamp],
            )?;
        }
        DELETE_VARIANTS_FOR_CHAT => {
            transaction.execute(
                "DELETE FROM message_parts WHERE chatId = ?1 AND variantIndex > 0",
                sqliteParams![deletion.chatId],
            )?;
            transaction.execute(
                "DELETE FROM message_variants WHERE chatId = ?1",
                sqliteParams![deletion.chatId],
            )?;
        }
        other => {
            return Err(SqliteStoreError::Message(format!(
                "unknown sync deletion table: {other}"
            )));
        }
    }
    Ok(())
}

/// Applies a complete authoritative synced chat record including its namespace objects.
fn upsertChat(
    transaction: &mut SqliteTransaction<'_>,
    chat: &ChatEntity,
) -> Result<(), SqliteStoreError> {
    transaction.execute("INSERT INTO chats (id, title, createdAt, updatedAt, inputTokens, outputTokens, currentWindowSize, displayOrder, workspaceId, parentChatId, locked, pinned, pluginExtensions, \"group\") VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14) ON CONFLICT(id) DO UPDATE SET title = excluded.title, createdAt = excluded.createdAt, updatedAt = excluded.updatedAt, inputTokens = excluded.inputTokens, outputTokens = excluded.outputTokens, currentWindowSize = excluded.currentWindowSize, displayOrder = excluded.displayOrder, workspaceId = excluded.workspaceId, parentChatId = excluded.parentChatId, locked = excluded.locked, pinned = excluded.pinned, pluginExtensions = excluded.pluginExtensions, \"group\" = excluded.\"group\"", sqliteParams![chat.id, chat.title, chat.createdAt, chat.updatedAt, chat.inputTokens, chat.outputTokens, chat.currentWindowSize, chat.displayOrder, chat.workspaceId, chat.parentChatId, chat.locked, chat.pinned, encodePluginExtensions(&chat.pluginExtensions)?, chat.group])?;
    Ok(())
}

/// Applies a complete authoritative synced message record including its namespace objects.
fn upsertMessage(
    transaction: &mut SqliteTransaction<'_>,
    message: &MessageEntity,
) -> Result<(), SqliteStoreError> {
    transaction.execute(
        "DELETE FROM message_parts WHERE chatId = ?1 AND messageTimestamp = ?2",
        sqliteParams![message.chatId, message.timestamp],
    )?;
    transaction.execute("INSERT INTO messages (chatId, sender, timestamp, orderIndex, roleName, selectedVariantIndex, provider, modelName, inputTokens, outputTokens, cachedInputTokens, sentAt, outputDurationMs, waitDurationMs, completedAt, completedExecutionGeneration, displayMode, isFavorite, pluginExtensions) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19) ON CONFLICT(chatId, timestamp) DO UPDATE SET sender = excluded.sender, orderIndex = excluded.orderIndex, roleName = excluded.roleName, selectedVariantIndex = excluded.selectedVariantIndex, provider = excluded.provider, modelName = excluded.modelName, inputTokens = excluded.inputTokens, outputTokens = excluded.outputTokens, cachedInputTokens = excluded.cachedInputTokens, sentAt = excluded.sentAt, outputDurationMs = excluded.outputDurationMs, waitDurationMs = excluded.waitDurationMs, completedAt = excluded.completedAt, completedExecutionGeneration = excluded.completedExecutionGeneration, displayMode = excluded.displayMode, isFavorite = excluded.isFavorite, pluginExtensions = excluded.pluginExtensions", sqliteParams![message.chatId, message.sender, message.timestamp, message.orderIndex, message.roleName, message.selectedVariantIndex, message.provider, message.modelName, message.inputTokens, message.outputTokens, message.cachedInputTokens, message.sentAt, message.outputDurationMs, message.waitDurationMs, message.completedAt, message.completedExecutionGeneration, message.displayMode, message.isFavorite, encodePluginExtensions(&message.pluginExtensions)?])?;
    Ok(())
}

/// Applies a complete authoritative synced variant record including its namespace objects.
fn upsertVariant(
    transaction: &mut SqliteTransaction<'_>,
    variant: &MessageVariantEntity,
) -> Result<(), SqliteStoreError> {
    transaction.execute("INSERT INTO message_variants (chatId, messageTimestamp, variantIndex, roleName, provider, modelName, inputTokens, outputTokens, cachedInputTokens, sentAt, outputDurationMs, waitDurationMs, completedAt, pluginExtensions) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14) ON CONFLICT(chatId, messageTimestamp, variantIndex) DO UPDATE SET roleName = excluded.roleName, provider = excluded.provider, modelName = excluded.modelName, inputTokens = excluded.inputTokens, outputTokens = excluded.outputTokens, cachedInputTokens = excluded.cachedInputTokens, sentAt = excluded.sentAt, outputDurationMs = excluded.outputDurationMs, waitDurationMs = excluded.waitDurationMs, completedAt = excluded.completedAt, pluginExtensions = excluded.pluginExtensions", sqliteParams![variant.chatId, variant.messageTimestamp, variant.variantIndex, variant.roleName, variant.provider, variant.modelName, variant.inputTokens, variant.outputTokens, variant.cachedInputTokens, variant.sentAt, variant.outputDurationMs, variant.waitDurationMs, variant.completedAt, encodePluginExtensions(&variant.pluginExtensions)?])?;
    Ok(())
}

/// Replaces every structured revision contained in one received sync payload.
fn replacePartRows(
    transaction: &mut SqliteTransaction<'_>,
    parts: &[MessagePartEntity],
) -> Result<(), SqliteStoreError> {
    let mut groups = BTreeMap::<(String, i64, i32), Vec<&MessagePartEntity>>::new();
    for part in parts {
        groups
            .entry((
                part.chatId.clone(),
                part.messageTimestamp,
                part.variantIndex,
            ))
            .or_default()
            .push(part);
    }
    for ((chatId, timestamp, variantIndex), parts) in groups {
        transaction.execute(
            "DELETE FROM message_parts WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
            sqliteParams![chatId, timestamp, variantIndex],
        )?;
        for part in parts {
            let attributesJson = serde_json::to_string(&part.attributes).map_err(|error| {
                SqliteStoreError::Message(format!(
                    "message-part attributes cannot serialize: {error}"
                ))
            })?;
            transaction.execute(
                r#"
                INSERT INTO message_parts (
                    chatId, messageTimestamp, variantIndex, partId, sequence, kind, content,
                    toolCallId, toolName, attributesJson
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                sqliteParams![
                    part.chatId,
                    part.messageTimestamp,
                    part.variantIndex,
                    part.partId,
                    part.sequence,
                    messagePartKindLabel(&part.kind),
                    part.content,
                    part.toolCallId,
                    part.toolName,
                    attributesJson,
                ],
            )?;
        }
    }
    Ok(())
}

/// Converts a canonical message-part kind into its SQL label.
fn messagePartKindLabel(kind: &MessagePartKind) -> &'static str {
    match kind {
        MessagePartKind::Markdown => "markdown",
        MessagePartKind::Thinking => "thinking",
        MessagePartKind::ToolCall => "tool_call",
        MessagePartKind::ToolResult => "tool_result",
        MessagePartKind::Status => "status",
    }
}

/// Converts one persisted SQL part label into its canonical message-part kind.
fn messagePartKindFromLabel(value: &str) -> Result<MessagePartKind, SqliteStoreError> {
    match value {
        "markdown" => Ok(MessagePartKind::Markdown),
        "thinking" => Ok(MessagePartKind::Thinking),
        "tool_call" => Ok(MessagePartKind::ToolCall),
        "tool_result" => Ok(MessagePartKind::ToolResult),
        "status" => Ok(MessagePartKind::Status),
        _ => Err(SqliteStoreError::Message(format!(
            "unknown message-part kind: {value}"
        ))),
    }
}

/// Reports whether a newer replaceable state for this entity is already stored.
fn hasNewerMergedEntityState(
    transaction: &mut SqliteTransaction<'_>,
    operation: &SyncOperation,
) -> Result<bool, SqliteStoreError> {
    if operation.semantics != SyncOperationSemantics::EntityState {
        return Ok(false);
    }
    Ok(transaction
        .queryOne(
            r#"
            SELECT 1 FROM sync_sql_operations
            WHERE originDeviceId = ?1
                AND domain = ?2
                AND entityType = ?3
                AND entityId = ?4
                AND semantics = ?5
                AND sequence > ?6
            LIMIT 1
            "#,
            sqliteParams![
                operation.originDeviceId,
                operation.domain,
                operation.entityType,
                operation.entityId,
                SyncOperationSemantics::EntityState.storageValue(),
                operation.sequence,
            ],
        )?
        .is_some())
}

/// Removes older replaceable states for the same entity and origin.
fn mergeOlderEntityStates(
    transaction: &mut SqliteTransaction<'_>,
    operation: &SyncOperation,
) -> Result<(), SqliteStoreError> {
    if operation.semantics != SyncOperationSemantics::EntityState {
        return Ok(());
    }
    transaction.execute(
        r#"
        DELETE FROM sync_sql_operations
        WHERE originDeviceId = ?1
            AND domain = ?2
            AND entityType = ?3
            AND entityId = ?4
            AND semantics = ?5
            AND sequence < ?6
        "#,
        sqliteParams![
            operation.originDeviceId,
            operation.domain,
            operation.entityType,
            operation.entityId,
            SyncOperationSemantics::EntityState.storageValue(),
            operation.sequence,
        ],
    )?;
    Ok(())
}

fn sequenceFor(
    transaction: &mut SqliteTransaction<'_>,
    originDeviceId: &str,
) -> Result<i64, SqliteStoreError> {
    let sequence = transaction
        .queryOne(
            "SELECT sequence FROM sync_sql_clocks WHERE originDeviceId = ?1",
            sqliteParams![originDeviceId],
        )?
        .map(|row| row.get(0))
        .transpose()?;
    Ok(sequence.unwrap_or(0))
}

fn observeOperation(
    transaction: &mut SqliteTransaction<'_>,
    operation: &SyncOperation,
) -> Result<(), SqliteStoreError> {
    let current = sequenceFor(transaction, &operation.originDeviceId)?;
    if operation.sequence > current {
        transaction.execute(
            r#"
            INSERT INTO sync_sql_clocks(originDeviceId, sequence)
            VALUES (?1, ?2)
            ON CONFLICT(originDeviceId) DO UPDATE SET sequence = excluded.sequence
            "#,
            sqliteParams![operation.originDeviceId, operation.sequence],
        )?;
    }
    Ok(())
}

fn requiredTimestamp(deletion: &ChatSyncDeletion) -> Result<i64, SqliteStoreError> {
    deletion.messageTimestamp.ok_or_else(|| {
        SqliteStoreError::Message(format!(
            "missing messageTimestamp for {}",
            deletion.tableName
        ))
    })
}

fn requiredVariantIndex(deletion: &ChatSyncDeletion) -> Result<i32, SqliteStoreError> {
    deletion.variantIndex.ok_or_else(|| {
        SqliteStoreError::Message(format!("missing variantIndex for {}", deletion.tableName))
    })
}

fn currentTimeMillis() -> Result<i64, SqlChatSyncStoreError> {
    operit_host_api::TimeUtils::tryCurrentTimeMillis().map_err(SqlChatSyncStoreError::Message)
}

#[cfg(test)]
#[path = "SqlChatSyncStoreTests/mod.rs"]
pub(crate) mod tests;

#[path = "SqlChatExtensionTransactions.rs"]
mod extension_transactions;
