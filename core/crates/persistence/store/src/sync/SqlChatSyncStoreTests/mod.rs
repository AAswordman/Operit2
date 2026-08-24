use super::*;

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::AppDatabase::DATABASE_VERSION;
use crate::sqliteParams;
use crate::RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost};
use operit_host_api::{
    HostError, HostResult, RuntimeSqliteConnection, RuntimeSqliteHost, RuntimeSqliteTransaction,
    RuntimeStorageEntry, RuntimeStorageHost, SqliteRow as HostSqliteRow, SqliteValue,
};
use operit_util::RuntimeStorageLayout::WORKSPACE_DIR_PATH;
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};
use rusqlite::types::Value as RusqliteValue;

static HOSTS: OnceLock<()> = OnceLock::new();
static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
static DATABASE_MUTEX: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug)]
struct TestRuntimeHost {
    root: PathBuf,
}

impl TestRuntimeHost {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn resolve(&self, path: &str) -> HostResult<PathBuf> {
        let path = Path::new(path);
        if path.is_absolute() {
            return Err(HostError::new(format!(
                "Runtime storage path must be relative: {}",
                path.display()
            )));
        }
        let mut resolved = self.root.clone();
        for component in path.components() {
            match component {
                Component::Normal(segment) => resolved.push(segment),
                Component::CurDir => {}
                _ => {
                    return Err(HostError::new(format!(
                        "Invalid runtime storage path: {}",
                        path.display()
                    )))
                }
            }
        }
        Ok(resolved)
    }
}

impl RuntimeStorageHost for TestRuntimeHost {
    fn runtimeRootDir(&self) -> Option<PathBuf> {
        Some(self.root.clone())
    }

    fn workspaceRootDir(&self) -> Option<PathBuf> {
        Some(self.root.join(WORKSPACE_DIR_PATH))
    }

    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        Ok(fs::read(self.resolve(path)?)?)
    }

    /// Reads one bounded byte range from the filesystem-backed test host.
    fn readBytesRange(&self, path: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        let content = self.readBytes(path)?;
        let start = usize::try_from(offset)
            .map_err(|_| HostError::new("runtime storage offset does not fit usize"))?;
        if start >= content.len() {
            return Ok(Vec::new());
        }
        let end = start
            .checked_add(length)
            .ok_or_else(|| HostError::new("runtime storage byte range overflows usize"))?
            .min(content.len());
        Ok(content[start..end].to_vec())
    }

    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    /// Appends bytes to the SQL chat sync test runtime root.
    fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        std::io::Write::write_all(&mut file, content)?;
        Ok(())
    }

    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        let path = self.resolve(path)?;
        if !path.exists() {
            return Ok(());
        }
        if path.is_dir() {
            if recursive {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_dir(path)?;
            }
        } else {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn exists(&self, path: &str) -> HostResult<bool> {
        Ok(self.resolve(path)?.exists())
    }

    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        let directory = self.resolve(prefix)?;
        let mut entries = Vec::new();
        if !directory.exists() {
            return Ok(entries);
        }
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let path = entry
                .path()
                .strip_prefix(&self.root)
                .map_err(|error| HostError::new(error.to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            entries.push(RuntimeStorageEntry {
                path,
                isDirectory: metadata.is_dir(),
                size: metadata.len() as i64,
            });
        }
        Ok(entries)
    }
}

impl RuntimeSqliteHost for TestRuntimeHost {
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let connection =
            rusqlite::Connection::open(path).map_err(|error| HostError::new(error.to_string()))?;
        connection
            .execute_batch(
                r#"
                    PRAGMA journal_mode = MEMORY;
                    PRAGMA synchronous = OFF;
                    PRAGMA temp_store = MEMORY;
                    "#,
            )
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(Box::new(TestRuntimeSqliteConnection { connection }))
    }
}

struct TestRuntimeSqliteConnection {
    connection: rusqlite::Connection,
}

impl RuntimeSqliteConnection for TestRuntimeSqliteConnection {
    fn executeBatch(&mut self, sql: &str) -> HostResult<()> {
        self.connection
            .execute_batch(sql)
            .map_err(|error| HostError::new(error.to_string()))
    }

    fn execute(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<usize> {
        let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
        self.connection
            .execute(sql, rusqlite::params_from_iter(params))
            .map_err(|error| HostError::new(error.to_string()))
    }

    fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<HostSqliteRow>> {
        queryRows(&self.connection, sql, params)
    }

    fn lastInsertRowId(&self) -> HostResult<i64> {
        Ok(self.connection.last_insert_rowid())
    }

    fn beginTransaction(&mut self) -> HostResult<Box<dyn RuntimeSqliteTransaction + '_>> {
        let transaction = self
            .connection
            .transaction()
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(Box::new(TestRuntimeSqliteTransaction { transaction }))
    }
}

struct TestRuntimeSqliteTransaction<'a> {
    transaction: rusqlite::Transaction<'a>,
}

impl RuntimeSqliteTransaction for TestRuntimeSqliteTransaction<'_> {
    fn execute(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<usize> {
        let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
        self.transaction
            .execute(sql, rusqlite::params_from_iter(params))
            .map_err(|error| HostError::new(error.to_string()))
    }

    fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<HostSqliteRow>> {
        queryRows(&self.transaction, sql, params)
    }

    fn lastInsertRowId(&self) -> HostResult<i64> {
        Ok(self.transaction.last_insert_rowid())
    }

    fn commit(self: Box<Self>) -> HostResult<()> {
        self.transaction
            .commit()
            .map_err(|error| HostError::new(error.to_string()))
    }
}

trait TestRusqliteConnection {
    fn prepareStatement<'a>(&'a self, sql: &str) -> rusqlite::Result<rusqlite::Statement<'a>>;
}

impl TestRusqliteConnection for rusqlite::Connection {
    fn prepareStatement<'a>(&'a self, sql: &str) -> rusqlite::Result<rusqlite::Statement<'a>> {
        self.prepare(sql)
    }
}

