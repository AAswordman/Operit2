use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_store::sqliteParams;
use operit_store::SqliteStore::{
    SqliteRow, SqliteRowGet, SqliteStore, SqliteStoreError, SqliteTransaction,
};
use operit_store::SyncOperationStore::{
    compactSyncOperations, SyncClock, SyncOperation, SyncOperationStore, SyncOperationStoreError,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::data::dao::ChatDao::ChatDao;
use crate::data::dao::MessageDao::MessageDao;
use crate::data::dao::MessageVariantDao::MessageVariantDao;
use crate::data::db::AppDatabase::{AppDatabase, AppDatabaseError};
use crate::data::model::ChatEntity::ChatEntity;
use crate::data::model::MessageEntity::MessageEntity;
use crate::data::model::MessageVariantEntity::MessageVariantEntity;

pub const CHAT_SYNC_DOMAIN: &str = "chat";

const DELETE_CHAT: &str = "chats";
const DELETE_MESSAGE: &str = "messages";
const DELETE_MESSAGES_FROM: &str = "messages_from";
const DELETE_MESSAGES_FOR_CHAT: &str = "messages_for_chat";
const DELETE_VARIANT: &str = "message_variants";
const DELETE_VARIANTS_FROM: &str = "message_variants_from";
const DELETE_VARIANTS_FOR_MESSAGE: &str = "message_variants_for_message";
const DELETE_VARIANTS_FOR_CHAT: &str = "message_variants_for_chat";

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatSyncDeletion {
    pub tableName: String,
    pub chatId: String,
    pub messageTimestamp: Option<i64>,
    pub variantIndex: Option<i32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatSyncPayload {
    pub chatRows: Vec<ChatEntity>,
    pub messageRows: Vec<MessageEntity>,
    pub variantRows: Vec<MessageVariantEntity>,
    pub deletions: Vec<ChatSyncDeletion>,
}

#[derive(Clone)]
pub struct SqlChatSyncStore {
    store: SqliteStore,
    originDeviceId: String,
}

impl SqlChatSyncStore {
    pub fn new(paths: RuntimeStorePaths, database: &AppDatabase) -> Result<Self, SqlChatSyncStoreError> {
        let deviceId = SyncOperationStore::native(paths).localDeviceId()?;
        Ok(Self {
            store: database.store().clone(),
            originDeviceId: format!("{deviceId}:sql"),
        })
    }

    pub fn default() -> Result<Self, SqlChatSyncStoreError> {
        let database = AppDatabase::default()?;
        Self::new(RuntimeStorePaths::default(), &database)
    }

    pub fn recordChatMetadata(&self, chatId: &str) -> Result<(), SqlChatSyncStoreError> {
        let payload = self.payloadForChatMetadata(chatId)?;
        if payload.chatRows.is_empty() {
            return Ok(());
        }
        self.appendLocalOperation("chat", chatId, "upsert", payload)?;
        Ok(())
    }

    pub fn recordChatSnapshot(&self, chatId: &str) -> Result<(), SqlChatSyncStoreError> {
        let payload = self.payloadForChatSnapshot(chatId)?;
        if payload.chatRows.is_empty() {
            return Ok(());
        }
        self.appendLocalOperation("chat", chatId, "upsert", payload)?;
        Ok(())
    }

    pub fn recordMessageSnapshot(
        &self,
        chatId: &str,
        timestamp: i64,
    ) -> Result<(), SqlChatSyncStoreError> {
        let payload = self.payloadForMessageSnapshot(chatId, timestamp)?;
        if payload.chatRows.is_empty() && payload.messageRows.is_empty() && payload.variantRows.is_empty() {
            return Ok(());
        }
        self.appendLocalOperation("message", &format!("{chatId}:{timestamp}"), "upsert", payload)?;
        Ok(())
    }

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
        self.appendLocalOperation("chat", chatId, "delete", payload)?;
        Ok(())
    }

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
        self.appendLocalOperation("message", &format!("{chatId}:{timestamp}"), "delete", payload)?;
        Ok(())
    }

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
        self.appendLocalOperation("messages", &format!("{chatId}:{timestamp}"), "delete", payload)?;
        Ok(())
    }

    pub fn recordAllMessagesForChatDeletion(&self, chatId: &str) -> Result<(), SqlChatSyncStoreError> {
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
        self.appendLocalOperation("messages", chatId, "delete", payload)?;
        Ok(())
    }

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
                operation, createdAt, schemaVersion
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
            operations.push(operation);
        }
        let mut operations = compactSyncOperations(operations);
        operations.truncate(limit);
        for operation in &mut operations {
            operation.payload = serde_json::to_value(readPayload(&self.store, &operation.opId)?)?;
        }
        Ok(operations)
    }

    pub fn applyOperation(&self, operation: &SyncOperation) -> Result<(), SqlChatSyncStoreError> {
        let payload: ChatSyncPayload = serde_json::from_value(operation.payload.clone())?;
        let didApply = self.store.transaction(|transaction| {
            if operation.sequence <= sequenceFor(transaction, &operation.originDeviceId)? {
                return Ok(false);
            }
            if operationExists(transaction, &operation.opId)? {
                observeOperation(transaction, operation)?;
                return Ok(false);
            }
            if hasNewerMergedUpsert(transaction, operation)? {
                observeOperation(transaction, operation)?;
                return Ok(false);
            }
            applyPayload(transaction, &payload)?;
            insertOperation(transaction, operation, &payload)?;
            observeOperation(transaction, operation)?;
            Ok(true)
        })?;
        if didApply {
            self.store.notifyInvalidated()?;
        }
        Ok(())
    }

    fn appendLocalOperation(
        &self,
        entityType: &str,
        entityId: &str,
        operationName: &str,
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
                payload: payloadValue,
                createdAt,
                schemaVersion: 1,
            };
            insertOperation(transaction, &operation, &payload)?;
            observeOperation(transaction, &operation)?;
            Ok(operation)
        })?;
        Ok(operation)
    }

    fn payloadForChatMetadata(&self, chatId: &str) -> Result<ChatSyncPayload, SqlChatSyncStoreError> {
        let chatDao = ChatDao::new(self.store.clone());
        let chatRows = chatDao.getChatById(chatId)?.into_iter().collect();
        Ok(ChatSyncPayload {
            chatRows,
            ..ChatSyncPayload::default()
        })
    }

    fn payloadForChatSnapshot(&self, chatId: &str) -> Result<ChatSyncPayload, SqlChatSyncStoreError> {
        let chatDao = ChatDao::new(self.store.clone());
        let messageDao = MessageDao::new(self.store.clone());
        let variantDao = MessageVariantDao::new(self.store.clone());
        let chatRows = chatDao.getChatById(chatId)?.into_iter().collect::<Vec<_>>();
        let messageRows = messageDao.getMessagesForChat(chatId)?;
        let variantRows = variantDao.getVariantsForChat(chatId)?;
        Ok(ChatSyncPayload {
            chatRows,
            messageRows,
            variantRows,
            deletions: Vec::new(),
        })
    }

    fn payloadForMessageSnapshot(
        &self,
        chatId: &str,
        timestamp: i64,
    ) -> Result<ChatSyncPayload, SqlChatSyncStoreError> {
        let chatDao = ChatDao::new(self.store.clone());
        let messageDao = MessageDao::new(self.store.clone());
        let variantDao = MessageVariantDao::new(self.store.clone());
        let chatRows = chatDao.getChatById(chatId)?.into_iter().collect::<Vec<_>>();
        let messageRows = messageDao
            .getMessageByTimestamp(chatId, timestamp)?
            .into_iter()
            .collect::<Vec<_>>();
        let variantRows = variantDao.getVariantsForMessage(chatId, timestamp)?;
        Ok(ChatSyncPayload {
            chatRows,
            messageRows,
            variantRows,
            deletions: Vec::new(),
        })
    }
}

