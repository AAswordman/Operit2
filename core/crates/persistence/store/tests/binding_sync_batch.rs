//! Binding clock preparation must not acknowledge application or lose replayable originals.
use std::{path::PathBuf, sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}}};
use operit_host_api::{HostError, HostResult, RuntimeSqliteConnection, RuntimeStorageEntry, RuntimeStorageHost};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::{CoreNodeBindingStore::CoreNodeBindingStore, SyncOperationStore::{SyncOperation, SyncOperationSemantics, SyncOperationStore}};
use serde_json::json;

struct ObservedHost {
    native: NativeRuntimeStorageHost,
    clock_writes: AtomicUsize,
    fail_clock: AtomicBool,
}
impl RuntimeStorageHost for ObservedHost {
    fn openTransactionDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> { self.native.openTransactionDatabase(path) }
    fn runtimeRootDir(&self) -> Option<PathBuf> { self.native.runtimeRootDir() }
    fn workspaceRootDir(&self) -> Option<PathBuf> { self.native.workspaceRootDir() }
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> { self.native.readBytes(path) }
    fn writeBytes(&self, path: &str, bytes: &[u8]) -> HostResult<()> { self.native.writeBytes(path, bytes) }
    fn writeBytesAtomically(&self, path: &str, bytes: &[u8]) -> HostResult<()> {
        if path.ends_with("/clocks.json") {
            if self.fail_clock.load(Ordering::SeqCst) { return Err(HostError::new("injected clock publication failure")); }
            self.clock_writes.fetch_add(1, Ordering::SeqCst);
        }
        self.native.writeBytesAtomically(path, bytes)
    }
    fn appendBytes(&self, path: &str, bytes: &[u8]) -> HostResult<()> { self.native.appendBytes(path, bytes) }
    fn flushFile(&self, path: &str) -> HostResult<()> { self.native.flushFile(path) }
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> { self.native.delete(path, recursive) }
    fn exists(&self, path: &str) -> HostResult<bool> { self.native.exists(path) }
    fn list(&self, path: &str) -> HostResult<Vec<RuntimeStorageEntry>> { self.native.list(path) }
}
struct Fixture { root: PathBuf, host: Arc<ObservedHost> }
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("operit-binding-batch-{}", uuid::Uuid::new_v4()));
        let host = Self::host(&root);
        Self { root, host }
    }
    fn host(root: &std::path::Path) -> Arc<ObservedHost> {
        Arc::new(ObservedHost { native: NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces")), clock_writes: AtomicUsize::new(0), fail_clock: AtomicBool::new(false) })
    }
    fn journal(&self) -> SyncOperationStore { SyncOperationStore::new(self.host.clone(), "runtime/sync") }
}
impl Drop for Fixture { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.root); } }
fn operation(sequence: i64, key: &str) -> SyncOperation {
    SyncOperation { opId: format!("remote:{sequence}"), originDeviceId: "remote".into(), sequence,
        domain: "binding".into(), entityType: "binding".into(), entityId: key.into(), operation: "upsert".into(), semantics: SyncOperationSemantics::EntityState,
        payload: json!({"key":key,"nodeId":"remote-node","generation":sequence}), createdAt: sequence, schemaVersion: 1 }
}

#[test]
fn received_batch_publishes_one_clock_and_keeps_owner_application_independent() {
    let f = Fixture::new();
    let bindings = CoreNodeBindingStore::new(f.host.clone()).unwrap();
    assert!(bindings.bindingOptional("first").unwrap().is_none());
    let journal = f.journal();
    let operations = vec![operation(1, "first"), operation(2, "second"), operation(3, "first")];
    journal.receiveOperations(&operations).unwrap();
    f.host.clock_writes.store(0, Ordering::SeqCst);
    bindings.prepareSyncedOperations(&operations).unwrap();
    assert_eq!(f.host.clock_writes.load(Ordering::SeqCst), 1);
    assert_eq!(journal.pendingOperations().unwrap().len(), 3);
    assert!(journal.shouldApplyOperation(&operations[0]).unwrap());
    assert!(bindings.bindingOptional("first").unwrap().is_none());
    for operation in &operations { bindings.applySyncedOperation(operation).unwrap(); }
    assert_eq!(f.host.clock_writes.load(Ordering::SeqCst), 1, "per-owner application must reuse the durable batch clock");
    assert_eq!(bindings.binding("first").unwrap().generation, 3);
    journal.completeOperations(&operations).unwrap();
    bindings.prepareSyncedOperations(&operations).unwrap();
    for operation in operations.iter().rev() { bindings.applySyncedOperation(operation).unwrap(); }
    assert_eq!(f.host.clock_writes.load(Ordering::SeqCst), 1, "replay must not rewrite an unchanged clock");
    assert_eq!(bindings.binding("first").unwrap().generation, 3);
    assert!(journal.pendingOperations().unwrap().is_empty());
    let reopened_host = Fixture::host(&f.root);
    let reopened = CoreNodeBindingStore::new(reopened_host.clone()).unwrap();
    assert_eq!(reopened.binding("first").unwrap().generation, 3);
    let reopened_journal = SyncOperationStore::new(reopened_host, "runtime/sync");
    assert_eq!(reopened_journal.currentLocalClock().unwrap().sequenceFor("remote"), 3);
    assert!(!reopened_journal.shouldApplyOperation(&operations[2]).unwrap());
}

