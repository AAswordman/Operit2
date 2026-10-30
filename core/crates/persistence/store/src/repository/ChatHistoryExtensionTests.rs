use super::*;
use crate::sync::SqlChatSyncStore::tests::{openTestStore, DATABASE_MUTEX};
use operit_model::ChatMessagePresentation::{ChatMessagePresentation, PRESENTATION_EXTENSION};
use operit_model::MessagePart::MessagePart;
use operit_model::PluginExtensionTarget::PluginExtensionTarget;
use serde_json::{json, Value};

/// Opens an actual host-backed SQLite manager with a base message and independent variants.
fn fixture(name: &str) -> (ChatHistoryManager, Arc<AppDatabase>) {
    let (paths, database, _) = openTestStore(name);
    let manager = ChatHistoryManager::create(paths).unwrap();
    let chat = ChatEntity::new("extensions-chat".to_string(), "Extensions".to_string(), 1);
    database.chatDao().insertChat(chat).unwrap();
    let message =
        ChatMessage::new_with_markdown_timestamp("ai".to_string(), "base".to_string(), 42);
    manager
        .addMessage("extensions-chat".to_string(), message)
        .unwrap();
    for index in 1..=3 {
        let mut variant = ChatMessage::new_with_markdown_timestamp(
            "ai".to_string(),
            format!("variant-{index}"),
            42,
        );
        variant
            .pluginExtensions
            .insert("actor".to_string(), json!({"revision": index}));
        manager
            .addMessageVariant("extensions-chat".to_string(), 42, variant)
            .unwrap();
    }
    (manager, database)
}

/// Selects an exact persisted conversation row for extension operations.
fn chatTarget() -> PluginExtensionTarget {
    PluginExtensionTarget::Chat {
        chatId: "extensions-chat".to_string(),
    }
}

/// Selects an exact persisted message revision without implicit selected-state resolution.
fn messageTarget(index: i32) -> PluginExtensionTarget {
    PluginExtensionTarget::Message {
        chatId: "extensions-chat".to_string(),
        messageTimestamp: 42,
        variantIndex: index,
    }
}

/// Verifies simultaneous owner updates preserve both namespaces on every real record kind.
#[test]
fn concurrent_owners_preserve_each_other_and_record_sync() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, _) = fixture("extensions-concurrent");
    for target in [chatTarget(), messageTarget(0), messageTarget(2)] {
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let mut threads = Vec::new();
        for owner in ["alpha", "beta"] {
            let manager = manager.clone();
            let target = target.clone();
            let barrier = barrier.clone();
            threads.push(std::thread::spawn(move || {
                barrier.wait();
                manager
                    .writePluginExtension(owner, &target, json!({"owner": owner}))
                    .unwrap();
            }));
        }
        barrier.wait();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(
            manager.readPluginExtension("alpha", &target).unwrap(),
            Some(json!({"owner":"alpha"}))
        );
        assert_eq!(
            manager.readPluginExtension("beta", &target).unwrap(),
            Some(json!({"owner":"beta"}))
        );
    }
    let ops = manager
        .syncStore
        .operationsSince(&crate::SyncOperationStore::SyncClock::default(), &[], 100)
        .unwrap();
    assert!(!ops.is_empty());
    let payload: crate::sync::SqlChatSyncStore::ChatSyncPayload =
        serde_json::from_value(ops.last().unwrap().payload.clone()).unwrap();
    let row = payload
        .variantRows
        .iter()
        .find(|row| row.variantIndex == 2)
        .unwrap();
    assert!(row.pluginExtensions.contains_key("alpha"));
    assert!(row.pluginExtensions.contains_key("beta"));
}

