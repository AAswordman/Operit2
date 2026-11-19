//! Standalone SQLite chat snapshots preserving database rows without archive conversion.
use crate::db::AppDatabase::{AppDatabase, DATABASE_VERSION};
use crate::sync::SqlChatSyncStore::SqlChatSyncStore;
use crate::RuntimeStorePaths::RuntimeStorePaths;
use crate::SqliteStore::{SqliteRowGet, SqliteStore, SqliteStoreError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const APPLICATION_ID: i32 = 0x4f434254;
const CHAT_TABLES: &[&str] = &["chats", "messages", "message_variants", "message_parts"];

/// Describes a verified database snapshot without loading conversation contents.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatDatabaseSnapshotInfo {
    pub createdAt: i64,
    pub chatCount: i64,
    pub messageCount: i64,
}

/// Captures committed SQLite pages, strips non-chat records in the copy, and purges freed pages.
pub fn create(
    destination: &str,
    createdAt: i64,
) -> Result<Option<ChatDatabaseSnapshotInfo>, String> {
    let database = AppDatabase::default().map_err(|e| e.to_string())?;
    let count: i64 = database
        .store()
        .queryScalar("SELECT count(*) FROM chats", vec![])
        .map_err(|e| e.to_string())?;
    if count == 0 {
        return Ok(None);
    }
    let storagePath =
        crate::RuntimeStorageHost::runtimeStoragePath(std::path::Path::new(destination));
    database
        .store()
        .snapshotDatabase(&storagePath)
        .map_err(|e| e.to_string())?;
    let image = SqliteStore::open(PathBuf::from(destination)).map_err(|e| e.to_string())?;
    image
        .executeBatch(
            "PRAGMA foreign_keys=OFF; PRAGMA secure_delete=ON; PRAGMA journal_mode=DELETE;",
        )
        .map_err(|e| e.to_string())?;
    image.transaction(|tx| {
        for row in tx.queryRows("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'", vec![])? {
            let name: String = row.get(0)?;
            if !CHAT_TABLES.contains(&name.as_str()) {
                tx.execute(&format!("DELETE FROM {}", crate::Transaction::sql_identifier(&name)),vec![])?;
            }
        }
        tx.execute("UPDATE chats SET workspaceId=NULL",vec![])?;
        tx.execute("DELETE FROM sqlite_sequence WHERE name NOT IN ('messages','message_variants')",vec![])?;
        tx.execute("CREATE TABLE _operit_chat_backup(format INTEGER NOT NULL, createdAt INTEGER NOT NULL)",vec![])?;
        tx.execute("INSERT INTO _operit_chat_backup VALUES(1,?1)",crate::sqliteParams![createdAt])?;
        Ok(())
    }).map_err(|e|e.to_string())?;
    image
        .executeBatch(&format!("PRAGMA application_id={APPLICATION_ID}; VACUUM;"))
        .map_err(|e| e.to_string())?;
    let info = inspectStore(&image)?;
    Ok((info.chatCount > 0).then_some(info))
}

/// Verifies SQLite integrity, schema compatibility and the snapshot's application marker.
pub fn inspect(path: &str) -> Result<ChatDatabaseSnapshotInfo, String> {
    let image = SqliteStore::open(PathBuf::from(path)).map_err(|e| e.to_string())?;
    inspectStore(&image)
}

