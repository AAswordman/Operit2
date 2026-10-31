use super::*;
use crate::SqliteStore::{SqliteRowGet, SqliteStore, SqliteStoreError};
use operit_host_native_storage::NativeRuntimeStorageHost;
use serde_json::json;
use std::sync::Arc;

struct Fixture {
    root: std::path::PathBuf,
    store: SqliteStore,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("operit-compaction-{}", uuid::Uuid::new_v4()));
        let host = Arc::new(NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        ));
        let store = SqliteStore::openWithHost(host, "runtime/compaction.sqlite").unwrap();
        store.executeBatch(SCHEMA).unwrap();
        store
            .executeBatch("CREATE TABLE payloads(id TEXT PRIMARY KEY,original TEXT NOT NULL)")
            .unwrap();
        Self { root, store }
    }
    fn insert(&self, operation: &SyncOperation, scopes: Option<BTreeSet<String>>) {
        self.store
            .transaction(|tx| {
                if retired(tx.raw(), operation)? {
                    return Ok(());
                }
                tx.execute(
                    "INSERT OR IGNORE INTO payloads VALUES(?1,?2)",
                    vec![
                        text(&operation.opId),
                        text(&serde_json::to_string(operation).unwrap()),
                    ],
                )?;
                for id in record(tx.raw(), operation, scopes)? {
                    tx.execute("DELETE FROM payloads WHERE id=?1", vec![text(&id)])?;
                }
                Ok(())
            })
            .unwrap();
    }
    fn originals(&self) -> Vec<SyncOperation> {
        self.store
            .queryRows("SELECT original FROM payloads ORDER BY id", vec![])
            .unwrap()
            .iter()
            .map(|row| serde_json::from_str(&row.get::<_, String>(0).unwrap()).unwrap())
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn operation(author: &str, time: i64, values: serde_json::Value) -> SyncOperation {
    SyncOperation {
        opId: format!("{author}:{time}"),
        originDeviceId: author.into(),
        sequence: time,
        createdAt: time,
        domain: "test".into(),
        entityType: "transaction".into(),
        entityId: "file".into(),
        operation: "commit".into(),
        semantics: SyncOperationSemantics::Transaction,
        payload: values,
        schemaVersion: 1,
    }
}
fn footprint(operation: &SyncOperation) -> Option<BTreeSet<String>> {
    Some(
        operation
            .payload
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect(),
    )
}
fn replay(
    operations: impl IntoIterator<Item = SyncOperation>,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    let mut state = std::collections::BTreeMap::new();
    for operation in
        super::super::ordered(operations, |op| &op.opId, Revision::fromOperation).unwrap()
    {
        for (key, value) in operation.payload.as_object().unwrap() {
            state.insert(key.clone(), value.clone());
        }
    }
    state
}
#[test]
fn indexed_whole_transaction_compaction_commutes_with_external_absolute_writes() {
    let history = [
        operation("a", 10, json!({"x":"A","y":1})),
        operation("b", 20, json!({"x":"AB"})),
        operation("c", 30, json!({"y":2})),
    ];
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let fixture = Fixture::new();
        for index in order {
            let op = &history[index];
            fixture.insert(op, footprint(op));
        }
        assert_eq!(fixture.originals().len(), 2);
        assert!(!fixture.originals().contains(&history[0]));
        for time in [5, 15, 25, 35] {
            let external = operation("external", time, json!({"x":"other","z":true}));
            assert_eq!(
                replay(history.iter().cloned().chain([external.clone()])),
                replay(fixture.originals().into_iter().chain([external]))
            );
        }
        fixture.insert(&history[0], footprint(&history[0]));
        assert_eq!(fixture.originals().len(), 2);
    }
}
#[test]
fn partial_effects_and_unproven_business_rules_stay_pinned() {
    let fixture = Fixture::new();
    let first = operation("a", 10, json!({"x":"A","y":1}));
    let second = operation("b", 20, json!({"x":"AB"}));
    fixture.insert(&first, footprint(&first));
    fixture.insert(&second, footprint(&second));
    assert_eq!(fixture.originals().len(), 2);
    let pinned = operation("c", 25, json!({"x":"catalog effect"}));
    fixture.insert(&pinned, None);
    let final_write = operation("d", 30, json!({"x":"final","y":2}));
    fixture.insert(&final_write, footprint(&final_write));
    assert_eq!(fixture.originals(), vec![pinned, final_write]);
}
#[test]
fn retirement_fingerprints_and_payload_deletion_rollback_together() {
    let fixture = Fixture::new();
    let first = operation("a", 10, json!({"x":"A"}));
    let second = operation("b", 20, json!({"x":"AB"}));
    fixture.insert(&first, footprint(&first));
    let result: Result<(), SqliteStoreError> = fixture.store.transaction(|tx| {
        for id in record(tx.raw(), &second, footprint(&second))? {
            tx.execute("DELETE FROM payloads WHERE id=?1", vec![text(&id)])?;
        }
        Err(SqliteStoreError::Message("injected failure".into()))
    });
    assert!(result.is_err());
    assert_eq!(fixture.originals(), vec![first.clone()]);
    fixture.insert(&second, footprint(&second));
    let mut collision = first;
    collision.payload = json!({"x":"forged"});
    assert!(fixture
        .store
        .transaction(|tx| {
            retired(tx.raw(), &collision)?;
            Ok(())
        })
        .is_err());
    assert_eq!(fixture.originals(), vec![second]);
}
