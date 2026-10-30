//! Acceptance scenarios run through real preference files, owner rules and the production journal.
use operit_host_api::{PluginStorage::StoragePreferencesMergeDescriptor, RuntimeStorageHost};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::{
    PreferenceLayout::PreferenceLayout,
    PreferenceMerge::{self, PreferenceMergeSchema, PreferenceMergeStore},
    PreferencesDataStore::{
        stringPreferencesKey, Preferences, PreferencesDataStore, PreferencesDataStoreError,
    },
    SyncOperationStore::{SyncOperation, SyncOperationStore},
    Transaction,
};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};

const CURRENT: &str = "runtime/config/preferences/contract.preferences.json";
const OLD: &str = "runtime/config/preferences/old-contract.preferences.json";
const LAYOUT: &str = "runtime/sync/contract-layout.preferences.json";

struct Fixture {
    root: PathBuf,
    host: Arc<NativeRuntimeStorageHost>,
}
impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("operit-business-contract-{}", uuid::Uuid::new_v4()));
        let host = Arc::new(NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        ));
        Self { root, host }
    }
    fn store(&self) -> PreferencesDataStore {
        current_store(self.host.clone(), CURRENT)
    }
    fn journal(&self) -> SyncOperationStore {
        SyncOperationStore::new(self.host.clone(), "runtime/sync")
    }
    fn reopen(&self) -> Arc<NativeRuntimeStorageHost> {
        Arc::new(NativeRuntimeStorageHost::new(
            self.root.join("runtime"),
            self.root.join("workspaces"),
        ))
    }
    fn apply(&self, originals: &[SyncOperation]) {
        apply(self.host.clone(), originals);
    }
    fn value(&self, key: &str) -> Option<Value> {
        self.store()
            .data()
            .unwrap()
            .get(&stringPreferencesKey(key))
            .cloned()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn migrate(
    from: u32,
    context: &mut Transaction::migration::Context,
) -> Result<(), PreferencesDataStoreError> {
    if from == 1 {
        Transaction::Rule::SplitRecords {
            key: "cards".into(),
            prefix: "card_".into(),
            identity: "id".into(),
        }
        .apply(context)
        .map_err(PreferencesDataStoreError::Message)?;
        Transaction::Rule::RenameKey {
            from: "name".into(),
            to: "displayName".into(),
        }
        .apply(context)
        .map_err(PreferencesDataStoreError::Message)?;
        context.map_preferences(from, &|_, values| {
            let key = stringPreferencesKey("encodedSettings");
            if let Some(value) = values.get(&key).cloned() {
                let decoded: Value = serde_json::from_str(value.as_str().ok_or_else(|| {
                    PreferencesDataStoreError::Message("Expected encoded settings".into())
                })?)?;
                values.set(&key, decoded);
            }
            Ok(())
        })?;
        Transaction::Rule::SplitObject {
            key: "encodedSettings".into(),
            fields: std::collections::BTreeMap::from([
                ("theme".into(), "theme".into()),
                ("binding".into(), "binding".into()),
            ]),
        }
        .apply(context)
        .map_err(PreferencesDataStoreError::Message)?;
    }
    Ok(())
}
fn current_store(host: Arc<dyn RuntimeStorageHost>, path: &str) -> PreferencesDataStore {
    PreferencesDataStore::newWithStorage(host, path).withMigration(2, migrate)
}
fn schema() -> PreferenceMergeSchema {
    PreferenceMergeSchema {
        stores: vec![PreferenceMergeStore {
            path: CURRENT.into(),
            encrypted: false,
            open: current_store,
            merge: StoragePreferencesMergeDescriptor {
                version: 1,
                catalog: None,
                references: vec![],
                projection: None,
            },
        }],
    }
}
fn apply(host: Arc<dyn RuntimeStorageHost>, originals: &[SyncOperation]) {
    PreferenceMerge::applyOperations(host, &schema(), &[], originals, |_, _| true).unwrap();
}

/// Authored operations are captured once, with controlled business clocks to model delayed devices.
fn authored(
    path: &str,
    format: u32,
    revision: i64,
    edit: impl FnOnce(&mut Preferences),
) -> Vec<SyncOperation> {
    let source = Fixture::new();
    PreferencesDataStore::newWithStorage(source.host.clone(), path)
        .withSchema(format, |_, _| Ok(()))
        .edit(edit)
        .unwrap();
    source
        .journal()
        .preferenceHistory()
        .unwrap()
        .into_iter()
        .map(|mut operation| {
            operation.createdAt = revision;
            operation.payload["logicalTime"] = json!(revision);
            operation
        })
        .collect()
}

#[test]
fn atomic_partial_overwrites_converge_in_both_arrival_orders() {
    let first = authored(CURRENT, 2, 10, |state| {
        state.set(&stringPreferencesKey("x"), 1);
        state.set(&stringPreferencesKey("y"), 2);
    });
    let second = authored(CURRENT, 2, 20, |state| {
        state.set(&stringPreferencesKey("x"), 3)
    });
    let a = Fixture::new();
    let b = Fixture::new();
    for (target, early, late) in [(&a, &first, &second), (&b, &second, &first)] {
        target.apply(early);
        target.apply(late);
        target.apply(late);
        assert_eq!(target.value("x"), Some(json!(3)));
        assert_eq!(target.value("y"), Some(json!(2)));
    }
    assert_eq!(
        a.store().syncSnapshot().unwrap().data,
        b.store().syncSnapshot().unwrap().data
    );
}

fn correct(source: &Fixture, target: &Fixture) {
    target.store().prepareSync().unwrap();
    let local = target.store().syncSnapshot().unwrap();
    let remote = source.store().syncSnapshot().unwrap();
    let correction = Transaction::compare(&local, &remote).unwrap().unwrap();
    assert!(target
        .store()
        .repairSyncedSnapshot(&correction.local_token, &correction.source)
        .unwrap());
    assert!(
        Transaction::compare(&target.store().syncSnapshot().unwrap(), &remote)
            .unwrap()
            .is_none()
    );
}

#[test]
fn corrected_x_at_50_preserves_late_y_at_40_and_blocks_only_stale_x() {
    let source = Fixture::new();
    source.apply(&authored(CURRENT, 2, 50, |state| {
        state.set(&stringPreferencesKey("x"), "new")
    }));
    let target = Fixture::new();
    correct(&source, &target);
    let late = authored(CURRENT, 2, 40, |state| {
        state.set(&stringPreferencesKey("x"), "old");
        state.set(&stringPreferencesKey("y"), "independent");
    });
    apply(target.reopen(), &late);
    let state = current_store(target.reopen(), CURRENT).data().unwrap();
    assert_eq!(state.get(&stringPreferencesKey("x")), Some(&json!("new")));
    assert_eq!(
        state.get(&stringPreferencesKey("y")),
        Some(&json!("independent"))
    );
    apply(source.host.clone(), &late);
    assert_eq!(
        current_store(target.reopen(), CURRENT)
            .syncSnapshot()
            .unwrap()
            .data,
        source.store().syncSnapshot().unwrap().data
    );
}

#[test]
fn correction_preserves_tombstones_while_unrelated_older_writes_apply() {
    let source = Fixture::new();
    let create = authored(CURRENT, 2, 10, |state| {
        state.set(&stringPreferencesKey("x"), "deleted")
    });
    source.apply(&create);
    let delete_source = Fixture::new();
    let writer = delete_source.store();
    writer
        .edit(|state| state.set(&stringPreferencesKey("x"), "deleted"))
        .unwrap();
    let prior = delete_source
        .journal()
        .preferenceHistory()
        .unwrap()
        .into_iter()
        .map(|op| op.opId)
        .collect::<std::collections::BTreeSet<_>>();
    writer
        .edit(|state| state.remove(&stringPreferencesKey("x")))
        .unwrap();
    let deletion = delete_source
        .journal()
        .preferenceHistory()
        .unwrap()
        .into_iter()
        .filter(|op| !prior.contains(&op.opId))
        .map(|mut op| {
            op.createdAt = 50;
            op.payload["logicalTime"] = json!(50);
            op
        })
        .collect::<Vec<_>>();
    assert!(!deletion.is_empty());
    source.apply(&deletion);
    assert_eq!(source.value("x"), None);
    let target = Fixture::new();
    correct(&source, &target);
    let late = authored(CURRENT, 2, 40, |state| {
        state.set(&stringPreferencesKey("x"), "stale resurrection");
        state.set(&stringPreferencesKey("y"), true);
    });
    apply(target.reopen(), &late);
    let state = current_store(target.reopen(), CURRENT).data().unwrap();
    assert_eq!(state.get(&stringPreferencesKey("x")), None);
    assert_eq!(state.get(&stringPreferencesKey("y")), Some(&json!(true)));
}

#[test]
fn combined_layout_and_format_conversion_keeps_original_identity_and_business_priority() {
    let target = Fixture::new();
    target.apply(&authored(CURRENT, 2, 30, |state| {
        state.set(&stringPreferencesKey("displayName"), "丙")
    }));
    let layout = PreferenceLayout::new(target.host.clone(), LAYOUT)
        .withSchema(1, |_, migration| {
            migration.moveFile(OLD, &current_store(migration.storage.clone(), CURRENT))
        });
    layout.prepare().unwrap();
    let delayed = authored(OLD, 1, 20, |state| {
        state.set(&stringPreferencesKey("name"), "乙")
    });
    target.apply(&delayed);
    assert_eq!(target.value("displayName"), Some(json!("丙")));
    let later = authored(OLD, 1, 40, |state| {
        state.set(&stringPreferencesKey("name"), "丁")
    });
    target.apply(&later);
    assert_eq!(target.value("displayName"), Some(json!("丁")));
    assert_eq!(target.value("name"), None);
    let archive = target.journal().preferenceHistory().unwrap();
    for original in delayed.iter().chain(&later) {
        assert!(archive.contains(original));
    }
    assert!(!target.host.exists(OLD).unwrap());
}

#[test]
fn one_owner_converter_handles_encoded_record_split_for_state_and_transactions() {
    let payload = json!({"theme":"dark", "binding":"role-id"}).to_string();
    let incoming = authored(CURRENT, 1, 10, |state| {
        state.set(&stringPreferencesKey("encodedSettings"), payload.clone())
    });
    let from_history = Fixture::new();
    from_history.apply(&incoming);
    let from_state = Fixture::new();
    let old = PreferencesDataStore::newWithStorage(from_state.host.clone(), CURRENT)
        .withSchema(1, |_, _| Ok(()));
    old.edit(|state| state.set(&stringPreferencesKey("encodedSettings"), payload))
        .unwrap();
    let before = from_state.journal().preferenceHistory().unwrap();
    let state = from_state.store().data().unwrap();
    assert_eq!(from_state.journal().preferenceHistory().unwrap(), before);
    for key in ["theme", "binding", "encodedSettings"] {
        assert_eq!(
            from_history.value(key),
            state.get(&stringPreferencesKey(key)).cloned()
        );
    }
    assert_eq!(from_history.value("theme"), Some(json!("dark")));
    assert_eq!(from_history.value("binding"), Some(json!("role-id")));
    assert_eq!(from_history.value("encodedSettings"), None);
    assert_eq!(
        from_history.journal().preferenceHistory().unwrap(),
        incoming
    );
}

#[test]
fn migrating_a_mixed_set_and_increment_retains_each_edit_semantics() {
    let target = Fixture::new();
    target.apply(&authored(CURRENT, 2, 10, |state| {
        state.set(&stringPreferencesKey("quota"), 100)
    }));
    let mixed = authored(CURRENT, 1, 20, |state| {
        state
            .incrementInteger(&stringPreferencesKey("counter"), 2)
            .unwrap();
        state.set(&stringPreferencesKey("quota"), 10);
    });
    target.apply(&mixed);
    target.apply(&mixed);
    assert_eq!(target.value("quota"), Some(json!(10)));
    assert_eq!(target.value("counter"), Some(json!(2)));
}

/// A correction retains additive effects, so an older assignment cannot erase a later increment.
#[test]
fn correction_and_replay_agree_when_an_assignment_precedes_a_covered_increment() {
    let source = Fixture::new();
    source.apply(&authored(CURRENT, 2, 10, |state| {
        state.set(&stringPreferencesKey("counter"), 10)
    }));
    source.apply(&authored(CURRENT, 2, 50, |state| {
        state
            .incrementInteger(&stringPreferencesKey("counter"), 3)
            .unwrap();
    }));
    let target = Fixture::new();
    correct(&source, &target);
    let late = authored(CURRENT, 2, 40, |state| {
        state.set(&stringPreferencesKey("counter"), 20)
    });
    source.apply(&late);
    apply(target.reopen(), &late);
    assert_eq!(source.value("counter"), Some(json!(23)));
    let restarted = current_store(target.reopen(), CURRENT);
    assert_eq!(
        restarted
            .data()
            .unwrap()
            .get(&stringPreferencesKey("counter")),
        Some(&json!(23))
    );
    assert_eq!(
        restarted.syncSnapshot().unwrap().data,
        source.store().syncSnapshot().unwrap().data
    );
}

#[test]
fn production_registry_verifies_replacement_and_rejects_a_stale_capture() {
    let source = Fixture::new();
    source.apply(&authored(CURRENT, 2, 50, |state| {
        state.set(&stringPreferencesKey("x"), "authority")
    }));
    let target = Fixture::new();
    target.apply(&authored(CURRENT, 2, 10, |state| {
        state.set(&stringPreferencesKey("x"), "old")
    }));
    let schema = schema();
    let mut registry =
        PreferenceMerge::registry(target.host.clone(), &schema, &[], |_, _| true).unwrap();
    let authoritative = source.store().syncSnapshot().unwrap();
    let capture = registry.snapshot(&authoritative.data.owner).unwrap();
    registry.owners.get_mut(CURRENT).unwrap().repair = Some(Box::new(|_, _, _| Ok(true)));
    assert!(registry
        .repair(
            &capture.data.owner,
            &capture.token().unwrap(),
            &authoritative
        )
        .unwrap_err()
        .contains("verification failed"));
    target
        .store()
        .edit(|state| state.set(&stringPreferencesKey("x"), "new local write"))
        .unwrap();
    assert!(!registry
        .repair(
            &capture.data.owner,
            &capture.token().unwrap(),
            &authoritative
        )
        .unwrap());
    assert_eq!(target.value("x"), Some(json!("new local write")));
}

#[test]
fn migrated_checkpoint_retains_priority_before_correction_and_delayed_history() {
    let source = Fixture::new();
    let originals = authored(CURRENT, 1, 50, |p| {
        p.set(&stringPreferencesKey("name"), "new")
    });
    let old_schema = PreferenceMergeSchema {
        stores: vec![PreferenceMergeStore::values(
            CURRENT,
            false,
            |host, path| {
                PreferencesDataStore::newWithStorage(host, path).withSchema(1, |_, _| Ok(()))
            },
        )],
    };
    PreferenceMerge::applyOperations(source.host.clone(), &old_schema, &[], &originals, |_, _| {
        true
    })
    .unwrap();
    // Force the old data to be a baseline: the next migration cannot repair its
    // scope versions by replaying this original again.
    let old = PreferencesDataStore::newWithStorage(source.host.clone(), CURRENT)
        .withSchema(1, |_, _| Ok(()));
    let capture = old.syncSnapshot().unwrap();
    assert!(old
        .repairSyncedSnapshot(&capture.token().unwrap(), &capture)
        .unwrap());
    let upgraded = source.store().syncSnapshot().unwrap();
    let scope = serde_json::to_string(&("displayName", Vec::<Transaction::Path>::new())).unwrap();
    assert_eq!(
        upgraded.data.versions.get(&scope).map(|r| r.createdAt),
        Some(50),
        "state migration must carry the scope revision to its destination"
    );
    let target = Fixture::new();
    correct(&source, &target);
    let delayed = authored(CURRENT, 1, 40, |p| {
        p.set(&stringPreferencesKey("name"), "stale")
    });
    for f in [&source, &target] {
        apply(f.reopen(), &delayed);
        assert_eq!(f.value("displayName"), Some(json!("new")));
    }
    assert_eq!(
        source.store().syncSnapshot().unwrap().data,
        target.store().syncSnapshot().unwrap().data
    );
}

#[test]
fn migrated_collection_checkpoint_blocks_absent_old_ids_after_correction() {
    let source = Fixture::new();
    let original = authored(CURRENT, 1, 50, |p| {
        p.set(
            &stringPreferencesKey("cards"),
            json!([{"id":"a","count":20}]),
        )
    });
    let old_schema = PreferenceMergeSchema {
        stores: vec![PreferenceMergeStore::values(
            CURRENT,
            false,
            |host, path| {
                PreferencesDataStore::newWithStorage(host, path).withSchema(1, |_, _| Ok(()))
            },
        )],
    };
    PreferenceMerge::applyOperations(source.host.clone(), &old_schema, &[], &original, |_, _| {
        true
    })
    .unwrap();
    let old = PreferencesDataStore::newWithStorage(source.host.clone(), CURRENT)
        .withSchema(1, |_, _| Ok(()));
    let snapshot = old.syncSnapshot().unwrap();
    assert!(old
        .repairSyncedSnapshot(&snapshot.token().unwrap(), &snapshot)
        .unwrap());
    let target = Fixture::new();
    correct(&source, &target);
    let delayed = authored(CURRENT, 1, 40, |p| {
        p.set(
            &stringPreferencesKey("cards"),
            json!([{"id":"a","count":0},{"id":"unseen","count":1}]),
        )
    });
    for f in [&source, &target] {
        apply(f.reopen(), &delayed);
        assert_eq!(f.value("card_a"), Some(json!({"id":"a","count":20})));
        assert_eq!(f.value("card_unseen"), None);
        assert_eq!(f.value("cards"), None);
    }
    assert_eq!(
        source.store().syncSnapshot().unwrap().data,
        target.store().syncSnapshot().unwrap().data
    );
}

#[test]
fn layout_and_format_move_preserve_the_complete_checkpoint_without_reimporting_it() {
    let fixture = Fixture::new();
    let originals = authored(OLD, 1, 50, |p| {
        p.set(&stringPreferencesKey("name"), "new");
        p.set(&stringPreferencesKey("counter"), 10);
    });
    let old_schema = PreferenceMergeSchema {
        stores: vec![PreferenceMergeStore::values(OLD, false, |host, path| {
            PreferencesDataStore::newWithStorage(host, path).withSchema(1, |_, _| Ok(()))
        })],
    };
    PreferenceMerge::applyOperations(
        fixture.host.clone(),
        &old_schema,
        &[],
        &originals,
        |_, _| true,
    )
    .unwrap();
    let old = PreferencesDataStore::newWithStorage(fixture.host.clone(), OLD)
        .withSchema(1, |_, _| Ok(()));
    old.try_edit_result(|p| p.incrementInteger(&stringPreferencesKey("counter"), 3))
        .unwrap();
    let snapshot = old.syncSnapshot().unwrap();
    let history = fixture.journal().preferenceHistory().unwrap();
    PreferenceLayout::new(fixture.host.clone(), LAYOUT)
        .withSchema(1, |_, layout| {
            layout.moveFile(OLD, &current_store(layout.storage.clone(), CURRENT))
        })
        .prepare()
        .unwrap();
    let moved = fixture.store().syncSnapshot().unwrap();
    assert_eq!(moved.data.revision, snapshot.data.revision);
    assert_eq!(moved.covered, snapshot.covered);
    assert_eq!(moved.data.additions, snapshot.data.additions);
    assert_eq!(moved.data.values["counter"], 13);
    assert_eq!(moved.data.values["displayName"], "new");
    assert_eq!(
        fixture.journal().preferenceHistory().unwrap(),
        history,
        "layout cannot allocate a new business import"
    );
    fixture.apply(&history);
    assert_eq!(
        fixture.value("counter"),
        Some(json!(13)),
        "covered increments must not be replayed twice"
    );
}

fn invalid_migration_store(host: Arc<dyn RuntimeStorageHost>, path: &str) -> PreferencesDataStore {
    PreferencesDataStore::newWithStorage(host, path).withMigration(2, |_, context| {
        if context.original.is_some() {
            context.after.versions.clear();
        }
        Ok(())
    })
}

#[test]
fn invalid_migration_keeps_whole_original_pending_and_other_owners_continue() {
    let target = Fixture::new();
    target.apply(&authored(CURRENT, 2, 10, |state| {
        state.set(&stringPreferencesKey("x"), "committed")
    }));
    let before = target.store().syncSnapshot().unwrap();
    let bytes = target.host.readBytes(CURRENT).unwrap();
    let invalid = authored(CURRENT, 1, 20, |state| {
        state.set(&stringPreferencesKey("x"), "rejected");
        state.set(&stringPreferencesKey("y"), "atomic sibling");
    });
    let independent = "runtime/config/preferences/independent.preferences.json";
    let ready = authored(independent, 2, 30, |state| {
        state.set(&stringPreferencesKey("ready"), true)
    });
    let mut declarations = schema();
    declarations.stores[0].open = invalid_migration_store;
    let mut other = declarations.stores[0].clone();
    other.path = independent.into();
    other.open = current_store;
    declarations.stores.push(other);
    let originals = invalid.iter().chain(&ready).cloned().collect::<Vec<_>>();
    let outcomes = PreferenceMerge::applyIndependentOperations(
        target.host.clone(),
        &declarations,
        &[],
        &originals,
        |_, _| true,
    );
    assert!(outcomes.iter().any(|outcome| outcome
        .result
        .as_ref()
        .is_err_and(|error| error.contains("violates"))));
    assert_eq!(target.host.readBytes(CURRENT).unwrap(), bytes);
    assert_eq!(target.store().syncSnapshot().unwrap(), before);
    assert_eq!(target.journal().pendingOperations().unwrap(), invalid);
    assert_eq!(
        current_store(target.host.clone(), independent)
            .data()
            .unwrap()
            .get(&stringPreferencesKey("ready")),
        Some(&json!(true))
    );
    declarations.stores[0].open = current_store;
    PreferenceMerge::applyOperations(target.host.clone(), &declarations, &[], &[], |_, _| true)
        .unwrap();
    assert!(target.journal().pendingOperations().unwrap().is_empty());
    assert_eq!(target.value("x"), Some(json!("rejected")));
    assert_eq!(target.value("y"), Some(json!("atomic sibling")));
}

fn old_schema() -> PreferenceMergeSchema {
    let mut declarations = schema();
    declarations.stores[0].path = OLD.into();
    declarations.stores[0].open =
        |host, path| PreferencesDataStore::newWithStorage(host, path).withSchema(1, |_, _| Ok(()));
    declarations
}

fn apply_old(target: &Fixture, operations: &[SyncOperation]) {
    PreferenceMerge::applyOperations(
        target.host.clone(),
        &old_schema(),
        &[],
        operations,
        |_, _| true,
    )
    .unwrap();
}

fn move_old(target: &Fixture) {
    PreferenceLayout::new(target.host.clone(), LAYOUT)
        .withSchema(1, |_, migration| {
            migration.moveFile(OLD, &current_store(migration.storage.clone(), CURRENT))
        })
        .prepare()
        .unwrap();
}

#[test]
fn layout_with_existing_destination_commutes_with_old_path_transactions() {
    let old = authored(OLD, 1, 10, |state| {
        state.set(&stringPreferencesKey("name"), "old");
        state.set(&stringPreferencesKey("sourceOnly"), true);
    });
    let destination = authored(CURRENT, 2, 20, |state| {
        state.set(&stringPreferencesKey("displayName"), "destination");
        state.set(&stringPreferencesKey("destinationOnly"), true);
    });
    let transaction = authored(OLD, 1, 30, |state| {
        state.set(&stringPreferencesKey("name"), "later");
        state.set(&stringPreferencesKey("atomicSibling"), true);
    });
    let left = Fixture::new();
    let right = Fixture::new();
    for target in [&left, &right] {
        apply_old(target, &old);
        target.apply(&destination);
        let old = PreferencesDataStore::newWithStorage(target.host.clone(), OLD)
            .withSchema(1, |_, _| Ok(()));
        let checkpoint = old.syncSnapshot().unwrap();
        old.repairSyncedSnapshot(&checkpoint.token().unwrap(), &checkpoint)
            .unwrap();
        let checkpoint = target.store().syncSnapshot().unwrap();
        target
            .store()
            .repairSyncedSnapshot(&checkpoint.token().unwrap(), &checkpoint)
            .unwrap();
    }
    // m_L(Apply(A,T)) = Apply(m_L(A),m_L(T)), with both physical owners in A.
    apply_old(&left, &transaction);
    move_old(&left);
    move_old(&right);
    right.apply(&transaction);
    let a = left.store().syncSnapshot().unwrap();
    let b = right.store().syncSnapshot().unwrap();
    assert_eq!(
        a.data, b.data,
        "layout migration must commute when its destination already exists"
    );
    assert_eq!(a.covered, b.covered);
    assert_eq!(a.data.values["displayName"], "later");
    assert_eq!(a.data.values["sourceOnly"], true);
    assert_eq!(a.data.values["destinationOnly"], true);
    assert_eq!(a.data.values["atomicSibling"], true);
    let before = a.clone();
    // Retrying an interrupted move must retain edits made after its first commit.
    left.store()
        .edit(|state| state.set(&stringPreferencesKey("afterMove"), true))
        .unwrap();
    PreferenceLayout::new(
        left.host.clone(),
        "runtime/sync/retry-layout.preferences.json",
    )
    .withSchema(1, |_, migration| {
        migration.moveFile(OLD, &current_store(migration.storage.clone(), CURRENT))
    })
    .prepare()
    .unwrap();
    assert_eq!(left.value("afterMove"), Some(json!(true)));
    assert!(left
        .store()
        .syncSnapshot()
        .unwrap()
        .covered
        .is_superset(&before.covered));
}

#[test]
fn interrupted_occupied_layout_deduplicates_shared_increments_and_retains_newer_edits() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let target = Fixture::new();
    apply_old(
        &target,
        &authored(OLD, 1, 10, |state| {
            state.set(&stringPreferencesKey("counter"), 10)
        }),
    );
    apply_old(
        &target,
        &authored(OLD, 1, 50, |state| {
            state
                .incrementInteger(&stringPreferencesKey("counter"), 3)
                .unwrap();
        }),
    );
    let source =
        PreferencesDataStore::newWithStorage(target.host.clone(), OLD).withSchema(1, |_, _| Ok(()));
    let mut shared = source.syncSnapshot().unwrap();
    Transaction::Migrations {
        owners: std::collections::BTreeMap::from([(
            CURRENT.into(),
            Transaction::Owner {
                backend: Transaction::Backend::DataStore,
                path: CURRENT.into(),
                format: 2,
                layout: 1,
            },
        )]),
        layouts: std::collections::BTreeMap::from([(OLD.into(), CURRENT.into())]),
        formats: std::collections::BTreeMap::from([((CURRENT.into(), 1), vec![])]),
    }
    .migrate_data(&mut shared.data)
    .unwrap();
    target.store().prepareSync().unwrap();
    let capture = target.store().syncSnapshot().unwrap();
    target
        .store()
        .repairSyncedSnapshot(&capture.token().unwrap(), &shared)
        .unwrap();
    target.apply(&authored(CURRENT, 2, 70, |state| {
        state
            .incrementInteger(&stringPreferencesKey("counter"), 2)
            .unwrap();
    }));
    assert_eq!(target.value("counter"), Some(json!(15)));
    let fail = Arc::new(AtomicBool::new(true));
    let interrupted = fail.clone();
    let layout =
        PreferenceLayout::new(target.host.clone(), LAYOUT).withSchema(1, move |_, migration| {
            migration.moveFile(OLD, &current_store(migration.storage.clone(), CURRENT))?;
            if interrupted.load(Ordering::SeqCst) {
                return Err(PreferencesDataStoreError::Message(
                    "interrupted after destination commit".into(),
                ));
            }
            Ok(())
        });
    let history = target.journal().preferenceHistory().unwrap();
    assert!(layout.prepare().is_err());
    assert_eq!(target.value("counter"), Some(json!(15)));
    assert!(!target.host.exists(LAYOUT).unwrap());
    assert!(!operit_store::PreferenceLayout::routes(target.host.clone())
        .unwrap()
        .contains_key(OLD));
    target
        .store()
        .edit(|state| state.set(&stringPreferencesKey("afterFailure"), true))
        .unwrap();
    let checkpoint = target.store().syncSnapshot().unwrap();
    let retry_history = target.journal().preferenceHistory().unwrap();
    fail.store(false, Ordering::SeqCst);
    layout.prepare().unwrap();
    layout.prepare().unwrap();
    assert_eq!(target.value("counter"), Some(json!(15)));
    assert_eq!(target.value("afterFailure"), Some(json!(true)));
    assert_eq!(target.store().syncSnapshot().unwrap().data, checkpoint.data);
    assert_eq!(
        target.store().syncSnapshot().unwrap().covered,
        checkpoint.covered
    );
    assert_eq!(target.journal().preferenceHistory().unwrap(), retry_history);
    assert_eq!(retry_history.len(), history.len() + 1);
}

