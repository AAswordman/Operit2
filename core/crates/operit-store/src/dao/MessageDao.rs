use crate::sqliteParams;
use crate::SqliteStore::{SqliteRow, SqliteRowGet, SqliteStore, SqliteStoreError, SqliteValue};

use operit_model::ChatMessageLocatorPreview::ChatMessageLocatorPreview;
use operit_model::MessageEntity::{ChatMessageCount, MessageEntity};

#[derive(Clone)]
pub struct MessageDao {
    store: SqliteStore,
}

impl MessageDao {
    pub fn new(store: SqliteStore) -> Self {
        Self { store }
    }

    pub fn getTotalMessageCount(&self) -> Result<i32, SqliteStoreError> {
        self.store
            .queryScalar("SELECT COUNT(*) FROM messages", sqliteParams![])
    }

    pub fn getMessagesForChat(&self, chatId: &str) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.selectMessages(
            "SELECT * FROM messages WHERE chatId = ?1 ORDER BY timestamp ASC",
            sqliteParams![chatId],
        )
    }

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
                        ELSE SUBSTR(content, 1, ?2)
                    END AS previewContent,
                    CASE
                        WHEN sender = 'user' AND displayMode = 'HIDDEN_PLACEHOLDER' THEN 0
                        ELSE LENGTH(content)
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
                        content,
                        MAX(1, INSTR(LOWER(content), LOWER(?2)) - (?3 / 2)),
                        ?3
                    ) AS previewContent,
                    LENGTH(content) AS contentLength,
                    displayMode AS displayMode,
                    isFavorite AS isFavorite
                FROM messages
                WHERE chatId = ?1
                    AND NOT (sender = 'user' AND displayMode = 'HIDDEN_PLACEHOLDER')
                    AND INSTR(LOWER(content), LOWER(?2)) > 0
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

    pub fn getLatestSummaryTimestamp(&self, chatId: &str) -> Result<Option<i64>, SqliteStoreError> {
        self.optionalTimestamp(
            "SELECT timestamp FROM messages WHERE chatId = ?1 AND sender = 'summary' ORDER BY timestamp DESC LIMIT 1",
            sqliteParams![chatId],
        )
    }

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

    pub fn existsUserMessage(&self, chatId: &str) -> Result<bool, SqliteStoreError> {
        self.exists(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE chatId = ?1 AND sender = 'user' LIMIT 1)",
            sqliteParams![chatId],
        )
    }

    pub fn getMaxOrderIndex(&self, chatId: &str) -> Result<Option<i32>, SqliteStoreError> {
        self.store
            .queryOne(
                "SELECT MAX(orderIndex) FROM messages WHERE chatId = ?1",
                sqliteParams![chatId],
            )?
            .map(|row| row.get(0))
            .transpose()
    }

    pub fn insertMessage(&self, message: MessageEntity) -> Result<i64, SqliteStoreError> {
        if message.messageId == 0 {
            self.store.execute(
                insertMessageSql(false),
                insertMessageParams(&message, false),
            )?;
            let rowId: i64 = self
                .store
                .queryScalar("SELECT last_insert_rowid()", sqliteParams![])?;
            Ok(rowId)
        } else {
            self.store
                .execute(insertMessageSql(true), insertMessageParams(&message, true))?;
            Ok(message.messageId)
        }
    }

    pub fn insertMessages(&self, messages: Vec<MessageEntity>) -> Result<(), SqliteStoreError> {
        self.store.transaction(|transaction| {
            for message in messages {
                if message.messageId == 0 {
                    transaction.execute(
                        insertMessageSql(false),
                        insertMessageParams(&message, false),
                    )?;
                } else {
                    transaction
                        .execute(insertMessageSql(true), insertMessageParams(&message, true))?;
                }
            }
            Ok(())
        })
    }

    pub fn copyMessagesToChat(
        &self,
        sourceChatId: &str,
        targetChatId: &str,
        upToTimestampInclusive: Option<i64>,
    ) -> Result<(), SqliteStoreError> {
        self.store.execute(
            r#"
                INSERT INTO messages (
                    chatId, sender, content, timestamp, orderIndex, roleName,
                    selectedVariantIndex, provider, modelName, inputTokens, outputTokens,
                    cachedInputTokens, sentAt, outputDurationMs, waitDurationMs,
                    completedAt, displayMode, isFavorite
                )
                SELECT
                    ?2, sender, content, timestamp, orderIndex, roleName,
                    selectedVariantIndex, provider, modelName, inputTokens, outputTokens,
                    cachedInputTokens, sentAt, outputDurationMs, waitDurationMs,
                    completedAt, displayMode, isFavorite
                FROM messages
                WHERE chatId = ?1 AND (?3 IS NULL OR timestamp <= ?3)
                "#,
            sqliteParams![sourceChatId, targetChatId, upToTimestampInclusive],
        )?;
        Ok(())
    }

    pub fn updateMessage(&self, message: MessageEntity) -> Result<(), SqliteStoreError> {
        self.store.execute(
            r#"
                UPDATE messages
                SET chatId = ?2, sender = ?3, content = ?4, timestamp = ?5,
                    orderIndex = ?6, roleName = ?7, selectedVariantIndex = ?8,
                    provider = ?9, modelName = ?10, inputTokens = ?11,
                    outputTokens = ?12, cachedInputTokens = ?13, sentAt = ?14,
                    outputDurationMs = ?15, waitDurationMs = ?16, completedAt = ?17,
                    displayMode = ?18, isFavorite = ?19
                WHERE messageId = ?1
                "#,
            sqliteParams![
                message.messageId,
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

    pub fn updateMessageContent(
        &self,
        messageId: i64,
        content: String,
    ) -> Result<(), SqliteStoreError> {
        self.store.execute(
            "UPDATE messages SET content = ?2 WHERE messageId = ?1",
            sqliteParams![messageId, content],
        )?;
        Ok(())
    }

    pub fn deleteAllMessagesForChat(&self, chatId: &str) -> Result<(), SqliteStoreError> {
        self.store.execute(
            "DELETE FROM messages WHERE chatId = ?1",
            sqliteParams![chatId],
        )?;
        Ok(())
    }

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

    pub fn deleteMessagesFrom(&self, chatId: &str, timestamp: i64) -> Result<(), SqliteStoreError> {
        self.store.execute(
            "DELETE FROM messages WHERE chatId = ?1 AND timestamp >= ?2",
            sqliteParams![chatId, timestamp],
        )?;
        Ok(())
    }

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

    pub fn searchChatIdsByContent(&self, query: &str) -> Result<Vec<String>, SqliteStoreError> {
        self.store
            .queryRows(
                "SELECT DISTINCT chatId FROM messages WHERE content LIKE '%' || ?1 || '%' ESCAPE '\\' COLLATE NOCASE",
                sqliteParams![query],
            )?
            .into_iter()
            .map(|row| row.get(0))
            .collect()
    }

    pub fn renameRoleName(&self, oldName: &str, newName: &str) -> Result<i32, SqliteStoreError> {
        let count = self.store.execute(
            "UPDATE messages SET roleName = ?2 WHERE roleName = ?1",
            sqliteParams![oldName, newName],
        )? as i32;
        Ok(count)
    }

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

    fn exists(&self, sql: &str, params: Vec<SqliteValue>) -> Result<bool, SqliteStoreError> {
        let value: i32 = self.store.queryScalar(sql, params)?;
        Ok(value != 0)
    }

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

    fn execute(&self, sql: &str, params: Vec<SqliteValue>) -> Result<(), SqliteStoreError> {
        self.store.execute(sql, params)?;
        Ok(())
    }
}

fn mapMessageEntity(row: &SqliteRow) -> Result<MessageEntity, SqliteStoreError> {
    Ok(MessageEntity {
        messageId: row.get("messageId")?,
        chatId: row.get("chatId")?,
        sender: row.get("sender")?,
        content: row.get("content")?,
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
        displayMode: row.get("displayMode")?,
        isFavorite: row.get("isFavorite")?,
    })
}

fn insertMessageSql(withMessageId: bool) -> &'static str {
    if withMessageId {
        r#"
        INSERT OR REPLACE INTO messages (
            messageId, chatId, sender, content, timestamp, orderIndex,
            roleName, selectedVariantIndex, provider, modelName, inputTokens,
            outputTokens, cachedInputTokens, sentAt, outputDurationMs,
            waitDurationMs, completedAt, displayMode, isFavorite
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
        "#
    } else {
        r#"
        INSERT OR REPLACE INTO messages (
            chatId, sender, content, timestamp, orderIndex,
            roleName, selectedVariantIndex, provider, modelName, inputTokens,
            outputTokens, cachedInputTokens, sentAt, outputDurationMs,
            waitDurationMs, completedAt, displayMode, isFavorite
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
        "#
    }
}

fn insertMessageParams(message: &MessageEntity, withMessageId: bool) -> Vec<SqliteValue> {
    if withMessageId {
        sqliteParams![
            message.messageId,
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
        ]
    } else {
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
        ]
    }
}
