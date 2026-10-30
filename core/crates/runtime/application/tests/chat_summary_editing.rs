use std::sync::{Arc, Mutex};

use operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost;
use operit_host_api::{
    HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    HostRuntimeTurnFuture,
};
use operit_host_native_filesystem::PosixFileSystemHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_model::{ChatEntity::ChatEntity, ChatMessage::ChatMessage};
use operit_runtime::services::core::ChatHistoryDelegate::ChatSelectionMode;
use operit_runtime::services::ChatServiceCore::ChatServiceCore;
use operit_store::db::AppDatabase::AppDatabase;
use operit_store::RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};

/// Retains plugin hooks while delivering preference notifications synchronously.
#[derive(Default)]
struct EditScheduler(Mutex<Vec<HostRuntimeAsyncTask>>);

impl HostRuntimeTaskSchedulerHost for EditScheduler {
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        Ok(0)
    }
    fn scheduleHostRuntimeTask(&self, _: &str, task: HostRuntimeTask) -> HostResult<()> {
        task();
        Ok(())
    }
    fn scheduleHostRuntimeAsyncTask(&self, _: &str, task: HostRuntimeAsyncTask) -> HostResult<()> {
        self.0.lock().unwrap().push(task);
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
        panic!("summary editing must not schedule timers")
    }
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture {
        Box::pin(async { Ok(()) })
    }
    fn waitForHostRuntimeDelay(&self, _: u64) -> HostRuntimeTurnFuture {
        panic!("summary editing must not wait for timers")
    }
}

/// Verifies summary edits reach durable parts without changing the message identity or metadata.
#[tokio::test]
async fn summary_memory_edits_are_persisted() {
    let root = std::env::temp_dir().join(format!("operit-summary-edit-{}", uuid::Uuid::new_v4()));
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
    setDefaultHostRuntimeTaskSchedulerHost(Arc::new(EditScheduler::default()));
    let database = AppDatabase::getDatabase(RuntimeStorePaths::new(runtime, workspaces)).unwrap();
    database
        .chatDao()
        .insertChat(ChatEntity::new("chat".into(), "Summary editing".into(), 1))
        .unwrap();
    let mut core = ChatServiceCore::new(
        ChatSelectionMode::FOLLOW_GLOBAL,
        Arc::new(PosixFileSystemHost::new()),
    );
    core.chatHistoryDelegate
        .openChatHistory("chat".into(), true)
        .unwrap();
    let mut summary =
        ChatMessage::new_with_markdown_timestamp("summary".into(), "Original memory".into(), 123);
    summary.inputTokens = 100;
    summary.outputTokens = 20;
    summary.completedAt = 124;
    core.chatHistoryDelegate
        .addMessageToChat(summary.clone(), Some("chat".into()));

    // Markdown containing protocol-like text must remain literal summary content.
    let edited = "Updated **memory**.\n\n<think>Remember this literally</think>";
    assert!(core.updateMessage("chat".into(), 123, edited.into()).await);
    let snapshot = core
        .chatHistoryDelegate
        .chatMessagesSnapshotForChat("chat".into());
    assert_eq!(snapshot.len(), 1);
    let mut expected = summary;
    expected.parts = vec![operit_model::MessagePart::MessagePart::markdown(
        "part-0".into(),
        0,
        edited.into(),
    )];
    assert_eq!(
        serde_json::to_value(&snapshot[0]).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );
    let parts = database
        .messagePartDao()
        .getPartsForMessage("chat", 123, 0)
        .unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].content, edited);
    let persisted = database
        .messageDao()
        .getMessageByTimestamp("chat", 123)
        .unwrap()
        .unwrap();
    assert_eq!(persisted.sender, "summary");
    assert_eq!(persisted.inputTokens, 100);
    assert_eq!(persisted.outputTokens, 20);
    assert_eq!(persisted.completedAt, 124);

    assert!(core.updateMessage("chat".into(), 123, String::new()).await);
    let parts = database
        .messagePartDao()
        .getPartsForMessage("chat", 123, 0)
        .unwrap();
    assert_eq!(parts.len(), 1);
    assert!(parts[0].content.is_empty());
    assert!(
        !core
            .updateMessage("chat".into(), 999, "Missing".into())
            .await
    );
}
