use crate::sqliteParams;
use crate::PluginExtensions::{decodePluginExtensions, encodePluginExtensions};
use crate::SqliteStore::{
    toSqliteValue, SqliteRow, SqliteRowGet, SqliteStore, SqliteStoreError, SqliteValue,
};

use operit_model::MessageVariantEntity::MessageVariantEntity;

const SELECT_VARIANT_COLUMNS: &str = r#"
    SELECT variantId, chatId, messageTimestamp, variantIndex, roleName,
        provider, modelName, inputTokens, outputTokens, cachedInputTokens,
        sentAt, outputDurationMs, waitDurationMs, completedAt, pluginExtensions
    FROM message_variants
"#;

#[derive(Clone)]
pub struct MessageVariantDao {
    store: SqliteStore,
}

impl MessageVariantDao {
    /// Processes the exact persisted variant record with explicit storage errors.
    pub fn new(store: SqliteStore) -> Self {
        Self { store }
    }

    /// Processes the exact persisted variant record with explicit storage errors.
    pub fn getVariantsForChat(
        &self,
        chatId: &str,
    ) -> Result<Vec<MessageVariantEntity>, SqliteStoreError> {
        self.selectVariants(
            &format!(
                "{SELECT_VARIANT_COLUMNS}
                WHERE chatId = ?1
                ORDER BY messageTimestamp ASC, variantIndex ASC"
            ),
            sqliteParams![chatId],
        )
    }

