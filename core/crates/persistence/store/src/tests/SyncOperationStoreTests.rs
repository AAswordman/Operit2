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
}

impl MemoryStorageHost {
    /// Returns the number of complete writes and appends served by this test host.
    fn writeCount(&self) -> usize {
        self.writeCount.load(Ordering::SeqCst)
    }
}

impl RuntimeStorageHost for MemoryStorageHost {
    fn runtimeRootDir(&self) -> Option<std::path::PathBuf> {
        None
    }

    fn workspaceRootDir(&self) -> Option<std::path::PathBuf> {
        None
    }

    fn readBytes(&self, path: &str) -> operit_host_api::HostResult<Vec<u8>> {
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

    let operations = store
        .operationsSince(&SyncClock::empty(), &[], 10)
        .unwrap();
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
