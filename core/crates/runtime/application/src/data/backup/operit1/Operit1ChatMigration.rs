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
        Operit1ToOperit2ChatArchiveBridge::Operit1RoomV10ToOperit2SqliteV28 => {
            buildChatArchiveFromOperit1RoomV10Database(connection, fileImportPlan, onMessageParsed)
        }
        Operit1ToOperit2ChatArchiveBridge::Operit1RoomV20ToOperit2SqliteV28
        | Operit1ToOperit2ChatArchiveBridge::Operit1RoomV21ToOperit2SqliteV28 => {
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
            group: chat.group,
            pluginExtensions: std::collections::BTreeMap::from([(
                crate::data::backup::CharacterPluginMigration::LEGACY_BINDING_NAMESPACE.to_string(),
                serde_json::json!({"characterCardName":chat.characterCardName,"characterGroupId":chat.characterGroupId}),
            )]),
            messages,
            createdAt: epochMillisToLocalDateTimeString(chat.createdAt)?,
            updatedAt: epochMillisToLocalDateTimeString(chat.updatedAt)?,
            inputTokens: chat.inputTokens,
            outputTokens: chat.outputTokens,
            currentWindowSize: chat.currentWindowSize,
            displayOrder: chat.displayOrder,
            workspaceId: fileImportPlan.rewriteChatWorkspace(chat.workspace)?,
            parentChatId: chat.parentChatId,
            locked: chat.locked,
            pinned: false,
        });
    }
    archiveFromOperit1Chats(chats)
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
            group: chat.group,
            pluginExtensions: std::collections::BTreeMap::from([(
                crate::data::backup::CharacterPluginMigration::LEGACY_BINDING_NAMESPACE.to_string(),
                serde_json::json!({"characterCardName":chat.characterCardName,"characterGroupId":chat.characterGroupId}),
            )]),
            messages,
            createdAt: epochMillisToLocalDateTimeString(chat.createdAt)?,
            updatedAt: epochMillisToLocalDateTimeString(chat.updatedAt)?,
            inputTokens: chat.inputTokens,
            outputTokens: chat.outputTokens,
            currentWindowSize: chat.currentWindowSize,
            displayOrder: chat.displayOrder,
            workspaceId: fileImportPlan.rewriteChatWorkspace(chat.workspace)?,
            parentChatId: chat.parentChatId,
            locked: chat.locked,
            pinned: chat.pinned,
        });
    }
    archiveFromOperit1Chats(chats)
}

/// Converts Operit1 message identities and workspace paths into a portable archive.
fn archiveFromOperit1Chats(
    mut chats: Vec<OperitArchivedChat>,
) -> Result<OperitChatArchive, String> {
    let mut workspaces = Vec::new();
    let mut pathToId = std::collections::BTreeMap::<String, String>::new();
    let timestamp = currentTimeMillis();
    for chat in &mut chats {
        migrateOperit1MessageTimestampIdentities(&mut chat.messages)?;
        let Some(path) = chat.workspaceId.take() else {
            continue;
        };
        if let Some(workspaceId) = pathToId.get(&path) {
            chat.workspaceId = Some(workspaceId.clone());
            continue;
        }
        let folderName = operit_model::Workspace::Workspace::folderNameFromPath(&path)?;
        let workspaceName = if chat.title.trim().is_empty() {
            folderName.clone()
        } else {
            chat.title.clone()
        };
        let workspace = operit_model::Workspace::Workspace::fromSingleFolder(
            workspaceName,
            folderName,
            path.clone(),
            timestamp,
        );
        workspace.validate()?;
        chat.workspaceId = Some(workspace.id.clone());
        pathToId.insert(path, workspace.id.clone());
        workspaces.push(workspace);
    }
    Ok(OperitChatArchive {
        archiveType: ARCHIVE_TYPE.to_string(),
        formatVersion: CURRENT_FORMAT_VERSION,
        exportedAt: timestamp,
        workspaces,
        chats,
    })
}

