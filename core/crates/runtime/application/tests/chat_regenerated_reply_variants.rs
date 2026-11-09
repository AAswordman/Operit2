use std::sync::{Arc, Mutex};

use operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost;
use operit_host_api::{
    HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    HostRuntimeTurnFuture,
};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_link::{CoreEventStream, CoreStream, CoreStreamSource};
use operit_model::{ChatEntity::ChatEntity, ChatMessage::ChatMessage};
use operit_runtime::services::core::ChatHistoryDelegate::{ChatHistoryDelegate, ChatSelectionMode};
use operit_store::db::AppDatabase::AppDatabase;
use operit_store::RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_util::MarkdownRenderStream::MarkdownStreamEvent;
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};

/// Retains scheduled work so the test can observe dispatch before any JS runtime is started.
#[derive(Default)]
struct VariantScheduler {
    tasks: Mutex<Vec<(String, HostRuntimeAsyncTask)>>,
}

impl HostRuntimeTaskSchedulerHost for VariantScheduler {
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

/// Reproduces TestFlight replies becoming identical or refusing to switch after regeneration.
#[test]
fn regenerated_replies_switch_and_delete_without_reusing_another_revision_stream() {
    let root =
        std::env::temp_dir().join(format!("operit-feedback-variants-{}", uuid::Uuid::new_v4()));
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
    setDefaultHostRuntimeTaskSchedulerHost(Arc::new(VariantScheduler::default()));
    let database = AppDatabase::getDatabase(RuntimeStorePaths::new(runtime, workspaces)).unwrap();
    database
        .chatDao()
        .insertChat(ChatEntity::new(
            "chat".into(),
            "Feedback variants".into(),
            1,
        ))
        .unwrap();
    let mut history = ChatHistoryDelegate::new(ChatSelectionMode::LOCAL_ONLY);
    history.currentChatIdFlow.set_value(Some("chat".into()));
    let original = reply("original reply", 123, 100);
    history
        .chatHistoryManager
        .addMessage("chat".into(), original.clone())
        .unwrap();
    let unrelated = reply("other active reply", 456, 0);
    history
        .chatHistoryManager
        .addMessage("chat".into(), unrelated.clone())
        .unwrap();
    let unrelated = with_stream(unrelated, "other-active-stream");

    // Completed replies may still carry a replayable stream in the open transcript.
    history.setCurrentChatMessagesInMemory(
        vec![
            with_stream(original.clone(), "original-stream"),
            unrelated.clone(),
        ],
        None,
        None,
    );
    let regenerated = reply("first regenerated reply", 123, 200);
    assert_eq!(
        history.addMessageVariant(123, regenerated.clone(), Some("chat".into())),
        1
    );
    let committed = history.currentChatMessagesSnapshot()[0].clone();

    // Switching must discard the selected revision's stream as well as its text.
    let mut streamed_revision = regenerated;
    streamed_revision.selectedVariantIndex = 1;
    streamed_revision.variantCount = 2;
    history.setCurrentChatMessagesInMemory(
        vec![
            with_stream(streamed_revision, "regenerated-stream"),
            unrelated.clone(),
        ],
        None,
        None,
    );
    history.selectMessageVariant(123, 0);
    let selected_original = history.currentChatMessagesSnapshot()[0].clone();
    assert_eq!(
        (committed.displayText(), selected_original.displayText()),
        ("first regenerated reply".into(), "original reply".into()),
        "newly committed and selected replies must display their own persisted content"
    );
    assert_eq!(committed.completedAt, 200);
    assert!(committed.contentStream.is_none());
    assert!(selected_original.contentStream.is_none());
    assert_eq!(
        history.currentChatMessagesSnapshot()[1].contentStream,
        unrelated.contentStream
    );

    let mut preview = reply("second reply still streaming", 123, 0);
    preview.selectedVariantIndex = 2;
    preview.variantCount = 3;
    preview.isVariantPreview = true;
    history.setCurrentChatMessagesInMemory(
        vec![
            with_stream(preview, "second-preview-stream"),
            unrelated.clone(),
        ],
        None,
        None,
    );
    assert_eq!(
        history.addMessageVariant(
            123,
            reply("second regenerated reply", 123, 300),
            Some("chat".into())
        ),
        2
    );
    let completed_preview = &history.currentChatMessagesSnapshot()[0];
    assert_eq!(completed_preview.displayText(), "second regenerated reply");
    assert_eq!(completed_preview.completedAt, 300);
    assert!(!completed_preview.isVariantPreview);
    assert!(completed_preview.contentStream.is_none());
    for (index, text) in [
        (0, "original reply"),
        (1, "first regenerated reply"),
        (2, "second regenerated reply"),
        (0, "original reply"),
    ] {
        history.selectMessageVariant(123, index);
        assert_eq!(history.currentChatMessagesSnapshot()[0].displayText(), text);
    }
    history.deleteMessageVariant(123, 0);
    assert_eq!(
        history.currentChatMessagesSnapshot()[0].displayText(),
        "first regenerated reply"
    );
    history.selectMessageVariant(123, 1);
    assert_eq!(
        history.currentChatMessagesSnapshot()[0].displayText(),
        "second regenerated reply"
    );
    history.deleteMessageVariant(123, 1);
    assert_eq!(
        history.currentChatMessagesSnapshot()[0].displayText(),
        "first regenerated reply"
    );
    history.reloadCurrentChatDisplayHistory("chat".into());
    let survivor = &history.currentChatMessagesSnapshot()[0];
    assert_eq!(survivor.variantCount, 1);
    assert_eq!(survivor.displayText(), "first regenerated reply");
    assert_eq!(
        history.currentChatMessagesSnapshot()[1].contentStream,
        unrelated.contentStream
    );
    AppDatabase::closeDatabase();
    std::fs::remove_dir_all(root).unwrap();
}

fn reply(text: &str, timestamp: i64, completed_at: i64) -> ChatMessage {
    ChatMessage {
        completedAt: completed_at,
        ..ChatMessage::new_with_markdown_timestamp("ai".into(), text.into(), timestamp)
    }
}

fn with_stream(mut message: ChatMessage, id: &str) -> ChatMessage {
    let source = Arc::new(CoreStreamSource::new(|_request| {
        let (_sender, receiver) = CoreEventStream::channel();
        Ok(receiver)
    }));
    message.contentStream = Some(CoreStream::<MarkdownStreamEvent>::fromSourceWithId(
        id.into(),
        source,
    ));
    message
}
