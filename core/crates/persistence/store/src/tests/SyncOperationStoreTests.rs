use super::*;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};

use operit_host_api::{HostError, RuntimeStorageEntry};
use operit_util::RuntimeStorageLayout::{
    CONFIG_PREFERENCES_DIR_PATH, RUNTIME_WEBSESSION_BROWSER_BOOKMARKS_PATH,
};
use serde_json::{json, Value};

#[derive(Clone, Default)]
struct MemoryStorageHost {
    journalHost: Arc<crate::transaction_test_host::TransactionJournalHost>,
    releasePreferencesCache: bool,
    files: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    writeCount: Arc<AtomicUsize>,
    readCount: Arc<AtomicUsize>,
    policyReads: Arc<AtomicUsize>,
    rangeBytes: Arc<AtomicUsize>,
    forbidWholeJournalReads: Arc<AtomicBool>,
    forbidControlHistoryReads: Arc<AtomicBool>,
    forbidWholePolicyReads: Arc<AtomicBool>,
    failAtomic: Arc<AtomicUsize>,
}

impl MemoryStorageHost {
    /// Returns the number of complete writes and appends served by this test host.
    fn writeCount(&self) -> usize {
        self.writeCount.load(Ordering::SeqCst)
    }
}

impl RuntimeStorageHost for MemoryStorageHost {
    fn openTransactionDatabase(&self, path: &str) -> operit_host_api::HostResult<Box<dyn operit_host_api::RuntimeSqliteConnection>> {
        self.journalHost.open(path)
    }

    fn retainPreferencesCache(&self) -> bool { !self.releasePreferencesCache }
    /// Confirms synchronous in-memory publication without copying or rewriting bytes.
    fn flushFile(&self, path: &str) -> operit_host_api::HostResult<()> {
        if !self.exists(path)? {
            return Err(operit_host_api::HostError::new("runtime storage entry not found"));
        }
        Ok(())
    }

    fn runtimeRootDir(&self) -> Option<std::path::PathBuf> {
        None
    }

    fn workspaceRootDir(&self) -> Option<std::path::PathBuf> {
        None
    }

    fn readBytes(&self, path: &str) -> operit_host_api::HostResult<Vec<u8>> {
        if self.forbidControlHistoryReads.load(Ordering::SeqCst)
            && (path.ends_with(".jsonl") || path.ends_with("space_policy.preferences.json")) {
            return Err(HostError::new("control query attempted to read authority history"));
        }
        if self.forbidWholeJournalReads.load(Ordering::SeqCst) && path.ends_with(".jsonl") {
            return Err(HostError::new("unbounded journal read"));
        }
        if self.forbidWholePolicyReads.load(Ordering::SeqCst) && path.ends_with("space_policy.preferences.json") {
            return Err(HostError::new("unbounded policy read"));
        }
        self.readCount.fetch_add(1, Ordering::SeqCst);
        if path.ends_with("space_policy.preferences.json") { self.policyReads.fetch_add(1, Ordering::SeqCst); }
        let files = self
            .files
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        match files.get(path) {
            Some(content) => Ok(content.clone()),
            None => Err(HostError::new(format!(
                "missing runtime storage file: {path}"
            ))),
        }
    }

    /// Reads one bounded byte range from an in-memory test file.
    fn readBytesRange(
        &self,
        path: &str,
        offset: u64,
        length: usize,
    ) -> operit_host_api::HostResult<Vec<u8>> {
        if self.forbidControlHistoryReads.load(Ordering::SeqCst)
            && (path.ends_with(".jsonl") || path.ends_with("space_policy.preferences.json")) {
            return Err(HostError::new("control query attempted to read authority history"));
        }
        if self.forbidWholeJournalReads.load(Ordering::SeqCst) && path.ends_with(".jsonl") {
            assert!(length <= crate::LegacyTransactionImport::JOURNAL_READ_BYTES);
        }
        self.readCount.fetch_add(1, Ordering::SeqCst);
        if path.ends_with("space_policy.preferences.json") { self.policyReads.fetch_add(1, Ordering::SeqCst); }
        let files = self.files.lock().map_err(|error| HostError::new(error.to_string()))?;
        let content = files.get(path).ok_or_else(|| HostError::new("missing runtime storage file"))?;
        let start = usize::try_from(offset)
            .map_err(|_| HostError::new("runtime storage offset does not fit usize"))?;
        if start >= content.len() {
            return Ok(Vec::new());
        }
        let end = start
            .checked_add(length)
            .ok_or_else(|| HostError::new("runtime storage byte range overflows usize"))?
            .min(content.len());
        self.rangeBytes.fetch_add(end - start, Ordering::SeqCst);
        Ok(content[start..end].to_vec())
    }

    /// Atomically replaces one in-memory file.

    fn writeBytesAtomically(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
        match self.failAtomic.load(Ordering::SeqCst) {
            1 if path.ends_with("receive_state.json") => return Err(HostError::new("receive ledger commit failed")),
            2 if path.contains("/pending/") => return Err(HostError::new("pending record commit failed")),
            3 if path.ends_with("clocks.json") => return Err(HostError::new("clock commit failed")),
            _ => {},
        }
        self.writeBytes(path, content)
    }

    fn writeBytes(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
        self.writeCount.fetch_add(1, Ordering::SeqCst);
        let mut files = self
            .files
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        files.insert(path.to_string(), content.to_vec());
        Ok(())
    }

    /// Appends bytes to one in-memory sync-operation storage entry.
    fn appendBytes(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
        self.writeCount.fetch_add(1, Ordering::SeqCst);
        self.files
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?
            .entry(path.to_string())
            .or_default()
            .extend_from_slice(content);
        Ok(())
    }

    fn delete(&self, path: &str, _recursive: bool) -> operit_host_api::HostResult<()> {
        let mut files = self
            .files
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        files.remove(path);
        Ok(())
    }

    fn exists(&self, path: &str) -> operit_host_api::HostResult<bool> {
        let files = self
            .files
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(files.contains_key(path))
    }

    fn list(&self, prefix: &str) -> operit_host_api::HostResult<Vec<RuntimeStorageEntry>> {
        let files = self
            .files
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(files
            .iter()
            .filter(|(path, _)| path.starts_with(prefix))
            .map(|(path, content)| RuntimeStorageEntry {
                path: path.clone(),
                isDirectory: false,
                size: content.len() as i64,
            })
            .collect())
    }
}

/// Builds one sync operation fixture with explicit compaction semantics.
fn operation(
    sequence: i64,
    entityType: &str,
    entityId: &str,
    operationName: &str,
    semantics: SyncOperationSemantics,
    payload: Value,
) -> SyncOperation {
    SyncOperation {
        opId: format!("device-a:{sequence}"),
        originDeviceId: "device-a".to_string(),
        sequence,
        domain: "chat".to_string(),
        entityType: entityType.to_string(),
        entityId: entityId.to_string(),
        operation: operationName.to_string(),
        semantics,
        payload,
        createdAt: sequence,
        schemaVersion: 1,
    }
}

/// Builds one replaceable entity-state fixture from a specific origin.
fn operationFromOrigin(
    originDeviceId: &str,
    sequence: i64,
    entityType: &str,
    entityId: &str,
    operationName: &str,
    payload: Value,
) -> SyncOperation {
    SyncOperation {
        opId: format!("{originDeviceId}:{sequence}"),
        originDeviceId: originDeviceId.to_string(),
        sequence,
        domain: "chat".to_string(),
        entityType: entityType.to_string(),
        entityId: entityId.to_string(),
        operation: operationName.to_string(),
        semantics: SyncOperationSemantics::EntityState,
        payload,
        createdAt: sequence,
        schemaVersion: 1,
    }
}

fn sequences(operations: &[SyncOperation]) -> Vec<i64> {
    operations
        .iter()
        .map(|operation| operation.sequence)
        .collect()
}

/// Policy reads must leave unrelated durable preference outboxes untouched.
#[test]
fn domain_filtered_reads_do_not_publish_unrelated_preference_outboxes() {
    use crate::PreferencesDataStore::{stringPreferencesKey, PreferencesDataStore};
    use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;

    for budgeted in [false, true] {
        for exportDomains in [vec!["preferences".to_string()], Vec::new()] {
            let storage = Arc::new(MemoryStorageHost::default());
            let store = SyncOperationStore::new(storage.clone(), RUNTIME_SYNC_DIR_PATH);
            let path = "runtime/config/domain-read.preferences.json";
            let preferences = PreferencesDataStore::newEncryptedWithStorage(storage.clone(), path);
            preferences.edit(|values| values.set(&stringPreferencesKey("enabled"), true)).unwrap();
            let mut policy = operationFromOrigin("peer", 1, "command", "policy", "apply", json!({}));
            policy.domain = "network_control".into();
            store.appendOperation(&policy).unwrap();
            let domains = vec!["network_control".to_string()];
            let before = storage.readBytes(path).unwrap();
            let read = |domains: &[String]| {
                if budgeted {
                    store.operationsSinceWithBudget(&SyncClock::empty(), domains, usize::MAX, usize::MAX, |_| true)
                } else {
                    store.operationsSince(&SyncClock::empty(), domains, usize::MAX)
                }.unwrap()
            };
            assert_eq!(read(&domains), vec![policy.clone()]);
            assert!(storage.readBytes(path).unwrap() == before,
                "policy queries must not drain or rewrite an unrelated encrypted outbox");

            // Exports which request preferences must still publish the pending edit.
            let exported = read(&exportDomains);
            assert!(exported.iter().any(|operation| operation.domain == "preferences"));
            assert_eq!(storage.readBytes(path).unwrap(), before);
            assert_eq!(read(&domains), vec![policy.clone()]);
            let reads = storage.readCount.load(Ordering::SeqCst);
            for _ in 0..5 { assert_eq!(read(&domains), vec![policy.clone()]); }
            // SQL planning does not reread business files or the retired JSONL carrier.
            assert_eq!(storage.readCount.load(Ordering::SeqCst) - reads,
                0,
                "warm policy queries must not load an unrelated encryption key");
        }
    }
}

/// Ensures domain reads avoid unrelated encrypted payloads and observe every append path.
#[test]
fn domain_reads_reuse_logs_and_observe_local_remote_and_unobserved_appends() {
    let storage = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(storage.clone(), "domain-read-test");
    let mut unrelated = operationFromOrigin("peer", 1, "preference", "secret", "upsert", json!({
        "format": ENCRYPTED_SYNC_PAYLOAD_FORMAT,
        "envelope": {"invalid": true}
    }));
    unrelated.domain = "unrelated".into();
    store.appendOperation(&unrelated).unwrap();
    let domains = vec!["chat".to_string()];
    assert!(store.operationsSince(&SyncClock::empty(), &domains, usize::MAX).unwrap().is_empty());
    let reads = storage.readCount.load(Ordering::SeqCst);
    for _ in 0..20 {
        assert!(store.operationsSince(&SyncClock::empty(), &domains, usize::MAX).unwrap().is_empty());
    }
    assert_eq!(storage.readCount.load(Ordering::SeqCst), reads);
    let remote = operationFromOrigin("peer", 2, "message", "remote", "upsert", json!({}));
    store.appendOperation(&remote).unwrap();
    let local = store.appendLocalOperation("peer", NewSyncOperation {
        domain: "chat".into(), entityType: "message".into(), entityId: "local".into(),
        operation: "upsert".into(), semantics: SyncOperationSemantics::EntityState, payload: json!({}),
    }).unwrap();
    let unobserved = operationFromOrigin("peer", 7, "message", "unobserved", "upsert", json!({}));
    store.appendUnobservedOperation(&unobserved).unwrap();
    let shared = SyncOperationStore::new(storage.clone(), "domain-read-test");
    let result = shared.operationsSince(&SyncClock::empty(), &domains, usize::MAX).unwrap();
    assert_eq!(result.len(), 3);
    for expected in [&remote, &local, &unobserved] {
        assert!(result.iter().any(|operation| operation == expected));
    }
    let mut clock = SyncClock::empty();
    clock.setSequence("peer", local.sequence);
    assert_eq!(shared.operationsSince(&clock, &domains, usize::MAX).unwrap(), vec![unobserved]);
    shared.setExportFloor("peer", 7).unwrap();
    assert!(shared.operationsSince(&SyncClock::empty(), &domains, usize::MAX).unwrap().is_empty());
}

/// Acknowledging a control operation must not hide business operations that
/// share the same origin and straddle its sequence.
#[test]
fn network_control_acknowledgement_preserves_business_sequences() {
    use crate::NetworkControlStore::NETWORK_CONTROL_SYNC_DOMAIN;
    let storage = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(storage, "control-clock-test");
    let domains = vec![
        "chat".to_string(),
        NETWORK_CONTROL_SYNC_DOMAIN.to_string(),
    ];
    let earlier = operationFromOrigin("peer", 5, "message", "earlier", "upsert", json!({}));
    let mut control = operationFromOrigin("peer", 6, "command", "policy", "apply", json!({}));
    control.domain = NETWORK_CONTROL_SYNC_DOMAIN.to_string();
    let later = operationFromOrigin("peer", 7, "message", "later", "upsert", json!({}));
    for operation in [&earlier, &control, &later] {
        store.appendUnobservedOperation(operation).unwrap();
    }
    store.observeNetworkControlOperation(&control).unwrap();
    // The acknowledged control page no longer exports, while both business
    // operations on either side of its sequence still do.
    let clock = store.localClock().unwrap();
    let pending = store
        .operationsSince(&clock, &domains, usize::MAX)
        .unwrap();
    assert_eq!(sequences(&pending), vec![5, 7]);
    store.observeOperation(&earlier).unwrap();
    let clock = store.localClock().unwrap();
    let pending = store
        .operationsSince(&clock, &domains, usize::MAX)
        .unwrap();
    assert_eq!(pending, vec![later.clone()]);
    // Replaying the same control page stays idempotent.
    store.observeNetworkControlOperation(&control).unwrap();
    let clock = store.localClock().unwrap();
    assert_eq!(
        store
            .operationsSince(&clock, &domains, usize::MAX)
            .unwrap(),
        vec![later]
    );
    // The acknowledgement is restricted to the control domain.
    assert!(store.observeNetworkControlOperation(&earlier).is_err());
}

