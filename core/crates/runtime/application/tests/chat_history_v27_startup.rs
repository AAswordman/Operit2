use std::fs;
use std::sync::Arc;

use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_model::PluginExtensionTarget::PluginExtensionTarget;
use operit_runtime::services::core::ChatHistoryDelegate::{ChatHistoryDelegate, ChatSelectionMode};
use operit_store::db::AppDatabase::AppDatabase;
use operit_store::PreferencesDataStore::{stringPreferencesKey, PreferencesDataStore};
use operit_store::RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};

const VERSION_27_FIXTURE: &str =
    include_str!("../../../persistence/store/src/db/fixtures/version27-chat-records.sql");

/// Starts the real history delegate over a genuine v27 database and nonempty persisted selection without any configuration owner.
#[test]
fn version27_selected_unbound_history_starts_and_remains_browsable() {
    let root = std::env::temp_dir().join(format!(
        "operit-history-v27-startup-{}",
        uuid::Uuid::new_v4()
    ));
    let runtimeRoot = root.join("runtime");
    let workspaceRoot = root.join("workspaces");
    fs::create_dir_all(&runtimeRoot).unwrap();
    fs::create_dir_all(&workspaceRoot).unwrap();
    setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(
        runtimeRoot.clone(),
        workspaceRoot.clone(),
    ));
    let host = Arc::new(NativeRuntimeStorageHost::new(
        runtimeRoot.clone(),
        workspaceRoot.clone(),
    ));
    setDefaultRuntimeStorageHost(host.clone());
    setDefaultRuntimeSqliteHost(host);
    let paths = RuntimeStorePaths::new(runtimeRoot, workspaceRoot);
    fs::create_dir_all(paths.sqlite_database_path().parent().unwrap()).unwrap();
    let old = rusqlite::Connection::open(paths.sqlite_database_path()).unwrap();
    old.execute_batch(VERSION_27_FIXTURE).unwrap();
    old.execute("INSERT INTO chats(id,title,createdAt,updatedAt,\"group\",characterCardName,characterGroupId) VALUES('v27-second','Other old history',300,400,'old-folder','old-card','old-role-group')",[]).unwrap();
    assert_eq!(
        old.query_row("PRAGMA user_version", [], |row| row.get::<_, i32>(0))
            .unwrap(),
        27
    );
    drop(old);
    let preferences = PreferencesDataStore::new(paths.current_chat_id_preferences_path());
    let currentKey = stringPreferencesKey("current_chat_id");
    let untouchedKey = stringPreferencesKey("startup-fixture-pref");
    preferences
        .edit(|values| {
            values.set(&currentKey, "v27-chat".to_string());
            values.set(&untouchedKey, "kept-user-setting".to_string());
        })
        .unwrap();
    let preferenceBytes = fs::read(paths.current_chat_id_preferences_path()).unwrap();

    // This integration test process installs no plugin API, hook or AI runtime; startup must only read real history.
    let mut delegate = ChatHistoryDelegate::new(ChatSelectionMode::FOLLOW_GLOBAL);
    delegate.initialize().unwrap();
    assert!(delegate.isInitialized);
    assert_eq!(
        delegate.currentChatIdFlow.value().as_deref(),
        Some("v27-chat")
    );
    assert!(delegate.chatConfigurationsFlow.value().is_empty());
    assert_eq!(
        fs::read(paths.current_chat_id_preferences_path()).unwrap(),
        preferenceBytes
    );
    let database = AppDatabase::getDatabase(paths.clone()).unwrap();
    assert_eq!(database.store().getUserVersion().unwrap(), 28);
    let history = delegate
        .chatHistoryManager
        .loadChatHistory("v27-chat".to_string())
        .unwrap()
        .unwrap();
    assert_eq!(history.title, "Kept title");
    assert_eq!(history.group.as_deref(), Some("legacy-sidebar"));
    assert_eq!(delegate.chatHistoryManager.loadChatHistory("v27-second".to_string()).unwrap().unwrap().group.as_deref(), Some("old-folder"));
    assert_eq!(history.workspaceId.as_deref(), Some("workspace-kept"));
    assert!(history.pluginExtensions.is_empty());
    let messages = delegate.currentChatMessagesSnapshot();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].timestamp, 42);
    assert_eq!(messages[0].selectedVariantIndex, 2);
    assert_eq!(messages[0].variantCount, 3);
    assert_eq!(messages[0].displayText(), "variant-two-kept");
    assert!(messages[0].pluginExtensions.is_empty());
    assert_eq!(
        delegate
            .chatHistoryManager
            .readPluginExtension(
                "unregistered-owner",
                &PluginExtensionTarget::Chat {
                    chatId: "v27-chat".to_string()
                }
            )
            .unwrap(),
        None
    );

    delegate.initialize().unwrap();
    assert_eq!(
        fs::read(paths.current_chat_id_preferences_path()).unwrap(),
        preferenceBytes
    );
    delegate
        .openChatHistory("v27-second".to_string(), false)
        .unwrap();
    assert_eq!(
        delegate.currentChatIdFlow.value().as_deref(),
        Some("v27-second")
    );
    assert!(delegate.currentChatMessagesSnapshot().is_empty());
    assert!(delegate.chatConfigurationsFlow.value().is_empty());
    assert_eq!(
        fs::read(paths.current_chat_id_preferences_path()).unwrap(),
        preferenceBytes
    );
    delegate
        .openChatHistory("v27-chat".to_string(), true)
        .unwrap();
    assert_eq!(
        delegate.currentChatMessagesSnapshot()[0].displayText(),
        "variant-two-kept"
    );
    assert_eq!(
        preferences
            .dataFlow()
            .first()
            .unwrap()
            .get(&currentKey)
            .map(String::as_str),
        Some("v27-chat")
    );
    assert_eq!(
        preferences
            .dataFlow()
            .first()
            .unwrap()
            .get(&untouchedKey)
            .map(String::as_str),
        Some("kept-user-setting")
    );
    let beforeInvalid = fs::read(paths.current_chat_id_preferences_path()).unwrap();
    assert_eq!(
        delegate
            .openChatHistory("missing-chat".to_string(), true)
            .unwrap_err(),
        "Chat does not exist: missing-chat"
    );
    assert_eq!(
        delegate.currentChatIdFlow.value().as_deref(),
        Some("v27-chat")
    );
    assert_eq!(
        fs::read(paths.current_chat_id_preferences_path()).unwrap(),
        beforeInvalid
    );
    assert!(!delegate
        .chatHistoryManager
        .chatExists("missing-chat".to_string())
        .unwrap());
    assert!(delegate.chatConfigurationsFlow.value().is_empty());
}
