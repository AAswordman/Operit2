//! Equal preference schemas are a prerequisite for both entry and transaction replication.
use std::{path::PathBuf, sync::Arc};

use operit_host_api::RuntimeStorageHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::{
    PreferenceMerge::{applyIndependentOperations, PreferenceMergeSchema, PreferenceMergeStore},
    PreferencesDataStore::{
        stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError,
        PreferencesSyncedEntry,
    },
    SyncOperationStore::{SyncOperation, SyncOperationKnowledge, SyncOperationStore},
};
use serde_json::{json, Value};

const PATH: &str = "runtime/config/preferences/version-check.preferences.json";
const VERSION_KEY: &str = "__operit_preferences_schema_version";

struct Fixture {
    root: PathBuf,
    host: Arc<NativeRuntimeStorageHost>,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "operit-preference-versions-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let host = Arc::new(NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        ));
        Self { root, host }
    }

    fn store(&self, path: &str, version: u32) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(self.host.clone(), path)
            .withSchema(version, |_, _| Ok(()))
    }

    fn history(&self) -> Vec<SyncOperation> {
        SyncOperationStore::new(self.host.clone(), "runtime/sync")
            .preferenceHistory()
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

fn current_values(_: &mut PreferencesSyncedEntry) -> Result<(), PreferencesDataStoreError> {
    Ok(())
}

#[test]
fn outbound_entries_transactions_and_imported_baselines_carry_the_preference_version() {
    let source = Fixture::new();
    source
        .host
        .writeBytes(
            PATH,
            &serde_json::to_vec(&json!({VERSION_KEY:3,"existing":true})).unwrap(),
        )
        .unwrap();
    let store = source.store(PATH, 3);
    store
        .edit(|p| p.set(&stringPreferencesKey("single"), 1))
        .unwrap();
    store
        .edit(|p| {
            p.set(&stringPreferencesKey("left"), 2);
            p.set(&stringPreferencesKey("right"), 3);
        })
        .unwrap();
    let operations = source.history();
    assert!(operations.iter().any(|op| op.operation == "transaction"));
    assert!(operations.iter().any(|op| op.payload["key"] == "existing"));
    assert!(operations.iter().all(|op| op.payload["schemaVersion"] == 3));
}

#[test]
fn older_newer_and_unversioned_data_do_not_change_local_values_or_schema() {
    for version in [0, 1, 3] {
        let source = Fixture::new();
        source
            .store(PATH, version)
            .edit(|p| p.set(&stringPreferencesKey("settings"), json!({"remote":true})))
            .unwrap();
        let mut incoming = source.history();
        if version == 0 {
            for operation in &mut incoming {
                operation
                    .payload
                    .as_object_mut()
                    .unwrap()
                    .remove("schemaVersion");
            }
        }
        let target = Fixture::new();
        let store = target.store(PATH, 2);
        store
            .edit(|p| p.set(&stringPreferencesKey("settings"), json!({"local":true})))
            .unwrap();
        let before = target.host.readBytes(PATH).unwrap();
        let outcomes = applyIndependentOperations(
            target.host.clone(),
            &Default::default(),
            &[],
            &incoming,
            |_, _| true,
        );
        assert!(outcomes.iter().all(|outcome| {
            outcome.skipped == (version < 2) && outcome.result.is_ok() == (version < 2)
        }));
        assert_eq!(target.host.readBytes(PATH).unwrap(), before);
        assert_eq!(store.schemaVersion().unwrap(), 2);
        assert_eq!(
            store.data().unwrap().get(&stringPreferencesKey("settings")),
            Some(&json!({"local":true}))
        );
        assert!(!target
            .history()
            .iter()
            .any(|op| incoming.iter().any(|remote| remote.opId == op.opId)));
    }
}

#[test]
fn matching_data_and_unrelated_files_commit_even_when_old_data_is_skipped() {
    let old = Fixture::new();
    old.store(PATH, 1)
        .edit(|p| p.set(&stringPreferencesKey("stale"), true))
        .unwrap();
    let source = Fixture::new();
    source
        .store(PATH, 2)
        .edit(|p| {
            p.set(&stringPreferencesKey("left"), 1);
            p.set(&stringPreferencesKey("right"), 2);
        })
        .unwrap();
    let other_path = "runtime/config/preferences/other-version-check.preferences.json";
    source
        .store(other_path, 0)
        .edit(|p| p.set(&stringPreferencesKey("healthy"), true))
        .unwrap();
    let target = Fixture::new();
    let local = target.store(PATH, 2);
    local.data().unwrap();
    let mut incoming = old.history();
    incoming.extend(source.history());
    let outcomes = applyIndependentOperations(
        target.host.clone(),
        &Default::default(),
        &[],
        &incoming,
        |_, _| true,
    );
    assert_eq!(outcomes.iter().filter(|outcome| outcome.skipped).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| !outcome.skipped && outcome.result.is_ok())
            .count(),
        2
    );
    let data = local.data().unwrap();
    assert!(!data.contains(&stringPreferencesKey("stale")));
    assert_eq!(data.get(&stringPreferencesKey("left")), Some(&json!(1)));
    assert_eq!(data.get(&stringPreferencesKey("right")), Some(&json!(2)));
    assert_eq!(
        target
            .store(other_path, 0)
            .data()
            .unwrap()
            .get(&stringPreferencesKey("healthy")),
        Some(&json!(true))
    );
}

