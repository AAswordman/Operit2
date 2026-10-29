use super::*;

use crate::sync::SqlChatSyncStore::tests::{openTestStore, DATABASE_MUTEX};
use std::sync::mpsc;
use std::time::Duration;

const CHAT_ID: &str = "chat-variants";
const MESSAGE_TIMESTAMP: i64 = 123;

/// Keeps a completed reply's totals when an older asynchronous summary refreshes the window.
#[test]
fn summary_context_window_update_preserves_completed_reply_totals() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, _) = chatOrderFixture("summary-context-window-totals");
    manager
        .updateChatTokenCounts("older".to_string(), 100, 20, 120)
        .unwrap();
    let snapshot = manager
        .loadChatHistory("older".to_string())
        .unwrap()
        .unwrap();
    manager
        .updateChatTokenCounts("older".to_string(), 160, 50, 210)
        .unwrap();

    let compacted = manager
        .updateChatContextWindow("older".to_string(), 40)
        .unwrap();

    assert_eq!((snapshot.inputTokens, snapshot.outputTokens), (100, 20));
    assert_eq!((compacted.inputTokens, compacted.outputTokens), (160, 50));
    assert_eq!(compacted.currentWindowSize, 40);
    assert_eq!(compacted.title, snapshot.title);
    let persisted = manager
        .loadChatHistory("older".to_string())
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            persisted.inputTokens,
            persisted.outputTokens,
            persisted.currentWindowSize
        ),
        (160, 50, 40)
    );
    assert!(manager
        .updateChatContextWindow("missing".to_string(), 40)
        .is_err());
}

/// Opens a chat whose binding arrives before its workspace preference record.
fn workspaceMetadataFixture(name: &str) -> (ChatHistoryManager, Arc<AppDatabase>, Workspace) {
    crate::PreferencesDataStore::installPreferenceObserverTestHost();
    let (paths, database, _) = openTestStore(name);
    let workspace = Workspace {
        id: "workspace-metadata".to_string(),
        name: "Project".to_string(),
        folders: vec![WorkspaceFolder {
            name: "project".to_string(),
            path: "/app/workspaces/project".to_string(),
        }],
        createdAt: 1,
        updatedAt: 1,
    };
    let mut chat = ChatEntity::new(CHAT_ID.to_string(), "Workspace chat".to_string(), 1);
    chat.workspaceId = Some(workspace.id.clone());
    database.chatDao().insertChat(chat).unwrap();
    let manager = ChatHistoryManager::create(paths).unwrap();
    (manager, database, workspace)
}

/// Receives a real repository emission without triggering a chat-table refresh.
fn receiveWorkspaceChat(receiver: &mpsc::Receiver<Vec<ChatHistory>>) -> ChatHistory {
    let histories = receiver.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(histories.len(), 1);
    histories.into_iter().next().unwrap()
}

/// Refreshes bound workspace names and paths when preference records arrive or change alone.
#[test]
fn workspace_preferences_refresh_chat_metadata_without_sql_changes() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database, mut workspace) = workspaceMetadataFixture("workspace-metadata-arrival");
    let originalChat = database.chatDao().getChatById(CHAT_ID).unwrap().unwrap();
    let histories = manager.chatHistoriesFlow().unwrap();
    let (sender, receiver) = mpsc::channel();
    let subscription = histories.subscribe(move |value| sender.send(value).unwrap());

    let initial = receiveWorkspaceChat(&receiver);
    assert_eq!(initial.workspaceId.as_deref(), Some(workspace.id.as_str()));
    assert_eq!(initial.workspaceName, None);
    assert_eq!(initial.workspacePrimaryPath, None);

    manager.workspaceStore.upsert(workspace.clone()).unwrap();
    let arrived = receiveWorkspaceChat(&receiver);
    assert_eq!(arrived.workspaceName.as_deref(), Some("Project"));
    assert_eq!(
        arrived.workspacePrimaryPath.as_deref(),
        Some("/app/workspaces/project")
    );
    assert_eq!(arrived, manager.loadChatHistories().unwrap().remove(0));

    workspace.name = "Renamed project".to_string();
    workspace.folders[0].path = "/app/workspaces/renamed".to_string();
    workspace.updatedAt = 2;
    manager.workspaceStore.upsert(workspace).unwrap();
    let renamed = receiveWorkspaceChat(&receiver);
    assert_eq!(renamed.workspaceName.as_deref(), Some("Renamed project"));
    assert_eq!(
        renamed.workspacePrimaryPath.as_deref(),
        Some("/app/workspaces/renamed")
    );
    assert_eq!(renamed, manager.loadChatHistories().unwrap().remove(0));
    assert_eq!(
        database.chatDao().getChatById(CHAT_ID).unwrap().unwrap(),
        originalChat
    );
    histories.unsubscribe(subscription);
}