#[test]
fn failed_clock_publication_retains_pending_originals_for_restart_and_retry() {
    let f = Fixture::new();
    let bindings = CoreNodeBindingStore::new(f.host.clone()).unwrap();
    let journal = f.journal();
    let operations = vec![operation(1, "first"), operation(2, "second")];
    journal.receiveOperations(&operations).unwrap();
    f.host.fail_clock.store(true, Ordering::SeqCst);
    assert!(bindings.prepareSyncedOperations(&operations).unwrap_err().contains("injected clock"));
    assert_eq!(journal.currentLocalClock().unwrap().sequenceFor("remote"), 0);
    let host = Fixture::host(&f.root);
    let journal = SyncOperationStore::new(host.clone(), "runtime/sync");
    let bindings = CoreNodeBindingStore::new(host).unwrap();
    let pending = journal.pendingOperations().unwrap();
    assert_eq!(pending.len(), operations.len());
    bindings.prepareSyncedOperations(&pending).unwrap();
    for operation in &pending { bindings.applySyncedOperation(operation).unwrap(); }
    journal.completeOperations(&pending).unwrap();
    assert!(journal.pendingOperations().unwrap().is_empty());
    assert_eq!(bindings.binding("second").unwrap().generation, 2);
}

#[test]
fn invalid_binding_does_not_prevent_preparing_and_applying_other_owners() {
    let f = Fixture::new();
    let bindings = CoreNodeBindingStore::new(f.host.clone()).unwrap();
    let journal = f.journal();
    let valid = operation(1, "valid");
    let mut invalid = operation(2, "invalid");
    invalid.payload["key"] = json!("different-key");
    journal.receiveOperations(&[valid.clone(), invalid.clone()]).unwrap();
    bindings.prepareSyncedOperations(&journal.pendingOperations().unwrap()).unwrap();
    assert_eq!(journal.currentLocalClock().unwrap().sequenceFor("remote"), 1);
    assert!(bindings.applySyncedOperation(&invalid).is_err());
    bindings.applySyncedOperation(&valid).unwrap();
    assert!(!journal.shouldApplyOperation(&valid).unwrap());
    journal.completeOperations(&[valid]).unwrap();
    assert_eq!(journal.pendingOperations().unwrap().len(), 1);
    assert_eq!(journal.pendingOperations().unwrap()[0].opId, invalid.opId);
}

#[test]
fn identical_redelivery_does_not_rewrite_pending_originals_and_still_rejects_conflicts() {
    use operit_store::SqliteStore::SqliteRowGet;
    let f = Fixture::new();
    let journal = f.journal();
    let original = operation(1, "first");
    journal.receiveOperations(&[original.clone()]).unwrap();
    let mut connection = f.host.openTransactionDatabase("runtime/sync/transactions.sqlite").unwrap();
    connection.executeBatch("CREATE TABLE write_audit(value INTEGER); CREATE TRIGGER audit_original_update AFTER UPDATE ON originals BEGIN INSERT INTO write_audit VALUES(1); END;").unwrap();
    journal.receiveOperations(&[original.clone()]).unwrap();
    journal.appendOperations(&[original.clone()]).unwrap();
    assert_eq!(connection.query("SELECT COUNT(*) FROM write_audit", vec![]).unwrap()[0].get::<_, i64>(0).unwrap(), 0);
    assert_eq!(journal.pendingOperations().unwrap(), vec![original.clone()]);
    let mut conflicting = original;
    conflicting.payload["nodeId"] = json!("other-peer");
    assert!(journal.receiveOperations(&[conflicting]).is_err());
    assert_eq!(journal.pendingOperations().unwrap().len(), 1);
}

