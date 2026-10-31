use operit_host_api::RuntimeStorageHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::{
    PreferenceMerge::{applyOperations, PreferenceMergeSchema, PreferenceMergeStore},
    PreferencesDataStore::{stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError},
    SyncOperationStore::{SyncOperation, SyncOperationSemantics},
    Transaction::{migration::Context, Rule, Snapshot},
};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};

const PATH: &str = "runtime/config/preferences/migration_context.preferences.json";
struct Fixture {
    root: PathBuf,
    host: Arc<dyn RuntimeStorageHost>,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("operit-migration-{}", uuid::Uuid::new_v4()));
        let host = Arc::new(NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        ));
        Self { root, host }
    }
    fn store(&self, version: u32, drop_factor: bool) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(self.host.clone(), PATH)
            .withMigration(version, move |version, context| {
                convert(version, context, drop_factor)
            })
    }
    fn apply(&self, version: u32, drop_factor: bool, operations: &[SyncOperation]) {
        fn keep(host: Arc<dyn RuntimeStorageHost>, path: &str) -> PreferencesDataStore {
            PreferencesDataStore::newWithStorage(host, path)
                .withMigration(1, |version, context| convert(version, context, false))
        }
        fn drop(host: Arc<dyn RuntimeStorageHost>, path: &str) -> PreferencesDataStore {
            PreferencesDataStore::newWithStorage(host, path)
                .withMigration(1, |version, context| convert(version, context, true))
        }
        fn legacy(host: Arc<dyn RuntimeStorageHost>, path: &str) -> PreferencesDataStore {
            PreferencesDataStore::newWithStorage(host, path).withSchema(0, |_, _| Ok(()))
        }
        let open = match (version, drop_factor) {
            (0, _) => legacy,
            (_, false) => keep,
            (_, true) => drop,
        };
        let schema = PreferenceMergeSchema {
            stores: vec![PreferenceMergeStore::values(PATH, false, open)],
        };
        applyOperations(self.host.clone(), &schema, &[], operations, |_, _| true).unwrap();
    }
    fn seed(&self) -> Snapshot {
        let store = self.store(0, false);
        store
            .edit(|values| {
                values.set(&stringPreferencesKey("factor"), 10);
                values.set(&stringPreferencesKey("number"), "1");
            })
            .unwrap();
        let snapshot = store.syncSnapshot().unwrap();
        assert!(store
            .repairSyncedSnapshot(&snapshot.token().unwrap(), &snapshot)
            .unwrap());
        snapshot
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn convert(
    version: u32,
    context: &mut Context,
    drop_factor: bool,
) -> Result<(), PreferencesDataStoreError> {
    context.map_preferences(version, &|_, values| {
        let factor = values
            .get(&stringPreferencesKey("factor"))
            .and_then(Value::as_i64)
            .unwrap_or(1);
        if let Some(encoded) = values
            .get(&stringPreferencesKey("number"))
            .and_then(Value::as_str)
        {
            let number: i64 = serde_json::from_str(encoded)?;
            values.set(&stringPreferencesKey("number"), number * factor);
        }
        Ok(())
    })?;
    if drop_factor {
        Rule::DropKey {
            key: "factor".into(),
        }
        .apply(context)
        .map_err(PreferencesDataStoreError::Message)?;
    }
    Ok(())
}
fn assignment(sequence: i64, version: u32, key: &str, value: Value) -> SyncOperation {
    SyncOperation {
        opId: format!("remote:{sequence}"),
        originDeviceId: "remote".into(),
        sequence,
        domain: "preferences".into(),
        entityType: "entry".into(),
        entityId: serde_json::to_string(&(PATH, key)).unwrap(),
        operation: "set".into(),
        semantics: SyncOperationSemantics::EntityState,
        payload: json!({"storagePath":PATH,"key":key,"value":value,"schemaVersion":version,"formatVersion":2,"encrypted":false}),
        createdAt: 2_000_000_000_000 + sequence,
        schemaVersion: 1,
    }
}

#[test]
fn late_transactions_use_the_covered_baseline_including_source_only_values() {
    for drop_factor in [false, true] {
        let actual = Fixture::new();
        let expected = Fixture::new();
        let baseline = actual.seed();
        let legacy = expected.store(0, false);
        let empty = legacy.syncSnapshot().unwrap();
        assert!(legacy
            .repairSyncedSnapshot(&empty.token().unwrap(), &baseline)
            .unwrap());
        let operations = [
            assignment(1, 0, "number", json!("2")),
            assignment(2, 0, "number", json!("3")),
        ];
        expected.apply(0, false, &operations);
        let expected = expected.store(1, drop_factor).syncSnapshot().unwrap().data;
        actual.store(1, drop_factor).syncSnapshot().unwrap();
        for _ in 0..2 {
            // Opening again exercises the durable source context, not an in-memory cache.
            actual.apply(1, drop_factor, &operations);
            let result = actual.store(1, drop_factor).syncSnapshot().unwrap().data;
            assert_eq!(result.values.get("number"), Some(&json!(30)));
            assert_eq!(result, expected, "m(Apply(A,T)) = Apply(m(A),m_A(T))");
        }
    }
}

#[test]
fn mixed_format_history_uses_current_edits_in_later_old_transaction_context() {
    let fixture = Fixture::new();
    let old = fixture.store(0, false);
    old.edit(|values| values.set(&stringPreferencesKey("factor"), 10))
        .unwrap();
    let baseline = old.syncSnapshot().unwrap();
    assert!(old
        .repairSyncedSnapshot(&baseline.token().unwrap(), &baseline)
        .unwrap());
    fixture.store(1, false).syncSnapshot().unwrap();
    fixture.apply(
        1,
        false,
        &[
            assignment(1, 1, "factor", json!(20)),
            assignment(2, 0, "number", json!("2")),
        ],
    );
    assert_eq!(
        fixture
            .store(1, false)
            .syncSnapshot()
            .unwrap()
            .data
            .values
            .get("number"),
        Some(&json!(40))
    );
}

#[test]
fn a_context_sensitive_numeric_projection_uses_the_original_source_state() {
    fn scale(
        _: u32,
        values: &mut operit_store::PreferencesDataStore::Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        let factor = values
            .get(&stringPreferencesKey("factor"))
            .and_then(Value::as_i64)
            .unwrap_or(1);
        if let Some(number) = values
            .get(&stringPreferencesKey("number"))
            .and_then(Value::as_i64)
        {
            values.set(&stringPreferencesKey("number"), number * factor);
        }
        Ok(())
    }
    fn current(host: Arc<dyn RuntimeStorageHost>, path: &str) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(host, path).withSchema(1, scale)
    }
    let fixture = Fixture::new();
    let old = fixture.store(0, false);
    old.edit(|values| {
        values.set(&stringPreferencesKey("factor"), 10);
        values.set(&stringPreferencesKey("number"), 1);
    })
    .unwrap();
    let baseline = old.syncSnapshot().unwrap();
    assert!(old
        .repairSyncedSnapshot(&baseline.token().unwrap(), &baseline)
        .unwrap());
    assert_eq!(
        current(fixture.host.clone(), PATH)
            .syncSnapshot()
            .unwrap()
            .data
            .values["number"],
        json!(10)
    );
    let schema = PreferenceMergeSchema {
        stores: vec![PreferenceMergeStore::values(PATH, false, current)],
    };
    applyOperations(
        fixture.host.clone(),
        &schema,
        &[],
        &[assignment(1, 0, "number", json!(2))],
        |_, _| true,
    )
    .unwrap();
    assert_eq!(
        current(fixture.host.clone(), PATH)
            .syncSnapshot()
            .unwrap()
            .data
            .values["number"],
        json!(20)
    );
}

