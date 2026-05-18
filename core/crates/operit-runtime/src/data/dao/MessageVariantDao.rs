use operit_store::SqliteStore::{SqliteStore, SqliteStoreError};
use rusqlite::{params, params_from_iter, Row};

use crate::data::model::MessageVariantEntity::MessageVariantEntity;

#[derive(Clone)]
pub struct MessageVariantDao {
    store: SqliteStore,
}

impl MessageVariantDao {
    pub fn new(store: SqliteStore) -> Self {
        Self { store }
    }

    pub fn getVariantsForChat(
        &self,
        chatId: &str,
    ) -> Result<Vec<MessageVariantEntity>, SqliteStoreError> {
        self.store.withConnection(|connection| {
            let mut statement = connection.prepare(
                r#"
                SELECT variantId, chatId, messageTimestamp, variantIndex, content, roleName,
                    provider, modelName, inputTokens, outputTokens, cachedInputTokens,
                    sentAt, outputDurationMs, waitDurationMs, completedAt
                FROM message_variants
                WHERE chatId = ?1
                ORDER BY messageTimestamp ASC, variantIndex ASC
                "#,
            )?;
            let rows = statement.query_map(params![chatId], mapMessageVariantEntity)?;
            rows.collect()
        })
    }

    pub fn getVariantsForMessages(
        &self,
        chatId: &str,
        messageTimestamps: Vec<i64>,
    ) -> Result<Vec<MessageVariantEntity>, SqliteStoreError> {
        self.store.withConnection(|connection| {
            let placeholders = messageTimestamps
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!(
                r#"
                SELECT variantId, chatId, messageTimestamp, variantIndex, content, roleName,
                    provider, modelName, inputTokens, outputTokens, cachedInputTokens,
                    sentAt, outputDurationMs, waitDurationMs, completedAt
                FROM message_variants
                WHERE chatId = ? AND messageTimestamp IN ({placeholders})
                ORDER BY messageTimestamp ASC, variantIndex ASC
                "#
            );
            let mut values: Vec<&dyn rusqlite::ToSql> = vec![&chatId];
            for timestamp in &messageTimestamps {
                values.push(timestamp);
            }
            let mut statement = connection.prepare(&sql)?;
            let rows =
                statement.query_map(params_from_iter(values), mapMessageVariantEntity)?;
            rows.collect()
        })
    }

    pub fn getVariantsForMessage(
        &self,
        chatId: &str,
        messageTimestamp: i64,
    ) -> Result<Vec<MessageVariantEntity>, SqliteStoreError> {
        self.store.withConnection(|connection| {
            let mut statement = connection.prepare(
                r#"
                SELECT variantId, chatId, messageTimestamp, variantIndex, content, roleName,
                    provider, modelName, inputTokens, outputTokens, cachedInputTokens,
                    sentAt, outputDurationMs, waitDurationMs, completedAt
                FROM message_variants
                WHERE chatId = ?1 AND messageTimestamp = ?2
                ORDER BY variantIndex ASC
                "#,
            )?;
            let rows =
                statement.query_map(params![chatId, messageTimestamp], mapMessageVariantEntity)?;
            rows.collect()
        })
    }