impl TestRusqliteConnection for rusqlite::Transaction<'_> {
    fn prepareStatement<'a>(&'a self, sql: &str) -> rusqlite::Result<rusqlite::Statement<'a>> {
        self.prepare(sql)
    }
}

fn queryRows(
    connection: &impl TestRusqliteConnection,
    sql: &str,
    params: Vec<SqliteValue>,
) -> HostResult<Vec<HostSqliteRow>> {
    let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
    let mut statement = connection
        .prepareStatement(sql)
        .map_err(|error| HostError::new(error.to_string()))?;
    let columns = statement
        .column_names()
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let mut rows = statement
        .query(rusqlite::params_from_iter(params))
        .map_err(|error| HostError::new(error.to_string()))?;
    let mut out = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|error| HostError::new(error.to_string()))?
    {
        let mut values = Vec::new();
        for index in 0..columns.len() {
            let value = row
                .get::<_, RusqliteValue>(index)
                .map_err(|error| HostError::new(error.to_string()))?;
            values.push(fromRusqliteValue(value));
        }
        out.push(HostSqliteRow {
            columns: columns.clone(),
            values,
        });
    }
    Ok(out)
}

fn toRusqliteValue(value: SqliteValue) -> RusqliteValue {
    match value {
        SqliteValue::Null => RusqliteValue::Null,
        SqliteValue::Integer(value) => RusqliteValue::Integer(value),
        SqliteValue::Real(value) => RusqliteValue::Real(value),
        SqliteValue::Text(value) => RusqliteValue::Text(value),
        SqliteValue::Blob(value) => RusqliteValue::Blob(value),
    }
}

fn fromRusqliteValue(value: RusqliteValue) -> SqliteValue {
    match value {
        RusqliteValue::Null => SqliteValue::Null,
        RusqliteValue::Integer(value) => SqliteValue::Integer(value),
        RusqliteValue::Real(value) => SqliteValue::Real(value),
        RusqliteValue::Text(value) => SqliteValue::Text(value),
        RusqliteValue::Blob(value) => SqliteValue::Blob(value),
    }
}

fn installTestHosts() {
    HOSTS.get_or_init(|| {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock must be after UNIX_EPOCH")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "operit2-sql-sync-tests-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("test runtime host root must be created");
        let host = Arc::new(TestRuntimeHost::new(root));
        setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(
            host.root.clone(),
            host.root.join(WORKSPACE_DIR_PATH),
        ));
        setDefaultRuntimeStorageHost(host.clone());
        setDefaultRuntimeSqliteHost(host);
    });
}

fn testPaths(name: &str) -> RuntimeStorePaths {
    installTestHosts();
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let runtimeDir = RuntimeStorePaths::default()
        .runtime_dir()
        .join(format!("sync-tests/{name}-{id}"));
    RuntimeStorePaths::new(runtimeDir.clone(), runtimeDir.join(WORKSPACE_DIR_PATH))
}

fn openTestStore(name: &str) -> (RuntimeStorePaths, Arc<AppDatabase>, SqlChatSyncStore) {
    AppDatabase::closeDatabase();
    let paths = testPaths(name);
    let database = AppDatabase::getDatabase(paths.clone()).unwrap();
    let syncStore = SqlChatSyncStore::new(paths.clone(), &database).unwrap();
    (paths, database, syncStore)
}

fn chat(chatId: &str) -> ChatEntity {
    ChatEntity::new(chatId.to_string(), "New Chat".to_string(), 1_000)
}

fn message(chatId: &str, timestamp: i64, content: &str) -> MessageEntity {
    MessageEntity {
        messageId: 0,
        chatId: chatId.to_string(),
        sender: "ai".to_string(),
        timestamp,
        orderIndex: 0,
        roleName: String::new(),
        selectedVariantIndex: 0,
        provider: "test-provider".to_string(),
        modelName: "test-model".to_string(),
        inputTokens: 0,
        outputTokens: 0,
        cachedInputTokens: 0,
        sentAt: 0,
        outputDurationMs: 0,
        waitDurationMs: 0,
        completedAt: 0,
        completedExecutionGeneration: 0,
        displayMode: "NORMAL".to_string(),
        isFavorite: false,
    }
}

/// Builds the markdown part row used by the sync fixtures.
fn messagePart(chatId: &str, timestamp: i64, content: &str) -> MessagePartEntity {
    MessagePartEntity::fromMessagePart(
        chatId.to_string(),
        timestamp,
        0,
        operit_model::MessagePart::MessagePart::markdown(
            "part-0".to_string(),
            0,
            content.to_string(),
        ),
    )
}

/// Inserts a fixture message and its canonical markdown part without replacing its parent chat.
fn insertChatMessage(database: &AppDatabase, chatId: &str, timestamp: i64, content: &str) -> i64 {
    if database.chatDao().getChatById(chatId).unwrap().is_none() {
        database.chatDao().insertChat(chat(chatId)).unwrap();
    }
    let messageId = database
        .messageDao()
        .insertMessage(message(chatId, timestamp, content))
        .unwrap();
    database
        .messagePartDao()
        .replaceParts(
            chatId,
            timestamp,
            0,
            vec![messagePart(chatId, timestamp, content)],
        )
        .unwrap();
    messageId
}

/// Replaces the markdown part used by one sync fixture message.
fn updateMessagePart(database: &AppDatabase, chatId: &str, timestamp: i64, content: String) {
    database
        .messagePartDao()
        .replaceParts(
            chatId,
            timestamp,
            0,
            vec![messagePart(chatId, timestamp, &content)],
        )
        .unwrap();
}

fn exportedPayload(operation: &SyncOperation) -> ChatSyncPayload {
    serde_json::from_value(operation.payload.clone()).unwrap()
}

fn sqlOperationCount(database: &AppDatabase) -> i64 {
    database
        .store()
        .queryScalar("SELECT COUNT(*) FROM sync_sql_operations", sqliteParams![])
        .unwrap()
}

