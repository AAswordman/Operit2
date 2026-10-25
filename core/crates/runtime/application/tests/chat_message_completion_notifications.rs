use std::sync::{Arc, Mutex};

use operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost;
use operit_host_api::{
    HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    HostRuntimeTurnFuture,
};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_model::{ChatEntity::ChatEntity, ChatMessage::ChatMessage};
use operit_runtime::services::core::ChatHistoryDelegate::{ChatHistoryDelegate, ChatSelectionMode};
use operit_store::db::AppDatabase::AppDatabase;
use operit_store::RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};

/// Retains scheduled work so the test can observe dispatch before any JS runtime is started.
#[derive(Default)]
struct NotificationScheduler {
    tasks: Mutex<Vec<(String, HostRuntimeAsyncTask)>>,
}

impl HostRuntimeTaskSchedulerHost for NotificationScheduler {
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        Ok(0)
    }
    fn scheduleHostRuntimeTask(&self, _: &str, task: HostRuntimeTask) -> HostResult<()> {
        task();
        Ok(())
    }
    fn scheduleHostRuntimeAsyncTask(
        &self,
        name: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        self.tasks.lock().unwrap().push((name.to_string(), task));
        Ok(())
    }
    fn scheduleHostRuntimeCooperativeAsyncTask(
        &self,
        name: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        self.scheduleHostRuntimeAsyncTask(name, task)
    }
    fn scheduleDelayedHostRuntimeTask(
        &self,
        _: &str,
        _: u64,
        _: HostRuntimeTask,
    ) -> HostResult<()> {
        panic!("message persistence must not schedule delayed work")
    }
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture {
        Box::pin(async { Ok(()) })
    }
    fn waitForHostRuntimeDelay(&self, _: u64) -> HostRuntimeTurnFuture {
        panic!("message persistence must not wait for a timer")
    }
}

/// Exercises the real placeholder, checkpoint and terminal commit paths in an isolated process.
#[test]
fn streaming_checkpoints_persist_without_scheduling_message_hooks() {
    let root = std::env::temp_dir().join(format!(
        "operit-message-completion-{}",
        uuid::Uuid::new_v4()
    ));
    let runtime = root.join("runtime");
    let workspaces = root.join("workspaces");
    std::fs::create_dir_all(&runtime).unwrap();
    setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(
        runtime.clone(),
        workspaces.clone(),
    ));
    let host = Arc::new(NativeRuntimeStorageHost::new(
        runtime.clone(),
        workspaces.clone(),
    ));
    setDefaultRuntimeStorageHost(host.clone());
    setDefaultRuntimeSqliteHost(host);
    let scheduler = Arc::new(NotificationScheduler::default());
    setDefaultHostRuntimeTaskSchedulerHost(scheduler.clone());
    let database = AppDatabase::getDatabase(RuntimeStorePaths::new(runtime, workspaces)).unwrap();
    database
        .chatDao()
        .insertChat(ChatEntity::new("chat".into(), "Completion test".into(), 1))
        .unwrap();
    let mut history = ChatHistoryDelegate::new(ChatSelectionMode::FOLLOW_GLOBAL);
    let placeholder = ChatMessage::new_with_markdown_timestamp("ai".into(), String::new(), 123);
    history.addMessageToChat(placeholder.clone(), Some("chat".into()));
    let snapshot = ChatMessage::new_with_markdown_timestamp("ai".into(), "x".repeat(4610), 123);
    for _ in 0..100 {
        history.addMessageToChat(snapshot.clone(), Some("chat".into()));
    }
    let parts = database
        .messagePartDao()
        .getPartsForMessage("chat", 123, 0)
        .unwrap();
    assert_eq!(parts[0].content, "x".repeat(4610));
    assert_eq!(
        database
            .messageDao()
            .getMessageByTimestamp("chat", 123)
            .unwrap()
            .unwrap()
            .completedAt,
        0
    );
    assert!(
        scheduler.tasks.lock().unwrap().is_empty(),
        "placeholders and checkpoints must not enter plugin notification scheduling"
    );

    let completed = ChatMessage {
        completedAt: 456,
        ..snapshot
    };
    history
        .commitAssistantMessageSegment("chat".into(), completed, None)
        .unwrap();
    assert_eq!(
        database
            .messageDao()
            .getMessageByTimestamp("chat", 123)
            .unwrap()
            .unwrap()
            .completedAt,
        456
    );
    let tasks = scheduler.tasks.lock().unwrap();
    assert_eq!(
        tasks.len(),
        1,
        "the terminal commit must schedule its notification"
    );
    assert_eq!(tasks[0].0, "operit-toolpkg-notifications");
    drop(tasks);
    AppDatabase::closeDatabase();
    std::fs::remove_dir_all(root).unwrap();
}
