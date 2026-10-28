use operit_host_api::RuntimeStorageHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::{
    PreferenceMerge::{applyOperations, PreferenceMergeSchema, PreferenceMergeStore},
    PreferencesDataStore::{
        stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError,
        PreferencesSyncedEntry,
    },
    SyncOperationStore::SyncOperationStore,
};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
struct Fixture {
    root: PathBuf,
    host: Arc<NativeRuntimeStorageHost>,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "operit-preferences-upgrade-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let host = Arc::new(NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        ));
        Self { root, host }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}
/// Declares just the paths used by the calling fixture.
fn fixture_merge_schema(paths: &[&str]) -> PreferenceMergeSchema {
    PreferenceMergeSchema {
        stores: paths.iter().map(|path| PreferenceMergeStore::values(
            *path, false, |storage, path| PreferencesDataStore::newWithStorage(storage, path),
            |_| Ok(()),
        )).collect(),
    }
}

#[test]
fn replay_keeps_existing_unjournaled_baseline() {
    let a = Fixture::new();
    let b = Fixture::new();
    let path = "runtime/config/preferences/review-counter.preferences.json";
    a.host
        .writeBytesAtomically(path, br#"{"counter":"10"}"#)
        .unwrap();
    let local = PreferencesDataStore::newWithStorage(a.host.clone(), path).withSchema(1, |_, p| {
        let key = stringPreferencesKey("counter");
        if let Some(encoded) = p
            .get(&key)
            .and_then(|value| value.as_str())
            .map(str::to_owned)
        {
            p.set(&key, serde_json::from_str::<i64>(&encoded)?);
        }
        Ok(())
    });
    let key = stringPreferencesKey("counter");
    local
        .try_edit_result(|p| p.incrementInteger(&key, 2))
        .unwrap();
    assert_eq!(local.data().unwrap().get(&key), Some(&json!(12)));
    let remote = PreferencesDataStore::newWithStorage(b.host.clone(), path).withSchema(1, |_, _| Ok(()));
    remote
        .try_edit_result(|p| p.incrementInteger(&key, 3))
        .unwrap();
    let incoming = SyncOperationStore::new(b.host.clone(), "runtime/sync")
        .preferenceHistory()
        .unwrap();
    applyOperations(
        a.host.clone(),
        &fixture_merge_schema(&[&path]),
        &[],
        &incoming,
        |_, _| true,
    )
    .unwrap();
    assert_eq!(local.data().unwrap().get(&key), Some(&json!(15)));
}
/// Migrates local historical values through the same contract as the owner's wire decoder.
fn migrate_old(version: u32, preferences: &mut operit_store::PreferencesDataStore::Preferences) -> Result<(), PreferencesDataStoreError> {
    if version == 0 {
        for (key, value) in preferences.entries() {
            if let Some(encoded) = value.as_str() {
                preferences.set(&stringPreferencesKey(&key), serde_json::from_str::<serde_json::Value>(encoded)?);
            }
        }
    }
    Ok(())
}

fn decode_old(entry: &mut PreferencesSyncedEntry) -> Result<(), PreferencesDataStoreError> {
    entry.decodeHistoricalValues(|version, p| {
        if version == 0 {
            let existing = p.entries();
            for (key, value) in existing {
                if let Some(encoded) = value.as_str() {
                    p.set(
                        &stringPreferencesKey(&key),
                        serde_json::from_str::<serde_json::Value>(encoded)?,
                    );
                }
            }
        }
        Ok(())
    })
}
#[test]
fn syncing_new_record_migrates_existing_local_records_before_promoting_schema() {
    let a = Fixture::new();
    let b = Fixture::new();
    let path = "runtime/config/preferences/review-representation.preferences.json";
    a.host
        .writeBytesAtomically(path, br#"{"old_record":"{\"enabled\":true}"}"#)
        .unwrap();
    let remote = PreferencesDataStore::newWithStorage(b.host.clone(), path).withSchema(1, |_, _| Ok(()));
    remote
        .edit(|p| {
            p.set(
                &stringPreferencesKey("new_record"),
                json!({"enabled":false}),
            )
        })
        .unwrap();
    let incoming = SyncOperationStore::new(b.host.clone(), "runtime/sync")
        .preferenceHistory()
        .unwrap();
    let schema = PreferenceMergeSchema {
        stores: vec![PreferenceMergeStore::values(path, false, |storage, path| PreferencesDataStore::newWithStorage(storage, path).withSchema(1, migrate_old), decode_old)],
    };
    applyOperations(a.host.clone(), &schema, &[], &incoming, |_, _| true).unwrap();
    let local = PreferencesDataStore::newWithStorage(a.host.clone(), path).withSchema(1, |_, p| {
        if let Some(encoded) = p
            .get(&stringPreferencesKey("old_record"))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
        {
            p.set(
                &stringPreferencesKey("old_record"),
                serde_json::from_str::<serde_json::Value>(&encoded)?,
            );
        }
        Ok(())
    });
    assert_eq!(local.schemaVersion().unwrap(), 1);
    assert_eq!(
        local
            .data()
            .unwrap()
            .get(&stringPreferencesKey("old_record")),
        Some(&json!({"enabled":true}))
    );
}

const COUNTER: &str = "runtime/config/preferences/upgrade-counter.preferences.json";

fn counter_schema() -> PreferenceMergeSchema {
    PreferenceMergeSchema {
        stores: vec![PreferenceMergeStore::values(COUNTER, false, |storage, path| PreferencesDataStore::newWithStorage(storage, path).withSchema(1, migrate_old), decode_old)],
    }
}

fn history(fixture: &Fixture) -> Vec<operit_store::SyncOperationStore::SyncOperation> {
    SyncOperationStore::new(fixture.host.clone(), "runtime/sync")
        .preferenceHistory()
        .unwrap()
}

fn counter(fixture: &Fixture) -> i64 {
    PreferencesDataStore::newWithStorage(fixture.host.clone(), COUNTER)
        .data()
        .unwrap()
        .get(&stringPreferencesKey("counter"))
        .unwrap()
        .as_i64()
        .unwrap()
}

fn apply(fixture: &Fixture, operations: &[operit_store::SyncOperationStore::SyncOperation]) {
    applyOperations(
        fixture.host.clone(),
        &counter_schema(),
        &[],
        operations,
        |_, _| true,
    )
    .unwrap();
}

#[test]
fn upgraded_devices_and_empty_receiver_converge_with_reversed_and_duplicate_delivery() {
    let a = Fixture::new();
    let b = Fixture::new();
    let c = Fixture::new();
    for (fixture, delta) in [(&a, 2), (&b, 3)] {
        fixture
            .host
            .writeBytesAtomically(COUNTER, br#"{"counter":"10"}"#)
            .unwrap();
        // Preparation imports untouched old data using the same owner path as sync.
        apply(fixture, &[]);
        PreferencesDataStore::newWithStorage(fixture.host.clone(), COUNTER)
            .try_edit_result(|p| p.incrementInteger(&stringPreferencesKey("counter"), delta))
            .unwrap();
    }
    let all = history(&a)
        .into_iter()
        .chain(history(&b))
        .collect::<Vec<_>>();
    for fixture in [&a, &b, &c] {
        for operation in all.iter().rev().chain(all.iter()) {
            apply(fixture, std::slice::from_ref(operation));
        }
        assert_eq!(counter(fixture), 15);
        let originals = history(fixture);
        apply(fixture, &all);
        assert_eq!(history(fixture), originals);
    }
}

/// An upgraded source republishes its local state in the new version and older operations are skipped.
#[test]
fn schema_upgrade_republishes_current_values_without_replaying_old_versions() {
    let source = Fixture::new();
    source.host.writeBytesAtomically(COUNTER, br#"{"counter":"10"}"#).unwrap();
    apply(&source, &[]);
    let key = stringPreferencesKey("counter");
    PreferencesDataStore::newWithStorage(source.host.clone(), COUNTER)
        .withSchema(1, |_, _| Ok(()))
        .try_edit_result(|values| values.incrementInteger(&key, 2)).unwrap();
    let before = history(&source);
    let upgraded = PreferencesDataStore::newWithStorage(source.host.clone(), COUNTER)
        .withSchema(42, |_, _| Ok(()));
    assert_eq!(upgraded.schemaVersion().unwrap(), 42);
    assert_eq!(history(&source), before);
    upgraded.try_edit_result(|values| values.incrementInteger(&key, 4)).unwrap();
    let operations = history(&source);
    assert!(operations.iter().any(|operation| operation.payload["schemaVersion"] == 42));
    let schema = PreferenceMergeSchema {
        stores: vec![PreferenceMergeStore::values(COUNTER, false, |storage, path| PreferencesDataStore::newWithStorage(storage, path).withSchema(42, migrate_old), decode_old)],
    };
    for reverse in [false, true] {
        let target = Fixture::new();
        let batch = if reverse { operations.iter().rev().cloned().collect::<Vec<_>>() } else { operations.clone() };
        for _ in 0..2 {
            let outcomes = operit_store::PreferenceMerge::applyIndependentOperations(
                target.host.clone(), &schema, &[], &batch, |_, _| true);
            assert!(outcomes.iter().any(|outcome| outcome.skipped));
            assert!(outcomes.iter().any(|outcome| outcome.result.is_ok()));
            assert_eq!(counter(&target), 16);
            assert!(history(&target).iter().all(|operation| operation.payload["schemaVersion"] == 42));
        }
    }
}

#[test]
fn incoming_delta_migrates_the_old_local_value_before_importing_it() {
    let a = Fixture::new();
    let b = Fixture::new();
    a.host
        .writeBytesAtomically(COUNTER, br#"{"counter":"10"}"#)
        .unwrap();
    PreferencesDataStore::newWithStorage(b.host.clone(), COUNTER)
        .withSchema(1, |_, _| Ok(()))
        .try_edit_result(|p| p.incrementInteger(&stringPreferencesKey("counter"), 3))
        .unwrap();
    apply(&a, &history(&b));
    assert_eq!(counter(&a), 13);
}

#[test]
fn existing_delta_history_derives_initial_value_without_counting_deltas_twice() {
    let a = Fixture::new();
    let b = Fixture::new();
    PreferencesDataStore::newWithStorage(b.host.clone(), COUNTER)
        .withSchema(1, |_, _| Ok(()))
        .try_edit_result(|p| p.incrementInteger(&stringPreferencesKey("counter"), 2))
        .unwrap();
    let operations = history(&b);
    // Simulates a pre-upgrade dataset: committed value includes a recorded delta, but no initial set.
    a.host
        .writeBytesAtomically(COUNTER, br#"{"counter":12}"#)
        .unwrap();
    let journal = SyncOperationStore::new(a.host.clone(), "runtime/sync");
    journal.appendOperations(&operations).unwrap();
    apply(&a, &[]);
    assert_eq!(counter(&a), 12);
    let imported = history(&a);
    apply(&b, &imported);
    assert_eq!(counter(&b), 12);
    apply(&a, &imported);
    assert_eq!(history(&a), imported);
}

#[test]
fn failed_migration_preserves_data_schema_and_history_and_can_retry() {
    fn reject(_: &mut PreferencesSyncedEntry) -> Result<(), PreferencesDataStoreError> {
        Err(PreferencesDataStoreError::Message(
            "injected migration failure".into(),
        ))
    }
    let a = Fixture::new();
    let before = br#"{"counter":"10"}"#;
    a.host.writeBytesAtomically(COUNTER, before).unwrap();
    let invalid = PreferenceMergeSchema {
        stores: vec![PreferenceMergeStore::values(COUNTER, false, |storage, path| PreferencesDataStore::newWithStorage(storage, path).withSchema(1, |_, _| Err(PreferencesDataStoreError::Message("injected migration failure".into()))), reject)],
    };
    assert!(applyOperations(a.host.clone(), &invalid, &[], &[], |_, _| true).is_err());
    assert_eq!(a.host.readBytes(COUNTER).unwrap(), before);
    assert!(history(&a).is_empty());
    apply(&a, &[]);
    assert_eq!(counter(&a), 10);
    assert_eq!(history(&a).len(), 1);
}

#[test]
fn existing_typed_roots_are_not_reimported_or_decoded_as_legacy_strings() {
    let a = Fixture::new();
    let b = Fixture::new();
    let path = "runtime/config/preferences/mixed-upgrade.preferences.json";
    let typed = stringPreferencesKey("typed");
    PreferencesDataStore::newWithStorage(b.host.clone(), path)
        .withSchema(1, |_, _| Ok(()))
        .edit(|p| p.set(&typed, "a plain string"))
        .unwrap();
    let originals = history(&b);
    let journal = SyncOperationStore::new(a.host.clone(), "runtime/sync");
    journal.appendOperations(&originals).unwrap();
    a.host
        .writeBytesAtomically(
            path,
            br#"{"old":"{\"enabled\":true}","typed":"a plain string"}"#,
        )
        .unwrap();
    let schema = PreferenceMergeSchema {
        stores: vec![PreferenceMergeStore::values(path, false, |storage, path| PreferencesDataStore::newWithStorage(storage, path).withSchema(1, |_, preferences| {
            let key = stringPreferencesKey("old");
            if let Some(encoded) = preferences.getTyped::<String>(&key)? {
                preferences.set(&key, serde_json::from_str::<serde_json::Value>(&encoded)?);
            }
            Ok(())
        }), decode_old)],
    };
    applyOperations(a.host.clone(), &schema, &[], &[], |_, _| true).unwrap();
    let data = PreferencesDataStore::newWithStorage(a.host.clone(), path)
        .data()
        .unwrap();
    assert_eq!(data.get(&typed), Some(&json!("a plain string")));
    assert_eq!(
        data.get(&stringPreferencesKey("old")),
        Some(&json!({"enabled":true}))
    );
    let imported = history(&a);
    assert_eq!(imported.len(), originals.len() + 1);
    assert!(imported.contains(&originals[0]));
    applyOperations(a.host.clone(), &schema, &[], &[], |_, _| true).unwrap();
    assert_eq!(history(&a), imported);
}

/// Builds immutable legacy field edits and complete native assignments in one source stream.
fn replay_operation(sequence: i64, operation: &str, mut payload: serde_json::Value) -> operit_store::SyncOperationStore::SyncOperation {
    use operit_store::SyncOperationStore::{SyncOperation, SyncOperationSemantics};
    if payload.get("schemaVersion").is_none() { payload["schemaVersion"] = json!(0); }
    let path = payload["storagePath"].as_str().unwrap();
    let entity = if operation == "transaction" {
        let keys = payload["entries"].as_array().unwrap().iter()
            .map(|entry| entry["key"].as_str().unwrap()).collect::<std::collections::BTreeSet<_>>();
        serde_json::to_string(&(path, keys)).unwrap()
    } else if let Some(json_path) = payload.get("jsonPath") {
        serde_json::to_string(&(path, payload["key"].as_str().unwrap(), json_path)).unwrap()
    } else {
        serde_json::to_string(&(path, payload["key"].as_str().unwrap())).unwrap()
    };
    SyncOperation {
        opId: format!("legacy-provider:{sequence}"), originDeviceId: "legacy-provider".into(),
        sequence, domain: "preferences".into(), entityType: "model_configs".into(), entityId: entity,
        operation: operation.into(), semantics: if operation == "transaction" {
            SyncOperationSemantics::Transaction
        } else { SyncOperationSemantics::EntityState },
        payload, createdAt: sequence, schemaVersion: 1,
    }
}

/// Builds the old field-level format without a fabricated provider creation event.
fn legacy_provider_field(path: &str, sequence: i64) -> operit_store::SyncOperationStore::SyncOperation {
    replay_operation(sequence, "set", json!({
        "storagePath": path, "key": "provider_DEEPSEEK", "encrypted": false,
        "value": null, "jsonPath": [{"kind":"field","value":"name"}],
        "jsonMutation": {"kind":"set","value":"older name"}
    }))
}

/// Skips old-schema field edits before replaying a current-schema complete assignment.
#[test]
fn replay_skips_old_version_fields_before_current_root_assignment() {
    let path = "runtime/config/preferences/late-provider-root.preferences.json";
    let provider = json!({"id":"DEEPSEEK", "name":"current name", "models":[]});
    let field = legacy_provider_field(path, 7);
    let root = replay_operation(6735, "transaction", json!({
        "storagePath":path, "encrypted":false, "formatVersion":2, "schemaVersion":4,
        "entries":[
            {"key":"provider_DEEPSEEK","operation":"set","value":provider},
            {"key":"provider_list","operation":"delete","value":null}
        ]
    }));
    for existing in [false, true] {
        let fixture = Fixture::new();
        if existing {
            fixture.host.writeBytesAtomically(path, &serde_json::to_vec(&json!({
                "provider_DEEPSEEK":provider
            })).unwrap()).unwrap();
            SyncOperationStore::new(fixture.host.clone(), "runtime/sync")
                .appendOperations(&[field.clone(), root.clone()]).unwrap();
        }
        PreferencesDataStore::newWithStorage(fixture.host.clone(), path)
            .withSchema(4, |_, _| Ok(())).data().unwrap();
        for batch in [vec![root.clone(), field.clone()], vec![field.clone(), root.clone()]] {
            let outcomes = operit_store::PreferenceMerge::applyIndependentOperations(
                fixture.host.clone(), &fixture_merge_schema(&[&path]), &[], &batch, |_,_| true);
            assert!(outcomes.iter().any(|outcome| outcome.skipped && outcome.result.is_ok() && outcome.operations == vec![field.clone()]));
            assert!(outcomes.iter().any(|outcome| outcome.result.is_ok() && outcome.operations.contains(&root)));
            let data = PreferencesDataStore::newWithStorage(fixture.host.clone(), path).data().unwrap();
            assert_eq!(data.get(&stringPreferencesKey("provider_DEEPSEEK")), Some(&provider));
            let originals = history(&fixture);
            assert_eq!(originals.contains(&field), existing);
            assert!(originals.contains(&root));
        }
    }
}

/// Preserves increments and sibling transaction mutations while eliminating superseded root effects.
#[test]
fn effective_replay_preserves_sibling_mutations_and_following_edits() {
    let path = "runtime/config/preferences/replay-transaction.preferences.json";
    let fixture = Fixture::new();
    let first = replay_operation(1, "transaction", json!({
        "storagePath":path,"encrypted":false,"formatVersion":2,
        "entries":[
            {"key":"provider_DEEPSEEK","operation":"set","value":{"name":"first"}},
            {"key":"counter","operation":"increment","value":2}
        ]
    }));
    let replacement = replay_operation(3, "set", json!({
        "storagePath":path,"encrypted":false,"formatVersion":2,
        "key":"provider_DEEPSEEK","value":{"name":"replacement"}
    }));
    let operations = vec![first, legacy_provider_field(path, 2), replacement, legacy_provider_field(path, 4)];
    for _ in 0..3 {
        applyOperations(fixture.host.clone(), &fixture_merge_schema(&[&path]), &[], &operations, |_,_| true).unwrap();
        let data = PreferencesDataStore::newWithStorage(fixture.host.clone(), path).data().unwrap();
        assert_eq!(data.get(&stringPreferencesKey("counter")), Some(&json!(2)));
        assert_eq!(data.get(&stringPreferencesKey("provider_DEEPSEEK")), Some(&json!({"name":"older name"})));
    }
}

/// Treats root deletion as an assignment while rejecting a later field edit on a deleted entity.
#[test]
fn effective_replay_honors_deletion_and_rejects_unresolved_field_dependencies() {
    let path = "runtime/config/preferences/replay-tombstone.preferences.json";
    let fixture = Fixture::new();
    let field = legacy_provider_field(path, 1);
    let deletion = replay_operation(2, "delete", json!({
        "storagePath":path,"key":"provider_DEEPSEEK","value":null,"encrypted":false,"formatVersion":2
    }));
    let originals = vec![field, deletion];
    applyOperations(fixture.host.clone(), &fixture_merge_schema(&[&path]), &[], &originals, |_,_| true).unwrap();
    let committed = fixture.host.readBytes(path).unwrap();
    let later = legacy_provider_field(path, 3);
    let error = applyOperations(fixture.host.clone(), &fixture_merge_schema(&[&path]), &[], &[later], |_,_| true).unwrap_err();
    assert!(error.contains("structured preference entry is missing"));
    assert_eq!(fixture.host.readBytes(path).unwrap(), committed);
    assert_eq!(history(&fixture), originals);
}

/// A surviving field dependency fails the complete dataset transaction before any sibling is published.
#[test]
fn effective_replay_commits_no_sibling_of_an_invalid_transaction() {
    let path = "runtime/config/preferences/replay-atomic.preferences.json";
    let fixture = Fixture::new();
    let initial = replay_operation(1, "set", json!({
        "storagePath":path,"key":"counter","value":10,"encrypted":false,"formatVersion":2
    }));
    applyOperations(fixture.host.clone(), &fixture_merge_schema(&[&path]), &[], &[initial], |_,_| true).unwrap();
    let committed = fixture.host.readBytes(path).unwrap();
    let transaction = replay_operation(3, "transaction", json!({
        "storagePath":path,"encrypted":false,"formatVersion":2,
        "entries":[
            {"key":"counter","operation":"increment","value":2},
            {"key":"other","operation":"set","value":"uncommitted"}
        ]
    }));
    let operations = [legacy_provider_field(path, 2), transaction];
    assert!(applyOperations(fixture.host.clone(), &fixture_merge_schema(&[&path]), &[], &operations, |_,_| true).is_err());
    assert_eq!(fixture.host.readBytes(path).unwrap(), committed);
}

/// Keeps committed ordinary datasets out of an unrelated delta's replay and lock scope.
#[test]
fn incremental_replay_only_materializes_the_changed_dataset() {
    use operit_store::SyncOperationStore::{SyncOperation, SyncOperationSemantics};
    let fixture = Fixture::new();
    let paths = (0..50).map(|dataset| format!("{}/probe-{dataset}.preferences.json",
        operit_util::RuntimeStorageLayout::CONFIG_PREFERENCES_DIR_PATH)).collect::<Vec<_>>();
    let mut history = Vec::new();
    for dataset in 0..50 {
        for sequence in 1..=200 {
            let origin = format!("probe-{dataset}");
            let path = &paths[dataset];
            history.push(SyncOperation {
                opId: format!("{origin}:{sequence}"), originDeviceId: origin, sequence,
                domain: "preferences".into(), entityType: "entry".into(),
                entityId: serde_json::to_string(&(&path, "value")).unwrap(),
                operation: "set".into(), semantics: SyncOperationSemantics::EntityState,
                payload: json!({"storagePath":path,"key":"value","value":sequence,
                    "encrypted":false,"formatVersion":1,"schemaVersion":0,"contractVersion":1,"logicalTime":sequence}),
                createdAt: sequence, schemaVersion: 1,
            });
        }
    }
    let schema = fixture_merge_schema(&paths.iter().map(String::as_str).collect::<Vec<_>>());
    applyOperations(fixture.host.clone(), &schema, &[], &history, |_,_| true).unwrap();
    let journal = SyncOperationStore::new(fixture.host.clone(), "runtime/sync");
    let start = operit_host_api::TimeUtils::currentTimeMillis();
    let history = journal.preferenceHistory().unwrap();
    eprintln!("PROBE historyMs={} count={}", (operit_host_api::TimeUtils::currentTimeMillis() - start), history.len());
    let mut delta = history[0].clone();
    delta.sequence = 201;
    delta.opId = format!("{}:201", delta.originDeviceId);
    delta.createdAt = 201;
    delta.payload["logicalTime"] = json!(201);
    delta.payload["value"] = json!(201);
    let start = operit_host_api::TimeUtils::currentTimeMillis();
    let path = delta.payload["storagePath"].as_str().unwrap().to_owned();
    let outcomes = operit_store::PreferenceMerge::applyIndependentOperations(
        fixture.host.clone(), &schema, &history, &[delta.clone()], |_,_| true,
    );
    eprintln!("PROBE oneDeltaMs={}", (operit_host_api::TimeUtils::currentTimeMillis() - start));
    assert_eq!(outcomes.len(), 1, "unrelated history must not acquire dataset locks or replay");
    assert_eq!(outcomes[0].paths, std::collections::BTreeSet::from([path.clone()]));
    assert!(outcomes[0].result.is_ok());
    assert_eq!(outcomes[0].operations, vec![delta]);
    let current = PreferencesDataStore::newWithStorage(fixture.host.clone(), path).data().unwrap();
    assert_eq!(current.get(&stringPreferencesKey("value")), Some(&json!(201)));
    assert_eq!(journal.preferenceHistory().unwrap().len(), 10_001);
}

/// Duplicate delivery validates immutable content without decoding committed business history again.
#[test]
fn duplicate_delivery_skips_replay_but_rejects_changed_originals() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DECODED: AtomicUsize = AtomicUsize::new(0);
    /// Counts owner decodes while preserving the historical representation contract.
    fn decode(entry: &mut PreferencesSyncedEntry) -> Result<(), PreferencesDataStoreError> {
        DECODED.fetch_add(1, Ordering::SeqCst);
        decode_old(entry)
    }
    let fixture = Fixture::new();
    let path = "runtime/config/preferences/duplicate-replay.preferences.json";
    let original = replay_operation(1, "set", json!({
        "storagePath":path,"key":"counter","value":10,"encrypted":false,"formatVersion":2,"schemaVersion":1
    }));
    let schema = PreferenceMergeSchema {
        stores: vec![PreferenceMergeStore::values(path, false, |storage, path| PreferencesDataStore::newWithStorage(storage, path).withSchema(1, migrate_old), decode)],
    };
    applyOperations(fixture.host.clone(), &schema, &[], &[original.clone()], |_,_|true).unwrap();
    let committed = fixture.host.readBytes(path).unwrap();
    DECODED.store(0, Ordering::SeqCst);
    applyOperations(fixture.host.clone(), &schema, &[], &[original.clone()], |_,_|true).unwrap();
    assert_eq!(DECODED.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.host.readBytes(path).unwrap(), committed);
    let mut conflicting = original.clone();
    conflicting.payload["value"] = json!(999);
    assert!(applyOperations(fixture.host.clone(), &schema, &[], &[conflicting], |_,_|true).is_err());
    assert_eq!(fixture.host.readBytes(path).unwrap(), committed);
    let mut later = original.clone();
    later.sequence = 2;
    later.opId = format!("{}:2", later.originDeviceId);
    later.payload["value"] = json!(12);
    applyOperations(fixture.host.clone(), &schema, &[], &[later], |_,_|true).unwrap();
    assert!(DECODED.load(Ordering::SeqCst) > 0);
    assert_eq!(PreferencesDataStore::newWithStorage(fixture.host.clone(), path).data().unwrap()
        .get(&stringPreferencesKey("counter")), Some(&json!(12)));
}

/// Cross-owner merge context excludes ordinary values while preserving declarations and alias identities.
#[test]
fn merge_history_contains_only_declarations_and_aliases() {
    let fixture = Fixture::new();
    let path = "runtime/config/preferences/merge-context.preferences.json";
    let business = replay_operation(1, "set", json!({
        "storagePath":path,"key":"counter","value":10,"encrypted":false,"formatVersion":2
    }));
    let declaration = replay_operation(2, "set", json!({
        "storagePath":path,"key":operit_store::PreferenceMerge::PLUGIN_MERGE_DECLARATION_KEY,
        "value":{"version":1},"encrypted":false,"formatVersion":2
    }));
    let alias = replay_operation(3, "alias", json!({
        "storagePath":path,"key":"old","value":"current","encrypted":false,"formatVersion":2
    }));
    let journal = SyncOperationStore::new(fixture.host.clone(), "runtime/sync");
    journal.appendOperations(&[business, declaration.clone(), alias.clone()]).unwrap();
    for _ in 0..2 {
        assert_eq!(journal.preferenceMergeHistory().unwrap(), vec![declaration.clone(), alias.clone()]);
        assert_eq!(journal.preferenceHistory().unwrap().len(), 3);
    }
}