fn sqlMessageRowCount(database: &AppDatabase) -> i64 {
    database
        .store()
        .queryScalar(
            "SELECT COUNT(*) FROM sync_sql_message_rows",
            sqliteParams![],
        )
        .unwrap()
}

/// Builds a current schema sync operation for merge and ordering fixtures.
fn upsertOperation(sequence: i64, content: &str) -> SyncOperation {
    let payload = ChatSyncPayload {
        chatRows: vec![chat("chat-remote")],
        messageRows: vec![message("chat-remote", 2_000, content)],
        partRows: vec![messagePart("chat-remote", 2_000, content)],
        variantRows: Vec::new(),
        deletions: Vec::new(),
    };
    SyncOperation {
        opId: format!("remote-sql:{sequence}"),
        originDeviceId: "remote-sql".to_string(),
        sequence,
        domain: CHAT_SYNC_DOMAIN.to_string(),
        entityType: "message".to_string(),
        entityId: "chat-remote:2000".to_string(),
        operation: "upsert".to_string(),
        semantics: SyncOperationSemantics::EntityState,
        payload: serde_json::to_value(payload).unwrap(),
        createdAt: sequence,
        schemaVersion: 5,
    }
}

/// Verifies that applying a remote chat operation updates observers of the chats table.
#[test]
fn remote_chat_operation_invalidates_chat_history_flow() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (_sourcePaths, sourceDatabase, sourceSyncStore) = openTestStore("history-flow-source");
    sourceDatabase
        .chatDao()
        .insertChat(chat("remote-flow-chat"))
        .unwrap();
    sourceSyncStore
        .recordChatMetadata("remote-flow-chat")
        .unwrap();
    let operations = sourceSyncStore
        .operationsSince(
            &SyncClock::default(),
            &[CHAT_SYNC_DOMAIN.to_string()],
            10,
        )
        .unwrap();
    assert_eq!(operations.len(), 1);

    AppDatabase::closeDatabase();
    let (_targetPaths, targetDatabase, targetSyncStore) = openTestStore("history-flow-target");
    let chatHistoriesFlow = targetDatabase.chatDao().getAllChats().unwrap();
    let observedChatIds = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let observedChatIdsForSubscription = Arc::clone(&observedChatIds);
    let _subscription = chatHistoriesFlow.subscribe(move |histories| {
        observedChatIdsForSubscription
            .lock()
            .unwrap()
            .push(histories.into_iter().map(|history| history.id).collect());
    });

    assert!(chatHistoriesFlow.value().is_empty());
    for operation in &operations {
        targetSyncStore.applyOperation(operation).unwrap();
    }

    assert_eq!(
        chatHistoriesFlow
            .value()
            .into_iter()
            .map(|history| history.id)
            .collect::<Vec<_>>(),
        vec!["remote-flow-chat".to_string()]
    );
    assert!(observedChatIds
        .lock()
        .unwrap()
        .iter()
        .any(|chatIds| chatIds == &["remote-flow-chat".to_string()]));
}
/// Verifies assistant completion, chat metadata, and sync rows commit as one generation.
#[test]
fn assistant_segment_commit_is_atomic_and_synchronizable() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (_sourcePaths, sourceDatabase, sourceSyncStore) = openTestStore("assistant-commit-source");
    let chatId = "chat-assistant-commit";
    let timestamp = 7_000;
    sourceDatabase.chatDao().insertChat(chat(chatId)).unwrap();
    insertChatMessage(&sourceDatabase, chatId, timestamp, "partial");

    let mut completedChat = sourceDatabase
        .chatDao()
        .getChatById(chatId)
        .unwrap()
        .expect("assistant completion chat must exist");
    completedChat.updatedAt = 7_100;
    completedChat.inputTokens = 11;
    completedChat.outputTokens = 13;
    completedChat.currentWindowSize = 4_096;
    let mut completedMessage = message(chatId, timestamp, "complete");
    completedMessage.completedAt = 7_100;
    completedMessage.completedExecutionGeneration = 1;
    completedMessage.inputTokens = 11;
    completedMessage.outputTokens = 13;
    let committedClock = sourceSyncStore
        .commitAssistantMessageSegment(
            completedChat.clone(),
            completedMessage.clone(),
            vec![messagePart(chatId, timestamp, "complete")],
        )
        .unwrap();
    assert!(committedClock.sequenceFor(&sourceSyncStore.originDeviceId) > 0);
    assert_eq!(
        sourceDatabase
            .messageDao()
            .getMessageByTimestamp(chatId, timestamp)
            .unwrap()
            .expect("completed assistant message must exist")
            .completedExecutionGeneration,
        1
    );
    assert_eq!(
        sourceDatabase
            .messagePartDao()
            .getPartsForMessage(chatId, timestamp, 0)
            .unwrap()[0]
            .content,
        "complete"
    );
    assert_eq!(
        sourceDatabase
            .chatDao()
            .getChatById(chatId)
            .unwrap()
            .expect("completed assistant chat metadata must exist"),
        completedChat
    );
    let operations = sourceSyncStore
        .operationsSince(&SyncClock::empty(), &[CHAT_SYNC_DOMAIN.to_string()], 10)
        .unwrap();
    AppDatabase::closeDatabase();

    let (_targetPaths, targetDatabase, targetSyncStore) = openTestStore("assistant-commit-target");
    for operation in &operations {
        targetSyncStore.applyOperation(operation).unwrap();
    }
    assert_eq!(
        targetDatabase
            .messageDao()
            .getMessageByTimestamp(chatId, timestamp)
            .unwrap()
            .expect("synchronized completed assistant message must exist")
            .completedExecutionGeneration,
        1
    );
    assert_eq!(
        targetDatabase
            .messagePartDao()
            .getPartsForMessage(chatId, timestamp, 0)
            .unwrap()[0]
            .content,
        "complete"
    );
    assert_eq!(
        targetDatabase
            .chatDao()
            .getChatById(chatId)
            .unwrap()
            .expect("synchronized completed assistant chat metadata must exist")
            .currentWindowSize,
        4_096
    );
    AppDatabase::closeDatabase();
}