#[test]
fn registered_owner_accepts_its_version_on_a_new_device_without_adopting_sender_versions() {
    for remote_version in [1, 2, 3] {
        let source = Fixture::new();
        source
            .store(PATH, remote_version)
            .edit(|p| p.set(&stringPreferencesKey("value"), 7))
            .unwrap();
        let target = Fixture::new();
        let schema = PreferenceMergeSchema {
            stores: vec![PreferenceMergeStore::values(PATH, false, 2, current_values)],
        };
        let outcomes = applyIndependentOperations(
            target.host.clone(),
            &schema,
            &[],
            &source.history(),
            |_, _| true,
        );
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].result.is_ok(), remote_version <= 2);
        assert_eq!(outcomes[0].skipped, remote_version < 2);
        if remote_version == 2 {
            assert_eq!(
                target
                    .store(PATH, 2)
                    .data()
                    .unwrap()
                    .get(&stringPreferencesKey("value")),
                Some(&json!(7))
            );
        } else {
            assert!(!target.host.exists(PATH).unwrap());
        }
    }
}

#[test]
fn replay_does_not_resurrect_an_archived_operation_from_an_older_schema() {
    let old = Fixture::new();
    old.store(PATH, 1)
        .edit(|p| p.set(&stringPreferencesKey("legacy_only"), "encoded old data"))
        .unwrap();
    let source = Fixture::new();
    source
        .store(PATH, 2)
        .edit(|p| p.set(&stringPreferencesKey("current"), true))
        .unwrap();
    let target = Fixture::new();
    let local = target.store(PATH, 2);
    local.data().unwrap();
    let journal = SyncOperationStore::new(target.host.clone(), "runtime/sync");
    let legacy = old.history();
    journal.appendOperations(&legacy).unwrap();
    for _ in 0..2 {
        let outcomes = applyIndependentOperations(
            target.host.clone(),
            &Default::default(),
            &legacy,
            &source.history(),
            |_, _| true,
        );
        assert!(outcomes.iter().all(|outcome| outcome.result.is_ok()));
        let data = local.data().unwrap();
        assert!(!data.contains(&stringPreferencesKey("legacy_only")));
        assert_eq!(
            data.get(&stringPreferencesKey("current")),
            Some(&json!(true))
        );
    }
    assert!(target.history().iter().any(|op| op.opId == legacy[0].opId));
    let disk: Value = serde_json::from_slice(&target.host.readBytes(PATH).unwrap()).unwrap();
    assert!(disk["__operit_preferences_replication"]["applied"]
        .get(&legacy[0].opId)
        .is_none());
}

#[test]
fn local_migration_republishes_existing_values_in_the_new_schema() {
    let source = Fixture::new();
    source
        .store(PATH, 1)
        .edit(|p| p.set(&stringPreferencesKey("kept"), json!({"local":true})))
        .unwrap();
    let upgraded = source.store(PATH, 2);
    upgraded.data().unwrap();
    upgraded
        .edit(|p| p.set(&stringPreferencesKey("edited"), 9))
        .unwrap();
    let incoming = source.history();
    let target = Fixture::new();
    let local = target.store(PATH, 2);
    local.data().unwrap();
    let outcomes = applyIndependentOperations(
        target.host.clone(),
        &Default::default(),
        &[],
        &incoming,
        |_, _| true,
    );
    assert!(outcomes.iter().any(|outcome| outcome.skipped));
    assert!(outcomes.iter().any(|outcome| outcome.result.is_ok()));
    let data = local.data().unwrap();
    assert_eq!(
        data.get(&stringPreferencesKey("kept")),
        Some(&json!({"local":true}))
    );
    assert_eq!(data.get(&stringPreferencesKey("edited")), Some(&json!(9)));
}

#[test]
fn missing_wire_version_does_not_match_an_unversioned_local_file() {
    let source = Fixture::new();
    source
        .store(PATH, 0)
        .edit(|p| p.set(&stringPreferencesKey("value"), true))
        .unwrap();
    let mut incoming = source.history();
    for operation in &mut incoming {
        operation
            .payload
            .as_object_mut()
            .unwrap()
            .remove("schemaVersion");
    }
    let target = Fixture::new();
    let outcomes = applyIndependentOperations(
        target.host.clone(),
        &Default::default(),
        &[],
        &incoming,
        |_, _| true,
    );
    assert!(outcomes
        .iter()
        .all(|outcome| outcome.skipped && outcome.result.is_ok()));
    assert!(!target.host.exists(PATH).unwrap());
}