/// Verifies cached policy reads immediately apply revocation and match a fresh replay.
#[test]
fn policy_cache_observes_remote_revocation_and_matches_fresh_replay() {
    use crate::NetworkControlStore::{NetworkControlIdentityAssignment, NetworkControlStore};
    let storage = Arc::new(MemoryStorageHost::default());
    let reader = NetworkControlStore::new(storage.clone()).unwrap();
    reader.bootstrapCurrentSpace().unwrap();
    let writer = NetworkControlStore::new(storage.clone()).unwrap();
    writer.admitMember("peer".into()).unwrap();
    writer.setIdentity(NetworkControlIdentityAssignment {
        nodeId: "peer".into(), roleId: "user".into(),
    }).unwrap();
    assert!(reader.nodeHasCapability("peer", "runtime.execute", None).unwrap());
    assert!(!reader.nodeIsDisconnected("peer").unwrap());
    writer.clearIdentity("peer".into()).unwrap();
    // Clearing resets to the default user identity: a member never becomes
    // identity-less, so basic capabilities survive the reset.
    assert_eq!(
        reader
            .currentState()
            .unwrap()
            .deviceIdentityIds
            .get("peer")
            .map(String::as_str),
        Some("user")
    );
    assert!(reader.nodeHasCapability("peer", "runtime.execute", None).unwrap());
    writer.disconnectNode("peer".into()).unwrap();
    assert!(reader.nodeIsDisconnected("peer").unwrap());
    assert_eq!(reader.currentState().unwrap(), writer.currentState().unwrap());
    let fresh = NetworkControlStore::new(storage).unwrap();
    assert_eq!(reader.currentState().unwrap(), fresh.currentState().unwrap());
}

/// Verifies clearing a restricted identity resets it to the default user
/// role, restoring basic capabilities instead of leaving the member
/// identity-less.
#[test]
fn clearing_identity_resets_to_default_user() {
    use crate::NetworkControlStore::{NetworkControlIdentityAssignment, NetworkControlStore};
    let storage = Arc::new(MemoryStorageHost::default());
    let store = NetworkControlStore::new(storage.clone()).unwrap();
    store.bootstrapCurrentSpace().unwrap();
    store.admitMember("peer".into()).unwrap();
    // The relay role has no runtime.execute, so the reset below is observable.
    store.setIdentity(NetworkControlIdentityAssignment {
        nodeId: "peer".into(), roleId: "relay".into(),
    }).unwrap();
    assert!(!store.nodeHasCapability("peer", "runtime.execute", None).unwrap());
    store.clearIdentity("peer".into()).unwrap();
    let state = store.currentState().unwrap();
    assert_eq!(
        state.deviceIdentityIds.get("peer").map(String::as_str),
        Some("user")
    );
    assert!(state.memberNodeIds.contains("peer"));
    assert!(store.nodeHasCapability("peer", "runtime.execute", None).unwrap());
    // Replaying the same log into a fresh instance converges identically.
    let replay = NetworkControlStore::new(storage).unwrap();
    assert_eq!(replay.currentState().unwrap(), state);
}

/// One generic admission persists mixed identities and remains stable across review retries and replay.
#[test]
fn space_admission_preserves_identity_map_through_retry_and_replay() {
    use crate::NetworkControlStore::{NetworkControlCommand, NetworkControlCommandRecord, NetworkControlRole, NetworkControlStore};
    for constrained in [false, true] {
    let storage = Arc::new(MemoryStorageHost { releasePreferencesCache: constrained, ..Default::default() });
    let store = if constrained { NetworkControlStore::newNodeLocal(storage.clone()).unwrap() }
        else { NetworkControlStore::new(storage.clone()).unwrap() };
    let bootstrap = store.bootstrapCurrentSpace().unwrap();
    let admin = bootstrap.memberNodeIds.iter().next().unwrap().clone();
    store.defineRole(NetworkControlRole {
        roleId: "display".into(), displayName: "Display".into(),
        capabilities: BTreeSet::from(["network.devices.view".into()]),
    }).unwrap();
    let identities = BTreeMap::from([
        ("phone".into(), "user".into()),
        ("esp".into(), "edge".into()),
        ("bridge".into(), "relay".into()),
        ("screen".into(), "display".into()),
        // Overlap with the destination must preserve its existing administrator.
        (admin.clone(), "user".into()),
    ]);
    let operation = store.admitSpaceForReview("source-space".into(), identities.clone(), "mixed").unwrap();
    let record: NetworkControlCommandRecord = serde_json::from_value(operation.payload.clone()).unwrap();
    assert_eq!(record.command, NetworkControlCommand::AdmitSpace {
        sourceSpaceId: "source-space".into(), identities: identities.clone(),
    });
    let encoded = &operation.payload["command"]["admit_space"];
    assert_eq!(encoded["identities"], serde_json::to_value(&identities).unwrap());
    assert!(encoded.get("nodeIds").is_none());
    assert!(encoded.get("edgeNodeIds").is_none());
    let operations = store.currentSpaceOperations().unwrap();
    assert_eq!(store.admitSpaceForReview("source-space".into(), identities.clone(), "mixed").unwrap(), operation);
    let mut changed = identities.clone();
    changed.insert("esp".into(), "relay".into());
    assert!(store.admitSpaceForReview("source-space".into(), changed, "mixed").is_err());
    assert_eq!(store.currentSpaceOperations().unwrap(), operations);
    let state = store.currentState().unwrap();
    for (node, identity) in &identities {
        assert!(state.memberNodeIds.contains(node));
        assert_eq!(state.deviceIdentityIds[node], if node == &admin { "admin" } else { identity });
    }
    assert_eq!(store.validateSpacePolicy(&state.spaceId, &operations).unwrap(), state);
    store.validateSpaceAdmission(&state.spaceId, "source-space", &identities.keys().cloned().collect(), &operations).unwrap();
    let reopened = if constrained { NetworkControlStore::newNodeLocal(storage).unwrap() }
        else { NetworkControlStore::new(storage).unwrap() };
    assert_eq!(reopened.currentState().unwrap(), state);
    }
}

/// Upgrade an existing join-only reviewer's log, then persist a new admission and reopen both.
#[test]
fn legacy_space_admission_survives_upgrade_without_rewriting_history() {
    for constrained in [false, true] {
    use crate::NetworkControlStore::{NetworkControlCommandRecord, NetworkControlIdentityAssignment, NetworkControlRole, NetworkControlStore};
    let storage = Arc::new(MemoryStorageHost { releasePreferencesCache: constrained, ..Default::default() });
    let store = NetworkControlStore::new(storage.clone()).unwrap();
    let space = store.bootstrapCurrentSpace().unwrap();
    store.admitMember("zz-reviewer".into()).unwrap();
    store.defineRole(NetworkControlRole {
        roleId: "reviewer".into(), displayName: "Reviewer".into(),
        capabilities: BTreeSet::from(["network.members.join".into(), "network.approval".into()]),
    }).unwrap();
    let mut legacy = store.setIdentity(NetworkControlIdentityAssignment {
        nodeId: "zz-reviewer".into(), roleId: "reviewer".into(),
    }).unwrap();
    legacy.opId = "zz-reviewer:1".into();
    legacy.originDeviceId = "zz-reviewer".into();
    legacy.sequence = 1;
    legacy.entityId = "old-admission".into();
    legacy.payload = json!({
        "commandId": legacy.entityId, "spaceId": space.spaceId, "issuerNodeId": "zz-reviewer",
        "command": {"admit_space": {"sourceSpaceId": "old-space", "nodeIds": ["old-phone"]}},
    });
    // Exercise the peer projection boundary with the reviewer's original join-only authority.
    store.projectPeerControlOperations("zz-reviewer", &[legacy.clone()]).unwrap();
    drop(store);
    // Persist the exact pre-upgrade wire record, including its immutable operation ID and payload.
    let journal = SyncOperationStore::new(storage.clone(), operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH);
    journal.appendUnobservedOperation(&legacy).unwrap();
    let upgraded = NetworkControlStore::new(storage.clone()).unwrap();
    assert_eq!(upgraded.currentState().unwrap().deviceIdentityIds["old-phone"], "user");
    let operations = upgraded.currentSpaceOperations().unwrap();
    upgraded.validateSpaceAdmission(&space.spaceId, "old-space", &BTreeSet::from(["old-phone".into()]), &operations).unwrap();
    // The same reviewer must not bypass the new identity-assignment checks by using the new format.
    let mut unauthorized = legacy.clone();
    unauthorized.opId = "zz-reviewer:2".into();
    unauthorized.sequence = 2;
    unauthorized.entityId = "new-unauthorized".into();
    unauthorized.payload["commandId"] = json!(unauthorized.entityId);
    unauthorized.payload["command"]["admit_space"] = json!({
        "sourceSpaceId": "new-space", "identities": {"unauthorized-phone": "user"},
    });
    let mut policy = operations.clone();
    policy.push(unauthorized);
    assert!(!upgraded.validateSpacePolicy(&space.spaceId, &policy).unwrap().memberNodeIds.contains("unauthorized-phone"));
    let new = upgraded.admitSpace("new-space".into(), BTreeMap::from([("new-esp".into(), "edge".into())])).unwrap();
    assert!(new.payload["command"]["admit_space"].get("nodeIds").is_none());
    let state = upgraded.currentState().unwrap();
    assert_eq!(state.deviceIdentityIds["new-esp"], "edge");
    drop(upgraded);
    let reopened = NetworkControlStore::new(storage).unwrap();
    assert_eq!(reopened.currentState().unwrap(), state);
    let operations = reopened.currentSpaceOperations().unwrap();
    assert_eq!(operations.iter().find(|op| op.opId == legacy.opId), Some(&legacy));
    // Mixed or malformed formats must not activate the legacy authorization path.
    for malformed in [
        json!({"sourceSpaceId":"old-space", "nodeIds":["old-phone"], "identities":{"old-phone":"admin"}}),
        json!({"sourceSpaceId":"old-space", "nodeIds":{"old-phone":"admin"}}),
        json!({"sourceSpaceId":"old-space", "identities":["old-phone"]}),
    ] {
        let mut record = legacy.payload.clone();
        record["command"]["admit_space"] = malformed;
        assert!(serde_json::from_value::<NetworkControlCommandRecord>(record).is_err());
    }
    }
}

/// Invalid assignments must not persist an operation or partially admit a valid prefix.
#[test]
fn space_admission_rejects_invalid_identity_maps_without_writes() {
    use crate::NetworkControlStore::NetworkControlStore;
    let storage = Arc::new(MemoryStorageHost::default());
    let store = NetworkControlStore::new(storage.clone()).unwrap();
    store.bootstrapCurrentSpace().unwrap();
    let before = store.currentState().unwrap();
    let operations = store.currentSpaceOperations().unwrap();
    let writes = storage.writeCount();
    for identity in ["unknown", ""] {
        assert!(store.admitSpace("source-space".into(), BTreeMap::from([
            ("a-valid".into(), "edge".into()), ("z-invalid".into(), identity.into()),
        ])).is_err());
    }
    assert!(store.admitSpace("source-space".into(), BTreeMap::new()).is_err());
    assert!(store.admitSpace("source-space".into(), BTreeMap::from([
        ("".into(), "edge".into()),
    ])).is_err());
    assert_eq!(store.currentState().unwrap(), before);
    assert_eq!(store.currentSpaceOperations().unwrap(), operations);
    assert_eq!(storage.writeCount(), writes);
}

/// A reviewer cannot use the admission map to grant capabilities it does not hold.
#[test]
fn space_admission_requires_identity_assignment_authority() {
    use crate::NetworkControlStore::{NetworkControlIdentityAssignment, NetworkControlRole, NetworkControlStore};
    let storage = Arc::new(MemoryStorageHost::default());
    let store = NetworkControlStore::new(storage.clone()).unwrap();
    let bootstrap = store.bootstrapCurrentSpace().unwrap();
    let local = bootstrap.memberNodeIds.iter().next().unwrap().clone();
    store.admitMember("backup-admin".into()).unwrap();
    store.setIdentity(NetworkControlIdentityAssignment { nodeId: "backup-admin".into(), roleId: "admin".into() }).unwrap();
    store.defineRole(NetworkControlRole {
        roleId: "reviewer".into(), displayName: "Reviewer".into(),
        capabilities: BTreeSet::from([
            "network.members.join".into(), "network.identity.assign".into(),
            "network.devices.view".into(), "network.approval".into(),
        ]),
    }).unwrap();
    store.setIdentity(NetworkControlIdentityAssignment { nodeId: local, roleId: "reviewer".into() }).unwrap();
    let before = store.currentState().unwrap();
    let operations = store.currentSpaceOperations().unwrap();
    let writes = storage.writeCount();
    assert!(store.admitSpace("source-space".into(), BTreeMap::from([
        ("new-admin".into(), "admin".into()),
    ])).unwrap_err().contains("lacks capability"));
    assert_eq!(store.currentState().unwrap(), before);
    assert_eq!(store.currentSpaceOperations().unwrap(), operations);
    assert_eq!(storage.writeCount(), writes);
}

/// Built-in identities describe node capabilities; removed roles cannot be assigned.
#[test]
fn builtin_identities_expose_only_current_node_capabilities() {
    use crate::NetworkControlStore::{NetworkControlIdentityAssignment, NetworkControlStore};
    let store = NetworkControlStore::new(Arc::new(MemoryStorageHost::default())).unwrap();
    store.bootstrapCurrentSpace().unwrap();
    store.admitMember("peer".into()).unwrap();
    let state = store.currentState().unwrap();
    assert_eq!(state.roles.keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["admin", "edge", "relay", "user"]);
    for (identity, relay, execute, storage) in [("relay", true, false, true), ("edge", false, true, false), ("user", true, true, true)] {
        store.setIdentity(NetworkControlIdentityAssignment { nodeId: "peer".into(), roleId: identity.into() }).unwrap();
        assert!(store.nodeHasCapability("peer", "network.devices.view", None).unwrap());
        assert_eq!(store.nodeHasCapability("peer", "network.relay", None).unwrap(), relay);
        assert_eq!(store.nodeHasCapability("peer", "runtime.execute", None).unwrap(), execute);
        assert_eq!(store.nodeHasCapability("peer", "storage.provide", None).unwrap(), storage);
        assert!(!store.nodeHasCapability("peer", "network.audit.read", None).unwrap());
    }
    for identity in ["auditor", "runner", "storage"] {
        assert!(store.setIdentity(NetworkControlIdentityAssignment { nodeId: "peer".into(), roleId: identity.into() }).is_err());
    }
}