/// Ensures stale history, ordinary metadata, parts and assistant commits cannot erase persisted owner updates.
#[test]
fn normal_edits_and_stale_snapshots_preserve_live_namespaces() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database) = fixture("extensions-stale-save");
    let mut stale = manager
        .loadChatHistory("extensions-chat".to_string())
        .unwrap()
        .unwrap();
    let staleMessage = manager
        .loadChatMessageVariant("extensions-chat", 42, 0)
        .unwrap();
    stale.messages = vec![staleMessage.clone()];
    for target in [chatTarget(), messageTarget(0), messageTarget(2)] {
        manager
            .writePluginExtension("alpha", &target, json!({"generation":7}))
            .unwrap();
        manager
            .writePluginExtension("beta", &target, json!({"generation":9}))
            .unwrap();
    }
    manager
        .updateChatTitle("extensions-chat".to_string(), "Retitled".to_string())
        .unwrap();
    manager
        .updateChatPinned("extensions-chat".to_string(), true)
        .unwrap();
    database
        .chatDao()
        .updateChatWorkspaceId("extensions-chat", Some("workspace-test".to_string()), 10)
        .unwrap();
    let mut edit = staleMessage.clone();
    edit.replace_with_markdown("edited".to_string());
    manager
        .updateMessage("extensions-chat".to_string(), edit.clone())
        .unwrap();
    manager
        .commitAssistantMessageSegment("extensions-chat".to_string(), edit, None)
        .unwrap();
    manager.saveChatHistory(stale).unwrap();
    assert_eq!(
        manager
            .loadChatMessageVariant("extensions-chat", 42, 2)
            .unwrap()
            .displayText(),
        "variant-2"
    );
    for target in [chatTarget(), messageTarget(0), messageTarget(2)] {
        assert_eq!(
            manager.readPluginExtension("alpha", &target).unwrap(),
            Some(json!({"generation":7}))
        );
        assert_eq!(
            manager.readPluginExtension("beta", &target).unwrap(),
            Some(json!({"generation":9}))
        );
    }
}

/// Ensures base promotion, alternate deletion, renumbering and hydration use each real variant's complete map.
#[test]
fn promotion_reordering_and_projection_follow_variant_snapshots() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, _) = fixture("extensions-variant-promotion");
    manager
        .writePluginExtension("base-only", &messageTarget(0), json!({"voice":"old"}))
        .unwrap();
    manager
        .writePluginExtension("new-only", &messageTarget(1), json!({"voice":"new"}))
        .unwrap();
    let promoted = manager.readPluginExtensions(&messageTarget(1)).unwrap();
    let last = manager.readPluginExtensions(&messageTarget(3)).unwrap();
    manager
        .selectMessageVariant("extensions-chat".to_string(), 42, 1)
        .unwrap();
    assert_eq!(
        manager.loadChatMessages("extensions-chat").unwrap()[0].pluginExtensions,
        promoted
    );
    manager
        .deleteMessageVariant("extensions-chat".to_string(), 42, 0)
        .unwrap();
    assert_eq!(
        manager.readPluginExtensions(&messageTarget(0)).unwrap(),
        promoted
    );
    assert!(manager
        .readPluginExtension("base-only", &messageTarget(0))
        .unwrap()
        .is_none());
    assert_eq!(
        manager.readPluginExtensions(&messageTarget(2)).unwrap(),
        last
    );
    manager
        .deleteMessageVariant("extensions-chat".to_string(), 42, 1)
        .unwrap();
    assert_eq!(
        manager.readPluginExtensions(&messageTarget(1)).unwrap(),
        last
    );
    manager
        .selectMessageVariant("extensions-chat".to_string(), 42, 1)
        .unwrap();
    assert_eq!(
        manager.loadChatMessages("extensions-chat").unwrap()[0].pluginExtensions,
        last
    );
}

/// Rejects malformed JSON, missing records, invalid targets and nonobject writes without silent metadata loss.
#[test]
fn strict_errors_and_owner_delete_affect_only_the_exact_record() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database) = fixture("extensions-validation");
    let target = messageTarget(2);
    assert!(manager
        .readPluginExtension("missing", &target)
        .unwrap()
        .is_none());
    assert!(!manager.deletePluginExtension("missing", &target).unwrap());
    for value in [Value::Null, json!(1), json!([]), json!("text")] {
        assert!(manager
            .writePluginExtension("alpha", &target, value)
            .is_err());
    }
    assert!(manager.readPluginExtensions(&messageTarget(99)).is_err());
    assert!(manager.readPluginExtensions(&messageTarget(-1)).is_err());
    assert!(serde_json::from_value::<PluginExtensionTarget>(
        json!({"kind":"message","chatId":"extensions-chat","messageTimestamp":42})
    )
    .is_err());
    manager
        .writePluginExtension("alpha", &target, json!({"keep":"no"}))
        .unwrap();
    manager
        .writePluginExtension("beta", &target, json!({"keep":"yes"}))
        .unwrap();
    assert!(manager.deletePluginExtension("alpha", &target).unwrap());
    assert_eq!(
        manager.readPluginExtension("beta", &target).unwrap(),
        Some(json!({"keep":"yes"}))
    );
    let operations = manager
        .syncStore
        .operationsSince(&crate::SyncOperationStore::SyncClock::default(), &[], 100)
        .unwrap();
    let payload: crate::sync::SqlChatSyncStore::ChatSyncPayload =
        serde_json::from_value(operations.last().unwrap().payload.clone()).unwrap();
    let record = payload
        .variantRows
        .iter()
        .find(|row| row.variantIndex == 2)
        .unwrap();
    assert!(!record.pluginExtensions.contains_key("alpha"));
    assert_eq!(record.pluginExtensions["beta"], json!({"keep":"yes"}));
    for bad in ["{broken", "[]", "{\"alpha\":12}"] {
        database
            .store()
            .execute(
                "UPDATE messages SET pluginExtensions = ?1 WHERE chatId = ?2 AND timestamp = ?3",
                sqliteParams![bad, "extensions-chat", 42],
            )
            .unwrap();
        assert!(manager.readPluginExtensions(&messageTarget(0)).is_err());
        assert!(manager
            .writePluginExtension("alpha", &messageTarget(0), json!({}))
            .is_err());
    }
}