#[test]
fn occupied_layout_and_routed_object_fields_agree_after_destination_deletion() {
    let source = authored(OLD, 1, 10, |state| {
        state.set(
            &stringPreferencesKey("profile"),
            json!({"name":"old","obsolete":true}),
        );
    });
    let deleting = Fixture::new();
    deleting
        .store()
        .edit(|state| {
            state.set(&stringPreferencesKey("profile"), json!({}));
        })
        .unwrap();
    deleting
        .store()
        .edit(|state| state.remove(&stringPreferencesKey("profile")))
        .unwrap();
    let mut deletion = deleting
        .journal()
        .preferenceHistory()
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(deletion.operation, "delete");
    deletion.createdAt = 20;
    deletion.payload["logicalTime"] = json!(20);
    let deletion = vec![deletion];
    for revision in [15, 30] {
        let field = authored(OLD, 1, revision, |state| {
            state
                .setObjectField(&stringPreferencesKey("profile"), "name", json!("new"))
                .unwrap();
        });
        let left = Fixture::new();
        let right = Fixture::new();
        for target in [&left, &right] {
            apply_old(target, &source);
            target.apply(&deletion);
        }
        apply_old(&left, &field);
        move_old(&left);
        move_old(&right);
        right.apply(&field);
        let a = left.store().syncSnapshot().unwrap();
        let b = right.store().syncSnapshot().unwrap();
        assert_eq!(a.data, b.data);
        assert_eq!(a.covered, b.covered);
        assert_eq!(
            left.value("profile"),
            if revision > 20 {
                Some(json!({"name":"new"}))
            } else {
                None
            }
        );
    }
}

