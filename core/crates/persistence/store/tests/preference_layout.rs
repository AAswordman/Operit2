//! Layout routes active originals and confirms explicit retirements; unregistered files are ignored.
use std::{path::PathBuf, sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}}};
use operit_host_api::{RuntimeStorageHost, PluginStorage::StoragePreferencesMergeDescriptor};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::{
    PreferenceLayout::PreferenceLayout,
    PreferenceMerge::{applyOperations, PreferenceMergeSchema, PreferenceMergeStore},
    PreferencesDataStore::{stringPreferencesKey, CoreNodeStateStore, PreferencesDataStore, PreferencesDataStoreError},
    SyncOperationStore::{SyncOperation, SyncOperationStore},
};
use serde_json::json;

const STATE: &str = "runtime/sync/test_preference_layout.preferences.json";
const OLD: &str = "runtime/config/preferences/retired.preferences.json";
const CURRENT: &str = "runtime/config/preferences/current.preferences.json";
const DIRECTORY: &str = "runtime/extensions/space/settings";

struct Fixture { root: PathBuf, host: Arc<NativeRuntimeStorageHost> }
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("operit-preference-layout-{}", uuid::Uuid::new_v4()));
        let host = Arc::new(NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces")));
        Self { root, host }
    }
    fn store(&self, path: &str) -> PreferencesDataStore { open(self.host.clone(), path) }
    fn journal(&self) -> SyncOperationStore { SyncOperationStore::new(self.host.clone(), "runtime/sync") }
    fn seed(&self) {
        CoreNodeStateStore::newWithStorage(self.host.clone(), OLD).edit(|state| state.set(&stringPreferencesKey("settings"), json!({"enabled":true}))).unwrap();
    }
}
impl Drop for Fixture { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.root); } }
fn open(host: Arc<dyn RuntimeStorageHost>, path: &str) -> PreferencesDataStore {
    PreferencesDataStore::newWithStorage(host, path).withSchema(2, |_, _| Ok(()))
}
fn schema() -> PreferenceMergeSchema {
    PreferenceMergeSchema { stores: vec![PreferenceMergeStore {
        path: CURRENT.into(), encrypted: false, open,
        merge: StoragePreferencesMergeDescriptor { version: 1, catalog: None, references: vec![], projection: None },
    }] }
}
fn layout(host: Arc<dyn RuntimeStorageHost>, attempts: Arc<AtomicUsize>, fail: Arc<AtomicBool>) -> PreferenceLayout {
    PreferenceLayout::new(host, STATE).withSchema(2, move |from, migration| {
        match from {
            0 => {
                attempts.fetch_add(1, Ordering::SeqCst);
                migration.moveFile(OLD, &open(migration.storage.clone(), CURRENT))?;
                if fail.load(Ordering::SeqCst) { return Err(PreferencesDataStoreError::Message("Interrupted layout migration".into())); }
                migration.retireFile(OLD);
            }
            1 => migration.retireDirectory(DIRECTORY),
            from => return Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
        }
        Ok(())
    })
}
fn originals(source: &Fixture) -> Vec<SyncOperation> {
    for path in [OLD.to_owned(), format!("{DIRECTORY}/package-a.preferences.json"), format!("{DIRECTORY}/skill-b.preferences.json")] {
        for revision in 1..=5 {
            source.store(&path).edit(|state| state.set(&stringPreferencesKey("settings"), json!({"revision":revision}))).unwrap();
        }
    }
    let originals = source.journal().preferenceHistory().unwrap();
    assert_eq!(originals.len(), 15);
    originals
}