/// Converts Room message-ID identities into distinct per-chat timestamp identities.
#[allow(non_snake_case)]
fn migrateOperit1MessageTimestampIdentities(
    messages: &mut [OperitArchivedMessage],
) -> Result<(), String> {
    // Room permits equal timestamps for distinct message IDs; Operit2 parts do not.
    // Reserve every source timestamp so collision conversion never changes another message.
    let sourceTimestamps = messages
        .iter()
        .map(|message| message.baseMessage.timestamp)
        .collect::<HashSet<_>>();
    let mut assignedTimestamps = HashSet::new();
    for message in messages {
        let sourceTimestamp = message.baseMessage.timestamp;
        let mut timestamp = sourceTimestamp;
        while assignedTimestamps.contains(&timestamp) {
            timestamp = timestamp.checked_add(1).ok_or_else(|| {
                format!("Operit1 message timestamp identity overflowed: {sourceTimestamp}")
            })?;
            while sourceTimestamps.contains(&timestamp) {
                timestamp = timestamp.checked_add(1).ok_or_else(|| {
                    format!("Operit1 message timestamp identity overflowed: {sourceTimestamp}")
                })?;
            }
        }
        assignedTimestamps.insert(timestamp);
        message.baseMessage.timestamp = timestamp;
    }
    Ok(())
}