#[test]
fn dynamic_space_owners_recover_pending_history_without_schema_registration() {
    use operit_store::SyncOperationStore::SyncOperationSemantics;
    use operit_util::RuntimeStorageLayout::*;
    let fixture = Fixture::new();
    let originals = [
        RUNTIME_SPACE_MEMBERS_DIR_PATH,
        RUNTIME_SPACE_DEVICE_PROFILES_DIR_PATH,
        RUNTIME_SPACE_DEVICE_PRESENCE_DIR_PATH,
        RUNTIME_SPACE_TOPOLOGY_DIR_PATH,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, directory)| {
        let path = format!("{directory}/old-peer.preferences.json");
        let record = json!({"nodeId":"old-peer","updatedAt":index});
        SyncOperation {
            opId: format!("old-peer:{}", index + 1),
            originDeviceId: "old-peer".into(),
            sequence: index as i64 + 1,
            domain: "preferences".into(),
            entityType: "entry".into(),
            entityId: serde_json::to_string(&(&path, "record")).unwrap(),
            operation: "set".into(),
            semantics: SyncOperationSemantics::EntityState,
            payload: json!({"storagePath":path,"key":"record","value":record.to_string(),
                "formatVersion":1,"encrypted":false}),
            createdAt: index as i64 + 1,
            schemaVersion: 1,
        }
    })
    .collect::<Vec<_>>();
    fixture.journal().receiveOperations(&originals).unwrap();
    for _ in 0..2 {
        let schema = PreferenceMergeSchema::default();
        let registry =
            PreferenceMerge::registry(fixture.reopen(), &schema, [], |_, _| true).unwrap();
        // A retry with no incoming batch must discover owners from the durable journal.
        let report = registry.receive(&[]).unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(report.pending.is_empty());
        let (snapshots, pending, errors) = registry.snapshots().unwrap();
        assert!(pending.is_empty());
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(snapshots.len(), originals.len());
        for (index, operation) in originals.iter().enumerate() {
            let path = operation.payload["storagePath"].as_str().unwrap();
            let snapshot = snapshots
                .iter()
                .find(|snapshot| snapshot.data.owner.path == path)
                .unwrap();
            assert_eq!(
                snapshot.data.values["record"],
                json!({"nodeId":"old-peer","updatedAt":index})
            );
            assert!(snapshot.covered.contains(&operation.opId));
        }
    }
    assert_eq!(fixture.journal().preferenceHistory().unwrap(), originals);
}