#[test]
fn chat_dao_update_chats_preserves_child_messages() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (_paths, database, _syncStore) = openTestStore("chat-dao-update");
    let chatId = "chat-update";
    insertChatMessage(&database, chatId, 9_000, "kept");

    let mut chat = database
        .chatDao()
        .getChatById(chatId)
        .unwrap()
        .expect("chat must exist");
    chat.displayOrder = 42;
    chat.group = Some("updated-group".to_string());
    chat.updatedAt = 9_100;
    database.chatDao().updateChats(vec![chat]).unwrap();

    let updated = database
        .chatDao()
        .getChatById(chatId)
        .unwrap()
        .expect("chat must remain");
    assert_eq!(updated.displayOrder, 42);
    assert_eq!(updated.group.as_deref(), Some("updated-group"));
    let messages = database.messageDao().getMessagesForChat(chatId).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(
        database
            .messagePartDao()
            .getPartsForMessage(chatId, messages[0].timestamp, 0)
            .unwrap()[0]
            .content,
        "kept"
    );
    AppDatabase::closeDatabase();
}

/// Verifies the logical message identity replaces repeated writes instead of duplicating rows.
#[test]
fn repeated_message_entity_insert_replaces_the_existing_row() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (_paths, database, _syncStore) = openTestStore("message-entity-identity");
    let chatId = "chat-message-entity-identity";
    let timestamp = 9_200;
    database.chatDao().insertChat(chat(chatId)).unwrap();

    let mut first = message(chatId, timestamp, "first");
    first.outputTokens = 1;
    database.messageDao().insertMessage(first).unwrap();

    let mut second = message(chatId, timestamp, "second");
    second.outputTokens = 2;
    database.messageDao().insertMessage(second).unwrap();

    let messages = database.messageDao().getMessagesForChat(chatId).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].outputTokens, 2);
    AppDatabase::closeDatabase();
}

#[test]
fn message_dao_locator_previews_match_kotlin_projection() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (_paths, database, _syncStore) = openTestStore("message-dao-locator");
    let chatId = "chat-locator";
    insertChatMessage(&database, chatId, 10_000, "alpha content");
    insertChatMessage(&database, chatId, 10_100, "beta searchable content");

    let previews = database
        .messageDao()
        .getLocatorPreviewsForChat(chatId, 80)
        .unwrap();
    assert_eq!(previews.len(), 2);
    assert_eq!(previews[0].messageIndex, Some(0));
    assert_eq!(previews[1].messageIndex, Some(1));

    let searchPreviews = database
        .messageDao()
        .searchLocatorPreviewsForChat(chatId, "searchable", 80)
        .unwrap();
    assert_eq!(searchPreviews.len(), 1);
    assert_eq!(searchPreviews[0].messageIndex, Some(1));
    assert_eq!(searchPreviews[0].previewContent, "beta searchable content");
    AppDatabase::closeDatabase();
}