/// Preserves the complete legacy Room source row while target projection deliberately omits its domain bindings.
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
            ORDER BY orderIndex ASC, timestamp ASC, messageId ASC
            "#,
            vec![SqliteValue::Text(chatId.to_string())],
        )
        .map_err(|error| error.to_string())?;
    let mut messages = Vec::with_capacity(rows.len());
    for row in &rows {
        let timestamp = sqliteRowI64(row, 2, "messages.timestamp")?;
        let sender = sqliteRowString(row, 0, "messages.sender")?;
        let content = sqliteRowString(row, 1, "messages.content")?;
        let parts = parseOperit1MessageParts(&sender, content);
        messages.push(OperitArchivedMessage {
            baseMessage: ChatMessage {
                sender,
                pluginExtensions: std::collections::BTreeMap::new(),
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
            ORDER BY orderIndex ASC, timestamp ASC, messageId ASC
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
                pluginExtensions: std::collections::BTreeMap::new(),
                parts: parseOperit1MessageParts(
                    &sender,
                    sqliteRowString(row, 1, "messages.content")?,
                ),
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
                pluginExtensions: std::collections::BTreeMap::new(),
                variantIndex: sqliteRowI32(row, 0, "message_variants.variantIndex")?,
                parts: parseOperit1MessageParts(
                    sender,
                    sqliteRowString(row, 1, "message_variants.content")?,
                ),
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
fn parseOperit1MessageParts(sender: &str, content: String) -> Vec<MessagePart> {
    if sender == "ai" {
        MessagePartCodec::parseAssistantMarkupForImport(&content)
    } else {
        vec![MessagePart::markdown("part-0".to_string(), 0, content)]
    }
}

/// Parses a display mode persisted by Operit1 Room into the current typed representation.
fn parseOperit1ChatMessageDisplayMode(value: &str) -> Result<ChatMessageDisplayMode, String> {
    match value {
        "NORMAL" => Ok(ChatMessageDisplayMode::NORMAL),
        "HIDDEN_PLACEHOLDER" => Ok(ChatMessageDisplayMode::HIDDEN_PLACEHOLDER),
        // Operit1's MessageEntity.toChatMessage also defaults unknown values to
        // NORMAL. This presentation hint must not prevent history migration.
        _ => Ok(ChatMessageDisplayMode::NORMAL),
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

#[cfg(test)]
mod chat_timestamp_tests {
    use super::*;

    /// Creates a source message with explicit content and timestamp for identity tests.
    fn archived_message(sender: &str, content: &str, timestamp: i64) -> OperitArchivedMessage {
        let mut baseMessage = ChatMessage::new_with_timestamp(
            sender.to_string(),
            vec![MessagePart::markdown(
                "part-0".to_string(),
                0,
                content.to_string(),
            )],
            0,
        );
        baseMessage.timestamp = timestamp;
        OperitArchivedMessage {
            baseMessage,
            variants: Vec::new(),
        }
    }

    /// Verifies equal AI and summary timestamps preserve both message bodies and order.
    #[test]
    fn migrates_duplicate_timestamps_without_losing_messages() {
        let mut messages = vec![
            archived_message("ai", "answer", 1_763_299_203_798),
            archived_message("summary", "summary", 1_763_299_203_798),
        ];
        migrateOperit1MessageTimestampIdentities(&mut messages).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].baseMessage.timestamp, 1_763_299_203_798);
        assert_eq!(messages[1].baseMessage.timestamp, 1_763_299_203_799);
        assert_eq!(messages[0].baseMessage.parts[0].content, "answer");
        assert_eq!(messages[1].baseMessage.parts[0].content, "summary");
    }

    /// Verifies collision conversion reserves timestamps belonging to later source messages.
    #[test]
    fn reserves_existing_timestamps_and_repeated_collision_assignments() {
        let mut messages = [10, 10, 11, 12, 10]
            .into_iter()
            .map(|timestamp| archived_message("user", "body", timestamp))
            .collect::<Vec<_>>();
        migrateOperit1MessageTimestampIdentities(&mut messages).unwrap();
        let timestamps = messages
            .iter()
            .map(|message| message.baseMessage.timestamp)
            .collect::<Vec<_>>();
        assert_eq!(timestamps, vec![10, 13, 11, 12, 14]);
        migrateOperit1MessageTimestampIdentities(&mut messages).unwrap();
        assert_eq!(
            messages
                .iter()
                .map(|message| message.baseMessage.timestamp)
                .collect::<Vec<_>>(),
            timestamps
        );
    }

    /// Verifies unique source timestamps remain unchanged, including their original order.
    #[test]
    fn preserves_unique_source_timestamps() {
        let mut messages = [30, 10, 20]
            .into_iter()
            .map(|timestamp| archived_message("user", "body", timestamp))
            .collect::<Vec<_>>();
        migrateOperit1MessageTimestampIdentities(&mut messages).unwrap();
        assert_eq!(
            messages
                .iter()
                .map(|message| message.baseMessage.timestamp)
                .collect::<Vec<_>>(),
            vec![30, 10, 20]
        );
    }

    /// Verifies exhausted timestamp identities fail explicitly instead of dropping a message.
    #[test]
    fn rejects_timestamp_identity_overflow() {
        let mut messages = vec![
            archived_message("ai", "answer", i64::MAX),
            archived_message("summary", "summary", i64::MAX),
        ];
        assert!(migrateOperit1MessageTimestampIdentities(&mut messages)
            .unwrap_err()
            .starts_with("Operit1 message timestamp identity overflowed:"));
    }
}

#[cfg(test)]
mod chat_markup_import_tests {
    use super::*;
    use operit_host_api::{HostError, HostResult, RuntimeSqliteTransaction};
    use operit_model::MessagePart::MessagePartKind;

    const ASSISTANT_TEXT: &str = "旧回复<tool_NtfY/>继续正文";
    const USER_TEXT: &str = "用户原文<think>不是思考块</think><tool_NtfY/>";

    /// Supplies the selected message and revision rows through the real reader contract.
    struct MessageRowsConnection {
        messages: Vec<SqliteRow>,
        variants: Vec<SqliteRow>,
    }

    impl RuntimeSqliteConnection for MessageRowsConnection {
        fn executeBatch(&mut self, _: &str) -> HostResult<()> {
            Err(HostError::new("unexpected write during history read"))
        }

        fn execute(&mut self, _: &str, _: Vec<SqliteValue>) -> HostResult<usize> {
            Err(HostError::new("unexpected write during history read"))
        }

        fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<SqliteRow>> {
            assert_eq!(params[0], SqliteValue::Text("chat-1".to_string()));
            if sql.contains("FROM message_variants") {
                assert_eq!(params.len(), 2);
                return Ok(if params[1] == SqliteValue::Integer(20) {
                    self.variants.clone()
                } else {
                    Vec::new()
                });
            }
            assert!(sql.contains("FROM messages"));
            Ok(self.messages.clone())
        }

        fn lastInsertRowId(&self) -> HostResult<i64> {
            Err(HostError::new("unexpected insert during history read"))
        }

        fn beginTransaction(&mut self) -> HostResult<Box<dyn RuntimeSqliteTransaction + '_>> {
            Err(HostError::new("unexpected transaction during history read"))
        }
    }

    fn text(value: &str) -> SqliteValue {
        SqliteValue::Text(value.to_string())
    }

    fn row(values: Vec<SqliteValue>) -> SqliteRow {
        SqliteRow {
            columns: Vec::new(),
            values,
        }
    }

    fn legacy_message(sender: &str, content: &str, timestamp: i64) -> SqliteRow {
        row(vec![
            text(sender),
            text(content),
            SqliteValue::Integer(timestamp),
            SqliteValue::Integer(timestamp),
            text("role"),
            text("provider"),
            text("model"),
        ])
    }

    fn modern_message(sender: &str, content: &str, timestamp: i64) -> SqliteRow {
        row(vec![
            text(sender),
            text(content),
            SqliteValue::Integer(timestamp),
            text("role"),
            SqliteValue::Integer(if sender == "ai" { 1 } else { 0 }),
            text("provider"),
            text("model"),
            SqliteValue::Integer(11),
            SqliteValue::Integer(12),
            SqliteValue::Integer(13),
            SqliteValue::Integer(14),
            SqliteValue::Integer(15),
            SqliteValue::Integer(16),
            SqliteValue::Integer(17),
            text("LEGACY_UNKNOWN_MODE"),
            SqliteValue::Integer(1),
        ])
    }

    /// A bad assistant fragment must not prevent either adjacent user message importing.
    #[test]
    fn room_v10_import_preserves_all_messages_with_self_closing_markup() {
        let mut connection = MessageRowsConnection {
            messages: vec![
                legacy_message("user", USER_TEXT, 10),
                legacy_message("ai", ASSISTANT_TEXT, 20),
                legacy_message("user", "后续用户消息", 30),
            ],
            variants: Vec::new(),
        };
        let mut parsed = 0;
        let messages = readOperit1RoomV10Messages(&mut connection, "chat-1", &mut || parsed += 1)
            .expect("legacy assistant markup must not block user history");
        assert_eq!(parsed, 3);
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].baseMessage.sender, "user");
        assert_eq!(
            messages[0].baseMessage.parts[0].kind,
            MessagePartKind::Markdown
        );
        assert_eq!(messages[0].baseMessage.parts[0].content, USER_TEXT);
        assert_eq!(
            MessagePartCodec::assistantMarkup(&messages[1].baseMessage.parts),
            ASSISTANT_TEXT
        );
        assert_eq!(messages[1].baseMessage.timestamp, 20);
        assert_eq!(messages[2].baseMessage.parts[0].content, "后续用户消息");
    }

    /// Room 20/21 revisions use the same lenient path as base messages.
    #[test]
    fn room_v20_v21_import_preserves_base_messages_and_variants() {
        let revision = "历史版本<tool_result_NtfY/><status type=\"completion\"/>";
        let mut connection = MessageRowsConnection {
            messages: vec![
                modern_message("user", USER_TEXT, 10),
                modern_message("ai", ASSISTANT_TEXT, 20),
                modern_message("user", "后续用户消息", 30),
            ],
            variants: vec![row(vec![
                SqliteValue::Integer(1),
                text(revision),
                text("variant-role"),
                text("variant-provider"),
                text("variant-model"),
                SqliteValue::Integer(21),
                SqliteValue::Integer(22),
                SqliteValue::Integer(23),
                SqliteValue::Integer(24),
                SqliteValue::Integer(25),
                SqliteValue::Integer(26),
                SqliteValue::Integer(27),
            ])],
        };
        let mut parsed = 0;
        let messages = readOperit1RoomV20Messages(&mut connection, "chat-1", &mut || parsed += 1)
            .expect("legacy message/revision markup must not block migration");
        assert_eq!(parsed, 3);
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].baseMessage.parts[0].content, USER_TEXT);
        let message = &messages[1];
        assert_eq!(
            MessagePartCodec::assistantMarkup(&message.baseMessage.parts),
            ASSISTANT_TEXT
        );
        assert_eq!(message.baseMessage.selectedVariantIndex, 1);
        assert_eq!(message.baseMessage.variantCount, 2);
        assert_eq!(message.baseMessage.inputTokens, 11);
        assert_eq!(message.baseMessage.outputTokens, 12);
        assert!(message.baseMessage.isFavorite);
        assert_eq!(
            message.baseMessage.displayMode,
            ChatMessageDisplayMode::NORMAL
        );
        assert_eq!(message.variants.len(), 1);
        let variant = &message.variants[0];
        assert_eq!(variant.variantIndex, 1);
        assert_eq!(MessagePartCodec::assistantMarkup(&variant.parts), revision);
        assert_eq!(variant.roleName, "variant-role");
        assert_eq!(variant.provider, "variant-provider");
        assert_eq!(variant.inputTokens, 21);
        assert_eq!(variant.completedAt, 27);
        assert_eq!(messages[2].baseMessage.parts[0].content, "后续用户消息");
    }

    /// Preserve known presentation modes and mirror Kotlin's default for unknown ones.
    #[test]
    fn import_defaults_unknown_display_modes_to_normal() {
        assert_eq!(
            parseOperit1ChatMessageDisplayMode("NORMAL").unwrap(),
            ChatMessageDisplayMode::NORMAL
        );
        assert_eq!(
            parseOperit1ChatMessageDisplayMode("HIDDEN_PLACEHOLDER").unwrap(),
            ChatMessageDisplayMode::HIDDEN_PLACEHOLDER
        );
        for mode in ["", "OTHER_MODE"] {
            assert_eq!(
                parseOperit1ChatMessageDisplayMode(mode).unwrap(),
                ChatMessageDisplayMode::NORMAL
            );
        }
    }
}
