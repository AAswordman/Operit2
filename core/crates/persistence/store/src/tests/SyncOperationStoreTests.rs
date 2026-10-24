use super::*;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};

use operit_host_api::{HostError, RuntimeStorageEntry};
use operit_util::RuntimeStorageLayout::{
    CONFIG_PREFERENCES_DIR_PATH, RUNTIME_WEBSESSION_BROWSER_BOOKMARKS_PATH,
};
use serde_json::{json, Value};

#[derive(Clone, Default)]
struct MemoryStorageHost {
    files: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    writeCount: Arc<AtomicUsize>,
    readCount: Arc<AtomicUsize>,
}

impl MemoryStorageHost {
    /// Returns the number of complete writes and appends served by this test host.
    fn writeCount(&self) -> usize {
        self.writeCount.load(Ordering::SeqCst)
    }
}

impl RuntimeStorageHost for MemoryStorageHost {
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
        self.readCount.fetch_add(1, Ordering::SeqCst);
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
        let content = self.readBytes(path)?;
        let start = usize::try_from(offset)
            .map_err(|_| HostError::new("runtime storage offset does not fit usize"))?;
        if start >= content.len() {
            return Ok(Vec::new());
        }
        let end = start
            .checked_add(length)
            .ok_or_else(|| HostError::new("runtime storage byte range overflows usize"))?
            .min(content.len());
        Ok(content[start..end].to_vec())
    }

    /// Atomically replaces one in-memory file.

    fn writeBytesAtomically(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
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
    let storage = Arc::new(MemoryStorageHost::default());
    let store = NetworkControlStore::new(storage.clone()).unwrap();
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
    assert_eq!(NetworkControlStore::new(storage).unwrap().currentState().unwrap(), state);
}

