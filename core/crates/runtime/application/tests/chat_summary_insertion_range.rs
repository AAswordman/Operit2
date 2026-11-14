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
use operit_providers::chat::llmprovider::AIService::SendMessageRequest;
use operit_providers::chat::llmprovider::OpenAIProvider::OpenAIProvider;
use operit_providers::chat::llmprovider::ThinkingDialect::RequestThinking;
use operit_providers::runtime_support::ChatConfigurationResult;
use operit_runtime::core::chat::AIMessageManager::AIMessageManager;
use operit_runtime::services::core::ChatHistoryDelegate::{ChatHistoryDelegate, ChatSelectionMode};
use operit_store::db::AppDatabase::AppDatabase;
use operit_store::RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

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

/// Verifies Kotlin's distinct summary-source and model-context boundaries against persisted history.
#[test]
fn summary_history_matches_kotlin_for_insertion_and_runtime_requests() {
    let root = std::env::temp_dir().join(format!("operit-summary-range-{}", uuid::Uuid::new_v4()));
    let runtime = root.join("runtime");
    let workspaces = root.join("workspaces");
    std::fs::create_dir_all(&runtime).unwrap();

    let host = Arc::new(NativeRuntimeStorageHost::new(
        runtime.clone(),
        workspaces.clone(),
    ));
    setDefaultRuntimeStorageHost(host.clone());
    setDefaultRuntimeSqliteHost(host);
    setDefaultHostRuntimeTaskSchedulerHost(Arc::new(SummaryRangeScheduler::default()));
    let database = AppDatabase::getDatabase(RuntimeStorePaths::default()).unwrap();
    database
        .chatDao()
        .insertChat(ChatEntity::new("chat".into(), "Summary range".into(), 1))
        .unwrap();
    database
        .chatDao()
        .insertChat(ChatEntity::new("other-chat".into(), "Other".into(), 2))
        .unwrap();
    let mut delegate = ChatHistoryDelegate::new(ChatSelectionMode::LOCAL_ONLY);
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
    let original_history = serde_json::to_value(delegate.chatHistoryManager.loadChatMessagesRange("chat".into(), "asc".into(), 0, 79).unwrap()).unwrap();

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
        serde_json::to_value(delegate.chatHistoryManager.loadChatMessagesRange("chat".into(), "asc".into(), 0, 79).unwrap()).unwrap(),
        original_history,
        "summary range loading must leave all persisted messages unchanged",
    );

    for (through, expected) in [
        (20, vec![10, 20]),
        (30, vec![30]),
        (50, vec![30, 40, 50]),
        (60, vec![60]),
        (80, vec![60, 70, 80]),
    ] {
        let history = delegate.getRuntimeChatHistoryUpTo("chat".into(), through);
        assert_eq!(
            history
                .iter()
                .map(|message| message.timestamp)
                .collect::<Vec<_>>(),
            expected,
            "regeneration must include the summary that existed at {through}",
        );
    }
    assert_eq!(
        delegate
            .getRuntimeChatHistory("chat".into())
            .iter()
            .map(|message| message.timestamp)
            .collect::<Vec<_>>(),
        vec![60, 70, 80],
        "normal sends must load the same compacted context as Kotlin",
    );
    assert!(!delegate.hasUserMessage("other-chat".into()));
    assert_compaction_request_boundaries(&mut delegate, &database);
}

/// Exercises the reported 400-message case through persistence, runtime loading and provider JSON.
fn assert_compaction_request_boundaries(
    delegate: &mut ChatHistoryDelegate,
    database: &AppDatabase,
) {
    database
        .chatDao()
        .insertChat(ChatEntity::new(
            "long-chat".into(),
            "Long history".into(),
            3,
        ))
        .unwrap();
    for index in 1..=400 {
        let sender = if index % 2 == 1 { "user" } else { "ai" };
        delegate
            .chatHistoryManager
            .addMessage(
                "long-chat".into(),
                ChatMessage::new_with_markdown_timestamp(
                    sender.into(),
                    format!("original-message-{index:03}"),
                    index * 10,
                ),
            )
            .unwrap();
    }
    let first = delegate
        .addSummaryMessage(
            ChatMessage::new_with_markdown_timestamp(
                "summary".into(),
                "checkpoint-one".into(),
                99_999,
            ),
            Some(3900),
            Some(3910),
            Some("long-chat".into()),
        )
        .unwrap();
    assert!(first.timestamp > 3900 && first.timestamp < 3910);

    let context = delegate.getRuntimeChatHistory("long-chat".into());
    assert_eq!(
        context.len(),
        11,
        "400 originals must become a summary plus the latest ten messages"
    );
    let request = provider_request(context);
    let encoded = request.to_string();
    assert!(encoded.contains("checkpoint-one"));
    for index in 1..=400 {
        assert_eq!(
            encoded.contains(&format!("original-message-{index:03}")),
            index > 390
        );
    }

    // A later summary must not change regeneration of a reply before its boundary.
    delegate
        .addSummaryMessage(
            ChatMessage::new_with_markdown_timestamp(
                "summary".into(),
                "checkpoint-two".into(),
                99_999,
            ),
            Some(4000),
            None,
            Some("long-chat".into()),
        )
        .unwrap();
    assert_eq!(delegate.getRuntimeChatHistory("long-chat".into()).len(), 1);
    assert!(
        delegate.hasUserMessage("long-chat".into()),
        "first-turn detection must see older users even when only a summary remains in context"
    );
    delegate
        .chatHistoryManager
        .addMessage(
            "long-chat".into(),
            ChatMessage::new_with_markdown_timestamp("user".into(), "next-question".into(), 4010),
        )
        .unwrap();
    let current = delegate.getRuntimeChatHistory("long-chat".into());
    assert_eq!(current.len(), 2);
    let encoded = provider_request(current).to_string();
    assert!(encoded.contains("checkpoint-two"));
    assert!(encoded.contains("next-question"));
    assert!(!encoded.contains("checkpoint-one"));
    assert!(!encoded.contains("original-message-"));

    let historical = delegate.getRuntimeChatHistoryUpTo("long-chat".into(), 4000);
    assert_eq!(historical.len(), 11);
    let encoded = provider_request(historical).to_string();
    assert!(encoded.contains("checkpoint-one"));
    assert!(!encoded.contains("checkpoint-two"));
    assert!(!encoded.contains("next-question"));
    assert!(!encoded.contains("original-message-390"));
    assert!(encoded.contains("original-message-391"));

    let next_summary_source = delegate.getRuntimeChatHistory("long-chat".into());
    assert_eq!(next_summary_source[0].sender, "summary");
    assert_eq!(next_summary_source[0].displayText(), "checkpoint-two");
    assert_eq!(
        delegate.chatHistoryManager.getMessageCountsForChats(&["long-chat".into()]).unwrap()["long-chat"],
        403,
        "compaction must retain all original messages for display and editing"
    );
    assert_display_window_boundaries(delegate);
}

