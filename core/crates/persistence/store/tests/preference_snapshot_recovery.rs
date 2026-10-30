//! Final comparison and correction use the same durable preference snapshot.
use std::{
    path::PathBuf,
    sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Arc, Mutex},
};

use operit_host_api::{HostError, HostResult, RuntimeSqliteConnection, RuntimeStorageEntry, RuntimeStorageHost};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::{
    CoreSpaceStore::CoreSpaceStore,
    PreferenceMerge::{self, PreferenceMergeSchema},
    PreferencesDataStore::{stringPreferencesKey, CoreNodeStateStore, PreferencesDataStore},
    SyncOperationStore::SyncOperationStore,
};
use serde_json::{json, Value};

struct ObservedHost {
    native: NativeRuntimeStorageHost,
    watched: Mutex<String>,
    writes: AtomicUsize,
    empty_writes: AtomicUsize,
    fail_writes: AtomicBool,
    fail_on_write: AtomicUsize,
}
impl ObservedHost {
    fn before_write(&self, path: &str, bytes: &[u8]) -> HostResult<()> {
        if path == *self.watched.lock().unwrap() {
            if self.fail_writes.load(Ordering::SeqCst) {
                return Err(HostError::new("interrupted snapshot restoration"));
            }
            let write = self.writes.fetch_add(1, Ordering::SeqCst) + 1;
            if self.fail_on_write.load(Ordering::SeqCst) == write {
                return Err(HostError::new("interrupted final file publication"));
            }
            let values: Value = serde_json::from_slice(bytes).unwrap();
            if values.get("record").is_none() {
                self.empty_writes.fetch_add(1, Ordering::SeqCst);
            }
        }
        Ok(())
    }
}
impl RuntimeStorageHost for ObservedHost {
    fn openTransactionDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        self.native.openTransactionDatabase(path)
    }
    fn runtimeRootDir(&self) -> Option<PathBuf> { self.native.runtimeRootDir() }
    fn workspaceRootDir(&self) -> Option<PathBuf> { self.native.workspaceRootDir() }
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> { self.native.readBytes(path) }
    fn writeBytes(&self, path: &str, bytes: &[u8]) -> HostResult<()> {
        self.before_write(path, bytes)?;
        self.native.writeBytes(path, bytes)
    }
    fn writeBytesAtomically(&self, path: &str, bytes: &[u8]) -> HostResult<()> {
        self.before_write(path, bytes)?;
        self.native.writeBytesAtomically(path, bytes)
    }
    fn appendBytes(&self, path: &str, bytes: &[u8]) -> HostResult<()> { self.native.appendBytes(path, bytes) }
    fn flushFile(&self, path: &str) -> HostResult<()> { self.native.flushFile(path) }
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> { self.native.delete(path, recursive) }
    fn exists(&self, path: &str) -> HostResult<bool> { self.native.exists(path) }
    fn list(&self, path: &str) -> HostResult<Vec<RuntimeStorageEntry>> { self.native.list(path) }
}

struct Fixture {
    root: PathBuf,
    host: Arc<ObservedHost>,
    path: String,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("operit-snapshot-recovery-{}", uuid::Uuid::new_v4()));
        let host = Arc::new(ObservedHost {
            native: NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces")),
            watched: Mutex::new(String::new()), writes: AtomicUsize::new(0),
            empty_writes: AtomicUsize::new(0), fail_writes: AtomicBool::new(false), fail_on_write: AtomicUsize::new(0),
        });
        let space = CoreSpaceStore::new(host.clone()).initialize().unwrap();
        let path = format!("runtime/space/members/{}.preferences.json", space.members[0]);
        let result = Self { root, host, path };
        result.store().prepareSync().unwrap();
        *result.host.watched.lock().unwrap() = result.path.clone();
        result
    }
    fn store(&self) -> PreferencesDataStore {
        let owner = PreferenceMergeSchema::default().owner(&self.path).unwrap().unwrap();
        (owner.open)(self.host.clone(), &self.path)
    }
    fn journal(&self) -> SyncOperationStore { SyncOperationStore::new(self.host.clone(), "runtime/sync") }
    fn clear_projection(&self) {
        CoreNodeStateStore::newWithStorage(self.host.clone(), &self.path).delete().unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.root); }
}