/// Verifies branch cloning, JSON archives and sync replay preserve all independently persisted maps.
#[test]
fn clone_archive_and_sync_preserve_complete_record_extensions() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, _) = fixture("extensions-roundtrip");
    manager
        .writePluginExtension("alpha", &chatTarget(), json!({"opaque":"conversation"}))
        .unwrap();
    manager
        .writePluginExtension("alpha", &messageTarget(0), json!({"opaque":"base"}))
        .unwrap();
    manager
        .selectMessageVariant("extensions-chat".to_string(), 42, 2)
        .unwrap();
    let branch = manager
        .newChatDraft(
            "Branch".to_string(),
            BTreeMap::from([("alpha".to_string(), json!({"opaque":"hook-owned-new"}))]),
            None,
            Some("extensions-chat".to_string()),
        )
        .unwrap();
    let branch = manager
        .commitChatDraft(branch, Some(("extensions-chat", Some(42))))
        .unwrap();
    assert_eq!(
        branch.pluginExtensions["alpha"],
        json!({"opaque":"hook-owned-new"})
    );
    for index in 0..=3 {
        let source = manager
            .loadChatMessageVariant("extensions-chat", 42, index)
            .unwrap();
        let cloned = manager
            .loadChatMessageVariant(&branch.id, 42, index)
            .unwrap();
        assert_eq!(cloned.pluginExtensions, source.pluginExtensions);
    }
    let archiveText = manager.exportChatHistoriesToJson().unwrap();
    let archive: OperitChatArchive = serde_json::from_str(&archiveText).unwrap();
    let original = archive
        .chats
        .iter()
        .find(|chat| chat.id == "extensions-chat")
        .unwrap()
        .clone();
    assert_eq!(
        original.pluginExtensions["alpha"],
        json!({"opaque":"conversation"})
    );
    assert_eq!(
        original.messages[0].baseMessage.pluginExtensions["alpha"],
        json!({"opaque":"base"})
    );
    assert_eq!(
        original.messages[0].variants[1].pluginExtensions["actor"],
        json!({"revision":2})
    );
    let mut imported = original.clone();
    imported.id = "imported-chat".to_string();
    manager.saveArchivedChat(imported, true).unwrap();
    for index in 0..=3 {
        assert_eq!(
            manager
                .loadChatMessageVariant("imported-chat", 42, index)
                .unwrap()
                .pluginExtensions,
            manager
                .loadChatMessageVariant("extensions-chat", 42, index)
                .unwrap()
                .pluginExtensions
        );
    }
    let operations = manager
        .syncStore
        .operationsSince(&crate::SyncOperationStore::SyncClock::default(), &[], 100)
        .unwrap();
    let (_, replica, replay) = openTestStore("extensions-replica");
    for operation in operations {
        replay.applyOperation(&operation).unwrap();
    }
    assert_eq!(
        replica
            .chatDao()
            .getChatById("extensions-chat")
            .unwrap()
            .unwrap()
            .pluginExtensions,
        original.pluginExtensions
    );
    assert_eq!(
        replica
            .messageVariantDao()
            .getVariantForMessage("extensions-chat", 42, 2)
            .unwrap()
            .unwrap()
            .pluginExtensions,
        original.messages[0].variants[1].pluginExtensions
    );
}