#[test]
fn dynamic_space_owners_only_accept_declared_record_paths() {
    use operit_util::RuntimeStorageLayout::*;
    let schema = PreferenceMergeSchema::default();
    for path in [
        format!("{RUNTIME_SPACE_MEMBERS_DIR_PATH}/.hidden.preferences.json"),
        format!("{RUNTIME_SPACE_MEMBERS_DIR_PATH}/nested/peer.preferences.json"),
        format!("{RUNTIME_SPACE_MEMBERS_DIR_PATH}/peer.json"),
        CHARACTER_CARDS_PREFERENCES_PATH.into(),
        format!("{DATA_MEMORY_CHARACTERS_DIR_PATH}/default_character/settings/memory_search_settings.preferences.json"),
    ] {
        assert!(schema.owner(&path).unwrap().is_none(), "{path}");
    }
}

#[test]
fn file_correction_seals_both_histories_and_future_edits_start_from_the_copied_file() {
    let source = Fixture::new();
    let target = Fixture::new();
    let old = authored(CURRENT, 2, 40, |values| values.set(&stringPreferencesKey("discarded"), "old history"));
    let new = authored(CURRENT, 2, 50, |values| values.set(&stringPreferencesKey("x"), "original"));
    target.apply(&old);
    source.apply(&new);
    // The materialized file is authoritative even when it cannot be reconstructed from history.
    let bytes = b"{\n  \"x\": \"repaired file\",\n  \"__operit_preferences_schema_version\": 2\n}\n";
    source.host.writeBytesAtomically(CURRENT, bytes).unwrap();
    let a = source.store().syncSnapshot().unwrap();
    let b = target.store().syncSnapshot().unwrap();
    let file = Transaction::preference::file_snapshot(&source.store(), &a.token().unwrap()).unwrap();
    let baseline = Transaction::compare(&a, &b).unwrap().unwrap().source;
    for (fixture, expected) in [(&target, b.token().unwrap()), (&source, a.token().unwrap())] {
        assert!(Transaction::preference::repair_file(&fixture.store(), &expected, &baseline, Some(&file)).unwrap());
        assert_eq!(fixture.host.readBytes(CURRENT).unwrap(), bytes);
        let reopened = current_store(fixture.reopen(), CURRENT);
        assert_eq!(reopened.syncSnapshot().unwrap().covered, baseline.covered);
        apply(fixture.reopen(), &old);
        apply(fixture.reopen(), &new);
        assert_eq!(reopened.data().unwrap().get(&stringPreferencesKey("x")), Some(&json!("repaired file")));
        assert!(reopened.data().unwrap().get(&stringPreferencesKey("discarded")).is_none());
    }
    let next = authored(CURRENT, 2, 60, |values| values.set(&stringPreferencesKey("y"), "future edit"));
    for fixture in [&source, &target] { apply(fixture.reopen(), &next); }
    let a = current_store(source.reopen(), CURRENT).syncSnapshot().unwrap();
    let b = current_store(target.reopen(), CURRENT).syncSnapshot().unwrap();
    assert_eq!(a.data, b.data);
    assert_eq!(a.data.values["x"], json!("repaired file"));
    assert_eq!(a.data.values["y"], json!("future edit"));
    assert!(!a.data.values.contains_key("discarded"));
}