/// Keeps an unbound chat free of workspace metadata after later workspace preference changes.
#[test]
fn workspace_preferences_do_not_restore_an_unbound_chat() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database, mut workspace) = workspaceMetadataFixture("workspace-metadata-unbind");
    let histories = manager.chatHistoriesFlow().unwrap();
    let (sender, receiver) = mpsc::channel();
    let subscription = histories.subscribe(move |value| sender.send(value).unwrap());
    receiveWorkspaceChat(&receiver);
    manager.workspaceStore.upsert(workspace.clone()).unwrap();
    receiveWorkspaceChat(&receiver);

    database
        .chatDao()
        .updateChatWorkspaceId(CHAT_ID, None, 2)
        .unwrap();
    let unbound = receiveWorkspaceChat(&receiver);
    assert_eq!(unbound.workspaceId, None);
    assert_eq!(unbound.workspaceName, None);
    assert_eq!(unbound.workspacePrimaryPath, None);
    assert_eq!(manager.getWorkspaceForChat(CHAT_ID).unwrap(), None);

    workspace.name = "Updated after unbind".to_string();
    workspace.updatedAt = 3;
    manager.workspaceStore.upsert(workspace).unwrap();
    assert_eq!(histories.value(), vec![unbound.clone()]);
    assert_eq!(manager.loadChatHistories().unwrap(), vec![unbound]);
    assert_eq!(manager.getWorkspaceForChat(CHAT_ID).unwrap(), None);
    histories.unsubscribe(subscription);
}

/// Deletes an unlocked orphaned workspace chat, its messages, selection, and binding without resolving the workspace.
#[test]
fn missing_workspace_chat_deletion_preserves_lock_policy_and_removes_history() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database, workspace) = workspaceMetadataFixture("workspace-metadata-delete");
    manager.bindingStore.createLocal(CHAT_ID).unwrap();
    manager.setCurrentChatId(CHAT_ID.to_string()).unwrap();
    let message = ChatMessage::new_with_markdown_timestamp(
        "user".to_string(),
        "Orphaned conversation".to_string(),
        MESSAGE_TIMESTAMP,
    );
    database
        .messageDao()
        .insertMessage(MessageEntity::fromChatMessage(
            CHAT_ID.to_string(),
            message,
            1,
            0,
        ))
        .unwrap();
    let histories = manager.chatHistoriesFlow().unwrap();
    assert_eq!(
        histories.value()[0].workspaceId.as_deref(),
        Some(workspace.id.as_str())
    );
    assert_eq!(histories.value()[0].workspaceName, None);
    assert!(manager
        .workspaceStore
        .getById(&workspace.id)
        .unwrap()
        .is_none());

    manager.updateChatLocked(CHAT_ID.to_string(), true).unwrap();
    assert!(!manager.deleteChatHistory(CHAT_ID.to_string()).unwrap());
    assert!(database.chatDao().getChatById(CHAT_ID).unwrap().is_some());
    assert!(manager.bindingStore.contains(CHAT_ID).unwrap());

    manager
        .updateChatLocked(CHAT_ID.to_string(), false)
        .unwrap();
    assert!(manager.deleteChatHistory(CHAT_ID.to_string()).unwrap());
    assert!(database.chatDao().getChatById(CHAT_ID).unwrap().is_none());
    assert!(database
        .messageDao()
        .getMessagesForChat(CHAT_ID)
        .unwrap()
        .is_empty());
    assert!(histories.value().is_empty());
    assert_eq!(manager.currentChatIdFlow().unwrap(), None);
    assert!(!manager.bindingStore.contains(CHAT_ID).unwrap());
    assert!(manager.workspaceStore.getAll().unwrap().is_empty());
}