/// Reveals distant targets with bounded hydration and leaves missing targets unchanged.
fn assert_display_window_boundaries(delegate: &mut ChatHistoryDelegate) {
    delegate.currentChatIdFlow.set_value(Some("long-chat".into()));
    let latest = delegate.loadLatestCurrentChatDisplayWindow();
    assert_eq!(latest.len(), 20);
    assert_eq!(latest.last().unwrap().timestamp, 4010);
    assert!(delegate.hasOlderDisplayHistory);
    assert!(!delegate.hasNewerDisplayHistory);

    assert!(delegate.revealMessageForCurrentChat(1000));
    let window = delegate.currentChatMessagesSnapshot();
    assert_eq!(window.len(), 20);
    assert_eq!(window.first().unwrap().timestamp, 910);
    assert_eq!(window[9].timestamp, 1000);
    assert_eq!(window.last().unwrap().timestamp, 1100);
    assert!(delegate.hasOlderDisplayHistory);
    assert!(delegate.hasNewerDisplayHistory);
    assert!(!delegate.revealMessageForCurrentChat(1005));
    assert_eq!(delegate.currentChatMessagesSnapshot(), window);
    assert!(!delegate.isLoadingDisplayWindow);

    assert!(delegate.loadOlderMessagesForCurrentChat());
    let older = delegate.currentChatMessagesSnapshot();
    assert_eq!(older.len(), 40);
    assert_eq!(older.first().unwrap().timestamp, 710);
    assert_eq!(older.last().unwrap().timestamp, 1100);

    assert!(delegate.loadNewerMessagesForCurrentChat());
    let newer = delegate.currentChatMessagesSnapshot();
    assert_eq!(newer.len(), 60);
    assert_eq!(newer.first().unwrap().timestamp, 710);
    assert_eq!(newer.last().unwrap().timestamp, 1300);

    assert!(delegate.revealMessageForCurrentChat(10));
    let first = delegate.currentChatMessagesSnapshot();
    assert_eq!(first.len(), 11);
    assert_eq!(first.first().unwrap().timestamp, 10);
    assert!(!delegate.hasOlderDisplayHistory);
    assert!(delegate.hasNewerDisplayHistory);

    assert!(delegate.revealMessageForCurrentChat(4010));
    assert_eq!(delegate.currentChatMessagesSnapshot().len(), 10);
    assert!(delegate.hasOlderDisplayHistory);
    assert!(!delegate.hasNewerDisplayHistory);
}

/// Encodes the real outgoing provider body from the same runtime projection used by chat sends.
fn provider_request(history: Vec<ChatMessage>) -> serde_json::Value {
    let profile = serde_json::json!({
        "id":"model:chat", "name":"Operit", "avatarUri":null, "introPrompt":"",
        "userPreferencesText":"", "openingStatement":"", "ttsConfigId":"voice",
        "modelBinding":{"providerId":"provider", "modelId":"model"},
        "toolAccess":{"enabled":false, "allowedBuiltinTools":[], "allowedPackages":[], "allowedSkills":[], "allowedMcpServers":[]},
        "resources":[]
    });
    let configuration: ChatConfigurationResult = serde_json::from_value(serde_json::json!({
        "contextKey":"context", "historyPolicy":{"isolateParticipants":false, "labelUserMessages":false},
        "profile":profile, "participants":[profile], "executionData":{"version":1}
    })).unwrap();
    let history = AIMessageManager::getMemoryFromMessages(history, &configuration).unwrap();
    OpenAIProvider::new(
        "https://model.invalid/chat/completions".into(),
        "fixture".into(),
        "model".into(),
        "OPENAI".into(),
        Vec::new(),
        false,
    )
    .create_request_body(&SendMessageRequest {
        chat_history: history,
        model_parameters: Vec::new(),
        thinking: RequestThinking::default(),
        stream: false,
        available_tools: Vec::new(),
        preserve_think_in_history: false,
        enable_retry: false,
        on_non_fatal_error: None,
        on_tool_invocation: None,
    })
    .unwrap()
}
