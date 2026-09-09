#[allow(non_snake_case)]
/// Builds a chat archive through one explicit Operit1-to-Operit2 database bridge.
fn buildChatArchiveFromOperit1ToOperit2DatabaseBridge<F>(
    bridge: Operit1ToOperit2ChatArchiveBridge,
    connection: &mut dyn RuntimeSqliteConnection,
    fileImportPlan: &SnapshotFileImportPlan,
    onMessageParsed: &mut F,
) -> Result<OperitChatArchive, String>
where
    F: FnMut(usize),
{
    match bridge {
        Operit1ToOperit2ChatArchiveBridge::Operit1RoomV10ToOperit2SqliteV27 => {
            buildChatArchiveFromOperit1RoomV10Database(connection, fileImportPlan, onMessageParsed)
        }
        Operit1ToOperit2ChatArchiveBridge::Operit1RoomV20ToOperit2SqliteV27
        | Operit1ToOperit2ChatArchiveBridge::Operit1RoomV21ToOperit2SqliteV27 => {
            buildChatArchiveFromOperit1RoomV20Database(connection, fileImportPlan, onMessageParsed)
        }
    }
}

/// Builds a chat archive from the original Operit1 Room schema version 10.
fn buildChatArchiveFromOperit1RoomV10Database<F>(
    connection: &mut dyn RuntimeSqliteConnection,
    fileImportPlan: &SnapshotFileImportPlan,
    onMessageParsed: &mut F,
) -> Result<OperitChatArchive, String>
where
    F: FnMut(usize),
{
    requireOperit1ChatTables(connection)?;
    let rows = connection
        .query(
            r#"
            SELECT id, title, createdAt, updatedAt, inputTokens, outputTokens,
                currentWindowSize, "group", displayOrder, workspace, parentChatId,
                characterCardName, locked
            FROM chats
            ORDER BY displayOrder ASC, updatedAt DESC
            "#,
            Vec::new(),
        )
        .map_err(|error| error.to_string())?;
    let chatRows = rows
        .iter()
        .map(|row| {
            Ok(Operit1ChatRow {
                id: sqliteRowString(row, 0, "chats.id")?,
                title: sqliteRowString(row, 1, "chats.title")?,
                createdAt: sqliteRowI64(row, 2, "chats.createdAt")?,
                updatedAt: sqliteRowI64(row, 3, "chats.updatedAt")?,
                inputTokens: sqliteRowI64(row, 4, "chats.inputTokens")?,
                outputTokens: sqliteRowI64(row, 5, "chats.outputTokens")?,
                currentWindowSize: sqliteRowI64(row, 6, "chats.currentWindowSize")?,
                group: sqliteRowOptionalString(row, 7, "chats.group")?,
                displayOrder: sqliteRowI64(row, 8, "chats.displayOrder")?,
                workspace: sqliteRowOptionalString(row, 9, "chats.workspace")?,
                parentChatId: sqliteRowOptionalString(row, 10, "chats.parentChatId")?,
                characterCardName: sqliteRowOptionalString(row, 11, "chats.characterCardName")?,
                characterGroupId: None,
                locked: sqliteRowI64(row, 12, "chats.locked")? != 0,
                pinned: false,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut chats = Vec::new();
    for (chatIndex, chat) in chatRows.into_iter().enumerate() {
        let mut onMessageForChat = || onMessageParsed(chatIndex + 1);
        let messages = readOperit1RoomV10Messages(connection, &chat.id, &mut onMessageForChat)?;
        chats.push(OperitArchivedChat {
            id: chat.id,
            title: chat.title,
            messages,
            createdAt: epochMillisToLocalDateTimeString(chat.createdAt)?,
            updatedAt: epochMillisToLocalDateTimeString(chat.updatedAt)?,
            inputTokens: chat.inputTokens,
            outputTokens: chat.outputTokens,
            currentWindowSize: chat.currentWindowSize,
            group: chat.group,
            displayOrder: chat.displayOrder,
            workspace: fileImportPlan.rewriteChatWorkspace(chat.workspace)?,
            parentChatId: chat.parentChatId,
            characterCardName: chat.characterCardName,
            characterGroupId: None,
            locked: chat.locked,
            pinned: false,
        });
    }
    Ok(OperitChatArchive {
        archiveType: ARCHIVE_TYPE.to_string(),
        formatVersion: CURRENT_FORMAT_VERSION,
        exportedAt: currentTimeMillis(),
        chats,
    })
}

/// Builds a chat archive from the Operit1 Room schemas 20 and 21.
fn buildChatArchiveFromOperit1RoomV20Database<F>(
    connection: &mut dyn RuntimeSqliteConnection,
    fileImportPlan: &SnapshotFileImportPlan,
    onMessageParsed: &mut F,
) -> Result<OperitChatArchive, String>
where
    F: FnMut(usize),
{
    requireOperit1ChatTables(connection)?;
    let rows = connection
        .query(
            r#"
            SELECT id, title, createdAt, updatedAt, inputTokens, outputTokens,
                currentWindowSize, "group", displayOrder, workspace, parentChatId,
                characterCardName, characterGroupId, locked, pinned
            FROM chats
            ORDER BY displayOrder ASC, updatedAt DESC
            "#,
            Vec::new(),
        )
        .map_err(|error| error.to_string())?;
    let chatRows = rows
        .iter()
        .map(|row| {
            Ok(Operit1ChatRow {
                id: sqliteRowString(row, 0, "chats.id")?,
                title: sqliteRowString(row, 1, "chats.title")?,
                createdAt: sqliteRowI64(row, 2, "chats.createdAt")?,
                updatedAt: sqliteRowI64(row, 3, "chats.updatedAt")?,
                inputTokens: sqliteRowI64(row, 4, "chats.inputTokens")?,
                outputTokens: sqliteRowI64(row, 5, "chats.outputTokens")?,
                currentWindowSize: sqliteRowI64(row, 6, "chats.currentWindowSize")?,
                group: sqliteRowOptionalString(row, 7, "chats.group")?,
                displayOrder: sqliteRowI64(row, 8, "chats.displayOrder")?,
                workspace: sqliteRowOptionalString(row, 9, "chats.workspace")?,
                parentChatId: sqliteRowOptionalString(row, 10, "chats.parentChatId")?,
                characterCardName: sqliteRowOptionalString(row, 11, "chats.characterCardName")?,
                characterGroupId: sqliteRowOptionalString(row, 12, "chats.characterGroupId")?,
                locked: sqliteRowI64(row, 13, "chats.locked")? != 0,
                pinned: sqliteRowI64(row, 14, "chats.pinned")? != 0,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut chats = Vec::new();
    for (chatIndex, chat) in chatRows.into_iter().enumerate() {
        let mut onMessageForChat = || onMessageParsed(chatIndex + 1);
        let messages = readOperit1RoomV20Messages(connection, &chat.id, &mut onMessageForChat)?;
        chats.push(OperitArchivedChat {
            id: chat.id,
            title: chat.title,
            messages,
            createdAt: epochMillisToLocalDateTimeString(chat.createdAt)?,
            updatedAt: epochMillisToLocalDateTimeString(chat.updatedAt)?,
            inputTokens: chat.inputTokens,
            outputTokens: chat.outputTokens,
            currentWindowSize: chat.currentWindowSize,
            group: chat.group,
            displayOrder: chat.displayOrder,
            workspace: fileImportPlan.rewriteChatWorkspace(chat.workspace)?,
            parentChatId: chat.parentChatId,
            characterCardName: chat.characterCardName,
            characterGroupId: chat.characterGroupId,
            locked: chat.locked,
            pinned: chat.pinned,
        });
    }
    Ok(OperitChatArchive {
        archiveType: ARCHIVE_TYPE.to_string(),
        formatVersion: CURRENT_FORMAT_VERSION,
        exportedAt: currentTimeMillis(),
        chats,
    })
}

#[derive(Clone, Debug)]
#[allow(non_snake_case)]
struct Operit1ChatRow {
    id: String,
    title: String,
    createdAt: i64,
    updatedAt: i64,
    inputTokens: i64,
    outputTokens: i64,
    currentWindowSize: i64,
    group: Option<String>,
    displayOrder: i64,
    workspace: Option<String>,
    parentChatId: Option<String>,
    characterCardName: Option<String>,
    characterGroupId: Option<String>,
    locked: bool,
    pinned: bool,
}

#[allow(non_snake_case)]
/// Reads archived messages for one Operit1 chat row.
fn readOperit1RoomV10Messages<F>(
    connection: &mut dyn RuntimeSqliteConnection,
    chatId: &str,
    onMessageParsed: &mut F,
) -> Result<Vec<OperitArchivedMessage>, String>
where
    F: FnMut(),
{
    let rows = connection
        .query(
            r#"
            SELECT sender, content, timestamp, orderIndex, roleName, provider, modelName
            FROM messages
            WHERE chatId = ?1
            ORDER BY orderIndex ASC, timestamp ASC
            "#,
            vec![SqliteValue::Text(chatId.to_string())],
        )
        .map_err(|error| error.to_string())?;
    let mut messages = Vec::with_capacity(rows.len());
    for row in &rows {
        let timestamp = sqliteRowI64(row, 2, "messages.timestamp")?;
        let sender = sqliteRowString(row, 0, "messages.sender")?;
        let content = sqliteRowString(row, 1, "messages.content")?;
        let parts = if sender == "ai" {
            MessagePartCodec::parseAssistantMarkup(&content)
                .map_err(|error| format!("Operit1 assistant message markup is invalid: {error}"))?
        } else {
            vec![MessagePart::markdown("part-0".to_string(), 0, content)]
        };
        messages.push(OperitArchivedMessage {
            baseMessage: ChatMessage {
                sender,
                parts,
                timestamp,
                roleName: sqliteRowString(row, 4, "messages.roleName")?,
                selectedVariantIndex: 0,
                variantCount: 1,
                provider: sqliteRowString(row, 5, "messages.provider")?,
                modelName: sqliteRowString(row, 6, "messages.modelName")?,
                inputTokens: 0,
                outputTokens: 0,
                cachedInputTokens: 0,
                sentAt: 0,
                outputDurationMs: 0,
                waitDurationMs: 0,
                completedAt: 0,
                completedExecutionGeneration: 0,
                displayMode: ChatMessageDisplayMode::NORMAL,
                isFavorite: false,
                isVariantPreview: false,
                contentStream: None,
            },
            variants: Vec::new(),
        });
        onMessageParsed();
    }
    Ok(messages)
}

/// Reads complete message and revision data from Operit1 Room schemas 20 and 21.
fn readOperit1RoomV20Messages<F>(
    connection: &mut dyn RuntimeSqliteConnection,
    chatId: &str,
    onMessageParsed: &mut F,
) -> Result<Vec<OperitArchivedMessage>, String>
where
    F: FnMut(),
{
    let rows = connection
        .query(
            r#"
            SELECT sender, content, timestamp, roleName, selectedVariantIndex, provider, modelName,
                inputTokens, outputTokens, cachedInputTokens, sentAt, outputDurationMs,
                waitDurationMs, completedAt, displayMode, isFavorite
            FROM messages
            WHERE chatId = ?1
            ORDER BY orderIndex ASC, timestamp ASC
            "#,
            vec![SqliteValue::Text(chatId.to_string())],
        )
        .map_err(|error| error.to_string())?;
    let mut messages = Vec::with_capacity(rows.len());
    for row in &rows {
        let sender = sqliteRowString(row, 0, "messages.sender")?;
        let timestamp = sqliteRowI64(row, 2, "messages.timestamp")?;
        let variants = readOperit1RoomV20MessageVariants(connection, chatId, timestamp, &sender)?;
        messages.push(OperitArchivedMessage {
            baseMessage: ChatMessage {
                sender: sender.clone(),
                parts: parseOperit1MessageParts(
                    &sender,
                    sqliteRowString(row, 1, "messages.content")?,
                )?,
                timestamp,
                roleName: sqliteRowString(row, 3, "messages.roleName")?,
                selectedVariantIndex: sqliteRowI32(row, 4, "messages.selectedVariantIndex")?,
                variantCount: variants.len() as i32 + 1,
                provider: sqliteRowString(row, 5, "messages.provider")?,
                modelName: sqliteRowString(row, 6, "messages.modelName")?,
                inputTokens: sqliteRowI64(row, 7, "messages.inputTokens")?,
                outputTokens: sqliteRowI64(row, 8, "messages.outputTokens")?,
                cachedInputTokens: sqliteRowI64(row, 9, "messages.cachedInputTokens")?,
                sentAt: sqliteRowI64(row, 10, "messages.sentAt")?,
                outputDurationMs: sqliteRowI64(row, 11, "messages.outputDurationMs")?,
                waitDurationMs: sqliteRowI64(row, 12, "messages.waitDurationMs")?,
                completedAt: sqliteRowI64(row, 13, "messages.completedAt")?,
                completedExecutionGeneration: 0,
                displayMode: parseOperit1ChatMessageDisplayMode(&sqliteRowString(
                    row,
                    14,
                    "messages.displayMode",
                )?)?,
                isFavorite: sqliteRowI64(row, 15, "messages.isFavorite")? != 0,
                isVariantPreview: false,
                contentStream: None,
            },
            variants,
        });
        onMessageParsed();
    }
    Ok(messages)
}

/// Reads every stored revision for one message in an Operit1 Room schema 20 or 21 database.
fn readOperit1RoomV20MessageVariants(
    connection: &mut dyn RuntimeSqliteConnection,
    chatId: &str,
    messageTimestamp: i64,
    sender: &str,
) -> Result<Vec<OperitArchivedMessageVariant>, String> {
    let rows = connection
        .query(
            r#"
            SELECT variantIndex, content, roleName, provider, modelName, inputTokens, outputTokens,
                cachedInputTokens, sentAt, outputDurationMs, waitDurationMs, completedAt
            FROM message_variants
            WHERE chatId = ?1 AND messageTimestamp = ?2
            ORDER BY variantIndex ASC
            "#,
            vec![
                SqliteValue::Text(chatId.to_string()),
                SqliteValue::Integer(messageTimestamp),
            ],
        )
        .map_err(|error| error.to_string())?;
    rows.iter()
        .map(|row| {
            Ok(OperitArchivedMessageVariant {
                variantIndex: sqliteRowI32(row, 0, "message_variants.variantIndex")?,
                parts: parseOperit1MessageParts(
                    sender,
                    sqliteRowString(row, 1, "message_variants.content")?,
                )?,
                roleName: sqliteRowString(row, 2, "message_variants.roleName")?,
                provider: sqliteRowString(row, 3, "message_variants.provider")?,
                modelName: sqliteRowString(row, 4, "message_variants.modelName")?,
                inputTokens: sqliteRowI64(row, 5, "message_variants.inputTokens")?,
                outputTokens: sqliteRowI64(row, 6, "message_variants.outputTokens")?,
                cachedInputTokens: sqliteRowI64(row, 7, "message_variants.cachedInputTokens")?,
                sentAt: sqliteRowI64(row, 8, "message_variants.sentAt")?,
                outputDurationMs: sqliteRowI64(row, 9, "message_variants.outputDurationMs")?,
                waitDurationMs: sqliteRowI64(row, 10, "message_variants.waitDurationMs")?,
                completedAt: sqliteRowI64(row, 11, "message_variants.completedAt")?,
            })
        })
        .collect()
}

/// Converts one legacy text payload into the runtime's canonical message parts.
fn parseOperit1MessageParts(sender: &str, content: String) -> Result<Vec<MessagePart>, String> {
    if sender == "ai" {
        MessagePartCodec::parseAssistantMarkup(&content)
            .map_err(|error| format!("Operit1 assistant message markup is invalid: {error}"))
    } else {
        Ok(vec![MessagePart::markdown(
            "part-0".to_string(),
            0,
            content,
        )])
    }
}

/// Parses a display mode persisted by Operit1 Room into the current typed representation.
fn parseOperit1ChatMessageDisplayMode(value: &str) -> Result<ChatMessageDisplayMode, String> {
    match value {
        "NORMAL" => Ok(ChatMessageDisplayMode::NORMAL),
        "HIDDEN_PLACEHOLDER" => Ok(ChatMessageDisplayMode::HIDDEN_PLACEHOLDER),
        other => Err(format!("Operit1 message display mode is unknown: {other}")),
    }
}

#[allow(non_snake_case)]
/// Returns whether the opened SQLite database contains the named table.
fn sqliteTableExists(
    connection: &mut dyn RuntimeSqliteConnection,
    tableName: &str,
) -> Result<bool, String> {
    let rows = connection
        .query(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            vec![SqliteValue::Text(tableName.to_string())],
        )
        .map_err(|error| error.to_string())?;
    let row = rows
        .first()
        .ok_or_else(|| "SQLite table existence query returned no row".to_string())?;
    Ok(sqliteRowI64(row, 0, "sqlite_master.exists")? != 0)
}

/// Returns chat and message counts from a complete Operit1 Room chat database.
fn operit1ChatDatabaseCounts(
    connection: &mut dyn RuntimeSqliteConnection,
) -> Result<(i32, i32), String> {
    requireOperit1ChatTables(connection)?;
    Ok((
        queryCount(connection, "SELECT COUNT(*) FROM chats")?,
        queryCount(connection, "SELECT COUNT(*) FROM messages")?,
    ))
}

/// Verifies the tables required by every supported Operit1 Room chat schema.
fn requireOperit1ChatTables(connection: &mut dyn RuntimeSqliteConnection) -> Result<(), String> {
    for tableName in ["chats", "messages"] {
        if !sqliteTableExists(connection, tableName)? {
            return Err(format!(
                "Operit1 Room chat database is missing required table: {tableName}"
            ));
        }
    }
    Ok(())
}

#[allow(non_snake_case)]
/// Counts rows with a scalar count query.
fn queryCount(connection: &mut dyn RuntimeSqliteConnection, sql: &str) -> Result<i32, String> {
    let rows = connection
        .query(sql, Vec::new())
        .map_err(|error| error.to_string())?;
    let row = rows
        .first()
        .ok_or_else(|| "SQLite count query returned no row".to_string())?;
    i32::try_from(sqliteRowI64(row, 0, "count")?)
        .map_err(|_| "SQLite count does not fit i32".to_string())
}

/// Reads one required integer from a host-neutral SQLite row.
fn sqliteRowI64(row: &SqliteRow, index: usize, label: &str) -> Result<i64, String> {
    row.valueAt(index)
        .map_err(|error| error.to_string())?
        .asI64()
        .map_err(|error| format!("{label}: {error}"))
}

/// Reads one nullable integer from a host-neutral SQLite row.
fn sqliteRowOptionalI64(row: &SqliteRow, index: usize, label: &str) -> Result<Option<i64>, String> {
    let value = row.valueAt(index).map_err(|error| error.to_string())?;
    if value.isNull() {
        return Ok(None);
    }
    value
        .asI64()
        .map(Some)
        .map_err(|error| format!("{label}: {error}"))
}

/// Reads one nullable real value from a host-neutral SQLite row.
fn sqliteRowOptionalF64(row: &SqliteRow, index: usize, label: &str) -> Result<Option<f64>, String> {
    let value = row.valueAt(index).map_err(|error| error.to_string())?;
    if value.isNull() {
        return Ok(None);
    }
    value
        .asF64()
        .map(Some)
        .map_err(|error| format!("{label}: {error}"))
}

/// Reads one required 32-bit integer from a host-neutral SQLite row.
fn sqliteRowI32(row: &SqliteRow, index: usize, label: &str) -> Result<i32, String> {
    let value = sqliteRowI64(row, index, label)?;
    i32::try_from(value).map_err(|_| format!("{label}: integer does not fit i32: {value}"))
}

/// Reads one required text value from a host-neutral SQLite row.
fn sqliteRowString(row: &SqliteRow, index: usize, label: &str) -> Result<String, String> {
    row.valueAt(index)
        .map_err(|error| error.to_string())?
        .asString()
        .map_err(|error| format!("{label}: {error}"))
}

/// Reads one nullable text value from a host-neutral SQLite row.
fn sqliteRowOptionalString(
    row: &SqliteRow,
    index: usize,
    label: &str,
) -> Result<Option<String>, String> {
    let value = row.valueAt(index).map_err(|error| error.to_string())?;
    if value.isNull() {
        return Ok(None);
    }
    value
        .asString()
        .map(Some)
        .map_err(|error| format!("{label}: {error}"))
}