/// Verifies version-22 migration creates canonical structured message parts.
#[test]
fn migrates_version_22_messages_to_final_structured_parts() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    AppDatabase::closeDatabase();
    let paths = testPaths("structured-message-migration");
    {
        let store = SqliteStore::open(paths.sqlite_database_path()).unwrap();
        store
            .executeBatch(
                r#"
                CREATE TABLE chats (id TEXT PRIMARY KEY NOT NULL);
                CREATE TABLE messages (
                    messageId INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                    chatId TEXT NOT NULL,
                    sender TEXT NOT NULL,
                    content TEXT NOT NULL,
                    timestamp INTEGER NOT NULL,
                    orderIndex INTEGER NOT NULL,
                    roleName TEXT NOT NULL DEFAULT '',
                    selectedVariantIndex INTEGER NOT NULL DEFAULT 0,
                    provider TEXT NOT NULL DEFAULT '',
                    modelName TEXT NOT NULL DEFAULT '',
                    inputTokens INTEGER NOT NULL DEFAULT 0,
                    outputTokens INTEGER NOT NULL DEFAULT 0,
                    cachedInputTokens INTEGER NOT NULL DEFAULT 0,
                    sentAt INTEGER NOT NULL DEFAULT 0,
                    outputDurationMs INTEGER NOT NULL DEFAULT 0,
                    waitDurationMs INTEGER NOT NULL DEFAULT 0,
                    completedAt INTEGER NOT NULL DEFAULT 0,
                    displayMode TEXT NOT NULL DEFAULT 'NORMAL',
                    isFavorite INTEGER NOT NULL DEFAULT 0
                );
                CREATE TABLE message_variants (
                    variantId INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                    chatId TEXT NOT NULL,
                    messageTimestamp INTEGER NOT NULL,
                    variantIndex INTEGER NOT NULL,
                    content TEXT NOT NULL,
                    roleName TEXT NOT NULL DEFAULT '',
                    provider TEXT NOT NULL DEFAULT '',
                    modelName TEXT NOT NULL DEFAULT '',
                    inputTokens INTEGER NOT NULL DEFAULT 0,
                    outputTokens INTEGER NOT NULL DEFAULT 0,
                    cachedInputTokens INTEGER NOT NULL DEFAULT 0,
                    sentAt INTEGER NOT NULL DEFAULT 0,
                    outputDurationMs INTEGER NOT NULL DEFAULT 0,
                    waitDurationMs INTEGER NOT NULL DEFAULT 0,
                    completedAt INTEGER NOT NULL DEFAULT 0
                );
                CREATE TABLE sync_sql_operations (
                    opId TEXT PRIMARY KEY NOT NULL,
                    originDeviceId TEXT NOT NULL,
                    sequence INTEGER NOT NULL,
                    domain TEXT NOT NULL,
                    entityType TEXT NOT NULL,
                    entityId TEXT NOT NULL,
                    operation TEXT NOT NULL,
                    createdAt INTEGER NOT NULL,
                    schemaVersion INTEGER NOT NULL
                );
                CREATE TABLE sync_sql_clocks (
                    originDeviceId TEXT PRIMARY KEY NOT NULL,
                    sequence INTEGER NOT NULL
                );
                CREATE TABLE sync_sql_message_rows (
                    opId TEXT NOT NULL,
                    chatId TEXT NOT NULL,
                    sender TEXT NOT NULL,
                    content TEXT NOT NULL,
                    timestamp INTEGER NOT NULL,
                    orderIndex INTEGER NOT NULL DEFAULT 0,
                    roleName TEXT NOT NULL DEFAULT '',
                    selectedVariantIndex INTEGER NOT NULL DEFAULT 0,
                    provider TEXT NOT NULL DEFAULT '',
                    modelName TEXT NOT NULL DEFAULT '',
                    inputTokens INTEGER NOT NULL DEFAULT 0,
                    outputTokens INTEGER NOT NULL DEFAULT 0,
                    cachedInputTokens INTEGER NOT NULL DEFAULT 0,
                    sentAt INTEGER NOT NULL DEFAULT 0,
                    outputDurationMs INTEGER NOT NULL DEFAULT 0,
                    waitDurationMs INTEGER NOT NULL DEFAULT 0,
                    completedAt INTEGER NOT NULL DEFAULT 0,
                    displayMode TEXT NOT NULL DEFAULT 'NORMAL',
                    isFavorite INTEGER NOT NULL DEFAULT 0,
                    PRIMARY KEY(opId, chatId, timestamp)
                );
                CREATE TABLE sync_sql_message_variant_rows (
                    opId TEXT NOT NULL,
                    chatId TEXT NOT NULL,
                    messageTimestamp INTEGER NOT NULL,
                    variantIndex INTEGER NOT NULL,
                    content TEXT NOT NULL,
                    roleName TEXT NOT NULL DEFAULT '',
                    provider TEXT NOT NULL DEFAULT '',
                    modelName TEXT NOT NULL DEFAULT '',
                    inputTokens INTEGER NOT NULL DEFAULT 0,
                    outputTokens INTEGER NOT NULL DEFAULT 0,
                    cachedInputTokens INTEGER NOT NULL DEFAULT 0,
                    sentAt INTEGER NOT NULL DEFAULT 0,
                    outputDurationMs INTEGER NOT NULL DEFAULT 0,
                    waitDurationMs INTEGER NOT NULL DEFAULT 0,
                    completedAt INTEGER NOT NULL DEFAULT 0,
                    PRIMARY KEY(opId, chatId, messageTimestamp, variantIndex)
                );
                INSERT INTO chats (id) VALUES ('chat-22');
                INSERT INTO messages (chatId, sender, content, timestamp, orderIndex)
                VALUES ('chat-22', 'ai', '<think>**Stored thought**</think># Stored heading<tool_G543 name="read_file">', 1, 0);
                INSERT INTO message_variants (chatId, messageTimestamp, variantIndex, content)
                VALUES ('chat-22', 1, 1, '# Stored variant');
                INSERT INTO sync_sql_operations (
                    opId, originDeviceId, sequence, domain, entityType, entityId,
                    operation, createdAt, schemaVersion
                ) VALUES ('operation-22', 'device', 1, 'chat', 'message', 'chat-22:1', 'upsert', 1, 1);
                INSERT INTO sync_sql_message_rows (opId, chatId, sender, content, timestamp)
                VALUES ('operation-22', 'chat-22', 'ai', '# Queued heading', 1);
                "#,
            )
            .unwrap();
        store.setUserVersion(22).unwrap();
    }

    let database = AppDatabase::getDatabase(paths).unwrap();
    assert_eq!(database.store().getUserVersion().unwrap(), DATABASE_VERSION);
    let baseParts = database
        .messagePartDao()
        .getPartsForMessage("chat-22", 1, 0)
        .unwrap();
    assert_eq!(baseParts.len(), 3);
    assert_eq!(baseParts[0].content, "");
    assert_eq!(baseParts[1].content, "**Stored thought**");
    assert_eq!(
        baseParts[2].content,
        "# Stored heading<tool_G543 name=\"read_file\">"
    );
    let variantParts = database
        .messagePartDao()
        .getPartsForMessage("chat-22", 1, 1)
        .unwrap();
    assert_eq!(variantParts[0].content, "# Stored variant");
    assert_eq!(
        database
            .store()
            .queryScalar::<i32>(
                "SELECT schemaVersion FROM sync_sql_operations WHERE opId = 'operation-22'",
                sqliteParams![],
            )
            .unwrap(),
        5
    );
    AppDatabase::closeDatabase();
}

