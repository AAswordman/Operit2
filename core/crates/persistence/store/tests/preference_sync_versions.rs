//! Real preference commits use the shared transaction model through migration, replay and repair.
use std::{path::PathBuf, sync::Arc};
use operit_host_api::RuntimeStorageHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::{PreferenceMerge::{applyIndependentOperations, PreferenceMergeSchema, PreferenceMergeStore},
    PreferencesDataStore::{PreferencesDataStore, PreferencesSyncedEntry, stringPreferencesKey},
    SyncOperationStore::{SyncOperation, SyncOperationStore}, Transaction::{self, Rule}};
use serde_json::{json, Value};

const PATH: &str = "runtime/config/preferences/version-check.preferences.json";
const OTHER: &str = "runtime/config/preferences/healthy.preferences.json";
const VERSION: &str = "__operit_preferences_schema_version";
struct Fixture { root: PathBuf, host: Arc<NativeRuntimeStorageHost> }
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("operit-preference-contract-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let host = Arc::new(NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces")));
        Self { root, host }
    }
    fn store(&self, path: &str, version: u32) -> PreferencesDataStore { store(self.host.clone(), path, version) }
    fn journal(&self) -> SyncOperationStore { SyncOperationStore::new(self.host.clone(), "runtime/sync") }
    fn history(&self) -> Vec<SyncOperation> { self.journal().preferenceHistory().unwrap() }
    fn value(&self, key: &str) -> Option<Value> { self.store(PATH, 2).data().unwrap().get(&stringPreferencesKey(key)).cloned() }
}
impl Drop for Fixture { fn drop(&mut self) { std::fs::remove_dir_all(&self.root).unwrap(); } }

fn store(host: Arc<dyn RuntimeStorageHost>, path: &str, version: u32) -> PreferencesDataStore {
    PreferencesDataStore::newWithStorage(host, path).withMigration(version, |from, model| {
        if from == 1 { Rule::RenameKey { from: "name".into(), to: "displayName".into() }.apply(model).map_err(operit_store::PreferencesDataStore::PreferencesDataStoreError::Message)?; }
        Ok(())
    })
}
fn schema(version: u32) -> PreferenceMergeSchema {
    let open: fn(Arc<dyn RuntimeStorageHost>, &str) -> PreferencesDataStore = match version {
        2 => |host, path| store(host, path, 2),
        3 => |host, path| store(host, path, 3),
        _ => panic!("test owner version"),
    };
    PreferenceMergeSchema { stores: [PATH, OTHER].into_iter().map(|path| PreferenceMergeStore {
        path: path.into(), encrypted: false, open,
        merge: operit_host_api::PluginStorage::StoragePreferencesMergeDescriptor { version: 1, catalog: None, references: vec![], projection: None },
    }).collect() }
}
fn apply(target: &Fixture, incoming: &[SyncOperation], version: u32) -> Vec<operit_store::PreferenceMerge::PreferenceMergeOutcome> {
    applyIndependentOperations(target.host.clone(), &schema(version), &[], incoming, |_, _| true)
}
fn successful(target: &Fixture, incoming: &[SyncOperation]) {
    let outcomes = apply(target, incoming, 2);
    assert!(!outcomes.is_empty());
    for outcome in outcomes { assert!(!outcome.skipped && outcome.result.is_ok(), "{:?}", outcome.result); }
}
fn write(f: &Fixture, version: u32, key: &str, value: Value, time: i64) -> Vec<SyncOperation> {
    f.store(PATH, version).edit(|p| p.set(&stringPreferencesKey(key), value)).unwrap();
    f.history().into_iter().map(|mut op| { op.createdAt = time; op.payload["logicalTime"] = json!(time); op }).collect()
}

