use operit_store::SqliteStore::{SqliteStore, SqliteStoreError};
use rusqlite::{params, params_from_iter, Row};

use crate::data::model::ChatMessageLocatorPreview::ChatMessageLocatorPreview;
use crate::data::model::MessageEntity::{ChatMessageCount, MessageEntity};

#[derive(Clone)]
pub struct MessageDao {
    store: SqliteStore,
}

impl MessageDao {
    pub fn new(store: SqliteStore) -> Self {
        Self { store }
    }

    pub fn getTotalMessageCount(&self) -> Result<i32, SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
        })
    }

    pub fn getMessagesForChat(&self, chatId: &str) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.selectMessages(
            "SELECT * FROM messages WHERE chatId = ?1 ORDER BY timestamp ASC",
            params![chatId],
        )
    }

    pub fn countMessagesForChatUpToTimestamp(
        &self,
        chatId: &str,
        upToTimestampInclusive: Option<i64>,
    ) -> Result<i32, SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.query_row(
                "SELECT COUNT(*) FROM messages WHERE chatId = ?1 AND (?2 IS NULL OR timestamp <= ?2)",
                params![chatId, upToTimestampInclusive],
                |row| row.get(0),
            )
        })
    }

    pub fn getLocatorPreviewsForChat(
        &self,
        chatId: &str,
        previewCharCount: i32,
    ) -> Result<Vec<ChatMessageLocatorPreview>, SqliteStoreError> {
        self.store.withConnection(|connection| {
            let mut statement = connection.prepare(
                r#"
                SELECT
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
            )?;
            let rows = statement.query_map(params![chatId, previewCharCount], |row| {
                Ok(ChatMessageLocatorPreview {
                    timestamp: row.get(0)?,
                    sender: row.get(1)?,
                    previewContent: row.get(2)?,
                    contentLength: row.get(3)?,
                    displayMode: row.get(4)?,
                    isFavorite: row.get(5)?,
                })
            })?;
            rows.collect()
        })
    }

    pub fn getMessagesForChatFromTimestampAsc(
        &self,
        chatId: &str,
        startTimestampInclusive: i64,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.selectMessages(
            "SELECT * FROM messages WHERE chatId = ?1 AND timestamp >= ?2 ORDER BY timestamp ASC",
            params![chatId, startTimestampInclusive],
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
            params![chatId, startTimestampInclusive, endTimestampInclusive],
        )
    }

    pub fn getMessagesForChatAsc(
        &self,
        chatId: &str,
        limit: i32,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.selectMessages(
            "SELECT * FROM messages WHERE chatId = ?1 ORDER BY timestamp ASC LIMIT ?2",
            params![chatId, limit],
        )
    }

    pub fn getMessagesForChatDesc(
        &self,
        chatId: &str,
        limit: i32,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.selectMessages(
            "SELECT * FROM messages WHERE chatId = ?1 ORDER BY timestamp DESC LIMIT ?2",
            params![chatId, limit],
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
            params![chatId, afterTimestampExclusive, limit],
        )
    }

    pub fn getMessagesForChatInRangeAsc(
        &self,
        chatId: &str,
        afterTimestampExclusive: Option<i64>,
        beforeTimestampExclusive: Option<i64>,
        upToTimestampInclusive: Option<i64>,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.store.withConnection(|connection| {
            let mut statement = connection.prepare(
                r#"
                SELECT * FROM messages
                WHERE chatId = ?1
                    AND (?2 IS NULL OR timestamp > ?2)
                    AND (?3 IS NULL OR timestamp < ?3)
                    AND (?4 IS NULL OR timestamp <= ?4)
                ORDER BY timestamp ASC
                "#,
            )?;
            let rows = statement.query_map(
                params![
                    chatId,
                    afterTimestampExclusive,
                    beforeTimestampExclusive,
                    upToTimestampInclusive,
                ],
                mapMessageEntity,
            )?;
            rows.collect()
        })
    }

    pub fn getMessagesForChatBeforeTimestampDesc(
        &self,
        chatId: &str,
        maxTimestamp: i64,
        limit: i32,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.selectMessages(
            "SELECT * FROM messages WHERE chatId = ?1 AND timestamp <= ?2 ORDER BY timestamp DESC LIMIT ?3",
            params![chatId, maxTimestamp, limit],
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
            params![chatId, beforeTimestampExclusive, limit],
        )
    }

    pub fn existsMessagesBeforeTimestamp(
        &self,
        chatId: &str,
        beforeTimestampExclusive: i64,
    ) -> Result<bool, SqliteStoreError> {
        self.exists(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE chatId = ?1 AND timestamp < ?2 LIMIT 1)",
            params![chatId, beforeTimestampExclusive],
        )
    }

    pub fn existsMessagesAfterTimestamp(
        &self,
        chatId: &str,
        afterTimestampExclusive: i64,
    ) -> Result<bool, SqliteStoreError> {
        self.exists(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE chatId = ?1 AND timestamp > ?2 LIMIT 1)",
            params![chatId, afterTimestampExclusive],
        )
    }

    pub fn getLatestSummaryTimestamp(&self, chatId: &str) -> Result<Option<i64>, SqliteStoreError> {
        self.optionalTimestamp(
            "SELECT timestamp FROM messages WHERE chatId = ?1 AND sender = 'summary' ORDER BY timestamp DESC LIMIT 1",
            params![chatId],
        )
    }

    pub fn getLatestSummaryTimestampBefore(
        &self,
        chatId: &str,
        beforeTimestampExclusive: i64,
    ) -> Result<Option<i64>, SqliteStoreError> {
        self.optionalTimestamp(
            "SELECT timestamp FROM messages WHERE chatId = ?1 AND sender = 'summary' AND timestamp < ?2 ORDER BY timestamp DESC LIMIT 1",
            params![chatId, beforeTimestampExclusive],
        )
    }

    pub fn getLatestSummaryTimestampUpTo(
        &self,
        chatId: &str,
        upToTimestampInclusive: i64,
    ) -> Result<Option<i64>, SqliteStoreError> {
        self.optionalTimestamp(
            "SELECT timestamp FROM messages WHERE chatId = ?1 AND sender = 'summary' AND timestamp <= ?2 ORDER BY timestamp DESC LIMIT 1",
            params![chatId, upToTimestampInclusive],
        )
    }

    pub fn existsUserMessage(&self, chatId: &str) -> Result<bool, SqliteStoreError> {
        self.exists(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE chatId = ?1 AND sender = 'user' LIMIT 1)",
            params![chatId],
        )
    }

    pub fn getMaxOrderIndex(&self, chatId: &str) -> Result<Option<i32>, SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.query_row(
                "SELECT MAX(orderIndex) FROM messages WHERE chatId = ?1",
                params![chatId],
                |row| row.get(0),
            )
        })
    }

    pub fn insertMessage(&self, message: MessageEntity) -> Result<i64, SqliteStoreError> {
        self.store.withConnection(|connection| {
            if message.messageId == 0 {
                connection.execute(
                    insertMessageSql(false),
                    params_from_iter(insertMessageParams(&message, false)),
                )?;
                Ok(connection.last_insert_rowid())
            } else {
                connection.execute(
                    insertMessageSql(true),
                    params_from_iter(insertMessageParams(&message, true)),
                )?;
                Ok(message.messageId)
            }
        })
    }

    pub fn insertMessages(&self, messages: Vec<MessageEntity>) -> Result<(), SqliteStoreError> {
        self.store.transaction(|transaction| {
            for message in messages {
                if message.messageId == 0 {
                    transaction.execute(
                        insertMessageSql(false),
                        params_from_iter(insertMessageParams(&message, false)),
                    )?;
                } else {
                    transaction.execute(
                        insertMessageSql(true),
                        params_from_iter(insertMessageParams(&message, true)),
                    )?;
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
        self.store.withConnection(|connection| {
            connection.execute(
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
                params![sourceChatId, targetChatId, upToTimestampInclusive],
            )?;
            Ok(())
        })
    }

    pub fn updateMessage(&self, message: MessageEntity) -> Result<(), SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.execute(
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
                params![
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
        })
    }

    pub fn updateMessageContent(
        &self,
        messageId: i64,
        content: String,
    ) -> Result<(), SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.execute(
                "UPDATE messages SET content = ?2 WHERE messageId = ?1",
                params![messageId, content],
            )?;
            Ok(())
        })
    }

    pub fn deleteAllMessagesForChat(&self, chatId: &str) -> Result<(), SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.execute("DELETE FROM messages WHERE chatId = ?1", params![chatId])?;
            Ok(())
        })
    }

    pub fn getMessageByTimestamp(
        &self,
        chatId: &str,
        timestamp: i64,
    ) -> Result<Option<MessageEntity>, SqliteStoreError> {
        self.store.withConnection(|connection| {
            let mut statement = connection.prepare(
                r#"
                SELECT * FROM messages
                WHERE chatId = ?1 AND timestamp = ?2
                LIMIT 1
                "#,
            )?;
            let result = statement.query_row(params![chatId, timestamp], mapMessageEntity);
            match result {
                Ok(message) => Ok(Some(message)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(error) => Err(error),
            }
        })
    }

    pub fn deleteMessagesFrom(
        &self,
        chatId: &str,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.execute(
                "DELETE FROM messages WHERE chatId = ?1 AND timestamp >= ?2",
                params![chatId, timestamp],
            )?;
            Ok(())
        })
    }

    pub fn deleteMessageByTimestamp(
        &self,
        chatId: &str,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.execute(
                "DELETE FROM messages WHERE chatId = ?1 AND timestamp = ?2",
                params![chatId, timestamp],
            )?;
            Ok(())
        })
    }

    pub fn getMessageCountsByChatId(&self) -> Result<Vec<ChatMessageCount>, SqliteStoreError> {
        self.store.withConnection(|connection| {
            let mut statement = connection.prepare(
                "SELECT chatId AS chatId, COUNT(*) AS count FROM messages GROUP BY chatId",
            )?;
            let rows = statement.query_map([], |row| {
                Ok(ChatMessageCount {
                    chatId: row.get(0)?,
                    count: row.get(1)?,
                })
            })?;
            rows.collect()
        })
    }

    pub fn updateSelectedVariantIndex(
        &self,
        chatId: &str,
        timestamp: i64,
        selectedVariantIndex: i32,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE messages SET selectedVariantIndex = ?3 WHERE chatId = ?1 AND timestamp = ?2",
            params![chatId, timestamp, selectedVariantIndex],
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
            params![chatId, timestamp, isFavorite],
        )
    }

    pub fn searchChatIdsByContent(&self, query: &str) -> Result<Vec<String>, SqliteStoreError> {
        self.store.withConnection(|connection| {
            let mut statement = connection.prepare(
                "SELECT DISTINCT chatId FROM messages WHERE content LIKE '%' || ?1 || '%' ESCAPE '\\' COLLATE NOCASE",
            )?;
            let rows = statement.query_map(params![query], |row| row.get(0))?;
            rows.collect()
        })
    }

    pub fn renameRoleName(&self, oldName: &str, newName: &str) -> Result<i32, SqliteStoreError> {
        self.store.withConnection(|connection| {
            Ok(connection.execute(
                "UPDATE messages SET roleName = ?2 WHERE roleName = ?1",
                params![oldName, newName],
            )? as i32)
        })
    }

    fn selectMessages<P: rusqlite::Params>(
        &self,
        sql: &str,
        params: P,
    ) -> Result<Vec<MessageEntity>, SqliteStoreError> {
        self.store.withConnection(|connection| {
            let mut statement = connection.prepare(sql)?;
            let rows = statement.query_map(params, mapMessageEntity)?;
            rows.collect()
        })
    }

    fn exists<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<bool, SqliteStoreError> {
        self.store.withConnection(|connection| {
            let value: i32 = connection.query_row(sql, params, |row| row.get(0))?;
            Ok(value != 0)
        })
    }

    fn optionalTimestamp<P: rusqlite::Params>(
        &self,
        sql: &str,
        params: P,
    ) -> Result<Option<i64>, SqliteStoreError> {
        self.store.withConnection(|connection| {
            let result = connection.query_row(sql, params, |row| row.get(0));
            match result {
                Ok(timestamp) => Ok(Some(timestamp)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(error) => Err(error),
            }
        })
    }

    fn execute<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<(), SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.execute(sql, params)?;
            Ok(())
        })
    }
}