/// Measures warm policy queries without rereading or decoding unrelated chat history.
#[test]
fn repeated_domain_queries_do_not_rescan_large_history() {
    let storage = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(storage.clone(), "large-history");
    let history = (1..=5000)
        .map(|sequence| {
            operationFromOrigin(
                "peer",
                sequence,
                "message",
                &sequence.to_string(),
                "upsert",
                json!({"content": "x".repeat(512)}),
            )
        })
        .collect::<Vec<_>>();
    store.appendOperations(&history).unwrap();
    let mut policy = operationFromOrigin("peer", 5001, "command", "policy", "apply", json!({}));
    policy.domain = "network_control".into();
    store.appendOperation(&policy).unwrap();
    let domains = vec!["network_control".to_string()];
    assert_eq!(store.operationsSince(&SyncClock::empty(), &domains, usize::MAX).unwrap(), vec![policy.clone()]);
    let reads = storage.readCount.load(Ordering::SeqCst);
    let bytes = storage.rangeBytes.load(Ordering::SeqCst);
    for _ in 0..100 {
        assert_eq!(store.operationsSince(&SyncClock::empty(), &domains, usize::MAX).unwrap(), vec![policy.clone()]);
    }
    assert_eq!(storage.readCount.load(Ordering::SeqCst), reads);
    assert_eq!(storage.rangeBytes.load(Ordering::SeqCst), bytes);

}

/// Paging reads each retained payload once instead of rescanning the remaining business history.
#[test]
fn filtered_pages_read_payload_bytes_linearly() {
    for (domain, semantics, encrypted) in [
        ("preferences", SyncOperationSemantics::EntityState, false),
        ("preferences", SyncOperationSemantics::EntityState, true),
        ("chat", SyncOperationSemantics::EntityState, false),
        ("plugin_storage", SyncOperationSemantics::Transaction, false),
    ] {
        let host = Arc::new(MemoryStorageHost::default());
        let store = SyncOperationStore::new(host.clone(), "linear-export");
        let operations = (1..=512).map(|sequence| {
            let mut entry = operation(sequence, "entry", &format!("entry-{sequence}"), "set",
                semantics, json!({"storagePath":"runtime/data/probe.json","content":"x".repeat(4096),"encrypted":encrypted}));
            entry.domain = domain.into();
            if encrypted {
                entry.payload["contractVersion"] = json!(1);
                entry.payload["logicalTime"] = json!(sequence);
                entry.createdAt = 10_000 - sequence;
            }
            entry
        }).collect::<Vec<_>>();
        store.appendOperations(&operations).unwrap();
        drop(store);
        let store = SyncOperationStore::new(host.clone(), "linear-export");
        store.receipt().unwrap();
        let bytes = host.rangeBytes.load(Ordering::SeqCst);
        let mut receipt = SyncReceipt::default();
        for (index, expected) in operations.chunks(16).enumerate() {
            let mut inventory = BTreeSet::new();
            let page = store.operationsSinceWithInventory(&receipt, &[domain.into()], 16, usize::MAX,
                (index == 0).then_some(&mut inventory), |entry| entry.payload["content"].is_string()).unwrap();
            if index == 0 { assert_eq!(inventory.len(), operations.len()); }
            assert_eq!(page, expected);
            for entry in page { receipt.observe(&entry.originDeviceId, entry.sequence); }
        }
        let readBytes = host.rangeBytes.load(Ordering::SeqCst) - bytes;
        let journalBytes = host.files.lock().unwrap().iter().filter(|(path, _)| path.ends_with(".jsonl"))
            .map(|(_, bytes)| bytes.len()).sum::<usize>();
        // The first inventory scans once; encrypted headers resolve once after reopening.
        let allowance = if encrypted { 3 } else { 2 };
        assert!(readBytes <= journalBytes * allowance,
            "{domain} encrypted={encrypted}: read {readBytes} bytes for a {journalBytes}-byte journal");
        eprintln!("{domain} encrypted={encrypted}: payload reads={readBytes} journal={journalBytes}");
    }
}

/// Resolves filtered revisions only for entities reached by the current page.
#[test]
fn filtered_compaction_does_not_open_later_entities() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(host.clone(), "lazy-compaction");
    let operations = (1..=192).map(|sequence| operation(sequence, "message",
        &format!("message-{}", (sequence - 1) / 3), "upsert", SyncOperationSemantics::EntityState,
        json!({"export":sequence % 3 != 0,"content":"x".repeat(4096)}))).collect::<Vec<_>>();
    store.appendOperations(&operations).unwrap();
    let bytes = host.rangeBytes.load(Ordering::SeqCst);
    let page = store.operationsSinceWithBudget(&SyncClock::empty(), &[], 8, usize::MAX,
        |entry| entry.payload["export"] == true).unwrap();
    assert_eq!(sequences(&page), vec![2, 5, 8, 11, 14, 17, 20, 23]);
    let readBytes = host.rangeBytes.load(Ordering::SeqCst) - bytes;
    assert!(readBytes < 32 * 5000, "small page opened later entity payloads: {readBytes} bytes");
    let mut inventory = BTreeSet::new();
    assert_eq!(store.operationsSinceWithInventory(&SyncClock::empty(), &[], 8, usize::MAX,
        Some(&mut inventory), |entry| entry.payload["export"] == true).unwrap(), page);
    assert_eq!(inventory.len(), 64);
}

/// Verifies a synchronization clock includes every required origin sequence.
#[test]
fn sync_clock_requires_every_origin_sequence() {
    let mut current = SyncClock::empty();
    current.setSequence("node-a", 8);
    current.setSequence("node-b", 3);

    let mut included = SyncClock::empty();
    included.setSequence("node-a", 8);
    included.setSequence("node-b", 2);
    assert!(current.includes(&included));

    let mut missing = included.clone();
    missing.setSequence("node-c", 1);
    assert!(!current.includes(&missing));

    let mut ahead = included;
    ahead.setSequence("node-b", 4);
    assert!(!current.includes(&ahead));
}

#[test]
fn compact_keeps_latest_upsert_for_each_entity() {
    let compacted = compactSyncOperations(vec![
        operation(
            1,
            "message",
            "chat-1:1",
            "upsert",
            SyncOperationSemantics::EntityState,
            json!({"content": "a"}),
        ),
        operation(
            2,
            "message",
            "chat-1:1",
            "upsert",
            SyncOperationSemantics::EntityState,
            json!({"content": "ab"}),
        ),
        operation(
            3,
            "message",
            "chat-1:2",
            "upsert",
            SyncOperationSemantics::EntityState,
            json!({"content": "other"}),
        ),
        operation(
            4,
            "chat",
            "chat-1",
            "upsert",
            SyncOperationSemantics::EntityState,
            json!({"title": "New Chat"}),
        ),
    ]);

    assert_eq!(sequences(&compacted), vec![2, 3, 4]);
    assert_eq!(compacted[0].payload["content"], "ab");
}

#[test]
fn compact_keeps_delete_transactions_between_upserts() {
    let compacted = compactSyncOperations(vec![
        operation(
            1,
            "message",
            "chat-1:1",
            "upsert",
            SyncOperationSemantics::EntityState,
            json!({"content": "old"}),
        ),
        operation(
            2,
            "message",
            "chat-1:1",
            "delete",
            SyncOperationSemantics::Transaction,
            json!({"deleted": true}),
        ),
        operation(
            3,
            "message",
            "chat-1:1",
            "upsert",
            SyncOperationSemantics::EntityState,
            json!({"content": "new"}),
        ),
    ]);

    assert_eq!(sequences(&compacted), vec![2, 3]);
    assert_eq!(compacted[0].operation, "delete");
    assert_eq!(compacted[1].payload["content"], "new");
}

#[test]
fn append_and_export_compact_repeated_stream_snapshots() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(host.clone(), "sync-test");
    let operations = vec![
        operation(
            1,
            "message",
            "chat-1:1",
            "upsert",
            SyncOperationSemantics::EntityState,
            json!({"content": "h"}),
        ),
        operation(
            2,
            "message",
            "chat-1:1",
            "upsert",
            SyncOperationSemantics::EntityState,
            json!({"content": "he"}),
        ),
        operation(
            3,
            "message",
            "chat-1:1",
            "upsert",
            SyncOperationSemantics::EntityState,
            json!({"content": "hello"}),
        ),
    ];
    store.appendOperations(&operations).unwrap();
    let mut sql = host.openTransactionDatabase("sync-test/transactions.sqlite").unwrap();
    assert_eq!(sql.query("SELECT COUNT(*) FROM originals",vec![]).unwrap()[0].valueAt(0).unwrap().asI64().unwrap(),3);

    let operations = store
        .operationsSince(&SyncClock::empty(), &["chat".to_string()], 100)
        .unwrap();

    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].sequence, 3);
    assert_eq!(operations[0].payload["content"], "hello");
}

#[test]
fn stress_compacts_many_stream_snapshots_to_one_exported_upsert() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(host, "sync-stress");

    for sequence in 1..=2_000 {
        store
            .appendOperation(&operation(
                sequence,
                "message",
                "chat-1:1",
                "upsert",
                SyncOperationSemantics::EntityState,
                json!({"content": format!("token-{sequence}")}),
            ))
            .unwrap();
    }

    let operations = store
        .operationsSince(&SyncClock::empty(), &["chat".to_string()], 100)
        .unwrap();

    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].sequence, 2_000);
    assert_eq!(operations[0].payload["content"], "token-2000");
}

#[test]
fn stress_compacts_many_entities_without_cross_entity_loss() {
    let mut operations = Vec::new();
    let mut sequence = 1;
    for deviceIndex in 0..4 {
        let deviceId = format!("device-{deviceIndex}");
        for round in 0..80 {
            for entityIndex in 0..30 {
                let entityId = format!("chat-{deviceIndex}:{entityIndex}");
                operations.push(operationFromOrigin(
                    &deviceId,
                    sequence,
                    "message",
                    &entityId,
                    "upsert",
                    json!({"round": round, "entity": entityId}),
                ));
                sequence += 1;
            }
        }
        operations.push(operationFromOrigin(
            &deviceId,
            sequence,
            "message",
            &format!("chat-{deviceIndex}:deleted"),
            "delete",
            json!({"deleted": true}),
        ));
        sequence += 1;
    }

    let compacted = compactSyncOperations(operations);
    let expectedUpserts = 4 * 30;
    let expectedDeletes = 4;

    assert_eq!(compacted.len(), expectedUpserts + expectedDeletes);
    assert_eq!(
        compacted
            .iter()
            .filter(|operation| operation.operation == "delete")
            .count(),
        expectedDeletes
    );
    assert!(compacted
        .iter()
        .filter(|operation| operation.operation == "upsert")
        .all(|operation| operation.payload["round"] == 79));
}

/// Verifies an entity-state tombstone replaces older state for the same entity.
#[test]
fn compact_keeps_only_latest_replaceable_entity_state() {
    let compacted = compactSyncOperations(vec![
        operation(
            1,
            "preference",
            "settings:theme",
            "set",
            SyncOperationSemantics::EntityState,
            json!({"value": "light"}),
        ),
        operation(
            2,
            "preference",
            "settings:theme",
            "delete",
            SyncOperationSemantics::EntityState,
            Value::Null,
        ),
    ]);

    assert_eq!(sequences(&compacted), vec![2]);
    assert_eq!(compacted[0].operation, "delete");
}

/// Verifies entity conflict order remains deterministic across store instances.
#[test]
fn entity_conflict_order_is_persisted_and_total() {
    let backend = MemoryStorageHost::default();
    let host = Arc::new(backend.clone());
    let store = SyncOperationStore::new(host.clone(), "sync-order");
    let older = SyncOperation {
        opId: "device-z:8".to_string(),
        originDeviceId: "device-z".to_string(),
        sequence: 8,
        domain: "preferences".to_string(),
        entityType: "settings".to_string(),
        entityId: format!("{CONFIG_PREFERENCES_DIR_PATH}/settings.json"),
        operation: "upsert".to_string(),
        semantics: SyncOperationSemantics::EntityState,
        payload: json!({"value": "older"}),
        createdAt: 100,
        schemaVersion: 1,
    };
    let newer = SyncOperation {
        opId: "device-a:1".to_string(),
        originDeviceId: "device-a".to_string(),
        sequence: 1,
        domain: older.domain.clone(),
        entityType: older.entityType.clone(),
        entityId: older.entityId.clone(),
        operation: "upsert".to_string(),
        semantics: SyncOperationSemantics::EntityState,
        payload: json!({"value": "newer"}),
        createdAt: 101,
        schemaVersion: 1,
    };

    assert!(store.shouldApplyOperation(&newer).unwrap());
    store.recordAppliedOperation(&newer).unwrap();
    assert!(!store.shouldApplyOperation(&older).unwrap());

    let reopened = SyncOperationStore::new(Arc::new(backend), "sync-order");
    assert!(!reopened.shouldApplyOperation(&older).unwrap());
    assert!(!reopened.shouldApplyOperation(&newer).unwrap());
}

/// Verifies a local operation immediately becomes the applied entity version.
#[test]
fn local_operation_records_its_entity_version() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(host, "sync-local-version");
    let operation = store
        .appendLocalOperation(
            "device-local",
            NewSyncOperation {
                domain: "runtime_file".to_string(),
                entityType: "file".to_string(),
                entityId: RUNTIME_WEBSESSION_BROWSER_BOOKMARKS_PATH.to_string(),
                operation: "upsert".to_string(),
                semantics: SyncOperationSemantics::EntityState,
                payload: json!({"contentBase64": "e30="}),
            },
        )
        .unwrap();

    assert!(!store.shouldApplyOperation(&operation).unwrap());
}

/// Verifies pre-join local operations are excluded from later Space exports.
#[test]
fn pre_join_operations_are_excluded_from_space_exports() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(host, "sync-pre-join-export");
    let deviceId = store.localDeviceId().unwrap();
    store
        .appendLocalOperation(
            &deviceId,
            NewSyncOperation {
                domain: "preferences".to_string(),
                entityType: "settings".to_string(),
                entityId: "before".to_string(),
                operation: "set".to_string(),
                semantics: SyncOperationSemantics::EntityState,
                payload: json!({"value": "before"}),
            },
        )
        .unwrap();
    store.markLocalOperationsUnexportable().unwrap();
    store
        .appendLocalOperation(
            &deviceId,
            NewSyncOperation {
                domain: "preferences".to_string(),
                entityType: "settings".to_string(),
                entityId: "after".to_string(),
                operation: "set".to_string(),
                semantics: SyncOperationSemantics::EntityState,
                payload: json!({"value": "after"}),
            },
        )
        .unwrap();

    let operations = store.operationsSince(&SyncClock::empty(), &[], 10).unwrap();
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].entityId, "after");
}

