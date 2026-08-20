use std::collections::{BTreeMap, BTreeSet};

use crate::sqliteParams;
use crate::RuntimeStorePaths::RuntimeStorePaths;
use crate::SqliteStore::{
    SqliteRow, SqliteRowGet, SqliteStore, SqliteStoreError, SqliteTransaction,
};
use crate::SyncOperationStore::{
    compactSyncOperations, publishSyncMutation, SyncClock, SyncOperation, SyncOperationSemantics,
    SyncOperationStore, SyncOperationStoreError,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::dao::ChatDao::ChatDao;
use crate::dao::MessageDao::MessageDao;
use crate::dao::MessagePartDao::MessagePartDao;
use crate::dao::MessageVariantDao::MessageVariantDao;
use crate::db::AppDatabase::{AppDatabase, AppDatabaseError};
use operit_model::ChatEntity::ChatEntity;
use operit_model::MessageEntity::MessageEntity;
use operit_model::MessagePart::MessagePartKind;
use operit_model::MessagePartEntity::MessagePartEntity;
use operit_model::MessageVariantEntity::MessageVariantEntity;

/// Sync domain used for SQL-backed chat history operations.
pub const CHAT_SYNC_DOMAIN: &str = "chat";

const CHAT_SYNC_OPERATION_SCHEMA_VERSION: i32 = 5;

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

        let variantDao = MessageVariantDao::new(self.store.clone());
        let partDao = MessagePartDao::new(self.store.clone());
        let variantRows = variantDao.getVariantsForMessage(&chat.id, message.timestamp)?;
        let mut partRows = partDao
            .getPartsForMessages(&chat.id, vec![message.timestamp])?
            .into_iter()
            .filter(|part| part.variantIndex != 0)
            .collect::<Vec<_>>();
        partRows.extend(baseParts);
        let payload = ChatSyncPayload {
            chatRows: vec![chat.clone()],
            messageRows: vec![message.clone()],
            partRows,
            variantRows,
            deletions: Vec::new(),
        };
        let payloadValue = serde_json::to_value(&payload)?;
        let createdAt = currentTimeMillis()?;
        self.store.transaction(|transaction| {
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

    /// Records deletion of one message and its variants.
    pub fn recordMessageDeletion(
        &self,
        chatId: &str,
        timestamp: i64,
    ) -> Result<(), SqlChatSyncStoreError> {
        let mut payload = self.payloadForChatMetadata(chatId)?;
        payload.deletions.push(ChatSyncDeletion {
            tableName: DELETE_VARIANTS_FOR_MESSAGE.to_string(),
            chatId: chatId.to_string(),
            messageTimestamp: Some(timestamp),
            variantIndex: None,
        });
        payload.deletions.push(ChatSyncDeletion {
            tableName: DELETE_MESSAGE.to_string(),
            chatId: chatId.to_string(),
            messageTimestamp: Some(timestamp),
            variantIndex: None,
        });
        self.appendLocalOperation(
            "message",
            &format!("{chatId}:{timestamp}"),
            "delete",
            SyncOperationSemantics::Transaction,
            payload,
        )?;
        Ok(())
    }

    /// Records deletion of messages from a timestamp onward.
    pub fn recordMessagesFromDeletion(
        &self,
        chatId: &str,
        timestamp: i64,
    ) -> Result<(), SqlChatSyncStoreError> {
        let mut payload = self.payloadForChatMetadata(chatId)?;
        payload.deletions.push(ChatSyncDeletion {
            tableName: DELETE_VARIANTS_FROM.to_string(),
            chatId: chatId.to_string(),
            messageTimestamp: Some(timestamp),
            variantIndex: None,
        });
        payload.deletions.push(ChatSyncDeletion {
            tableName: DELETE_MESSAGES_FROM.to_string(),
            chatId: chatId.to_string(),
            messageTimestamp: Some(timestamp),
            variantIndex: None,
        });
        self.appendLocalOperation(
            "messages",
            &format!("{chatId}:{timestamp}"),
            "delete",
            SyncOperationSemantics::Transaction,
            payload,
        )?;
        Ok(())
    }

    /// Records deletion of all messages and variants for one chat.
    pub fn recordAllMessagesForChatDeletion(
        &self,
        chatId: &str,
    ) -> Result<(), SqlChatSyncStoreError> {
        let mut payload = self.payloadForChatMetadata(chatId)?;
        payload.deletions.push(ChatSyncDeletion {
            tableName: DELETE_VARIANTS_FOR_CHAT.to_string(),
            chatId: chatId.to_string(),
            messageTimestamp: None,
            variantIndex: None,
        });
        payload.deletions.push(ChatSyncDeletion {
            tableName: DELETE_MESSAGES_FOR_CHAT.to_string(),
            chatId: chatId.to_string(),
            messageTimestamp: None,
            variantIndex: None,
        });
        self.appendLocalOperation(
            "messages",
            chatId,
            "delete",
            SyncOperationSemantics::Transaction,
            payload,
        )?;
        Ok(())
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

    /// Lists compacted SQL chat operations newer than the supplied clock.
    pub fn operationsSince(
        &self,
        clock: &SyncClock,
        domains: &[String],
        limit: usize,
    ) -> Result<Vec<SyncOperation>, SqlChatSyncStoreError> {
        let domainSet = domains.iter().cloned().collect::<BTreeSet<_>>();
        if !domainSet.is_empty() && !domainSet.contains(CHAT_SYNC_DOMAIN) {
            return Ok(Vec::new());
        }
        let rows = self.store.queryRows(
            r#"
            SELECT opId, originDeviceId, sequence, domain, entityType, entityId,
                operation, semantics, createdAt, schemaVersion
            FROM sync_sql_operations
            WHERE domain = ?1
            ORDER BY createdAt ASC, originDeviceId ASC, sequence ASC
            "#,
            sqliteParams![CHAT_SYNC_DOMAIN],
        )?;
        let mut operations = Vec::new();
        for row in rows {
            let operation = mapOperationMetadata(&row)?;
            if operation.sequence <= clock.sequenceFor(&operation.originDeviceId) {
                continue;
            }
            if operation.sequence
                <= self
                    .syncOperationStore
                    .exportFloorFor(&operation.originDeviceId)?
            {
                continue;
            }
            operations.push(operation);
        }
        let mut operations = compactSyncOperations(operations);
        operations.truncate(limit);
        for operation in &mut operations {
            operation.payload = serde_json::to_value(readPayload(&self.store, &operation.opId)?)?;
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
        let payload = decodePayload(operation)?;
        let didApply = self.store.transaction(|transaction| {
            if operation.sequence <= sequenceFor(transaction, &operation.originDeviceId)? {
                return Ok(false);
            }
            if operationExists(transaction, &operation.opId)? {
                observeOperation(transaction, operation)?;
                return Ok(false);
            }
            if !ignoreNewerEntityState && hasNewerMergedEntityState(transaction, operation)? {
                observeOperation(transaction, operation)?;
                return Ok(false);
            }
            applyPayload(transaction, operation, &payload)?;
            insertOperation(transaction, operation, &payload)?;
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
        let payloadValue = serde_json::to_value(&payload)?;
        let createdAt = currentTimeMillis()?;
        let operation = self.store.transaction(|transaction| {
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
                payload: payloadValue,
                createdAt,
                schemaVersion: CHAT_SYNC_OPERATION_SCHEMA_VERSION,
            };
            insertOperation(transaction, &operation, &payload)?;
            observeOperation(transaction, &operation)?;
            Ok(operation)
        })?;
        publishSyncMutation();
        Ok(operation)
    }

    /// Builds a payload containing only chat metadata.
    fn payloadForChatMetadata(
        &self,
        chatId: &str,
    ) -> Result<ChatSyncPayload, SqlChatSyncStoreError> {
        let chatDao = ChatDao::new(self.store.clone());
        let chatRows = chatDao.getChatById(chatId)?.into_iter().collect();
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
        let partDao = MessagePartDao::new(self.store.clone());
        let variantDao = MessageVariantDao::new(self.store.clone());
        let chatRows = chatDao.getChatById(chatId)?.into_iter().collect::<Vec<_>>();
        let messageRows = messageDao.getMessagesForChat(chatId)?;
        let partRows = partDao.getPartsForChat(chatId)?;
        let variantRows = variantDao.getVariantsForChat(chatId)?;
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
        let partDao = MessagePartDao::new(self.store.clone());
        let variantDao = MessageVariantDao::new(self.store.clone());
        let chatRows = chatDao.getChatById(chatId)?.into_iter().collect::<Vec<_>>();
        let messageRows = messageDao
            .getMessageByTimestamp(chatId, timestamp)?
            .into_iter()
            .collect::<Vec<_>>();
        let variantRows = variantDao.getVariantsForMessage(chatId, timestamp)?;
        let partRows = partDao.getPartsForMessages(chatId, vec![timestamp])?;
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

fn readPayload(store: &SqliteStore, opId: &str) -> Result<ChatSyncPayload, SqliteStoreError> {
    Ok(ChatSyncPayload {
        chatRows: readChatRows(store, opId)?,
        messageRows: readMessageRows(store, opId)?,
        partRows: readPartRows(store, opId)?,
        variantRows: readVariantRows(store, opId)?,
        deletions: readDeletions(store, opId)?,
    })
}

fn readChatRows(store: &SqliteStore, opId: &str) -> Result<Vec<ChatEntity>, SqliteStoreError> {
    store
        .queryRows(
            r#"
            SELECT id, title, createdAt, updatedAt, inputTokens, outputTokens,
                currentWindowSize, "group", displayOrder, workspace,
                parentChatId, characterCardName, characterGroupId, locked, pinned
            FROM sync_sql_chat_rows
            WHERE opId = ?1
            ORDER BY id
            "#,
            sqliteParams![opId],
        )?
        .into_iter()
        .map(|row| {
            Ok(ChatEntity {
                id: row.get(0)?,
                title: row.get(1)?,
                createdAt: row.get(2)?,
                updatedAt: row.get(3)?,
                inputTokens: row.get(4)?,
                outputTokens: row.get(5)?,
                currentWindowSize: row.get(6)?,
                group: row.get(7)?,
                displayOrder: row.get(8)?,
                workspace: row.get(9)?,
                parentChatId: row.get(10)?,
                characterCardName: row.get(11)?,
                characterGroupId: row.get(12)?,
                locked: row.get(13)?,
                pinned: row.get(14)?,
            })
        })
        .collect()
}

fn readMessageRows(
    store: &SqliteStore,
    opId: &str,
) -> Result<Vec<MessageEntity>, SqliteStoreError> {
    store
        .queryRows(
            r#"
            SELECT chatId, sender, timestamp, orderIndex, roleName,
                selectedVariantIndex, provider, modelName, inputTokens, outputTokens,
                cachedInputTokens, sentAt, outputDurationMs, waitDurationMs,
                completedAt, completedExecutionGeneration, displayMode, isFavorite
            FROM sync_sql_message_rows
            WHERE opId = ?1
            ORDER BY chatId, timestamp
            "#,
            sqliteParams![opId],
        )?
        .into_iter()
        .map(|row| {
            Ok(MessageEntity {
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
            })
        })
        .collect()
}

fn readVariantRows(
    store: &SqliteStore,
    opId: &str,
) -> Result<Vec<MessageVariantEntity>, SqliteStoreError> {
    store
        .queryRows(
            r#"
            SELECT chatId, messageTimestamp, variantIndex, roleName,
                provider, modelName, inputTokens, outputTokens, cachedInputTokens,
                sentAt, outputDurationMs, waitDurationMs, completedAt
            FROM sync_sql_message_variant_rows
            WHERE opId = ?1
            ORDER BY chatId, messageTimestamp, variantIndex
            "#,
            sqliteParams![opId],
        )?
        .into_iter()
        .map(|row| {
            Ok(MessageVariantEntity {
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
            })
        })
        .collect()
}

/// Reads structured part rows stored for one pending sync operation.
fn readPartRows(
    store: &SqliteStore,
    opId: &str,
) -> Result<Vec<MessagePartEntity>, SqliteStoreError> {
    store
        .queryRows(
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

fn readDeletions(
    store: &SqliteStore,
    opId: &str,
) -> Result<Vec<ChatSyncDeletion>, SqliteStoreError> {
    store
        .queryRows(
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

fn insertOperation(
    transaction: &mut SqliteTransaction<'_>,
    operation: &SyncOperation,
    payload: &ChatSyncPayload,
) -> Result<(), SqliteStoreError> {
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

fn insertChatSyncRow(
    transaction: &mut SqliteTransaction<'_>,
    opId: &str,
    chat: &ChatEntity,
) -> Result<(), SqliteStoreError> {
    transaction.execute(
        r#"
        INSERT INTO sync_sql_chat_rows (
            opId, id, title, createdAt, updatedAt, inputTokens, outputTokens,
            currentWindowSize, "group", displayOrder, workspace,
            parentChatId, characterCardName, characterGroupId, locked, pinned
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
        "#,
        sqliteParams![
            opId,
            chat.id,
            chat.title,
            chat.createdAt,
            chat.updatedAt,
            chat.inputTokens,
            chat.outputTokens,
            chat.currentWindowSize,
            chat.group,
            chat.displayOrder,
            chat.workspace,
            chat.parentChatId,
            chat.characterCardName,
            chat.characterGroupId,
            chat.locked,
            chat.pinned,
        ],
    )?;
    Ok(())
}

fn insertMessageSyncRow(
    transaction: &mut SqliteTransaction<'_>,
    opId: &str,
    message: &MessageEntity,
) -> Result<(), SqliteStoreError> {
    transaction.execute(
        r#"
        INSERT INTO sync_sql_message_rows (
            opId, chatId, sender, timestamp, orderIndex, roleName,
            selectedVariantIndex, provider, modelName, inputTokens, outputTokens,
            cachedInputTokens, sentAt, outputDurationMs, waitDurationMs,
            completedAt, completedExecutionGeneration, displayMode, isFavorite
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
        "#,
        sqliteParams![
            opId,
            message.chatId,
            message.sender,
            message.timestamp,
            message.orderIndex,
            message.roleName,
            message.selectedVariantIndex,
            message.provider,
            message.modelName,
            message.inputTokens,
            message.outputTokens,
            message.cachedInputTokens,
            message.sentAt,
            message.outputDurationMs,
            message.waitDurationMs,
            message.completedAt,
            message.completedExecutionGeneration,
            message.displayMode,
            message.isFavorite,
        ],
    )?;
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

fn insertVariantSyncRow(
    transaction: &mut SqliteTransaction<'_>,
    opId: &str,
    variant: &MessageVariantEntity,
) -> Result<(), SqliteStoreError> {
    transaction.execute(
        r#"
        INSERT INTO sync_sql_message_variant_rows (
            opId, chatId, messageTimestamp, variantIndex, roleName,
            provider, modelName, inputTokens, outputTokens, cachedInputTokens,
            sentAt, outputDurationMs, waitDurationMs, completedAt
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        "#,
        sqliteParams![
            opId,
            variant.chatId,
            variant.messageTimestamp,
            variant.variantIndex,
            variant.roleName,
            variant.provider,
            variant.modelName,
            variant.inputTokens,
            variant.outputTokens,
            variant.cachedInputTokens,
            variant.sentAt,
            variant.outputDurationMs,
            variant.waitDurationMs,
            variant.completedAt,
        ],
    )?;
    Ok(())
}

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

fn upsertChat(
    transaction: &mut SqliteTransaction<'_>,
    chat: &ChatEntity,
) -> Result<(), SqliteStoreError> {
    transaction.execute(
        r#"
        INSERT INTO chats (
            id, title, createdAt, updatedAt, inputTokens, outputTokens,
            currentWindowSize, "group", displayOrder, workspace,
            parentChatId, characterCardName, characterGroupId, locked, pinned
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            createdAt = excluded.createdAt,
            updatedAt = excluded.updatedAt,
            inputTokens = excluded.inputTokens,
            outputTokens = excluded.outputTokens,
            currentWindowSize = excluded.currentWindowSize,
            "group" = excluded."group",
            displayOrder = excluded.displayOrder,
            workspace = excluded.workspace,
            parentChatId = excluded.parentChatId,
            characterCardName = excluded.characterCardName,
            characterGroupId = excluded.characterGroupId,
            locked = excluded.locked,
            pinned = excluded.pinned
        "#,
        sqliteParams![
            chat.id,
            chat.title,
            chat.createdAt,
            chat.updatedAt,
            chat.inputTokens,
            chat.outputTokens,
            chat.currentWindowSize,
            chat.group,
            chat.displayOrder,
            chat.workspace,
            chat.parentChatId,
            chat.characterCardName,
            chat.characterGroupId,
            chat.locked,
            chat.pinned,
        ],
    )?;
    Ok(())
}

fn upsertMessage(
    transaction: &mut SqliteTransaction<'_>,
    message: &MessageEntity,
) -> Result<(), SqliteStoreError> {
    transaction.execute(
        "DELETE FROM message_parts WHERE chatId = ?1 AND messageTimestamp = ?2",
        sqliteParams![message.chatId, message.timestamp],
    )?;
    transaction.execute(
        "DELETE FROM messages WHERE chatId = ?1 AND timestamp = ?2",
        sqliteParams![message.chatId, message.timestamp],
    )?;
    transaction.execute(
        r#"
        INSERT INTO messages (
            chatId, sender, timestamp, orderIndex, roleName,
            selectedVariantIndex, provider, modelName, inputTokens, outputTokens,
            cachedInputTokens, sentAt, outputDurationMs, waitDurationMs,
            completedAt, completedExecutionGeneration, displayMode, isFavorite
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
        "#,
        sqliteParams![
            message.chatId,
            message.sender,
            message.timestamp,
            message.orderIndex,
            message.roleName,
            message.selectedVariantIndex,
            message.provider,
            message.modelName,
            message.inputTokens,
            message.outputTokens,
            message.cachedInputTokens,
            message.sentAt,
            message.outputDurationMs,
            message.waitDurationMs,
            message.completedAt,
            message.completedExecutionGeneration,
            message.displayMode,
            message.isFavorite,
        ],
    )?;
    Ok(())
}

fn upsertVariant(
    transaction: &mut SqliteTransaction<'_>,
    variant: &MessageVariantEntity,
) -> Result<(), SqliteStoreError> {
    transaction.execute(
        "DELETE FROM message_variants WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
        sqliteParams![variant.chatId, variant.messageTimestamp, variant.variantIndex],
    )?;
    transaction.execute(
        r#"
        INSERT INTO message_variants (
            chatId, messageTimestamp, variantIndex, roleName, provider,
            modelName, inputTokens, outputTokens, cachedInputTokens, sentAt,
            outputDurationMs, waitDurationMs, completedAt
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        "#,
        sqliteParams![
            variant.chatId,
            variant.messageTimestamp,
            variant.variantIndex,
            variant.roleName,
            variant.provider,
            variant.modelName,
            variant.inputTokens,
            variant.outputTokens,
            variant.cachedInputTokens,
            variant.sentAt,
            variant.outputDurationMs,
            variant.waitDurationMs,
            variant.completedAt,
        ],
    )?;
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

fn operationExists(
    transaction: &mut SqliteTransaction<'_>,
    opId: &str,
) -> Result<bool, SqliteStoreError> {
    Ok(transaction
        .queryOne(
            "SELECT 1 FROM sync_sql_operations WHERE opId = ?1 LIMIT 1",
            sqliteParams![opId],
        )?
        .is_some())
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
mod tests;