    pub fn getVariantForMessage(
        &self,
        chatId: &str,
        messageTimestamp: i64,
        variantIndex: i32,
    ) -> Result<Option<MessageVariantEntity>, SqliteStoreError> {
        self.store.withConnection(|connection| {
            let mut statement = connection.prepare(
                r#"
                SELECT variantId, chatId, messageTimestamp, variantIndex, content, roleName,
                    provider, modelName, inputTokens, outputTokens, cachedInputTokens,
                    sentAt, outputDurationMs, waitDurationMs, completedAt
                FROM message_variants
                WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3
                LIMIT 1
                "#,
            )?;
            let result = statement.query_row(
                params![chatId, messageTimestamp, variantIndex],
                mapMessageVariantEntity,
            );
            match result {
                Ok(variant) => Ok(Some(variant)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(error) => Err(error),
            }
        })
    }

    pub fn insertVariant(&self, variant: MessageVariantEntity) -> Result<i64, SqliteStoreError> {
        self.store.withConnection(|connection| {
            if variant.variantId == 0 {
                connection.execute(
                    insertVariantSql(false),
                    params_from_iter(insertVariantParams(&variant, false)),
                )?;
                Ok(connection.last_insert_rowid())
            } else {
                connection.execute(
                    insertVariantSql(true),
                    params_from_iter(insertVariantParams(&variant, true)),
                )?;
                Ok(variant.variantId)
            }
        })
    }

    pub fn insertVariants(&self, variants: Vec<MessageVariantEntity>) -> Result<(), SqliteStoreError> {
        self.store.transaction(|transaction| {
            for variant in variants {
                if variant.variantId == 0 {
                    transaction.execute(
                        insertVariantSql(false),
                        params_from_iter(insertVariantParams(&variant, false)),
                    )?;
                } else {
                    transaction.execute(
                        insertVariantSql(true),
                        params_from_iter(insertVariantParams(&variant, true)),
                    )?;
                }
            }
            Ok(())
        })
    }

    pub fn copyVariantsToChat(
        &self,
        sourceChatId: &str,
        targetChatId: &str,
        upToTimestampInclusive: Option<i64>,
    ) -> Result<(), SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.execute(
                r#"
                INSERT INTO message_variants (
                    chatId, messageTimestamp, variantIndex, content, roleName, provider,
                    modelName, inputTokens, outputTokens, cachedInputTokens, sentAt,
                    outputDurationMs, waitDurationMs, completedAt
                )
                SELECT
                    ?2, messageTimestamp, variantIndex, content, roleName, provider,
                    modelName, inputTokens, outputTokens, cachedInputTokens, sentAt,
                    outputDurationMs, waitDurationMs, completedAt
                FROM message_variants
                WHERE chatId = ?1 AND (?3 IS NULL OR messageTimestamp <= ?3)
                "#,
                params![sourceChatId, targetChatId, upToTimestampInclusive],
            )?;
            Ok(())
        })
    }

    pub fn updateVariant(&self, variant: MessageVariantEntity) -> Result<(), SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.execute(
                r#"
                UPDATE message_variants
                SET chatId = ?2, messageTimestamp = ?3, variantIndex = ?4, content = ?5,
                    roleName = ?6, provider = ?7, modelName = ?8, inputTokens = ?9,
                    outputTokens = ?10, cachedInputTokens = ?11, sentAt = ?12,
                    outputDurationMs = ?13, waitDurationMs = ?14, completedAt = ?15
                WHERE variantId = ?1
                "#,
                params![
                    variant.variantId,
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
        })
    }

    pub fn deleteVariant(
        &self,
        chatId: &str,
        messageTimestamp: i64,
        variantIndex: i32,
    ) -> Result<(), SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.execute(
                "DELETE FROM message_variants WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
                params![chatId, messageTimestamp, variantIndex],
            )?;
            Ok(())
        })
    }

    pub fn deleteVariantsForMessage(
        &self,
        chatId: &str,
        messageTimestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.execute(
                "DELETE FROM message_variants WHERE chatId = ?1 AND messageTimestamp = ?2",
                params![chatId, messageTimestamp],
            )?;
            Ok(())
        })
    }

    pub fn deleteVariantsFrom(
        &self,
        chatId: &str,
        messageTimestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.execute(
                "DELETE FROM message_variants WHERE chatId = ?1 AND messageTimestamp >= ?2",
                params![chatId, messageTimestamp],
            )?;
            Ok(())
        })
    }

    pub fn deleteAllVariantsForChat(&self, chatId: &str) -> Result<(), SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.execute("DELETE FROM message_variants WHERE chatId = ?1", params![chatId])?;
            Ok(())
        })
    }
}

fn mapMessageVariantEntity(row: &Row<'_>) -> Result<MessageVariantEntity, rusqlite::Error> {
    Ok(MessageVariantEntity {
        variantId: row.get(0)?,
        chatId: row.get(1)?,
        messageTimestamp: row.get(2)?,
        variantIndex: row.get(3)?,
        content: row.get(4)?,
        roleName: row.get(5)?,
        provider: row.get(6)?,
        modelName: row.get(7)?,
        inputTokens: row.get(8)?,
        outputTokens: row.get(9)?,
        cachedInputTokens: row.get(10)?,
        sentAt: row.get(11)?,
        outputDurationMs: row.get(12)?,
        waitDurationMs: row.get(13)?,
        completedAt: row.get(14)?,
    })
}

fn insertVariantSql(withVariantId: bool) -> &'static str {
    if withVariantId {
        r#"
        INSERT OR REPLACE INTO message_variants (
            variantId, chatId, messageTimestamp, variantIndex, content, roleName,
            provider, modelName, inputTokens, outputTokens, cachedInputTokens,
            sentAt, outputDurationMs, waitDurationMs, completedAt
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
        "#
    } else {
        r#"
        INSERT OR REPLACE INTO message_variants (
            chatId, messageTimestamp, variantIndex, content, roleName,
            provider, modelName, inputTokens, outputTokens, cachedInputTokens,
            sentAt, outputDurationMs, waitDurationMs, completedAt
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        "#
    }
}

fn insertVariantParams<'a>(
    variant: &'a MessageVariantEntity,
    withVariantId: bool,
) -> Vec<&'a dyn rusqlite::ToSql> {
    if withVariantId {
        vec![
            &variant.variantId,
            &variant.chatId,
            &variant.messageTimestamp,
            &variant.variantIndex,
            &variant.content,
            &variant.roleName,
            &variant.provider,
            &variant.modelName,
            &variant.inputTokens,
            &variant.outputTokens,
            &variant.cachedInputTokens,
            &variant.sentAt,
            &variant.outputDurationMs,
            &variant.waitDurationMs,
            &variant.completedAt,
        ]
    } else {
        vec![
            &variant.chatId,
            &variant.messageTimestamp,
            &variant.variantIndex,
            &variant.content,
            &variant.roleName,
            &variant.provider,
            &variant.modelName,
            &variant.inputTokens,
            &variant.outputTokens,
            &variant.cachedInputTokens,
            &variant.sentAt,
            &variant.outputDurationMs,
            &variant.waitDurationMs,
            &variant.completedAt,
        ]
    }
}
