//! Compaction of one chat must persist and refresh even when another chat is open.
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

/// Retains background tasks while delivering synchronous work immediately.
#[derive(Default)]
struct CrossChatScheduler(Mutex<Vec<HostRuntimeAsyncTask>>);

impl HostRuntimeTaskSchedulerHost for CrossChatScheduler {
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
        panic!("cross-chat compaction must not schedule timers")
    }
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture {
        Box::pin(async { Ok(()) })
    }
    fn waitForHostRuntimeDelay(&self, _: u64) -> HostRuntimeTurnFuture {
        panic!("cross-chat compaction must not wait for timers")
    }
}

/// A finished summary must persist for its origin chat after the user switches away,
/// and the origin window refresh must not disturb the open chat's history or totals.
#[test]
fn switched_away_summary_persists_and_refreshes_only_its_origin_chat() {
    let root = std::env::temp_dir().join(format!(
        "operit-summary-cross-chat-{}",
        uuid::Uuid::new_v4()
    ));
    let runtime = root.join("runtime");
    let workspaces = root.join("workspaces");
    std::fs::create_dir_all(&runtime).unwrap();

    let host = Arc::new(NativeRuntimeStorageHost::new(
        runtime.clone(),
        workspaces.clone(),
    ));
    setDefaultRuntimeStorageHost(host.clone());
    setDefaultRuntimeSqliteHost(host);
    setDefaultHostRuntimeTaskSchedulerHost(Arc::new(CrossChatScheduler::default()));
    let database = AppDatabase::getDatabase(RuntimeStorePaths::default()).unwrap();
    database
        .chatDao()
        .insertChat(ChatEntity::new("origin".into(), "Origin".into(), 1))
        .unwrap();
    database
        .chatDao()
        .insertChat(ChatEntity::new("current".into(), "Current".into(), 2))
        .unwrap();
    let mut delegate = ChatHistoryDelegate::new(ChatSelectionMode::LOCAL_ONLY);
    for (chat, timestamp, sender) in [
        ("origin", 10, "user"),
        ("origin", 20, "ai"),
        ("origin", 30, "user"),
        ("origin", 40, "ai"),
        ("current", 100, "user"),
        ("current", 110, "ai"),
    ] {
        delegate
            .chatHistoryManager
            .addMessage(
                chat.into(),
                ChatMessage::new_with_markdown_timestamp(
                    sender.into(),
                    format!("{sender} at {timestamp}"),
                    timestamp,
                ),
            )
            .unwrap();
    }
    // The user switches away while the origin chat's summary is still generating.
    delegate.currentChatIdFlow.set_value(Some("current".into()));
    let currentFlow = delegate.chatMessageFlowForChat("current".into());
    let currentBefore = currentFlow.value();
    assert_eq!(currentBefore.len(), 2);
    delegate
        .chatHistoryManager
        .updateChatContextWindow("current".into(), 4_321)
        .unwrap();

    // The exact persistence call generateAndStoreSummary makes for its captured request chat.
    let persisted = delegate
        .addSummaryMessage(
            ChatMessage::new_with_markdown_timestamp(
                "summary".into(),
                "compacted origin".into(),
                99_999,
            ),
            Some(20),
            Some(30),
            Some("origin".into()),
        )
        .expect("origin summary must persist after a chat switch");
    assert!(persisted.timestamp > 20 && persisted.timestamp < 30);
    assert_eq!(
        delegate
            .getRuntimeChatHistory("origin".into())
            .iter()
            .map(|message| message.sender.as_str())
            .collect::<Vec<_>>(),
        vec!["summary", "user", "ai"],
        "origin runtime context must compact around the persisted summary",
    );
    // The open chat keeps its own persisted history and live display flow untouched.
    assert_eq!(
        currentFlow.value(),
        currentBefore,
        "a background compaction must not reload the open chat's display flow",
    );
    let currentMessages = delegate
        .chatHistoryManager
        .loadChatMessagesRange("current".into(), "asc".into(), 0, 200)
        .unwrap();
    assert_eq!(currentMessages.len(), 2);
    assert!(currentMessages
        .iter()
        .all(|message| message.sender != "summary"));

    // The window refresh targets the origin row only, leaving the open chat's totals intact.
    let origin = delegate
        .chatHistoryManager
        .updateChatContextWindow("origin".into(), 15_088)
        .unwrap();
    assert_eq!(origin.currentWindowSize, 15_088);
    let current = delegate
        .chatHistoryManager
        .loadChatHistory("current".into())
        .unwrap()
        .unwrap();
    assert_eq!(current.currentWindowSize, 4_321);
    assert_eq!(
        delegate
            .chatHistoryManager
            .loadChatHistory("origin".into())
            .unwrap()
            .unwrap()
            .currentWindowSize,
        15_088,
    );
}