/// Rejects duplicate draft identity and invalid clone points without persisting opening rows or partial chats.
#[test]
fn draft_transaction_failure_leaves_no_chat_or_opening_record() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database) = fixture("extensions-draft-rollback");
    let mut draft = manager
        .newChatDraft("Unpublished".to_string(), BTreeMap::new(), None, None)
        .unwrap();
    draft.messages.push(ChatMessage::new_with_markdown(
        "ai".to_string(),
        "opening".to_string(),
    ));
    let id = draft.id.clone();
    assert!(manager
        .commitChatDraft(draft, Some(("extensions-chat", Some(999))))
        .is_err());
    assert!(database.chatDao().getChatById(&id).unwrap().is_none());
    assert!(database
        .messageDao()
        .getMessagesForChat(&id)
        .unwrap()
        .is_empty());
}

/// Verifies executing conversations and dirty revisions reject owner writes/deletes while reads and completed history remain usable.
#[test]
fn execution_lease_guards_direct_store_metadata_mutations() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, _) = fixture("extensions-execution-lease");
    manager
        .writePluginExtension("alpha", &chatTarget(), json!({"identity":"original"}))
        .unwrap();
    let lease = manager.beginChatExecution("extensions-chat").unwrap();
    lease.protectRevision(42, 2).unwrap();
    assert_eq!(
        manager.readPluginExtension("alpha", &chatTarget()).unwrap(),
        Some(json!({"identity":"original"}))
    );
    assert!(manager
        .writePluginExtension("alpha", &chatTarget(), json!({"identity":"changed"}))
        .is_err());
    assert!(manager
        .deletePluginExtension("alpha", &chatTarget())
        .is_err());
    assert!(manager
        .writePluginExtension("alpha", &messageTarget(2), json!({"voice":"changed"}))
        .is_err());
    assert!(manager
        .deletePluginExtension("actor", &messageTarget(2))
        .is_err());
    assert!(manager
        .syncStore
        .mutatePluginExtension("alpha", &chatTarget(), Some(json!({})))
        .is_err());
    manager
        .writePluginExtension("alpha", &messageTarget(1), json!({"completed":true}))
        .unwrap();
    assert!(manager
        .deletePluginExtension("alpha", &messageTarget(1))
        .unwrap());
    assert!(manager.beginChatExecution("extensions-chat").is_err());
    lease.release().unwrap();
    manager
        .writePluginExtension("alpha", &chatTarget(), json!({"identity":"after-complete"}))
        .unwrap();
    manager
        .writePluginExtension("alpha", &messageTarget(2), json!({"after-complete":true}))
        .unwrap();
}

/// Ensures cancellation/error release and stale callback drops cannot unlock a subsequent generation's identity.
#[test]
fn lease_drop_and_idempotent_cancel_preserve_new_generation_guard() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, _) = fixture("extensions-execution-drop");
    {
        let _failedRequest = manager.beginChatExecution("extensions-chat").unwrap();
    }
    manager
        .writePluginExtension("alpha", &chatTarget(), json!({"error-released":true}))
        .unwrap();
    let cancelled = manager.beginChatExecution("extensions-chat").unwrap();
    let staleCallback = cancelled.clone();
    cancelled.release().unwrap();
    let current = manager.beginChatExecution("extensions-chat").unwrap();
    drop(staleCallback);
    drop(cancelled);
    assert!(manager
        .writePluginExtension("alpha", &chatTarget(), json!({"must-not-write":true}))
        .is_err());
    current.release().unwrap();
    current.release().unwrap();
    assert!(manager
        .deletePluginExtension("alpha", &chatTarget())
        .unwrap());
}

/// Releases early request errors even when the streaming runtime already retained a cloned lease.
#[test]
fn early_request_failure_does_not_leak_a_runtime_clone_lease() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, _) = fixture("extensions-request-handoff");
    let runtimeLease = manager.beginChatExecution("extensions-chat").unwrap();
    {
        let _failedRequest = runtimeLease.requestGuard();
    }
    manager
        .writePluginExtension("alpha", &chatTarget(), json!({"released":true}))
        .unwrap();
    let next = manager.beginChatExecution("extensions-chat").unwrap();
    let mut successful = next.requestGuard();
    successful.handoff();
    drop(successful);
    drop(runtimeLease);
    assert!(manager
        .deletePluginExtension("alpha", &chatTarget())
        .is_err());
    next.release().unwrap();
    assert!(manager
        .deletePluginExtension("alpha", &chatTarget())
        .unwrap());
}

