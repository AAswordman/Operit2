use super::*;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::test_host_support::{installSharedTestHost, SHARED_TEST_HOST_MUTEX};

#[derive(Clone, Serialize, Deserialize)]
struct SyncTestEntity {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    updatedAt: i64,
    content: String,
}

impl ObjectBoxEntity for SyncTestEntity {
    fn objectBoxId(&self) -> i64 {
        self.id
    }

    fn setObjectBoxId(&mut self, id: i64) {
        self.id = id;
    }
}

fn syncedEntityPayload(updatedAt: Option<i64>, content: &str) -> serde_json::Value {
    let mut entity = json!({ "content": content });
    if let Some(updatedAt) = updatedAt {
        entity["updatedAt"] = json!(updatedAt);
    }
    json!({ "__entityType": "SyncTestEntity", "entity": entity })
}

/// Applies every sync state transition of one entity through the real static
/// apply path, exercising last-writer-wins, tie rejection, timestampless
/// compatibility, and delete semantics.
#[test]
fn objectbox_synced_upsert_resolves_conflicts_by_updated_at() {
    let _guard = SHARED_TEST_HOST_MUTEX
        .lock()
        .expect("shared test host mutex must not poison");
    installSharedTestHost();
    let databasePath = RuntimeStorePaths::default()
        .runtime_dir()
        .join("objectbox-sync-tests/entity.sqlite");
    let databaseStoragePath = crate::RuntimeStorageHost::runtimeStoragePath(&databasePath);
    let entityId = format!("{databaseStoragePath}#42");
    let store = ObjectBox::<SyncTestEntity>::new(databasePath.with_extension(""), "SyncTestEntity");
    let storedContent = || store.all().unwrap().remove(0).content;

    // First state lands on an empty store.
    ObjectBox::<SyncTestEntity>::applySyncedEntity(
        &entityId,
        "upsert",
        syncedEntityPayload(Some(1000), "first"),
    )
    .unwrap();
    assert_eq!(storedContent(), "first");

    // A tie (same updatedAt — for example an access-only replay where only
    // lastAccessedAt moved) must not clobber the stored state.
    ObjectBox::<SyncTestEntity>::applySyncedEntity(
        &entityId,
        "upsert",
        syncedEntityPayload(Some(1000), "replayed-read"),
    )
    .unwrap();
    assert_eq!(storedContent(), "first");

    // A stale state loses as well.
    ObjectBox::<SyncTestEntity>::applySyncedEntity(
        &entityId,
        "upsert",
        syncedEntityPayload(Some(999), "stale"),
    )
    .unwrap();
    assert_eq!(storedContent(), "first");

    // A genuinely newer state wins.
    ObjectBox::<SyncTestEntity>::applySyncedEntity(
        &entityId,
        "upsert",
        syncedEntityPayload(Some(2000), "newer"),
    )
    .unwrap();
    let stored = store.all().unwrap().remove(0);
    assert_eq!(stored.content, "newer");
    assert_eq!(stored.updatedAt, 2000);

    // Entities without an updatedAt field keep the previous overwrite behavior.
    ObjectBox::<SyncTestEntity>::applySyncedEntity(
        &entityId,
        "upsert",
        syncedEntityPayload(None, "no-timestamp-a"),
    )
    .unwrap();
    assert_eq!(storedContent(), "no-timestamp-a");
    ObjectBox::<SyncTestEntity>::applySyncedEntity(
        &entityId,
        "upsert",
        syncedEntityPayload(None, "no-timestamp-b"),
    )
    .unwrap();
    assert_eq!(storedContent(), "no-timestamp-b");

    // Deletes stay unconditional.
    ObjectBox::<SyncTestEntity>::applySyncedEntity(
        &entityId,
        "delete",
        json!({ "__entityType": "SyncTestEntity" }),
    )
    .unwrap();
    assert!(store.all().unwrap().is_empty());
}