/// Verifies version-24 migration restores parts required by locator previews and hydration.
#[test]
fn migrates_version_23_message_revisions_to_canonical_visible_parts() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    AppDatabase::closeDatabase();
    let paths = testPaths("canonical-visible-parts-migration");
    let chatId = "chat-23";
    let timestamp = 23_000;
    {
        let store = SqliteStore::open(paths.sqlite_database_path()).unwrap();
        store
            .executeBatch(
                r#"
                CREATE TABLE chats (id TEXT PRIMARY KEY NOT NULL);
                CREATE TABLE messages (
                    messageId INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                    chatId TEXT NOT NULL,
                    sender TEXT NOT NULL,
                    timestamp INTEGER NOT NULL,
                    orderIndex INTEGER NOT NULL,
                    roleName TEXT NOT NULL DEFAULT '',
                    selectedVariantIndex INTEGER NOT NULL DEFAULT 0,
                    provider TEXT NOT NULL DEFAULT '',
                    modelName TEXT NOT NULL DEFAULT '',
                    inputTokens INTEGER NOT NULL DEFAULT 0,
                    outputTokens INTEGER NOT NULL DEFAULT 0,
                    cachedInputTokens INTEGER NOT NULL DEFAULT 0,
                    sentAt INTEGER NOT NULL DEFAULT 0,
                    outputDurationMs INTEGER NOT NULL DEFAULT 0,
                    waitDurationMs INTEGER NOT NULL DEFAULT 0,
                    completedAt INTEGER NOT NULL DEFAULT 0,
                    displayMode TEXT NOT NULL DEFAULT 'NORMAL',
                    isFavorite INTEGER NOT NULL DEFAULT 0
                );
                CREATE INDEX index_messages_chatId_timestamp
                    ON messages(chatId, timestamp);
                CREATE TABLE message_variants (
                    variantId INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                    chatId TEXT NOT NULL,
                    messageTimestamp INTEGER NOT NULL,
                    variantIndex INTEGER NOT NULL,
                    roleName TEXT NOT NULL DEFAULT '',
                    provider TEXT NOT NULL DEFAULT '',
                    modelName TEXT NOT NULL DEFAULT '',
                    inputTokens INTEGER NOT NULL DEFAULT 0,
                    outputTokens INTEGER NOT NULL DEFAULT 0,
                    cachedInputTokens INTEGER NOT NULL DEFAULT 0,
                    sentAt INTEGER NOT NULL DEFAULT 0,
                    outputDurationMs INTEGER NOT NULL DEFAULT 0,
                    waitDurationMs INTEGER NOT NULL DEFAULT 0,
                    completedAt INTEGER NOT NULL DEFAULT 0
                );
                CREATE TABLE message_parts (
                    chatId TEXT NOT NULL,
                    messageTimestamp INTEGER NOT NULL,
                    variantIndex INTEGER NOT NULL,
                    partId TEXT NOT NULL,
                    sequence INTEGER NOT NULL,
                    kind TEXT NOT NULL,
                    content TEXT NOT NULL,
                    toolCallId TEXT,
                    toolName TEXT,
                    attributesJson TEXT NOT NULL,
                    PRIMARY KEY(chatId, messageTimestamp, variantIndex, partId)
                );
                CREATE TABLE sync_sql_clocks (
                    originDeviceId TEXT PRIMARY KEY NOT NULL,
                    sequence INTEGER NOT NULL
                );
                CREATE TABLE sync_sql_operations (
                    opId TEXT PRIMARY KEY NOT NULL,
                    originDeviceId TEXT NOT NULL,
                    sequence INTEGER NOT NULL,
                    domain TEXT NOT NULL,
                    entityType TEXT NOT NULL,
                    entityId TEXT NOT NULL,
                    operation TEXT NOT NULL,
                    createdAt INTEGER NOT NULL,
                    schemaVersion INTEGER NOT NULL
                );
                CREATE TABLE sync_sql_message_rows (
                    opId TEXT NOT NULL,
                    chatId TEXT NOT NULL,
                    sender TEXT NOT NULL,
                    timestamp INTEGER NOT NULL,
                    orderIndex INTEGER NOT NULL,
                    roleName TEXT NOT NULL,
                    selectedVariantIndex INTEGER NOT NULL,
                    provider TEXT NOT NULL,
                    modelName TEXT NOT NULL,
                    inputTokens INTEGER NOT NULL,
                    outputTokens INTEGER NOT NULL,
                    cachedInputTokens INTEGER NOT NULL,
                    sentAt INTEGER NOT NULL,
                    outputDurationMs INTEGER NOT NULL,
                    waitDurationMs INTEGER NOT NULL,
                    completedAt INTEGER NOT NULL,
                    displayMode TEXT NOT NULL,
                    isFavorite INTEGER NOT NULL,
                    PRIMARY KEY(opId, chatId, timestamp)
                );
                INSERT INTO chats (id) VALUES ('chat-23');
                INSERT INTO messages (chatId, sender, timestamp, orderIndex)
                VALUES ('chat-23', 'ai', 23000, 0);
                INSERT INTO message_parts (
                    chatId, messageTimestamp, variantIndex, partId, sequence, kind,
                    content, toolCallId, toolName, attributesJson
                ) VALUES ('chat-23', 23000, 0, 'part-0', 0, 'thinking',
                    'internal only', NULL, NULL, '{}');
                "#,
            )
            .unwrap();
        store.setUserVersion(23).unwrap();
    }

    let database = AppDatabase::getDatabase(paths).unwrap();
    let parts = database
        .messagePartDao()
        .getPartsForMessage(chatId, timestamp, 0)
        .unwrap();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].partId, "part--1");
    assert_eq!(parts[0].sequence, -1);
    assert_eq!(parts[0].content, "");
    assert_eq!(
        parts[1].kind,
        operit_model::MessagePart::MessagePartKind::Thinking
    );
    let previews = database
        .messageDao()
        .getLocatorPreviewsForChat(chatId, 48)
        .unwrap();
    assert_eq!(previews[0].previewContent, "");
    assert_eq!(previews[0].contentLength, 0);
    AppDatabase::closeDatabase();
}

/// Verifies version-25 migration deterministically renames execution generation columns.
#[test]
fn migrates_version_25_execution_generation_columns() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    AppDatabase::closeDatabase();
    let paths = testPaths("execution-generation-migration");
    {
        let store = SqliteStore::open(paths.sqlite_database_path()).unwrap();
        store
            .executeBatch(
                r#"
                CREATE TABLE messages (
                    messageId INTEGER PRIMARY KEY NOT NULL,
                    completedRouteGeneration INTEGER NOT NULL
                );
                CREATE TABLE sync_sql_message_rows (
                    opId TEXT PRIMARY KEY NOT NULL,
                    completedRouteGeneration INTEGER NOT NULL
                );
                INSERT INTO messages (messageId, completedRouteGeneration) VALUES (1, 7);
                INSERT INTO sync_sql_message_rows (opId, completedRouteGeneration)
                VALUES ('operation-25', 9);
                "#,
            )
            .unwrap();
        store.setUserVersion(25).unwrap();
    }

    let database = AppDatabase::getDatabase(paths).unwrap();

    assert_eq!(database.store().getUserVersion().unwrap(), DATABASE_VERSION);
    assert_eq!(
        database
            .store()
            .queryScalar::<i64>(
                "SELECT completedExecutionGeneration FROM messages WHERE messageId = 1",
                sqliteParams![],
            )
            .unwrap(),
        7
    );
    assert_eq!(
        database
            .store()
            .queryScalar::<i64>(
                "SELECT completedExecutionGeneration FROM sync_sql_message_rows WHERE opId = 'operation-25'",
                sqliteParams![],
            )
            .unwrap(),
        9
    );
    AppDatabase::closeDatabase();
}