/// Missing routing metadata cannot veto offline deletion or its idempotent cleanup retry.
#[test]
fn chat_deletion_without_binding_is_durable_and_cannot_be_routed_back() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database, _) = workspaceMetadataFixture("delete-without-binding");
    assert!(!manager.bindingStore.contains(CHAT_ID).unwrap());
    manager.setCurrentChatId(CHAT_ID.into()).unwrap();
    assert!(manager.deleteChatHistory(CHAT_ID.into()).unwrap());
    assert!(manager.syncStore.chatIsDeleted(CHAT_ID).unwrap());
    assert!(manager.canDeleteChatHistory(CHAT_ID.into()).unwrap());
    assert!(manager.deleteChatHistory(CHAT_ID.into()).unwrap());
    assert_eq!(manager.currentChatIdFlow().unwrap(), None);
    assert!(database.chatDao().getChatById(CHAT_ID).unwrap().is_none());

    // A delayed route change may arrive independently; it must not recreate SQL history.
    let mut route = manager.bindingStore.createLocal("late-route-template").unwrap().operation;
    route.originDeviceId = "late-route-source".into();
    route.sequence = 1;
    route.opId = "late-route-source:1".into();
    route.entityId = CHAT_ID.into();
    route.payload["key"] = serde_json::json!(CHAT_ID);
    route.createdAt += 100_000;
    manager.bindingStore.applyBootstrapOperation(&route).unwrap();
    assert!(manager.bindingStore.contains(CHAT_ID).unwrap());
    assert!(manager.ensureRoutedChat(CHAT_ID.into()).is_err());
    assert!(database.chatDao().getChatById(CHAT_ID).unwrap().is_none());
    assert!(manager.deleteChatHistory(CHAT_ID.into()).unwrap());
    assert!(!manager.bindingStore.contains(CHAT_ID).unwrap());
}

/// Creates one original reply and ordered variants with colliding part identities.
fn variantFixture(name: &str, alternateCount: i32) -> Arc<AppDatabase> {
    let (_, database, _) = openTestStore(name);
    database
        .chatDao()
        .insertChat(ChatEntity::new(
            CHAT_ID.to_string(),
            "Variants".to_string(),
            1,
        ))
        .unwrap();
    let mut message = ChatMessage::new_with_markdown_timestamp(
        "ai".to_string(),
        "original".to_string(),
        MESSAGE_TIMESTAMP,
    );
    message.roleName = "original-role".to_string();
    message.isFavorite = true;
    message.completedExecutionGeneration = 42;
    database
        .messageDao()
        .insertMessage(MessageEntity::fromChatMessage(
            CHAT_ID.to_string(),
            message.clone(),
            7,
            0,
        ))
        .unwrap();
    database
        .messagePartDao()
        .replaceParts(
            CHAT_ID,
            MESSAGE_TIMESTAMP,
            0,
            messagePartEntities(CHAT_ID, MESSAGE_TIMESTAMP, 0, &message.parts),
        )
        .unwrap();
    for index in 1..=alternateCount {
        let mut variant = ChatMessage::new_with_markdown_timestamp(
            "ai".to_string(),
            format!("revision-{index}"),
            MESSAGE_TIMESTAMP,
        );
        variant.roleName = format!("role-{index}");
        variant.provider = format!("provider-{index}");
        variant.modelName = format!("model-{index}");
        variant.inputTokens = 10 + i64::from(index);
        variant.outputTokens = 20 + i64::from(index);
        variant.cachedInputTokens = 30 + i64::from(index);
        variant.sentAt = 40 + i64::from(index);
        variant.outputDurationMs = 50 + i64::from(index);
        variant.waitDurationMs = 60 + i64::from(index);
        variant.completedAt = 70 + i64::from(index);
        database
            .messageVariantDao()
            .insertVariant(MessageVariantEntity::fromChatMessage(
                CHAT_ID.to_string(),
                MESSAGE_TIMESTAMP,
                index,
                variant.clone(),
                0,
            ))
            .unwrap();
        database
            .messagePartDao()
            .replaceParts(
                CHAT_ID,
                MESSAGE_TIMESTAMP,
                index,
                messagePartEntities(CHAT_ID, MESSAGE_TIMESTAMP, index, &variant.parts),
            )
            .unwrap();
    }
    database
}

/// Runs the exact persistence mutation used by the chat history manager.
fn deleteRevision(database: &AppDatabase, index: i32) -> ChatHistoryManagerResult<()> {
    let base = database
        .messageDao()
        .getMessageByTimestamp(CHAT_ID, MESSAGE_TIMESTAMP)
        .unwrap()
        .unwrap();
    let variants = database
        .messageVariantDao()
        .getVariantsForMessage(CHAT_ID, MESSAGE_TIMESTAMP)
        .unwrap();
    deleteMessageVariantRevision(database.store(), &base, &variants, index)
}