#[test]
fn equal_files_with_a_half_committed_baseline_require_correction_after_restart() {
    let source = Fixture::new();
    let target = Fixture::new();
    let source_history = authored(CURRENT,2,50,|values|values.set(&stringPreferencesKey("x"),"source"));
    let target_history = authored(CURRENT,2,40,|values|values.set(&stringPreferencesKey("old"),"superseded"));
    source.apply(&source_history);
    target.apply(&target_history);
    let a = source.store().syncSnapshot().unwrap();
    let b = target.store().syncSnapshot().unwrap();
    let baseline = Transaction::compare(&a,&b).unwrap().unwrap().source;
    assert!(target.store().repairSyncedSnapshot(&b.token().unwrap(),&baseline).unwrap());
    // Simulate disconnection before the source installs its new baseline.
    let manifests = |fixture: &Fixture| {
        let schema = schema();
        let registry = PreferenceMerge::registry(fixture.reopen(),&schema,[],|_,_|true).unwrap();
        let manifests = registry.manifests().unwrap().0;
        manifests.into_iter().find(|m|m.owner.path == CURRENT).unwrap()
    };
    let local = manifests(&source);
    let remote = manifests(&target);
    assert_eq!(local.digest,remote.digest);
    assert_ne!(local.baseline,remote.baseline);
    assert_ne!(Transaction::compare_manifests(&local,&remote).unwrap(),Transaction::Source::Equal);
    let reopened = current_store(source.reopen(),CURRENT);
    assert!(reopened.repairSyncedSnapshot(&a.token().unwrap(),&baseline).unwrap());
    assert_eq!(Transaction::compare_manifests(&manifests(&source),&manifests(&target)).unwrap(),Transaction::Source::Equal);
    source.apply(&target_history);
    target.apply(&source_history);
    assert_eq!(source.store().syncSnapshot().unwrap().data,target.store().syncSnapshot().unwrap().data);
}