#[test]
fn record_message_snapshots_are_merged_into_final_stream_state() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (_paths, database, syncStore) = openTestStore("stream-merge");
    let chatId = "chat-stream";
    let timestamp = 10_000;
    let messageId = insertChatMessage(&database, chatId, timestamp, "");

    for index in 1..=100 {
        updateMessagePart(&database, chatId, timestamp, format!("token-{index}"));
        syncStore.recordMessageSnapshot(chatId, timestamp).unwrap();
    }

    assert_eq!(sqlOperationCount(&database), 1);
    let operations = syncStore
        .operationsSince(&SyncClock::empty(), &[CHAT_SYNC_DOMAIN.to_string()], 10)
        .unwrap();
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].sequence, 100);
    assert_eq!(operations[0].schemaVersion, 5);
    let payload = exportedPayload(&operations[0]);
    assert_eq!(payload.messageRows.len(), 1);
    assert_eq!(payload.partRows[0].content, "token-100");
    AppDatabase::closeDatabase();
}

#[test]
fn compacted_stream_snapshot_applies_to_new_receiver() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (_sourcePaths, sourceDatabase, sourceSyncStore) = openTestStore("source-stream");
    let chatId = "chat-apply";
    let timestamp = 11_000;
    let messageId = insertChatMessage(&sourceDatabase, chatId, timestamp, "");

    for index in 1..=50 {
        updateMessagePart(&sourceDatabase, chatId, timestamp, format!("chunk-{index}"));
        sourceSyncStore
            .recordMessageSnapshot(chatId, timestamp)
            .unwrap();
    }
    let operations = sourceSyncStore
        .operationsSince(&SyncClock::empty(), &[CHAT_SYNC_DOMAIN.to_string()], 10)
        .unwrap();
    AppDatabase::closeDatabase();

    let (_targetPaths, targetDatabase, targetSyncStore) = openTestStore("target-stream");
    for operation in &operations {
        targetSyncStore.applyOperation(operation).unwrap();
    }

    let message = targetDatabase
        .messageDao()
        .getMessageByTimestamp(chatId, timestamp)
        .unwrap()
        .unwrap();
    assert_eq!(
        targetDatabase
            .messagePartDao()
            .getPartsForMessage(chatId, timestamp, 0)
            .unwrap()[0]
            .content,
        "chunk-50"
    );
    assert_eq!(
        targetSyncStore
            .localClock()
            .unwrap()
            .sequenceFor(&operations[0].originDeviceId),
        50
    );
    AppDatabase::closeDatabase();
}

#[test]
fn older_merged_upsert_does_not_revert_newer_state() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (_paths, database, syncStore) = openTestStore("older-upsert");
    let newer = upsertOperation(2, "new");
    let older = upsertOperation(1, "old");

    syncStore.applyOperation(&newer).unwrap();
    syncStore.applyOperation(&older).unwrap();

    let message = database
        .messageDao()
        .getMessageByTimestamp("chat-remote", 2_000)
        .unwrap()
        .unwrap();
    assert_eq!(
        database
            .messagePartDao()
            .getPartsForMessage("chat-remote", 2_000, 0)
            .unwrap()[0]
            .content,
        "new"
    );
    assert_eq!(sqlOperationCount(&database), 1);
    AppDatabase::closeDatabase();
}

#[test]
fn delete_transaction_survives_compaction_and_applies() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (_sourcePaths, sourceDatabase, sourceSyncStore) = openTestStore("source-delete");
    let chatId = "chat-delete";
    let timestamp = 12_000;
    insertChatMessage(&sourceDatabase, chatId, timestamp, "remove-me");
    sourceSyncStore
        .recordMessageSnapshot(chatId, timestamp)
        .unwrap();
    sourceDatabase
        .messageDao()
        .deleteMessageByTimestamp(chatId, timestamp)
        .unwrap();
    sourceSyncStore
        .recordMessageDeletion(chatId, timestamp)
        .unwrap();
    let operations = sourceSyncStore
        .operationsSince(&SyncClock::empty(), &[CHAT_SYNC_DOMAIN.to_string()], 10)
        .unwrap();
    assert_eq!(
        operations
            .iter()
            .map(|operation| operation.operation.as_str())
            .collect::<Vec<_>>(),
        vec!["upsert", "delete"]
    );
    AppDatabase::closeDatabase();

    let (_targetPaths, targetDatabase, targetSyncStore) = openTestStore("target-delete");
    for operation in &operations {
        targetSyncStore.applyOperation(operation).unwrap();
    }

    assert!(targetDatabase
        .chatDao()
        .getChatById(chatId)
        .unwrap()
        .is_some());
    assert!(targetDatabase
        .messageDao()
        .getMessageByTimestamp(chatId, timestamp)
        .unwrap()
        .is_none());
    AppDatabase::closeDatabase();
}

#[test]
fn stress_stream_snapshots_export_single_final_operation() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (_paths, database, syncStore) = openTestStore("stress-stream");
    let chatId = "chat-stress";
    let timestamp = 13_000;
    let messageId = insertChatMessage(&database, chatId, timestamp, "");

    for index in 1..=1_000 {
        updateMessagePart(
            &database,
            chatId,
            timestamp,
            format!("stress-token-{index}"),
        );
        syncStore.recordMessageSnapshot(chatId, timestamp).unwrap();
    }

    assert_eq!(sqlOperationCount(&database), 1);
    let operations = syncStore
        .operationsSince(&SyncClock::empty(), &[CHAT_SYNC_DOMAIN.to_string()], 10)
        .unwrap();
    assert_eq!(operations.len(), 1);
    let payload = exportedPayload(&operations[0]);
    assert_eq!(payload.partRows[0].content, "stress-token-1000");
    AppDatabase::closeDatabase();
}