/// Verifies concurrent stores allocate unique sequences and recover from a lagging clock.
#[test]
fn concurrent_store_instances_append_unique_local_sequences() {
    const WORKER_COUNT: usize = 8;
    const OPERATIONS_PER_WORKER: usize = 32;

    let backend = MemoryStorageHost::default();
    let host: Arc<dyn RuntimeStorageHost> = Arc::new(backend.clone());
    let barrier = Arc::new(Barrier::new(WORKER_COUNT));
    let mut workers = Vec::new();
    for workerIndex in 0..WORKER_COUNT {
        let host = Arc::clone(&host);
        let barrier = Arc::clone(&barrier);
        workers.push(std::thread::spawn(move || {
            let store = SyncOperationStore::new(host, "sync-concurrent");
            barrier.wait();
            for operationIndex in 0..OPERATIONS_PER_WORKER {
                store
                    .appendLocalOperation(
                        "device-local",
                        NewSyncOperation {
                            domain: "preferences".to_string(),
                            entityType: "concurrency".to_string(),
                            entityId: format!("{workerIndex}:{operationIndex}"),
                            operation: "set".to_string(),
                            semantics: SyncOperationSemantics::Transaction,
                            payload: json!({
                                "workerIndex": workerIndex,
                                "operationIndex": operationIndex,
                            }),
                        },
                    )
                    .unwrap();
            }
        }));
    }
    for worker in workers {
        worker.join().unwrap();
    }

    let store = SyncOperationStore::new(host, "sync-concurrent");
    let expectedCount = WORKER_COUNT * OPERATIONS_PER_WORKER;
    assert_eq!(
        store.localClock().unwrap().sequenceFor("device-local"),
        expectedCount as i64
    );
    let operations = store
        .operationsSince(&SyncClock::empty(), &[], expectedCount + 1)
        .unwrap();
    assert_eq!(operations.len(), expectedCount);
    assert_eq!(
        operations
            .iter()
            .map(|operation| operation.sequence)
            .collect::<BTreeSet<_>>(),
        (1..=expectedCount as i64).collect::<BTreeSet<_>>()
    );

    let mut laggingClock = SyncClock::empty();
    laggingClock.setSequence("device-local", expectedCount as i64 - 1);
    store.writeLocalClock(&laggingClock).unwrap();
    drop(store);

    let reopenedHost: Arc<dyn RuntimeStorageHost> = Arc::new(backend);
    let reopened = SyncOperationStore::new(reopenedHost, "sync-concurrent");
    let recovered = reopened
        .appendLocalOperation(
            "device-local",
            NewSyncOperation {
                domain: "preferences".to_string(),
                entityType: "concurrency".to_string(),
                entityId: "after-restart".to_string(),
                operation: "set".to_string(),
                semantics: SyncOperationSemantics::Transaction,
                payload: json!({"recovered": true}),
            },
        )
        .unwrap();
    assert_eq!(recovered.sequence, expectedCount as i64 + 1);
}

#[test]
#[ignore]
fn stress_ultra_compacts_many_entities_without_cross_entity_loss() {
    let deviceCount = 4;
    let entityCount = 30;
    let updateRounds = 8_000;
    let mut operations = Vec::new();
    let mut sequence = 1;
    for deviceIndex in 0..deviceCount {
        let deviceId = format!("device-{deviceIndex}");
        for round in 0..updateRounds {
            for entityIndex in 0..entityCount {
                let entityId = format!("chat-{deviceIndex}:{entityIndex}");
                operations.push(operationFromOrigin(
                    &deviceId,
                    sequence,
                    "message",
                    &entityId,
                    "upsert",
                    json!({"round": round, "entity": entityId}),
                ));
                sequence += 1;
            }
        }
        operations.push(operationFromOrigin(
            &deviceId,
            sequence,
            "message",
            &format!("chat-{deviceIndex}:deleted"),
            "delete",
            json!({"deleted": true}),
        ));
        sequence += 1;
    }

    let rawCount = operations.len();
    let compacted = compactSyncOperations(operations);
    let expectedUpserts = deviceCount * entityCount;
    let expectedDeletes = deviceCount;

    eprintln!(
        "sync operation ultra stress: raw_operations={rawCount}, compacted_operations={}",
        compacted.len()
    );
    assert_eq!(
        rawCount,
        deviceCount * entityCount * updateRounds + deviceCount
    );
    assert_eq!(compacted.len(), expectedUpserts + expectedDeletes);
    assert_eq!(
        compacted
            .iter()
            .filter(|operation| operation.operation == "delete")
            .count(),
        expectedDeletes
    );
    assert!(compacted
        .iter()
        .filter(|operation| operation.operation == "upsert")
        .all(|operation| operation.payload["round"] == updateRounds - 1));
}

/// Builds canonical structured preference identities, independent of plaintext payloads.
fn preferenceOperation(origin: &str, sequence: i64, path: Value) -> SyncOperation {
    let entityId = if path.as_array().unwrap().is_empty() {
        json!([
            "runtime/config/preferences/model_configs.preferences.json",
            "provider_DEEPSEEK"
        ])
    } else {
        json!([
            "runtime/config/preferences/model_configs.preferences.json",
            "provider_DEEPSEEK",
            path
        ])
    };
    let mut op = operationFromOrigin(
        origin,
        sequence,
        "model_configs",
        &entityId.to_string(),
        "set",
        Value::Null,
    );
    op.domain = "preferences".to_string();
    op
}

/// Keeps earlier entity versions needed to reconstruct durable identity equivalences.
#[test]
fn compact_preferences_preserves_identity_replay_history() {
    let parent =
        json!([{"kind": "field", "value": "models"}, {"kind": "item", "value": "old-model"}]);
    let child = json!([{"kind": "field", "value": "models"}, {"kind": "item", "value": "old-model"}, {"kind": "field", "value": "capabilitiesOverride"}]);
    let sibling = json!([{"kind": "field", "value": "apiKey"}]);
    let compacted = compactSyncOperations(vec![
        preferenceOperation("android", 1, parent.clone()),
        preferenceOperation("android", 2, child.clone()),
        preferenceOperation("android", 3, sibling),
        preferenceOperation("android", 4, parent),
        preferenceOperation("android", 5, child),
    ]);
    assert_eq!(sequences(&compacted), vec![1, 2, 3, 4, 5]);
}

/// Retains independent origin histories and their complete chronological replacements.
#[test]
fn compact_preferences_preserves_other_origins_and_edits_after_entry_replacement() {
    let child = json!([{"kind": "field", "value": "apiKey"}]);
    let compacted = compactSyncOperations(vec![
        preferenceOperation("android", 1, child.clone()),
        preferenceOperation("windows", 2, child.clone()),
        preferenceOperation("android", 3, json!([])),
        preferenceOperation("android", 4, child),
    ]);
    assert_eq!(
        compacted
            .iter()
            .map(|op| (op.originDeviceId.as_str(), op.sequence))
            .collect::<Vec<_>>(),
        vec![
            ("android", 1),
            ("windows", 2),
            ("android", 3),
            ("android", 4)
        ]
    );
}

/// Removes legacy preference history once a later native migration snapshot
/// covers the same keys, keeping paged synchronization applicable to migrated peers.
#[test]
fn compact_preferences_retains_originals_across_storage_migrations() {
    let path = "runtime/config/preferences/user_preferences.preferences.json";
    let mut legacy = preferenceOperation("device-a", 1, json!([]));
    legacy.payload = json!({
        "storagePath": path,
        "key": "theme_mode",
        "value": "dark",
        "encrypted": false,
    });
    let native = SyncOperation {
        opId: "device-a:2".into(),
        originDeviceId: "device-a".into(),
        sequence: 2,
        domain: "preferences".into(),
        entityType: "user_preferences".into(),
        entityId: "migration".into(),
        operation: "transaction".into(),
        semantics: SyncOperationSemantics::Transaction,
        payload: json!({
            "storagePath": path,
            "encrypted": false,
            "formatVersion": 2,
            "schemaVersion": 3,
            "migrationSnapshot": true,
            "entries": [{"key": "theme_mode", "operation": "set", "value": "light"}],
        }),
        createdAt: 2,
        schemaVersion: 3,
    };

    let original = vec![legacy, native];
    assert_eq!(compactSyncOperations(original.clone()), original);
}

/// Does not treat an ordinary native edit as an old migration boundary when
/// another legacy key still needs to be covered by the later full snapshot.
#[test]
fn compaction_does_not_infer_coverage_from_matching_key_sets() {
    let path = "runtime/config/preferences/user_preferences.preferences.json";
    let mut legacyTheme = preferenceOperation("device-a", 1, json!([]));
    legacyTheme.payload = json!({
        "storagePath": path,
        "key": "theme_mode",
        "value": "dark",
        "encrypted": false,
    });
    let mut legacyFont = preferenceOperation("device-a", 2, json!([]));
    legacyFont.payload = json!({
        "storagePath": path,
        "key": "font_type",
        "value": "system",
        "encrypted": false,
    });

    let mut ordinaryNativeEdit = preferenceOperation("device-a", 3, json!([]));
    ordinaryNativeEdit.payload = json!({
        "storagePath": path,
        "key": "theme_mode",
        "value": "light",
        "encrypted": false,
        "formatVersion": 2,
        "schemaVersion": 2,
    });

    let mut migration = preferenceOperation("device-a", 4, json!([]));
    migration.operation = "transaction".into();
    migration.semantics = SyncOperationSemantics::Transaction;
    migration.payload = json!({
        "storagePath": path,
        "encrypted": false,
        "formatVersion": 2,
        "schemaVersion": 2,
        "entries": [
            {"key": "theme_mode", "operation": "set", "value": "light"},
            {"key": "font_type", "operation": "set", "value": "system"},
        ],
    });

    let original = vec![legacyTheme, legacyFont, ordinaryNativeEdit, migration];
    assert_eq!(compactSyncOperations(original.clone()), original);
}


/// Builds a raw journal with one corrupt middle line and verifies reads skip it.
#[test]
fn corrupt_lines_are_skipped_by_reads_and_reported_by_scans() {
    let storage = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(storage.clone(), "corrupt-journal");
    // Legacy files are compatibility input; current writes use SQLite.
    let path = "corrupt-journal/operations/device-a.jsonl";
    let raw = (1..=3).map(|sequence| serde_json::to_string(&operation(sequence, "message", &format!("m{sequence}"), "upsert", SyncOperationSemantics::EntityState, json!({"text":"x"}))).unwrap() + "\n").collect::<String>();
    storage.writeBytes(path, raw.as_bytes()).unwrap();
    storage.writeBytes("corrupt-journal/devices.json", br#"["device-a"]"#).unwrap();
    let original = storage.readBytes(path).unwrap();
    let lines: Vec<&[u8]> = original.split_inclusive(|byte| *byte == b'\n').collect();
    assert_eq!(lines.len(), 3);
    let mut corrupted = Vec::new();
    corrupted.extend_from_slice(lines[0]);
    corrupted.extend_from_slice(b"{ this is not json\n");
    corrupted.extend_from_slice(lines[2]);
    storage.writeBytes(path, &corrupted).unwrap();

    // Close the journal owner before simulating offline file damage. Reopening
    // must rebuild from disk rather than keeping a process-pinned cache alive.
    drop(store);
    let reopened = SyncOperationStore::new(storage.clone(), "corrupt-journal");
    assert!(reopened.operationsSince(&SyncClock::empty(), &[], 100).is_err());

    // The raw scan classifies every line instead of failing on the bad one.
    let scan = reopened.scanOperationLog("device-a").unwrap();
    assert!(scan.exists);
    assert_eq!(scan.totalLines, 3);
    assert_eq!(scan.decodedLines, 2);
    assert_eq!(scan.highestSequence, 3);
    assert_eq!(scan.findings.len(), 1);
    assert_eq!(scan.findings[0].lineNumber, 2);
    assert_eq!(scan.findings[0].byteOffset, lines[0].len() as u64);
    assert_eq!(scan.findings[0].byteLength, "{ this is not json\n".len() as u64);

    reopened.quarantineOperationLines("device-a", &[scan.findings[0].byteOffset]).unwrap();
    let repaired = reopened.operationsSince(&SyncClock::empty(), &["chat".into()], 100).unwrap();
    assert_eq!(sequences(&repaired), vec![1,3]);
}

/// A missing journal scans as absent rather than as an error.
#[test]
fn missing_journal_scans_as_absent() {
    let storage = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(storage, "empty-journal");
    let scan = store.scanOperationLog("nobody").unwrap();
    assert!(!scan.exists);
    assert_eq!(scan.totalLines, 0);
    assert!(scan.findings.is_empty());
}

/// Quarantine removes exactly the damaged lines, keeps them reversible and
/// leaves the journal readable through the same and fresh store instances.
#[test]
fn quarantine_moves_damaged_lines_to_a_reversible_sidecar() {
    let storage = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(storage.clone(), "quarantine-journal");
    // Legacy files are compatibility input; current writes use SQLite.
    let path = "quarantine-journal/operations/device-a.jsonl";
    let raw = (1..=2).map(|sequence| serde_json::to_string(&operation(sequence, "message", &format!("m{sequence}"), "upsert", SyncOperationSemantics::EntityState, json!({"text":"x"}))).unwrap() + "\n").collect::<String>();
    storage.writeBytes(path, raw.as_bytes()).unwrap();
    storage.writeBytes("quarantine-journal/devices.json", br#"["device-a"]"#).unwrap();
    let original = storage.readBytes(path).unwrap();
    let lines: Vec<&[u8]> = original.split_inclusive(|byte| *byte == b'\n').collect();
    let badOffset = lines[0].len() as u64;
    let mut corrupted = Vec::new();
    corrupted.extend_from_slice(lines[0]);
    corrupted.extend_from_slice(b"not json at all\n");
    corrupted.extend_from_slice(lines[1]);
    storage.writeBytes(path, &corrupted).unwrap();

    let scan = store.scanOperationLog("device-a").unwrap();
    assert_eq!(scan.findings.len(), 1);
    let offsets: Vec<u64> = scan.findings.iter().map(|finding| finding.byteOffset).collect();
    assert_eq!(offsets, vec![badOffset]);

    let moved = store.quarantineOperationLines("device-a", &offsets).unwrap();
    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].byteOffset, badOffset);
    assert_eq!(moved[0].line, "not json at all");
    assert!(!moved[0].lineHash.is_empty());

    // The journal is clean again for both the same instance and a fresh one.
    let rescan = store.scanOperationLog("device-a").unwrap();
    assert!(rescan.findings.is_empty());
    assert_eq!(rescan.decodedLines, 2);
    assert_eq!(rescan.highestSequence, 2);
    let reopened = SyncOperationStore::new(storage.clone(), "quarantine-journal");
    assert_eq!(
        sequences(&reopened.operationsSince(&SyncClock::empty(), &[], 100).unwrap()),
        vec![1, 2]
    );

    // The sidecar preserves the removed bytes verbatim for manual recovery.
    let sidecar = storage.readBytes("quarantine-journal/operations/device-a.jsonl.quarantine").unwrap();
    let sidecarLine = String::from_utf8(sidecar).unwrap();
    let entry: serde_json::Value = serde_json::from_str(sidecarLine.trim_end_matches('\n')).unwrap();
    assert_eq!(entry["line"].as_str().unwrap(), "not json at all");
    assert_eq!(entry["byteOffset"].as_u64().unwrap(), badOffset);

    // Quarantining offsets that no longer match any line is a no-op.
    let empty = store.quarantineOperationLines("device-a", &[999_999]).unwrap();
    assert!(empty.is_empty());
}

