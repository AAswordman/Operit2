//! Production journal regressions: restart, atomic identity validation and legacy handover.
use std::{path::PathBuf, sync::Arc};
use operit_host_api::{RuntimeStorageHost, SqliteValue};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::SyncOperationStore::{SyncClock, SyncOperation, SyncOperationKnowledge, SyncOperationSemantics, SyncOperationStore};
use serde_json::json;

const ROOT: &str = "runtime/sync";
struct Fixture { root: PathBuf, host: Arc<NativeRuntimeStorageHost> }
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("operit-production-journal-{}", uuid::Uuid::new_v4()));
        let host = Arc::new(NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces")));
        Self { root, host }
    }
    fn journal(&self) -> SyncOperationStore { SyncOperationStore::new(self.host.clone(), ROOT) }
    fn reopen(&self) -> SyncOperationStore {
        SyncOperationStore::new(Arc::new(NativeRuntimeStorageHost::new(self.root.join("runtime"), self.root.join("workspaces"))), ROOT)
    }
}
impl Drop for Fixture { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.root); } }

fn operation(origin: &str, sequence: i64) -> SyncOperation {
    SyncOperation {
        opId: format!("{origin}:{sequence}"), originDeviceId: origin.into(), sequence,
        domain: "test_sql".into(), entityType: "record".into(), entityId: "example".into(),
        operation: "transaction".into(), semantics: SyncOperationSemantics::Transaction,
        payload: json!({"value":sequence}), createdAt: sequence * 10, schemaVersion: 1,
    }
}

#[test]
fn pending_and_sparse_receipts_survive_restart_without_a_file_ledger() {
    let fixture = Fixture::new();
    let op = operation("peer", 3);
    let journal = fixture.journal();
    journal.receiveOperations(&[op.clone()]).unwrap();
    drop(journal);
    let journal = fixture.reopen();
    assert_eq!(journal.pendingOperations().unwrap(), vec![op.clone()]);
    let receipt = journal.receipt().unwrap();
    assert!(receipt.containsSequence("peer", 3));
    assert!(!receipt.containsSequence("peer", 1));
    assert_eq!(receipt.sequenceFor("peer"), 0);
    assert_eq!(journal.operationsSince(&SyncClock::empty(), &[], 10).unwrap(), vec![op]);
    assert!(!fixture.host.exists(&format!("{ROOT}/receive_state.json")).unwrap());
    assert!(!fixture.host.exists(&format!("{ROOT}/pending")).unwrap());
    assert!(!fixture.host.exists(&format!("{ROOT}/operations")).unwrap());
}

#[test]
fn identity_collision_rolls_back_the_whole_batch_in_both_orders() {
    for conflict_first in [false, true] {
        let fixture = Fixture::new();
        let journal = fixture.journal();
        let original = operation("peer", 1);
        journal.receiveOperations(&[original.clone()]).unwrap();
        let mut conflict = original.clone();
        conflict.payload = json!({"value":"changed original"});
        let fresh = operation("peer", 2);
        let batch = if conflict_first { vec![conflict, fresh] } else { vec![fresh, conflict] };
        assert!(journal.receiveOperations(&batch).is_err());
        let reopened = fixture.reopen();
        assert_eq!(reopened.pendingOperations().unwrap(), vec![original]);
        assert!(!reopened.receipt().unwrap().containsSequence("peer", 2));
    }
}

#[test]
fn completion_preserves_the_original_for_export_and_rejects_changed_redelivery() {
    let fixture = Fixture::new();
    let journal = fixture.journal();
    let op = operation("peer", 1);
    journal.receiveOperations(&[op.clone()]).unwrap();
    journal.completeOperations(&[op.clone()]).unwrap();
    drop(journal);
    let journal = fixture.reopen();
    assert!(journal.pendingOperations().unwrap().is_empty());
    assert_eq!(journal.operationsSince(&SyncClock::empty(), &[], 10).unwrap(), vec![op.clone()]);
    journal.receiveOperations(&[op.clone()]).unwrap();
    assert!(journal.pendingOperations().unwrap().is_empty());
    let mut changed = op;
    changed.createdAt += 1;
    assert!(journal.receiveOperations(&[changed]).is_err());
}

#[test]
fn legacy_handover_imports_pending_and_keeps_original_files_read_only() {
    let fixture = Fixture::new();
    let journal = fixture.journal();
    let archived = operation("peer", 1);
    let pending = operation("peer", 3);
    let legacy = format!("{}\n", serde_json::to_string(&archived).unwrap());
    let log = journal.operationsPath("peer");
    fixture.host.writeBytesAtomically(&log, legacy.as_bytes()).unwrap();
    fixture.host.writeBytesAtomically(&format!("{ROOT}/devices.json"), br#"["peer"]"#).unwrap();
    let received = serde_json::to_vec(&json!({
        "receipt":{"ranges":{"peer":[[1,1],[3,3]]}}, "pending":{pending.opId.clone(): pending.clone()}
    })).unwrap();
    fixture.host.writeBytesAtomically(&format!("{ROOT}/receive_state.json"), &received).unwrap();
    assert_eq!(journal.pendingOperations().unwrap(), vec![pending.clone()]);
    journal.completeOperations(&[pending]).unwrap();
    journal.receiveOperations(&[operation("peer", 4)]).unwrap();
    assert_eq!(fixture.host.readBytes(&log).unwrap(), legacy.as_bytes());
    assert_eq!(fixture.host.readBytes(&format!("{ROOT}/receive_state.json")).unwrap(), received);
    let receipt = fixture.reopen().receipt().unwrap();
    assert!(receipt.containsSequence("peer", 1));
    assert!(!receipt.containsSequence("peer", 2));
    assert!(receipt.containsSequence("peer", 3));
    assert!(receipt.containsSequence("peer", 4));
}

#[test]
fn failed_legacy_import_rolls_back_before_retry() {
    let fixture = Fixture::new();
    let journal = fixture.journal();
    let op = operation("peer", 1);
    let valid = format!("{}\n", serde_json::to_string(&op).unwrap());
    let log = journal.operationsPath("peer");
    fixture.host.writeBytesAtomically(&log, format!("{valid}broken-json\n").as_bytes()).unwrap();
    fixture.host.writeBytesAtomically(&format!("{ROOT}/devices.json"), br#"["peer"]"#).unwrap();
    assert!(journal.receipt().is_err());
    let mut sql = fixture.host.openTransactionDatabase(&format!("{ROOT}/transactions.sqlite")).unwrap();
    assert_eq!(sql.query("SELECT COUNT(*) FROM originals", vec![]).unwrap()[0].valueAt(0).unwrap(), &SqliteValue::Integer(0));
    fixture.host.writeBytesAtomically(&log, valid.as_bytes()).unwrap();
    assert!(journal.receipt().unwrap().containsSequence("peer", 1));
    assert_eq!(journal.operationsSince(&SyncClock::empty(), &[], 10).unwrap(), vec![op]);
}