fn mapOperationMetadata(row: &SqliteRow) -> Result<SyncOperation, SqliteStoreError> {
    Ok(SyncOperation {
        opId: row.get("opId")?,
        originDeviceId: row.get("originDeviceId")?,
        sequence: row.get("sequence")?,
        domain: row.get("domain")?,
        entityType: row.get("entityType")?,
        entityId: row.get("entityId")?,
        operation: row.get("operation")?,
        payload: serde_json::Value::Null,
        createdAt: row.get("createdAt")?,
        schemaVersion: row.get("schemaVersion")?,
    })
}

fn readPayload(store: &SqliteStore, opId: &str) -> Result<ChatSyncPayload, SqliteStoreError> {
    Ok(ChatSyncPayload {
        chatRows: readChatRows(store, opId)?,
        messageRows: readMessageRows(store, opId)?,
        variantRows: readVariantRows(store, opId)?,
        deletions: readDeletions(store, opId)?,
    })
}

fn readChatRows(store: &SqliteStore, opId: &str) -> Result<Vec<ChatEntity>, SqliteStoreError> {
    store
        .queryRows(
            r#"
            SELECT id, title, createdAt, updatedAt, inputTokens, outputTokens,
                currentWindowSize, "group", displayOrder, workspace, workspaceEnv,
                parentChatId, characterCardName, characterGroupId, locked
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
                workspaceEnv: row.get(10)?,
                parentChatId: row.get(11)?,
                characterCardName: row.get(12)?,
                characterGroupId: row.get(13)?,
                locked: row.get(14)?,
            })
        })
        .collect()
}

fn readMessageRows(store: &SqliteStore, opId: &str) -> Result<Vec<MessageEntity>, SqliteStoreError> {
    store
        .queryRows(
            r#"
            SELECT chatId, sender, content, timestamp, orderIndex, roleName,
                selectedVariantIndex, provider, modelName, inputTokens, outputTokens,
                cachedInputTokens, sentAt, outputDurationMs, waitDurationMs,
                completedAt, displayMode, isFavorite
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
                content: row.get(2)?,
                timestamp: row.get(3)?,
                orderIndex: row.get(4)?,
                roleName: row.get(5)?,
                selectedVariantIndex: row.get(6)?,
                provider: row.get(7)?,
                modelName: row.get(8)?,
                inputTokens: row.get(9)?,
                outputTokens: row.get(10)?,
                cachedInputTokens: row.get(11)?,
                sentAt: row.get(12)?,
                outputDurationMs: row.get(13)?,
                waitDurationMs: row.get(14)?,
                completedAt: row.get(15)?,
                displayMode: row.get(16)?,
                isFavorite: row.get(17)?,
            })
        })
        .collect()
}