/// Checks actual database structure rather than trusting the filename or extension.
fn inspectStore(image: &SqliteStore) -> Result<ChatDatabaseSnapshotInfo, String> {
    image
        .executeBatch("PRAGMA trusted_schema=OFF")
        .map_err(|e| e.to_string())?;
    if image
        .queryOne(
            "SELECT 1 FROM sqlite_master WHERE type IN ('trigger','view') LIMIT 1",
            vec![],
        )
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err("Chat snapshots cannot contain executable schema objects".into());
    }
    let app: i32 = image
        .queryScalar("PRAGMA application_id", vec![])
        .map_err(|e| e.to_string())?;
    if app != APPLICATION_ID {
        return Err("This is not an Operit chat database snapshot".into());
    }
    let version = image.getUserVersion().map_err(|e| e.to_string())?;
    if version <= 0 || version > DATABASE_VERSION {
        return Err(format!("Unsupported chat database schema: {version}"));
    }
    let integrity: String = image
        .queryScalar("PRAGMA integrity_check", vec![])
        .map_err(|e| e.to_string())?;
    if integrity != "ok" {
        return Err(format!("Chat database integrity check failed: {integrity}"));
    }
    if image
        .queryOne("PRAGMA foreign_key_check", vec![])
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err("Chat database contains invalid relationships".into());
    }
    let row = image
        .queryOne("SELECT format,createdAt FROM _operit_chat_backup", vec![])
        .map_err(|e| e.to_string())?
        .ok_or("Chat snapshot metadata is missing")?;
    if row.get::<_, i32>(0).map_err(|e| e.to_string())? != 1 {
        return Err("Unsupported chat snapshot format".into());
    }
    Ok(ChatDatabaseSnapshotInfo {
        createdAt: row.get(1).map_err(|e| e.to_string())?,
        chatCount: image
            .queryScalar("SELECT count(*) FROM chats", vec![])
            .map_err(|e| e.to_string())?,
        messageCount: image
            .queryScalar("SELECT count(*) FROM messages", vec![])
            .map_err(|e| e.to_string())?,
    })
}

/// Replaces chat tables atomically and seals existing sync history to prevent old replay undoing restore.
pub fn restore(path: &str) -> Result<ChatDatabaseSnapshotInfo, String> {
    let info = inspect(path)?;
    let source = AppDatabase::openSnapshot(PathBuf::from(path)).map_err(|e| e.to_string())?;
    let sourceSync =
        SqlChatSyncStore::new(RuntimeStorePaths::default(), &source).map_err(|e| e.to_string())?;
    let snapshot = sourceSync.syncSnapshot().map_err(|e| e.to_string())?;
    let live = AppDatabase::default().map_err(|e| e.to_string())?;
    let sync = SqlChatSyncStore::default().map_err(|e| e.to_string())?;
    let bindings = crate::CoreNodeBindingStore::CoreNodeBindingStore::default()?;
    let restoredIds = source
        .store()
        .queryRows("SELECT id FROM chats", vec![])
        .map_err(|e| e.to_string())?;
    live.store()
        .withIdleConversations(|| {
            // Initialize execution ownership for chats recovered after uninstall.
            // Existing bindings remain authoritative; routing metadata is not read from backups.
            for row in &restoredIds {
                let id: String = row.get(0)?;
                if !bindings.contains(&id).map_err(fail)? {
                    bindings.createLocal(&id).map_err(fail)?;
                }
            }
            let current = sync.syncSnapshot().map_err(fail)?;
            let token = current.token().map_err(fail)?;
            if !sync.repairSyncedSnapshot(&token, &snapshot).map_err(fail)? {
                return Err(fail(
                    "Chat data changed during restoration; select the snapshot again",
                ));
            }
            Ok(())
        })
        .map_err(|e| e.to_string())?;
    let selection = crate::PreferencesDataStore::CoreNodeStateStore::newWithStorage(
        crate::RuntimeStorageHost::defaultRuntimeStorageHost(),
        crate::RuntimeStorageHost::runtimeStoragePath(
            &RuntimeStorePaths::default().current_chat_id_preferences_path(),
        ),
    );
    selection
        .edit(|preferences| {
            preferences
                .remove(&crate::repository::ChatHistoryManager::PreferencesKeys::CURRENT_CHAT_ID())
        })
        .map_err(|e| e.to_string())?;
    Ok(info)
}

/// Converts restoration errors into the common SQLite transaction error surface.
fn fail(error: impl std::fmt::Display) -> SqliteStoreError {
    SqliteStoreError::Message(error.to_string())
}