/// Captures every persisted message field, variant, and part for mutation assertions.
fn snapshot(
    database: &AppDatabase,
) -> (
    MessageEntity,
    Vec<MessageVariantEntity>,
    Vec<MessagePartEntity>,
) {
    (
        database
            .messageDao()
            .getMessageByTimestamp(CHAT_ID, MESSAGE_TIMESTAMP)
            .unwrap()
            .unwrap(),
        database
            .messageVariantDao()
            .getVariantsForMessage(CHAT_ID, MESSAGE_TIMESTAMP)
            .unwrap(),
        database.messagePartDao().getPartsForChat(CHAT_ID).unwrap(),
    )
}

/// Checks contiguous revision identities, selected state, and the matching structured content.
fn assertRevisions(database: &AppDatabase, selectedIndex: i32, contents: &[&str]) {
    let (base, variants, parts) = snapshot(database);
    assert_eq!(base.selectedVariantIndex, selectedIndex);
    assert_eq!(
        variants
            .iter()
            .map(|variant| variant.variantIndex)
            .collect::<Vec<_>>(),
        (1..contents.len() as i32).collect::<Vec<_>>()
    );
    assert_eq!(
        parts
            .iter()
            .map(|part| part.variantIndex)
            .collect::<Vec<_>>(),
        (0..contents.len() as i32).collect::<Vec<_>>()
    );
    assert_eq!(
        parts
            .iter()
            .map(|part| part.content.as_str())
            .collect::<Vec<_>>(),
        contents
    );
}

/// Promotes the first alternate while preserving the original message identity and chat flags.
#[test]
fn deleting_original_promotes_metadata_and_parts_and_reindexes_alternates() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let database = variantFixture("delete-original", 3);
    let (original, variants, _) = snapshot(&database);
    deleteRevision(&database, 0).unwrap();
    assertRevisions(&database, 0, &["revision-1", "revision-2", "revision-3"]);
    let (promoted, remaining, _) = snapshot(&database);
    assert_eq!(promoted.messageId, original.messageId);
    assert_eq!(promoted.timestamp, original.timestamp);
    assert_eq!(promoted.orderIndex, original.orderIndex);
    assert_eq!(promoted.isFavorite, original.isFavorite);
    assert_eq!(
        promoted.completedExecutionGeneration,
        original.completedExecutionGeneration
    );
    assert_eq!(promoted.roleName, variants[0].roleName);
    assert_eq!(promoted.provider, variants[0].provider);
    assert_eq!(promoted.modelName, variants[0].modelName);
    assert_eq!(promoted.inputTokens, variants[0].inputTokens);
    assert_eq!(promoted.outputTokens, variants[0].outputTokens);
    assert_eq!(promoted.cachedInputTokens, variants[0].cachedInputTokens);
    assert_eq!(promoted.sentAt, variants[0].sentAt);
    assert_eq!(promoted.outputDurationMs, variants[0].outputDurationMs);
    assert_eq!(promoted.waitDurationMs, variants[0].waitDurationMs);
    assert_eq!(promoted.completedAt, variants[0].completedAt);
    assert_eq!(remaining[0].variantId, variants[1].variantId);
    assert_eq!(remaining[1].variantId, variants[2].variantId);
}

/// Selects the next remaining revision after deleting a middle alternate.
#[test]
fn deleting_middle_reindexes_parts_and_selects_next_revision() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let database = variantFixture("delete-middle", 3);
    deleteRevision(&database, 2).unwrap();
    assertRevisions(&database, 2, &["original", "revision-1", "revision-3"]);
}

/// Selects the preceding revision after deleting the last alternate.
#[test]
fn deleting_last_selects_previous_revision() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let database = variantFixture("delete-last", 3);
    deleteRevision(&database, 3).unwrap();
    assertRevisions(&database, 2, &["original", "revision-1", "revision-2"]);
}

/// Supports deleting either revision from a two-revision message without deleting the message.
#[test]
fn deleting_from_two_revisions_keeps_one_complete_reply() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    for (deletedIndex, expected) in [(0, "revision-1"), (1, "original")] {
        let database = variantFixture("delete-two", 1);
        deleteRevision(&database, deletedIndex).unwrap();
        assertRevisions(&database, 0, &[expected]);
    }
}