/// Independent devices sharing a relative sync root must never deduplicate
/// each other's operations, including when they initialize in the same tick.
#[test]
fn sync_origins_are_unique_and_remain_stable_after_reopening() {
    let mut identities = BTreeSet::new();
    for _ in 0..32 {
        let host = Arc::new(MemoryStorageHost::default());
        let store = SyncOperationStore::new(host.clone(), "runtime/sync");
        let identity = store.localDeviceId().unwrap();
        assert!(
            identities.insert(identity.clone()),
            "two devices share an operation origin"
        );
        assert_eq!(
            SyncOperationStore::new(Arc::new((*host).clone()), "runtime/sync")
                .localDeviceId()
                .unwrap(),
            identity
        );
    }
    let host = Arc::new(MemoryStorageHost::default());
    host.writeBytes("runtime/sync/local_device_id", b"existing-origin")
        .unwrap();
    assert_eq!(
        SyncOperationStore::new(host, "runtime/sync")
            .localDeviceId()
            .unwrap(),
        "existing-origin"
    );
}

/// Merges exact intervals without claiming missing predecessors or overflowing their bounds.
#[test]
fn receipt_ranges_preserve_gaps_and_merge_adjacent_intervals() {
    let mut receipt = SyncReceipt::default();
    receipt.observeRange("device-a", 102, 104);
    receipt.observeRange("device-a", 1, 100);
    assert_eq!(receipt.sequenceFor("device-a"), 100);
    assert!(!receipt.containsSequence("device-a", 101));
    assert!(receipt.containsSequence("device-a", 103));
    receipt.observe("device-a", 101);
    assert_eq!(receipt.ranges["device-a"], vec![[1, 104]]);
    receipt.observe("device-a", i64::MAX);
    receipt.observe("device-a", i64::MAX - 1);
    assert_eq!(
        receipt.ranges["device-a"],
        vec![[1, 104], [i64::MAX - 1, i64::MAX]]
    );
    receipt.validate().unwrap();
    receipt.ranges.insert("invalid".into(), vec![[4, 3]]);
    assert!(receipt.validate().is_err());
}

/// Keeps failed work durable while later completed work retains its receipt after native compaction.
#[test]
fn received_work_survives_restart_and_completion_keeps_sparse_receipts() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(host.clone(), "receive-test");
    let earlier = operation(
        101,
        "message",
        "first",
        "upsert",
        SyncOperationSemantics::EntityState,
        json!(1),
    );
    let later = operation(
        102,
        "message",
        "second",
        "upsert",
        SyncOperationSemantics::EntityState,
        json!(2),
    );
    store
        .receiveOperations(&[earlier.clone(), later.clone()])
        .unwrap();
    assert!(store.preferenceHistory().unwrap().is_empty());
    assert_eq!(store.operationsSince(&SyncClock::empty(), &[], 10).unwrap(), vec![earlier.clone(), later.clone()]);
    store
        .completeOperations(std::slice::from_ref(&later))
        .unwrap();
    // A new host identity bypasses the process-local cache while preserving its files.
    let reopened = SyncOperationStore::new(Arc::new((*host).clone()), "receive-test");
    assert_eq!(reopened.pendingOperations().unwrap(), vec![earlier.clone()]);
    let receipt = reopened.receipt().unwrap();
    assert!(receipt.containsSequence("device-a", 101));
    assert!(receipt.containsSequence("device-a", 102));
    assert!(!receipt.containsSequence("device-a", 100));
    assert_eq!(receipt.sequenceFor("device-a"), 0);
    reopened.receiveOperations(&[later]).unwrap();
    assert_eq!(reopened.pendingOperations().unwrap(), vec![earlier.clone()]);
    reopened.completeOperations(&[earlier]).unwrap();
    assert!(reopened.pendingOperations().unwrap().is_empty());
    assert_eq!(reopened.receipt().unwrap(), receipt);
}

/// Requests an unseen older sequence even after a later sequence advanced the local clock.
#[test]
fn sparse_receipt_exports_missing_predecessor_before_page_limit() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(host, "sparse-export");
    let earlier = operation(
        101,
        "message",
        "first",
        "upsert",
        SyncOperationSemantics::EntityState,
        json!(1),
    );
    let later = operation(
        102,
        "message",
        "second",
        "upsert",
        SyncOperationSemantics::EntityState,
        json!(2),
    );
    store.appendOperation(&later).unwrap();
    store.appendOperation(&earlier).unwrap();
    assert_eq!(store.localClock().unwrap().sequenceFor("device-a"), 102);
    let mut receipt = SyncReceipt::default();
    receipt.observe("device-a", 102);
    for domains in [vec![], vec!["chat".to_owned()]] {
        assert_eq!(
            store.operationsSince(&receipt, &domains, 1).unwrap(),
            vec![earlier.clone()]
        );
    }
}

/// Combines independent control acknowledgements with sparse receipts and out-of-order journal offsets.
#[test]
fn control_acknowledgements_and_sparse_receipts_share_export_filtering() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(host, "mixed-control-receipt-export");
    let earlier = operation(
        101,
        "message",
        "earlier",
        "upsert",
        SyncOperationSemantics::EntityState,
        json!(1),
    );
    let mut control = operation(
        102,
        "command",
        "policy",
        "apply",
        SyncOperationSemantics::EntityState,
        json!(2),
    );
    control.domain = NETWORK_CONTROL_SYNC_DOMAIN.to_owned();
    let later = operation(
        103,
        "message",
        "later",
        "upsert",
        SyncOperationSemantics::EntityState,
        json!(3),
    );
    for entry in [&later, &earlier, &control] {
        store.appendUnobservedOperation(entry).unwrap();
    }
    for domains in [
        vec![],
        vec!["chat".to_owned(), NETWORK_CONTROL_SYNC_DOMAIN.to_owned()],
    ] {
        let mut receipt = SyncReceipt::default();
        receipt.observeRange(&networkControlClockKey("device-a"), 1, 102);
        assert_eq!(
            store
                .operationsSince(&receipt, &domains, usize::MAX)
                .unwrap(),
            vec![earlier.clone(), later.clone()]
        );
        let mut sparse = receipt.clone();
        sparse.observe("device-a", 103);
        assert_eq!(
            store.operationsSince(&sparse, &domains, 1).unwrap(),
            vec![earlier.clone()]
        );
        receipt.observeRange("device-a", 1, 101);
        assert_eq!(
            store.operationsSince(&receipt, &domains, 1).unwrap(),
            vec![later.clone()]
        );
    }
}

/// Rejects conflicting immutable deliveries atomically and never acknowledges a partial receive batch.
#[test]
fn conflicting_received_operation_does_not_commit_other_batch_entries() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(host, "conflicting-receive");
    let mut first = operation(
        1,
        "message",
        "first",
        "upsert",
        SyncOperationSemantics::EntityState,
        json!(1),
    );
    first.domain = "binding".into();
    let second = operation(
        2,
        "message",
        "second",
        "upsert",
        SyncOperationSemantics::EntityState,
        json!(2),
    );
    store
        .receiveOperations(std::slice::from_ref(&first))
        .unwrap();
    let mut conflicting = first.clone();
    conflicting.payload = json!("changed");
    assert!(store.receiveOperations(&[second, conflicting]).is_err());
    assert_eq!(store.pendingOperations().unwrap(), vec![first]);
    assert!(!store.receipt().unwrap().containsSequence("device-a", 2));
}

/// Uses the existing encrypted preference encoding for durable receive intent.
#[test]
fn pending_preferences_are_encrypted_at_rest() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(host.clone(), "encrypted-receive");
    let mut incoming = operation(
        1,
        "entry",
        "secret",
        "set",
        SyncOperationSemantics::EntityState,
        json!({"storagePath":"runtime/config/preferences/secret.preferences.json", "encrypted":true, "key":"token", "value":"pending-secret-token"}),
    );
    incoming.domain = "preferences".into();
    store
        .receiveOperations(std::slice::from_ref(&incoming))
        .unwrap();
    let mut sql = host.openTransactionDatabase("encrypted-receive/transactions.sqlite").unwrap();
    let rows = sql.query("SELECT original FROM originals", vec![]).unwrap();
    let pending = rows[0].valueAt(0).unwrap().asString().unwrap();
    let encoded: Value = serde_json::from_str(&pending).unwrap();
    assert_eq!(encoded["payload"]["format"], ENCRYPTED_SYNC_PAYLOAD_FORMAT);
    assert!(!pending.contains("pending-secret-token"));
    assert!(!host.exists("encrypted-receive/receive_state.json").unwrap());
    let reopened = SyncOperationStore::new(Arc::new((*host).clone()), "encrypted-receive");
    assert_eq!(reopened.pendingOperations().unwrap(), vec![incoming]);
}

/// Retired history cannot consume a page and hide later theme and role preferences.
#[test]
fn retained_preferences_advance_past_retired_history_with_count_and_byte_limits() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(host, "retained-preference-export");
    let retiredPath = "runtime/config/preferences/character_cards.preferences.json";
    let mut history = (1..=520)
        .map(|sequence| {
            let mut entry = operation(
                sequence,
                "entry",
                &format!("retired-{sequence}"),
                "set",
                SyncOperationSemantics::EntityState,
                json!({
                    "storagePath": retiredPath, "formatVersion": 2,
                    "key": format!("record_{sequence}"), "value": "retired",
                }),
            );
            entry.domain = "preferences".into();
            entry
        })
        .collect::<Vec<_>>();
    let mut theme = operation(
        521,
        "entry",
        "saved-theme",
        "set",
        SyncOperationSemantics::EntityState,
        json!({
            "storagePath": "runtime/config/preferences/user_preferences.preferences.json",
            "formatVersion": 2, "key": "theme_config_saved", "value": {"id":"saved"},
        }),
    );
    theme.domain = "preferences".into();
    let mut card = theme.clone();
    card.opId = "device-a:522".into();
    card.sequence = 522;
    card.createdAt = 522;
    card.entityId = "saved-card".into();
    card.payload = json!({
        "storagePath": "runtime/plugin_data/space/com.operit.character_cards/cards.preferences.json",
        "formatVersion": 2, "key": "record_saved", "value": {"id":"saved"},
    });
    history.extend([theme.clone(), card.clone()]);
    store.appendOperations(&history).unwrap();
    let byteBudget = serde_json::to_vec(&theme).unwrap().len();
    for domains in [vec![], vec!["preferences".to_owned()]] {
        for (limit, maxBytes) in [(1, usize::MAX), (usize::MAX, byteBudget)] {
            let mut receipt = SyncReceipt::default();
            for expected in [&theme, &card] {
                let page = store
                    .operationsSinceWithBudget(&receipt, &domains, limit, maxBytes, |operation| {
                        operation.payload["storagePath"] != retiredPath
                    })
                    .unwrap();
                assert_eq!(page, vec![expected.clone()]);
                receipt.observe(&expected.originDeviceId, expected.sequence);
            }
            let page = store
                .operationsSinceWithBudget(&receipt, &domains, limit, maxBytes, |operation| {
                    operation.payload["storagePath"] != retiredPath
                })
                .unwrap();
            assert!(page.is_empty());
            assert!(!receipt.containsSequence("device-a", 1));
        }
    }
}

/// Non-chat operations retain immutable content verification after completion and restart.
#[test]
fn completed_receipt_rejects_changed_original_payload_after_restart() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(host.clone(), "receipt-content");
    let mut original = operation(1, "message", "id", "upsert", SyncOperationSemantics::Transaction, json!({"content":"original"}));
    original.domain = "binding".into();
    store.receiveOperations(&[original.clone()]).unwrap();
    store.completeOperations(&[original.clone()]).unwrap();
    let restarted = SyncOperationStore::new(Arc::new((*host).clone()), "receipt-content");
    restarted.receiveOperations(&[original.clone()]).unwrap();
    let mut changed = original; changed.payload = json!({"content":"changed"});
    assert!(restarted.receiveOperations(&[changed]).is_err());
    assert!(restarted.pendingOperations().unwrap().is_empty());
}