/// Upgrade an existing join-only reviewer's log, then persist a new admission and reopen both.
#[test]
fn legacy_space_admission_survives_upgrade_without_rewriting_history() {
    use crate::NetworkControlStore::{NetworkControlCommandRecord, NetworkControlIdentityAssignment, NetworkControlRole, NetworkControlStore};
    let storage = Arc::new(MemoryStorageHost::default());
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
    use crate::NetworkControlStore::{NetworkControlIdentityAssignment, NetworkControlRole, NetworkControlStore};
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
    for capability in ["chat.read", "chat.write"] {
        assert!(store.defineRole(NetworkControlRole { roleId: "retired-chat-role".into(),
            displayName: "Retired chat role".into(),
            capabilities: BTreeSet::from([capability.into()]),
        }).is_err());
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
    let started = std::time::Instant::now();
    for _ in 0..100 {
        assert_eq!(store.operationsSince(&SyncClock::empty(), &domains, usize::MAX).unwrap(), vec![policy.clone()]);
    }
    let indexed = started.elapsed();
    assert_eq!(storage.readCount.load(Ordering::SeqCst), reads);
    let started = std::time::Instant::now();
    for _ in 0..100 {
        let content = store.readOperationLog("peer").unwrap();
        let decoded = store.decodeOperationLog(&content).unwrap();
        assert_eq!(decoded.into_iter().filter(|operation| operation.domain == "network_control").count(), 1);
    }
    eprintln!("domain-query comparison: 100 queries, 5000 history records; indexed={indexed:?}, full_log_decode={:?}; warm_storage_reads=0", started.elapsed());
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
    assert_eq!(host.writeCount(), 3);

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
    for sequence in 1..=3 {
        store
            .appendUnobservedOperation(&operation(
                sequence,
                "message",
                &format!("m{sequence}"),
                "upsert",
                SyncOperationSemantics::EntityState,
                json!({"text": "x"}),
            ))
            .unwrap();
    }
    let path = "corrupt-journal/operations/device-a.jsonl";
    let original = storage.readBytes(path).unwrap();
    let lines: Vec<&[u8]> = original.split_inclusive(|byte| *byte == b'\n').collect();
    assert_eq!(lines.len(), 3);
    let mut corrupted = Vec::new();
    corrupted.extend_from_slice(lines[0]);
    corrupted.extend_from_slice(b"{ this is not json\n");
    corrupted.extend_from_slice(lines[2]);
    storage.writeBytes(path, &corrupted).unwrap();

    // A fresh instance rebuilds its index from the damaged journal and must
    // still export the two intact operations.
    let reopened = SyncOperationStore::new(storage.clone(), "corrupt-journal");
    let readable = reopened
        .operationsSince(&SyncClock::empty(), &[], 100)
        .unwrap();
    assert_eq!(sequences(&readable), vec![1, 3]);

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

    // Domain-filtered reads over the same journal also skip the damage.
    let domainRead = reopened
        .operationsSince(&SyncClock::empty(), &["chat".to_string()], 100)
        .unwrap();
    assert_eq!(sequences(&domainRead), vec![1, 3]);
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
    for sequence in 1..=2 {
        store
            .appendUnobservedOperation(&operation(
                sequence,
                "message",
                &format!("m{sequence}"),
                "upsert",
                SyncOperationSemantics::EntityState,
                json!({"text": "x"}),
            ))
            .unwrap();
    }
    let path = "quarantine-journal/operations/device-a.jsonl";
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
    assert!(
        store
            .operationsSince(&SyncClock::empty(), &[], 10)
            .unwrap()
            .is_empty()
    );
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
    let first = operation(
        1,
        "message",
        "first",
        "upsert",
        SyncOperationSemantics::EntityState,
        json!(1),
    );
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
    let bytes = host
        .readBytes("encrypted-receive/receive_state.json")
        .unwrap();
    assert!(
        !String::from_utf8(bytes)
            .unwrap()
            .contains("pending-secret-token")
    );
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

/// Completed SQL-owned operations retain content verification independently of their pending payloads.
#[test]
fn completed_receipt_rejects_changed_original_payload_after_restart() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = SyncOperationStore::new(host.clone(), "receipt-content");
    let original = operation(1, "message", "id", "upsert", SyncOperationSemantics::Transaction, json!({"content":"original"}));
    store.receiveOperations(&[original.clone()]).unwrap();
    store.completeOperations(&[original.clone()]).unwrap();
    let restarted = SyncOperationStore::new(Arc::new((*host).clone()), "receipt-content");
    restarted.receiveOperations(&[original.clone()]).unwrap();
    let mut changed = original; changed.payload = json!({"content":"changed"});
    assert!(restarted.receiveOperations(&[changed]).is_err());
    assert!(restarted.pendingOperations().unwrap().is_empty());
}

/// A retry after an interrupted append must remain a separate readable journal record.
#[test]
fn intact_operations_survive_retry_after_partial_journal_tail() {
    for batch in [false, true] {
        let host = Arc::new(MemoryStorageHost::default());
        let store = SyncOperationStore::new(host.clone(), "runtime/sync");
        let first = operation(1, "message", "one", "set", SyncOperationSemantics::Transaction, json!({"text":"one"}));
        let second = operation(2, "message", "two", "set", SyncOperationSemantics::Transaction, json!({"text":"two"}));
        let mut interrupted = serde_json::to_vec(&first).unwrap();
        interrupted.extend_from_slice(b"\n{\"opId\":");
        host.writeBytes(&store.operationsPath("device-a"), &interrupted).unwrap();
        // First record spans an invalid tail in the offset index, but its identity remains readable.
        store.appendOperations(std::slice::from_ref(&first)).unwrap();
        if batch {
            store.appendOperations(std::slice::from_ref(&second)).unwrap();
        } else {
            store.appendUnobservedOperation(&second).unwrap();
        }
        // A new Host instance rebuilds all caches from the persisted bytes.
        let restarted = SyncOperationStore::new(Arc::new((*host).clone()), "runtime/sync");
        assert_eq!(restarted.operationsSince(&SyncClock::empty(), &[], 100).unwrap(), vec![first, second]);
    }
}
