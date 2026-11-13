use super::*;
use crate::SyncOperationStore::{SyncClock, SyncOperationKnowledge};
use operit_host_native_storage::NativeRuntimeStorageHost;

const ROOT: &str = "runtime/sync";
const FILE: &str = "runtime/plugin_data/space/com.operit.workflow/workflows.json";

struct Node(std::path::PathBuf);
impl Node {
    fn new() -> Self { Self(std::env::temp_dir().join(format!("operit-file-index-{}", uuid::Uuid::new_v4()))) }
    fn host(&self) -> Arc<NativeRuntimeStorageHost> {
        Arc::new(NativeRuntimeStorageHost::new(self.0.join("runtime"), self.0.join("workspaces")))
    }
}
impl Drop for Node { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }

fn collect(files: &RuntimeFileSyncStore) {
    files.syncOperationStore.withJournal(|journal| journal.collect_file_blobs(files.storageHost.as_ref(), ROOT, i64::MAX)).unwrap();
}

fn legacy(host: &dyn RuntimeStorageHost, origin: &str, sequence: i64, bytes: &[u8]) -> SyncOperation {
    let reference = RuntimeFileSyncReference { contentHash: contentHash(bytes), size: bytes.len() as i64 };
    host.writeBytes(&format!("{ROOT}/blobs/{}", reference.contentHash), bytes).unwrap();
    SyncOperation { opId: format!("{origin}:{sequence}"), originDeviceId: origin.into(), sequence,
        domain: RUNTIME_FILE_SYNC_DOMAIN.into(), entityType: "file".into(), entityId: FILE.into(),
        operation: "upsert".into(), semantics: SyncOperationSemantics::EntityState,
        payload: serde_json::to_value(reference).unwrap(), createdAt: sequence, schemaVersion: 1 }
}

#[test]
fn repeated_file_updates_only_index_live_bytes_and_keep_the_wire_protocol() {
    let node = Node::new();
    let host = node.host();
    let files = RuntimeFileSyncStore::new(host.clone(), ROOT);
    let mut latest = Vec::new();
    for run in 0..100 {
        latest = format!("{{\"version\":2,\"run\":{run},\"output\":\"{}\"}}", "x".repeat(100_000)).into_bytes();
        files.writeBytes(FILE, &latest).unwrap();
    }
    assert!(host.list(&format!("{ROOT}/blobs")).unwrap().is_empty());
    let operations = files.syncOperationStore.operationsSince(&SyncClock::empty(), &[RUNTIME_FILE_SYNC_DOMAIN.into()], 100).unwrap();
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].sequence, 100);
    let reference = RuntimeFileSyncStore::requiredBlob(&operations[0]).unwrap().unwrap();
    assert!(files.hasBlob(&reference).unwrap());
    let mut transferred = Vec::new();
    while transferred.len() < latest.len() {
        transferred.extend(files.readBlobChunk(&reference.contentHash, transferred.len() as i64, 64 * 1024).unwrap());
    }
    assert_eq!(transferred, latest);
    assert_eq!(files.readBlob(&reference).unwrap(), latest);
    assert_eq!(files.readStagedJson::<serde_json::Value>(&reference).unwrap().0, serde_json::from_slice::<serde_json::Value>(&latest).unwrap());
    let receipt = files.syncOperationStore.receipt().unwrap();
    assert!((1..=100).all(|sequence| receipt.containsSequence(&operations[0].originDeviceId, sequence)));

    // An unchanged old peer still downloads the same hash/size payload, then materializes it.
    let peer = Node::new();
    let peer_host = peer.host();
    let destination = RuntimeFileSyncStore::new(peer_host.clone(), ROOT);
    destination.syncOperationStore.receiveOperations(&operations).unwrap();
    destination.writeBlob(&reference, &transferred).unwrap();
    RuntimeFileSyncStore::applySyncedOperation(peer_host.clone(), ROOT, FILE, "upsert", operations[0].payload.clone()).unwrap();
    collect(&destination);
    assert!(peer_host.exists(&destination.blobStoragePath(&reference).unwrap()).unwrap(), "pending reception must retain its blob");
    destination.syncOperationStore.completeOperations(&operations).unwrap();
    collect(&destination);
    assert!(!peer_host.exists(&destination.blobStoragePath(&reference).unwrap()).unwrap());
    assert_eq!(destination.readBlob(&reference).unwrap(), latest);
}