fn readVariantRows(store: &SqliteStore, opId: &str) -> Result<Vec<MessageVariantEntity>, SqliteStoreError> {
    store
        .queryRows(
            r#"
            SELECT chatId, messageTimestamp, variantIndex, content, roleName,
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
                content: row.get(3)?,
                roleName: row.get(4)?,
                provider: row.get(5)?,
                modelName: row.get(6)?,
                inputTokens: row.get(7)?,
                outputTokens: row.get(8)?,
                cachedInputTokens: row.get(9)?,
                sentAt: row.get(10)?,
                outputDurationMs: row.get(11)?,
                waitDurationMs: row.get(12)?,
                completedAt: row.get(13)?,
            })
        })
        .collect()
}

fn readDeletions(store: &SqliteStore, opId: &str) -> Result<Vec<ChatSyncDeletion>, SqliteStoreError> {
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
    mergeOlderUpserts(transaction, operation)?;
    transaction.execute(
        r#"
        INSERT INTO sync_sql_operations (
            opId, originDeviceId, sequence, domain, entityType, entityId,
            operation, createdAt, schemaVersion
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
        sqliteParams![
            operation.opId,
            operation.originDeviceId,
            operation.sequence,
            operation.domain,
            operation.entityType,
            operation.entityId,
            operation.operation,
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
            currentWindowSize, "group", displayOrder, workspace, workspaceEnv,
            parentChatId, characterCardName, characterGroupId, locked
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
            chat.workspaceEnv,
            chat.parentChatId,
            chat.characterCardName,
            chat.characterGroupId,
            chat.locked,
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
            opId, chatId, sender, content, timestamp, orderIndex, roleName,
            selectedVariantIndex, provider, modelName, inputTokens, outputTokens,
            cachedInputTokens, sentAt, outputDurationMs, waitDurationMs,
            completedAt, displayMode, isFavorite
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
        "#,
        sqliteParams![
            opId,
            message.chatId,
            message.sender,
            message.content,
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
            message.displayMode,
            message.isFavorite,
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
            opId, chatId, messageTimestamp, variantIndex, content, roleName,
            provider, modelName, inputTokens, outputTokens, cachedInputTokens,
            sentAt, outputDurationMs, waitDurationMs, completedAt
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
        "#,
        sqliteParams![
            opId,
            variant.chatId,
            variant.messageTimestamp,
            variant.variantIndex,
            variant.content,
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

fn applyPayload(transaction: &mut SqliteTransaction<'_>, payload: &ChatSyncPayload) -> Result<(), SqliteStoreError> {
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
    Ok(())
}

fn applyDeletion(
    transaction: &mut SqliteTransaction<'_>,
    deletion: &ChatSyncDeletion,
) -> Result<(), SqliteStoreError> {
    match deletion.tableName.as_str() {
        DELETE_CHAT => {
            transaction.execute("DELETE FROM chats WHERE id = ?1", sqliteParams![deletion.chatId])?;
        }
        DELETE_MESSAGE => {
            let timestamp = requiredTimestamp(deletion)?;
            transaction.execute(
                "DELETE FROM messages WHERE chatId = ?1 AND timestamp = ?2",
                sqliteParams![deletion.chatId, timestamp],
            )?;
        }
        DELETE_MESSAGES_FROM => {
            let timestamp = requiredTimestamp(deletion)?;
            transaction.execute(
                "DELETE FROM messages WHERE chatId = ?1 AND timestamp >= ?2",
                sqliteParams![deletion.chatId, timestamp],
            )?;
        }
        DELETE_MESSAGES_FOR_CHAT => {
            transaction.execute(
                "DELETE FROM messages WHERE chatId = ?1",
                sqliteParams![deletion.chatId],
            )?;
        }
        DELETE_VARIANT => {
            let timestamp = requiredTimestamp(deletion)?;
            let variantIndex = requiredVariantIndex(deletion)?;
            transaction.execute(
                "DELETE FROM message_variants WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
                sqliteParams![deletion.chatId, timestamp, variantIndex],
            )?;
        }
        DELETE_VARIANTS_FROM => {
            let timestamp = requiredTimestamp(deletion)?;
            transaction.execute(
                "DELETE FROM message_variants WHERE chatId = ?1 AND messageTimestamp >= ?2",
                sqliteParams![deletion.chatId, timestamp],
            )?;
        }
        DELETE_VARIANTS_FOR_MESSAGE => {
            let timestamp = requiredTimestamp(deletion)?;
            transaction.execute(
                "DELETE FROM message_variants WHERE chatId = ?1 AND messageTimestamp = ?2",
                sqliteParams![deletion.chatId, timestamp],
            )?;
        }
        DELETE_VARIANTS_FOR_CHAT => {
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

fn upsertChat(transaction: &mut SqliteTransaction<'_>, chat: &ChatEntity) -> Result<(), SqliteStoreError> {
    transaction.execute(
        r#"
        INSERT INTO chats (
            id, title, createdAt, updatedAt, inputTokens, outputTokens,
            currentWindowSize, "group", displayOrder, workspace, workspaceEnv,
            parentChatId, characterCardName, characterGroupId, locked
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
            workspaceEnv = excluded.workspaceEnv,
            parentChatId = excluded.parentChatId,
            characterCardName = excluded.characterCardName,
            characterGroupId = excluded.characterGroupId,
            locked = excluded.locked
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
            chat.workspaceEnv,
            chat.parentChatId,
            chat.characterCardName,
            chat.characterGroupId,
            chat.locked,
        ],
    )?;
    Ok(())
}

fn upsertMessage(transaction: &mut SqliteTransaction<'_>, message: &MessageEntity) -> Result<(), SqliteStoreError> {
    transaction.execute(
        "DELETE FROM messages WHERE chatId = ?1 AND timestamp = ?2",
        sqliteParams![message.chatId, message.timestamp],
    )?;
    transaction.execute(
        r#"
        INSERT INTO messages (
            chatId, sender, content, timestamp, orderIndex, roleName,
            selectedVariantIndex, provider, modelName, inputTokens, outputTokens,
            cachedInputTokens, sentAt, outputDurationMs, waitDurationMs,
            completedAt, displayMode, isFavorite
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
        "#,
        sqliteParams![
            message.chatId,
            message.sender,
            message.content,
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
            chatId, messageTimestamp, variantIndex, content, roleName, provider,
            modelName, inputTokens, outputTokens, cachedInputTokens, sentAt,
            outputDurationMs, waitDurationMs, completedAt
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        "#,
        sqliteParams![
            variant.chatId,
            variant.messageTimestamp,
            variant.variantIndex,
            variant.content,
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

fn operationExists(transaction: &mut SqliteTransaction<'_>, opId: &str) -> Result<bool, SqliteStoreError> {
    Ok(transaction
        .queryOne(
            "SELECT 1 FROM sync_sql_operations WHERE opId = ?1 LIMIT 1",
            sqliteParams![opId],
        )?
        .is_some())
}

fn hasNewerMergedUpsert(
    transaction: &mut SqliteTransaction<'_>,
    operation: &SyncOperation,
) -> Result<bool, SqliteStoreError> {
    if operation.operation != "upsert" {
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
                AND operation = 'upsert'
                AND sequence > ?5
            LIMIT 1
            "#,
            sqliteParams![
                operation.originDeviceId,
                operation.domain,
                operation.entityType,
                operation.entityId,
                operation.sequence,
            ],
        )?
        .is_some())
}

fn mergeOlderUpserts(
    transaction: &mut SqliteTransaction<'_>,
    operation: &SyncOperation,
) -> Result<(), SqliteStoreError> {
    if operation.operation != "upsert" {
        return Ok(());
    }
    transaction.execute(
        r#"
        DELETE FROM sync_sql_operations
        WHERE originDeviceId = ?1
            AND domain = ?2
            AND entityType = ?3
            AND entityId = ?4
            AND operation = 'upsert'
            AND sequence < ?5
        "#,
        sqliteParams![
            operation.originDeviceId,
            operation.domain,
            operation.entityType,
            operation.entityId,
            operation.sequence,
        ],
    )?;
    Ok(())
}

fn sequenceFor(transaction: &mut SqliteTransaction<'_>, originDeviceId: &str) -> Result<i64, SqliteStoreError> {
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
        SqliteStoreError::Message(format!(
            "missing variantIndex for {}",
            deletion.tableName
        ))
    })
}

fn currentTimeMillis() -> Result<i64, SqlChatSyncStoreError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| SqlChatSyncStoreError::Message(error.to_string()))?;
    Ok(duration.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::path::{Component, Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    use operit_host_api::{
        HostError, HostResult, RuntimeSqliteConnection, RuntimeSqliteHost, RuntimeSqliteTransaction,
        RuntimeStorageEntry, RuntimeStorageHost, SqliteRow as HostSqliteRow, SqliteValue,
    };
    use operit_store::sqliteParams;
    use operit_store::RuntimeStorageHost::{
        setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost,
    };
    use rusqlite::types::Value as RusqliteValue;

    static HOSTS: OnceLock<()> = OnceLock::new();
    static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
    static DATABASE_MUTEX: Mutex<()> = Mutex::new(());

    #[derive(Clone, Debug)]
    struct TestRuntimeHost {
        root: PathBuf,
    }

    impl TestRuntimeHost {
        fn new(root: PathBuf) -> Self {
            Self { root }
        }

        fn resolve(&self, path: &str) -> HostResult<PathBuf> {
            let path = Path::new(path);
            if path.is_absolute() {
                return Err(HostError::new(format!(
                    "Runtime storage path must be relative: {}",
                    path.display()
                )));
            }
            let mut resolved = self.root.clone();
            for component in path.components() {
                match component {
                    Component::Normal(segment) => resolved.push(segment),
                    Component::CurDir => {}
                    _ => {
                        return Err(HostError::new(format!(
                            "Invalid runtime storage path: {}",
                            path.display()
                        )))
                    }
                }
            }
            Ok(resolved)
        }
    }

    impl RuntimeStorageHost for TestRuntimeHost {
        fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
            Ok(fs::read(self.resolve(path)?)?)
        }

        fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
            let path = self.resolve(path)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, content)?;
            Ok(())
        }

        fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
            let path = self.resolve(path)?;
            if !path.exists() {
                return Ok(());
            }
            if path.is_dir() {
                if recursive {
                    fs::remove_dir_all(path)?;
                } else {
                    fs::remove_dir(path)?;
                }
            } else {
                fs::remove_file(path)?;
            }
            Ok(())
        }

        fn exists(&self, path: &str) -> HostResult<bool> {
            Ok(self.resolve(path)?.exists())
        }

        fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
            let directory = self.resolve(prefix)?;
            let mut entries = Vec::new();
            if !directory.exists() {
                return Ok(entries);
            }
            for entry in fs::read_dir(directory)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                let path = entry
                    .path()
                    .strip_prefix(&self.root)
                    .map_err(|error| HostError::new(error.to_string()))?
                    .to_string_lossy()
                    .replace('\\', "/");
                entries.push(RuntimeStorageEntry {
                    path,
                    isDirectory: metadata.is_dir(),
                    size: metadata.len() as i64,
                });
            }
            Ok(entries)
        }
    }

    impl RuntimeSqliteHost for TestRuntimeHost {
        fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
            let path = self.resolve(path)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let connection = rusqlite::Connection::open(path)
                .map_err(|error| HostError::new(error.to_string()))?;
            connection
                .execute_batch(
                    r#"
                    PRAGMA journal_mode = MEMORY;
                    PRAGMA synchronous = OFF;
                    PRAGMA temp_store = MEMORY;
                    "#,
                )
                .map_err(|error| HostError::new(error.to_string()))?;
            Ok(Box::new(TestRuntimeSqliteConnection { connection }))
        }
    }

    struct TestRuntimeSqliteConnection {
        connection: rusqlite::Connection,
    }

    impl RuntimeSqliteConnection for TestRuntimeSqliteConnection {
        fn executeBatch(&mut self, sql: &str) -> HostResult<()> {
            self.connection
                .execute_batch(sql)
                .map_err(|error| HostError::new(error.to_string()))
        }

        fn execute(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<usize> {
            let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
            self.connection
                .execute(sql, rusqlite::params_from_iter(params))
                .map_err(|error| HostError::new(error.to_string()))
        }

        fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<HostSqliteRow>> {
            queryRows(&self.connection, sql, params)
        }

        fn lastInsertRowId(&self) -> HostResult<i64> {
            Ok(self.connection.last_insert_rowid())
        }

        fn beginTransaction(&mut self) -> HostResult<Box<dyn RuntimeSqliteTransaction + '_>> {
            let transaction = self
                .connection
                .transaction()
                .map_err(|error| HostError::new(error.to_string()))?;
            Ok(Box::new(TestRuntimeSqliteTransaction { transaction }))
        }
    }

    struct TestRuntimeSqliteTransaction<'a> {
        transaction: rusqlite::Transaction<'a>,
    }

    impl RuntimeSqliteTransaction for TestRuntimeSqliteTransaction<'_> {
        fn execute(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<usize> {
            let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
            self.transaction
                .execute(sql, rusqlite::params_from_iter(params))
                .map_err(|error| HostError::new(error.to_string()))
        }

        fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<HostSqliteRow>> {
            queryRows(&self.transaction, sql, params)
        }

        fn lastInsertRowId(&self) -> HostResult<i64> {
            Ok(self.transaction.last_insert_rowid())
        }

        fn commit(self: Box<Self>) -> HostResult<()> {
            self.transaction
                .commit()
                .map_err(|error| HostError::new(error.to_string()))
        }
    }

    trait TestRusqliteConnection {
        fn prepareStatement<'a>(&'a self, sql: &str) -> rusqlite::Result<rusqlite::Statement<'a>>;
    }

    impl TestRusqliteConnection for rusqlite::Connection {
        fn prepareStatement<'a>(&'a self, sql: &str) -> rusqlite::Result<rusqlite::Statement<'a>> {
            self.prepare(sql)
        }
    }

    impl TestRusqliteConnection for rusqlite::Transaction<'_> {
        fn prepareStatement<'a>(&'a self, sql: &str) -> rusqlite::Result<rusqlite::Statement<'a>> {
            self.prepare(sql)
        }
    }

    fn queryRows(
        connection: &impl TestRusqliteConnection,
        sql: &str,
        params: Vec<SqliteValue>,
    ) -> HostResult<Vec<HostSqliteRow>> {
        let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
        let mut statement = connection
            .prepareStatement(sql)
            .map_err(|error| HostError::new(error.to_string()))?;
        let columns = statement
            .column_names()
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let mut rows = statement
            .query(rusqlite::params_from_iter(params))
            .map_err(|error| HostError::new(error.to_string()))?;
        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|error| HostError::new(error.to_string()))?
        {
            let mut values = Vec::new();
            for index in 0..columns.len() {
                let value = row
                    .get::<_, RusqliteValue>(index)
                    .map_err(|error| HostError::new(error.to_string()))?;
                values.push(fromRusqliteValue(value));
            }
            out.push(HostSqliteRow {
                columns: columns.clone(),
                values,
            });
        }
        Ok(out)
    }

    fn toRusqliteValue(value: SqliteValue) -> RusqliteValue {
        match value {
            SqliteValue::Null => RusqliteValue::Null,
            SqliteValue::Integer(value) => RusqliteValue::Integer(value),
            SqliteValue::Real(value) => RusqliteValue::Real(value),
            SqliteValue::Text(value) => RusqliteValue::Text(value),
            SqliteValue::Blob(value) => RusqliteValue::Blob(value),
        }
    }

    fn fromRusqliteValue(value: RusqliteValue) -> SqliteValue {
        match value {
            RusqliteValue::Null => SqliteValue::Null,
            RusqliteValue::Integer(value) => SqliteValue::Integer(value),
            RusqliteValue::Real(value) => SqliteValue::Real(value),
            RusqliteValue::Text(value) => SqliteValue::Text(value),
            RusqliteValue::Blob(value) => SqliteValue::Blob(value),
        }
    }

    fn installTestHosts() {
        HOSTS.get_or_init(|| {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("test clock must be after UNIX_EPOCH")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "operit2-sql-sync-tests-{}-{nanos}",
                std::process::id()
            ));
            fs::create_dir_all(&root).expect("test runtime host root must be created");
            let host = Arc::new(TestRuntimeHost::new(root));
            setDefaultRuntimeStorageHost(host.clone());
            setDefaultRuntimeSqliteHost(host);
        });
    }

    fn testPaths(name: &str) -> RuntimeStorePaths {
        installTestHosts();
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        RuntimeStorePaths::new(
            RuntimeStorePaths::default()
                .root_dir()
                .join(format!("sync-tests/{name}-{id}")),
        )
    }

    fn openTestStore(name: &str) -> (RuntimeStorePaths, Arc<AppDatabase>, SqlChatSyncStore) {
        AppDatabase::closeDatabase();
        let paths = testPaths(name);
        let database = AppDatabase::getDatabase(paths.clone()).unwrap();
        let syncStore = SqlChatSyncStore::new(paths.clone(), &database).unwrap();
        (paths, database, syncStore)
    }

    fn chat(chatId: &str) -> ChatEntity {
        ChatEntity::new(chatId.to_string(), "New Chat".to_string(), 1_000)
    }

    fn message(chatId: &str, timestamp: i64, content: &str) -> MessageEntity {
        MessageEntity {
            messageId: 0,
            chatId: chatId.to_string(),
            sender: "ai".to_string(),
            content: content.to_string(),
            timestamp,
            orderIndex: 0,
            roleName: String::new(),
            selectedVariantIndex: 0,
            provider: "test-provider".to_string(),
            modelName: "test-model".to_string(),
            inputTokens: 0,
            outputTokens: 0,
            cachedInputTokens: 0,
            sentAt: 0,
            outputDurationMs: 0,
            waitDurationMs: 0,
            completedAt: 0,
            displayMode: "NORMAL".to_string(),
            isFavorite: false,
        }
    }

    fn insertChatMessage(database: &AppDatabase, chatId: &str, timestamp: i64, content: &str) -> i64 {
        database.chatDao().insertChat(chat(chatId)).unwrap();
        database
            .messageDao()
            .insertMessage(message(chatId, timestamp, content))
            .unwrap()
    }

    fn exportedPayload(operation: &SyncOperation) -> ChatSyncPayload {
        serde_json::from_value(operation.payload.clone()).unwrap()
    }

    fn sqlOperationCount(database: &AppDatabase) -> i64 {
        database
            .store()
            .queryScalar("SELECT COUNT(*) FROM sync_sql_operations", sqliteParams![])
            .unwrap()
    }

    fn sqlMessageRowCount(database: &AppDatabase) -> i64 {
        database
            .store()
            .queryScalar("SELECT COUNT(*) FROM sync_sql_message_rows", sqliteParams![])
            .unwrap()
    }

    fn upsertOperation(sequence: i64, content: &str) -> SyncOperation {
        let payload = ChatSyncPayload {
            chatRows: vec![chat("chat-remote")],
            messageRows: vec![message("chat-remote", 2_000, content)],
            variantRows: Vec::new(),
            deletions: Vec::new(),
        };
        SyncOperation {
            opId: format!("remote-sql:{sequence}"),
            originDeviceId: "remote-sql".to_string(),
            sequence,
            domain: CHAT_SYNC_DOMAIN.to_string(),
            entityType: "message".to_string(),
            entityId: "chat-remote:2000".to_string(),
            operation: "upsert".to_string(),
            payload: serde_json::to_value(payload).unwrap(),
            createdAt: sequence,
            schemaVersion: 1,
        }
    }

    #[test]
    fn record_message_snapshots_are_merged_into_final_stream_state() {
        let _guard = DATABASE_MUTEX.lock().unwrap();
        let (_paths, database, syncStore) = openTestStore("stream-merge");
        let chatId = "chat-stream";
        let timestamp = 10_000;
        let messageId = insertChatMessage(&database, chatId, timestamp, "");

        for index in 1..=100 {
            database
                .messageDao()
                .updateMessageContent(messageId, format!("token-{index}"))
                .unwrap();
            syncStore.recordMessageSnapshot(chatId, timestamp).unwrap();
        }

        assert_eq!(sqlOperationCount(&database), 1);
        let operations = syncStore
            .operationsSince(&SyncClock::empty(), &[CHAT_SYNC_DOMAIN.to_string()], 10)
            .unwrap();
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].sequence, 100);
        let payload = exportedPayload(&operations[0]);
        assert_eq!(payload.messageRows.len(), 1);
        assert_eq!(payload.messageRows[0].content, "token-100");
        AppDatabase::closeDatabase();
    }

    #[test]
    fn compacted_stream_snapshot_applies_to_new_receiver() {
        let _guard = DATABASE_MUTEX.lock().unwrap();
        let (_sourcePaths, sourceDatabase, sourceSyncStore) = openTestStore("source-stream");
        let chatId = "chat-apply";
        let timestamp = 11_000;
        let messageId = insertChatMessage(&sourceDatabase, chatId, timestamp, "");

        for index in 1..=50 {
            sourceDatabase
                .messageDao()
                .updateMessageContent(messageId, format!("chunk-{index}"))
                .unwrap();
            sourceSyncStore.recordMessageSnapshot(chatId, timestamp).unwrap();
        }
        let operations = sourceSyncStore
            .operationsSince(&SyncClock::empty(), &[CHAT_SYNC_DOMAIN.to_string()], 10)
            .unwrap();
        AppDatabase::closeDatabase();

        let (_targetPaths, targetDatabase, targetSyncStore) = openTestStore("target-stream");
        for operation in &operations {
            targetSyncStore.applyOperation(operation).unwrap();
        }

        let message = targetDatabase
            .messageDao()
            .getMessageByTimestamp(chatId, timestamp)
            .unwrap()
            .unwrap();
        assert_eq!(message.content, "chunk-50");
        assert_eq!(targetSyncStore.localClock().unwrap().sequenceFor(&operations[0].originDeviceId), 50);
        AppDatabase::closeDatabase();
    }

    #[test]
    fn older_merged_upsert_does_not_revert_newer_state() {
        let _guard = DATABASE_MUTEX.lock().unwrap();
        let (_paths, database, syncStore) = openTestStore("older-upsert");
        let newer = upsertOperation(2, "new");
        let older = upsertOperation(1, "old");

        syncStore.applyOperation(&newer).unwrap();
        syncStore.applyOperation(&older).unwrap();

        let message = database
            .messageDao()
            .getMessageByTimestamp("chat-remote", 2_000)
            .unwrap()
            .unwrap();
        assert_eq!(message.content, "new");
        assert_eq!(sqlOperationCount(&database), 1);
        AppDatabase::closeDatabase();
    }

    #[test]
    fn delete_transaction_survives_compaction_and_applies() {
        let _guard = DATABASE_MUTEX.lock().unwrap();
        let (_sourcePaths, sourceDatabase, sourceSyncStore) = openTestStore("source-delete");
        let chatId = "chat-delete";
        let timestamp = 12_000;
        insertChatMessage(&sourceDatabase, chatId, timestamp, "remove-me");
        sourceSyncStore.recordMessageSnapshot(chatId, timestamp).unwrap();
        sourceDatabase
            .messageDao()
            .deleteMessageByTimestamp(chatId, timestamp)
            .unwrap();
        sourceSyncStore.recordMessageDeletion(chatId, timestamp).unwrap();
        let operations = sourceSyncStore
            .operationsSince(&SyncClock::empty(), &[CHAT_SYNC_DOMAIN.to_string()], 10)
            .unwrap();
        assert_eq!(operations.iter().map(|operation| operation.operation.as_str()).collect::<Vec<_>>(), vec!["upsert", "delete"]);
        AppDatabase::closeDatabase();

        let (_targetPaths, targetDatabase, targetSyncStore) = openTestStore("target-delete");
        for operation in &operations {
            targetSyncStore.applyOperation(operation).unwrap();
        }

        assert!(targetDatabase
            .chatDao()
            .getChatById(chatId)
            .unwrap()
            .is_some());
        assert!(targetDatabase
            .messageDao()
            .getMessageByTimestamp(chatId, timestamp)
            .unwrap()
            .is_none());
        AppDatabase::closeDatabase();
    }

    #[test]
    fn stress_stream_snapshots_export_single_final_operation() {
        let _guard = DATABASE_MUTEX.lock().unwrap();
        let (_paths, database, syncStore) = openTestStore("stress-stream");
        let chatId = "chat-stress";
        let timestamp = 13_000;
        let messageId = insertChatMessage(&database, chatId, timestamp, "");

        for index in 1..=1_000 {
            database
                .messageDao()
                .updateMessageContent(messageId, format!("stress-token-{index}"))
                .unwrap();
            syncStore.recordMessageSnapshot(chatId, timestamp).unwrap();
        }

        assert_eq!(sqlOperationCount(&database), 1);
        let operations = syncStore
            .operationsSince(&SyncClock::empty(), &[CHAT_SYNC_DOMAIN.to_string()], 10)
            .unwrap();
        assert_eq!(operations.len(), 1);
        let payload = exportedPayload(&operations[0]);
        assert_eq!(payload.messageRows[0].content, "stress-token-1000");
        AppDatabase::closeDatabase();
    }

    #[test]
    fn stress_many_messages_roundtrip_with_stream_compaction() {
        let _guard = DATABASE_MUTEX.lock().unwrap();
        let (_sourcePaths, sourceDatabase, sourceSyncStore) = openTestStore("stress-roundtrip-source");
        let chatId = "chat-stress-roundtrip";
        let messageCount = 60;
        let updateRounds = 30;
        let mut messageIds = Vec::new();

        sourceDatabase.chatDao().insertChat(chat(chatId)).unwrap();
        for messageIndex in 0..messageCount {
            let timestamp = 20_000 + messageIndex as i64;
            let messageId = sourceDatabase
                .messageDao()
                .insertMessage(message(chatId, timestamp, ""))
                .unwrap();
            messageIds.push((timestamp, messageId));
        }

        for round in 1..=updateRounds {
            if round % 50 == 0 {
                eprintln!("sql sync ultra stress: recording round {round}/{updateRounds}");
            }
            for (messageIndex, (timestamp, messageId)) in messageIds.iter().enumerate() {
                sourceDatabase
                    .messageDao()
                    .updateMessageContent(
                        *messageId,
                        format!("message-{messageIndex}-round-{round}"),
                    )
                    .unwrap();
                sourceSyncStore.recordMessageSnapshot(chatId, *timestamp).unwrap();
            }
        }

        assert_eq!(sqlOperationCount(&sourceDatabase), messageCount as i64);
        let operations = sourceSyncStore
            .operationsSince(&SyncClock::empty(), &[CHAT_SYNC_DOMAIN.to_string()], messageCount + 10)
            .unwrap();
        assert_eq!(operations.len(), messageCount);
        assert!(operations.iter().all(|operation| operation.operation == "upsert"));
        AppDatabase::closeDatabase();

        let (_targetPaths, targetDatabase, targetSyncStore) = openTestStore("stress-roundtrip-target");
        for operation in &operations {
            targetSyncStore.applyOperation(operation).unwrap();
        }

        for messageIndex in 0..messageCount {
            let timestamp = 20_000 + messageIndex as i64;
            let message = targetDatabase
                .messageDao()
                .getMessageByTimestamp(chatId, timestamp)
                .unwrap()
                .unwrap();
            assert_eq!(
                message.content,
                format!("message-{messageIndex}-round-{updateRounds}")
            );
        }
        assert_eq!(
            targetSyncStore
                .localClock()
                .unwrap()
                .sequenceFor(&operations[0].originDeviceId),
            (messageCount * updateRounds) as i64
        );
        AppDatabase::closeDatabase();
    }

    #[test]
    #[ignore]
    fn stress_ultra_many_messages_roundtrip_with_stream_compaction() {
        let _guard = DATABASE_MUTEX.lock().unwrap();
        let (_sourcePaths, sourceDatabase, sourceSyncStore) = openTestStore("stress-ultra-source");
        let chatId = "chat-stress-ultra";
        let messageCount = 600;
        let updateRounds = 300;
        let mut messageIds = Vec::new();

        sourceDatabase.chatDao().insertChat(chat(chatId)).unwrap();
        for messageIndex in 0..messageCount {
            let timestamp = 30_000 + messageIndex as i64;
            let messageId = sourceDatabase
                .messageDao()
                .insertMessage(message(chatId, timestamp, ""))
                .unwrap();
            messageIds.push((timestamp, messageId));
        }

        for round in 1..=updateRounds {
            for (messageIndex, (timestamp, messageId)) in messageIds.iter().enumerate() {
                sourceDatabase
                    .messageDao()
                    .updateMessageContent(
                        *messageId,
                        format!("message-{messageIndex}-round-{round}"),
                    )
                    .unwrap();
                sourceSyncStore.recordMessageSnapshot(chatId, *timestamp).unwrap();
            }
        }

        let rawSnapshotCount = messageCount * updateRounds;
        let operationRows = sqlOperationCount(&sourceDatabase);
        let messageRows = sqlMessageRowCount(&sourceDatabase);
        let operations = sourceSyncStore
            .operationsSince(&SyncClock::empty(), &[CHAT_SYNC_DOMAIN.to_string()], messageCount + 10)
            .unwrap();
        let exportedPayloadBytes = operations
            .iter()
            .map(|operation| serde_json::to_vec(&operation.payload).unwrap().len())
            .sum::<usize>();
        eprintln!(
            "sql sync ultra stress: raw_snapshots={rawSnapshotCount}, sync_sql_operations={operationRows}, sync_sql_message_rows={messageRows}, exported_operations={}, exported_payload_bytes={exportedPayloadBytes}",
            operations.len()
        );

        assert_eq!(operationRows, messageCount as i64);
        assert_eq!(messageRows, messageCount as i64);
        assert_eq!(operations.len(), messageCount);
        assert!(operations.iter().all(|operation| operation.operation == "upsert"));
        AppDatabase::closeDatabase();

        let (_targetPaths, targetDatabase, targetSyncStore) = openTestStore("stress-ultra-target");
        for operation in &operations {
            targetSyncStore.applyOperation(operation).unwrap();
        }

        for messageIndex in 0..messageCount {
            let timestamp = 30_000 + messageIndex as i64;
            let message = targetDatabase
                .messageDao()
                .getMessageByTimestamp(chatId, timestamp)
                .unwrap()
                .unwrap();
            assert_eq!(
                message.content,
                format!("message-{messageIndex}-round-{updateRounds}")
            );
        }
        assert_eq!(
            targetSyncStore
                .localClock()
                .unwrap()
                .sequenceFor(&operations[0].originDeviceId),
            (messageCount * updateRounds) as i64
        );
        AppDatabase::closeDatabase();
    }
}
