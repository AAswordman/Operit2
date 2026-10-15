use super::*;
use crate::dao::ChatDao::{chatInsertParams, chatInsertSql, mapChatEntity};
use crate::dao::MessageDao::{insertMessageParams, insertMessageSql, mapMessageEntity};
use crate::dao::MessagePartDao::{mapMessagePartEntity, partParams};
use crate::dao::MessageVariantDao::mapMessageVariantEntity;
use operit_model::ChatHistory::ChatHistory;
use operit_model::PluginExtensionTarget::PluginExtensionTarget;
use serde_json::Value;

/// Returns exact row statements for one host-validated extension target.
pub(crate) fn extensionStatements(
    target: &PluginExtensionTarget,
) -> Result<
    (
        &'static str,
        &'static str,
        Vec<operit_host_api::SqliteValue>,
    ),
    SqliteStoreError,
> {
    target.validate().map_err(SqliteStoreError::Message)?;
    Ok(match target {
        PluginExtensionTarget::Chat { chatId } => (
            "SELECT pluginExtensions FROM chats WHERE id = ?1",
            "UPDATE chats SET pluginExtensions = ?2 WHERE id = ?1",
            sqliteParams![chatId],
        ),
        PluginExtensionTarget::Message { chatId, messageTimestamp, variantIndex: 0 } => (
            "SELECT pluginExtensions FROM messages WHERE chatId = ?1 AND timestamp = ?2",
            "UPDATE messages SET pluginExtensions = ?3 WHERE chatId = ?1 AND timestamp = ?2",
            sqliteParams![chatId, messageTimestamp],
        ),
        PluginExtensionTarget::Message { chatId, messageTimestamp, variantIndex } => (
            "SELECT pluginExtensions FROM message_variants WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
            "UPDATE message_variants SET pluginExtensions = ?4 WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
            sqliteParams![chatId, messageTimestamp, variantIndex],
        ),
    })
}

/// Reads complete records from the same transaction that performed their mutation.
fn payloadInTransaction(
    transaction: &mut SqliteTransaction<'_>,
    chatId: &str,
    timestamp: Option<i64>,
) -> Result<ChatSyncPayload, SqliteStoreError> {
    let chat = transaction
        .queryOne("SELECT * FROM chats WHERE id = ?1", sqliteParams![chatId])?
        .ok_or_else(|| SqliteStoreError::Message(format!("Chat does not exist: {chatId}")))?;
    Ok(ChatSyncPayload {
        chatRows: vec![mapChatEntity(&chat)?],
        messageRows: transaction.queryRows("SELECT * FROM messages WHERE chatId = ?1 AND (?2 IS NULL OR timestamp = ?2) ORDER BY orderIndex, timestamp", sqliteParams![chatId, timestamp])?.iter().map(mapMessageEntity).collect::<Result<_, _>>()?,
        variantRows: transaction.queryRows("SELECT * FROM message_variants WHERE chatId = ?1 AND (?2 IS NULL OR messageTimestamp = ?2) ORDER BY messageTimestamp, variantIndex", sqliteParams![chatId, timestamp])?.iter().map(mapMessageVariantEntity).collect::<Result<_, _>>()?,
        partRows: transaction.queryRows("SELECT * FROM message_parts WHERE chatId = ?1 AND (?2 IS NULL OR messageTimestamp = ?2) ORDER BY messageTimestamp, variantIndex, sequence", sqliteParams![chatId, timestamp])?.iter().map(mapMessagePartEntity).collect::<Result<_, _>>()?,
        deletions: Vec::new(),
    })
}

/// Records a sync operation atomically with its authoritative record mutation.
fn recordInTransaction(
    transaction: &mut SqliteTransaction<'_>,
    origin: &str,
    payload: &ChatSyncPayload,
    createdAt: i64,
) -> Result<(), SqliteStoreError> {
    let sequence = sequenceFor(transaction, origin)? + 1;
    let chatId = &payload.chatRows[0].id;
    let operation = SyncOperation {
        opId: format!("{origin}:{sequence}"),
        originDeviceId: origin.to_string(),
        sequence,
        domain: CHAT_SYNC_DOMAIN.to_string(),
        entityType: "chat".to_string(),
        entityId: chatId.clone(),
        operation: "upsert".to_string(),
        semantics: SyncOperationSemantics::Transaction,
        payload: serde_json::to_value(payload)
            .map_err(|error| SqliteStoreError::Message(error.to_string()))?,
        createdAt,
        schemaVersion: CHAT_SYNC_OPERATION_SCHEMA_VERSION,
    };
    insertOperation(transaction, &operation, payload)?;
    observeOperation(transaction, &operation)
}

impl SqlChatSyncStore {
    /// Reads the complete map from one exact persisted row without accepting missing records.
    pub fn readPluginExtensions(
        &self,
        target: &PluginExtensionTarget,
    ) -> Result<BTreeMap<String, Value>, SqlChatSyncStoreError> {
        let (select, _, params) = extensionStatements(target)?;
        let row = self.store.queryOne(select, params)?.ok_or_else(|| {
            SqliteStoreError::Message(format!(
                "Plugin extension record does not exist: {target:?}"
            ))
        })?;
        Ok(decodePluginExtensions(&row.get::<_, String>(0)?)?)
    }