#[test]
fn moved_owner_applies_and_ten_retired_namespace_originals_are_confirmed() {
    let incoming = originals(&Fixture::new());
    let target = Fixture::new();
    target.seed();
    let attempts = Arc::new(AtomicUsize::new(0));
    let manager = layout(target.host.clone(), attempts.clone(), Arc::new(AtomicBool::new(false)));
    manager.prepare().unwrap();
    let old_bytes = target.host.readBytes(OLD).unwrap();
    let destination = target.store(CURRENT);
    assert_eq!(destination.data().unwrap().get(&stringPreferencesKey("settings")), Some(&json!({"enabled":true})));
    destination.edit(|state| state.set(&stringPreferencesKey("settings"), json!({"enabled":false}))).unwrap();
    let journal = target.journal();
    journal.receiveOperations(&incoming).unwrap();
    let mut pending = journal.pendingOperations().unwrap();
    let mut history = incoming.clone();
    assert_eq!(manager.prepareOperations(&mut history, &mut pending).unwrap(), 0);
    assert_eq!(original_map(&pending), original_map(&incoming));
    assert_eq!(history, incoming);
    assert_eq!(journal.pendingOperationCount().unwrap(), 15);
    let declarations = schema();
    let registry = operit_store::PreferenceMerge::registry(target.host.clone(), &declarations, incoming.iter(), |_, _| true).unwrap();
    let report = registry.receive(&incoming).unwrap();
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(report.skipped, 10);
    assert!(report.pending.is_empty());
    assert_eq!(registry.receive(&incoming).unwrap().skipped, 0);
    let (snapshots, pending, errors) = registry.snapshots().unwrap();
    assert!(pending.is_empty());
    assert!(errors.is_empty(), "{errors:?}");
    assert!(snapshots.iter().all(|snapshot| snapshot.data.owner.path == CURRENT));
    let history = original_map(&journal.preferenceHistory().unwrap());
    for operation in &incoming { assert_eq!(history.get(&operation.opId), Some(operation)); }
    assert_eq!(target.host.readBytes(OLD).unwrap(), old_bytes);
    assert!(!target.host.exists(DIRECTORY).unwrap());
    assert_eq!(destination.data().unwrap().get(&stringPreferencesKey("settings")), Some(&json!({"enabled":false})));
    manager.prepare().unwrap();
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

#[test]
fn interrupted_layout_keeps_all_originals_until_resume_and_owner_application() {
    let incoming = originals(&Fixture::new());
    let target = Fixture::new();
    target.seed();
    let attempts = Arc::new(AtomicUsize::new(0));
    let fail = Arc::new(AtomicBool::new(true));
    let manager = layout(target.host.clone(), attempts.clone(), fail.clone());
    target.journal().receiveOperations(&incoming).unwrap();
    let mut pending = target.journal().pendingOperations().unwrap();
    let mut history = incoming.clone();
    assert!(manager.prepareOperations(&mut history, &mut pending).is_err());
    assert_eq!(history, incoming);
    assert_eq!(original_map(&pending), original_map(&incoming));
    assert!(!target.host.exists(STATE).unwrap());
    assert!(operit_store::PreferenceLayout::retirements(target.host.clone()).unwrap().is_empty());
    fail.store(false, Ordering::SeqCst);
    assert_eq!(manager.prepareOperations(&mut history, &mut pending).unwrap(), 0);
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert_eq!(target.journal().pendingOperationCount().unwrap(), 15);
    assert_eq!(operit_store::PreferenceLayout::routes(target.host.clone()).unwrap().get(OLD), Some(&CURRENT.to_owned()));
}

#[test]
fn retirement_keeps_neighboring_paths_and_future_layout_waits() {
    let source = Fixture::new();
    let path = format!("{DIRECTORY}-current/skill.preferences.json");
    source.store(&path).edit(|state| state.set(&stringPreferencesKey("value"), true)).unwrap();
    let incoming = source.journal().preferenceHistory().unwrap();
    let target = Fixture::new();
    target.seed();
    let manager = layout(target.host.clone(), Arc::new(AtomicUsize::new(0)), Arc::new(AtomicBool::new(false)));
    let mut pending = incoming.clone();
    assert_eq!(manager.prepareOperations(&mut Vec::new(), &mut pending).unwrap(), 0);
    assert_eq!(original_map(&pending), original_map(&incoming));
    assert!(applyOperations(target.host.clone(), &schema(), &[], &incoming, |_, _| true).is_ok());
    assert!(target.journal().pendingOperations().unwrap().is_empty());
    assert!(target.journal().skippedOperations().unwrap().is_empty());
    let old_layout = PreferenceLayout::new(target.host.clone(), STATE).withSchema(1, |_, _| Ok(()));
    assert!(old_layout.prepareOperations(&mut Vec::new(), &mut pending).is_err());
    assert_eq!(original_map(&pending), original_map(&incoming));
}

#[test]
fn layout_progress_cannot_be_changed_by_a_remote_business_transaction() {
    let source = Fixture::new();
    source.store(STATE).edit(|state| state.set(&stringPreferencesKey("route_file:injected"), CURRENT)).unwrap();
    let incoming = source.journal().preferenceHistory().unwrap();
    let target = Fixture::new();
    target.seed();
    let manager = layout(target.host.clone(), Arc::new(AtomicUsize::new(0)), Arc::new(AtomicBool::new(false)));
    let mut pending = incoming.clone();
    assert!(manager.prepareOperations(&mut Vec::new(), &mut pending).is_err());
    assert_eq!(original_map(&pending), original_map(&incoming));
}

fn original_map(operations: &[SyncOperation]) -> std::collections::BTreeMap<String,SyncOperation> {
    operations.iter().cloned().map(|operation| (operation.opId.clone(),operation)).collect()
}

#[test]
fn retired_file_is_confirmed_while_an_unregistered_neighbor_is_ignored_after_reopen() {
    use operit_store::SyncOperationStore::SyncOperationKnowledge;
    let source = Fixture::new();
    let neighbor = format!("{OLD}.other.preferences.json");
    for path in [OLD, neighbor.as_str()] {
        source
            .store(path)
            .edit(|values| values.set(&stringPreferencesKey("value"), true))
            .unwrap();
    }
    let incoming = source.journal().preferenceHistory().unwrap();
    let target = Fixture::new();
    PreferenceLayout::new(target.host.clone(), STATE)
        .withSchema(1, |_, migration| {
            migration.retireFile(OLD);
            Ok(())
        })
        .prepare()
        .unwrap();
    for attempt in 0..2 {
        let host = Arc::new(NativeRuntimeStorageHost::new(
            target.root.join("runtime"),
            target.root.join("workspaces"),
        ));
        let schema = PreferenceMergeSchema::default();
        let registry =
            operit_store::PreferenceMerge::registry(host, &schema, [], |_, _| true).unwrap();
        let report = registry.receive(&incoming).unwrap();
        assert_eq!(report.skipped, 2 * usize::from(attempt == 0));
        assert!(report.applied.is_empty());
        assert!(report.errors.is_empty());
        assert!(target.journal().pendingOperations().unwrap().is_empty());
        assert!(target.journal().skippedOperations().unwrap().is_empty());
        let receipt = target.journal().receipt().unwrap();
        for op in &incoming {
            // Receipt acknowledges durable reception; pending separately tracks application.
            assert!(receipt.containsSequence(&op.originDeviceId, op.sequence));
        }
        assert!(!target.host.exists(OLD).unwrap());
        assert!(!target.host.exists(&neighbor).unwrap());
        let mut conflicting = incoming
            .iter()
            .find(|op| op.payload["storagePath"] == OLD)
            .unwrap()
            .clone();
        conflicting.payload["value"] = json!(false);
        assert!(registry.receive(&[conflicting]).is_err());
    }
    let history = target.journal().preferenceHistory().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(
        history[0],
        *incoming
            .iter()
            .find(|op| op.payload["storagePath"] == OLD)
            .unwrap()
    );
}