/// Persists delivered presentation snapshots while retaining live plugin namespaces and variants.
#[test]
fn assistant_presentation_survives_reload_and_final_commit() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database) = fixture("extensions-assistant-presentation");
    let mut message = manager
        .loadChatMessageVariant("extensions-chat", 42, 0)
        .unwrap();
    message.pluginExtensions.insert(
        PRESENTATION_EXTENSION.into(),
        json!({"bubbles": [], "pending": true}),
    );
    manager
        .commitAssistantMessageSegment("extensions-chat".into(), message.clone(), None)
        .unwrap();
    for target in [chatTarget(), messageTarget(0), messageTarget(2)] {
        manager
            .writePluginExtension("alpha", &target, json!({"generation": 7}))
            .unwrap();
        manager
            .writePluginExtension("beta", &target, json!({"generation": 9}))
            .unwrap();
    }
    let lease = manager.beginChatExecution("extensions-chat").unwrap();
    lease.protectRevision(42, 0).unwrap();
    let first = vec![MessagePart::markdown(
        "sentence-0".into(),
        0,
        "first。".into(),
    )];
    let second = vec![MessagePart::markdown(
        "sentence-1".into(),
        0,
        "second。".into(),
    )];
    message.replace_with_markdown("first。second。".into());
    let mut presentation = ChatMessagePresentation {
        bubbles: vec![first],
        pending: true,
    };
    message.pluginExtensions.insert(
        PRESENTATION_EXTENSION.into(),
        serde_json::to_value(&presentation).unwrap(),
    );
    manager
        .commitAssistantMessageSegment("extensions-chat".into(), message.clone(), None)
        .unwrap();
    assert_eq!(
        database
            .messageDao()
            .getMessageByTimestamp("extensions-chat", 42)
            .unwrap()
            .unwrap()
            .pluginExtensions[PRESENTATION_EXTENSION],
        serde_json::to_value(&presentation).unwrap()
    );
    assert!(manager
        .writePluginExtension("alpha", &messageTarget(0), json!({"generation": 8}))
        .is_err());
    presentation.bubbles.push(second);
    presentation.pending = false;
    message.pluginExtensions.insert(
        PRESENTATION_EXTENSION.into(),
        serde_json::to_value(&presentation).unwrap(),
    );
    manager
        .commitAssistantMessageSegment("extensions-chat".into(), message, Some((10, 20, 30)))
        .unwrap();
    let reloaded = manager
        .loadChatMessageVariant("extensions-chat", 42, 0)
        .unwrap();
    assert_eq!(reloaded.displayText(), "first。second。");
    assert_eq!(
        serde_json::from_value::<ChatMessagePresentation>(
            reloaded.pluginExtensions[PRESENTATION_EXTENSION].clone()
        )
        .unwrap(),
        presentation
    );
    assert_eq!(
        manager
            .loadChatMessageVariant("extensions-chat", 42, 2)
            .unwrap()
            .displayText(),
        "variant-2"
    );
    for target in [chatTarget(), messageTarget(0), messageTarget(2)] {
        assert_eq!(
            manager.readPluginExtension("alpha", &target).unwrap(),
            Some(json!({"generation": 7}))
        );
        assert_eq!(
            manager.readPluginExtension("beta", &target).unwrap(),
            Some(json!({"generation": 9}))
        );
    }
    let operations = manager
        .syncStore
        .operationsSince(&crate::SyncOperationStore::SyncClock::default(), &[], 100)
        .unwrap();
    let payload: crate::sync::SqlChatSyncStore::ChatSyncPayload =
        serde_json::from_value(operations.last().unwrap().payload.clone()).unwrap();
    assert_eq!(
        payload.messageRows[0].pluginExtensions,
        reloaded.pluginExtensions
    );
    assert_eq!(payload.chatRows[0].inputTokens, 10);
    assert_eq!(payload.chatRows[0].outputTokens, 20);
    lease.release().unwrap();
}