#[test]
fn stress_many_messages_roundtrip_with_stream_compaction() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (_sourcePaths, sourceDatabase, sourceSyncStore) = openTestStore("stress-roundtrip-source");
    let chatId = "chat-stress-roundtrip";
    let messageCount = 60;
    let updateRounds = 30;
    let mut messageIds = Vec::new();

    sourceDatabase.chatDao().insertChat(chat(chatId)).unwrap();
    for messageIndex in 0..messageCount {
        let timestamp = 20_000 + messageIndex as i64;
        let messageId = sourceDatabase
            .messageDao()
            .insertMessage(message(chatId, timestamp, ""))
            .unwrap();
        updateMessagePart(&sourceDatabase, chatId, timestamp, String::new());
        messageIds.push((timestamp, messageId));
    }

    for round in 1..=updateRounds {
        if round % 50 == 0 {
            eprintln!("sql sync ultra stress: recording round {round}/{updateRounds}");
        }
        for (messageIndex, (timestamp, messageId)) in messageIds.iter().enumerate() {
            updateMessagePart(
                &sourceDatabase,
                chatId,
                *timestamp,
                format!("message-{messageIndex}-round-{round}"),
            );
            sourceSyncStore
                .recordMessageSnapshot(chatId, *timestamp)
                .unwrap();
        }
    }

    assert_eq!(sqlOperationCount(&sourceDatabase), messageCount as i64);
    let operations = sourceSyncStore
        .operationsSince(
            &SyncClock::empty(),
            &[CHAT_SYNC_DOMAIN.to_string()],
            messageCount + 10,
        )
        .unwrap();
    assert_eq!(operations.len(), messageCount);
    assert!(operations
        .iter()
        .all(|operation| operation.operation == "upsert"));
    AppDatabase::closeDatabase();

    let (_targetPaths, targetDatabase, targetSyncStore) = openTestStore("stress-roundtrip-target");
    for operation in &operations {
        targetSyncStore.applyOperation(operation).unwrap();
    }

    for messageIndex in 0..messageCount {
        let timestamp = 20_000 + messageIndex as i64;
        let message = targetDatabase
            .messageDao()
            .getMessageByTimestamp(chatId, timestamp)
            .unwrap()
            .unwrap();
        assert_eq!(
            targetDatabase
                .messagePartDao()
                .getPartsForMessage(chatId, timestamp, 0)
                .unwrap()[0]
                .content,
            format!("message-{messageIndex}-round-{updateRounds}")
        );
    }
    assert_eq!(
        targetSyncStore
            .localClock()
            .unwrap()
            .sequenceFor(&operations[0].originDeviceId),
        (messageCount * updateRounds) as i64
    );
    AppDatabase::closeDatabase();
}

#[test]
#[ignore]
fn stress_ultra_many_messages_roundtrip_with_stream_compaction() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (_sourcePaths, sourceDatabase, sourceSyncStore) = openTestStore("stress-ultra-source");
    let chatId = "chat-stress-ultra";
    let messageCount = 600;
    let updateRounds = 300;
    let mut messageIds = Vec::new();

    sourceDatabase.chatDao().insertChat(chat(chatId)).unwrap();
    for messageIndex in 0..messageCount {
        let timestamp = 30_000 + messageIndex as i64;
        let messageId = sourceDatabase
            .messageDao()
            .insertMessage(message(chatId, timestamp, ""))
            .unwrap();
        updateMessagePart(&sourceDatabase, chatId, timestamp, String::new());
        messageIds.push((timestamp, messageId));
    }

    for round in 1..=updateRounds {
        for (messageIndex, (timestamp, messageId)) in messageIds.iter().enumerate() {
            updateMessagePart(
                &sourceDatabase,
                chatId,
                *timestamp,
                format!("message-{messageIndex}-round-{round}"),
            );
            sourceSyncStore
                .recordMessageSnapshot(chatId, *timestamp)
                .unwrap();
        }
    }

    let rawSnapshotCount = messageCount * updateRounds;
    let operationRows = sqlOperationCount(&sourceDatabase);
    let messageRows = sqlMessageRowCount(&sourceDatabase);
    let operations = sourceSyncStore
        .operationsSince(
            &SyncClock::empty(),
            &[CHAT_SYNC_DOMAIN.to_string()],
            messageCount + 10,
        )
        .unwrap();
    let exportedPayloadBytes = operations
        .iter()
        .map(|operation| serde_json::to_vec(&operation.payload).unwrap().len())
        .sum::<usize>();
    eprintln!(
            "sql sync ultra stress: raw_snapshots={rawSnapshotCount}, sync_sql_operations={operationRows}, sync_sql_message_rows={messageRows}, exported_operations={}, exported_payload_bytes={exportedPayloadBytes}",
            operations.len()
        );

    assert_eq!(operationRows, messageCount as i64);
    assert_eq!(messageRows, messageCount as i64);
    assert_eq!(operations.len(), messageCount);
    assert!(operations
        .iter()
        .all(|operation| operation.operation == "upsert"));
    AppDatabase::closeDatabase();

    let (_targetPaths, targetDatabase, targetSyncStore) = openTestStore("stress-ultra-target");
    for operation in &operations {
        targetSyncStore.applyOperation(operation).unwrap();
    }

    for messageIndex in 0..messageCount {
        let timestamp = 30_000 + messageIndex as i64;
        let message = targetDatabase
            .messageDao()
            .getMessageByTimestamp(chatId, timestamp)
            .unwrap()
            .unwrap();
        assert_eq!(
            targetDatabase
                .messagePartDao()
                .getPartsForMessage(chatId, timestamp, 0)
                .unwrap()[0]
                .content,
            format!("message-{messageIndex}-round-{updateRounds}")
        );
    }
    assert_eq!(
        targetSyncStore
            .localClock()
            .unwrap()
            .sequenceFor(&operations[0].originDeviceId),
        (messageCount * updateRounds) as i64
    );
    AppDatabase::closeDatabase();
}
