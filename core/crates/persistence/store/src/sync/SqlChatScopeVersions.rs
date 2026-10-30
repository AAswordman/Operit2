//! Business clocks travel with SQL snapshots, including empty scopes left by range deletions.
use super::*;
use crate::Transaction::Revision;

pub(super) fn prepare(store: &SqliteStore) -> Result<(), SqliteStoreError> {
    store.transaction(|tx| {
        if tx.queryOne("SELECT 1 FROM sqlite_master WHERE type='table' AND name='sync_sql_versions'", vec![])?.is_some() { return Ok(()); }
        tx.execute("CREATE TABLE sync_sql_versions (kind TEXT NOT NULL,chatId TEXT NOT NULL,timestamp INTEGER NOT NULL,variantIndex INTEGER NOT NULL,createdAt INTEGER NOT NULL,origin TEXT NOT NULL,sequence INTEGER NOT NULL,PRIMARY KEY(kind,chatId,timestamp,variantIndex))", vec![])?;
        // One-time import of the scope clocks retained by the previous SQL journal.
        for row in tx.queryRows("SELECT * FROM sync_sql_operations ORDER BY createdAt,originDeviceId,sequence", vec![])? {
            let operation = mapOperationMetadata(&row)?;
            let payload = readPayload(&mut |sql, params| tx.queryRows(sql, params), &operation.opId)?;
            observe_payload(tx, &operation, &payload)?;
        }
        Ok(())
    })
}

fn save(tx: &mut SqliteTransaction<'_>, operation: &SyncOperation, kind: &str, chat: &str, timestamp: Option<i64>, index: Option<i32>) -> Result<(), SqliteStoreError> {
    tx.execute("INSERT INTO sync_sql_versions VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(kind,chatId,timestamp,variantIndex) DO UPDATE SET createdAt=excluded.createdAt,origin=excluded.origin,sequence=excluded.sequence WHERE (excluded.origin=sync_sql_versions.origin AND excluded.sequence>sync_sql_versions.sequence) OR (excluded.origin!=sync_sql_versions.origin AND (excluded.createdAt,excluded.origin,excluded.sequence) > (sync_sql_versions.createdAt,sync_sql_versions.origin,sync_sql_versions.sequence))",
        sqliteParams![kind, chat, timestamp.unwrap_or(-1), index.unwrap_or(-1), operation.createdAt, operation.originDeviceId, operation.sequence])?;
    Ok(())
}

pub(super) fn observe_payload(tx: &mut SqliteTransaction<'_>, operation: &SyncOperation, payload: &ChatSyncPayload) -> Result<(), SqliteStoreError> {
    for chat in &payload.chatRows { save(tx, operation, DELETE_CHAT, &chat.id, None, None)?; }
    for message in &payload.messageRows { save(tx, operation, DELETE_MESSAGE, &message.chatId, Some(message.timestamp), None)?; }
    for variant in &payload.variantRows { save(tx, operation, DELETE_VARIANT, &variant.chatId, Some(variant.messageTimestamp), Some(variant.variantIndex))?; }
    for part in &payload.partRows {
        let (kind, index) = if part.variantIndex == 0 { (DELETE_MESSAGE, None) } else { (DELETE_VARIANT, Some(part.variantIndex)) };
        save(tx, operation, kind, &part.chatId, Some(part.messageTimestamp), index)?;
    }
    for deletion in &payload.deletions { save(tx, operation, &format!("delete:{}", deletion.tableName), &deletion.chatId, deletion.messageTimestamp, deletion.variantIndex)?; }
    Ok(())
}

pub(super) fn accepts(tx: &mut SqliteTransaction<'_>, operation: &SyncOperation, kind: &str, chat: &str, timestamp: Option<i64>, index: Option<i32>) -> Result<bool, SqliteStoreError> {
    let latest = tx.queryRows(
        "SELECT createdAt,origin,sequence FROM sync_sql_versions WHERE chatId=?1 AND (
          (kind IN (?2,'delete:'||?2) AND timestamp=?3 AND variantIndex=?4)
          OR (?2 IN ('messages','message_variants') AND (kind='delete:messages_for_chat' OR (kind='delete:messages_from' AND timestamp<=?3)))
          OR (?2='message_variants' AND ((kind='delete:messages' AND timestamp=?3) OR kind='delete:message_variants_for_chat' OR (kind='delete:message_variants_from' AND timestamp<=?3) OR (kind='delete:message_variants_for_message' AND timestamp=?3)))
        )",
        sqliteParams![chat, kind, timestamp.unwrap_or(-1), index.unwrap_or(-1)],
    )?;
    for row in latest {
        let revision = Revision { createdAt: row.get(0)?, originDeviceId: row.get(1)?, sequence: row.get(2)? };
        if crate::Transaction::newer(&revision,&Revision::fromOperation(operation)) { return Ok(false); }
    }
    Ok(true)
}

pub(super) fn delete(tx: &mut SqliteTransaction<'_>, operation: &SyncOperation, deletion: &ChatSyncDeletion, bootstrap: bool) -> Result<(), SqliteStoreError> {
    if bootstrap || deletion.tableName == DELETE_CHAT { return applyDeletion(tx, deletion); }
    match deletion.tableName.as_str() {
        DELETE_MESSAGE | DELETE_VARIANT => {
            if accepts(tx, operation, &deletion.tableName, &deletion.chatId, deletion.messageTimestamp, deletion.variantIndex)? { applyDeletion(tx, deletion)?; }
        }
        DELETE_MESSAGES_FROM | DELETE_MESSAGES_FOR_CHAT => {
            let from = if deletion.tableName == DELETE_MESSAGES_FROM { Some(requiredTimestamp(deletion)?) } else { None };
            for row in tx.queryRows("SELECT timestamp FROM messages WHERE chatId=?1 AND (?2 IS NULL OR timestamp>=?2)", sqliteParams![deletion.chatId, from])? {
                let timestamp = row.get(0)?;
                if accepts(tx, operation, DELETE_MESSAGE, &deletion.chatId, Some(timestamp), None)? {
                    applyDeletion(tx, &ChatSyncDeletion { tableName: DELETE_MESSAGE.into(), chatId: deletion.chatId.clone(), messageTimestamp: Some(timestamp), variantIndex: None })?;
                }
            }
        }
        DELETE_VARIANTS_FROM | DELETE_VARIANTS_FOR_MESSAGE | DELETE_VARIANTS_FOR_CHAT => {
            let exact = deletion.tableName == DELETE_VARIANTS_FOR_MESSAGE;
            let from = if deletion.tableName == DELETE_VARIANTS_FOR_CHAT { None } else { Some(requiredTimestamp(deletion)?) };
            for row in tx.queryRows("SELECT messageTimestamp,variantIndex FROM message_variants WHERE chatId=?1 AND (?2 IS NULL OR (CASE WHEN ?3 THEN messageTimestamp=?2 ELSE messageTimestamp>=?2 END))", sqliteParams![deletion.chatId, from, exact])? {
                let timestamp = row.get(0)?;
                let index = row.get(1)?;
                if accepts(tx, operation, DELETE_VARIANT, &deletion.chatId, Some(timestamp), Some(index))? {
                    applyDeletion(tx, &ChatSyncDeletion { tableName: DELETE_VARIANT.into(), chatId: deletion.chatId.clone(), messageTimestamp: Some(timestamp), variantIndex: Some(index) })?;
                }
            }
        }
        _ => applyDeletion(tx, deletion)?,
    }
    Ok(())
}