/// Finalizes cancellation with only delivered bubbles and keeps the canonical partial response.
#[test]
fn cancelled_presentation_keeps_only_delivered_bubbles() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, _) = fixture("extensions-presentation-cancel");
    let mut message = manager
        .loadChatMessageVariant("extensions-chat", 42, 0)
        .unwrap();
    message.replace_with_markdown("first。second still being typed".into());
    let mut presentation = ChatMessagePresentation {
        bubbles: vec![vec![MessagePart::markdown(
            "sentence-0".into(),
            0,
            "first。".into(),
        )]],
        pending: true,
    };
    message.pluginExtensions.insert(
        PRESENTATION_EXTENSION.into(),
        serde_json::to_value(&presentation).unwrap(),
    );
    manager
        .commitAssistantMessageSegment("extensions-chat".into(), message.clone(), None)
        .unwrap();
    presentation.pending = false;
    message.pluginExtensions.insert(
        PRESENTATION_EXTENSION.into(),
        serde_json::to_value(&presentation).unwrap(),
    );
    manager
        .commitAssistantMessageSegment("extensions-chat".into(), message, None)
        .unwrap();
    let reloaded = manager
        .loadChatMessageVariant("extensions-chat", 42, 0)
        .unwrap();
    assert_eq!(reloaded.displayText(), "first。second still being typed");
    assert_eq!(
        reloaded.pluginExtensions[PRESENTATION_EXTENSION],
        serde_json::to_value(&presentation).unwrap()
    );
}

/// Rejects invalid native presentation updates before changing persisted parts or sync operations.
#[test]
fn invalid_assistant_presentation_does_not_mutate_history() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, _) = fixture("extensions-presentation-validation");
    let original = manager
        .loadChatMessageVariant("extensions-chat", 42, 0)
        .unwrap();
    let operations = manager
        .syncStore
        .operationsSince(&crate::SyncOperationStore::SyncClock::default(), &[], 100)
        .unwrap();
    let mut invalid = original.clone();
    invalid.replace_with_markdown("must not be saved".into());
    invalid.pluginExtensions.insert(
        PRESENTATION_EXTENSION.into(),
        json!({"bubbles": [], "pending": "invalid"}),
    );
    assert!(manager
        .commitAssistantMessageSegment("extensions-chat".into(), invalid, None)
        .is_err());
    assert_eq!(
        manager
            .loadChatMessageVariant("extensions-chat", 42, 0)
            .unwrap(),
        original
    );
    assert_eq!(
        manager
            .syncStore
            .operationsSince(&crate::SyncOperationStore::SyncClock::default(), &[], 100)
            .unwrap()
            .len(),
        operations.len()
    );
}

/// Locator titles use the saved author of the selected revision in both query paths.
#[test]
fn locator_previews_preserve_saved_participant_names() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, _) = fixture("locator-participant-names");
    manager.writePluginExtension("host.chat.identity", &messageTarget(0),
        json!({"title":"原始参与者","avatarUri":null})).unwrap();
    manager.writePluginExtension("host.chat.identity", &messageTarget(2),
        json!({"title":"另一个参与者","avatarUri":null})).unwrap();

    // Only recorded participant identities supply names; obsolete role names are ignored.
    for (timestamp, sender, roleName, title) in [
        (43, "user", "Old user", Some("小明")),
        (44, "ai", "废弃角色卡名字", None),
        (45, "ai", "", None),
        (46, "user", "废弃角色卡名字", None),
        (47, "ai", "废弃角色卡名字", Some("  ")),
    ] {
        let mut message = ChatMessage::new_with_markdown_timestamp(
            sender.to_string(), "locator-body".to_string(), timestamp);
        message.roleName = roleName.to_string();
        if let Some(title) = title {
            message.pluginExtensions.insert("host.chat.identity".into(),
                json!({"title":title,"avatarUri":null}));
        }
        manager.addMessage("extensions-chat".to_string(), message).unwrap();
    }
    for (index, query, expectedName) in [
        (0, "", "原始参与者"),
        (0, "base", "原始参与者"),
        (2, "", "另一个参与者"),
        (2, "variant-2", "另一个参与者"),
        (1, "variant-1", ""),
    ] {
        manager.selectMessageVariant("extensions-chat".to_string(), 42, index).unwrap();
        let previews = manager.loadChatMessageLocatorPreviews(
            "extensions-chat".to_string(), query.to_string()).unwrap();
        assert_eq!(previews[0].displayName, expectedName);
        assert_eq!(previews[0].timestamp, 42);
    }
    for query in ["", "locator-body"] {
        let previews = manager.loadChatMessageLocatorPreviews(
            "extensions-chat".to_string(), query.to_string()).unwrap();
        let names: Vec<_> = previews.iter().filter(|preview| preview.timestamp > 42)
            .map(|preview| preview.displayName.as_str()).collect();
        assert_eq!(names, ["小明", "", "", "", ""]);
    }
}
