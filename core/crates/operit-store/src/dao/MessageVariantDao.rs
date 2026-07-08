use crate::sqliteParams;
use crate::SqliteStore::{
    toSqliteValue, SqliteRow, SqliteRowGet, SqliteStore, SqliteStoreError, SqliteValue,
};

use operit_model::MessageVariantEntity::MessageVariantEntity;

const SELECT_VARIANT_COLUMNS: &str = r#"
    SELECT variantId, chatId, messageTimestamp, variantIndex, content, roleName,
        provider, modelName, inputTokens, outputTokens, cachedInputTokens,
        sentAt, outputDurationMs, waitDurationMs, completedAt
    FROM message_variants
"#;

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
        self.selectVariants(
            &format!(
                "{SELECT_VARIANT_COLUMNS}
                WHERE chatId = ?1
                ORDER BY messageTimestamp ASC, variantIndex ASC"
            ),
            sqliteParams![chatId],
        )
    }

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

    pub fn insertVariant(&self, variant: MessageVariantEntity) -> Result<i64, SqliteStoreError> {
        if variant.variantId == 0 {
            self.store.execute(
                insertVariantSql(false),
                insertVariantParams(&variant, false),
            )?;
            self.store
                .queryScalar("SELECT last_insert_rowid()", sqliteParams![])
        } else {
            self.store
                .execute(insertVariantSql(true), insertVariantParams(&variant, true))?;
            Ok(variant.variantId)
        }
    }

    pub fn insertVariants(
        &self,
        variants: Vec<MessageVariantEntity>,
    ) -> Result<(), SqliteStoreError> {
        self.store.transaction(|transaction| {
            for variant in variants {
                if variant.variantId == 0 {
                    transaction.execute(
                        insertVariantSql(false),
                        insertVariantParams(&variant, false),
                    )?;
                } else {
                    transaction
                        .execute(insertVariantSql(true), insertVariantParams(&variant, true))?;
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
        self.store.execute(
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
            sqliteParams![sourceChatId, targetChatId, upToTimestampInclusive],
        )?;
        Ok(())
    }

    pub fn updateVariant(&self, variant: MessageVariantEntity) -> Result<(), SqliteStoreError> {
        self.store.execute(
            r#"
                UPDATE message_variants
                SET chatId = ?2, messageTimestamp = ?3, variantIndex = ?4, content = ?5,
                    roleName = ?6, provider = ?7, modelName = ?8, inputTokens = ?9,
                    outputTokens = ?10, cachedInputTokens = ?11, sentAt = ?12,
                    outputDurationMs = ?13, waitDurationMs = ?14, completedAt = ?15
                WHERE variantId = ?1
                "#,
            sqliteParams![
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
    }

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

    pub fn deleteAllVariantsForChat(&self, chatId: &str) -> Result<(), SqliteStoreError> {
        self.store.execute(
            "DELETE FROM message_variants WHERE chatId = ?1",
            sqliteParams![chatId],
        )?;
        Ok(())
    }

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

fn mapMessageVariantEntity(row: &SqliteRow) -> Result<MessageVariantEntity, SqliteStoreError> {
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

fn insertVariantParams(variant: &MessageVariantEntity, withVariantId: bool) -> Vec<SqliteValue> {
    if withVariantId {
        sqliteParams![
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
        ]
    } else {
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
        ]
    }
}
