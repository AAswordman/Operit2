use super::*;

use crate::sync::SqlChatSyncStore::tests::{openTestStore, DATABASE_MUTEX};

const CHAT_ID: &str = "chat-variants";
const MESSAGE_TIMESTAMP: i64 = 123;

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