    /// Mutates exactly one authenticated namespace and records its full persisted state in the same transaction.
    pub fn mutatePluginExtension(
        &self,
        owner: &str,
        target: &PluginExtensionTarget,
        value: Option<Value>,
    ) -> Result<bool, SqlChatSyncStoreError> {
        if owner.trim().is_empty() {
            return Err(SqlChatSyncStoreError::Message(
                "Plugin extension owner must be nonempty".to_string(),
            ));
        }
        if let Some(value) = &value {
            if !value.is_object() {
                return Err(SqlChatSyncStoreError::Message(
                    "Plugin extension value must be a JSON object".to_string(),
                ));
            }
        }
        let (select, update, params) = extensionStatements(target)?;
        let createdAt = currentTimeMillis()?;
        let changed = self.store.withPluginExtensionMutation(target, || {
            self.store.transaction(|transaction| {
                let row = transaction
                    .queryOne(select, params.clone())?
                    .ok_or_else(|| {
                        SqliteStoreError::Message(format!(
                            "Plugin extension record does not exist: {target:?}"
                        ))
                    })?;
                let mut extensions = decodePluginExtensions(&row.get::<_, String>(0)?)?;
                match &value {
                    Some(value) => {
                        extensions.insert(owner.to_string(), value.clone());
                    }
                    None => {
                        if extensions.remove(owner).is_none() {
                            return Ok(false);
                        }
                    }
                }
                let mut updateParams = params;
                updateParams.push(operit_host_api::SqliteValue::Text(encodePluginExtensions(
                    &extensions,
                )?));
                if transaction.execute(update, updateParams)? != 1 {
                    return Err(SqliteStoreError::Message(
                        "Plugin extension update did not affect exactly one record".to_string(),
                    ));
                }
                let timestamp = match target {
                    PluginExtensionTarget::Chat { .. } => None,
                    PluginExtensionTarget::Message {
                        messageTimestamp, ..
                    } => Some(*messageTimestamp),
                };
                let payload = payloadInTransaction(transaction, target.chatId(), timestamp)?;
                recordInTransaction(transaction, &self.originDeviceId, &payload, createdAt)?;
                Ok(true)
            })
        })?;
        if changed {
            self.store.notifyInvalidated()?;
            publishSyncMutation();
        }
        Ok(changed)
    }