/// Repeated original deletion promotes each survivor and keeps all part indices navigable.
#[test]
fn repeated_deletion_preserves_contiguous_revision_indices() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let database = variantFixture("delete-repeated", 3);
    deleteRevision(&database, 1).unwrap();
    assertRevisions(&database, 1, &["original", "revision-2", "revision-3"]);
    deleteRevision(&database, 0).unwrap();
    assertRevisions(&database, 0, &["revision-2", "revision-3"]);
    deleteRevision(&database, 0).unwrap();
    assertRevisions(&database, 0, &["revision-3"]);
    assert!(deleteRevision(&database, 0).is_err());
    assertRevisions(&database, 0, &["revision-3"]);
}

/// Rejects nonexistent revision indices and sole-revision deletion without any mutation.
#[test]
fn invalid_deletion_leaves_persisted_message_unchanged() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let database = variantFixture("delete-invalid", 2);
    let before = snapshot(&database);
    for index in [-1, 3, 100] {
        assert!(matches!(
            deleteRevision(&database, index),
            Err(ChatHistoryManagerError::IllegalArgument(_))
        ));
        assert_eq!(snapshot(&database), before);
    }
    let database = variantFixture("delete-sole", 0);
    let before = snapshot(&database);
    assert!(matches!(
        deleteRevision(&database, 0),
        Err(ChatHistoryManagerError::IllegalState(_))
    ));
    assert_eq!(snapshot(&database), before);
}

/// Rolls back promotion, deletion, parts migration, and reindexing on a database write failure.
#[test]
fn failed_reindex_rolls_back_the_entire_revision_deletion() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let database = variantFixture("delete-atomic", 3);
    database.store().executeBatch(
        "CREATE TRIGGER reject_revision_reindex BEFORE UPDATE ON message_variants BEGIN SELECT RAISE(ABORT, 'test reindex failure'); END;",
    ).unwrap();
    let before = snapshot(&database);
    for index in [0, 1] {
        assert!(deleteRevision(&database, index).is_err());
        assert_eq!(snapshot(&database), before);
    }
}

/// Creates a manually ordered sidebar with a pinned chat above two ordinary chats.
fn chatOrderFixture(name: &str) -> (ChatHistoryManager, Arc<AppDatabase>) {
    let (paths, database, _) = openTestStore(name);
    let manager = ChatHistoryManager::create(paths).unwrap();
    for (index, id) in ["pinned", "recent", "older"].into_iter().enumerate() {
        let mut chat = ChatEntity::new(id.to_string(), id.to_string(), 1);
        chat.displayOrder = index as i64;
        chat.pinned = id == "pinned";
        chat.pluginExtensions.insert(
            "fixture.owner".to_string(),
            serde_json::json!({"opaque": id}),
        );
        database.chatDao().insertChat(chat).unwrap();
    }
    (manager, database)
}

/// Verifies canonical chat persistence using real SQLite-backed repository operations.
fn sidebarChatIds(manager: &ChatHistoryManager) -> Vec<String> {
    manager
        .chatHistoriesFlow()
        .unwrap()
        .value()
        .into_iter()
        .map(|chat| chat.id)
        .collect()
}

/// Verifies canonical chat persistence using real SQLite-backed repository operations.
#[test]
fn new_user_message_promotes_chat_below_pins_and_preserves_metadata() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database) = chatOrderFixture("chat-order-user-send");
    let original = database.chatDao().getChatById("older").unwrap().unwrap();
    let recent = database.chatDao().getChatById("recent").unwrap().unwrap();
    manager
        .addMessage(
            "older".to_string(),
            ChatMessage::new_with_markdown_timestamp("user".to_string(), "hello".to_string(), 100),
        )
        .unwrap();

    assert_eq!(sidebarChatIds(&manager), ["pinned", "older", "recent"]);
    let promoted = database.chatDao().getChatById("older").unwrap().unwrap();
    assert!(promoted.displayOrder < recent.displayOrder);
    assert_eq!(
        promoted.clone(),
        ChatEntity {
            group: None,
            displayOrder: promoted.displayOrder,
            updatedAt: promoted.updatedAt,
            ..original
        }
    );
    assert_eq!(
        database.chatDao().getChatById("recent").unwrap().unwrap(),
        recent
    );
}