/// A journal/projection duplicate has one replay effect; conflicting immutable
/// payloads must still fail before any policy state is returned.
#[test]
fn control_replay_merges_projection_duplicates_and_rejects_conflicts() {
    for constrained in [false, true] {
    use crate::NetworkControlStore::NetworkControlStore;
    use crate::PreferencesDataStore::stringPreferencesKey;
    let storage = Arc::new(MemoryStorageHost { releasePreferencesCache: constrained, ..Default::default() });
    let store = NetworkControlStore::new(storage.clone()).unwrap();
    store.bootstrapCurrentSpace().unwrap();
    store.admitMember("peer".into()).unwrap();
    let before = store.currentSpaceOperations().unwrap();
    let state = store.currentState().unwrap();
    let projection = NetworkControlStore::projectionStore(storage);
    projection.edit(|prefs| {
        for operation in before.iter().rev() {
            prefs.set(&stringPreferencesKey(&operation.opId), serde_json::to_value(operation).unwrap());
        }
    }).unwrap();
    assert_eq!(store.currentSpaceOperations().unwrap(), before);
    assert_eq!(store.currentState().unwrap(), state);
    let mut conflict = before.last().unwrap().clone();
    conflict.payload["command"]["admit_member"]["nodeId"] = json!("other");
    projection.edit(|prefs| prefs.set(&stringPreferencesKey(&conflict.opId), serde_json::to_value(&conflict).unwrap())).unwrap();
    assert!(store.currentState().unwrap_err().contains("Conflicting immutable"));
    }
}


/// Compact replay must accept the string-encoded unit command as well as
/// object-encoded commands and retain its membership effect after reopening.
#[test]
fn bounded_control_replay_accepts_leave_space_unit_commands() {
    use crate::NetworkControlStore::NetworkControlStore;
    let storage = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let store = NetworkControlStore::newNodeLocal(storage.clone()).unwrap();
    let initial = store.bootstrapCurrentSpace().unwrap();
    assert!(store.leaveCurrentSpace().unwrap().is_some());
    let reopened = NetworkControlStore::newNodeLocal(storage).unwrap();
    let state = reopened.currentState().unwrap();
    assert_eq!(state.spaceId, initial.spaceId);
    assert!(state.memberNodeIds.is_empty());
}


/// A constrained endpoint can relay an existing root through an ordinary member,
/// but cannot accept a changed immutable payload under that root's identity.
#[test]
fn bounded_peer_control_repeats_exact_known_authority_and_applies_a_signed_exit() {
    use crate::NetworkControlStore::NetworkControlStore;
    let storage = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let store = NetworkControlStore::newNodeLocal(storage.clone()).unwrap();
    store.bootstrapCurrentSpace().unwrap();
    let bootstrap = store.currentSpaceOperations().unwrap().remove(0);
    store.admitMember("peer".into()).unwrap();
    let before = store.currentState().unwrap();
    // Legacy caches need not use the operation ID as their physical map key.
    let projection = NetworkControlStore::projectionStore(storage.clone());
    projection.setTypedEntry("legacy-root-key", &bootstrap).unwrap();
    projection.removeEntry(&bootstrap.opId).unwrap();
    store.projectPeerControlOperations("peer", std::slice::from_ref(&bootstrap)).unwrap();
    assert_eq!(store.currentState().unwrap(), before);
    let files = storage.files.lock().unwrap().clone();
    let mut changed = bootstrap.clone();
    changed.payload["unknownSignedField"] = json!({"changed":true});
    assert!(store.projectPeerControlOperations("peer", &[changed]).is_err());
    assert_eq!(*storage.files.lock().unwrap(), files);
    let mut leave = bootstrap;
    leave.opId = "peer-leave".into();
    leave.entityId = leave.opId.clone();
    leave.originDeviceId = "peer".into();
    leave.sequence = 1;
    leave.createdAt = store.currentSpaceOperations().unwrap().iter().map(|operation| operation.createdAt).max().unwrap() + 1;
    leave.payload = json!({"commandId":leave.entityId, "spaceId":before.spaceId,
        "issuerNodeId":"peer", "command":"leave_space"});
    store.projectPeerControlOperations("peer", &[leave]).unwrap();
    assert!(!store.currentState().unwrap().memberNodeIds.contains("peer"));
}

/// Export preserves unknown payloads, rejects changed revisions and never writes.
#[test]
fn endpoint_control_export_pages_complete_immutable_authority_without_mutation() {
    use crate::NetworkControlStore::NetworkControlStore;
    let storage = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let store = NetworkControlStore::newNodeLocal(storage.clone()).unwrap();
    let state = store.bootstrapCurrentSpace().unwrap();
    store.admitMember("peer-a".into()).unwrap();
    store.admitMember("peer-b".into()).unwrap();
    let mut expected = store.currentSpaceOperations().unwrap();
    let projection = NetworkControlStore::projectionStore(storage.clone());
    expected[0].payload["futureField"] = json!({"keep": ["设备", null, 7]});
    projection.setTypedEntry(&expected[0].opId, &expected[0]).unwrap();
    // A legacy physical key and a semantically equal duplicate must not drop or
    // duplicate authority. Other Spaces do not leak into this export.
    projection.setTypedEntry("legacy-alias", &expected[0]).unwrap();
    let mut archived = expected[0].clone();
    archived.opId = "archived".into();
    archived.payload["spaceId"] = json!("old-space");
    projection.setTypedEntry(&archived.opId, &archived).unwrap();
    let writes = storage.writeCount();
    let manifest = store.spaceControlManifest(&state.spaceId).unwrap();
    let first = store.spaceControlExport(&state.spaceId, None, None).unwrap();
    let revision = first.revision.clone();
    assert_eq!(manifest.revision, revision);
    assert_eq!(manifest.count, expected.len());
    assert_eq!(first.count, expected.len());
    let mut after = None;
    let mut actual = Vec::new();
    loop {
        let page = store.spaceControlExport(&state.spaceId, Some(&revision), after.as_deref()).unwrap();
        assert_eq!(page.count, expected.len());
        let Some(operation) = page.operation else { break; };
        after = Some(operation.opId.clone());
        actual.push(operation);
    }
    expected.sort_by(|left, right| left.opId.cmp(&right.opId));
    assert_eq!(actual, expected);
    assert_eq!(storage.writeCount(), writes);
    assert!(store.spaceControlExport(&state.spaceId, Some(&revision), Some("missing")).unwrap_err().contains("CURSOR_INVALID"));
    store.admitMember("peer-c".into()).unwrap();
    assert!(store.spaceControlExport(&state.spaceId, Some(&revision), after.as_deref()).unwrap_err().contains("EXPORT_CHANGED"));
    let mut conflict = expected[0].clone();
    conflict.payload["futureField"] = json!(false);
    projection.setTypedEntry("conflicting-alias", &conflict).unwrap();
    assert!(store.spaceControlExport(&state.spaceId, None, None).unwrap_err().contains("Conflicting immutable"));
}

/// An endpoint must migrate its legacy journal before advertising a complete export.
#[test]
fn endpoint_control_export_refuses_to_omit_unmigrated_journal_authority() {
    use crate::NetworkControlStore::NetworkControlStore;
    let storage = Arc::new(MemoryStorageHost::default());
    let legacy = NetworkControlStore::new(storage.clone()).unwrap();
    let state = legacy.bootstrapCurrentSpace().unwrap();
    let expected = legacy.currentSpaceOperations().unwrap();
    let endpoint = NetworkControlStore::newNodeLocal(storage.clone()).unwrap();
    assert!(endpoint.spaceControlExport(&state.spaceId, None, None).unwrap_err().contains("MIGRATION_REQUIRED"));
    endpoint.migrateNodeLocalProjection(storage.as_ref()).unwrap();
    let page = endpoint.spaceControlExport(&state.spaceId, None, None).unwrap();
    assert_eq!(page.count, expected.len());
    assert_eq!(page.operation, Some(expected[0].clone()));
    let full = legacy.spaceControlExport(&state.spaceId, None, None).unwrap();
    assert_eq!(full.count, page.count);
    assert_eq!(full.operation, page.operation);
}

/// Cold receipt/index construction and every query intent use the same bounded reader.
#[test]
fn journal_queries_use_bounded_ranges_after_reopen() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(host.clone(), "bounded");
    let operations = (1..=40).map(|sequence| operation(sequence, "message", &format!("m{sequence}"),
        "upsert", SyncOperationSemantics::EntityState, json!({"text":"x".repeat(8192)}))).collect::<Vec<_>>();
    store.appendOperations(&operations).unwrap();
    drop(store);
    host.forbidWholeJournalReads.store(true, Ordering::SeqCst);
    let reopened = SyncOperationStore::new(host.clone(), "bounded");
    assert!(reopened.receipt().unwrap().containsSequence("device-a", 40));
    for domains in [vec![], vec![operations[0].domain.clone()]] {
        assert_eq!(reopened.operationsSinceWithBudget(&SyncClock::empty(), &domains, 1, 1024, |_|true).unwrap(),
            vec![operations[0].clone()]);
    }
    assert!(reopened.preferenceMergeHistory().unwrap().is_empty());
    assert!(!reopened.scanOperationLog("device-a").unwrap().exists);
}

/// Shared handles serialize writes; the registry owns neither the state nor its Host.
#[test]
fn journal_registry_releases_state_after_last_handle() {
    let host = Arc::new(MemoryStorageHost::default());
    let first = SyncOperationStore::new(host.clone(), "lifecycle");
    let second = SyncOperationStore::new(host.clone(), "lifecycle");
    assert!(Arc::ptr_eq(&first.sharedState, &second.sharedState));
    let state = Arc::downgrade(&first.sharedState);
    drop(first);
    assert!(state.upgrade().is_some());
    drop(second);
    assert!(state.upgrade().is_none());
    let weak_host = Arc::downgrade(&host);
    drop(host);
    assert!(weak_host.upgrade().is_none());
}

/// Pending originals are planned with archived states before compaction and paging.
#[test]
fn relay_pages_compact_archived_and_pending_records_together() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(host, "combined-export");
    let first = operation(1, "message", "same", "upsert", SyncOperationSemantics::EntityState, json!(1));
    let newer = operation(2, "message", "same", "upsert", SyncOperationSemantics::EntityState, json!(2));
    let other = operation(3, "message", "other", "upsert", SyncOperationSemantics::EntityState, json!(3));
    store.appendOperation(&first).unwrap();
    store.receiveOperations(&[newer.clone(), other]).unwrap();
    let mut inventory = BTreeSet::new();
    let page = store.operationsSinceWithInventory(&SyncClock::empty(), &[], 1, 1024, Some(&mut inventory), |_| true).unwrap();
    assert_eq!(page, vec![newer.clone()]);
    assert_eq!(inventory.len(), 2, "total must include later pages but exclude compacted old states");
    assert!(!inventory.contains(&first.opId));
    assert_eq!(store.operationsSinceWithBudget(&SyncClock::empty(), &[], 1, 1024, |_|true).unwrap(), vec![newer.clone()]);
    // A caller's retention rule still runs before state compaction.
    assert_eq!(store.operationsSinceWithBudget(&SyncClock::empty(), &[], 1, 1024, |op|op.sequence != 2).unwrap(), vec![first]);
    assert_eq!(store.pendingOperationCount().unwrap(), 2);
    store.completeOperations(&[newer]).unwrap();
    assert_eq!(store.pendingOperationCount().unwrap(), 1);
}

/// A receipt never acknowledges a payload whose durable file failed, and completion
/// never removes a file before its metadata commit succeeds.
#[test]
fn receive_and_completion_keep_payloads_and_receipts_atomic() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(host.clone(), "receive-atomic");
    assert_eq!(store.pendingOperationCount().unwrap(),0);
    let mut sql = host.openTransactionDatabase("receive-atomic/transactions.sqlite").unwrap();
    let incoming = operation(1,"message","one","upsert",SyncOperationSemantics::EntityState,json!("data"));
    sql.executeBatch("CREATE TRIGGER reject_receive BEFORE INSERT ON originals BEGIN SELECT RAISE(ABORT,'receive failed'); END").unwrap();
    assert!(store.receiveOperations(&[incoming.clone()]).is_err());
    assert!(!store.receipt().unwrap().containsSequence("device-a",1));
    assert_eq!(store.pendingOperationCount().unwrap(),0);
    sql.executeBatch("DROP TRIGGER reject_receive").unwrap();
    store.receiveOperations(&[incoming.clone()]).unwrap();
    sql.executeBatch("CREATE TRIGGER reject_completion BEFORE UPDATE ON originals BEGIN SELECT RAISE(ABORT,'completion failed'); END").unwrap();
    assert!(store.completeOperations(&[incoming.clone()]).is_err());
    assert_eq!(store.pendingOperations().unwrap(),vec![incoming.clone()]);
    sql.executeBatch("DROP TRIGGER reject_completion").unwrap();
    store.completeOperations(&[incoming]).unwrap();
    assert_eq!(store.pendingOperationCount().unwrap(),0);
    assert!(store.receipt().unwrap().containsSequence("device-a",1));
}

/// The adapted authority reader keeps the bounded journal path without publishing
/// unrelated preference outboxes, and retains archived/pending export semantics.
#[test]
fn archived_control_replay_is_bounded_and_does_not_publish_preference_outboxes() {
    use crate::PreferencesDataStore::{PreferencesDataStore, stringPreferencesKey};
    use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;

    let host = Arc::new(MemoryStorageHost::default());
    let path = "runtime/config/preferences/control-replay.preferences.json";
    let preferences = PreferencesDataStore::newWithStorage(host.clone(), path);
    preferences.edit(|data| data.set(&stringPreferencesKey("theme"), json!("dark"))).unwrap();
    let committed: Value = serde_json::from_slice(&host.readBytes(path).unwrap()).unwrap();
    assert!(committed.get("__operit_transaction_commit").is_none());

    let store = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
    let mut archived = operation(1, "command", "archived", "apply",
        SyncOperationSemantics::Transaction, json!({"permission":"archived"}));
    archived.domain = NETWORK_CONTROL_SYNC_DOMAIN.to_owned();
    let business = operation(2, "message", "unrelated", "upsert",
        SyncOperationSemantics::EntityState, json!({"text":"x".repeat(8192)}));
    let mut pending = operation(3, "command", "pending", "apply",
        SyncOperationSemantics::Transaction, json!({"permission":"pending"}));
    pending.domain = NETWORK_CONTROL_SYNC_DOMAIN.to_owned();
    store.appendOperations(&[archived.clone(), business]).unwrap();
    store.receiveOperations(std::slice::from_ref(&pending)).unwrap();
    drop(store);

    host.forbidWholeJournalReads.store(true, Ordering::SeqCst);
    let reopened = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
    let before = host.readBytes(path).unwrap();
    assert_eq!(reopened.archivedControlOperations().unwrap(), vec![archived, pending.clone()]);
    assert_eq!(host.readBytes(path).unwrap(), before);

    reopened.setExportFloor("device-a", 1).unwrap();
    assert_eq!(reopened.archivedControlOperations().unwrap(), vec![pending]);
    assert_eq!(host.readBytes(path).unwrap(), before);
    // The fixture has a real pending outbox: an explicit export still publishes it.
    assert!(reopened.publishPreferenceOutboxes().unwrap().is_empty());
    let published: Value = serde_json::from_slice(&host.readBytes(path).unwrap()).unwrap();
    assert!(published.get("__operit_transaction_commit").is_none());
}