    /// Processes the exact persisted variant record with explicit storage errors.
    pub fn getVariantsForMessages(
        &self,
        chatId: &str,
        messageTimestamps: Vec<i64>,
    ) -> Result<Vec<MessageVariantEntity>, SqliteStoreError> {
        let placeholders = messageTimestamps
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "{SELECT_VARIANT_COLUMNS}
            WHERE chatId = ? AND messageTimestamp IN ({placeholders})
            ORDER BY messageTimestamp ASC, variantIndex ASC"
        );
        let mut params = sqliteParams![chatId];
        for timestamp in &messageTimestamps {
            params.push(toSqliteValue(timestamp));
        }
        self.selectVariants(&sql, params)
    }

    /// Processes the exact persisted variant record with explicit storage errors.
    pub fn getVariantsForMessage(
        &self,
        chatId: &str,
        messageTimestamp: i64,
    ) -> Result<Vec<MessageVariantEntity>, SqliteStoreError> {
        self.selectVariants(
            &format!(
                "{SELECT_VARIANT_COLUMNS}
                WHERE chatId = ?1 AND messageTimestamp = ?2
                ORDER BY variantIndex ASC"
            ),
            sqliteParams![chatId, messageTimestamp],
        )
    }

    /// Processes the exact persisted variant record with explicit storage errors.
    pub fn getVariantForMessage(
        &self,
        chatId: &str,
        messageTimestamp: i64,
        variantIndex: i32,
    ) -> Result<Option<MessageVariantEntity>, SqliteStoreError> {
        self.store
            .queryOne(
                &format!(
                    "{SELECT_VARIANT_COLUMNS}
                    WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3
                    LIMIT 1"
                ),
                sqliteParams![chatId, messageTimestamp, variantIndex],
            )?
            .map(|row| mapMessageVariantEntity(&row))
            .transpose()
    }

    /// Processes the exact persisted variant record with explicit storage errors.
    pub fn insertVariant(&self, variant: MessageVariantEntity) -> Result<i64, SqliteStoreError> {
        if variant.variantId == 0 {
            self.store.execute(
                insertVariantSql(false),
                insertVariantParams(&variant, false)?,
            )?;
            self.store
                .queryScalar("SELECT last_insert_rowid()", sqliteParams![])
        } else {
            self.store
                .execute(insertVariantSql(true), insertVariantParams(&variant, true)?)?;
            Ok(variant.variantId)
        }
    }

    /// Processes the exact persisted variant record with explicit storage errors.
    pub fn insertVariants(
        &self,
        variants: Vec<MessageVariantEntity>,
    ) -> Result<(), SqliteStoreError> {
        self.store.transaction(|transaction| {
            for variant in variants {
                if variant.variantId == 0 {
                    transaction.execute(
                        insertVariantSql(false),
                        insertVariantParams(&variant, false)?,
                    )?;
                } else {
                    transaction
                        .execute(insertVariantSql(true), insertVariantParams(&variant, true)?)?;
                }
            }
            Ok(())
        })
    }

    /// Processes the exact persisted variant record with explicit storage errors.
    pub fn copyVariantsToChat(
        &self,
        sourceChatId: &str,
        targetChatId: &str,
        upToTimestampInclusive: Option<i64>,
    ) -> Result<(), SqliteStoreError> {
        self.store.execute(
            r#"
                INSERT INTO message_variants (
                    chatId, messageTimestamp, variantIndex, roleName, provider,
                    modelName, inputTokens, outputTokens, cachedInputTokens, sentAt,
                    outputDurationMs, waitDurationMs, completedAt, pluginExtensions
                )
                SELECT
                    ?2, messageTimestamp, variantIndex, roleName, provider,
                    modelName, inputTokens, outputTokens, cachedInputTokens, sentAt,
                    outputDurationMs, waitDurationMs, completedAt, pluginExtensions
                FROM message_variants
                WHERE chatId = ?1 AND (?3 IS NULL OR messageTimestamp <= ?3)
                "#,
            sqliteParams![sourceChatId, targetChatId, upToTimestampInclusive],
        )?;
        Ok(())
    }

    /// Processes the exact persisted variant record with explicit storage errors.
    pub fn updateVariant(&self, variant: MessageVariantEntity) -> Result<(), SqliteStoreError> {
        self.store.execute(
            r#"
                UPDATE message_variants
                SET chatId = ?2, messageTimestamp = ?3, variantIndex = ?4,
                    roleName = ?5, provider = ?6, modelName = ?7, inputTokens = ?8,
                    outputTokens = ?9, cachedInputTokens = ?10, sentAt = ?11,
                    outputDurationMs = ?12, waitDurationMs = ?13, completedAt = ?14
                WHERE variantId = ?1
                "#,
            sqliteParams![
                variant.variantId,
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

    /// Processes the exact persisted variant record with explicit storage errors.
    pub fn deleteVariant(
        &self,
        chatId: &str,
        messageTimestamp: i64,
        variantIndex: i32,
    ) -> Result<(), SqliteStoreError> {
        self.store.execute(
            "DELETE FROM message_variants WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
            sqliteParams![chatId, messageTimestamp, variantIndex],
        )?;
        Ok(())
    }

    /// Processes the exact persisted variant record with explicit storage errors.
    pub fn deleteVariantsForMessage(
        &self,
        chatId: &str,
        messageTimestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.store.execute(
            "DELETE FROM message_variants WHERE chatId = ?1 AND messageTimestamp = ?2",
            sqliteParams![chatId, messageTimestamp],
        )?;
        Ok(())
    }

    /// Processes the exact persisted variant record with explicit storage errors.
    pub fn deleteVariantsFrom(
        &self,
        chatId: &str,
        messageTimestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.store.execute(
            "DELETE FROM message_variants WHERE chatId = ?1 AND messageTimestamp >= ?2",
            sqliteParams![chatId, messageTimestamp],
        )?;
        Ok(())
    }

    /// Processes the exact persisted variant record with explicit storage errors.
    pub fn deleteAllVariantsForChat(&self, chatId: &str) -> Result<(), SqliteStoreError> {
        self.store.execute(
            "DELETE FROM message_variants WHERE chatId = ?1",
            sqliteParams![chatId],
        )?;
        Ok(())
    }

    /// Processes the exact persisted variant record with explicit storage errors.
    fn selectVariants(
        &self,
        sql: &str,
        params: Vec<SqliteValue>,
    ) -> Result<Vec<MessageVariantEntity>, SqliteStoreError> {
        self.store
            .queryRows(sql, params)?
            .into_iter()
            .map(|row| mapMessageVariantEntity(&row))
            .collect()
    }
}

/// Processes the exact persisted variant record with explicit storage errors.
pub(crate) fn mapMessageVariantEntity(
    row: &SqliteRow,
) -> Result<MessageVariantEntity, SqliteStoreError> {
    Ok(MessageVariantEntity {
        variantId: row.get(0)?,
        chatId: row.get(1)?,
        messageTimestamp: row.get(2)?,
        variantIndex: row.get(3)?,
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
        pluginExtensions: decodePluginExtensions(&row.get::<_, String>(14)?)?,
    })
}

/// Processes the exact persisted variant record with explicit storage errors.
pub(crate) fn insertVariantSql(withVariantId: bool) -> &'static str {
    if withVariantId {
        r#"
        INSERT INTO message_variants (
            variantId, chatId, messageTimestamp, variantIndex, roleName,
            provider, modelName, inputTokens, outputTokens, cachedInputTokens,
            sentAt, outputDurationMs, waitDurationMs, completedAt, pluginExtensions
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
        ON CONFLICT(chatId, messageTimestamp, variantIndex) DO UPDATE SET roleName = excluded.roleName, provider = excluded.provider, modelName = excluded.modelName, inputTokens = excluded.inputTokens, outputTokens = excluded.outputTokens, cachedInputTokens = excluded.cachedInputTokens, sentAt = excluded.sentAt, outputDurationMs = excluded.outputDurationMs, waitDurationMs = excluded.waitDurationMs, completedAt = excluded.completedAt
        "#
    } else {
        r#"
        INSERT INTO message_variants (
            chatId, messageTimestamp, variantIndex, roleName,
            provider, modelName, inputTokens, outputTokens, cachedInputTokens,
            sentAt, outputDurationMs, waitDurationMs, completedAt, pluginExtensions
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        ON CONFLICT(chatId, messageTimestamp, variantIndex) DO UPDATE SET roleName = excluded.roleName, provider = excluded.provider, modelName = excluded.modelName, inputTokens = excluded.inputTokens, outputTokens = excluded.outputTokens, cachedInputTokens = excluded.cachedInputTokens, sentAt = excluded.sentAt, outputDurationMs = excluded.outputDurationMs, waitDurationMs = excluded.waitDurationMs, completedAt = excluded.completedAt
        "#
    }
}

/// Processes the exact persisted variant record with explicit storage errors.
pub(crate) fn insertVariantParams(
    variant: &MessageVariantEntity,
    withVariantId: bool,
) -> Result<Vec<SqliteValue>, SqliteStoreError> {
    if withVariantId {
        Ok(sqliteParams![
            variant.variantId,
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
            encodePluginExtensions(&variant.pluginExtensions)?,
        ])
    } else {
        Ok(sqliteParams![
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
            encodePluginExtensions(&variant.pluginExtensions)?,
        ])
    }
}