/// Verifies canonical chat persistence using real SQLite-backed repository operations.
#[test]
fn successive_user_sends_follow_send_order_even_with_negative_display_orders() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database) = chatOrderFixture("chat-order-successive-sends");
    // A saved order can be smaller than the negated current time. Promotion must
    // still move ahead of it rather than relying solely on the wall clock.
    database
        .chatDao()
        .updateChatOrder("recent", -9_000_000_000_000_000)
        .unwrap();
    for (index, id) in ["older", "recent", "older"].into_iter().enumerate() {
        manager
            .addMessage(
                id.to_string(),
                ChatMessage::new_with_markdown_timestamp(
                    "user".to_string(),
                    "hello".to_string(),
                    100 + index as i64,
                ),
            )
            .unwrap();
        let expectedOther = if id == "older" { "recent" } else { "older" };
        assert_eq!(sidebarChatIds(&manager), ["pinned", id, expectedOther]);
    }
}

/// Verifies canonical chat persistence using real SQLite-backed repository operations.
#[test]
fn replies_edits_and_duplicate_user_saves_do_not_promote_chat() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database) = chatOrderFixture("chat-order-no-promote-on-update");
    let userMessage =
        ChatMessage::new_with_markdown_timestamp("user".to_string(), "original".to_string(), 100);
    manager
        .addMessage("older".to_string(), userMessage.clone())
        .unwrap();
    manager
        .addMessage(
            "recent".to_string(),
            ChatMessage::new_with_markdown_timestamp("user".to_string(), "latest".to_string(), 101),
        )
        .unwrap();
    let savedOrder = database
        .chatDao()
        .getChatById("older")
        .unwrap()
        .unwrap()
        .displayOrder;
    manager
        .updateMessage(
            "older".to_string(),
            ChatMessage::new_with_markdown_timestamp("user".to_string(), "edited".to_string(), 100),
        )
        .unwrap();
    manager
        .addMessage("older".to_string(), userMessage)
        .unwrap();
    for (index, sender) in ["ai", "summary", "tool"].into_iter().enumerate() {
        let mut message = ChatMessage::new_with_markdown_timestamp(
            sender.to_string(),
            "reply".to_string(),
            102 + index as i64,
        );
        manager
            .addMessage("older".to_string(), message.clone())
            .unwrap();
        message.completedAt = 200;
        manager.updateMessage("older".to_string(), message).unwrap();
        assert_eq!(sidebarChatIds(&manager), ["pinned", "recent", "older"]);
        assert_eq!(
            database
                .chatDao()
                .getChatById("older")
                .unwrap()
                .unwrap()
                .displayOrder,
            savedOrder
        );
    }
}

/// Verifies canonical chat persistence using real SQLite-backed repository operations.
#[test]
fn new_background_user_message_promotes_chat_through_update_path() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database) = chatOrderFixture("chat-order-background-send");
    let message = ChatMessage::new_with_markdown_timestamp(
        "user".to_string(),
        "background send".to_string(),
        100,
    );
    manager
        .updateMessage("older".to_string(), message.clone())
        .unwrap();
    assert_eq!(sidebarChatIds(&manager), ["pinned", "older", "recent"]);
    let savedOrder = database
        .chatDao()
        .getChatById("older")
        .unwrap()
        .unwrap()
        .displayOrder;
    manager.updateMessage("older".to_string(), message).unwrap();
    assert_eq!(
        database
            .chatDao()
            .getChatById("older")
            .unwrap()
            .unwrap()
            .displayOrder,
        savedOrder
    );
}

/// Verifies canonical chat persistence using real SQLite-backed repository operations.
#[test]
fn user_send_in_pinned_chat_moves_it_to_front_without_unpinning() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database) = chatOrderFixture("chat-order-pinned-send");
    manager.updateChatPinned("older".to_string(), true).unwrap();
    assert_eq!(sidebarChatIds(&manager), ["pinned", "older", "recent"]);
    manager
        .addMessage(
            "older".to_string(),
            ChatMessage::new_with_markdown_timestamp("user".to_string(), "hello".to_string(), 100),
        )
        .unwrap();
    assert_eq!(sidebarChatIds(&manager), ["older", "pinned", "recent"]);
    assert!(
        database
            .chatDao()
            .getChatById("older")
            .unwrap()
            .unwrap()
            .pinned
    );
}