#[test]
fn structural_rules_apply_to_an_old_transaction_after_receiving_only_a_canonical_checkpoint() {
    fn split(host: Arc<dyn RuntimeStorageHost>, path: &str) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(host, path).withMigration(1, |_, context| {
            Rule::SplitRecords {
                key: "records".into(),
                prefix: "record_".into(),
                identity: "id".into(),
            }
            .apply(context)
            .map_err(PreferencesDataStoreError::Message)
        })
    }
    let authority = Fixture::new();
    let reference = Fixture::new();
    let receiver = Fixture::new();
    let old = authority.store(0, false);
    old.edit(|values| values.set(&stringPreferencesKey("records"), json!([{"id":"a"}])))
        .unwrap();
    let baseline = old.syncSnapshot().unwrap();
    assert!(old
        .repairSyncedSnapshot(&baseline.token().unwrap(), &baseline)
        .unwrap());
    let target = reference.store(0, false);
    assert!(target
        .repairSyncedSnapshot(&target.syncSnapshot().unwrap().token().unwrap(), &baseline)
        .unwrap());
    let normalized = split(authority.host.clone(), PATH).syncSnapshot().unwrap();
    let target = split(receiver.host.clone(), PATH);
    assert!(target
        .repairSyncedSnapshot(
            &target.syncSnapshot().unwrap().token().unwrap(),
            &normalized
        )
        .unwrap());
    let incoming = assignment(1, 0, "records", json!([{"id":"b"}]));
    reference.apply(0, false, &[incoming.clone()]);
    let expected = split(reference.host.clone(), PATH)
        .syncSnapshot()
        .unwrap()
        .data;
    let schema = PreferenceMergeSchema {
        stores: vec![PreferenceMergeStore::values(PATH, false, split)],
    };
    applyOperations(receiver.host.clone(), &schema, &[], &[incoming], |_, _| {
        true
    })
    .unwrap();
    assert_eq!(target.syncSnapshot().unwrap().data, expected);
}

#[test]
fn an_opaque_migration_skips_missing_source_context_instead_of_using_a_false_preimage() {
    fn current(host: Arc<dyn RuntimeStorageHost>, path: &str) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(host, path)
            .withMigration(1, |version, context| convert(version, context, true))
    }
    let authority = Fixture::new();
    authority.seed();
    let baseline = current(authority.host.clone(), PATH)
        .syncSnapshot()
        .unwrap();
    let receiver = Fixture::new();
    let store = current(receiver.host.clone(), PATH);
    assert!(store
        .repairSyncedSnapshot(&store.syncSnapshot().unwrap().token().unwrap(), &baseline)
        .unwrap());
    let incoming = assignment(1, 0, "number", json!("2"));
    let schema = PreferenceMergeSchema {
        stores: vec![PreferenceMergeStore::values(PATH, false, current)],
    };
    let error = applyOperations(
        receiver.host.clone(),
        &schema,
        &[],
        &[incoming.clone()],
        |_, _| true,
    )
    .unwrap_err();
    assert!(error.contains("Missing source baseline"), "{error}");
    assert_eq!(store.syncSnapshot().unwrap().data, baseline.data);
    let journal = operit_store::SyncOperationStore::SyncOperationStore::new(
        receiver.host.clone(),
        "runtime/sync",
    );
    assert!(journal.pendingOperationIds().unwrap().is_empty());
    assert!(journal.skippedOperations().unwrap().contains(&incoming));
}