#[test]
fn upgrade_deletes_old_logs_and_file_copies_after_a_durable_import() {
    let node = Node::new();
    let host = node.host();
    let operations: Vec<_> = (1..=3).map(|sequence| legacy(host.as_ref(), "old-device", sequence, format!("revision {sequence}").as_bytes())).collect();
    host.writeBytes(FILE, b"revision 3").unwrap();
    // Discovery also imports a log whose devices.json registration was lost.
    let log = format!("{ROOT}/operations/old-device.jsonl");
    let bytes = operations.iter().map(|op| serde_json::to_string(op).unwrap()+"\n").collect::<String>();
    host.writeBytes(&log, bytes.as_bytes()).unwrap();
    let quarantine = format!("{log}.quarantine");
    host.writeBytes(&quarantine, b"manual recovery evidence").unwrap();
    let pending = format!("{ROOT}/pending/orphan.json");
    host.writeBytes(&pending, b"orphan private staging").unwrap();
    {
        let journal = SyncOperationStore::new(host.clone(), ROOT);
        let exported = journal.operationsSince(&SyncClock::empty(), &[RUNTIME_FILE_SYNC_DOMAIN.into()], 100).unwrap();
        assert_eq!(exported, vec![operations[2].clone()]);
        assert!(!host.exists(&log).unwrap());
        assert!(!host.exists(&quarantine).unwrap());
        assert!(!host.exists(&pending).unwrap());
        assert!(host.list(&format!("{ROOT}/blobs")).unwrap().is_empty());
        assert!(journal.receipt().unwrap().containsSequence("old-device", 1));
    }
    let files = RuntimeFileSyncStore::new(node.host(), ROOT);
    let reference = RuntimeFileSyncStore::requiredBlob(&operations[2]).unwrap().unwrap();
    assert_eq!(files.readBlob(&reference).unwrap(), b"revision 3");
    assert_eq!(host.readBytes(FILE).unwrap(), b"revision 3");
}

#[test]
fn pending_deferred_and_transaction_references_protect_legacy_content() {
    let node = Node::new();
    let host = node.host();
    let old = legacy(host.as_ref(), "remote", 1, b"pending content");
    let reference = RuntimeFileSyncStore::requiredBlob(&old).unwrap().unwrap();
    let files = RuntimeFileSyncStore::new(host.clone(), ROOT);
    files.syncOperationStore.receiveOperations(std::slice::from_ref(&old)).unwrap();
    files.syncOperationStore.withJournal(|journal| journal.transaction(|tx| {
        tx.execute("INSERT OR IGNORE INTO sync_file_garbage VALUES(?1,0)", vec![crate::Transaction::text(&reference.contentHash)])?;
        Ok(())
    })).unwrap();
    collect(&files);
    assert_eq!(files.readBlob(&reference).unwrap(), b"pending content");
    let inventory = format!("{ROOT}/workspace_inventory.json");
    host.writeBytes(&inventory, &serde_json::to_vec(&serde_json::json!({"deferred":{"workspaces/test/file":reference}})).unwrap()).unwrap();
    files.syncOperationStore.completeOperations(std::slice::from_ref(&old)).unwrap();
    files.writeBytes(FILE, b"new live state").unwrap();
    collect(&files);
    assert!(host.exists(&files.blobStoragePath(&reference).unwrap()).unwrap());
    host.delete(&inventory, false).unwrap();
    let mut shared = old;
    shared.opId = "other:1".into();
    shared.originDeviceId = "other".into();
    shared.domain = "attachment".into();
    files.syncOperationStore.appendOperation(&shared).unwrap();
    collect(&files);
    assert!(host.exists(&files.blobStoragePath(&reference).unwrap()).unwrap());
    files.syncOperationStore.withJournal(|journal| journal.transaction(|tx|
        crate::Transaction::Journal::retire_ids(tx, [shared.opId.clone()]))).unwrap();
    collect(&files);
    assert!(!host.exists(&files.blobStoragePath(&reference).unwrap()).unwrap());
}

#[test]
fn externally_changed_files_never_serve_the_previous_hash() {
    let node = Node::new();
    let host = node.host();
    let files = RuntimeFileSyncStore::new(host.clone(), ROOT);
    files.writeBytes(FILE, b"before").unwrap();
    let reference = RuntimeFileSyncReference { contentHash: contentHash(b"before"), size: 6 };
    let modified_at = host.modifiedAt(FILE).unwrap();
    host.writeBytes(FILE, b"external edit").unwrap();
    host.setModifiedAt(FILE, modified_at + 1000).unwrap();
    assert!(!files.hasBlob(&reference).unwrap());
    assert!(files.readBlobChunk(&reference.contentHash, 0, 64).is_err());
    files.writeBytes(FILE, b"external edit").unwrap();
    let reference = RuntimeFileSyncReference { contentHash: contentHash(b"external edit"), size: 13 };
    assert_eq!(files.readBlob(&reference).unwrap(), b"external edit");
    files.delete(FILE).unwrap();
    assert!(!files.hasBlob(&reference).unwrap());
}

#[test]
fn upgrade_preserves_legacy_blobs_when_live_files_cannot_replace_them() {
    for live in [None, Some(b"external edit".as_slice())] {
        let node = Node::new();
        let host = node.host();
        let operation = legacy(host.as_ref(), "old-device", 1, b"original");
        let reference = RuntimeFileSyncStore::requiredBlob(&operation).unwrap().unwrap();
        host.writeBytes(&format!("{ROOT}/operations/old-device.jsonl"), serde_json::to_string(&operation).unwrap().as_bytes()).unwrap();
        if let Some(bytes) = live { host.writeBytes(FILE, bytes).unwrap(); }
        let files = RuntimeFileSyncStore::new(host.clone(), ROOT);
        collect(&files);
        assert_eq!(files.readBlob(&reference).unwrap(), b"original");
        files.writeBytes(FILE, b"new state").unwrap();
        collect(&files);
        assert!(!host.exists(&files.blobStoragePath(&reference).unwrap()).unwrap());
    }
}