/// Receipt lookup validates immutable conflicts even when the matching receipt
/// sorts before the conflicting alias, and cannot append another admission.
#[test]
fn bounded_review_receipts_validate_complete_history_and_local_sequences() {
    use crate::NetworkControlStore::NetworkControlStore;
    let storage = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let store = NetworkControlStore::newNodeLocal(storage.clone()).unwrap();
    let initial = store.bootstrapCurrentSpace().unwrap();
    let issuer = initial.memberNodeIds.iter().next().unwrap();
    for index in 0..20 { store.updatePolicy(format!("history-{index}"), "unchanged".into()).unwrap(); }
    let identities = BTreeMap::from([("esp".into(), "edge".into())]);
    let receipt = store.admitSpaceForReview("source".into(), identities.clone(), "bounded").unwrap();
    assert_eq!(receipt.sequence, 22);
    assert_eq!(store.spaceControlOperation(&initial.spaceId, &receipt.entityId, issuer).unwrap(), Some(receipt.clone()));
    assert!(store.spaceControlOperation(&initial.spaceId, &receipt.entityId, "other").unwrap().is_none());
    assert_eq!(store.admitSpaceForReview("source".into(), identities.clone(), "bounded").unwrap(), receipt);
    let next = store.updatePolicy("after-review".into(), "kept".into()).unwrap();
    assert_eq!(next.sequence, receipt.sequence + 1);
    let projection = NetworkControlStore::projectionStore(storage.clone());
    let mut alias = receipt.clone();
    alias.payload["futureField"] = json!({"immutable": true});
    projection.setTypedEntry("zz-conflicting-alias", &alias).unwrap();
    let writes = storage.writeCount();
    assert!(store.spaceControlOperation(&initial.spaceId, &receipt.entityId, issuer)
        .unwrap_err().contains("Conflicting immutable"));
    assert!(store.admitSpaceForReview("source".into(), identities, "bounded").is_err());
    assert_eq!(storage.writeCount(), writes);
}

/// Paged authority must never allocate a whole expanded policy on current
/// endpoint files. Its digest and all unknown payload bytes stay compatible.
#[test]
fn endpoint_control_export_streams_ranges_with_identical_digest_for_all_key_orders() {
    use crate::NetworkControlStore::NetworkControlStore;
    use sha2::{Digest, Sha256};
    let storage = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let store = NetworkControlStore::newNodeLocal(storage.clone()).unwrap();
    let state = store.bootstrapCurrentSpace().unwrap();
    for index in 0..12 { store.admitMember(format!("peer-{index}-range-boundary")).unwrap(); }
    let projection = NetworkControlStore::projectionStore(storage.clone());
    let mut expected = store.currentSpaceOperations().unwrap();
    expected[0].payload["futureField"] = json!({"text":"设备  \n \"".repeat(100),"unknown":[null,9]});
    projection.setTypedEntry(&expected[0].opId, &expected[0]).unwrap();
    expected.sort_by(|a,b| a.opId.cmp(&b.opId));
    // The range parser must hash legacy structural whitespace verbatim too.
    let path = "runtime/link_access/space_policy.preferences.json";
    let pretty: Value = serde_json::from_slice(&storage.readBytes(path).unwrap()).unwrap();
    storage.writeBytes(path, &serde_json::to_vec_pretty(&pretty).unwrap()).unwrap();
    let revision = projection.withEncodedEntries(|entries| {
        let mut hash = Sha256::new();
        hash.update(b"operit.endpoint-control-export.v1\0");
        hash.update((state.spaceId.len() as u64).to_be_bytes()); hash.update(state.spaceId.as_bytes());
        for (key, raw) in entries {
            for bytes in [key.as_bytes(), raw.get().as_bytes()] {
                hash.update((bytes.len() as u64).to_be_bytes()); hash.update(bytes);
            }
        }
        format!("{:x}", hash.finalize())
    }).unwrap();
    let original = storage.readBytes(path).unwrap();
    let writes = storage.writeCount();
    storage.forbidWholePolicyReads.store(true, Ordering::SeqCst);
    let manifest = store.spaceControlManifest(&state.spaceId).unwrap();
    assert_eq!(manifest.revision, revision);
    assert_eq!(manifest.count, expected.len());
    let mut after = None;
    let mut actual = Vec::new();
    loop {
        let page = store.spaceControlExport(&state.spaceId, Some(&revision), after.as_deref()).unwrap();
        assert_eq!(page.revision, revision);
        assert_eq!(page.count, expected.len());
        let Some(operation) = page.operation else { break; };
        after = Some(operation.opId.clone()); actual.push(operation);
    }
    assert_eq!(actual, expected);
    assert_eq!(storage.writeCount(), writes);
    storage.forbidWholePolicyReads.store(false, Ordering::SeqCst);
    assert_eq!(storage.readBytes(path).unwrap(), original);
    // Unordered legacy maps retain the old sorted export and exact digest.
    let entries: BTreeMap<String, Box<serde_json::value::RawValue>> = serde_json::from_slice(&original).unwrap();
    let reversed = format!("{{{}}}", entries.iter().rev().map(|(key, raw)|
        format!("{}:{}", serde_json::to_string(key).unwrap(), raw.get())).collect::<Vec<_>>().join(","));
    storage.writeBytes(path, reversed.as_bytes()).unwrap();
    let legacy = store.spaceControlExport(&state.spaceId, Some(&revision), None).unwrap();
    assert_eq!(legacy.operation, expected.first().cloned());
    assert_eq!(storage.readBytes(path).unwrap(), reversed.as_bytes());
}

/// Reopened query handles share the live projection without reading or retaining its history.
#[test]
fn control_queries_share_projection_and_observe_revocation() {
    use crate::NetworkControlStore::NetworkControlStore;
    for constrained in [false, true] {
        let host = Arc::new(MemoryStorageHost { releasePreferencesCache: constrained, ..Default::default() });
        let owner = NetworkControlStore::newNodeLocal(host.clone()).unwrap();
        owner.bootstrapCurrentSpace().unwrap();
        owner.admitMember("peer".into()).unwrap();
        for index in 0..32 { owner.updatePolicy("display".into(), index.to_string()).unwrap(); }
        owner.currentState().unwrap();
        let reads = host.policyReads.load(Ordering::SeqCst);
        for _ in 0..32 {
            let reader = NetworkControlStore::newNodeLocal(host.clone()).unwrap();
            assert!(reader.nodeHasCapability("peer", "runtime.execute", None).unwrap());
            assert!(!reader.nodeIsDisconnected("peer").unwrap());
        }
        assert_eq!(host.policyReads.load(Ordering::SeqCst), reads, "read-only admission checks reread history");
        let reader = NetworkControlStore::newNodeLocal(host.clone()).unwrap();
        owner.disconnectNode("peer".into()).unwrap();
        assert!(reader.nodeIsDisconnected("peer").unwrap());
        owner.removeMember("peer".into()).unwrap();
        assert!(!reader.nodeHasCapability("peer", "runtime.execute", None).unwrap());
        owner.admitMember("peer".into()).unwrap();
        assert!(!reader.nodeIsDisconnected("peer").unwrap());
    }
}

/// Direct journal publication invalidates the same projection as node-local imports.
#[test]
fn control_projection_observes_independent_journal_appends() {
    use crate::NetworkControlStore::{NetworkControlStore, NetworkControlCommand, NetworkControlCommandRecord};
    let host = Arc::new(MemoryStorageHost::default());
    let owner = NetworkControlStore::new(host.clone()).unwrap();
    let before = owner.bootstrapCurrentSpace().unwrap();
    let issuer = before.memberNodeIds.iter().next().unwrap().clone();
    let journal = SyncOperationStore::new(host.clone(), "runtime/sync");
    let mutation = journal.appendLocalOperation(&issuer, NewSyncOperation {
        domain: "network_control".into(), entityType: "command".into(), entityId: "external-policy".into(),
        operation: "apply".into(), semantics: SyncOperationSemantics::Transaction,
        payload: serde_json::to_value(NetworkControlCommandRecord {
            commandId: "external-policy".into(), spaceId: before.spaceId.clone(), issuerNodeId: issuer.clone(),
            command: NetworkControlCommand::PolicyUpdate { policyId: "external".into(), value: "committed".into() },
        }).unwrap(),
    }).unwrap();
    assert_eq!(mutation.domain, "network_control");
    assert_eq!(owner.currentState().unwrap().policies.get("external").map(String::as_str), Some("committed"));
}

/// Verifies change summaries drop progress this device only learned from peers.
#[test]
fn authored_clock_retains_only_local_origins() {
    let mut clock = SyncClock::empty();
    clock.setSequence("core-local", 3);
    clock.setSequence("core-local:sql", 7);
    clock.setSequence("core-local:preferences:abcd", 2);
    clock.setSequence("core-remote", 9);
    clock.setSequence("core-remote:sql", 4);
    clock.retainAuthoredBy("core-local");
    assert_eq!(
        clock.sequences.keys().cloned().collect::<Vec<_>>(),
        vec!["core-local", "core-local:preferences:abcd", "core-local:sql"]
    );
}

/// Queries, including newly opened store handles, must never reopen authority
/// history after initialization. Unrelated chat writes must not invalidate it.
#[test]
fn control_state_reads_skip_history_after_business_writes_and_handle_reopen() {
    use crate::NetworkControlStore::NetworkControlStore;
    use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
    let host = Arc::new(MemoryStorageHost::default());
    let control = NetworkControlStore::new(host.clone()).unwrap();
    control.bootstrapCurrentSpace().unwrap();
    control.admitMember("phone".into()).unwrap();
    let expected = control.currentState().unwrap();
    drop(control);

    let journal = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
    let history = (1..=1000)
        .map(|sequence| {
            operationFromOrigin(
                "chat-peer",
                sequence,
                "message",
                &sequence.to_string(),
                "upsert",
                json!({"text": "x".repeat(1024)}),
            )
        })
        .collect::<Vec<_>>();
    journal.appendOperations(&history).unwrap();
    drop(journal);
    host.forbidControlHistoryReads.store(true, Ordering::SeqCst);
    for _ in 0..10 {
        let reopened = NetworkControlStore::new(host.clone()).unwrap();
        for _ in 0..100 {
            assert_eq!(reopened.currentState().unwrap(), expected);
            assert!(reopened
                .nodeHasCapability("phone", "runtime.execute", None)
                .unwrap());
            assert!(!reopened.nodeIsDisconnected("phone").unwrap());
        }
        assert_eq!(reopened.currentSpaceOperations().unwrap().len(), 2);
    }
}

fn controlTestCommand(
    template: &SyncOperation,
    sequence: i64,
    createdAt: i64,
    command: crate::NetworkControlStore::NetworkControlCommand,
) -> SyncOperation {
    use crate::NetworkControlStore::NetworkControlCommandRecord;
    let mut operation = template.clone();
    operation.opId = format!("{}:{sequence}", operation.originDeviceId);
    operation.sequence = sequence;
    operation.createdAt = createdAt;
    operation.entityId = format!("test-control-{sequence}");
    let mut record: NetworkControlCommandRecord =
        serde_json::from_value(template.payload.clone()).unwrap();
    record.commandId = operation.entityId.clone();
    record.command = command;
    operation.payload = serde_json::to_value(record).unwrap();
    operation
}

/// State follows deterministic command order, including commands delivered late,
/// and is already maintained before the next query or duplicate delivery.
#[test]
fn control_state_write_path_reorders_late_commands_and_survives_cold_reopen() {
    use crate::NetworkControlStore::{NetworkControlCommand, NetworkControlStore};
    use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
    let host = Arc::new(MemoryStorageHost::default());
    let control = NetworkControlStore::new(host.clone()).unwrap();
    control.bootstrapCurrentSpace().unwrap();
    let template = control.admitMember("phone".into()).unwrap();
    let journal = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
    let disconnect = controlTestCommand(
        &template,
        template.sequence + 1,
        template.createdAt + 100,
        NetworkControlCommand::DisconnectNode {
            nodeId: "phone".into(),
        },
    );
    journal.appendUnobservedOperation(&disconnect).unwrap();
    assert!(control.nodeIsDisconnected("phone").unwrap());
    let late = controlTestCommand(
        &template,
        template.sequence + 2,
        template.createdAt + 50,
        NetworkControlCommand::AdmitMember {
            nodeId: "phone".into(),
        },
    );
    journal.appendOperations(&[late.clone()]).unwrap();
    journal.appendUnobservedOperation(&late).unwrap();
    assert!(
        control.nodeIsDisconnected("phone").unwrap(),
        "late reconnect precedes the disconnect"
    );
    let reconnect = controlTestCommand(
        &template,
        template.sequence + 3,
        template.createdAt + 200,
        NetworkControlCommand::AdmitMember {
            nodeId: "phone".into(),
        },
    );
    journal.appendOperation(&reconnect).unwrap();
    let expected = control.currentState().unwrap();
    assert!(!expected.disconnectedNodeIds.contains("phone"));
    let operations = control.currentSpaceOperations().unwrap();
    assert_eq!(
        control
            .validateSpacePolicy(&expected.spaceId, &operations)
            .unwrap(),
        expected
    );

    // A different host identity over the durable bytes forces startup recovery,
    // rather than reusing the process-local state maintained above.
    let cold_host = Arc::new(host.as_ref().clone());
    let cold = NetworkControlStore::new(cold_host).unwrap();
    assert_eq!(cold.currentState().unwrap(), expected);
}