#[test]
fn file_clock_batch_survives_reopen_without_acknowledging_missing_blobs() {
    use operit_store::RuntimeFileSyncStore::RuntimeFileSyncStore;
    let f = Fixture::new();
    let files = RuntimeFileSyncStore::new(f.host.clone(), "runtime/sync");
    let journal = f.journal();
    let mut valid = operation(1, "unused");
    valid.domain = "runtime_file".into();
    valid.entityType = "file".into();
    valid.entityId = "runtime/extensions/space/test.txt".into();
    valid.payload = json!({"contentHash":"0".repeat(64),"size":1});
    let mut malformed = valid.clone();
    malformed.opId = "remote:2".into(); malformed.sequence = 2; malformed.payload = json!({"size":1});
    journal.receiveOperations(&[valid.clone(), malformed.clone()]).unwrap();
    let receipt = journal.receipt().unwrap();
    f.host.clock_writes.store(0, Ordering::SeqCst);
    files.prepareSyncedOperations(&[valid.clone(), malformed.clone()]).unwrap();
    assert_eq!(f.host.clock_writes.load(Ordering::SeqCst), 1);
    assert_eq!(journal.localClock().unwrap().sequenceFor("remote"), 1);
    assert_eq!(journal.pendingOperations().unwrap().len(), 2);
    assert!(RuntimeFileSyncStore::applySyncedOperation(f.host.clone(), "runtime/sync", &valid.entityId, &valid.operation, valid.payload.clone()).is_err());
    assert!(journal.shouldApplyOperation(&valid).unwrap());
    assert_eq!(journal.receipt().unwrap(), receipt);
    let reopened = SyncOperationStore::new(Fixture::host(&f.root), "runtime/sync");
    assert_eq!(reopened.pendingOperations().unwrap().len(), 2);
    assert!(reopened.shouldApplyOperation(&valid).unwrap());
    let mut deletion = valid.clone();
    deletion.operation = "delete".into(); deletion.payload = serde_json::Value::Null;
    // A different original cannot replace the immutable pending operation.
    assert!(reopened.appendOperation(&deletion).is_err());
}

#[test]
fn control_batch_publishes_one_independent_clock_and_keeps_pending_recoverable() {
    let f = Fixture::new();
    let journal = f.journal();
    let operations = (1..=50).map(|sequence| {
        let mut command = operation(sequence, "control");
        command.domain = operit_store::NetworkControlStore::NETWORK_CONTROL_SYNC_DOMAIN.into();
        command
    }).collect::<Vec<_>>();
    journal.receiveOperations(&operations).unwrap();
    journal.appendUnobservedOperations(&operations).unwrap();
    f.host.fail_clock.store(true, Ordering::SeqCst);
    assert!(journal.observeNetworkControlOperations(&operations).is_err());
    assert_eq!(journal.currentLocalClock().unwrap().sequenceFor("remote"), 0);
    assert_eq!(journal.pendingOperations().unwrap().len(), 50);
    f.host.fail_clock.store(false, Ordering::SeqCst);
    f.host.clock_writes.store(0, Ordering::SeqCst);
    journal.observeNetworkControlOperations(&operations).unwrap();
    journal.observeNetworkControlOperations(&operations).unwrap();
    assert_eq!(f.host.clock_writes.load(Ordering::SeqCst), 1);
    assert_eq!(journal.currentLocalClock().unwrap().sequenceFor("remote"), 0);
    let cold = SyncOperationStore::new(Fixture::host(&f.root), "runtime/sync");
    assert_eq!(cold.currentLocalClock().unwrap(), journal.currentLocalClock().unwrap());
    assert_eq!(cold.pendingOperations().unwrap().len(), 50);
}

#[test]
fn business_clock_batch_preserves_pending_work_and_retries_failed_publication() {
    let f = Fixture::new();
    let journal = f.journal();
    let operations = vec![operation(1, "first"), operation(2, "second"), operation(3, "third")];
    journal.receiveOperations(&operations).unwrap();
    f.host.clock_writes.store(0, Ordering::SeqCst);
    journal.observeOperations(&operations).unwrap();
    assert_eq!(f.host.clock_writes.load(Ordering::SeqCst), 1);
    assert_eq!(journal.pendingOperations().unwrap().len(), 3);
    assert!(operations.iter().all(|op| journal.shouldApplyOperation(op).unwrap()));
    journal.observeOperations(&operations).unwrap();
    assert_eq!(f.host.clock_writes.load(Ordering::SeqCst), 1);
    let later = operation(4, "fourth");
    journal.receiveOperations(std::slice::from_ref(&later)).unwrap();
    f.host.fail_clock.store(true, Ordering::SeqCst);
    assert!(journal.observeOperations([&later]).is_err());
    assert_eq!(journal.currentLocalClock().unwrap().sequenceFor("remote"), 3);
    assert_eq!(journal.pendingOperations().unwrap().len(), 4);
    f.host.fail_clock.store(false, Ordering::SeqCst);
    journal.observeOperations([&later]).unwrap();
    assert_eq!(journal.currentLocalClock().unwrap().sequenceFor("remote"), 4);
    let mut control = operation(5, "control");
    control.domain = "network_control".into();
    assert!(journal.observeOperations([&control]).is_err());
    assert_eq!(journal.currentLocalClock().unwrap().sequenceFor("remote"), 4);
}