/// Leaves no message, part, sidebar change, or sync operation when either write stage fails.
#[test]
fn message_insert_and_sync_failure_leave_no_partial_message() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    for table in ["message_parts", "sync_sql_operations"] {
        let (manager, database) = chatOrderFixture(&format!("atomic-message-insert-{table}"));
        let chatBefore = database.chatDao().getChatById("older").unwrap();
        let clockBefore = manager.syncClock().unwrap();
        database.store().executeBatch(&format!(
            "CREATE TRIGGER reject_write BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT, 'injected write failure'); END;"
        )).unwrap();
        let message =
            ChatMessage::new_with_markdown_timestamp("user".to_string(), "hello".to_string(), 100);
        assert!(manager.addMessage("older".to_string(), message).is_err());
        assert!(database
            .messageDao()
            .getMessageByTimestamp("older", 100)
            .unwrap()
            .is_none());
        assert!(database
            .messagePartDao()
            .getPartsForChat("older")
            .unwrap()
            .is_empty());
        assert_eq!(database.chatDao().getChatById("older").unwrap(), chatBefore);
        assert_eq!(manager.syncClock().unwrap(), clockBefore);
    }
}

/// Preserves the old metadata, contents, and clock when replacing revision parts fails.
#[test]
fn failed_message_update_keeps_the_complete_previous_revision() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database) = chatOrderFixture("atomic-message-update");
    let mut message =
        ChatMessage::new_with_markdown_timestamp("ai".to_string(), "original".to_string(), 100);
    message.provider = "original-provider".to_string();
    manager
        .addMessage("older".to_string(), message.clone())
        .unwrap();
    let before = manager.loadChatMessages("older").unwrap();
    let clockBefore = manager.syncClock().unwrap();
    database.store().executeBatch(
        "CREATE TRIGGER reject_parts BEFORE INSERT ON message_parts BEGIN SELECT RAISE(ABORT, 'injected part failure'); END;"
    ).unwrap();
    message.provider = "changed-provider".to_string();
    message.replace_with_markdown("changed".to_string());
    assert!(manager.updateMessage("older".to_string(), message).is_err());
    assert_eq!(manager.loadChatMessages("older").unwrap(), before);
    assert_eq!(manager.syncClock().unwrap(), clockBefore);
}

/// Rejects an empty revision while continuing to store a canonical empty Markdown response.
#[test]
fn empty_part_list_cannot_create_a_message_without_base_parts() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database) = chatOrderFixture("atomic-empty-message");
    let mut message =
        ChatMessage::new_with_markdown_timestamp("ai".to_string(), String::new(), 100);
    message.parts.clear();
    assert!(manager.updateMessage("older".to_string(), message).is_err());
    assert!(database
        .messageDao()
        .getMessageByTimestamp("older", 100)
        .unwrap()
        .is_none());
    let message = ChatMessage::new_with_markdown_timestamp("ai".to_string(), String::new(), 100);
    manager.addMessage("older".to_string(), message).unwrap();
    let loaded = manager
        .loadChatMessagesDesc("older".to_string(), 10, None)
        .unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].parts.len(), 1);
    assert_eq!(loaded[0].displayText(), "");
}

/// Keeps the base selection and variant numbering unchanged after a failed regenerated response.
#[test]
fn failed_variant_insert_does_not_leave_an_empty_revision() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database) = chatOrderFixture("atomic-message-variant");
    let message =
        ChatMessage::new_with_markdown_timestamp("ai".to_string(), "original".to_string(), 100);
    manager
        .addMessage("older".to_string(), message.clone())
        .unwrap();
    let before = manager.loadChatMessages("older").unwrap();
    let clockBefore = manager.syncClock().unwrap();
    database.store().executeBatch(
        "CREATE TRIGGER reject_variant_parts BEFORE INSERT ON message_parts WHEN NEW.variantIndex > 0 BEGIN SELECT RAISE(ABORT, 'injected variant failure'); END;"
    ).unwrap();
    assert!(manager
        .addMessageVariant("older".to_string(), 100, message.clone())
        .is_err());
    assert!(database
        .messageVariantDao()
        .getVariantsForMessage("older", 100)
        .unwrap()
        .is_empty());
    assert_eq!(manager.loadChatMessages("older").unwrap(), before);
    assert_eq!(manager.syncClock().unwrap(), clockBefore);
    database
        .store()
        .executeBatch("DROP TRIGGER reject_variant_parts")
        .unwrap();
    assert_eq!(
        manager
            .addMessageVariant("older".to_string(), 100, message)
            .unwrap(),
        1
    );
}

