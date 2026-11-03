use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::PreferencesDataStore::{stringPreferencesKey, PreferencesDataStore};
use operit_store::SyncOperationStore::{SyncClock, SyncOperationKnowledge, SyncOperationStore};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const SYNC: &str = "runtime/sync";
const FILE: &str = "runtime/config/preferences/workspaces.preferences.json";

struct Node(std::path::PathBuf);
impl Node {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("operit-legacy-recovery-{}", uuid::Uuid::new_v4())))
    }
    fn storage(&self) -> Arc<NativeRuntimeStorageHost> {
        Arc::new(NativeRuntimeStorageHost::new(
            self.0.join("custom runtime"),
            self.0.join("custom workspaces"),
        ))
    }
}
impl Drop for Node {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn seed(host: &dyn RuntimeStorageHost, external: bool, payload_exists: bool) {
    let op = json!({"opId":"remote:1", "originDeviceId":"remote", "sequence":1,
        "domain":"preferences", "entityType":"preference", "entityId":"runtime/unrelated.preferences.json",
        "operation":"set", "semantics":"transaction", "payload":{}, "createdAt":1, "schemaVersion":1});
    let bytes = serde_json::to_vec(&op).unwrap();
    let pending: Value = if external {
        json!({"record": {
            "offset":0,"length":bytes.len(), "op_id":"remote:1", "origin_device_id":"remote", "sequence":1,
            "domain":"preferences", "entity_type":"preference", "entity_id":"runtime/unrelated.preferences.json",
            "operation":"set", "semantics":"transaction", "route":{"encrypted":false,"storage_path":null,
            "merge_metadata":false,"contract_version":null,"logical_time":null,"initial":false}, "created_at":1
        }})
    } else {
        op
    };
    let state = json!({"receipt":{"ranges":{"completed":[[1,8]]}},"pending":{"remote:1":pending},"fingerprints":{}});
    host.writeBytes(
        "runtime/sync/receive_state.json",
        &serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();
    host.writeBytes(FILE, br#"{"sentinel":"keep"}"#).unwrap();
    if payload_exists {
        let path = format!(
            "runtime/sync/pending/{:x}.json",
            Sha256::digest(b"remote:1")
        );
        host.writeBytes(&path, &bytes).unwrap();
    }
}

fn assert_business_data(store: &PreferencesDataStore) {
    assert_eq!(
        store
            .data()
            .unwrap()
            .getTyped::<String>(&stringPreferencesKey("sentinel"))
            .unwrap()
            .as_deref(),
        Some("keep")
    );
}

#[test]
fn missing_legacy_payload_resets_ledger_and_rebuilds_exportable_baseline() {
    let node = Node::new();
    let host = node.storage();
    seed(host.as_ref(), true, false);
    host.writeBytes(
        "runtime/sync/preference_outboxes.json",
        br#"{"runtime/old.preferences.json":false}"#,
    )
    .unwrap();
    let original_file = host.readBytes(FILE).unwrap();
    {
        let journal = SyncOperationStore::new(host.clone(), SYNC);
        let store = PreferencesDataStore::newWithStorage(host.clone(), FILE);
        assert_business_data(&store);
        assert_eq!(host.readBytes(FILE).unwrap(), original_file);
        assert!(journal.receipt().unwrap().ranges.is_empty());
        assert!(journal.pendingOperationIds().unwrap().is_empty());
        assert!(journal.preferenceDatasets().unwrap().is_empty());
        let snapshot = store.syncSnapshot().unwrap();
        assert_eq!(snapshot.data.values["sentinel"], json!("keep"));
        let history = journal.preferenceHistory().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].payload["storagePath"], FILE);
        assert!(snapshot.covered.contains(&history[0].opId));

        let peer = Node::new();
        let destination = PreferencesDataStore::newWithStorage(peer.storage(), FILE);
        let empty = destination.syncSnapshot().unwrap();
        assert!(destination
            .repairSyncedSnapshot(&empty.token().unwrap(), &snapshot)
            .unwrap());
        assert_business_data(&destination);
    }
    drop(host);
    // A fresh Host must use the committed reset marker, even if the retired inputs get worse.
    let host = node.storage();
    host.writeBytes("runtime/sync/receive_state.json", b"broken ledger")
        .unwrap();
    let journal = SyncOperationStore::new(host.clone(), SYNC);
    let store = PreferencesDataStore::newWithStorage(host, FILE);
    assert_business_data(&store);
    assert_eq!(journal.preferenceHistory().unwrap().len(), 1);
}

#[test]
fn valid_preview18_inline_and_later_external_ledgers_still_import() {
    for external in [false, true] {
        let node = Node::new();
        let host = node.storage();
        seed(host.as_ref(), external, external);
        assert_business_data(&PreferencesDataStore::newWithStorage(host.clone(), FILE));
        let journal = SyncOperationStore::new(host, SYNC);
        assert_eq!(journal.pendingOperationIds().unwrap(), vec!["remote:1"]);
        assert!(journal.receipt().unwrap().containsSequence("completed", 8));
    }
}

#[test]
fn corrupt_legacy_inputs_reset_without_losing_business_files() {
    for path in [
        "runtime/sync/receive_state.json",
        "runtime/sync/preference_outboxes.json",
        "runtime/sync/operations/remote.jsonl",
    ] {
        let node = Node::new();
        let host = node.storage();
        seed(host.as_ref(), false, false);
        host.writeBytes("runtime/sync/devices.json", br#"["remote"]"#)
            .unwrap();
        host.writeBytes(path, b"corrupt legacy data").unwrap();
        assert_business_data(&PreferencesDataStore::newWithStorage(host.clone(), FILE));
        let journal = SyncOperationStore::new(host, SYNC);
        assert!(journal.receipt().unwrap().ranges.is_empty());
        assert!(journal.pendingOperationIds().unwrap().is_empty());
    }
}

#[test]
fn reset_preserves_authored_sequence_floor() {
    let node = Node::new();
    let host = node.storage();
    seed(host.as_ref(), true, false);
    host.writeBytes("runtime/sync/local_device_id", b"local")
        .unwrap();
    host.writeBytes(
        "runtime/sync/clocks.json",
        br#"{"sequences":{"local:preferences":41}}"#,
    )
    .unwrap();
    let store = PreferencesDataStore::newWithStorage(host.clone(), FILE);
    store
        .edit(|values| values.set(&stringPreferencesKey("sentinel"), "changed"))
        .unwrap();
    let journal = SyncOperationStore::new(host, SYNC);
    let history = journal.preferenceHistory().unwrap();
    let local = history
        .iter()
        .find(|op| op.originDeviceId == "local:preferences")
        .unwrap();
    assert_eq!(local.sequence, 42);
    assert!(journal
        .operationsSince(&SyncClock::empty(), &["preferences".into()], 100)
        .unwrap()
        .contains(local));
}

#[test]
fn failed_reset_is_not_reported_as_success() {
    let node = Node::new();
    let host = node.storage();
    seed(host.as_ref(), true, false);
    let journal = SyncOperationStore::new(host.clone(), SYNC);
    // Force a fresh open to retry the legacy import, then reject the reset marker.
    assert!(journal.receipt().is_ok());
    let mut sql = host
        .openTransactionDatabase("runtime/sync/transactions.sqlite")
        .unwrap();
    sql.executeBatch(
        "DELETE FROM journal_meta;
        CREATE TRIGGER reject_reset BEFORE INSERT ON journal_meta
        BEGIN SELECT RAISE(ABORT,'reset cannot commit'); END;",
    )
    .unwrap();
    drop(sql);
    drop(journal);
    drop(host);
    let host = node.storage();
    let journal = SyncOperationStore::new(host.clone(), SYNC);
    assert!(journal
        .receipt()
        .unwrap_err()
        .to_string()
        .contains("reset cannot commit"));
    assert_eq!(host.readBytes(FILE).unwrap(), br#"{"sentinel":"keep"}"#);
}
