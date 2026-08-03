use crate::sqliteParams;
use crate::SqliteStore::{SqliteRow, SqliteRowGet, SqliteStore, SqliteStoreError};

use operit_model::MessagePart::MessagePartKind;
use operit_model::MessagePartEntity::MessagePartEntity;

const SELECT_PART_COLUMNS: &str = r#"
    SELECT chatId, messageTimestamp, variantIndex, partId, sequence, kind, content,
        toolCallId, toolName, attributesJson
    FROM message_parts
"#;

/// Provides persistence operations for ordered structured message parts.
#[derive(Clone)]
pub struct MessagePartDao {
    store: SqliteStore,
}

impl MessagePartDao {
    /// Creates a part DAO bound to one SQLite store.
    pub fn new(store: SqliteStore) -> Self {
        Self { store }
    }

    /// Loads all parts for one message revision in display order.
    pub fn getPartsForMessage(
        &self,
        chatId: &str,
        messageTimestamp: i64,
        variantIndex: i32,
    ) -> Result<Vec<MessagePartEntity>, SqliteStoreError> {
        self.selectParts(
            &format!(
                "{SELECT_PART_COLUMNS}
                WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3
                ORDER BY sequence ASC"
            ),
            sqliteParams![chatId, messageTimestamp, variantIndex],
        )
    }

    /// Loads all parts for a chat so repository hydration can select message revisions.
    pub fn getPartsForChat(
        &self,
        chatId: &str,
    ) -> Result<Vec<MessagePartEntity>, SqliteStoreError> {
        self.selectParts(
            &format!(
                "{SELECT_PART_COLUMNS}
                WHERE chatId = ?1
                ORDER BY messageTimestamp ASC, variantIndex ASC, sequence ASC"
            ),
            sqliteParams![chatId],
        )
    }

    /// Replaces the complete ordered part set for one message revision atomically.
    pub fn replaceParts(
        &self,
        chatId: &str,
        messageTimestamp: i64,
        variantIndex: i32,
        parts: Vec<MessagePartEntity>,
    ) -> Result<(), SqliteStoreError> {
        self.store.transaction(|transaction| {
            transaction.execute(
                "DELETE FROM message_parts WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
                sqliteParams![chatId, messageTimestamp, variantIndex],
            )?;
            for part in parts {
                transaction.execute(
                    r#"
                    INSERT INTO message_parts (
                        chatId, messageTimestamp, variantIndex, partId, sequence, kind, content,
                        toolCallId, toolName, attributesJson
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                    "#,
                    partParams(&part)?,
                )?;
            }
            Ok(())
        })
    }

    /// Copies all selected revision parts into a different chat identifier.
    pub fn copyPartsToChat(
        &self,
        sourceChatId: &str,
        targetChatId: &str,
        upToTimestampInclusive: Option<i64>,
    ) -> Result<(), SqliteStoreError> {
        self.store.execute(
            r#"
            INSERT INTO message_parts (
                chatId, messageTimestamp, variantIndex, partId, sequence, kind, content,
                toolCallId, toolName, attributesJson
            )
            SELECT
                ?2, messageTimestamp, variantIndex, partId, sequence, kind, content,
                toolCallId, toolName, attributesJson
            FROM message_parts
            WHERE chatId = ?1 AND (?3 IS NULL OR messageTimestamp <= ?3)
            "#,
            sqliteParams![sourceChatId, targetChatId, upToTimestampInclusive],
        )?;
        Ok(())
    }

    /// Deletes all parts belonging to one message revision.
    pub fn deletePartsForMessage(
        &self,
        chatId: &str,
        messageTimestamp: i64,
        variantIndex: i32,
    ) -> Result<(), SqliteStoreError> {
        self.store.execute(
            "DELETE FROM message_parts WHERE chatId = ?1 AND messageTimestamp = ?2 AND variantIndex = ?3",
            sqliteParams![chatId, messageTimestamp, variantIndex],
        )?;
        Ok(())
    }

    /// Deletes all parts belonging to all revisions of one message.
    pub fn deletePartsForMessageTimestamp(
        &self,
        chatId: &str,
        messageTimestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.store.execute(
            "DELETE FROM message_parts WHERE chatId = ?1 AND messageTimestamp = ?2",
            sqliteParams![chatId, messageTimestamp],
        )?;
        Ok(())
    }

    /// Deletes parts from one timestamp through the end of a chat.
    pub fn deletePartsFrom(
        &self,
        chatId: &str,
        messageTimestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.store.execute(
            "DELETE FROM message_parts WHERE chatId = ?1 AND messageTimestamp >= ?2",
            sqliteParams![chatId, messageTimestamp],
        )?;
        Ok(())
    }

    /// Deletes every structured part associated with a chat.
    pub fn deleteAllPartsForChat(&self, chatId: &str) -> Result<(), SqliteStoreError> {
        self.store.execute(
            "DELETE FROM message_parts WHERE chatId = ?1",
            sqliteParams![chatId],
        )?;
        Ok(())
    }

    /// Loads part rows with the caller's ordering and filtering statement.
    fn selectParts(
        &self,
        sql: &str,
        params: Vec<operit_host_api::SqliteValue>,
    ) -> Result<Vec<MessagePartEntity>, SqliteStoreError> {
        self.store
            .queryRows(sql, params)?
            .into_iter()
            .map(|row| mapMessagePartEntity(&row))
            .collect()
    }
}

/// Converts one SQLite row into a typed message part entity.
fn mapMessagePartEntity(row: &SqliteRow) -> Result<MessagePartEntity, SqliteStoreError> {
    let attributesJson: String = row.get("attributesJson")?;
    let kind: String = row.get("kind")?;
    Ok(MessagePartEntity {
        chatId: row.get("chatId")?,
        messageTimestamp: row.get("messageTimestamp")?,
        variantIndex: row.get("variantIndex")?,
        partId: row.get("partId")?,
        sequence: row.get("sequence")?,
        kind: partKindFromLabel(&kind)?,
        content: row.get("content")?,
        toolCallId: row.get("toolCallId")?,
        toolName: row.get("toolName")?,
        attributes: serde_json::from_str(&attributesJson).map_err(|error| {
            SqliteStoreError::Message(format!("invalid message-part attributes JSON: {error}"))
        })?,
    })
}

/// Serializes a message part entity for one INSERT statement.
fn partParams(
    part: &MessagePartEntity,
) -> Result<Vec<operit_host_api::SqliteValue>, SqliteStoreError> {
    let attributesJson = serde_json::to_string(&part.attributes).map_err(|error| {
        SqliteStoreError::Message(format!("message-part attributes cannot serialize: {error}"))
    })?;
    Ok(sqliteParams![
        part.chatId,
        part.messageTimestamp,
        part.variantIndex,
        part.partId,
        part.sequence,
        partKindLabel(&part.kind),
        part.content,
        part.toolCallId,
        part.toolName,
        attributesJson,
    ])
}

/// Converts a message-part kind into its database label.
fn partKindLabel(kind: &MessagePartKind) -> &'static str {
    match kind {
        MessagePartKind::Markdown => "markdown",
        MessagePartKind::Thinking => "thinking",
        MessagePartKind::ToolCall => "tool_call",
        MessagePartKind::ToolResult => "tool_result",
        MessagePartKind::Status => "status",
    }
}

/// Converts a database label into a message-part kind.
fn partKindFromLabel(value: &str) -> Result<MessagePartKind, SqliteStoreError> {
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