/// Preserves message metadata, every revision, and sync state when any deletion stage fails.
#[test]
fn failed_message_deletions_preserve_parts_and_variants() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    for mode in 0..3 {
        let (manager, database) = chatOrderFixture(&format!("atomic-message-delete-{mode}"));
        let message =
            ChatMessage::new_with_markdown_timestamp("ai".to_string(), "original".to_string(), 100);
        manager
            .addMessage("older".to_string(), message.clone())
            .unwrap();
        manager
            .addMessageVariant("older".to_string(), 100, message)
            .unwrap();
        let before = manager.loadChatMessages("older").unwrap();
        let clockBefore = manager.syncClock().unwrap();
        database.store().executeBatch(
            "CREATE TRIGGER reject_delete BEFORE DELETE ON messages BEGIN SELECT RAISE(ABORT, 'injected delete failure'); END;"
        ).unwrap();
        let result = match mode {
            0 => manager.deleteMessage("older".to_string(), 100),
            1 => manager.deleteMessagesFrom("older".to_string(), 100),
            2 => manager.clearChatMessages("older".to_string()),
            _ => unreachable!(),
        };
        assert!(result.is_err());
        assert_eq!(manager.loadChatMessages("older").unwrap(), before);
        assert_eq!(
            database
                .messagePartDao()
                .getPartsForMessage("older", 100, 0)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            database
                .messagePartDao()
                .getPartsForMessage("older", 100, 1)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(manager.syncClock().unwrap(), clockBefore);
    }
}

/// Preserves page boundaries and summary context selection when loading transaction snapshots.
#[test]
fn message_snapshots_preserve_paging_and_summary_boundaries() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, _) = chatOrderFixture("message-snapshot-boundaries");
    for (timestamp, sender) in [
        (100, "user"),
        (200, "summary"),
        (300, "ai"),
        (400, "summary"),
        (500, "user"),
    ] {
        let message = ChatMessage::new_with_markdown_timestamp(
            sender.to_string(),
            timestamp.to_string(),
            timestamp,
        );
        manager.addMessage("older".to_string(), message).unwrap();
    }
    let newest = manager
        .loadChatMessagesDesc("older".to_string(), 2, None)
        .unwrap();
    assert_eq!(
        newest
            .iter()
            .map(|message| message.timestamp)
            .collect::<Vec<_>>(),
        [500, 400]
    );
    let older = manager
        .loadOlderChatMessages("older".to_string(), 400, 2)
        .unwrap();
    assert_eq!(
        older
            .iter()
            .map(|message| message.timestamp)
            .collect::<Vec<_>>(),
        [200, 300]
    );
    let context = manager
        .loadRuntimeChatMessages("older".to_string())
        .unwrap();
    assert_eq!(
        context
            .iter()
            .map(|message| message.timestamp)
            .collect::<Vec<_>>(),
        [400, 500]
    );
    for (before, through, expected) in [
        (Some(400), None, vec![300]),
        (None, Some(300), vec![300]),
        (Some(500), Some(300), vec![]),
    ] {
        let context = manager
            .loadMessagesAfterLatestSummaryInRange("older".to_string(), before, through)
            .unwrap();
        assert_eq!(
            context
                .iter()
                .map(|message| message.timestamp)
                .collect::<Vec<_>>(),
            expected
        );
    }
}

/// Rejects incomplete imported records without replacing the existing chat contents or sync state.
#[test]
fn incomplete_import_cannot_commit_a_message_without_parts() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (manager, database) = chatOrderFixture("atomic-incomplete-import");
    let message =
        ChatMessage::new_with_markdown_timestamp("ai".to_string(), "original".to_string(), 100);
    manager
        .addMessage("older".to_string(), message.clone())
        .unwrap();
    let before = manager.loadChatMessages("older").unwrap();
    let clockBefore = manager.syncClock().unwrap();
    let chat = database.chatDao().getChatById("older").unwrap().unwrap();
    let message = MessageEntity::fromChatMessage("older".to_string(), message, 0, 0);
    assert!(manager
        .syncStore
        .commitImportedChatRecords(chat, vec![message], Vec::new(), Vec::new())
        .is_err());
    assert_eq!(manager.loadChatMessages("older").unwrap(), before);
    assert_eq!(manager.syncClock().unwrap(), clockBefore);
}
