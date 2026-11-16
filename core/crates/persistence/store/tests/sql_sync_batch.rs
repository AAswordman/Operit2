//! Received SQL pages commit atomically and notify observers after the committed batch.
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_model::ChatEntity::ChatEntity;
use operit_store::{
    db::AppDatabase::AppDatabase,
    CoreNodeIdentityStore::CoreNodeIdentityStore,
    RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost},
    RuntimeStorePaths::RuntimeStorePaths,
    SyncOperationStore::{SyncOperation, SyncOperationSemantics},
    sync::SqlChatSyncStore::{SqlChatSyncStore, ChatSyncPayload},
    Transaction,
};

fn operation(sequence: i64, chat: &str) -> SyncOperation {
    SyncOperation {
        opId: format!("batch-peer:{sequence}"), originDeviceId: "batch-peer".into(), sequence,
        domain: "chat".into(), entityType: "chat".into(), entityId: chat.into(),
        operation: "upsert".into(), semantics: SyncOperationSemantics::EntityState,
        payload: serde_json::to_value(ChatSyncPayload {
            chatRows: vec![ChatEntity::new(chat.into(), chat.into(), sequence)],
            ..Default::default()
        }).unwrap(), createdAt: sequence, schemaVersion: 8,
    }
}

#[test]
fn sql_batch_commits_once_replays_and_rolls_back_conflicts_with_initialization_barriers() {
    let root = std::env::temp_dir().join(format!("operit-sql-batch-{}", uuid::Uuid::new_v4()));
    let host = Arc::new(NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces")));
    setDefaultRuntimeStorageHost(host.clone());
    setDefaultRuntimeSqliteHost(host.clone());
    CoreNodeIdentityStore::new(host).writeNodeId("batch-local".into()).unwrap();
    let database = AppDatabase::getDatabase(RuntimeStorePaths::default()).unwrap();
    let sync = SqlChatSyncStore::new(RuntimeStorePaths::default(), &database).unwrap();
    use operit_store::SqliteStore::SqliteRowGet;
    let plan = database.store().queryRows("EXPLAIN QUERY PLAN SELECT createdAt,origin,sequence FROM sync_sql_versions WHERE chatId='first'", vec![]).unwrap();
    assert!(plan.iter().any(|row| row.get::<_, String>("detail").unwrap().contains("sync_sql_versions_chat")));
    let notifications = Arc::new(AtomicUsize::new(0));
    let observed_rows = Arc::new(AtomicUsize::new(0));
    let (count, rows) = (notifications.clone(), observed_rows.clone());
    database.store().addInvalidationObserver(move |store| {
        count.fetch_add(1, Ordering::SeqCst);
        rows.store(store.queryScalar::<i64>("SELECT COUNT(*) FROM chats", vec![])? as usize, Ordering::SeqCst);
        Ok(())
    }).unwrap();
    let first = operation(1, "first");
    let second = operation(2, "second");
    sync.applyOperations(&[first.clone(), second.clone()]).unwrap();
    assert_eq!(notifications.load(Ordering::SeqCst), 1);
    assert_eq!(observed_rows.load(Ordering::SeqCst), 2);
    sync.applyOperations(&[first.clone(), second.clone()]).unwrap();
    assert_eq!(notifications.load(Ordering::SeqCst), 1, "redelivery must not notify observers");
    let third = operation(3, "third");
    let mut conflict = first.clone();
    conflict.payload["chatRows"][0]["title"] = serde_json::json!("changed immutable identity");
    let before = sync.syncSnapshot().unwrap();
    assert!(sync.applyOperations(&[third.clone(), conflict]).is_err());
    assert_eq!(sync.syncSnapshot().unwrap(), before, "the prefix and receipt must roll back together");
    assert_eq!(notifications.load(Ordering::SeqCst), 1);
    let mut invalid = operation(4, "invalid");
    invalid.schemaVersion = -1;
    assert!(sync.applyOperations(&[third.clone(), invalid]).is_err());
    assert_eq!(sync.syncSnapshot().unwrap(), before);
    // Re-delivering initialization between incremental runs must not nest transactions.
    let initialization = Transaction::compaction::Initialization::from_snapshot(&before).operation("chat", false).unwrap();
    sync.applyOperations(&[first, initialization, third.clone()]).unwrap();
    assert!(sync.syncCoverage().unwrap().contains(&third.opId));
    assert_eq!(database.store().queryScalar::<i64>("SELECT COUNT(*) FROM chats", vec![]).unwrap(), 3);
    sync.applyOperations(&[third]).unwrap();
    drop(sync);
    drop(database);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn buffered_digest_preserves_json_bytes_for_small_large_and_escaped_values() {
    use sha2::{Digest, Sha256};
    for value in [serde_json::json!(null), serde_json::json!({"quoted":"\"\\\n中文","small":3}),
        serde_json::json!({"large":"abc\"\n中文".repeat(20000)})] {
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(Transaction::digest(&value).unwrap(), format!("{:x}", Sha256::digest(&bytes)));
    }
}