#[test]
fn outbound_transactions_and_imports_carry_versions_and_atomic_boundaries() {
    let f = Fixture::new();
    f.host.writeBytes(PATH, &serde_json::to_vec(&json!({VERSION:2,"existing":true})).unwrap()).unwrap();
    f.store(PATH, 2).edit(|p| { p.set(&stringPreferencesKey("x"), 1); p.set(&stringPreferencesKey("y"), 2); }).unwrap();
    let history = f.history();
    assert!(history.iter().any(|op| op.operation == "transaction" && op.payload["entries"].as_array().unwrap().len() == 2));
    assert!(history.iter().all(|op| op.payload["schemaVersion"] == 2));
}

#[test]
fn state_and_old_transactions_use_the_same_rule_without_new_business_writes() {
    let source = Fixture::new();
    let incoming = write(&source, 1, "name", json!("甲"), 10);
    let before = source.history();
    source.store(PATH, 2).data().unwrap();
    assert_eq!(source.history(), before);
    let target = Fixture::new();
    successful(&target, &incoming);
    assert_eq!(target.value("displayName"), Some(json!("甲")));
    assert_eq!(target.value("name"), None);
    let snapshot = target.store(PATH, 2).syncSnapshot().unwrap();
    assert_eq!(snapshot.data.revision.unwrap().createdAt, 10);
    assert!(incoming.iter().all(|op| snapshot.covered.contains(&op.opId)));
}

#[test]
fn delayed_old_formats_keep_their_original_business_priority() {
    for (time, expected) in [(20, "丙"), (40, "乙")] {
        let current = Fixture::new();
        let old = Fixture::new();
        let a = write(&current, 2, "displayName", json!("丙"), 30);
        let b = write(&old, 1, "name", json!("乙"), time);
        let target = Fixture::new();
        successful(&target, &a);
        successful(&target, &b);
        assert_eq!(target.value("displayName"), Some(json!(expected)));
        successful(&target, &b); // Duplicate delivery keeps the same result.
        assert_eq!(target.value("displayName"), Some(json!(expected)));
    }
}

#[test]
fn unversioned_supported_history_is_converted_and_archived_originals_remain_exact() {
    let source = Fixture::new();
    let mut incoming = write(&source, 1, "kept", json!(true), 10);
    for operation in &mut incoming { operation.payload.as_object_mut().unwrap().remove("schemaVersion"); }
    let target = Fixture::new();
    successful(&target, &incoming);
    assert_eq!(target.value("kept"), Some(json!(true)));
    assert!(incoming.iter().all(|op| target.history().contains(op)));
}

#[test]
fn future_data_waits_and_applies_after_owner_upgrade() {
    let source = Fixture::new();
    let incoming = write(&source, 3, "future", json!(true), 40);
    let target = Fixture::new();
    target.store(PATH, 2).data().unwrap();
    let before = target.host.readBytes(PATH).unwrap();
    target.journal().receiveOperations(&incoming).unwrap();
    let outcomes = apply(&target, &incoming, 2);
    assert!(outcomes.iter().all(|o| !o.skipped && o.result.is_err()));
    assert_eq!(target.host.readBytes(PATH).unwrap(), before);
    assert_eq!(target.journal().pendingOperations().unwrap(), incoming);
    assert!(apply(&target, &incoming, 3).iter().all(|o| o.result.is_ok()));
    assert_eq!(target.store(PATH, 3).data().unwrap().get(&stringPreferencesKey("future")), Some(&json!(true)));
}

#[test]
fn an_undeclared_converter_retains_old_transactions_for_recovery() {
    let source = Fixture::new();
    let incoming = write(&source, 1, "name", json!("甲"), 10);
    let target = Fixture::new();
    let mut schema = schema(2);
    schema.stores[0].open = |host, path| PreferencesDataStore::newWithStorage(host, path).withMigration(2, |from, _| Err(operit_store::PreferencesDataStore::PreferencesDataStoreError::MissingMigration { from, to: from+1 }));
    let outcomes = applyIndependentOperations(target.host.clone(), &schema, &[], &incoming, |_, _| true);
    assert!(outcomes.iter().any(|o| o.result.as_ref().is_err_and(|e| e.contains("missing preferences migration"))));
    assert_eq!(target.value("displayName"), None);
    successful(&target, &incoming);
    assert_eq!(target.value("displayName"), Some(json!("甲")));
}