#[test]
fn comparison_restores_a_missing_space_projection_without_publishing_an_empty_record() {
    let f = Fixture::new();
    let store = f.store();
    let original = store.syncSnapshot().unwrap();
    let history = f.journal().preferenceHistory().unwrap();
    f.clear_projection();
    assert!(!f.host.exists(&f.path).unwrap());
    // Exercise the production registry, not only the DataStore adapter.
    let schema = PreferenceMergeSchema::default();
    let registry = PreferenceMerge::registry(f.host.clone(), &schema, [], |_, _| true).unwrap();
    let (snapshots, pending, errors) = registry.snapshots().unwrap();
    assert!(pending.is_empty());
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(snapshots.iter().find(|s| s.data.owner.path == f.path), Some(&original));
    assert!(CoreSpaceStore::new(f.host.clone()).space().is_ok());
    assert_eq!(f.host.empty_writes.load(Ordering::SeqCst), 0);
    assert_eq!(f.journal().preferenceHistory().unwrap(), history);
    let writes = f.host.writes.load(Ordering::SeqCst);
    for _ in 0..3 {
        assert_eq!(registry.snapshot(&original.data.owner).unwrap(), original);
        store.prepareSync().unwrap();
    }
    assert_eq!(f.host.writes.load(Ordering::SeqCst), writes, "unchanged comparison must not rewrite files");
}

