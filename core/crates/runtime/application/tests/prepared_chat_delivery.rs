use operit_host_api::HostManager::HostManager;
use operit_host_native_filesystem::PosixFileSystemHost;
use operit_host_native_scheduler::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost,
};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_model::ChatEntity::ChatEntity;
use operit_plugin_sdk::js_sdk::chat::ChatPreparedMessageRequest;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::services::PreparedChatDeliveryService::PreparedChatDeliveryService;
use operit_store::db::AppDatabase::AppDatabase;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use std::sync::Arc;

/// Runs in its own test process because runtime storage defaults are process-global.
#[test]
fn prepared_delivery_persists_an_assistant_once_without_switching_chats() {
    // Native runtime globals retain SQLite readers until process exit; the parent owns cleanup.
    let root = match std::env::var_os("OPERIT_PREPARED_TEST_ROOT") {
        Some(root) => std::path::PathBuf::from(root),
        None => {
            let root = std::env::temp_dir().join(format!("operit-prepared-chat-{}", uuid::Uuid::new_v4()));
            let result = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "prepared_delivery_persists_an_assistant_once_without_switching_chats"])
                .env("OPERIT_PREPARED_TEST_ROOT", &root).output().unwrap();
            assert!(result.status.success(), "{}\n{}", String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
            assert!(root.is_absolute() && root.starts_with(std::env::temp_dir()));
            std::fs::remove_dir_all(&root).unwrap();
            return;
        }
    };
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.join("runtime"),
        root.join("workspaces"),
    ));
    let mut host = HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
    host.runtimeStorageHost = Some(storage.clone());
    host.runtimeSqliteHost = Some(storage);
    host.hostRuntimeTaskSchedulerHost = Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    host.hostJavaScriptRuntimeHost = Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    let application = OperitApplication::newWithContext(host);
    let database = AppDatabase::getDatabase(RuntimeStorePaths::default()).unwrap();
    database
        .chatDao()
        .insertChat(ChatEntity::new(
            "prepared-target".into(),
            "Target".into(),
            1,
        ))
        .unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let before = application
            .chatRuntimeHolder
            .lock()
            .await
            .getCore(ChatRuntimeSlot::MAIN)
            .chatHistoryDelegate
            .currentChatIdFlow
            .value();
        let request = ChatPreparedMessageRequest {
            chatId: "prepared-target".into(),
            text: "Hello from a prepared message".into(),
            timestamp: 123,
            notify: false,
        };
        let deliver = |request| {
            PreparedChatDeliveryService::deliver(
                &application.hostManager,
                &application.chatRuntimeHolder,
                request,
            )
        };
        let first = deliver(request.clone()).await.unwrap();
        assert!(!first.duplicate);
        let again = deliver(request.clone()).await.unwrap();
        assert!(again.duplicate);
        let mut holder = application.chatRuntimeHolder.lock().await;
        let core = holder.getCore(ChatRuntimeSlot::MAIN);
        assert_eq!(core.chatHistoryDelegate.currentChatIdFlow.value(), before);
        let messages = core
            .chatHistoryDelegate
            .getRuntimeChatHistory("prepared-target".into());
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].sender, "ai");
        assert_eq!(messages[0].displayText(), "Hello from a prepared message");
        assert!(messages[0].completedAt > 0);
        // Plain-text deliveries carry the same scheduling tag as generated replies so the
        // host can identify every delivery uniformly; preparedAt records the commit time.
        assert_eq!(
            messages[0].pluginExtensions.get("host.chat.scheduled"),
            Some(&serde_json::json!({ "preparedAt": messages[0].completedAt }))
        );
        // Genuine generation measurements and immutable author metadata survive delayed persistence.
        let mut snapshot = operit_model::ChatMessage::ChatMessage::new_with_markdown_timestamp("ai".into(), "Contextual reply".into(), 125);
        snapshot.roleName = "Tea companion".into();
        snapshot.modelName = "role-model".into();
        snapshot.provider = "role-provider".into();
        snapshot.inputTokens = 321;
        snapshot.outputTokens = 24;
        snapshot.cachedInputTokens = 200;
        snapshot.waitDurationMs = 50;
        snapshot.outputDurationMs = 120;
        snapshot.sentAt = 10;
        snapshot.completedAt = 180;
        snapshot.pluginExtensions.insert("host.chat.identity".into(), serde_json::json!({"title":"Tea companion","avatarUri":"/virtual/avatar.png"}));
        snapshot.pluginExtensions.insert("com.operit.character_cards".into(), serde_json::json!({"participantId":"tea-role"}));
        // The generation path stamps its own preparedAt before snapshot commit; the snapshot
        // commit must keep that tag untouched instead of re-stamping it with the commit time.
        snapshot.pluginExtensions.insert("host.chat.scheduled".into(), serde_json::json!({ "preparedAt": 180 }));
        assert!(!core.chatHistoryDelegate.commitPreparedAssistantMessageSnapshot("prepared-target", snapshot.clone()).unwrap());
        assert!(core.chatHistoryDelegate.commitPreparedAssistantMessageSnapshot("prepared-target", snapshot.clone()).unwrap());
        let mut persisted = core.chatHistoryDelegate.chatHistoryManager.loadChatMessageVariant("prepared-target", 125, 0).unwrap();
        // The repository adds canonical VFS attachment locators independently of generation metadata.
        for part in &mut persisted.parts { part.attributes.clear(); }
        assert_eq!(persisted, snapshot);
        // A later summary hides older turns from model context, but must not erase
        // the canonical delivery receipt used by a retry after a restart.
        core.chatHistoryDelegate.addMessageToChat(
            operit_model::ChatMessage::ChatMessage::new_with_markdown_timestamp(
                "summary".into(),
                "A later summary".into(),
                126,
            ),
            Some("prepared-target".into()),
        );
        assert_eq!(
            core.chatHistoryDelegate
                .getRuntimeChatHistory("prepared-target".into())
                .len(),
            1
        );
        drop(holder);
        assert!(deliver(request.clone()).await.unwrap().duplicate);
        // An idempotent retry must not rewrite the committed delivery or its scheduling tag.
        let mut holder = application.chatRuntimeHolder.lock().await;
        let retried = holder
            .getCore(ChatRuntimeSlot::MAIN)
            .chatHistoryDelegate
            .chatHistoryManager
            .loadChatMessageVariant("prepared-target", 123, 0)
            .unwrap();
        assert_eq!(
            retried.pluginExtensions.get("host.chat.scheduled"),
            Some(&serde_json::json!({ "preparedAt": retried.completedAt }))
        );
        drop(holder);
        let mut conflict = request.clone();
        conflict.text = "Conflicting replacement".into();
        assert!(deliver(conflict).await.unwrap_err().contains("conflicts"));
        let mut missing = request.clone();
        missing.chatId = "deleted-chat".into();
        assert!(deliver(missing).await.is_err());
        let mut future = request.clone();
        future.timestamp = i64::MAX;
        assert!(deliver(future).await.is_err());
        let mut blank = request;
        blank.text = " ".into();
        assert!(deliver(blank).await.is_err());
        use operit_plugin_sdk::js_sdk::chat::ChatScheduledMessageRequest;
        let support = application.toolHandler.runtimeSupport();
        let scheduled = ChatScheduledMessageRequest {
            id: "future-slot".into(),
            chatId: "prepared-target".into(),
            title: "Title".into(),
            text: "Scheduled hello".into(),
            generation: None,
            timestamp: operit_host_api::TimeUtils::currentTimeMillis() + 60000,
        };
        let first = support
            .scheduleChatMessage("plugin-a", scheduled.clone())
            .await
            .unwrap();
        assert_eq!(first.message.status, "scheduled");
        assert_eq!(
            support
                .scheduleChatMessage("plugin-a", scheduled.clone())
                .await
                .unwrap()
                .message
                .id,
            "future-slot"
        );
        let mut conflict = scheduled.clone();
        conflict.text = "Different content".into();
        assert!(support
            .scheduleChatMessage("plugin-a", conflict)
            .await
            .is_err());
        assert!(support
            .listScheduledChatMessages("plugin-b")
            .await
            .unwrap()
            .messages
            .is_empty());
        assert_eq!(
            support
                .listScheduledChatMessages("plugin-a")
                .await
                .unwrap()
                .messages
                .len(),
            1
        );
        support
            .cancelScheduledChatMessage("plugin-b", "future-slot".into())
            .await
            .unwrap();
        assert_eq!(
            support
                .listScheduledChatMessages("plugin-a")
                .await
                .unwrap()
                .messages[0]
                .status,
            "scheduled"
        );
        support
            .cancelScheduledChatMessage("plugin-a", "future-slot".into())
            .await
            .unwrap();
        assert_eq!(
            support
                .listScheduledChatMessages("plugin-a")
                .await
                .unwrap()
                .messages[0]
                .status,
            "cancelled"
        );

        // A fresh service reads persisted plans without a plugin running, then commits the due message itself.
        let mut restored = scheduled.clone();
        restored.id = "restored-slot".into();
        support
            .scheduleChatMessage("plugin-a", restored)
            .await
            .unwrap();
        let storage = application.hostManager.runtimeStorageHost.as_ref().unwrap();
        let mut ledger: serde_json::Value = serde_json::from_slice(
            &storage
                .readBytes("runtime/scheduled_chat_messages.json")
                .unwrap(),
        )
        .unwrap();
        for entry in ledger.as_array_mut().unwrap() {
            if entry["request"]["id"] == "restored-slot" {
                entry["request"]["timestamp"] = 500.into();
            }
        }
        storage
            .writeBytesAtomically(
                "runtime/scheduled_chat_messages.json",
                &serde_json::to_vec(&ledger).unwrap(),
            )
            .unwrap();
        let restored_service = PreparedChatDeliveryService::default();
        restored_service
            .reconcile(&application.hostManager, &application.chatRuntimeHolder)
            .await
            .unwrap();
        let receipts = restored_service
            .list(&application.hostManager, "plugin-a")
            .await
            .unwrap();
        // Extremely stale non-iOS plans are skipped by the host to avoid delivering a burst on startup.
        assert_eq!(
            receipts
                .messages
                .iter()
                .find(|message| message.id == "restored-slot")
                .unwrap()
                .status,
            "skipped"
        );
        // Move a new plan just into the past, within its delivery window, without waiting on a wall clock timer.
        let mut next = scheduled;
        next.id = "due-slot".into();
        support.scheduleChatMessage("plugin-a", next).await.unwrap();
        let mut ledger: serde_json::Value = serde_json::from_slice(
            &storage
                .readBytes("runtime/scheduled_chat_messages.json")
                .unwrap(),
        )
        .unwrap();
        let due_at = operit_host_api::TimeUtils::currentTimeMillis() - 1000;
        for entry in ledger.as_array_mut().unwrap() {
            if entry["request"]["id"] == "due-slot" {
                entry["request"]["timestamp"] = due_at.into();
            }
        }
        storage
            .writeBytesAtomically(
                "runtime/scheduled_chat_messages.json",
                &serde_json::to_vec(&ledger).unwrap(),
            )
            .unwrap();
        restored_service
            .reconcile(&application.hostManager, &application.chatRuntimeHolder)
            .await
            .unwrap();
        restored_service
            .reconcile(&application.hostManager, &application.chatRuntimeHolder)
            .await
            .unwrap();
        let receipt = restored_service
            .list(&application.hostManager, "plugin-a")
            .await
            .unwrap();
        assert_eq!(
            receipt
                .messages
                .iter()
                .find(|message| message.id == "due-slot")
                .unwrap()
                .status,
            "delivered"
        );
        let holder = application.chatRuntimeHolder.lock().await;
        let manager = &holder
            .cores
            .values()
            .next()
            .unwrap()
            .chatHistoryDelegate
            .chatHistoryManager;
        let committed = manager
            .loadChatMessageVariant("prepared-target", due_at, 0)
            .unwrap();
        assert_eq!(committed.sender, "ai");
        assert_eq!(committed.displayText(), "Scheduled hello");
        // Reconciler-driven plain-text deliveries are tagged exactly like direct deliveries.
        assert_eq!(
            committed.pluginExtensions.get("host.chat.scheduled"),
            Some(&serde_json::json!({ "preparedAt": committed.completedAt }))
        );
        drop(holder);

        // A restored prepared reply must be withdrawn when its participant cannot be resolved.
        let stale = ChatScheduledMessageRequest {
            id: "unresolved-participant".into(), chatId: "prepared-target".into(),
            title: "Title".into(), text: "Obsolete reply".into(), generation: None,
            timestamp: operit_host_api::TimeUtils::currentTimeMillis() + 60000,
        };
        support.scheduleChatMessage("plugin-a", stale.clone()).await.unwrap();
        snapshot.timestamp = stale.timestamp;
        let mut ledger: serde_json::Value = serde_json::from_slice(&storage.readBytes("runtime/scheduled_chat_messages.json").unwrap()).unwrap();
        let entry = ledger.as_array_mut().unwrap().iter_mut().find(|entry| entry["request"]["id"] == stale.id).unwrap();
        entry["prepared"] = serde_json::json!({ "message": snapshot, "historyFingerprint": "old-history", "configurationFingerprint": "old-participant" });
        storage.writeBytesAtomically("runtime/scheduled_chat_messages.json", &serde_json::to_vec(&ledger).unwrap()).unwrap();
        // A fresh reconciler checks future appointments immediately, even without the plugin polling.
        let restored_service = PreparedChatDeliveryService::default();
        restored_service.reconcile(&application.hostManager, &application.chatRuntimeHolder).await.unwrap();
        let receipts = restored_service.list(&application.hostManager, "plugin-a").await.unwrap();
        let receipt = receipts.messages.iter().find(|receipt| receipt.id == stale.id).unwrap();
        assert_eq!(receipt.status, "invalidated");
    });
    drop(application);
    AppDatabase::closeDatabase();
    drop(database);
}