/// Legacy field edits cannot modify the migrated file or keep the receiver retrying.
#[test]
fn obsolete_and_unversioned_operations_leave_pending_and_never_reenter_after_redelivery() {
    for missing in [false, true] {
        let source = Fixture::new();
        source
            .store(PATH, 1)
            .edit(|p| {
                p.set(
                    &stringPreferencesKey("settings"),
                    json!({"subpackageStates":{}}),
                )
            })
            .unwrap();
        let mut incoming = source.history();
        let operation = incoming.last_mut().unwrap();
        operation.payload["value"] = Value::Null;
        operation.payload["jsonPath"] = json!([{"kind":"field","value":"subpackageStates"},{"kind":"field","value":"worldbook"}]);
        operation.payload["jsonMutation"] = json!({"kind":"set","value":true});
        operation.payload["formatVersion"] = json!(1);
        operation.entityId =
            serde_json::to_string(&(PATH, "settings", &operation.payload["jsonPath"])).unwrap();
        if missing {
            operation
                .payload
                .as_object_mut()
                .unwrap()
                .remove("schemaVersion");
        }
        let target = Fixture::new();
        let local = target.store(PATH, 2);
        local.data().unwrap();
        let before = target.host.readBytes(PATH).unwrap();
        let journal = SyncOperationStore::new(target.host.clone(), "runtime/sync");
        journal.receiveOperations(&incoming).unwrap();
        let pending = journal.pendingOperations().unwrap();
        assert_eq!(pending, incoming);
        let outcomes = applyIndependentOperations(
            target.host.clone(),
            &Default::default(),
            &[],
            &pending,
            |_, _| true,
        );
        for outcome in outcomes {
            assert!(
                outcome.skipped && outcome.result.is_ok(),
                "{:?}",
                outcome.result
            );
            journal.skipOperations(&outcome.operations).unwrap();
        }
        assert_eq!(journal.pendingOperationCount().unwrap(), 0);
        assert_eq!(target.host.readBytes(PATH).unwrap(), before);
        assert_eq!(journal.preferenceHistory().unwrap(), incoming);
        drop(local);
        drop(journal);
        let reopenedHost = Arc::new(NativeRuntimeStorageHost::new(
            target.root.join("runtime"),
            target.root.join("workspaces"),
        ));
        let reopened = SyncOperationStore::new(reopenedHost.clone(), "runtime/sync");
        reopened.receiveOperations(&incoming).unwrap();
        assert_eq!(reopened.pendingOperationCount().unwrap(), 0);
        assert!(reopened
            .receipt()
            .unwrap()
            .containsSequence(&incoming[0].originDeviceId, incoming[0].sequence));
        // An unrelated current-version write must not replay the archived old field edit.
        let current = Fixture::new();
        current
            .store(PATH, 2)
            .edit(|p| p.set(&stringPreferencesKey("healthy"), true))
            .unwrap();
        let outcomes = applyIndependentOperations(
            reopenedHost.clone(),
            &Default::default(),
            &reopened.preferenceMergeHistory().unwrap(),
            &current.history(),
            |_, _| true,
        );
        assert!(outcomes
            .iter()
            .all(|outcome| !outcome.skipped && outcome.result.is_ok()));
        let data = PreferencesDataStore::newWithStorage(reopenedHost, PATH)
            .data()
            .unwrap();
        assert!(!data.contains(&stringPreferencesKey("settings")));
        assert_eq!(
            data.get(&stringPreferencesKey("healthy")),
            Some(&json!(true))
        );
        let mut conflicting = incoming.clone();
        conflicting[0].payload["value"] = json!("changed original");
        assert!(reopened.receiveOperations(&conflicting).is_err());
    }
}

/// A newer operation stays pending until the receiver migrates and can interpret it.
#[test]
fn future_schema_operations_remain_pending_and_apply_after_local_upgrade() {
    let source = Fixture::new();
    source
        .store(PATH, 3)
        .edit(|p| p.set(&stringPreferencesKey("future"), json!({"enabled":true})))
        .unwrap();
    let incoming = source.history();
    let target = Fixture::new();
    target.store(PATH, 2).data().unwrap();
    let before = target.host.readBytes(PATH).unwrap();
    let journal = SyncOperationStore::new(target.host.clone(), "runtime/sync");
    journal.receiveOperations(&incoming).unwrap();
    let outcomes = applyIndependentOperations(
        target.host.clone(),
        &Default::default(),
        &[],
        &journal.pendingOperations().unwrap(),
        |_, _| true,
    );
    assert!(outcomes
        .iter()
        .all(|outcome| !outcome.skipped && outcome.result.is_err()));
    assert_eq!(journal.pendingOperations().unwrap(), incoming);
    assert_eq!(target.host.readBytes(PATH).unwrap(), before);
    let upgraded = target.store(PATH, 3);
    upgraded.data().unwrap();
    let outcomes = applyIndependentOperations(
        target.host.clone(),
        &Default::default(),
        &[],
        &journal.pendingOperations().unwrap(),
        |_, _| true,
    );
    for outcome in outcomes {
        assert!(!outcome.skipped && outcome.result.is_ok());
        journal.completeOperations(&outcome.operations).unwrap();
    }
    assert_eq!(journal.pendingOperationCount().unwrap(), 0);
    assert_eq!(
        upgraded
            .data()
            .unwrap()
            .get(&stringPreferencesKey("future")),
        Some(&json!({"enabled":true}))
    );
}