fn mapMessageEntity(row: &Row<'_>) -> Result<MessageEntity, rusqlite::Error> {
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

fn insertMessageParams<'a>(
    message: &'a MessageEntity,
    withMessageId: bool,
) -> Vec<&'a dyn rusqlite::ToSql> {
    if withMessageId {
        vec![
            &message.messageId,
            &message.chatId,
            &message.sender,
            &message.content,
            &message.timestamp,
            &message.orderIndex,
            &message.roleName,
            &message.selectedVariantIndex,
            &message.provider,
            &message.modelName,
            &message.inputTokens,
            &message.outputTokens,
            &message.cachedInputTokens,
            &message.sentAt,
            &message.outputDurationMs,
            &message.waitDurationMs,
            &message.completedAt,
            &message.displayMode,
            &message.isFavorite,
        ]
    } else {
        vec![
            &message.chatId,
            &message.sender,
            &message.content,
            &message.timestamp,
            &message.orderIndex,
            &message.roleName,
            &message.selectedVariantIndex,
            &message.provider,
            &message.modelName,
            &message.inputTokens,
            &message.outputTokens,
            &message.cachedInputTokens,
            &message.sentAt,
            &message.outputDurationMs,
            &message.waitDurationMs,
            &message.completedAt,
            &message.displayMode,
            &message.isFavorite,
        ]
    }
}