    /// Commits a validated chat draft, its initial messages or exact branch clone, and sync state atomically.
    pub fn commitChatDraft(
        &self,
        draft: &ChatHistory,
        source: Option<(&str, Option<i64>)>,
    ) -> Result<(), SqlChatSyncStoreError> {
        let chat = ChatEntity::fromChatHistory(draft);
        let params = chatInsertParams(&chat)?;
        let createdAt = currentTimeMillis()?;
        self.store.transaction(|transaction| {
            transaction.execute(chatInsertSql(), params)?;
            if let Some((sourceChatId, throughTimestamp)) = source {
                if transaction.queryOne("SELECT id FROM chats WHERE id = ?1", sqliteParams![sourceChatId])?.is_none() {
                    return Err(SqliteStoreError::Message(format!("Branch source chat does not exist: {sourceChatId}")));
                }
                if let Some(timestamp) = throughTimestamp {
                    if transaction.queryOne("SELECT timestamp FROM messages WHERE chatId = ?1 AND timestamp = ?2", sqliteParams![sourceChatId, timestamp])?.is_none() {
                        return Err(SqliteStoreError::Message(format!("Branch source message does not exist: {sourceChatId}:{timestamp}")));
                    }
                }
                transaction.execute("INSERT INTO messages (chatId, sender, timestamp, orderIndex, roleName, selectedVariantIndex, provider, modelName, inputTokens, outputTokens, cachedInputTokens, sentAt, outputDurationMs, waitDurationMs, completedAt, completedExecutionGeneration, displayMode, isFavorite, pluginExtensions) SELECT ?2, sender, timestamp, orderIndex, roleName, selectedVariantIndex, provider, modelName, inputTokens, outputTokens, cachedInputTokens, sentAt, outputDurationMs, waitDurationMs, completedAt, completedExecutionGeneration, displayMode, isFavorite, pluginExtensions FROM messages WHERE chatId = ?1 AND (?3 IS NULL OR timestamp <= ?3)", sqliteParams![sourceChatId, draft.id, throughTimestamp])?;
                transaction.execute("INSERT INTO message_variants (chatId, messageTimestamp, variantIndex, roleName, provider, modelName, inputTokens, outputTokens, cachedInputTokens, sentAt, outputDurationMs, waitDurationMs, completedAt, pluginExtensions) SELECT ?2, messageTimestamp, variantIndex, roleName, provider, modelName, inputTokens, outputTokens, cachedInputTokens, sentAt, outputDurationMs, waitDurationMs, completedAt, pluginExtensions FROM message_variants WHERE chatId = ?1 AND (?3 IS NULL OR messageTimestamp <= ?3)", sqliteParams![sourceChatId, draft.id, throughTimestamp])?;
                transaction.execute("INSERT INTO message_parts (chatId, messageTimestamp, variantIndex, partId, sequence, kind, content, toolCallId, toolName, attributesJson) SELECT ?2, messageTimestamp, variantIndex, partId, sequence, kind, content, toolCallId, toolName, attributesJson FROM message_parts WHERE chatId = ?1 AND (?3 IS NULL OR messageTimestamp <= ?3)", sqliteParams![sourceChatId, draft.id, throughTimestamp])?;
            }
            for (index, message) in draft.messages.iter().enumerate() {
                if message.selectedVariantIndex != 0 {
                    return Err(SqliteStoreError::Message("Initial draft messages must be explicit base revisions".to_string()));
                }
                let entity = MessageEntity::fromChatMessage(draft.id.clone(), message.clone(), index as i32, 0);
                transaction.execute(insertMessageSql(false), insertMessageParams(&entity, false)?)?;
                for part in &message.parts {
                    let part = MessagePartEntity::fromMessagePart(draft.id.clone(), message.timestamp, 0, part.clone());
                    transaction.execute("INSERT INTO message_parts (chatId, messageTimestamp, variantIndex, partId, sequence, kind, content, toolCallId, toolName, attributesJson) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)", partParams(&part)?)?;
                }
            }
            let payload = payloadInTransaction(transaction, &draft.id, None)?;
            recordInTransaction(transaction, &self.originDeviceId, &payload, createdAt)
        })?;
        self.store.notifyInvalidated()?;
        publishSyncMutation();
        Ok(())
    }
    /// Imports complete chat and independent revision records in one transaction without a metadata side table.
    pub fn commitImportedChatRecords(
        &self,
        chat: ChatEntity,
        messages: Vec<MessageEntity>,
        variants: Vec<MessageVariantEntity>,
        parts: Vec<MessagePartEntity>,
    ) -> Result<(), SqlChatSyncStoreError> {
        crate::PluginExtensions::validatePluginExtensions(&chat.pluginExtensions)?;
        for message in &messages {
            if message.chatId != chat.id {
                return Err(SqlChatSyncStoreError::Message(
                    "Imported message belongs to a different chat".to_string(),
                ));
            }
            crate::PluginExtensions::validatePluginExtensions(&message.pluginExtensions)?;
        }
        for variant in &variants {
            if variant.chatId != chat.id
                || variant.variantIndex <= 0
                || !messages
                    .iter()
                    .any(|message| message.timestamp == variant.messageTimestamp)
            {
                return Err(SqlChatSyncStoreError::Message(
                    "Imported variant has an invalid owning message or index".to_string(),
                ));
            }
            crate::PluginExtensions::validatePluginExtensions(&variant.pluginExtensions)?;
        }
        for message in &messages {
            if message.selectedVariantIndex < 0
                || (message.selectedVariantIndex > 0
                    && !variants.iter().any(|variant| {
                        variant.messageTimestamp == message.timestamp
                            && variant.variantIndex == message.selectedVariantIndex
                    }))
            {
                return Err(SqlChatSyncStoreError::Message(
                    "Imported selected variant does not exist".to_string(),
                ));
            }
        }
        let createdAt = currentTimeMillis()?;
        self.store.transaction(|transaction| {
            upsertChat(transaction, &chat)?;
            transaction.execute(
                "DELETE FROM message_parts WHERE chatId = ?1",
                sqliteParams![chat.id],
            )?;
            transaction.execute(
                "DELETE FROM message_variants WHERE chatId = ?1",
                sqliteParams![chat.id],
            )?;
            transaction.execute(
                "DELETE FROM messages WHERE chatId = ?1",
                sqliteParams![chat.id],
            )?;
            for message in &messages {
                upsertMessage(transaction, message)?;
            }
            for variant in &variants {
                upsertVariant(transaction, variant)?;
            }
            replacePartRows(transaction, &parts)?;
            let mut payload = payloadInTransaction(transaction, &chat.id, None)?;
            payload.deletions.push(ChatSyncDeletion {
                tableName: DELETE_MESSAGES_FOR_CHAT.to_string(),
                chatId: chat.id.clone(),
                messageTimestamp: None,
                variantIndex: None,
            });
            payload.deletions.push(ChatSyncDeletion {
                tableName: DELETE_VARIANTS_FOR_CHAT.to_string(),
                chatId: chat.id.clone(),
                messageTimestamp: None,
                variantIndex: None,
            });
            recordInTransaction(transaction, &self.originDeviceId, &payload, createdAt)
        })?;
        self.store.notifyInvalidated()?;
        publishSyncMutation();
        Ok(())
    }
}
