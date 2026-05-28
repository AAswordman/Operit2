use super::*;

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use operit_host_api::{
    HostError, HostResult, RuntimeSqliteConnection, RuntimeSqliteHost, RuntimeSqliteTransaction,
    RuntimeStorageEntry, RuntimeStorageHost, SqliteRow as HostSqliteRow, SqliteValue,
};
use operit_store::sqliteParams;
use operit_store::RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost};
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
    fn rootDir(&self) -> Option<PathBuf> {
        Some(self.root.clone())
    }

    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        Ok(fs::read(self.resolve(path)?)?)
    }

    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
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
        setDefaultRuntimeStorageHost(host.clone());
        setDefaultRuntimeSqliteHost(host);
    });
}

fn testPaths(name: &str) -> RuntimeStorePaths {
    installTestHosts();
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    RuntimeStorePaths::new(
        RuntimeStorePaths::default()
            .root_dir()
            .join(format!("sync-tests/{name}-{id}")),
    )
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
        content: content.to_string(),
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
        displayMode: "NORMAL".to_string(),
        isFavorite: false,
    }
}

fn insertChatMessage(database: &AppDatabase, chatId: &str, timestamp: i64, content: &str) -> i64 {
    database.chatDao().insertChat(chat(chatId)).unwrap();
    database
        .messageDao()
        .insertMessage(message(chatId, timestamp, content))
        .unwrap()
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

fn upsertOperation(sequence: i64, content: &str) -> SyncOperation {
    let payload = ChatSyncPayload {
        chatRows: vec![chat("chat-remote")],
        messageRows: vec![message("chat-remote", 2_000, content)],
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
        payload: serde_json::to_value(payload).unwrap(),
        createdAt: sequence,
        schemaVersion: 1,
    }
}

#[test]
fn record_message_snapshots_are_merged_into_final_stream_state() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (_paths, database, syncStore) = openTestStore("stream-merge");
    let chatId = "chat-stream";
    let timestamp = 10_000;
    let messageId = insertChatMessage(&database, chatId, timestamp, "");

    for index in 1..=100 {
        database
            .messageDao()
            .updateMessageContent(messageId, format!("token-{index}"))
            .unwrap();
        syncStore.recordMessageSnapshot(chatId, timestamp).unwrap();
    }

    assert_eq!(sqlOperationCount(&database), 1);
    let operations = syncStore
        .operationsSince(&SyncClock::empty(), &[CHAT_SYNC_DOMAIN.to_string()], 10)
        .unwrap();
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].sequence, 100);
    let payload = exportedPayload(&operations[0]);
    assert_eq!(payload.messageRows.len(), 1);
    assert_eq!(payload.messageRows[0].content, "token-100");
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
        sourceDatabase
            .messageDao()
            .updateMessageContent(messageId, format!("chunk-{index}"))
            .unwrap();
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
    assert_eq!(message.content, "chunk-50");
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
    assert_eq!(message.content, "new");
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
        database
            .messageDao()
            .updateMessageContent(messageId, format!("stress-token-{index}"))
            .unwrap();
        syncStore.recordMessageSnapshot(chatId, timestamp).unwrap();
    }

    assert_eq!(sqlOperationCount(&database), 1);
    let operations = syncStore
        .operationsSince(&SyncClock::empty(), &[CHAT_SYNC_DOMAIN.to_string()], 10)
        .unwrap();
    assert_eq!(operations.len(), 1);
    let payload = exportedPayload(&operations[0]);
    assert_eq!(payload.messageRows[0].content, "stress-token-1000");
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
        messageIds.push((timestamp, messageId));
    }

    for round in 1..=updateRounds {
        if round % 50 == 0 {
            eprintln!("sql sync ultra stress: recording round {round}/{updateRounds}");
        }
        for (messageIndex, (timestamp, messageId)) in messageIds.iter().enumerate() {
            sourceDatabase
                .messageDao()
                .updateMessageContent(*messageId, format!("message-{messageIndex}-round-{round}"))
                .unwrap();
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
            message.content,
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
        messageIds.push((timestamp, messageId));
    }

    for round in 1..=updateRounds {
        for (messageIndex, (timestamp, messageId)) in messageIds.iter().enumerate() {
            sourceDatabase
                .messageDao()
                .updateMessageContent(*messageId, format!("message-{messageIndex}-round-{round}"))
                .unwrap();
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
            message.content,
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
