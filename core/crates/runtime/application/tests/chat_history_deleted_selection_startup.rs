use std::fs;
use std::sync::Arc;

use operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost;
use operit_host_api::{
    HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    HostRuntimeTurnFuture,
};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_model::ChatEntity::ChatEntity;
use operit_runtime::services::core::ChatHistoryDelegate::{ChatHistoryDelegate, ChatSelectionMode};
use operit_store::db::AppDatabase::{AppDatabase, DATABASE_VERSION};
use operit_store::PreferencesDataStore::{stringPreferencesKey, PreferencesDataStore};
use operit_store::RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_store::sync::SqlChatSyncStore::SqlChatSyncStore;
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};

/// Delivers preference observers without starting plugin tasks in this isolated startup test.
struct StartupScheduler;

impl HostRuntimeTaskSchedulerHost for StartupScheduler {
    /// Supplies a deterministic clock for preference notifications.
    fn monotonicTimeMillis(&self) -> HostResult<u64> { Ok(0) }
    /// Publishes committed preference changes synchronously.
    fn scheduleHostRuntimeTask(&self, _: &str, task: HostRuntimeTask) -> HostResult<()> {
        task();
        Ok(())
    }
    /// Rejects plugin work outside the startup history boundary.
    fn scheduleHostRuntimeAsyncTask(&self, _: &str, _: HostRuntimeAsyncTask) -> HostResult<()> {
        panic!("history selection restoration must not schedule plugin work")
    }
    /// Rejects cooperative plugin work outside the startup history boundary.
    fn scheduleHostRuntimeCooperativeAsyncTask(&self, _: &str, _: HostRuntimeAsyncTask) -> HostResult<()> {
        panic!("history selection restoration must not schedule plugin work")
    }
    /// Rejects timers outside the startup history boundary.
    fn scheduleDelayedHostRuntimeTask(&self, _: &str, _: u64, _: HostRuntimeTask) -> HostResult<()> {
        panic!("history selection restoration must not schedule timers")
    }
    /// Completes observer turns without waiting.
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture { Box::pin(async { Ok(()) }) }
    /// Rejects delays outside the startup history boundary.
    fn waitForHostRuntimeDelay(&self, _: u64) -> HostRuntimeTurnFuture {
        panic!("history selection restoration must not wait for timers")
    }
}

/// Restores startup after migration removes the selected legacy row, but preserves unexplained errors.
#[test]
fn deleted_selected_chat_is_cleared_after_upgrade_without_hiding_unknown_missing_chats() {
    let root = std::env::temp_dir().join(format!("operit-deleted-selection-{}", uuid::Uuid::new_v4()));
    let runtimeRoot = root.join("runtime");
    let workspaceRoot = root.join("workspaces");
    fs::create_dir_all(&runtimeRoot).unwrap();
    setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(runtimeRoot.clone(), workspaceRoot.clone()));
    let host = Arc::new(NativeRuntimeStorageHost::new(runtimeRoot.clone(), workspaceRoot.clone()));
    setDefaultRuntimeStorageHost(host.clone());
    setDefaultRuntimeSqliteHost(host);
    setDefaultHostRuntimeTaskSchedulerHost(Arc::new(StartupScheduler));
    let paths = RuntimeStorePaths::new(runtimeRoot, workspaceRoot);
    let database = AppDatabase::getDatabase(paths.clone()).unwrap();
    database.chatDao().insertChat(ChatEntity::new("deleted-chat".into(), "Old selection".into(), 1)).unwrap();
    database.chatDao().insertChat(ChatEntity::new("live-chat".into(), "Keep this".into(), 2)).unwrap();
    let sync = SqlChatSyncStore::new(paths.clone(), &database).unwrap();
    assert!(sync.deleteChat("deleted-chat").unwrap());
    // Recreate the exact pre-fix inconsistency: a retained deletion plus its resurrected business row.
    database.store().executeBatch("INSERT INTO chats(id,title,createdAt,updatedAt) VALUES('deleted-chat','Legacy resurrection',1,1); DROP INDEX index_sync_sql_deletions_chat_lifecycle; PRAGMA user_version=28;").unwrap();
    drop(sync);
    drop(database);
    AppDatabase::closeDatabase();

    let preferences = PreferencesDataStore::new(paths.current_chat_id_preferences_path());
    let currentKey = stringPreferencesKey("current_chat_id");
    let untouchedKey = stringPreferencesKey("startup-fixture-pref");
    preferences.edit(|values| {
        values.set(&currentKey, "deleted-chat".to_string());
        values.set(&untouchedKey, "keep-setting".to_string());
    }).unwrap();

    let journal = operit_store::SyncOperationStore::SyncOperationStore::native(paths.clone());
    let selectionHistory = || journal.preferenceHistory().unwrap().into_iter()
        .filter(|operation| operation.payload["storagePath"] == operit_util::RuntimeStorageLayout::CURRENT_CHAT_ID_PREFERENCES_PATH)
        .collect::<Vec<_>>();
    let originalSelectionHistory = selectionHistory();

    let mut delegate = ChatHistoryDelegate::new(ChatSelectionMode::FOLLOW_GLOBAL);
    delegate.initialize().unwrap();
    assert!(delegate.isInitialized);
    assert!(delegate.currentChatIdFlow.value().is_none());
    assert!(delegate.currentChatMessagesSnapshot().is_empty());
    assert!(delegate.chatHistoryManager.currentChatIdFlow().unwrap().is_none());
    assert!(delegate.chatHistoryManager.chatIsDeleted("deleted-chat").unwrap());
    assert!(!delegate.chatHistoryManager.chatExists("deleted-chat".into()).unwrap());
    assert!(delegate.chatHistoryManager.chatExists("live-chat".into()).unwrap());
    assert_eq!(AppDatabase::getDatabase(paths.clone()).unwrap().store().getUserVersion().unwrap(), DATABASE_VERSION);
    assert_eq!(preferences.dataFlow().first().unwrap().get(&untouchedKey), Some(&serde_json::json!("keep-setting")));
    assert_eq!(selectionHistory(), originalSelectionHistory);
    let clearedBytes = fs::read(paths.current_chat_id_preferences_path()).unwrap();
    let mut restarted = ChatHistoryDelegate::new(ChatSelectionMode::FOLLOW_GLOBAL);
    restarted.initialize().unwrap();
    assert_eq!(fs::read(paths.current_chat_id_preferences_path()).unwrap(), clearedBytes);

    delegate.chatHistoryManager.setCurrentChatId("unknown-chat".into()).unwrap();
    let invalidBytes = fs::read(paths.current_chat_id_preferences_path()).unwrap();
    let mut invalid = ChatHistoryDelegate::new(ChatSelectionMode::FOLLOW_GLOBAL);
    assert_eq!(invalid.initialize().unwrap_err(), "Chat does not exist: unknown-chat");
    assert!(!invalid.isInitialized);
    assert_eq!(fs::read(paths.current_chat_id_preferences_path()).unwrap(), invalidBytes);

    delegate.chatHistoryManager.setCurrentChatId("live-chat".into()).unwrap();
    let validBytes = fs::read(paths.current_chat_id_preferences_path()).unwrap();
    let mut valid = ChatHistoryDelegate::new(ChatSelectionMode::FOLLOW_GLOBAL);
    valid.initialize().unwrap();
    assert_eq!(valid.currentChatIdFlow.value().as_deref(), Some("live-chat"));
    assert_eq!(fs::read(paths.current_chat_id_preferences_path()).unwrap(), validBytes);
}