/// A failed receipt commit must not publish authority. Completing a pending-only
/// command removes it, while completing an archived command preserves its state.
#[test]
fn control_state_tracks_atomic_receipts_and_pending_completion() {
    use crate::NetworkControlStore::{NetworkControlCommand, NetworkControlStore};
    use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
    let host = Arc::new(MemoryStorageHost::default());
    let control = NetworkControlStore::new(host.clone()).unwrap();
    control.bootstrapCurrentSpace().unwrap();
    let template = control.admitMember("phone".into()).unwrap();
    let journal = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
    let disconnect = controlTestCommand(
        &template,
        template.sequence + 1,
        template.createdAt + 1,
        NetworkControlCommand::DisconnectNode {
            nodeId: "phone".into(),
        },
    );
    let mut sql = host.openTransactionDatabase(&format!("{RUNTIME_SYNC_DIR_PATH}/transactions.sqlite")).unwrap();
    sql.executeBatch("CREATE TRIGGER reject_control_receive BEFORE INSERT ON originals BEGIN SELECT RAISE(ABORT,'receive failed'); END").unwrap();
    assert!(journal.receiveOperations(&[disconnect.clone()]).is_err());
    assert!(!control.nodeIsDisconnected("phone").unwrap());
    sql.executeBatch("DROP TRIGGER reject_control_receive").unwrap();
    journal.receiveOperations(&[disconnect.clone()]).unwrap();
    assert!(control.nodeIsDisconnected("phone").unwrap());
    journal.completeOperations(&[disconnect.clone()]).unwrap();
    assert!(!control.nodeIsDisconnected("phone").unwrap());

    let later = controlTestCommand(
        &template,
        template.sequence + 2,
        template.createdAt + 2,
        NetworkControlCommand::DisconnectNode {
            nodeId: "phone".into(),
        },
    );
    journal.receiveOperations(&[later.clone()]).unwrap();
    journal.appendUnobservedOperation(&later).unwrap();
    journal.completeOperations(&[later]).unwrap();
    assert!(control.nodeIsDisconnected("phone").unwrap());
}

/// Repair is a mutation too: removing a journal command must immediately update
/// the existing readers instead of leaving their authorization snapshot stale.
#[test]
fn quarantining_legacy_files_preserves_committed_sql_authority() {
    use crate::NetworkControlStore::NetworkControlStore;
    use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
    let host = Arc::new(MemoryStorageHost::default());
    let control = NetworkControlStore::new(host.clone()).unwrap();
    control.bootstrapCurrentSpace().unwrap();
    control.admitMember("phone".into()).unwrap();
    let disconnect = control.disconnectNode("phone".into()).unwrap();
    let journal = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
    let path = journal.operationsPath(&disconnect.originDeviceId);
    host.writeBytes(&path, (serde_json::to_string(&disconnect).unwrap() + "\n").as_bytes()).unwrap();
    journal.quarantineOperationLines(&disconnect.originDeviceId, &[0]).unwrap();
    assert!(control.nodeIsDisconnected("phone").unwrap());
    assert!(control.currentSpaceOperations().unwrap().contains(&disconnect));
    journal.reloadNetworkControlState().unwrap();
    assert!(control.nodeIsDisconnected("phone").unwrap());
}

/// Shared read models must stay scoped to the host, even when both use the same
/// canonical paths, and the registry must never keep a storage host alive.
#[test]
fn control_state_is_storage_scoped_without_retaining_hosts() {
    use crate::NetworkControlStore::NetworkControlStore;
    let first = Arc::new(MemoryStorageHost::default());
    let second = Arc::new(MemoryStorageHost::default());
    let a = NetworkControlStore::new(first.clone()).unwrap();
    let b = NetworkControlStore::new(second.clone()).unwrap();
    a.bootstrapCurrentSpace().unwrap();
    b.bootstrapCurrentSpace().unwrap();
    a.admitMember("only-first".into()).unwrap();
    assert!(a
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains("only-first"));
    assert!(!b
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains("only-first"));
    // Verify the control registry's ownership independently of other store
    // registries: it must retain the model but only weakly reference its host.
    use crate::NetworkControlReadModel::{ControlReadModels, NetworkControlReadModel};
    let registryHost: Arc<dyn RuntimeStorageHost> = Arc::new(MemoryStorageHost::default());
    let weak = Arc::downgrade(&registryHost);
    let mut registry = ControlReadModels::default();
    registry.insert(
        &registryHost,
        "control",
        Arc::new(Mutex::new(NetworkControlReadModel::default())),
    );
    drop(registryHost);
    assert!(weak.upgrade().is_none());
}

/// The journal append can commit before a later metadata write fails. Readers
/// must follow the durable command rather than retain the pre-write authority.
#[test]
fn control_state_follows_durable_append_when_clock_commit_fails() {
    use crate::NetworkControlStore::NetworkControlStore;
    let host = Arc::new(MemoryStorageHost::default());
    let control = NetworkControlStore::new(host.clone()).unwrap();
    control.bootstrapCurrentSpace().unwrap();
    control.admitMember("phone".into()).unwrap();
    host.failAtomic.store(3, Ordering::SeqCst);
    assert!(control.disconnectNode("phone".into()).is_err());
    assert!(control.nodeIsDisconnected("phone").unwrap());
    host.failAtomic.store(0, Ordering::SeqCst);
    let cold = NetworkControlStore::new(Arc::new(host.as_ref().clone())).unwrap();
    assert_eq!(
        control.currentState().unwrap(),
        cold.currentState().unwrap()
    );
}

/// Concurrent durable arrivals and new query handles share one publication,
/// independently of arrival order or the lifetime of temporary stores.
#[test]
fn concurrent_control_writes_publish_one_shared_state() {
    use crate::NetworkControlStore::{NetworkControlCommand, NetworkControlStore};
    use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
    let host = Arc::new(MemoryStorageHost::default());
    let control = NetworkControlStore::new(host.clone()).unwrap();
    control.bootstrapCurrentSpace().unwrap();
    let template = control.admitMember("phone".into()).unwrap();
    let start = Arc::new(Barrier::new(5));
    let mut writers = Vec::new();
    for worker in 0..4 {
        let host = host.clone();
        let template = template.clone();
        let start = start.clone();
        writers.push(std::thread::spawn(move || {
            let journal = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
            start.wait();
            for index in 0..10 {
                let ordinal = worker * 10 + index + 1;
                let operation = controlTestCommand(
                    &template,
                    template.sequence + ordinal,
                    template.createdAt + ordinal,
                    NetworkControlCommand::PolicyUpdate {
                        policyId: "test.concurrent".into(),
                        value: ordinal.to_string(),
                    },
                );
                journal.appendUnobservedOperation(&operation).unwrap();
                let query = NetworkControlStore::new(host.clone()).unwrap();
                assert!(query.currentState().unwrap().initialized);
            }
        }));
    }
    start.wait();
    for writer in writers {
        writer.join().unwrap();
    }
    let state = control.currentState().unwrap();
    assert_eq!(state.policies["test.concurrent"], "40");
    let commands = control.currentSpaceOperations().unwrap();
    assert_eq!(
        control
            .validateSpacePolicy(&state.spaceId, &commands)
            .unwrap(),
        state
    );
}

#[test]
fn repeated_preference_snapshots_do_not_rewrite_prepared_files() {
    use crate::PreferencesDataStore::{stringPreferencesKey, PreferencesDataStore};
    let storage = Arc::new(MemoryStorageHost::default());
    let stores: Vec<_> = (0..20).map(|index| {
        let path = format!("runtime/plugin_data/space/com.operit.test/{index}.preferences.json");
        let store = PreferencesDataStore::newWithStorage(storage.clone(), &path)
            .withSchema(1, |_, _| Ok(()));
        store.edit(|values| values.set(&stringPreferencesKey("value"), json!(index))).unwrap();
        store
    }).collect();
    let snapshots: Vec<_> = stores.iter().map(|store| store.syncSnapshot().unwrap()).collect();
    let writes = storage.writeCount();
    for _ in 0..5 {
        for (store, expected) in stores.iter().zip(&snapshots) {
            assert_eq!(store.syncSnapshot().unwrap(), *expected);
        }
    }
    assert_eq!(storage.writeCount(), writes, "100 unchanged snapshots must perform zero file writes");
    let upgraded = stores[0].clone().withSchema(2, |_, values| {
        values.set(&stringPreferencesKey("upgraded"), json!(true));
        Ok(())
    });
    assert!(!upgraded.isSyncPrepared().unwrap());
    assert_eq!(upgraded.syncSnapshot().unwrap().data.values["upgraded"], json!(true));
    assert!(upgraded.isSyncPrepared().unwrap());
    assert!(storage.writeCount() > writes, "a real format migration must still be persisted");
}

#[test]
fn preference_replay_reads_only_its_completed_sql_target() {
    let storage = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(storage, "runtime/sync");
    let path = "runtime/plugin_data/space/com.operit.test/selected.preferences.json";
    let make = |sequence, path: &str| {
        let mut op = operationFromOrigin("peer", sequence, "preference", &serde_json::to_string(&(path, "value")).unwrap(),
            "set", json!({"storagePath":path,"key":"value","value":sequence,"encrypted":false,"formatVersion":1}));
        op.domain = "preferences".into();
        op
    };
    let selected = make(1, path);
    let mut completed = vec![selected.clone()];
    completed.extend((2..1002).map(|sequence| make(sequence, "runtime/plugin_data/space/com.operit.test/other.preferences.json")));
    store.appendOperations(&completed).unwrap();
    let pending = make(1002, path);
    store.receiveOperations(std::slice::from_ref(&pending)).unwrap();
    // An all-history scan would try to parse these unrelated headers and fail.
    store.withJournal(|journal| journal.transaction(|tx| {
        tx.execute("UPDATE originals SET header='not a JSON header' WHERE target<>?1", vec![crate::SqliteStore::SqliteValue::Text(path.into())])?;
        Ok(())
    })).unwrap();
    for _ in 0..3 {
        assert_eq!(store.archivedPreferenceHistory(path).unwrap(), vec![selected.clone()]);
    }
    store.completeOperations(std::slice::from_ref(&pending)).unwrap();
    assert_eq!(store.archivedPreferenceHistory(path).unwrap(), vec![selected, pending]);
}

#[test]
fn empty_preference_preparation_only_initializes_requested_owners() {
    use crate::{PreferenceMerge::{self, PreferenceMergeSchema, PreferenceMergeStore},
        PreferencesDataStore::{stringPreferencesKey, CoreNodeStateStore, PreferencesDataStore}};
    let storage = Arc::new(MemoryStorageHost::default());
    PreferencesDataStore::newWithStorage(storage.clone(), "runtime/plugin_data/space/com.operit.test/existing.preferences.json")
        .edit(|values| values.set(&stringPreferencesKey("value"), json!("existing"))).unwrap();
    let path = "runtime/config/preferences/requested.preferences.json";
    CoreNodeStateStore::newWithStorage(storage.clone(), path)
        .edit(|values| values.set(&stringPreferencesKey("value"), json!("requested"))).unwrap();
    let schema = PreferenceMergeSchema { stores: vec![PreferenceMergeStore::values(path, false,
        |storage, path| PreferencesDataStore::newWithStorage(storage, path))] };
    let outcomes = PreferenceMerge::applyIndependentOperations(storage.clone(), &schema, &[], &[], |_, _| true);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].paths, BTreeSet::from([path.into()]));
    assert!(outcomes[0].result.is_ok(), "{:?}", outcomes[0].result);
    let writes = storage.writeCount();
    assert!(PreferenceMerge::applyIndependentOperations(storage.clone(), &schema, &[], &[], |_, _| true).is_empty());
    assert_eq!(storage.writeCount(), writes);
}

/// Run explicitly to compare the previous all-header scan with indexed owner replay.
#[test]
#[ignore = "manual history query benchmark"]
fn benchmark_preference_owner_history_queries() {
    let store = SyncOperationStore::new(Arc::new(MemoryStorageHost::default()), "runtime/sync");
    let paths: Vec<_> = (0..45).map(|i| format!("runtime/plugin_data/space/com.operit.test/{i}.preferences.json")).collect();
    let operations: Vec<_> = (1..=15_389).map(|sequence| {
        let path = &paths[sequence as usize % paths.len()];
        let mut op = operationFromOrigin("peer", sequence, "preference", &serde_json::to_string(&(path, "value")).unwrap(),
            "set", json!({"storagePath":path,"key":"value","value":"x".repeat(256),"encrypted":false,"formatVersion":1}));
        op.domain = "preferences".into();
        op
    }).collect();
    store.appendOperations(&operations).unwrap();
    let start = std::time::Instant::now();
    let previous: Vec<Vec<SyncOperation>> = paths.iter().map(|path| {
        store.withJournal(|journal| journal.headers(Some(false))).unwrap().into_iter()
            .filter(|header| header.route.storage_path.as_deref() == Some(path))
            .map(|header| store.original(&header.op_id).unwrap()).collect()
    }).collect();
    let before = start.elapsed();
    let start = std::time::Instant::now();
    let indexed: Vec<_> = paths.iter().map(|path| store.archivedPreferenceHistory(path).unwrap()).collect();
    let after = start.elapsed();
    assert_eq!(indexed, previous);
    eprintln!("history replay: {} originals / {} owners; all-header scan={before:?}, indexed={after:?}, speedup={:.2}x",
        operations.len(), paths.len(), before.as_secs_f64() / after.as_secs_f64());
}

#[test]
fn empty_preference_preparation_replays_explicit_history_for_a_prepared_dynamic_owner() {
    use crate::{PreferenceMerge::{self, PreferenceMergeSchema},
        PreferencesDataStore::{stringPreferencesKey, PreferencesDataStore}};
    let source = Arc::new(MemoryStorageHost::default());
    let target = Arc::new(MemoryStorageHost::default());
    let path = "runtime/plugin_data/space/com.operit.test/history.preferences.json";
    let remote = PreferencesDataStore::newWithStorage(source.clone(), path);
    remote.edit(|values| values.set(&stringPreferencesKey("remote"), json!(true))).unwrap();
    let history = SyncOperationStore::new(source, "runtime/sync").preferenceHistory().unwrap();
    let local = PreferencesDataStore::newWithStorage(target.clone(), path);
    local.prepareSync().unwrap();
    // A caller explicitly supplying history still requests replay, even without incoming work.
    let outcomes = PreferenceMerge::applyIndependentOperations(target, &PreferenceMergeSchema::default(), &history, &[], |_, _| true);
    assert_eq!(outcomes.len(), 1);
    assert!(outcomes[0].result.is_ok(), "{:?}", outcomes[0].result);
    assert_eq!(local.data().unwrap().get(&stringPreferencesKey("remote")), Some(&json!(true)));
}
