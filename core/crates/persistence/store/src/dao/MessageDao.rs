use crate::sqliteParams;
use crate::PluginExtensions::{decodePluginExtensions, encodePluginExtensions};
use crate::SqliteStore::{SqliteRow, SqliteRowGet, SqliteStore, SqliteStoreError, SqliteValue};

use operit_model::ChatMessageLocatorPreview::ChatMessageLocatorPreview;
use operit_model::MessageEntity::{ChatMessageCount, MessageEntity};

#[derive(Clone)]
pub struct MessageDao {
    store: SqliteStore,
}

impl MessageDao {
    /// Processes the exact persisted message record with explicit storage errors.
    pub fn new(store: SqliteStore) -> Self {
        Self { store }
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getTotalMessageCount(&self) -> Result<i32, SqliteStoreError> {
        self.store
            .queryScalar("SELECT COUNT(*) FROM messages", sqliteParams![])
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getMessagesForChat(&self, chatId: &str) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.selectMessages(
            "SELECT * FROM messages WHERE chatId = ?1 ORDER BY timestamp ASC",
            sqliteParams![chatId],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn countMessagesForChatUpToTimestamp(
        &self,
        chatId: &str,
        upToTimestampInclusive: Option<i64>,
    ) -> Result<i32, SqliteStoreError> {
        self.store.queryScalar(
            "SELECT COUNT(*) FROM messages WHERE chatId = ?1 AND (?2 IS NULL OR timestamp <= ?2)",
            sqliteParams![chatId, upToTimestampInclusive],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getLocatorPreviewsForChat(
        &self,
        chatId: &str,
        previewCharCount: i32,
    ) -> Result<Vec<ChatMessageLocatorPreview>, SqliteStoreError> {
        self.store
            .queryRows(
                r#"
                SELECT
                    (
                        SELECT COUNT(*)
                        FROM messages AS earlier
                        WHERE earlier.chatId = messages.chatId
                            AND earlier.timestamp < messages.timestamp
                    ) AS messageIndex,
                    timestamp AS timestamp,
                    sender AS sender,
                    CASE
                        WHEN sender = 'user' AND displayMode = 'HIDDEN_PLACEHOLDER' THEN ''
                        ELSE SUBSTR((
                            SELECT GROUP_CONCAT(content, '')
                            FROM (
                                SELECT content FROM message_parts
                                WHERE message_parts.chatId = messages.chatId
                                    AND message_parts.messageTimestamp = messages.timestamp
                                    AND message_parts.variantIndex = messages.selectedVariantIndex
                                    AND message_parts.kind IN ('markdown', 'status')
                                ORDER BY sequence ASC
                            )
                        ), 1, ?2)
                    END AS previewContent,
                    CASE
                        WHEN sender = 'user' AND displayMode = 'HIDDEN_PLACEHOLDER' THEN 0
                        ELSE LENGTH((
                            SELECT GROUP_CONCAT(content, '')
                            FROM (
                                SELECT content FROM message_parts
                                WHERE message_parts.chatId = messages.chatId
                                    AND message_parts.messageTimestamp = messages.timestamp
                                    AND message_parts.variantIndex = messages.selectedVariantIndex
                                    AND message_parts.kind IN ('markdown', 'status')
                                ORDER BY sequence ASC
                            )
                        ))
                    END AS contentLength,
                    displayMode AS displayMode,
                    isFavorite AS isFavorite
                FROM messages
                WHERE chatId = ?1
                ORDER BY timestamp ASC
                "#,
                sqliteParams![chatId, previewCharCount],
            )?
            .into_iter()
            .map(|row| {
                Ok(ChatMessageLocatorPreview {
                    messageIndex: row.get(0)?,
                    timestamp: row.get(1)?,
                    sender: row.get(2)?,
                    previewContent: row.get(3)?,
                    contentLength: row.get(4)?,
                    displayMode: row.get(5)?,
                    isFavorite: row.get(6)?,
                })
            })
            .collect()
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn searchLocatorPreviewsForChat(
        &self,
        chatId: &str,
        query: &str,
        previewCharCount: i32,
    ) -> Result<Vec<ChatMessageLocatorPreview>, SqliteStoreError> {
        self.store
            .queryRows(
                r#"
                SELECT
                    (
                        SELECT COUNT(*)
                        FROM messages AS earlier
                        WHERE earlier.chatId = messages.chatId
                            AND earlier.timestamp < messages.timestamp
                    ) AS messageIndex,
                    timestamp AS timestamp,
                    sender AS sender,
                    SUBSTR(
                        (
                            SELECT GROUP_CONCAT(content, '')
                            FROM (
                                SELECT content FROM message_parts
                                WHERE message_parts.chatId = messages.chatId
                                    AND message_parts.messageTimestamp = messages.timestamp
                                    AND message_parts.variantIndex = messages.selectedVariantIndex
                                    AND message_parts.kind IN ('markdown', 'status')
                                ORDER BY sequence ASC
                            )
                        ),
                        MAX(1, INSTR(LOWER((
                            SELECT GROUP_CONCAT(content, '')
                            FROM (
                                SELECT content FROM message_parts
                                WHERE message_parts.chatId = messages.chatId
                                    AND message_parts.messageTimestamp = messages.timestamp
                                    AND message_parts.variantIndex = messages.selectedVariantIndex
                                    AND message_parts.kind IN ('markdown', 'status')
                                ORDER BY sequence ASC
                            )
                        )), LOWER(?2)) - (?3 / 2)),
                        ?3
                    ) AS previewContent,
                    LENGTH((
                        SELECT GROUP_CONCAT(content, '')
                        FROM (
                            SELECT content FROM message_parts
                            WHERE message_parts.chatId = messages.chatId
                                AND message_parts.messageTimestamp = messages.timestamp
                                AND message_parts.variantIndex = messages.selectedVariantIndex
                                AND message_parts.kind IN ('markdown', 'status')
                            ORDER BY sequence ASC
                        )
                    )) AS contentLength,
                    displayMode AS displayMode,
                    isFavorite AS isFavorite
                FROM messages
                WHERE chatId = ?1
                    AND NOT (sender = 'user' AND displayMode = 'HIDDEN_PLACEHOLDER')
                    AND INSTR(LOWER((
                        SELECT GROUP_CONCAT(content, '')
                        FROM (
                            SELECT content FROM message_parts
                            WHERE message_parts.chatId = messages.chatId
                                AND message_parts.messageTimestamp = messages.timestamp
                                AND message_parts.variantIndex = messages.selectedVariantIndex
                                AND message_parts.kind IN ('markdown', 'status')
                            ORDER BY sequence ASC
                        )
                    )), LOWER(?2)) > 0
                ORDER BY timestamp ASC
                "#,
                sqliteParams![chatId, query, previewCharCount],
            )?
            .into_iter()
            .map(|row| {
                Ok(ChatMessageLocatorPreview {
                    messageIndex: row.get(0)?,
                    timestamp: row.get(1)?,
                    sender: row.get(2)?,
                    previewContent: row.get(3)?,
                    contentLength: row.get(4)?,
                    displayMode: row.get(5)?,
                    isFavorite: row.get(6)?,
                })
            })
            .collect()
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getMessagesForChatFromTimestampAsc(
        &self,
        chatId: &str,
        startTimestampInclusive: i64,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.selectMessages(
            "SELECT * FROM messages WHERE chatId = ?1 AND timestamp >= ?2 ORDER BY timestamp ASC",
            sqliteParams![chatId, startTimestampInclusive],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getMessagesForChatWindowAsc(
        &self,
        chatId: &str,
        startTimestampInclusive: i64,
        endTimestampInclusive: i64,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.selectMessages(
            "SELECT * FROM messages WHERE chatId = ?1 AND timestamp >= ?2 AND timestamp <= ?3 ORDER BY timestamp ASC",
            sqliteParams![chatId, startTimestampInclusive, endTimestampInclusive],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getMessagesForChatAsc(
        &self,
        chatId: &str,
        limit: i32,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.selectMessages(
            "SELECT * FROM messages WHERE chatId = ?1 ORDER BY timestamp ASC LIMIT ?2",
            sqliteParams![chatId, limit],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getMessagesForChatDesc(
        &self,
        chatId: &str,
        limit: i32,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.selectMessages(
            "SELECT * FROM messages WHERE chatId = ?1 ORDER BY timestamp DESC LIMIT ?2",
            sqliteParams![chatId, limit],
        )
    }

    /// Loads an ascending message range using a zero-based offset and limit.
    pub fn getMessagesForChatAscRange(
        &self,
        chatId: &str,
        offset: i32,
        limit: i32,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.selectMessages(
            "SELECT * FROM messages WHERE chatId = ?1 ORDER BY timestamp ASC LIMIT ?3 OFFSET ?2",
            sqliteParams![chatId, offset, limit],
        )
    }

    /// Loads a descending message range using a zero-based offset and limit.
    pub fn getMessagesForChatDescRange(
        &self,
        chatId: &str,
        offset: i32,
        limit: i32,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.selectMessages(
            "SELECT * FROM messages WHERE chatId = ?1 ORDER BY timestamp DESC LIMIT ?3 OFFSET ?2",
            sqliteParams![chatId, offset, limit],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getMessagesForChatAfterTimestampExclusiveAsc(
        &self,
        chatId: &str,
        afterTimestampExclusive: i64,
        limit: i32,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.selectMessages(
            "SELECT * FROM messages WHERE chatId = ?1 AND timestamp > ?2 ORDER BY timestamp ASC LIMIT ?3",
            sqliteParams![chatId, afterTimestampExclusive, limit],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getMessagesForChatInRangeAsc(
        &self,
        chatId: &str,
        afterTimestampExclusive: Option<i64>,
        beforeTimestampExclusive: Option<i64>,
        upToTimestampInclusive: Option<i64>,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.store
            .queryRows(
                r#"
                SELECT * FROM messages
                WHERE chatId = ?1
                    AND (?2 IS NULL OR timestamp > ?2)
                    AND (?3 IS NULL OR timestamp < ?3)
                    AND (?4 IS NULL OR timestamp <= ?4)
                ORDER BY timestamp ASC
                "#,
                sqliteParams![
                    chatId,
                    afterTimestampExclusive,
                    beforeTimestampExclusive,
                    upToTimestampInclusive,
                ],
            )?
            .into_iter()
            .map(|row| mapMessageEntity(&row))
            .collect()
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getMessagesForChatBeforeTimestampDesc(
        &self,
        chatId: &str,
        maxTimestamp: i64,
        limit: i32,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.selectMessages(
            "SELECT * FROM messages WHERE chatId = ?1 AND timestamp <= ?2 ORDER BY timestamp DESC LIMIT ?3",
            sqliteParams![chatId, maxTimestamp, limit],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getMessagesForChatBeforeTimestampExclusiveDesc(
        &self,
        chatId: &str,
        beforeTimestampExclusive: i64,
        limit: i32,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.selectMessages(
            "SELECT * FROM messages WHERE chatId = ?1 AND timestamp < ?2 ORDER BY timestamp DESC LIMIT ?3",
            sqliteParams![chatId, beforeTimestampExclusive, limit],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn existsMessagesBeforeTimestamp(
        &self,
        chatId: &str,
        beforeTimestampExclusive: i64,
    ) -> Result<bool, SqliteStoreError> {
        self.exists(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE chatId = ?1 AND timestamp < ?2 LIMIT 1)",
            sqliteParams![chatId, beforeTimestampExclusive],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn existsMessagesAfterTimestamp(
        &self,
        chatId: &str,
        afterTimestampExclusive: i64,
    ) -> Result<bool, SqliteStoreError> {
        self.exists(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE chatId = ?1 AND timestamp > ?2 LIMIT 1)",
            sqliteParams![chatId, afterTimestampExclusive],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getLatestSummaryTimestamp(&self, chatId: &str) -> Result<Option<i64>, SqliteStoreError> {
        self.optionalTimestamp(
            "SELECT timestamp FROM messages WHERE chatId = ?1 AND sender = 'summary' ORDER BY timestamp DESC LIMIT 1",
            sqliteParams![chatId],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getLatestSummaryTimestampBefore(
        &self,
        chatId: &str,
        beforeTimestampExclusive: i64,
    ) -> Result<Option<i64>, SqliteStoreError> {
        self.optionalTimestamp(
            "SELECT timestamp FROM messages WHERE chatId = ?1 AND sender = 'summary' AND timestamp < ?2 ORDER BY timestamp DESC LIMIT 1",
            sqliteParams![chatId, beforeTimestampExclusive],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getLatestSummaryTimestampUpTo(
        &self,
        chatId: &str,
        upToTimestampInclusive: i64,
    ) -> Result<Option<i64>, SqliteStoreError> {
        self.optionalTimestamp(
            "SELECT timestamp FROM messages WHERE chatId = ?1 AND sender = 'summary' AND timestamp <= ?2 ORDER BY timestamp DESC LIMIT 1",
            sqliteParams![chatId, upToTimestampInclusive],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn existsUserMessage(&self, chatId: &str) -> Result<bool, SqliteStoreError> {
        self.exists(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE chatId = ?1 AND sender = 'user' LIMIT 1)",
            sqliteParams![chatId],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getMaxOrderIndex(&self, chatId: &str) -> Result<Option<i32>, SqliteStoreError> {
        self.store
            .queryOne(
                "SELECT MAX(orderIndex) FROM messages WHERE chatId = ?1",
                sqliteParams![chatId],
            )?
            .map(|row| row.get(0))
            .transpose()
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn insertMessage(&self, message: MessageEntity) -> Result<i64, SqliteStoreError> {
        if message.messageId == 0 {
            self.store.execute(
                insertMessageSql(false),
                insertMessageParams(&message, false)?,
            )?;
            let rowId: i64 = self
                .store
                .queryScalar("SELECT last_insert_rowid()", sqliteParams![])?;
            Ok(rowId)
        } else {
            self.store
                .execute(insertMessageSql(true), insertMessageParams(&message, true)?)?;
            Ok(message.messageId)
        }
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn insertMessages(&self, messages: Vec<MessageEntity>) -> Result<(), SqliteStoreError> {
        self.store.transaction(|transaction| {
            for message in messages {
                if message.messageId == 0 {
                    transaction.execute(
                        insertMessageSql(false),
                        insertMessageParams(&message, false)?,
                    )?;
                } else {
                    transaction
                        .execute(insertMessageSql(true), insertMessageParams(&message, true)?)?;
                }
            }
            Ok(())
        })
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn copyMessagesToChat(
        &self,
        sourceChatId: &str,
        targetChatId: &str,
        upToTimestampInclusive: Option<i64>,
    ) -> Result<(), SqliteStoreError> {
        self.store.execute(
            r#"
                INSERT INTO messages (
                    chatId, sender, timestamp, orderIndex, roleName,
                    selectedVariantIndex, provider, modelName, inputTokens, outputTokens,
                    cachedInputTokens, sentAt, outputDurationMs, waitDurationMs,
                    completedAt, completedExecutionGeneration, displayMode, isFavorite, pluginExtensions
                )
                SELECT
                    ?2, sender, timestamp, orderIndex, roleName,
                    selectedVariantIndex, provider, modelName, inputTokens, outputTokens,
                    cachedInputTokens, sentAt, outputDurationMs, waitDurationMs,
                    completedAt, completedExecutionGeneration, displayMode, isFavorite, pluginExtensions
                FROM messages
                WHERE chatId = ?1 AND (?3 IS NULL OR timestamp <= ?3)
                "#,
            sqliteParams![sourceChatId, targetChatId, upToTimestampInclusive],
        )?;
        Ok(())
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn updateMessage(&self, message: MessageEntity) -> Result<(), SqliteStoreError> {
        self.store.execute(
            r#"
                UPDATE messages
                SET chatId = ?2, sender = ?3, timestamp = ?4,
                    orderIndex = ?5, roleName = ?6, selectedVariantIndex = ?7,
                    provider = ?8, modelName = ?9, inputTokens = ?10,
                    outputTokens = ?11, cachedInputTokens = ?12, sentAt = ?13,
                    outputDurationMs = ?14, waitDurationMs = ?15, completedAt = ?16,
                    completedExecutionGeneration = ?17,
                    displayMode = ?18, isFavorite = ?19
                WHERE messageId = ?1
                "#,
            sqliteParams![
                message.messageId,
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

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn deleteAllMessagesForChat(&self, chatId: &str) -> Result<(), SqliteStoreError> {
        self.store.execute(
            "DELETE FROM messages WHERE chatId = ?1",
            sqliteParams![chatId],
        )?;
        Ok(())
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getMessageByTimestamp(
        &self,
        chatId: &str,
        timestamp: i64,
    ) -> Result<Option<MessageEntity>, SqliteStoreError> {
        self.store
            .queryOne(
                r#"
                SELECT * FROM messages
                WHERE chatId = ?1 AND timestamp = ?2
                LIMIT 1
                "#,
                sqliteParams![chatId, timestamp],
            )?
            .map(|row| mapMessageEntity(&row))
            .transpose()
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn deleteMessagesFrom(&self, chatId: &str, timestamp: i64) -> Result<(), SqliteStoreError> {
        self.store.execute(
            "DELETE FROM messages WHERE chatId = ?1 AND timestamp >= ?2",
            sqliteParams![chatId, timestamp],
        )?;
        Ok(())
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn deleteMessageByTimestamp(
        &self,
        chatId: &str,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.store.execute(
            "DELETE FROM messages WHERE chatId = ?1 AND timestamp = ?2",
            sqliteParams![chatId, timestamp],
        )?;
        Ok(())
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn getMessageCountsByChatId(&self) -> Result<Vec<ChatMessageCount>, SqliteStoreError> {
        self.store
            .queryRows(
                "SELECT chatId AS chatId, COUNT(*) AS count FROM messages GROUP BY chatId",
                sqliteParams![],
            )?
            .into_iter()
            .map(|row| {
                Ok(ChatMessageCount {
                    chatId: row.get(0)?,
                    count: row.get(1)?,
                })
            })
            .collect()
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn updateSelectedVariantIndex(
        &self,
        chatId: &str,
        timestamp: i64,
        selectedVariantIndex: i32,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE messages SET selectedVariantIndex = ?3 WHERE chatId = ?1 AND timestamp = ?2",
            sqliteParams![chatId, timestamp, selectedVariantIndex],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn updateMessageFavorite(
        &self,
        chatId: &str,
        timestamp: i64,
        isFavorite: bool,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE messages SET isFavorite = ?3 WHERE chatId = ?1 AND timestamp = ?2",
            sqliteParams![chatId, timestamp, isFavorite],
        )
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn searchChatIdsByContent(&self, query: &str) -> Result<Vec<String>, SqliteStoreError> {
        self.store
            .queryRows(
                r#"
                SELECT DISTINCT messages.chatId
                FROM messages
                INNER JOIN message_parts
                    ON message_parts.chatId = messages.chatId
                    AND message_parts.messageTimestamp = messages.timestamp
                    AND message_parts.variantIndex = messages.selectedVariantIndex
                WHERE message_parts.kind IN ('markdown', 'status')
                    AND message_parts.content LIKE '%' || ?1 || '%' ESCAPE '\\' COLLATE NOCASE
                "#,
                sqliteParams![query],
            )?
            .into_iter()
            .map(|row| row.get(0))
            .collect()
    }

    /// Processes the exact persisted message record with explicit storage errors.
    pub fn renameRoleName(&self, oldName: &str, newName: &str) -> Result<i32, SqliteStoreError> {
        let count = self.store.execute(
            "UPDATE messages SET roleName = ?2 WHERE roleName = ?1",
            sqliteParams![oldName, newName],
        )? as i32;
        Ok(count)
    }

    /// Processes the exact persisted message record with explicit storage errors.
    fn selectMessages(
        &self,
        sql: &str,
        params: Vec<SqliteValue>,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.store
            .queryRows(sql, params)?
            .into_iter()
            .map(|row| mapMessageEntity(&row))
            .collect()
    }

    /// Processes the exact persisted message record with explicit storage errors.
    fn exists(&self, sql: &str, params: Vec<SqliteValue>) -> Result<bool, SqliteStoreError> {
        let value: i32 = self.store.queryScalar(sql, params)?;
        Ok(value != 0)
    }

    /// Processes the exact persisted message record with explicit storage errors.
    fn optionalTimestamp(
        &self,
        sql: &str,
        params: Vec<SqliteValue>,
    ) -> Result<Option<i64>, SqliteStoreError> {
        self.store
            .queryOne(sql, params)?
            .map(|row| row.get(0))
            .transpose()
    }

    /// Processes the exact persisted message record with explicit storage errors.
    fn execute(&self, sql: &str, params: Vec<SqliteValue>) -> Result<(), SqliteStoreError> {
        self.store.execute(sql, params)?;
        Ok(())
    }
}

/// Processes the exact persisted message record with explicit storage errors.
pub(crate) fn mapMessageEntity(row: &SqliteRow) -> Result<MessageEntity, SqliteStoreError> {
    Ok(MessageEntity {
        messageId: row.get("messageId")?,
        chatId: row.get("chatId")?,
        sender: row.get("sender")?,
        timestamp: row.get("timestamp")?,
        orderIndex: row.get("orderIndex")?,
        roleName: row.get("roleName")?,
        selectedVariantIndex: row.get("selectedVariantIndex")?,
        provider: row.get("provider")?,
        modelName: row.get("modelName")?,
        inputTokens: row.get("inputTokens")?,
        outputTokens: row.get("outputTokens")?,
        cachedInputTokens: row.get("cachedInputTokens")?,
        sentAt: row.get("sentAt")?,
        outputDurationMs: row.get("outputDurationMs")?,
        waitDurationMs: row.get("waitDurationMs")?,
        completedAt: row.get("completedAt")?,
        completedExecutionGeneration: row.get("completedExecutionGeneration")?,
        displayMode: row.get("displayMode")?,
        isFavorite: row.get("isFavorite")?,
        pluginExtensions: decodePluginExtensions(&row.get::<_, String>("pluginExtensions")?)?,
    })
}

/// Processes the exact persisted message record with explicit storage errors.
pub(crate) fn insertMessageSql(withMessageId: bool) -> &'static str {
    if withMessageId {
        r#"
        INSERT INTO messages (
            messageId, chatId, sender, timestamp, orderIndex,
            roleName, selectedVariantIndex, provider, modelName, inputTokens,
            outputTokens, cachedInputTokens, sentAt, outputDurationMs,
            waitDurationMs, completedAt, completedExecutionGeneration, displayMode, isFavorite, pluginExtensions
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
        ON CONFLICT(chatId, timestamp) DO UPDATE SET sender = excluded.sender, orderIndex = excluded.orderIndex, roleName = excluded.roleName, selectedVariantIndex = excluded.selectedVariantIndex, provider = excluded.provider, modelName = excluded.modelName, inputTokens = excluded.inputTokens, outputTokens = excluded.outputTokens, cachedInputTokens = excluded.cachedInputTokens, sentAt = excluded.sentAt, outputDurationMs = excluded.outputDurationMs, waitDurationMs = excluded.waitDurationMs, completedAt = excluded.completedAt, completedExecutionGeneration = excluded.completedExecutionGeneration, displayMode = excluded.displayMode, isFavorite = excluded.isFavorite
        "#
    } else {
        r#"
        INSERT INTO messages (
            chatId, sender, timestamp, orderIndex,
            roleName, selectedVariantIndex, provider, modelName, inputTokens,
            outputTokens, cachedInputTokens, sentAt, outputDurationMs,
            waitDurationMs, completedAt, completedExecutionGeneration, displayMode, isFavorite, pluginExtensions
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
        ON CONFLICT(chatId, timestamp) DO UPDATE SET sender = excluded.sender, orderIndex = excluded.orderIndex, roleName = excluded.roleName, selectedVariantIndex = excluded.selectedVariantIndex, provider = excluded.provider, modelName = excluded.modelName, inputTokens = excluded.inputTokens, outputTokens = excluded.outputTokens, cachedInputTokens = excluded.cachedInputTokens, sentAt = excluded.sentAt, outputDurationMs = excluded.outputDurationMs, waitDurationMs = excluded.waitDurationMs, completedAt = excluded.completedAt, completedExecutionGeneration = excluded.completedExecutionGeneration, displayMode = excluded.displayMode, isFavorite = excluded.isFavorite
        "#
    }
}

/// Processes the exact persisted message record with explicit storage errors.
pub(crate) fn insertMessageParams(
    message: &MessageEntity,
    withMessageId: bool,
) -> Result<Vec<SqliteValue>, SqliteStoreError> {
    if withMessageId {
        Ok(sqliteParams![
            message.messageId,
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
            encodePluginExtensions(&message.pluginExtensions)?,
        ])
    } else {
        Ok(sqliteParams![
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
            encodePluginExtensions(&message.pluginExtensions)?,
        ])
    }
}
