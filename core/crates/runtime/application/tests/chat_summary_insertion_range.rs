use std::sync::{Arc, Mutex};

use operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost;
use operit_host_api::{
    HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    HostRuntimeTurnFuture,
};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_model::{
    ChatEntity::ChatEntity, ChatMessage::ChatMessage,
    ChatMessageDisplayMode::ChatMessageDisplayMode,
};
use operit_runtime::services::core::ChatHistoryDelegate::{ChatHistoryDelegate, ChatSelectionMode};
use operit_store::RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_store::db::AppDatabase::AppDatabase;
use operit_util::RuntimeStoreRoot::{RuntimeStoreRootConfig, setDefaultRuntimeStoreRootConfig};

/// Retains plugin hooks while delivering preference notifications synchronously.
#[derive(Default)]
struct SummaryRangeScheduler(Mutex<Vec<HostRuntimeAsyncTask>>);

impl HostRuntimeTaskSchedulerHost for SummaryRangeScheduler {
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
        panic!("summary range loading must not schedule timers")
    }
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture {
        Box::pin(async { Ok(()) })
    }
    fn waitForHostRuntimeDelay(&self, _: u64) -> HostRuntimeTurnFuture {
        panic!("summary range loading must not wait for timers")
    }
}

/// Exercises the delegate used by long-press insertion against real persisted history.
#[test]
fn summary_insertion_respects_latest_summary_in_selected_range() {
    let root = std::env::temp_dir().join(format!("operit-summary-range-{}", uuid::Uuid::new_v4()));
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
    setDefaultHostRuntimeTaskSchedulerHost(Arc::new(SummaryRangeScheduler::default()));
    let database = AppDatabase::getDatabase(RuntimeStorePaths::new(runtime, workspaces)).unwrap();
    database
        .chatDao()
        .insertChat(ChatEntity::new("chat".into(), "Summary range".into(), 1))
        .unwrap();
    database
        .chatDao()
        .insertChat(ChatEntity::new("other-chat".into(), "Other".into(), 2))
        .unwrap();
    let delegate = ChatHistoryDelegate::new(ChatSelectionMode::LOCAL_ONLY);
    for (timestamp, sender) in [
        (10, "user"),
        (20, "ai"),
        (30, "summary"),
        (40, "user"),
        (45, "ai"),
        (50, "ai"),
        (60, "summary"),
        (70, "user"),
        (80, "ai"),
    ] {
        let mut message = ChatMessage::new_with_markdown_timestamp(
            sender.into(),
            format!("{sender} at {timestamp}"),
            timestamp,
        );
        if timestamp == 45 {
            message.displayMode = ChatMessageDisplayMode::HIDDEN_PLACEHOLDER;
        }
        delegate
            .chatHistoryManager
            .addMessage("chat".into(), message)
            .unwrap();
    }
    delegate
        .chatHistoryManager
        .addMessage(
            "other-chat".into(),
            ChatMessage::new_with_markdown_timestamp("summary".into(), "Other chat".into(), 75),
        )
        .unwrap();
    let original_history = serde_json::to_value(delegate.getChatHistory("chat".into())).unwrap();

    for (case, before_exclusive, up_to_inclusive, expected) in [
        ("AI before any summary", None, Some(20), vec![10, 20]),
        ("user before any summary", Some(10), None, vec![]),
        (
            "historical AI after first summary",
            None,
            Some(50),
            vec![40, 50],
        ),
        (
            "historical user after first summary",
            Some(40),
            None,
            vec![],
        ),
        ("exclusive boundary", Some(50), None, vec![40]),
        (
            "latest AI after second summary",
            None,
            Some(80),
            vec![70, 80],
        ),
        (
            "user immediately after second summary",
            Some(70),
            None,
            vec![],
        ),
        ("inclusive summary boundary", None, Some(60), vec![]),
        ("exclusive summary boundary", Some(60), None, vec![40, 50]),
        ("unbounded latest range", None, None, vec![70, 80]),
    ] {
        let loaded = delegate.loadMessagesForSummaryInsertion(
            "chat".into(),
            before_exclusive,
            up_to_inclusive,
        );
        assert_eq!(
            loaded
                .iter()
                .map(|message| message.timestamp)
                .collect::<Vec<_>>(),
            expected,
            "{case}",
        );
    }
    assert_eq!(
        serde_json::to_value(delegate.getChatHistory("chat".into())).unwrap(),
        original_history,
        "summary range loading must leave all persisted messages unchanged",
    );
}