#[test]
fn unknown_owners_stay_pending_while_an_independent_owner_commits() {
    let source = Fixture::new();
    source.store(PATH, 2).edit(|p| p.set(&stringPreferencesKey("healthy"), true)).unwrap();
    source.store("runtime/config/preferences/unknown.preferences.json", 2).edit(|p| p.set(&stringPreferencesKey("unknown"), true)).unwrap();
    let target = Fixture::new();
    let outcomes = apply(&target, &source.history(), 2);
    assert!(outcomes.iter().any(|o| o.paths.contains(PATH) && o.result.is_ok()));
    assert!(outcomes.iter().any(|o| !o.paths.contains(PATH) && !o.skipped && o.result.is_err()));
    assert_eq!(target.value("healthy"), Some(json!(true)));
    assert!(!target.host.exists("runtime/config/preferences/unknown.preferences.json").unwrap());
}

#[test]
fn previously_skipped_history_is_recovered_by_actual_business_replay() {
    let source = Fixture::new();
    let old = write(&source, 1, "name", json!("甲"), 10);
    let target = Fixture::new();
    target.store(PATH, 2).prepareSync().unwrap();
    target.journal().receiveOperations(&old).unwrap();
    target.journal().appendOperations(&old).unwrap();
    target.journal().completeOperations(&old).unwrap();
    let current = Fixture::new();
    successful(&target, &write(&current, 2, "healthy", json!(true), 20));
    assert_eq!(target.value("displayName"), Some(json!("甲")));
    assert_eq!(target.value("healthy"), Some(json!(true)));
}

#[test]
fn correction_is_durable_and_late_old_logs_cannot_undo_it() {
    let source = Fixture::new();
    successful(&source, &write(&Fixture::new(), 2, "displayName", json!("丙"), 30));
    let target = Fixture::new();
    successful(&target, &write(&Fixture::new(), 1, "name", json!("甲"), 10));
    let local = target.store(PATH, 2);
    let a = local.syncSnapshot().unwrap();
    let b = source.store(PATH, 2).syncSnapshot().unwrap();
    let correction = Transaction::compare(&a, &b).unwrap().unwrap();
    assert!(local.repairSyncedSnapshot(&correction.local_token, &correction.source).unwrap());
    assert!(Transaction::compare(&local.syncSnapshot().unwrap(), &b).unwrap().is_none());
    drop(local);
    let reopened = Arc::new(NativeRuntimeStorageHost::new(target.root.join("runtime"), target.root.join("workspaces")));
    let late = write(&Fixture::new(), 1, "name", json!("乙"), 20);
    let outcomes = applyIndependentOperations(reopened.clone(), &schema(2), &[], &late, |_, _| true);
    assert!(outcomes.iter().all(|o| o.result.is_ok()), "{:?}", outcomes.iter().map(|o| &o.result).collect::<Vec<_>>());
    assert_eq!(store(reopened, PATH, 2).data().unwrap().get(&stringPreferencesKey("displayName")), Some(&json!("丙")));
    let newer = write(&Fixture::new(), 2, "unrelated", json!(true), 40);
    successful(&target, &newer);
    assert_eq!(target.value("displayName"), Some(json!("丙")));
    assert_eq!(target.value("unrelated"), Some(json!(true)));
}

#[test]
fn correction_rejects_intervening_business_writes() {
    let target = Fixture::new();
    let local = target.store(PATH, 2);
    local.prepareSync().unwrap();
    let capture = local.syncSnapshot().unwrap();
    local.edit(|p| p.set(&stringPreferencesKey("racing"), true)).unwrap();
    assert!(!local.repairSyncedSnapshot(&capture.token().unwrap(), &capture).unwrap());
    assert_eq!(target.value("racing"), Some(json!(true)));
}