#[test]
fn a_schema_only_file_can_be_corrected_with_its_observed_token() {
    let f = Fixture::new();
    let store = f.store();
    let original = store.syncSnapshot().unwrap();
    let history = f.journal().preferenceHistory().unwrap();
    // Reproduce files produced by the previous consistency implementation, including a live cache.
    f.host.native.writeBytes(&f.path, br#"{"__operit_preferences_schema_version":3}"#).unwrap();
    let observed = store.syncSnapshot().unwrap();
    assert!(observed.data.values.is_empty());
    assert_ne!(observed.token().unwrap(), original.token().unwrap());
    assert!(store.repairSyncedSnapshot(&observed.token().unwrap(), &original).unwrap());
    assert_eq!(store.syncSnapshot().unwrap().data, original.data);
    assert!(CoreSpaceStore::new(f.host.clone()).space().is_ok());
    assert_eq!(f.journal().preferenceHistory().unwrap(), history);
}

#[test]
fn correction_rejects_a_changed_file_even_when_the_sql_checkpoint_is_unchanged() {
    let f = Fixture::new();
    let store = f.store();
    let original = store.syncSnapshot().unwrap();
    let mut changed: Value = serde_json::from_slice(&f.host.readBytes(&f.path).unwrap()).unwrap();
    changed["record"]["spaceName"] = json!("changed outside the transaction API");
    let bytes = serde_json::to_vec(&changed).unwrap();
    f.host.native.writeBytes(&f.path, &bytes).unwrap();
    assert!(!store.repairSyncedSnapshot(&original.token().unwrap(), &original).unwrap());
    assert_eq!(f.host.readBytes(&f.path).unwrap(), bytes);
    let observed = store.syncSnapshot().unwrap();
    assert!(store.repairSyncedSnapshot(&observed.token().unwrap(), &original).unwrap());
    assert_eq!(store.syncSnapshot().unwrap().data, original.data);
}

#[test]
fn a_failed_projection_restore_stays_absent_and_retries_from_the_same_checkpoint() {
    let f = Fixture::new();
    let original = f.store().syncSnapshot().unwrap();
    f.clear_projection();
    f.host.fail_writes.store(true, Ordering::SeqCst);
    assert!(f.store().syncSnapshot().is_err());
    assert!(!f.host.exists(&f.path).unwrap());
    f.host.fail_writes.store(false, Ordering::SeqCst);
    assert_eq!(f.store().syncSnapshot().unwrap(), original);
    assert_eq!(f.host.empty_writes.load(Ordering::SeqCst), 0);
}

#[test]
fn a_committed_record_deletion_is_preserved_when_its_projection_is_missing() {
    let f = Fixture::new();
    let store = f.store();
    store.edit(|values| { values.remove(&stringPreferencesKey("record")); }).unwrap();
    let deleted = store.syncSnapshot().unwrap();
    assert!(deleted.data.values.is_empty());
    f.clear_projection();
    assert_eq!(store.syncSnapshot().unwrap(), deleted);
    assert!(!f.host.exists(&f.path).unwrap(), "an empty owner needs no placeholder file");
}

#[test]
fn interrupted_file_copy_recovers_the_exact_file_and_its_baseline() {
    use operit_store::Transaction::preference;
    let f = Fixture::new();
    let original = f.store().syncSnapshot().unwrap();
    let file = preference::file_snapshot(&f.store(), &original.token().unwrap()).unwrap() + "\n";
    f.host.native.writeBytes(&f.path, br#"{"__operit_preferences_schema_version":3}"#).unwrap();
    let observed = f.store().syncSnapshot().unwrap();
    f.host.fail_on_write.store(f.host.writes.load(Ordering::SeqCst) + 2, Ordering::SeqCst);
    assert!(preference::repair_file(&f.store(), &observed.token().unwrap(), &original, Some(&file)).is_err());
    // The SQL commit succeeded but final publication failed: the intent must finish the copy.
    let intent: Value = serde_json::from_slice(&f.host.readBytes(&f.path).unwrap()).unwrap();
    assert!(intent.get("__operit_transaction_commit").is_some());
    let recovered = f.store().syncSnapshot().unwrap();
    assert_eq!(recovered.data, original.data);
    assert_eq!(recovered.covered, original.covered);
    assert_eq!(f.host.readBytes(&f.path).unwrap(), file.as_bytes());
    assert!(CoreSpaceStore::new(f.host.clone()).space().is_ok());
}

#[test]
fn invalid_file_correction_leaves_the_current_file_and_checkpoint_untouched() {
    let f = Fixture::new();
    let original = f.store().syncSnapshot().unwrap();
    let bytes = f.host.readBytes(&f.path).unwrap();
    let result = operit_store::Transaction::preference::repair_file(&f.store(), &original.token().unwrap(), &original, Some("{}"));
    assert!(result.is_err());
    assert_eq!(f.host.readBytes(&f.path).unwrap(), bytes);
    assert_eq!(f.store().syncSnapshot().unwrap(), original);
}

#[test]
fn a_missing_generic_preference_installs_the_source_format_and_exact_file() {
    use operit_store::Transaction::preference;
    let source = Fixture::new();
    let target = Fixture::new();
    let path = "runtime/plugin_data/space/com.operit.test/versioned.preferences.json";
    let store = PreferencesDataStore::newWithStorage(source.host.clone(), path).withSchema(1, |_, _| Ok(()));
    store.edit(|values| values.set(&stringPreferencesKey("record"), json!({"name":"authority"}))).unwrap();
    let snapshot = store.syncSnapshot().unwrap();
    let file = preference::file_snapshot(&store, &snapshot.token().unwrap()).unwrap();
    let mut schema = PreferenceMergeSchema::default();
    schema.stores.push(schema.owner(path).unwrap().unwrap());
    let registry = PreferenceMerge::registry(target.host.clone(), &schema, [], |_, _| true).unwrap();
    assert!(registry.repair_file(&snapshot.data.owner, "", &snapshot, Some(&file)).unwrap());
    assert_eq!(target.host.readBytes(path).unwrap(), file.as_bytes());
    assert_eq!(registry.snapshot(&snapshot.data.owner).unwrap().data, snapshot.data);
    let reopened = Arc::new(NativeRuntimeStorageHost::new(target.root.join("runtime"), target.root.join("workspaces")));
    let schema = PreferenceMergeSchema::default();
    let registry = PreferenceMerge::registry(reopened, &schema, [], |_, _| true).unwrap();
    assert_eq!(registry.snapshot(&snapshot.data.owner).unwrap().data, snapshot.data);
}

#[test]
fn aligning_the_baseline_of_an_unchanged_file_does_not_rewrite_it() {
    use operit_store::Transaction::preference;
    let f = Fixture::new();
    let store = f.store();
    let original = store.syncSnapshot().unwrap();
    let file = preference::file_snapshot(&store, &original.token().unwrap()).unwrap();
    let mut source = original.clone();
    source.covered.insert("other-history:1".into());
    let writes = f.host.writes.load(Ordering::SeqCst);
    assert!(preference::repair_file(&store, &original.token().unwrap(), &source, Some(&file)).unwrap());
    assert_eq!(f.host.writes.load(Ordering::SeqCst), writes);
    assert_eq!(f.host.readBytes(&f.path).unwrap(), file.as_bytes());
    assert_eq!(preference::baseline_digest(&store).unwrap(), source.baseline_digest().unwrap());
    assert!(store.syncSnapshot().unwrap().covered.contains("other-history:1"));
    assert!(!preference::repair_file(&store, &original.token().unwrap(), &source, Some(&file)).unwrap());
    assert_eq!(f.host.writes.load(Ordering::SeqCst), writes);
}

#[test]
fn a_rejected_missing_file_copy_does_not_create_a_placeholder_checkpoint() {
    use operit_store::Transaction::preference;
    let f = Fixture::new();
    let path = "runtime/plugin_data/space/com.operit.test/invalid.preferences.json";
    let store = PreferencesDataStore::newWithStorage(f.host.clone(), path);
    let source = operit_store::Transaction::Snapshot { node: "peer".into(), data: operit_store::Transaction::Data::empty(
        operit_store::Transaction::Owner { backend: operit_store::Transaction::Backend::DataStore, path: path.into(), format: 1, layout: 1 }
    ), covered: Default::default(), generation: 0 };
    assert!(preference::repair_file(&store, "", &source, Some("{} ")).is_err());
    assert!(!f.host.exists(path).unwrap());
    assert!(!f.journal().preferenceOwnerPaths().unwrap().contains(&path.to_owned()));
    let valid = "{\"__operit_preferences_schema_version\":1}";
    assert!(preference::repair_file(&store, "", &source, Some(valid)).unwrap());
    assert_eq!(f.host.readBytes(path).unwrap(), valid.as_bytes());
}
