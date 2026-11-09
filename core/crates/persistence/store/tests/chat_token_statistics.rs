//! Conversation accounting follows persisted assistant usage, including interrupted replies.
use std::sync::Arc;

use operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost;
use operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_model::{ChatEntity::ChatEntity, ChatMessage::ChatMessage};
use operit_store::{
    db::AppDatabase::AppDatabase,
    repository::ChatHistoryManager::ChatHistoryManager,
    sync::SqlChatSyncStore::{ChatSyncPayload, SqlChatSyncStore},
    CoreNodeIdentityStore::CoreNodeIdentityStore,
    RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost},
    RuntimeStorePaths::RuntimeStorePaths,
    SyncOperationStore::SyncClock,
};
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};

/// Exercises the repository and its persisted/synchronized totals through a real storage host.
#[test]
fn assistant_usage_updates_conversation_totals_once_with_or_without_a_window() {
    let root = std::env::temp_dir().join(format!("operit-chat-tokens-{}", uuid::Uuid::new_v4()));
    let runtime = root.join("runtime");
    let workspaces = root.join("workspaces");
    std::fs::create_dir_all(&runtime).unwrap();
    let paths = RuntimeStorePaths::new(runtime.clone(), workspaces.clone());
    setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(
        runtime.clone(),
        workspaces.clone(),
    ));
    let host = Arc::new(NativeRuntimeStorageHost::new(runtime, workspaces));
    setDefaultRuntimeStorageHost(host.clone());
    setDefaultRuntimeSqliteHost(host.clone());
    CoreNodeIdentityStore::new(host)
        .writeNodeId("token-statistics-source".into())
        .unwrap();
    setDefaultHostRuntimeTaskSchedulerHost(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    let database = AppDatabase::getDatabase(paths.clone()).unwrap();
    let manager = ChatHistoryManager::new(paths.clone()).unwrap();
    for id in ["chat", "other"] {
        database
            .chatDao()
            .insertChat(ChatEntity::new(id.into(), id.into(), 1))
            .unwrap();
    }
    manager
        .updateChatTokenCounts("chat".into(), 100, 20, 120)
        .unwrap();
    let assert_totals = |input, output, window| {
        let chat = manager.loadChatHistory("chat".into()).unwrap().unwrap();
        assert_eq!(
            (chat.inputTokens, chat.outputTokens, chat.currentWindowSize),
            (input, output, window)
        );
        let other = manager.loadChatHistory("other".into()).unwrap().unwrap();
        assert_eq!((other.inputTokens, other.outputTokens), (0, 0));
    };
    let mut interrupted =
        ChatMessage::new_with_markdown_timestamp("ai".into(), "partial".into(), 10);
    manager
        .addMessage("chat".into(), interrupted.clone())
        .unwrap();
    interrupted.inputTokens = 200;
    interrupted.outputTokens = 37;
    interrupted.cachedInputTokens = 80;
    interrupted.completedAt = 11;
    manager
        .commitAssistantMessageSegment("chat".into(), interrupted.clone(), None)
        .unwrap();
    assert_totals(300, 57, 120);
    // A repeated final/presentation commit must not charge the same reply twice.
    manager
        .commitAssistantMessageSegment("chat".into(), interrupted.clone(), None)
        .unwrap();
    assert_totals(300, 57, 120);

    // An asynchronous summary can change the window between terminal commits.
    manager.updateChatContextWindow("chat".into(), 40).unwrap();
    let mut failed =
        ChatMessage::new_with_markdown_timestamp("ai".into(), "failed partial".into(), 20);
    manager.addMessage("chat".into(), failed.clone()).unwrap();
    failed.inputTokens = 50;
    failed.outputTokens = 5;
    failed.completedAt = 21;
    manager
        .commitAssistantMessageSegment("chat".into(), failed, None)
        .unwrap();
    assert_totals(350, 62, 40);

    let mut completed =
        ChatMessage::new_with_markdown_timestamp("ai".into(), "complete".into(), 30);
    manager
        .addMessage("chat".into(), completed.clone())
        .unwrap();
    completed.inputTokens = 150;
    completed.outputTokens = 25;
    completed.completedAt = 31;
    manager
        .commitAssistantMessageSegment("chat".into(), completed.clone(), Some(210))
        .unwrap();
    assert_totals(500, 87, 210);
    // A later correction adds only the difference, preserving other replies' usage.
    interrupted.outputTokens = 40;
    manager
        .commitAssistantMessageSegment("chat".into(), interrupted, None)
        .unwrap();
    assert_totals(500, 90, 210);

    let sync = SqlChatSyncStore::new(paths.clone(), &database).unwrap();
    let operations = sync.operationsSince(&SyncClock::empty(), &[], 100).unwrap();
    let payload: ChatSyncPayload =
        serde_json::from_value(operations.last().unwrap().payload.clone()).unwrap();
    assert_eq!(
        (
            payload.chatRows[0].inputTokens,
            payload.chatRows[0].outputTokens
        ),
        (500, 90)
    );
    assert_eq!(
        (
            payload.messageRows[0].inputTokens,
            payload.messageRows[0].outputTokens
        ),
        (200, 40)
    );
    AppDatabase::closeDatabase();
    let reopened = AppDatabase::getDatabase(paths).unwrap();
    let chat = reopened.chatDao().getChatById("chat").unwrap().unwrap();
    assert_eq!(
        (chat.inputTokens, chat.outputTokens, chat.currentWindowSize),
        (500, 90, 210)
    );
    AppDatabase::closeDatabase();
    std::fs::remove_dir_all(root).unwrap();
}
