use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use operit_host_api::{HostError, HostSecretStore, RuntimeStorageEntry, RuntimeStorageHost};
use serde_json::{json, Value};

use crate::PreferencesEncryption::tests::{
    loadOrCreateWithSecretStoreForTest, ENCRYPTION_HOST_SECRET_KEY_FOR_TEST,
};
use crate::SyncOperationStore::{SyncClock, SyncOperationStore};
use crate::CoreNodeIdentityStore::CoreNodeIdentityStore;
use operit_util::RuntimeStorageLayout::{
    CONFIG_PREFERENCES_DIR_PATH, GITHUB_AUTH_PREFERENCES_PATH, MODEL_CONFIGS_PREFERENCES_PATH,
    PREFERENCES_ENCRYPTION_KEY_PATH, RUNTIME_LINK_ACCESS_IDENTITY_PATH, RUNTIME_SYNC_DIR_PATH,
};

use super::{
    combine2, combine5, mutableStateFlow, stringPreferencesKey, CoreNodeSecretStore, Preferences,
    PreferencesDataStore, PreferencesDataStoreError, PreferencesSyncedEntry,
    PREFERENCES_SCHEMA_VERSION_KEY_NAME,
};

fn collected<T: Clone>(values: &Arc<Mutex<Vec<T>>>) -> Vec<T> {
    values
        .lock()
        .expect("test values mutex must not be poisoned")
        .clone()
}

/// Declares just the paths used by the calling fixture.
fn fixture_merge_schema(paths: &[&str]) -> crate::PreferenceMerge::PreferenceMergeSchema {
    crate::PreferenceMerge::PreferenceMergeSchema {
        stores: paths.iter().map(|path| crate::PreferenceMerge::PreferenceMergeStore::values(
            *path, false, |storage, path| PreferencesDataStore::newWithStorage(storage, path),
            |_| Ok(()),
        )).collect(),
    }
}

#[test]
fn combine2_emits_initial_and_updates_from_each_source() {
    let first = mutableStateFlow(1);
    let second = mutableStateFlow(10);
    let firstFlow = first.asStateFlow();
    let secondFlow = second.asStateFlow();
    let combined = combine2(&firstFlow, &secondFlow, |a, b| a + b);

    let values = Arc::new(Mutex::new(Vec::new()));
    let valuesForSubscription = Arc::clone(&values);
    let _subscription = combined.subscribe(move |value| {
        valuesForSubscription
            .lock()
            .expect("test values mutex must not be poisoned")
            .push(value);
    });

    assert_eq!(collected(&values), vec![11]);
    first.set_value(2);
    assert_eq!(collected(&values), vec![11, 12]);
    second.set_value(20);
    assert_eq!(collected(&values), vec![11, 12, 22]);
    second.set_value(20);
    assert_eq!(collected(&values), vec![11, 12, 22]);
}

/// Verifies that a mutable state-flow update emits only an actual state change.
#[test]
fn mutable_state_flow_update_publishes_one_changed_value() {
    let state = mutableStateFlow(vec![1]);
    let values = Arc::new(Mutex::new(Vec::new()));
    let valuesForSubscription = Arc::clone(&values);
    let _subscription = state.subscribe(move |value| {
        valuesForSubscription
            .lock()
            .expect("test values mutex must not be poisoned")
            .push(value);
    });

    state.update(|items| items.push(2));
    state.update(|_| {});

    assert_eq!(collected(&values), vec![vec![1], vec![1, 2]]);
}

#[test]
fn combine5_keeps_latest_values_from_all_sources() {
    let first = mutableStateFlow("a".to_string());
    let second = mutableStateFlow("b".to_string());
    let third = mutableStateFlow("c".to_string());
    let fourth = mutableStateFlow("d".to_string());
    let fifth = mutableStateFlow("e".to_string());
    let firstFlow = first.asStateFlow();
    let secondFlow = second.asStateFlow();
    let thirdFlow = third.asStateFlow();
    let fourthFlow = fourth.asStateFlow();
    let fifthFlow = fifth.asStateFlow();
    let combined = combine5(
        &firstFlow,
        &secondFlow,
        &thirdFlow,
        &fourthFlow,
        &fifthFlow,
        |a, b, c, d, e| format!("{a}{b}{c}{d}{e}"),
    );

    assert_eq!(combined.value(), "abcde");
    third.set_value("C".to_string());
    assert_eq!(combined.value(), "abCde");
    first.set_value("A".to_string());
    assert_eq!(combined.value(), "AbCde");
    fifth.set_value("E".to_string());
    assert_eq!(combined.value(), "AbCdE");
}

#[test]
fn derived_state_unsubscribes_from_sources_when_dropped() {
    let first = mutableStateFlow(1);
    let second = mutableStateFlow(2);
    let firstFlow = first.asStateFlow();
    let secondFlow = second.asStateFlow();
    let transformCount = Arc::new(Mutex::new(0));

    {
        let transformCountForCombine = Arc::clone(&transformCount);
        let combined = combine2(&firstFlow, &secondFlow, move |a, b| {
            *transformCountForCombine
                .lock()
                .expect("test transform count mutex must not be poisoned") += 1;
            a + b
        });
        assert_eq!(combined.value(), 3);
        assert_eq!(
            *transformCount
                .lock()
                .expect("test transform count mutex must not be poisoned"),
            1
        );
    }

    first.set_value(10);
    second.set_value(20);
    assert_eq!(
        *transformCount
            .lock()
            .expect("test transform count mutex must not be poisoned"),
        1
    );
}

#[derive(Clone, Default)]
struct MemoryStorageHost {
    releasePreferencesCache: bool,
    files: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    readCount: Arc<AtomicUsize>,
    writeCount: Arc<AtomicUsize>,
    blockedWrite: Arc<Mutex<Option<BlockedWrite>>>,
    failedWrites: Arc<Mutex<BTreeSet<String>>>,
    failedFlushes: Arc<Mutex<BTreeSet<String>>>,
    observedWrites: Arc<Mutex<Option<(String, Sender<Vec<u8>>)>>>,
}

struct BlockedWrite {
    started: Sender<()>,
    release: Receiver<()>,
}

impl MemoryStorageHost {
    /// Captures each committed snapshot for one file, including intermediate writes.
    fn observeWrites(&self, path: &str) -> Receiver<Vec<u8>> {
        let (sender, receiver) = channel();
        *self.observedWrites.lock().unwrap() = Some((path.to_string(), sender));
        receiver
    }

    /// Returns how many storage read operations this host has served.
    fn readCount(&self) -> usize {
        self.readCount.load(Ordering::SeqCst)
    }

    /// Returns how many storage write operations this host has served.
    fn writeCount(&self) -> usize {
        self.writeCount.load(Ordering::SeqCst)
    }

    /// Blocks the next storage write until the returned sender is signalled.
    fn blockNextWrite(&self) -> (Receiver<()>, Sender<()>) {
        let (startedSender, startedReceiver) = channel();
        let (releaseSender, releaseReceiver) = channel();
        let mut blockedWrite = self
            .blockedWrite
            .lock()
            .expect("test blocked write mutex must not be poisoned");
        *blockedWrite = Some(BlockedWrite {
            started: startedSender,
            release: releaseReceiver,
        });
        (startedReceiver, releaseSender)
    }
}

#[derive(Clone, Default)]
struct MemorySecretStore {
    secrets: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

impl RuntimeStorageHost for MemoryStorageHost {
    fn retainPreferencesCache(&self) -> bool { !self.releasePreferencesCache }
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
        if self.failedWrites.lock().unwrap().contains(path) {
            return Err(HostError::new("injected atomic write failure"));
        }
        self.writeCount.fetch_add(1, Ordering::SeqCst);
        let blockedWrite = self
            .blockedWrite
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?
            .take();
        if let Some(blockedWrite) = blockedWrite {
            blockedWrite
                .started
                .send(())
                .map_err(|error| HostError::new(error.to_string()))?;
            blockedWrite
                .release
                .recv()
                .map_err(|error| HostError::new(error.to_string()))?;
        }
        let mut files = self
            .files
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        files.insert(path.to_string(), content.to_vec());
        drop(files);
        if let Some((observedPath, sender)) = &*self.observedWrites.lock().unwrap() {
            if observedPath == path {
                let _ = sender.send(content.to_vec());
            }
        }
        Ok(())
    }

    /// Appends bytes to one in-memory preferences storage entry.
    fn appendBytes(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
        if self.failedWrites.lock().unwrap().contains(path) {
            return Err(HostError::new("injected journal write failure"));
        }
        self.writeCount.fetch_add(1, Ordering::SeqCst);
        self.files
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?
            .entry(path.to_string())
            .or_default()
            .extend_from_slice(content);
        Ok(())
    }

    fn flushFile(&self, path: &str) -> operit_host_api::HostResult<()> {
        if self.failedFlushes.lock().unwrap().contains(path) {
            return Err(HostError::new("injected journal flush failure"));
        }
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

impl HostSecretStore for MemorySecretStore {
    /// Reads secret bytes from the in-memory test secret store.
    fn readSecret(&self, key: &str) -> operit_host_api::HostResult<Option<Vec<u8>>> {
        let secrets = self
            .secrets
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(secrets.get(key).cloned())
    }

    /// Writes secret bytes into the in-memory test secret store.
    fn writeSecret(&self, key: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
        let mut secrets = self
            .secrets
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        secrets.insert(key.to_string(), content.to_vec());
        Ok(())
    }

    /// Deletes secret bytes from the in-memory test secret store.
    fn deleteSecret(&self, key: &str) -> operit_host_api::HostResult<()> {
        let mut secrets = self
            .secrets
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        secrets.remove(key);
        Ok(())
    }
}

#[test]
/// Verifies legacy preferences run each migration once and persist only the final snapshot.
fn preferences_schema_runs_sequential_migrations_atomically() {
    let host = Arc::new(MemoryStorageHost::default());
    let storagePath = format!("{CONFIG_PREFERENCES_DIR_PATH}/schema_migration.preferences.json");
    let store = PreferencesDataStore::newNodeLocalWithStorage(host.clone(), storagePath, false)
        .withSchema(2, |version, preferences| match version {
            0 => {
                preferences.set(&stringPreferencesKey("first"), "created".to_string());
                Ok(())
            }
            1 => {
                let first = preferences
                    .getTyped::<String>(&stringPreferencesKey("first"))
                    .unwrap()
                    .expect("version-one data");
                preferences.set(&stringPreferencesKey("second"), format!("{first}-migrated"));
                Ok(())
            }
            from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
        });

    let preferences = store.data().expect("schema migration");
    assert_eq!(
        preferences.get(&stringPreferencesKey("first")),
        Some(&json!("created"))
    );
    assert_eq!(
        preferences.get(&stringPreferencesKey("second")),
        Some(&json!("created-migrated"))
    );
    assert_eq!(
        preferences.get(&stringPreferencesKey(PREFERENCES_SCHEMA_VERSION_KEY_NAME)),
        Some(&json!(2))
    );
    store.data().expect("current schema read");
    assert_eq!(host.writeCount(), 1);
}

/// The first visible file must contain its complete record, even with a schema upgrade.
#[test]
fn schema_first_record_write_never_publishes_an_empty_record() {
    for replicated in [false, true] {
        for mode in ["replace", "edit", "migrate"] {
            let host = Arc::new(MemoryStorageHost::default());
            let path =
                format!("runtime/space/topology/atomic-{replicated}-{mode}.preferences.json");
            let snapshots = host.observeWrites(&path);
            let store = if replicated {
                PreferencesDataStore::newWithStorage(host.clone(), &path)
            } else {
                PreferencesDataStore::newNodeLocalWithStorage(host.clone(), &path, false)
            }
            .withSchema(1, |_, _| Ok(()));
            let record = json!({"nodeId":"node-local", "peers":[], "links":[], "updatedAt":1});
            match mode {
                "replace" => {
                    let mut preferences = Preferences::default();
                    preferences.set(&stringPreferencesKey("record"), record.clone());
                    store.replace(preferences).unwrap();
                }
                "edit" => store
                    .edit(|p| p.set(&stringPreferencesKey("record"), record.clone()))
                    .unwrap(),
                "migrate" => store
                    .migrate(|p| -> Result<(), PreferencesDataStoreError> {
                        p.set(&stringPreferencesKey("record"), record.clone());
                        Ok(())
                    })
                    .unwrap(),
                _ => unreachable!(),
            }
            let snapshots = snapshots.try_iter().collect::<Vec<_>>();
            assert!(!snapshots.is_empty());
            for snapshot in &snapshots {
                let value: Value = serde_json::from_slice(snapshot).unwrap();
                assert_eq!(
                    value.get("record"),
                    Some(&record),
                    "{mode} (replicated={replicated}) published an incomplete record: {value}"
                );
            }
            assert_eq!(
                snapshots.len(),
                1,
                "a complete record must have one publication point"
            );
            assert_eq!(
                store.data().unwrap().get(&stringPreferencesKey("record")),
                Some(&record)
            );
        }
    }
}

/// A fresh peer must publish each imported Space file with its record already present.
#[test]
fn space_preference_sync_publishes_only_complete_records() {
    use crate::CoreSpaceStore::CoreSpaceStore;
    use crate::PreferenceMerge::{applyIndependentOperations, PreferenceMergeSchema};
    for directory in ["members", "device_profiles", "device_presence", "topology"] {
        let host = Arc::new(MemoryStorageHost::default());
        let path = format!("runtime/space/{directory}/peer.preferences.json");
        let snapshots = host.observeWrites(&path);
        let record = json!({"nodeId":"peer", "updatedAt":1});
        let mut operation = nativePreferenceOperation(&path, vec![("record", record.clone())], 1);
        operation.payload["schemaVersion"] = json!(3);
        let schema = PreferenceMergeSchema {
            stores: vec![CoreSpaceStore::preferenceSyncStore(&path).unwrap()],
        };
        let outcomes = applyIndependentOperations(host.clone(), &schema, &[], &[operation], |_, _| true);
        assert!(outcomes.iter().all(|outcome| outcome.result.is_ok()));
        let snapshots = snapshots.try_iter().collect::<Vec<_>>();
        assert_eq!(snapshots.len(), 1, "{path} must have one complete publication");
        let persisted: Value = serde_json::from_slice(&snapshots[0]).unwrap();
        assert_eq!(persisted["record"], record);
        assert_eq!(persisted[PREFERENCES_SCHEMA_VERSION_KEY_NAME], 3);
    }
}

/// Version checks must not recreate an exited peer's deleted files as empty records.
#[test]
fn space_preference_sync_rejected_versions_leave_no_placeholder() {
    use crate::CoreSpaceStore::CoreSpaceStore;
    use crate::PreferenceMerge::{applyIndependentOperations, PreferenceMergeSchema};
    for directory in ["members", "device_profiles", "device_presence", "topology"] {
        for version in [None, Some(2), Some(4)] {
            let host = Arc::new(MemoryStorageHost::default());
            let path = format!("runtime/space/{directory}/peer.preferences.json");
            let snapshots = host.observeWrites(&path);
            let mut operation = nativePreferenceOperation(&path, vec![("record", json!({"nodeId":"peer"}))], 1);
            match version {
                Some(version) => { operation.payload["schemaVersion"] = json!(version); }
                None => { operation.payload.as_object_mut().unwrap().remove("schemaVersion"); }
            }
            let schema = PreferenceMergeSchema {
                stores: vec![CoreSpaceStore::preferenceSyncStore(&path).unwrap()],
            };
            let outcomes = applyIndependentOperations(host.clone(), &schema, &[], &[operation], |_, _| true);
            assert_eq!(outcomes.len(), 1);
            assert_eq!(outcomes[0].skipped, version != Some(4));
            assert_eq!(outcomes[0].result.is_ok(), version != Some(4));
            assert!(!host.exists(&path).unwrap(), "version {version:?} recreated {path}");
            assert_eq!(snapshots.try_iter().count(), 0);
        }
    }
}

/// Failed incoming replay must not commit the schema before the business record.
#[test]
fn space_preference_sync_failed_replay_leaves_no_placeholder() {
    use crate::PreferenceMerge::{applyIndependentOperations, PreferenceMergeSchema, PreferenceMergeStore};
    let host = Arc::new(MemoryStorageHost::default());
    let path = "runtime/space/members/peer.preferences.json";
    let schema = PreferenceMergeSchema {
        stores: vec![PreferenceMergeStore::values(
            path, false,
            |storage, path| PreferencesDataStore::newWithStorage(storage, path).withSchema(3, |_, _| Ok(())),
            |_| Err(PreferencesDataStoreError::Message("injected record decode failure".into())),
        )],
    };
    let mut operation = nativePreferenceOperation(path, vec![("record", json!({"nodeId":"peer"}))], 1);
    operation.payload["schemaVersion"] = json!(3);
    let outcomes = applyIndependentOperations(host.clone(), &schema, &[], &[operation], |_, _| true);
    assert_eq!(outcomes.len(), 1);
    assert!(outcomes[0].result.as_ref().unwrap_err().contains("injected record decode failure"));
    assert!(!host.exists(path).unwrap());
}

/// A schema change still commits when the business edit is a no-op, without exporting an operation.
#[test]
fn schema_upgrade_with_unchanged_record_still_commits_once() {
    for mode in ["replace", "edit", "migrate"] {
        let host = Arc::new(MemoryStorageHost::default());
        let path = format!("runtime/space/topology/schema-noop-{mode}.preferences.json");
        let record = json!({"nodeId":"local", "peers":[], "links":[], "updatedAt":1});
        host.writeBytes(
            &path,
            &serde_json::to_vec(&json!({
                PREFERENCES_SCHEMA_VERSION_KEY_NAME: 1, "record": record,
            }))
            .unwrap(),
        )
        .unwrap();
        let snapshots = host.observeWrites(&path);
        let store =
            PreferencesDataStore::newWithStorage(host.clone(), &path).withSchema(2, |_, _| Ok(()));
        match mode {
            "replace" => {
                let mut p = Preferences::default();
                p.set(&stringPreferencesKey("record"), record.clone());
                store.replace(p).unwrap();
            }
            "edit" => store.edit(|_| {}).unwrap(),
            "migrate" => store
                .migrate(|_| -> Result<(), PreferencesDataStoreError> { Ok(()) })
                .unwrap(),
            _ => unreachable!(),
        }
        assert_eq!(snapshots.try_iter().count(), 1);
        let persisted: Value = serde_json::from_slice(&host.readBytes(&path).unwrap()).unwrap();
        assert_eq!(persisted[PREFERENCES_SCHEMA_VERSION_KEY_NAME], 2);
        assert_eq!(persisted["record"], record);
        assert!(SyncOperationStore::new(host, RUNTIME_SYNC_DIR_PATH)
            .preferenceHistory()
            .unwrap()
            .is_empty());
    }
}

/// A failed business edit must not leave a schema-only file that breaks the next startup.
#[test]
fn schema_first_record_failed_edit_leaves_no_placeholder() {
    let host = Arc::new(MemoryStorageHost::default());
    let path = "runtime/space/topology/failed-record.preferences.json";
    let store =
        PreferencesDataStore::newWithStorage(host.clone(), path).withSchema(1, |_, _| Ok(()));
    let result = store.migrate(|p| -> Result<(), PreferencesDataStoreError> {
        p.set(&stringPreferencesKey("record"), json!({"nodeId":"local"}));
        Err(PreferencesDataStoreError::Message(
            "injected edit failure".into(),
        ))
    });
    assert!(result.is_err());
    assert!(
        !host.exists(path).unwrap(),
        "failed initialization must not publish a placeholder"
    );
}

#[test]
/// Verifies version-one identity records migrate whether they were stored as
/// a JSON string or as the object written by an intermediate build.
fn core_node_identity_migration_accepts_legacy_string_and_object_records() {
    let fixtures = [
        (
            json!("{\"deviceId\":\"core-legacy-string\"}"),
            "core-legacy-string",
            Value::Null,
        ),
        (
            json!({
                "deviceId": "core-legacy-object",
                "deviceInfo": {
                    "model": "legacy-model",
                    "platform": "linux"
                }
            }),
            "core-legacy-object",
            json!({"model": "legacy-model", "platform": "linux"}),
        ),
    ];
    for (record, expectedNodeId, expectedDeviceInfo) in fixtures {
        let host = Arc::new(MemoryStorageHost::default());
        let mut preferences = Preferences::default();
        preferences.set(
            &stringPreferencesKey(PREFERENCES_SCHEMA_VERSION_KEY_NAME),
            "1".to_string(),
        );
        preferences.set(&stringPreferencesKey("record"), record);
        host.writeBytes(
            RUNTIME_LINK_ACCESS_IDENTITY_PATH,
            &serde_json::to_vec_pretty(&preferences).expect("identity preferences serialization"),
        )
        .expect("identity preferences write");

        let identity = CoreNodeIdentityStore::new(host.clone())
            .initialize()
            .expect("legacy identity must migrate");
        assert_eq!(identity.nodeId, expectedNodeId);
        let stored: Value = serde_json::from_slice(
            &host
                .readBytes(RUNTIME_LINK_ACCESS_IDENTITY_PATH)
                .expect("migrated identity read"),
        )
        .expect("migrated identity JSON");
        assert_eq!(
            stored[PREFERENCES_SCHEMA_VERSION_KEY_NAME],
            json!(2),
            "migration must advance the schema version"
        );
        assert_eq!(stored["record"]["deviceId"], json!(expectedNodeId));
        if !expectedDeviceInfo.is_null() {
            assert_eq!(stored["record"]["deviceInfo"], expectedDeviceInfo);
        }
    }
}

#[test]
/// Verifies a failed migration leaves both storage and the cached snapshot unchanged.
fn preferences_schema_failure_does_not_persist_partial_state() {
    let host = Arc::new(MemoryStorageHost::default());
    let storagePath = format!("{CONFIG_PREFERENCES_DIR_PATH}/failed_schema.preferences.json");
    let store =
        PreferencesDataStore::newNodeLocalWithStorage(host.clone(), storagePath.clone(), false)
            .withSchema(1, |version, preferences| {
                if version != 0 {
                    return Err(PreferencesDataStoreError::MissingMigration {
                        from: version,
                        to: version + 1,
                    });
                }
                preferences.set(&stringPreferencesKey("partial"), "value".to_string());
                Err(PreferencesDataStoreError::Message(
                    "migration failed".to_string(),
                ))
            });

    assert!(matches!(
        store.data(),
        Err(PreferencesDataStoreError::Message(message)) if message == "migration failed"
    ));
    assert!(!host.exists(&storagePath).expect("storage existence"));
    assert_eq!(host.writeCount(), 0);
}

#[test]
/// Verifies a runtime refuses preferences written by a newer schema.
fn preferences_schema_rejects_newer_version() {
    let host = Arc::new(MemoryStorageHost::default());
    let storagePath = format!("{CONFIG_PREFERENCES_DIR_PATH}/newer_schema.preferences.json");
    let mut preferences = Preferences::default();
    preferences.set(
        &stringPreferencesKey(PREFERENCES_SCHEMA_VERSION_KEY_NAME),
        "3".to_string(),
    );
    host.writeBytes(
        &storagePath,
        &serde_json::to_vec_pretty(&preferences).expect("preferences serialization"),
    )
    .expect("preferences write");
    let store = PreferencesDataStore::newNodeLocalWithStorage(host.clone(), storagePath, false)
        .withSchema(2, |version, _| {
            Err(PreferencesDataStoreError::MissingMigration {
                from: version,
                to: version + 1,
            })
        });

    assert!(matches!(
        store.data(),
        Err(PreferencesDataStoreError::SchemaVersionTooNew {
            actual: 3,
            expected: 2
        })
    ));
    assert_eq!(host.writeCount(), 1);
}

#[test]
/// Verifies that legacy encrypted preferences keys move into host secrets.
fn preferences_encryption_migrates_old_secure_key_into_host_secret_store() {
    let host = MemoryStorageHost::default();
    let secretStore = MemorySecretStore::default();
    let legacyKey = br#"{
  "format": "operit.preferences.encryption.key",
  "version": 1,
  "algorithm": "XChaCha20Poly1305",
  "keyId": "legacy-key-id",
  "key": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
}"#;

    host.writeBytes(PREFERENCES_ENCRYPTION_KEY_PATH, legacyKey)
        .expect("legacy key write");

    let encryption = loadOrCreateWithSecretStoreForTest(&host, Some(&secretStore))
        .expect("migrated encryption key");
    let migrationPath = format!("{CONFIG_PREFERENCES_DIR_PATH}/migration_test.json");
    let encrypted = encryption
        .encrypt(&migrationPath, b"secret preferences")
        .expect("encrypted bytes");
    let decrypted = encryption
        .decrypt(&migrationPath, &encrypted)
        .expect("decrypted bytes");

    assert_eq!(decrypted, b"secret preferences");
    assert_eq!(
        secretStore
            .readSecret(ENCRYPTION_HOST_SECRET_KEY_FOR_TEST)
            .expect("host secret read"),
        Some(legacyKey.to_vec())
    );
    assert_eq!(
        host.exists(PREFERENCES_ENCRYPTION_KEY_PATH)
            .expect("legacy key exists check"),
        false
    );
}

#[test]
/// Verifies that stores with equal virtual paths stay isolated across storage hosts.
fn preference_stores_isolate_shared_paths_by_storage_host() {
    let firstHost = Arc::new(MemoryStorageHost::default());
    let secondHost = Arc::new(MemoryStorageHost::default());
    let storagePath = format!("{CONFIG_PREFERENCES_DIR_PATH}/shared_path.preferences.json");
    let firstStore = PreferencesDataStore::newWithStorage(firstHost, storagePath.clone());
    let secondStore = PreferencesDataStore::newWithStorage(secondHost, storagePath);

    let mut firstPreferences = Preferences::default();
    firstPreferences.set(
        &stringPreferencesKey("access_token"),
        "first-host-token".to_string(),
    );
    firstStore
        .replace(firstPreferences)
        .expect("first host preferences write");

    assert_eq!(
        secondStore
            .data()
            .expect("second host preferences read")
            .get(&stringPreferencesKey("access_token")),
        None
    );

    let mut secondPreferences = Preferences::default();
    secondPreferences.set(
        &stringPreferencesKey("access_token"),
        "second-host-token".to_string(),
    );
    secondStore
        .replace(secondPreferences)
        .expect("second host preferences write");

    assert_eq!(
        firstStore
            .data()
            .expect("first host preferences read")
            .get(&stringPreferencesKey("access_token")),
        Some(&json!("first-host-token"))
    );
    assert_eq!(
        secondStore
            .data()
            .expect("second host preferences read")
            .get(&stringPreferencesKey("access_token")),
        Some(&json!("second-host-token"))
    );
}

#[test]
fn encrypted_store_round_trips_without_plaintext_file() {
    let host = Arc::new(MemoryStorageHost::default());
    let store =
        PreferencesDataStore::newEncryptedWithStorage(host.clone(), GITHUB_AUTH_PREFERENCES_PATH);

    let mut preferences = Preferences::default();
    preferences.set(
        &stringPreferencesKey("access_token"),
        "secret-token".to_string(),
    );
    preferences.set(&stringPreferencesKey("token_type"), "bearer".to_string());
    preferences.set(
        &stringPreferencesKey("user_info"),
        "{\"login\":\"codex\"}".to_string(),
    );

    store.replace(preferences.clone()).expect("store write");

    let stored = host
        .readBytes(GITHUB_AUTH_PREFERENCES_PATH)
        .expect("encrypted file");
    let storedJson: Value = serde_json::from_slice(&stored).expect("encrypted envelope");
    assert_eq!(storedJson["format"], "operit.preferences.encrypted");
    assert_eq!(storedJson["version"], 1);
    assert_eq!(storedJson["algorithm"], "XChaCha20Poly1305");
    assert_eq!(storedJson["keyId"].is_string(), true);
    assert_eq!(storedJson["nonce"].is_string(), true);
    assert_eq!(storedJson["ciphertext"].is_string(), true);
    assert!(storedJson.get("access_token").is_none());
    assert!(storedJson.get("token_type").is_none());
    assert!(storedJson.get("user_info").is_none());

    let roundTrip = store.data().expect("decrypted store");
    assert_eq!(
        roundTrip.get(&stringPreferencesKey("access_token")),
        Some(&json!("secret-token"))
    );
    assert_eq!(
        roundTrip.get(&stringPreferencesKey("token_type")),
        Some(&json!("bearer"))
    );
    assert_eq!(
        roundTrip.get(&stringPreferencesKey("user_info")),
        Some(&json!("{\"login\":\"codex\"}"))
    );
}

#[test]
/// Verifies encrypted stores migrate legacy plaintext preference maps in place.
fn encrypted_store_migrates_legacy_plaintext_preferences() {
    let host = Arc::new(MemoryStorageHost::default());
    let storagePath = format!("{CONFIG_PREFERENCES_DIR_PATH}/legacy_encrypted.preferences.json");
    let mut legacyPreferences = Preferences::default();
    legacyPreferences.set(
        &stringPreferencesKey("provider_list"),
        "[\"DEEPSEEK\"]".to_string(),
    );
    legacyPreferences.set(
        &stringPreferencesKey("provider_DEEPSEEK"),
        "{\"apiKey\":\"secret\"}".to_string(),
    );
    let plaintext = serde_json::to_vec_pretty(&legacyPreferences)
        .expect("legacy plaintext preferences serialization");
    host.writeBytes(&storagePath, &plaintext)
        .expect("legacy plaintext preferences write");

    let store = PreferencesDataStore::newEncryptedWithStorage(host.clone(), storagePath.clone());
    let loaded = store.data().expect("migrated preferences read");

    assert_eq!(loaded, legacyPreferences);
    let stored = host
        .readBytes(&storagePath)
        .expect("migrated encrypted preferences");
    let storedJson: Value =
        serde_json::from_slice(&stored).expect("migrated encrypted preferences envelope");
    assert_eq!(storedJson["format"], "operit.preferences.encrypted");
    assert!(String::from_utf8(stored)
        .expect("migrated encrypted preferences utf8")
        .find("DEEPSEEK")
        .is_none());
}

#[test]
/// Verifies CoreNode secret stores never write Space synchronization operations.
fn core_node_secret_store_does_not_record_sync_operations() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = CoreNodeSecretStore::newWithStorage(host.clone(), GITHUB_AUTH_PREFERENCES_PATH);

    let mut preferences = Preferences::default();
    preferences.set(
        &stringPreferencesKey("access_token"),
        "github-secret-token".to_string(),
    );

    store.replace(preferences).expect("store write");

    assert_eq!(
        host.list(RUNTIME_SYNC_DIR_PATH)
            .expect("sync directory list")
            .len(),
        0
    );
}

#[test]
/// Verifies encrypted Space stores hide secrets in files and sync logs.
fn encrypted_space_store_records_key_operations_without_plaintext_log() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = PreferencesDataStore::newEncryptedWithStorage(
        host.clone(),
        MODEL_CONFIGS_PREFERENCES_PATH,
    );

    let mut preferences = Preferences::default();
    preferences.set(
        &stringPreferencesKey("api_key"),
        "sk-model-secret".to_string(),
    );
    preferences.set(
        &stringPreferencesKey("provider_list"),
        "[\"DEEPSEEK\"]".to_string(),
    );

    store.replace(preferences).expect("store write");

    let storedPreferences = host
        .readBytes(MODEL_CONFIGS_PREFERENCES_PATH)
        .expect("encrypted preferences file");
    let storedPreferencesJson: Value =
        serde_json::from_slice(&storedPreferences).expect("encrypted preferences envelope");
    assert_eq!(
        storedPreferencesJson["format"],
        "operit.preferences.encrypted"
    );
    assert!(String::from_utf8(storedPreferences)
        .expect("encrypted preferences utf8")
        .find("sk-model-secret")
        .is_none());

    SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH)
        .publishPreferenceOutboxes()
        .unwrap();
    let operationEntries = host
        .list(&format!("{RUNTIME_SYNC_DIR_PATH}/operations"))
        .expect("operation directory list");
    assert_eq!(operationEntries.len(), 1);
    let operationLog = String::from_utf8(
        host.readBytes(&operationEntries[0].path)
            .expect("operation log"),
    )
    .expect("operation log utf8");
    assert!(operationLog.find("sk-model-secret").is_none());
    assert!(operationLog.find("operit.sync.encrypted_payload").is_some());

    let operations = SyncOperationStore::new(host, RUNTIME_SYNC_DIR_PATH)
        .operationsSince(&SyncClock::empty(), &["preferences".to_string()], 10)
        .expect("decoded operations");
    assert_eq!(operations.len(), 1);
    let transaction = &operations[0];
    assert_eq!(transaction.operation, "transaction");
    let apiKeyEntry = transaction.payload["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["key"] == "api_key")
        .unwrap();
    assert_eq!(apiKeyEntry["operation"], "set");
    assert_eq!(apiKeyEntry["value"], "sk-model-secret");
    assert_eq!(transaction.payload["encrypted"], true);
}

#[test]
/// Verifies concurrent mutations of different keys converge without snapshot replacement.
fn synchronized_preference_keys_merge_independently() {
    let firstHost = Arc::new(MemoryStorageHost::default());
    let secondHost = Arc::new(MemoryStorageHost::default());
    let targetHost = Arc::new(MemoryStorageHost::default());
    let storagePath = format!("{CONFIG_PREFERENCES_DIR_PATH}/key_merge.preferences.json");
    let firstStore = PreferencesDataStore::newWithStorage(firstHost.clone(), storagePath.clone());
    let secondStore = PreferencesDataStore::newWithStorage(secondHost.clone(), storagePath.clone());

    firstStore
        .edit(|preferences| {
            preferences.set(&stringPreferencesKey("theme"), "dark".to_string());
        })
        .expect("first key edit");
    secondStore
        .edit(|preferences| {
            preferences.set(&stringPreferencesKey("language"), "zh-CN".to_string());
        })
        .expect("second key edit");

    let mut operations = SyncOperationStore::new(firstHost, RUNTIME_SYNC_DIR_PATH)
        .operationsSince(&SyncClock::empty(), &["preferences".to_string()], 10)
        .expect("first host operations");
    operations.extend(
        SyncOperationStore::new(secondHost, RUNTIME_SYNC_DIR_PATH)
            .operationsSince(&SyncClock::empty(), &["preferences".to_string()], 10)
            .expect("second host operations"),
    );

    crate::PreferenceMerge::applyOperations(targetHost.clone(), &fixture_merge_schema(&[&storagePath]), &[], &operations, |_, _| true)
        .expect("preference operation application");

    let targetStore = PreferencesDataStore::newWithStorage(targetHost, storagePath);
    let preferences = targetStore.data().expect("merged target preferences");
    assert_eq!(
        preferences.get(&stringPreferencesKey("theme")),
        Some(&json!("dark"))
    );
    assert_eq!(
        preferences.get(&stringPreferencesKey("language")),
        Some(&json!("zh-CN"))
    );
}

#[test]
/// Verifies complete entity synchronization preserves nulls and concurrent distinct records.
fn whole_json_preferences_sync_independent_records() {
    let firstHost = Arc::new(MemoryStorageHost::default());
    let secondHost = Arc::new(MemoryStorageHost::default());
    let targetHost = Arc::new(MemoryStorageHost::default());
    let path = format!("{CONFIG_PREFERENCES_DIR_PATH}/whole_record_sync.preferences.json");
    let firstStore = PreferencesDataStore::newWithStorage(firstHost.clone(), path.clone());
    let secondStore = PreferencesDataStore::newWithStorage(secondHost.clone(), path.clone());
    let targetStore = PreferencesDataStore::newWithStorage(targetHost.clone(), path.clone());
    let first = json!({"id":"A", "name":"First", "models":[{"id":"m", "optional":null}]});
    let second = json!({"id":"B", "name":"Second", "models":[]});
    firstStore
        .edit(|preferences| {
            preferences.set(&stringPreferencesKey("provider_A"), first.clone());
        })
        .unwrap();
    secondStore
        .edit(|preferences| {
            preferences.set(&stringPreferencesKey("provider_B"), second.clone());
            preferences.set(&stringPreferencesKey("nullable"), Value::Null);
        })
        .unwrap();
    let mut operations = SyncOperationStore::new(firstHost, RUNTIME_SYNC_DIR_PATH)
        .operationsSince(&SyncClock::empty(), &["preferences".to_owned()], 512)
        .unwrap();
    operations.extend(
        SyncOperationStore::new(secondHost, RUNTIME_SYNC_DIR_PATH)
            .operationsSince(&SyncClock::empty(), &["preferences".to_owned()], 512)
            .unwrap(),
    );
    assert_eq!(operations.len(), 2);
    for operation in &operations {
        assert!(operation.payload.get("jsonMutation").is_none());
        assert!(operation.payload.get("jsonPath").is_none());
    }

    crate::PreferenceMerge::applyOperations(targetHost, &fixture_merge_schema(&[&path]), &[], &operations, |_, _| true).unwrap();
    let preferences = targetStore.data().unwrap();
    assert_eq!(
        preferences.get(&stringPreferencesKey("provider_A")),
        Some(&first)
    );
    assert_eq!(
        preferences.get(&stringPreferencesKey("provider_B")),
        Some(&second)
    );
    assert_eq!(
        preferences.get(&stringPreferencesKey("nullable")),
        Some(&Value::Null)
    );
}

#[test]
fn stores_with_same_path_share_latest_in_memory_preferences() {
    let host = Arc::new(MemoryStorageHost::default());
    let storagePath = format!("{CONFIG_PREFERENCES_DIR_PATH}/shared_state_test.preferences.json");
    let first = PreferencesDataStore::newWithStorage(host.clone(), storagePath.clone());
    let second = PreferencesDataStore::newWithStorage(host, storagePath);

    first
        .edit(|preferences| {
            preferences.set(&stringPreferencesKey("api_key"), "sk-test".to_string());
        })
        .expect("first edit");

    second
        .edit(|preferences| {
            preferences.set(
                &stringPreferencesKey("provider_list"),
                "[\"DEEPSEEK\"]".to_string(),
            );
        })
        .expect("second edit");

    let preferences = first.data().expect("preferences");
    assert_eq!(
        preferences.get(&stringPreferencesKey("api_key")),
        Some(&json!("sk-test"))
    );
    assert_eq!(
        preferences.get(&stringPreferencesKey("provider_list")),
        Some(&json!("[\"DEEPSEEK\"]"))
    );
}

#[test]
/// Verifies that a later store instance reuses the process-wide preference snapshot.
fn transient_stores_reuse_cached_preferences_without_another_storage_read() {
    let host = Arc::new(MemoryStorageHost::default());
    let storagePath =
        format!("{CONFIG_PREFERENCES_DIR_PATH}/transient_store_cache.preferences.json");
    let mut preferences = Preferences::default();
    preferences.set(
        &stringPreferencesKey("provider_list"),
        "[\"DEEPSEEK\"]".to_string(),
    );
    host.writeBytes(
        &storagePath,
        &serde_json::to_vec_pretty(&preferences).expect("preferences serialization"),
    )
    .expect("preferences write");

    {
        let store = PreferencesDataStore::newWithStorage(host.clone(), storagePath.clone());
        assert_eq!(store.data().expect("first preferences read"), preferences);
    }

    let laterStore = PreferencesDataStore::newWithStorage(host.clone(), storagePath);
    assert_eq!(
        laterStore.data().expect("cached preferences read"),
        preferences
    );
    assert_eq!(host.readCount(), 1);
}

#[test]
/// Verifies that reads continue from the last committed snapshot while an edit persists.
fn reads_do_not_wait_for_an_in_progress_preferences_write() {
    let host = Arc::new(MemoryStorageHost::default());
    let storagePath = format!("{CONFIG_PREFERENCES_DIR_PATH}/read_during_write.preferences.json");
    let store = PreferencesDataStore::newWithStorage(host.clone(), storagePath);
    let mut initialPreferences = Preferences::default();
    initialPreferences.set(&stringPreferencesKey("theme"), "light".to_string());
    store
        .replace(initialPreferences.clone())
        .expect("initial preferences write");

    let (writeStarted, releaseWrite) = host.blockNextWrite();
    let storeForEdit = store.clone();
    let edit = thread::spawn(move || {
        storeForEdit.edit(|preferences| {
            preferences.set(&stringPreferencesKey("theme"), "dark".to_string());
        })
    });
    writeStarted
        .recv_timeout(Duration::from_secs(1))
        .expect("preferences write must start");

    let storeForRead = store.clone();
    let (readSender, readReceiver) = channel();
    let read = thread::spawn(move || {
        let _ = readSender.send(storeForRead.data());
    });
    let duringWrite = readReceiver.recv_timeout(Duration::from_millis(100));

    releaseWrite.send(()).expect("preferences write release");
    edit.join().expect("edit thread join").expect("edit result");
    read.join().expect("read thread join");

    let preferences = duringWrite
        .expect("data must not wait for the storage write")
        .expect("data result");
    assert_eq!(
        preferences.get(&stringPreferencesKey("theme")),
        Some(&json!("light"))
    );
    assert_eq!(
        store
            .data()
            .expect("updated preferences read")
            .get(&stringPreferencesKey("theme")),
        Some(&json!("dark"))
    );
}

#[test]
/// Verifies that an unchanged edit does not persist another preference snapshot.
fn unchanged_edit_skips_storage_persistence() {
    let host = Arc::new(MemoryStorageHost::default());
    let storagePath = format!("{CONFIG_PREFERENCES_DIR_PATH}/unchanged_edit.preferences.json");
    let store = PreferencesDataStore::newWithStorage(host.clone(), storagePath);

    store
        .edit(|preferences| {
            preferences.set(&stringPreferencesKey("theme"), "light".to_string());
        })
        .expect("initial preferences edit");
    let initialWriteCount = host.writeCount();

    store
        .edit(|preferences| {
            preferences.set(&stringPreferencesKey("theme"), "light".to_string());
        })
        .expect("unchanged preferences edit");
    assert_eq!(host.writeCount(), initialWriteCount);
}

/// A derived sharing job follows its owner's lifetime, not its update callback.
#[test]
fn state_flow_upstream_collection_cancels_only_after_last_owner() {
    let cancellation = super::FlowCancellation::new();
    let state = mutableStateFlow(1).asStateFlow();
    let collect = state.upstreamCollector(cancellation.clone());
    let retained = state.clone();
    drop(state);
    assert!(!cancellation.isCancelled());
    assert!(collect(2));
    assert_eq!(retained.value(), 2);
    drop(retained);
    assert!(cancellation.isCancelled());
    assert!(!collect(3));
}

/// stateIn's observation callback must not retain its own resulting StateFlow.
#[test]
fn state_in_releases_observation_on_last_state_drop() {
    let observationCancelled = super::FlowCancellation::new();
    let cancelledForObservation = observationCancelled.clone();
    let callbacks = Arc::new(Mutex::new(Vec::new()));
    let callbacksForObservation = callbacks.clone();
    let source = super::Flow::newObservedWithObservation(
        || Ok(7),
        |_| false,
        super::FlowObservation {
            subscribe: Arc::new(move |callback| {
                callbacksForObservation.lock().unwrap().push(callback);
                super::FlowObservationSubscription {
                    _guard: Box::new(super::FlowSubscription {
                        cancellation: cancelledForObservation.clone(),
                        _observation: None,
                    }),
                }
            }),
        },
    );
    let state = source.stateIn(super::CoroutineScope, super::SharingStarted::Lazily, 0);
    assert_eq!(state.value(), 7);
    let retained = state.clone();
    drop(state);
    assert!(!observationCancelled.isCancelled());
    drop(retained);
    assert!(observationCancelled.isCancelled());
    // A late notification from the old source is harmless.
    for callback in callbacks.lock().unwrap().iter() {
        callback();
    }
}

#[test]
/// A configured source may edit and then remove an unused model before a new
/// device joins. Compaction must not export a leaf edit without its model.
fn structured_model_sync_after_editing_and_deleting_unused_model() {
    let sourceHost = Arc::new(MemoryStorageHost::default());
    let targetHost = Arc::new(MemoryStorageHost::default());
    let providerKey = stringPreferencesKey("provider_DEEPSEEK");
    let baseline = json!({
        "id": "DEEPSEEK",
        "models": [{"id": "deepseek-v4-flash", "capabilitiesOverride": {"toolCall": true}}]
    });
    let sourceStore =
        PreferencesDataStore::newWithStorage(sourceHost.clone(), MODEL_CONFIGS_PREFERENCES_PATH);
    let targetStore =
        PreferencesDataStore::newWithStorage(targetHost.clone(), MODEL_CONFIGS_PREFERENCES_PATH);
    // Managers create their baseline in a local, non-exported schema migration.
    for store in [&sourceStore, &targetStore] {
        store
            .migrate(|preferences| {
                preferences.set(&providerKey, baseline.clone());
                Ok::<(), PreferencesDataStoreError>(())
            })
            .expect("initialize local defaults");
    }
    let write = |provider: &Value| {
        sourceStore
            .edit(|preferences| {
                preferences.set(&providerKey, provider.clone());
            })
            .expect("edit source provider");
    };
    let mut provider = baseline.clone();
    provider["models"].as_array_mut().unwrap().push(json!({
        "id": "unused-model", "capabilitiesOverride": {"toolCall": false}
    }));
    write(&provider);
    provider["models"][1]["capabilitiesOverride"]["toolCall"] = json!(true);
    write(&provider);
    provider["models"].as_array_mut().unwrap().remove(1);
    write(&provider);
    provider["models"].as_array_mut().unwrap().push(json!({
        "id": "deepseek-flash", "capabilitiesOverride": {"toolCall": true}
    }));
    write(&provider);

    let operations = SyncOperationStore::new(sourceHost, RUNTIME_SYNC_DIR_PATH)
        .operationsSince(&SyncClock::empty(), &["preferences".to_string()], 512)
        .expect("export initial model synchronization");

    crate::PreferenceMerge::applyOperations(targetHost, &fixture_merge_schema(&[MODEL_CONFIGS_PREFERENCES_PATH]), &[], &operations, |_, _| true)
        .expect("new device must receive the configured model");
    let preferences = targetStore.data().expect("synced provider");
    let actual = preferences.get(&providerKey).unwrap().clone();
    assert_eq!(actual, provider);
}

/// Commits independent bindings through the production path despite a malformed model dataset.
#[test]
fn preference_sync_keeps_independent_binding_progress_when_model_file_fails() {
    use operit_util::RuntimeStorageLayout::FUNCTIONAL_CONFIGS_PREFERENCES_PATH;
    let sourceHost = Arc::new(MemoryStorageHost::default());
    let targetHost = Arc::new(MemoryStorageHost::default());
    let bindingKey = stringPreferencesKey("function_model_binding");
    let providerKey = stringPreferencesKey("provider_DEEPSEEK");
    let sourceBindings = PreferencesDataStore::newWithStorage(
        sourceHost.clone(),
        FUNCTIONAL_CONFIGS_PREFERENCES_PATH,
    );
    let sourceModels = PreferencesDataStore::newWithStorage(
        sourceHost.clone(),
        MODEL_CONFIGS_PREFERENCES_PATH,
    );
    let targetBindings = PreferencesDataStore::newWithStorage(
        targetHost.clone(),
        FUNCTIONAL_CONFIGS_PREFERENCES_PATH,
    );
    let originalBinding =
        json!({"CHAT": {"providerId": "DEEPSEEK", "modelId": "deepseek-v4-flash"}});
    let newBinding = json!({"CHAT": {"providerId": "DEEPSEEK", "modelId": "deepseek-flash"}});
    targetBindings
        .migrate(|preferences| {
            preferences.set(&bindingKey, originalBinding.clone());
            Ok::<(), PreferencesDataStoreError>(())
        })
        .unwrap();
    sourceModels
        .migrate(|preferences| {
            preferences.set(
                &providerKey,
                json!({"id": "DEEPSEEK", "models": []}).to_string(),
            );
            Ok::<(), PreferencesDataStoreError>(())
        })
        .unwrap();
    sourceBindings
        .edit(|preferences| {
            preferences.set(&bindingKey, newBinding.clone());
        })
        .unwrap();
    sourceModels
        .edit(|preferences| {
            preferences.set(
                &providerKey,
                json!({"id": "DEEPSEEK", "models": [{"id": "deepseek-flash"}]}),
            );
        })
        .unwrap();
    let operations = SyncOperationStore::new(sourceHost, RUNTIME_SYNC_DIR_PATH)
        .operationsSince(&SyncClock::empty(), &["preferences".to_string()], 512)
        .unwrap();

    targetHost
        .writeBytes(MODEL_CONFIGS_PREFERENCES_PATH, b"{")
        .unwrap();
    crate::PreferenceMerge::applyOperations(
        targetHost.clone(), &fixture_merge_schema(&[MODEL_CONFIGS_PREFERENCES_PATH, operit_util::RuntimeStorageLayout::FUNCTIONAL_CONFIGS_PREFERENCES_PATH]),
        &[], &operations, |_, _| true,
    ).expect_err("a malformed dataset must report its own failure");
    assert_eq!(targetHost.readBytes(MODEL_CONFIGS_PREFERENCES_PATH).unwrap(), b"{");
    assert_eq!(
        targetBindings.data().unwrap().get(&bindingKey),
        Some(&newBinding)
    );
}

/// Executes preference observation immediately so a test can verify that no
/// callback sees a partially published multi-file synchronization batch.
pub(super) struct ImmediatePreferenceObserverScheduler;

impl operit_host_api::HostRuntimeTaskSchedulerHost for ImmediatePreferenceObserverScheduler {
    fn monotonicTimeMillis(&self) -> operit_host_api::HostResult<u64> {
        Ok(0)
    }

    fn scheduleHostRuntimeTask(
        &self,
        _: &str,
        task: operit_host_api::HostRuntimeTask,
    ) -> operit_host_api::HostResult<()> {
        task();
        Ok(())
    }

    fn scheduleHostRuntimeAsyncTask(
        &self,
        _: &str,
        _: operit_host_api::HostRuntimeAsyncTask,
    ) -> operit_host_api::HostResult<()> {
        Err(HostError::new(
            "async tasks are not used by preference observer tests",
        ))
    }

    fn scheduleHostRuntimeCooperativeAsyncTask(
        &self,
        _: &str,
        _: operit_host_api::HostRuntimeAsyncTask,
    ) -> operit_host_api::HostResult<()> {
        Err(HostError::new(
            "cooperative async tasks are not used by preference observer tests",
        ))
    }

    fn scheduleDelayedHostRuntimeTask(
        &self,
        _: &str,
        _: u64,
        _: operit_host_api::HostRuntimeTask,
    ) -> operit_host_api::HostResult<()> {
        Err(HostError::new(
            "delayed tasks are not used by preference observer tests",
        ))
    }

    fn waitForHostRuntimeTaskTurn(&self) -> operit_host_api::HostRuntimeTurnFuture {
        Box::pin(async { Ok(()) })
    }

    fn waitForHostRuntimeDelay(&self, _: u64) -> operit_host_api::HostRuntimeTurnFuture {
        Box::pin(async {
            Err(HostError::new(
                "timers are not used by preference observer tests",
            ))
        })
    }
}

/// Repeated delivery uses the same materializer and does not write unchanged datasets.
#[test]
fn preference_sync_replay_skips_unchanged_files_and_notifications() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = PreferencesDataStore::newWithStorage(host.clone(), MODEL_CONFIGS_PREFERENCES_PATH);
    let operations = [nativePreferenceOperation(MODEL_CONFIGS_PREFERENCES_PATH,
        vec![("setting", json!("synced-value"))], 1)];
    let schema = fixture_merge_schema(&[MODEL_CONFIGS_PREFERENCES_PATH]);
    crate::PreferenceMerge::applyOperations(host.clone(), &schema, &[], &operations, |_, _| true).unwrap();
    let writes = host.writeCount();
    let version = *store.changeSignal.version.lock().unwrap();
    crate::PreferenceMerge::applyOperations(host.clone(), &schema, &[], &operations, |_, _| true).unwrap();
    crate::PreferenceMerge::applyOperations(host.clone(), &schema, &[], &[], |_, _| true).unwrap();
    assert_eq!(host.writeCount(), writes);
    assert_eq!(*store.changeSignal.version.lock().unwrap(), version);
}

/// An encryption mismatch rejects that operation while the declared owner can still commit.
#[test]
fn preference_sync_isolates_an_operation_with_conflicting_encryption() {
    let host = Arc::new(MemoryStorageHost::default());
    let plain = nativePreferenceOperation(MODEL_CONFIGS_PREFERENCES_PATH,
        vec![("setting", json!("synced-value"))], 1);
    let mut encrypted = nativePreferenceOperation(MODEL_CONFIGS_PREFERENCES_PATH,
        vec![("setting", json!("synced-value"))], 2);
    encrypted.payload["encrypted"] = json!(true);
    crate::PreferenceMerge::applyOperations(host.clone(), &fixture_merge_schema(&[MODEL_CONFIGS_PREFERENCES_PATH]),
        &[], &[plain, encrypted], |_, _| true).expect_err("a preferences file must have one encryption policy");
    assert_eq!(PreferencesDataStore::newWithStorage(host, MODEL_CONFIGS_PREFERENCES_PATH)
        .data().unwrap().get(&stringPreferencesKey("setting")), Some(&json!("synced-value")));
}

/// Node-local cleanup must invalidate every handle sharing the deleted path.
#[test]
fn node_local_delete_clears_shared_snapshot_and_recreates_identical_records() {
    let host = Arc::new(MemoryStorageHost::default());
    let path = "runtime/space/device_profiles/returning-core.preferences.json";
    let first = super::CoreNodeStateStore::newWithStorage(host.clone(), path);
    let second = super::CoreNodeStateStore::newWithStorage(host.clone(), path);
    let mut snapshot = super::emptyPreferences();
    snapshot.set(
        &stringPreferencesKey("record"),
        "unchanged-profile".to_string(),
    );
    first.replaceRecoverably(snapshot.clone()).unwrap();
    assert_eq!(second.data().unwrap(), snapshot);
    first.delete().unwrap();
    assert!(!host.exists(path).unwrap());
    assert_eq!(second.data().unwrap(), super::emptyPreferences());
    second.replaceRecoverably(snapshot.clone()).unwrap();
    assert!(host.exists(path).unwrap());
    assert_eq!(first.data().unwrap(), snapshot);
    assert!(!host.exists("runtime/sync/operations").unwrap());
}

/// Builds a native complete-entry operation without relying on a local journal.
fn nativePreferenceOperation(
    path: &str,
    entries: Vec<(&str, Value)>,
    sequence: i64,
) -> crate::SyncOperationStore::SyncOperation {
    use crate::SyncOperationStore::{SyncOperation, SyncOperationSemantics};
    let transaction = entries.len() > 1;
    let mut keys = entries.iter().map(|(key, _)| *key).collect::<Vec<_>>();
    keys.sort();
    let (entityId, payload) = if transaction {
        (
            serde_json::to_string(&(path, keys)).unwrap(),
            json!({
                "storagePath":path, "encrypted":false, "formatVersion":2, "schemaVersion":0,
                "entries":entries.into_iter().map(|(key, value)| json!({"key":key,"operation":"set","value":value})).collect::<Vec<_>>()
            }),
        )
    } else {
        (
            super::preferenceMutationEntityId(path, entries[0].0, &[]).unwrap(),
            json!({
                "storagePath":path, "encrypted":false, "formatVersion":2, "schemaVersion":0, "key":entries[0].0,"value":entries[0].1
            }),
        )
    };
    SyncOperation {
        opId: format!("test:{sequence}"),
        originDeviceId: "test".into(),
        sequence,
        domain: "preferences".into(),
        entityType: "test".into(),
        entityId,
        operation: if transaction { "transaction" } else { "set" }.into(),
        semantics: if transaction {
            SyncOperationSemantics::Transaction
        } else {
            SyncOperationSemantics::EntityState
        },
        payload,
        createdAt: sequence,
        schemaVersion: 1,
    }
}

/// Replays overlapping edits in either arrival order without losing the transaction's other record.
#[test]
fn preference_transactions_converge_when_an_overlapping_edit_arrives_first() {
    let path = "runtime/config/preferences/transaction-order.preferences.json";
    let first = nativePreferenceOperation(
        path,
        vec![("A", json!({"name":"old"})), ("B", json!({"name":"keep"}))],
        1,
    );
    let later = nativePreferenceOperation(path, vec![("A", json!({"name":"new"}))], 2);
    let forward = Arc::new(MemoryStorageHost::default());
    let reverse = Arc::new(MemoryStorageHost::default());
    crate::PreferenceMerge::applyOperations(
        forward.clone(),
        &fixture_merge_schema(&[&path]),
        &[],
        &[first.clone()],
        |_, _| true,
    )
    .unwrap();
    crate::PreferenceMerge::applyOperations(
        forward.clone(),
        &fixture_merge_schema(&[&path]),
        &[first.clone()],
        &[later.clone()],
        |_, _| true,
    )
    .unwrap();
    crate::PreferenceMerge::applyOperations(
        reverse.clone(),
        &fixture_merge_schema(&[&path]),
        &[],
        &[later.clone()],
        |_, _| true,
    )
    .unwrap();
    crate::PreferenceMerge::applyOperations(
        reverse.clone(),
        &fixture_merge_schema(&[&path]),
        &[later],
        &[first],
        |_, _| true,
    )
    .unwrap();
    let result = PreferencesDataStore::newNodeLocalWithStorage(forward, path, false)
        .data()
        .unwrap();
    assert_eq!(
        result,
        PreferencesDataStore::newNodeLocalWithStorage(reverse, path, false)
            .data()
            .unwrap()
    );
    assert_eq!(
        result.get(&stringPreferencesKey("A")),
        Some(&json!({"name":"new"}))
    );
    assert_eq!(
        result.get(&stringPreferencesKey("B")),
        Some(&json!({"name":"keep"}))
    );
}

/// An invalid transaction commits none of its entries and leaves an independent operation usable.
#[test]
fn preference_transaction_validation_rejects_only_the_invalid_transaction() {
    let path = "runtime/config/preferences/invalid-transaction.preferences.json";
    let host = Arc::new(MemoryStorageHost::default());
    let valid = nativePreferenceOperation(path, vec![("first", json!(true))], 1);
    let mut invalid =
        nativePreferenceOperation(path, vec![("A", json!(1)), ("B", json!(2))], 2);
    invalid.payload["entries"][1]["key"] = json!("A");
    assert!(crate::PreferenceMerge::applyOperations(
        host.clone(),
        &fixture_merge_schema(&[&path]),
        &[],
        &[valid, invalid],
        |_, _| true,
    )
    .is_err());
    let data = PreferencesDataStore::newWithStorage(host, path).data().unwrap();
    assert_eq!(data.get(&stringPreferencesKey("first")), Some(&json!(true)));
    assert!(!data.contains(&stringPreferencesKey("A")));
    assert!(!data.contains(&stringPreferencesKey("B")));
}

/// A local storage migration changes representation without publishing business replacements.
#[test]
fn local_schema_migration_does_not_publish_business_operations() {
    let path = "runtime/config/preferences/schema-sync.preferences.json";
    let host = Arc::new(MemoryStorageHost::default());
    host.writeBytes(
        path,
        &serde_json::to_vec(&json!({
            PREFERENCES_SCHEMA_VERSION_KEY_NAME:1, "A":"{\"name\":\"legacy\"}"
        }))
        .unwrap(),
    )
    .unwrap();
    let store =
        PreferencesDataStore::newWithStorage(host.clone(), path).withSchema(2, |version, p| {
            assert_eq!(version, 1);
            let value: Value = serde_json::from_str(
                &p.getTyped::<String>(&stringPreferencesKey("A"))?.unwrap(),
            )?;
            p.set(&stringPreferencesKey("A"), value);
            Ok(())
        });
    assert_eq!(
        store.data().unwrap().get(&stringPreferencesKey("A")),
        Some(&json!({"name":"legacy"}))
    );
    assert_eq!(store.schemaVersion().unwrap(), 2);
    assert!(SyncOperationStore::new(host, RUNTIME_SYNC_DIR_PATH)
        .preferenceHistory()
        .unwrap()
        .is_empty());
}

/// A migration supersedes historical encoded values even if a stale operation arrives after it.
#[test]
fn preference_migration_replay_supersedes_legacy_encoded_root_values() {
    let path = "runtime/config/preferences/legacy-replay.preferences.json";
    let mut old = nativePreferenceOperation(path, vec![("A", json!("{\"name\":\"old\"}"))], 1);
    old.payload.as_object_mut().unwrap().remove("formatVersion");
    let native = nativePreferenceOperation(path, vec![("A", json!({"name":"native"}))], 2);
    let host = Arc::new(MemoryStorageHost::default());
    crate::PreferenceMerge::applyOperations(
        host.clone(),
        &fixture_merge_schema(&[&path]),
        &[],
        &[native.clone()],
        |_, _| true,
    )
    .unwrap();
    crate::PreferenceMerge::applyOperations(
        host.clone(),
        &fixture_merge_schema(&[&path]),
        &[native],
        &[old],
        |_, _| true,
    )
    .unwrap();
    assert_eq!(
        PreferencesDataStore::newNodeLocalWithStorage(host, path, false)
            .data()
            .unwrap()
            .get(&stringPreferencesKey("A")),
        Some(&json!({"name":"native"}))
    );
}

/// Retiring one key from an immutable transaction preserves its other business records.
#[test]
fn preference_transaction_owner_retirement_preserves_active_entries() {
    let path = "runtime/config/preferences/owner-retirement.preferences.json";
    let operation = nativePreferenceOperation(
        path,
        vec![("retired", json!(true)), ("active", json!({"name":"keep"}))],
        1,
    );
    let original = operation.clone();
    let host = Arc::new(MemoryStorageHost::default());
    crate::PreferenceMerge::applyOperations(
        host.clone(),
        &fixture_merge_schema(&[&path]),
        &[],
        &[operation.clone()],
        |_, key| key != "retired",
    )
    .unwrap();
    let snapshot = PreferencesDataStore::newNodeLocalWithStorage(host, path, false)
        .data()
        .unwrap();
    assert!(!snapshot.contains(&stringPreferencesKey("retired")));
    assert_eq!(
        snapshot.get(&stringPreferencesKey("active")),
        Some(&json!({"name":"keep"}))
    );
    assert_eq!(original, operation);
}

/// Internal conflict history retains local provenance even after the Space export boundary advances.
#[test]
fn preference_replay_history_is_independent_of_the_export_floor() {
    let path = "runtime/config/preferences/export-floor.preferences.json";
    let host = Arc::new(MemoryStorageHost::default());
    let data = PreferencesDataStore::newWithStorage(host.clone(), path);
    data.edit(|p| {
        p.set(&stringPreferencesKey("A"), json!({"name":"local"}));
        p.set(&stringPreferencesKey("B"), json!({"name":"keep"}));
    })
    .unwrap();
    let journal = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
    let history = journal.preferenceHistory().unwrap();
    journal.markLocalOperationsUnexportable().unwrap();
    assert!(journal
        .operationsSince(&SyncClock::empty(), &["preferences".into()], 100)
        .unwrap()
        .is_empty());
    assert_eq!(journal.preferenceHistory().unwrap(), history);
    let remote = nativePreferenceOperation(path, vec![("A", json!({"name":"remote-old"}))], 1);
    crate::PreferenceMerge::applyOperations(
        host,
        &fixture_merge_schema(&[&path]),
        &history,
        &[remote],
        |_, _| true,
    )
    .unwrap();
    assert_eq!(
        data.data().unwrap().get(&stringPreferencesKey("A")),
        Some(&json!({"name":"local"}))
    );
}

/// Unversioned historical data cannot enter a locally upgraded preference file.
#[test]
fn historical_payload_is_skipped_after_a_local_schema_upgrade() {
    let path = "runtime/config/preferences/stale-owner.preferences.json";
    let host = Arc::new(MemoryStorageHost::default());
    let store = PreferencesDataStore::newNodeLocalWithStorage(host.clone(), path, false)
        .withSchema(2, |_, _| Ok(()));
    store.data().unwrap();
    let mut old = nativePreferenceOperation(path, vec![("setting", json!(true))], 1);
    old.payload.as_object_mut().unwrap().remove("formatVersion");
    crate::PreferenceMerge::applyOperations(host, &fixture_merge_schema(&[&path]), &[], &[old], |_, _| {
        true
    })
    .expect("an obsolete preference is terminally skipped");
    assert_eq!(store.schemaVersion().unwrap(), 2);
    assert_eq!(
        store.data().unwrap().get(&stringPreferencesKey("setting")),
        None
    );
}

/// Replaying a complete newer schema snapshot never executes older native schema operations on the current file.
#[test]
fn replay_orders_native_history_without_copying_storage_versions() {
    let path = "runtime/config/preferences/native-version-replay.preferences.json";
    let host = Arc::new(MemoryStorageHost::default());
    let local = PreferencesDataStore::newNodeLocalWithStorage(host.clone(), path, false)
        .withSchema(4, |_, _| Ok(()));
    local.data().unwrap();
    let mut old = nativePreferenceOperation(
        path,
        vec![("A", json!({"name":"old"})), ("B", json!(true))],
        1,
    );
    old.payload["schemaVersion"] = json!(2);
    let mut new = nativePreferenceOperation(
        path,
        vec![("A", json!({"name":"new"})), ("B", json!(false))],
        2,
    );
    new.payload["schemaVersion"] = json!(4);
    crate::PreferenceMerge::applyOperations(
        host.clone(),
        &fixture_merge_schema(&[&path]),
        &[],
        &[new.clone()],
        |_, _| true,
    )
    .unwrap();
    crate::PreferenceMerge::applyOperations(
        host.clone(),
        &fixture_merge_schema(&[&path]),
        &[new],
        &[old],
        |_, _| true,
    )
    .expect("old-schema history is terminally skipped");
    let result = PreferencesDataStore::newNodeLocalWithStorage(host, path, false)
        .data()
        .unwrap();
    assert_eq!(
        result.get(&stringPreferencesKey(PREFERENCES_SCHEMA_VERSION_KEY_NAME)),
        Some(&json!(4))
    );
    assert_eq!(
        result.get(&stringPreferencesKey("A")),
        Some(&json!({"name":"new"}))
    );
    assert_eq!(result.get(&stringPreferencesKey("B")), Some(&json!(false)));
}

/// Replays explicit increment and object-field operations exactly once across repeated synchronization.
#[test]
fn explicit_preference_mutations_converge_without_business_key_policies() {
    let path = "runtime/config/preferences/explicit-mutations.preferences.json";
    let firstHost = Arc::new(MemoryStorageHost::default());
    let secondHost = Arc::new(MemoryStorageHost::default());
    let first = PreferencesDataStore::newWithStorage(firstHost.clone(), path);
    let second = PreferencesDataStore::newWithStorage(secondHost.clone(), path);
    first
        .try_edit_result(|preferences| -> Result<(), PreferencesDataStoreError> {
            preferences.incrementInteger(&stringPreferencesKey("arbitrary_counter"), 7)?;
            preferences.incrementInteger(&stringPreferencesKey("arbitrary_counter"), 5)?;
            preferences.setObjectField(
                &stringPreferencesKey("arbitrary_object"),
                "left",
                json!(true),
            )?;
            Ok(())
        })
        .unwrap();
    second
        .try_edit_result(|preferences| -> Result<(), PreferencesDataStoreError> {
            preferences.incrementInteger(&stringPreferencesKey("arbitrary_counter"), 11)?;
            preferences.setObjectField(
                &stringPreferencesKey("arbitrary_object"),
                "right",
                json!(false),
            )?;
            Ok(())
        })
        .unwrap();
    let firstHistory = SyncOperationStore::new(firstHost.clone(), RUNTIME_SYNC_DIR_PATH)
        .preferenceHistory()
        .unwrap();
    let secondHistory = SyncOperationStore::new(secondHost.clone(), RUNTIME_SYNC_DIR_PATH)
        .preferenceHistory()
        .unwrap();
    let schema = fixture_merge_schema(&[&path]);
    for _ in 0..2 {
        crate::PreferenceMerge::applyOperations(
            firstHost.clone(),
            &schema,
            &firstHistory,
            &secondHistory,
            |_, _| true,
        )
        .unwrap();
        crate::PreferenceMerge::applyOperations(
            secondHost.clone(),
            &schema,
            &secondHistory,
            &firstHistory,
            |_, _| true,
        )
        .unwrap();
        for store in [&first, &second] {
            let snapshot = store.data().unwrap();
            assert_eq!(
                snapshot.get(&stringPreferencesKey("arbitrary_counter")),
                Some(&json!(23))
            );
            assert_eq!(
                snapshot.get(&stringPreferencesKey("arbitrary_object")),
                Some(&json!({"left":true,"right":false}))
            );
        }
    }
}

/// Rejects arithmetic overflow before any atomic preference transaction is persisted.
#[test]
fn explicit_preference_increment_overflow_preserves_the_committed_snapshot() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = PreferencesDataStore::newWithStorage(
        host.clone(),
        "runtime/config/preferences/increment-overflow.preferences.json",
    );
    store
        .edit(|preferences| preferences.set(&stringPreferencesKey("count"), i64::MAX))
        .unwrap();
    let writes = host.writeCount();
    let result = store.try_edit_result(|preferences| -> Result<(), PreferencesDataStoreError> {
        preferences.set(&stringPreferencesKey("uncommitted"), true);
        preferences.incrementInteger(&stringPreferencesKey("count"), 1)
    });
    assert!(result.is_err());
    assert_eq!(host.writeCount(), writes);
    assert!(!store
        .data()
        .unwrap()
        .contains(&stringPreferencesKey("uncommitted")));
}

/// Declares a business-free UUID catalog used by identity history regressions.
fn identityHistorySchema() -> crate::PreferenceMerge::PreferenceMergeSchema {
    use crate::PreferenceMerge::*;
    PreferenceMergeSchema {
        stores: vec![PreferenceMergeStore {
            path: "runtime/config/preferences/identity-history.preferences.json".to_string(),
            encrypted: false,
            open: |storage, path| PreferencesDataStore::newWithStorage(storage, path),
            decode: None,
            merge: operit_host_api::PluginStorage::StoragePreferencesMergeDescriptor {
                version: 1,
                catalog: Some(
                    operit_host_api::PluginStorage::StoragePreferencesMergeCatalog {
                        key_prefix: "entity_".to_string(),
                        key_suffix: "".to_string(),
                        identity_field: "id".to_string(),
                        name_field: "name".to_string(),
                        ignored_fields: vec!["updatedAt".to_string()],
                        retired_keys: vec![],
                    },
                ),
                references: vec![],
                projection: None,
            },
        }],
    }
}

/// Historical equality does not discard independently edited identities in either delivery order.
#[test]
fn catalog_merge_compares_current_tips_instead_of_historical_snapshots() {
    let schema = identityHistorySchema();
    let path = schema.stores[0].path.as_str();
    let operations = vec![
        nativePreferenceOperation(
            path,
            vec![("entity_a", json!({"id":"a","name":"Same","value":0}))],
            1,
        ),
        nativePreferenceOperation(
            path,
            vec![("entity_b", json!({"id":"b","name":"Same","value":0}))],
            2,
        ),
        nativePreferenceOperation(
            path,
            vec![("entity_a", json!({"id":"a","name":"Same","value":1}))],
            3,
        ),
        nativePreferenceOperation(
            path,
            vec![("entity_b", json!({"id":"b","name":"Same","value":2}))],
            4,
        ),
    ];
    for incoming in [
        operations.clone(),
        operations.iter().rev().cloned().collect(),
    ] {
        let host = Arc::new(MemoryStorageHost::default());
        let refs = crate::PreferenceMerge::applyOperations(
            host.clone(),
            &schema,
            &[],
            &incoming,
            |_, _| true,
        )
        .unwrap();
        assert!(refs.is_empty());
        let snapshot = PreferencesDataStore::newNodeLocalWithStorage(host.clone(), path, false)
            .data()
            .unwrap();
        assert_eq!(
            snapshot.get(&stringPreferencesKey("entity_a")),
            Some(&json!({"id":"a","name":"Same","value":1}))
        );
        assert_eq!(
            snapshot.get(&stringPreferencesKey("entity_b")),
            Some(&json!({"id":"b","name":"Same（1）","value":2}))
        );
        assert_eq!(
            SyncOperationStore::new(host, RUNTIME_SYNC_DIR_PATH)
                .preferenceHistory()
                .unwrap(),
            operations
        );
    }
}

/// A recorded current-state coalescence redirects later edits and deletions on every receiver.
#[test]
fn catalog_identity_decisions_survive_replay_edit_delete_and_new_receiver() {
    let schema = identityHistorySchema();
    let path = schema.stores[0].path.as_str();
    let host = Arc::new(MemoryStorageHost::default());
    let mut history = vec![
        nativePreferenceOperation(
            path,
            vec![("entity_a", json!({"id":"a","name":"Same","value":0}))],
            1,
        ),
        nativePreferenceOperation(
            path,
            vec![("entity_b", json!({"id":"b","name":"Same","value":0}))],
            2,
        ),
    ];
    let mut alias = nativePreferenceOperation(path, vec![("entity_b", json!("entity_a"))], 3);
    alias.operation = "alias".into();
    alias.semantics = crate::SyncOperationStore::SyncOperationSemantics::Transaction;
    history.push(alias);
    let refs = crate::PreferenceMerge::applyOperations(
        host.clone(),
        &schema,
        &[],
        &history,
        |_, _| true,
    )
    .unwrap();
    assert_eq!(
        refs.get(&format!("{path}\u{001f}b")),
        Some(&"a".to_string())
    );
    history.push(nativePreferenceOperation(
        path,
        vec![("entity_b", json!({"id":"b","name":"Edited","value":7}))],
        4,
    ));
    for target in [host.clone(), Arc::new(MemoryStorageHost::default())] {
        for _ in 0..2 {
            crate::PreferenceMerge::applyOperations(
                target.clone(),
                &schema,
                &[],
                &history,
                |_, _| true,
            )
            .unwrap();
            let snapshot =
                PreferencesDataStore::newNodeLocalWithStorage(target.clone(), path, false)
                    .data()
                    .unwrap();
            assert_eq!(
                snapshot.get(&stringPreferencesKey("entity_a")),
                Some(&json!({"id":"a","name":"Edited","value":7}))
            );
            assert!(snapshot.get(&stringPreferencesKey("entity_b")).is_none());
        }
        let mut deletion = nativePreferenceOperation(path, vec![("entity_b", Value::Null)], 5);
        deletion.operation = "delete".into();
        crate::PreferenceMerge::applyOperations(
            target.clone(),
            &schema,
            &history,
            &[deletion],
            |_, _| true,
        )
        .unwrap();
        let snapshot = PreferencesDataStore::newNodeLocalWithStorage(target, path, false)
            .data()
            .unwrap();
        assert!(snapshot.get(&stringPreferencesKey("entity_a")).is_none());
        assert!(snapshot.get(&stringPreferencesKey("entity_b")).is_none());
    }
}

/// Invalid aliases are rejected before any preferences or identity decision is published.
#[test]
fn catalog_identity_alias_requires_declared_catalog_keys_and_transaction_semantics() {
    let schema = identityHistorySchema();
    let path = schema.stores[0].path.as_str();
    let host = Arc::new(MemoryStorageHost::default());
    let mut operation =
        nativePreferenceOperation(path, vec![("unrelated", json!("entity_a"))], 1);
    operation.operation = "alias".into();
    assert!(PreferencesSyncedEntry::fromOperation(&operation).is_err());
    operation.semantics = crate::SyncOperationStore::SyncOperationSemantics::Transaction;
    assert!(crate::PreferenceMerge::applyOperations(
        host.clone(),
        &schema,
        &[],
        &[operation],
        |_, _| true,
    )
    .is_err());
    assert_eq!(host.writeCount(), 0);
}

/// A transport page cannot establish identity while the source's later edits are still in flight.
#[test]
fn transport_completion_never_coalesces_stable_identities() {
    let schema = identityHistorySchema();
    let path = schema.stores[0].path.as_str();
    let host = Arc::new(MemoryStorageHost::default());
    let journal = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
    let mut history = vec![
        nativePreferenceOperation(
            path,
            vec![("entity_a", json!({"id":"a","name":"Same","value":0}))],
            1,
        ),
        nativePreferenceOperation(
            path,
            vec![("entity_b", json!({"id":"b","name":"Same","value":0}))],
            2,
        ),
    ];
    let refs = crate::PreferenceMerge::applyOperations(
        host.clone(),
        &schema,
        &[],
        &history,
        |_, _| true,
    )
    .unwrap();
    assert!(refs.is_empty());
    assert_eq!(journal.preferenceHistory().unwrap(), history);
    history.push(nativePreferenceOperation(
        path,
        vec![("entity_b", json!({"id":"b","name":"Same","value":2}))],
        3,
    ));
    let refs = crate::PreferenceMerge::applyOperations(
        host.clone(),
        &schema,
        &[],
        &history,
        |_, _| true,
    )
    .unwrap();
    assert!(refs.is_empty());
    let snapshot = PreferencesDataStore::newNodeLocalWithStorage(host, path, false)
        .data()
        .unwrap();
    assert_eq!(
        snapshot.get(&stringPreferencesKey("entity_a")).unwrap()["value"],
        json!(0)
    );
    assert_eq!(
        snapshot.get(&stringPreferencesKey("entity_b")).unwrap()["value"],
        json!(2)
    );
    assert_eq!(journal.preferenceHistory().unwrap(), history);
}

/// An offline delta from a different schema cannot mutate a current-version counter.
#[test]
fn offline_old_schema_increment_is_skipped_after_a_local_upgrade() {
    let host = Arc::new(MemoryStorageHost::default());
    let path = "runtime/config/preferences/offline-counter.preferences.json";
    let store = PreferencesDataStore::newNodeLocalWithStorage(host.clone(), path, false)
        .withSchema(4, |_, _| Ok(()));
    store.data().unwrap();
    let mut baseline = nativePreferenceOperation(path, vec![("counter", json!(10))], 1);
    baseline.payload["schemaVersion"] = json!(4);
    let mut offline = nativePreferenceOperation(path, vec![("counter", json!(2))], 2);
    offline.originDeviceId = "offline".into();
    offline.sequence = 1;
    offline.opId = "offline:1".into();
    offline.payload["schemaVersion"] = json!(1);
    offline.operation = "increment".into();
    offline.semantics = crate::SyncOperationStore::SyncOperationSemantics::Transaction;
    let operations = vec![baseline, offline];
    for _ in 0..2 {
        let outcomes = crate::PreferenceMerge::applyIndependentOperations(
            host.clone(),
            &fixture_merge_schema(&[&path]),
            &[],
            &operations,
            |_, _| true,
        );
        assert_eq!(outcomes.iter().filter(|outcome| outcome.skipped).count(), 1);
        assert!(outcomes.iter().all(|outcome| outcome.result.is_ok()));
        assert_eq!(
            store.data().unwrap().get(&stringPreferencesKey("counter")),
            Some(&json!(10))
        );
        assert_eq!(store.schemaVersion().unwrap(), 4);
    }
    assert_eq!(
        SyncOperationStore::new(host, RUNTIME_SYNC_DIR_PATH)
            .preferenceHistory()
            .unwrap(),
        vec![operations[0].clone()]
    );
}

/// A broken plugin catalog cannot prevent its referenced ordinary configuration from committing.
#[test]
fn failed_plugin_dataset_does_not_block_its_reference_target() {
    use crate::PreferenceMerge::*;
    use operit_host_api::PluginStorage::{
        StoragePreferencesMergeKey, StoragePreferencesMergeReference,
    };
    let host = Arc::new(MemoryStorageHost::default());
    let mut schema = identityHistorySchema();
    let target = schema.stores[0].path.clone();
    let plugin = "runtime/plugin_data/space/test_plugin/cards.preferences.json";
    let mut declaration = schema.stores[0].clone();
    declaration.path = plugin.into();
    declaration
        .merge
        .references
        .push(StoragePreferencesMergeReference {
            key: StoragePreferencesMergeKey::Exact {
                key: "selection".into(),
            },
            path: vec![],
            target: target.clone(),
        });
    schema.stores.push(declaration);
    let incoming = vec![
        nativePreferenceOperation(
            plugin,
            vec![("entity_bad", json!({"id":"wrong", "name":"Bad"}))],
            1,
        ),
        nativePreferenceOperation(
            &target,
            vec![("entity_a", json!({"id":"a", "name":"A"}))],
            2,
        ),
    ];
    let outcomes =
        applyIndependentOperations(host.clone(), &schema, &[], &incoming, |_, _| true);
    let failed = outcomes
        .iter()
        .find(|outcome| outcome.result.is_err())
        .unwrap();
    assert_eq!(failed.paths, BTreeSet::from([plugin.to_owned()]));
    assert_eq!(failed.operations, vec![incoming[0].clone()]);
    assert!(!host.exists(plugin).unwrap());
    let target = PreferencesDataStore::newWithStorage(host, &target)
        .data()
        .unwrap();
    assert_eq!(
        target.get(&stringPreferencesKey("entity_a")),
        Some(&json!({"id":"a", "name":"A"}))
    );
}

/// Failed atomic publication cannot create an exportable operation or advance the business cache.
#[test]
fn failed_local_commit_has_no_phantom_operation() {
    let host = Arc::new(MemoryStorageHost::default());
    let path = "runtime/config/preferences/commit-failure.preferences.json";
    let store = PreferencesDataStore::newWithStorage(host.clone(), path);
    let key = stringPreferencesKey("counter");
    store.edit(|p| p.set(&key, 5)).unwrap();
    let journal = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
    let before = journal.preferenceHistory().unwrap();
    let bytes = host.readBytes(path).unwrap();
    host.failedWrites.lock().unwrap().insert(path.into());
    assert!(store
        .edit(|p| {
            p.incrementInteger(&key, 2).unwrap();
        })
        .is_err());
    assert_eq!(store.data().unwrap().get(&key), Some(&json!(5)));
    assert_eq!(host.readBytes(path).unwrap(), bytes);
    assert_eq!(journal.preferenceHistory().unwrap(), before);
    host.failedWrites.lock().unwrap().clear();
    store
        .edit(|p| {
            p.incrementInteger(&key, 2).unwrap();
        })
        .unwrap();
    let history = journal.preferenceHistory().unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[1].sequence, 2);
    assert_eq!(store.data().unwrap().get(&key), Some(&json!(7)));
}

/// A committed edit remains recoverable after journal failure and a complete process-cache restart.
#[test]
fn committed_outbox_survives_journal_failure_and_restart() {
    let host = Arc::new(MemoryStorageHost::default());
    let path = "runtime/config/preferences/outbox-restart.preferences.json";
    let store = PreferencesDataStore::newWithStorage(host.clone(), path);
    let key = stringPreferencesKey("counter");
    store
        .edit(|p| {
            p.incrementInteger(&key, 2).unwrap();
        })
        .unwrap();
    let committed = store.data().unwrap();
    let original = committed.replication.outbox[0].clone();
    let journal = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
    host.failedWrites
        .lock()
        .unwrap()
        .insert(journal.operationsPath(&original.originDeviceId));
    assert_eq!(journal.publishPreferenceOutboxes().unwrap().len(), 1);
    assert_eq!(store.data().unwrap().get(&key), Some(&json!(2)));
    assert!(journal.preferenceHistory().unwrap().is_empty());
    host.failedWrites.lock().unwrap().clear();
    let restartedHost = Arc::new((*host).clone());
    let restarted = SyncOperationStore::new(restartedHost.clone(), RUNTIME_SYNC_DIR_PATH);
    assert_eq!(
        restarted.preferenceHistory().unwrap(),
        vec![original.clone()]
    );
    assert_eq!(restarted.preferenceHistory().unwrap(), vec![original]);
    let restored = PreferencesDataStore::newWithStorage(restartedHost, path)
        .data()
        .unwrap();
    assert_eq!(restored.get(&key), Some(&json!(2)));
    assert!(restored.replication.outbox.is_empty());
    assert!(serde_json::to_value(restored)
        .unwrap()
        .get(super::PREFERENCES_REPLICATION_KEY)
        .is_none());
}

/// A failed outbox acknowledgement republishes the same identity without adding a second journal line.
#[test]
fn outbox_acknowledgement_failure_is_idempotent() {
    let host = Arc::new(MemoryStorageHost::default());
    let path = "runtime/config/preferences/outbox-ack.preferences.json";
    let store = PreferencesDataStore::newWithStorage(host.clone(), path);
    store
        .edit(|p| p.set(&stringPreferencesKey("ready"), true))
        .unwrap();
    let original = store.data().unwrap().replication.outbox[0].clone();
    let journal = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
    host.failedWrites.lock().unwrap().insert(path.into());
    assert_eq!(journal.publishPreferenceOutboxes().unwrap().len(), 1);
    let log = host
        .readBytes(&journal.operationsPath(&original.originDeviceId))
        .unwrap();
    host.failedWrites.lock().unwrap().clear();
    assert!(journal.publishPreferenceOutboxes().unwrap().is_empty());
    assert_eq!(
        host.readBytes(&journal.operationsPath(&original.originDeviceId))
            .unwrap(),
        log
    );
    assert_eq!(journal.preferenceHistory().unwrap(), vec![original]);
}

/// Dataset failure does not prevent committed operations from another outbox from being exported.
#[test]
fn failed_outbox_does_not_block_another_dataset() {
    let host = Arc::new(MemoryStorageHost::default());
    let failed =
        PreferencesDataStore::newWithStorage(host.clone(), "runtime/a.preferences.json");
    let healthy =
        PreferencesDataStore::newWithStorage(host.clone(), "runtime/b.preferences.json");
    failed
        .edit(|p| p.set(&stringPreferencesKey("a"), true))
        .unwrap();
    healthy
        .edit(|p| p.set(&stringPreferencesKey("b"), true))
        .unwrap();
    let journal = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
    let bad = failed.data().unwrap().replication.outbox[0].clone();
    let good = healthy.data().unwrap().replication.outbox[0].clone();
    host.failedWrites
        .lock()
        .unwrap()
        .insert(journal.operationsPath(&bad.originDeviceId));
    assert_eq!(journal.preferenceHistory().unwrap(), vec![good]);
    assert!(!failed.data().unwrap().replication.outbox.is_empty());
}

/// Remote state and its application identity are both absent after a failed durable publication.
#[test]
fn failed_remote_commit_keeps_original_pending_until_atomic_retry() {
    let host = Arc::new(MemoryStorageHost::default());
    let path = "runtime/config/preferences/remote-commit.preferences.json";
    let operation = nativePreferenceOperation(path, vec![("a", json!(1)), ("b", json!(2))], 1);
    let journal = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
    journal.receiveOperations(&[operation.clone()]).unwrap();
    host.failedWrites.lock().unwrap().insert(path.into());
    let outcomes = crate::PreferenceMerge::applyIndependentOperations(
        host.clone(),
        &fixture_merge_schema(&[&path]),
        &[],
        &[operation.clone()],
        |_, _| true,
    );
    assert!(outcomes[0].result.is_err());
    assert!(!host.exists(path).unwrap());
    assert_eq!(
        journal.pendingOperations().unwrap(),
        vec![operation.clone()]
    );
    assert!(journal.preferenceHistory().unwrap().is_empty());
    host.failedWrites.lock().unwrap().clear();
    let outcomes = crate::PreferenceMerge::applyIndependentOperations(
        host.clone(),
        &fixture_merge_schema(&[&path]),
        &[],
        &[operation.clone()],
        |_, _| true,
    );
    assert!(outcomes[0].result.is_ok());
    let preferences = PreferencesDataStore::newWithStorage(host, path)
        .data()
        .unwrap();
    assert_eq!(preferences.get(&stringPreferencesKey("a")), Some(&json!(1)));
    assert_eq!(preferences.get(&stringPreferencesKey("b")), Some(&json!(2)));
    assert!(preferences
        .replication
        .applied
        .contains_key(&operation.opId));
    assert_eq!(preferences.replication.outbox, vec![operation.clone()]);
    journal.completeOperations(&[operation.clone()]).unwrap();
    assert!(journal.pendingOperations().unwrap().is_empty());
    assert_eq!(journal.preferenceHistory().unwrap(), vec![operation]);
}

/// Historical wall-clock rollback cannot reverse two operations from the same origin.
#[test]
fn replay_preserves_source_sequence_during_clock_rollback() {
    let host = Arc::new(MemoryStorageHost::default());
    let path = "runtime/config/preferences/clock-rollback.preferences.json";
    let mut earlier = nativePreferenceOperation(path, vec![("value", json!(1))], 1);
    let mut later = nativePreferenceOperation(path, vec![("value", json!(2))], 2);
    earlier.createdAt = 1000;
    later.createdAt = 1;
    crate::PreferenceMerge::applyOperations(
        host.clone(),
        &fixture_merge_schema(&[&path]),
        &[],
        &[later, earlier],
        |_, _| true,
    )
    .unwrap();
    assert_eq!(
        PreferencesDataStore::newWithStorage(host, path)
            .data()
            .unwrap()
            .get(&stringPreferencesKey("value")),
        Some(&json!(2))
    );
}

/// A local edit carries a logical time after every remote causal observation in its dataset.
#[test]
fn local_preference_order_advances_past_remote_logical_time() {
    let host = Arc::new(MemoryStorageHost::default());
    let path = "runtime/config/preferences/logical-clock.preferences.json";
    let mut remote = nativePreferenceOperation(path, vec![("value", json!(1))], 1);
    let future = operit_host_api::TimeUtils::tryCurrentTimeMillis().unwrap() + 1_000_000;
    remote.payload["contractVersion"] = json!(1);
    remote.payload["logicalTime"] = json!(future);
    crate::PreferenceMerge::applyOperations(
        host.clone(),
        &fixture_merge_schema(&[&path]),
        &[],
        &[remote],
        |_, _| true,
    )
    .unwrap();
    let store = PreferencesDataStore::newWithStorage(host.clone(), path);
    store
        .edit(|p| p.set(&stringPreferencesKey("value"), 2))
        .unwrap();
    let history = SyncOperationStore::new(host, RUNTIME_SYNC_DIR_PATH)
        .preferenceHistory()
        .unwrap();
    let local = history
        .iter()
        .find(|operation| operation.originDeviceId != "test")
        .unwrap();
    assert!(local.payload["logicalTime"].as_i64().unwrap() > future);
    assert_eq!(local.payload["schemaVersion"], json!(0));
}

/// A failed alias-owner commit cannot redirect a successfully committed external reference.
#[test]
fn failed_alias_owner_does_not_project_uncommitted_identity_decisions() {
    use crate::PreferenceMerge::*;
    use operit_host_api::PluginStorage::{
        StoragePreferencesMergeKey, StoragePreferencesMergeReference,
    };
    let host = Arc::new(MemoryStorageHost::default());
    let mut schema = identityHistorySchema();
    let target = schema.stores[0].path.clone();
    let source = "runtime/config/preferences/alias-source.preferences.json";
    let mut declaration = schema.stores[0].clone();
    declaration.path = source.into();
    declaration.merge.catalog = None;
    declaration
        .merge
        .references
        .push(StoragePreferencesMergeReference {
            key: StoragePreferencesMergeKey::Exact {
                key: "selection".into(),
            },
            path: vec![],
            target: target.clone(),
        });
    schema.stores.push(declaration);
    let mut alias =
        nativePreferenceOperation(&target, vec![("entity_b", json!("entity_a"))], 2);
    alias.operation = "alias".into();
    alias.semantics = crate::SyncOperationStore::SyncOperationSemantics::Transaction;
    let incoming = vec![
        nativePreferenceOperation(&target, vec![("entity_a", json!({"id":"a","name":"A"}))], 1),
        alias,
        nativePreferenceOperation(source, vec![("selection", json!("b"))], 3),
    ];
    host.failedWrites.lock().unwrap().insert(target.clone());
    let outcomes =
        applyIndependentOperations(host.clone(), &schema, &[], &incoming, |_, _| true);
    assert!(outcomes
        .iter()
        .find(|outcome| outcome.paths.contains(&target))
        .unwrap()
        .result
        .is_err());
    assert!(outcomes
        .iter()
        .find(|outcome| outcome.paths.contains(source))
        .unwrap()
        .result
        .is_ok());
    let store = PreferencesDataStore::newNodeLocalWithStorage(host.clone(), source, false);
    assert_eq!(
        store
            .data()
            .unwrap()
            .get(&stringPreferencesKey("selection")),
        Some(&json!("b"))
    );
    assert!(!host.exists(&target).unwrap());
    host.failedWrites.lock().unwrap().clear();
    let outcomes =
        applyIndependentOperations(host.clone(), &schema, &[], &incoming, |_, _| true);
    assert!(outcomes.iter().all(|outcome| outcome.result.is_ok()));
    assert_eq!(
        store
            .data()
            .unwrap()
            .get(&stringPreferencesKey("selection")),
        Some(&json!("a"))
    );
}

/// Two immutable identities cannot silently replace one another at the same source sequence.
#[test]
fn conflicting_origin_sequence_is_rejected_before_materialization() {
    let path = "runtime/config/preferences/sequence-collision.preferences.json";
    let first = nativePreferenceOperation(path, vec![("counter", json!(10))], 1);
    let mut conflict = first.clone();
    conflict.opId = "conflicting-identity".into();
    conflict.payload["value"] = json!(12);
    assert!(PreferencesDataStore::prepareSyncedEntries(&[], &[first, conflict]).is_err());
}

/// Locked replay sees local commits made after the caller's history snapshot, both archived and pending.
#[test]
fn stale_history_preserves_local_transactions_in_the_journal_and_outbox() {
    use crate::PreferenceMerge::applyOperations;
    for archived in [false, true] {
        let host = Arc::new(MemoryStorageHost::default());
        let path = "runtime/config/preferences/stale-history.preferences.json";
        let store = PreferencesDataStore::newWithStorage(host.clone(), path);
        let key = stringPreferencesKey("counter");
        store.edit(|preferences| preferences.set(&key, 10)).unwrap();
        let journal = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
        let history = journal.preferenceHistory().unwrap();
        store
            .edit(|preferences| {
                preferences.incrementInteger(&key, 2).unwrap();
            })
            .unwrap();
        let local = store.data().unwrap().replication.outbox[0].clone();
        if archived {
            journal.preferenceHistory().unwrap();
        } else {
            host.failedWrites
                .lock()
                .unwrap()
                .insert(journal.operationsPath(&local.originDeviceId));
        }
        let mut remote = nativePreferenceOperation(path, vec![("counter", json!(3))], 1);
        remote.operation = "increment".into();
        remote.semantics = crate::SyncOperationStore::SyncOperationSemantics::Transaction;
        remote.payload["contractVersion"] = json!(1);
        remote.payload["logicalTime"] =
            json!(local.payload["logicalTime"].as_i64().unwrap() + 100);
        for _ in 0..2 {
            applyOperations(
                host.clone(),
                &fixture_merge_schema(&[&path]),
                &history,
                &[remote.clone()],
                |_, _| true,
            )
            .unwrap();
            assert_eq!(
                store.data().unwrap().get(&key),
                Some(&json!(15)),
                "archived={archived}"
            );
        }
        host.failedWrites.lock().unwrap().clear();
        let originals = journal.preferenceHistory().unwrap();
        assert_eq!(originals.len(), 3);
        assert!(originals.contains(&local));
        assert!(originals.contains(&remote));
    }
}

/// Archival is acknowledged only after original bytes are durable; retry keeps the same identity.
#[test]
fn outbox_survives_flush_failure_without_duplicate_append() {
    let host = Arc::new(MemoryStorageHost::default());
    let path = "runtime/config/preferences/outbox-flush.preferences.json";
    let store = PreferencesDataStore::newWithStorage(host.clone(), path);
    store.edit(|p| p.set(&stringPreferencesKey("ready"), true)).unwrap();
    let original = store.data().unwrap().replication.outbox[0].clone();
    let journal = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
    let logPath = journal.operationsPath(&original.originDeviceId);
    host.failedFlushes.lock().unwrap().insert(logPath.clone());
    assert_eq!(journal.publishPreferenceOutboxes().unwrap().len(), 1);
    assert_eq!(store.data().unwrap().replication.outbox, vec![original.clone()]);
    let bytes = host.readBytes(&logPath).unwrap();
    host.failedFlushes.lock().unwrap().clear();
    assert!(journal.publishPreferenceOutboxes().unwrap().is_empty());
    assert!(store.data().unwrap().replication.outbox.is_empty());
    assert_eq!(host.readBytes(&logPath).unwrap(), bytes);
    assert_eq!(journal.preferenceHistory().unwrap(), vec![original]);
}

/// A missing key in one owner must not prevent other owners from committing or replaying.
#[test]
fn dataset_history_filters_encrypted_owners_before_decryption() {
    let host = Arc::new(MemoryStorageHost::default());
    let healthyPath = MODEL_CONFIGS_PREFERENCES_PATH;
    let brokenPath = operit_util::RuntimeStorageLayout::TTS_CONFIGS_PREFERENCES_PATH;
    for path in [healthyPath, brokenPath] {
        PreferencesDataStore::newEncryptedWithStorage(host.clone(), path)
            .edit(|p| p.set(&stringPreferencesKey("ready"), true)).unwrap();
    }
    let journal = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
    let originals = journal.preferenceHistory().unwrap();
    let healthy = originals.iter().find(|op| op.payload["storagePath"] == healthyPath).unwrap();
    let broken = originals.iter().find(|op| op.payload["storagePath"] == brokenPath).unwrap();
    let logPath = journal.operationsPath(&broken.originDeviceId);
    let mut encoded: Value = serde_json::from_slice(&host.readBytes(&logPath).unwrap()).unwrap();
    encoded["payload"]["envelope"]["keyId"] = json!("old-unavailable-key");
    let damaged = serde_json::to_vec(&encoded).unwrap();
    host.writeBytes(&logPath, &damaged).unwrap();

    let restarted = Arc::new((*host).clone());
    let journal = SyncOperationStore::new(restarted.clone(), RUNTIME_SYNC_DIR_PATH);
    assert_eq!(journal.archivedPreferenceHistory(healthyPath).unwrap(), vec![healthy.clone()]);
    let plain = PreferencesDataStore::newWithStorage(restarted.clone(), "runtime/config/preferences/startup.preferences.json");
    plain.edit(|p| p.set(&stringPreferencesKey("booted"), true)).unwrap();
    assert_eq!(plain.data().unwrap().get(&stringPreferencesKey("booted")), Some(&json!(true)));
    let healthyStore = PreferencesDataStore::newEncryptedWithStorage(restarted, healthyPath);
    healthyStore.edit(|p| p.set(&stringPreferencesKey("updated"), true)).unwrap();
    let error = journal.archivedPreferenceHistory(brokenPath).unwrap_err().to_string();
    assert!(error.contains("key id mismatch"), "{error}");
    assert!(error.contains(brokenPath), "{error}");
    assert_eq!(host.readBytes(&logPath).unwrap(), damaged);
}

/// User callbacks edit private snapshots; unwinding must not publish them or poison future edits.
#[test]
fn a_panicking_edit_reloads_the_commit_and_allows_the_next_edit() {
    let host = Arc::new(MemoryStorageHost::default());
    let path = format!("{CONFIG_PREFERENCES_DIR_PATH}/panic-edit.preferences.json");
    let store = PreferencesDataStore::newWithStorage(host, path);
    let key = stringPreferencesKey("counter");
    store.edit(|values| values.set(&key, 1)).unwrap();
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = store.edit(|values| {
            values.set(&key, 999);
            panic!("injected preference callback panic");
        });
    }));
    assert!(failed.is_err());
    assert_eq!(store.data().unwrap().get(&key), Some(&json!(1)));
    store.edit(|values| values.set(&key, 2)).unwrap();
    assert_eq!(store.data().unwrap().get(&key), Some(&json!(2)));
}

#[test]
fn replacing_and_deleting_shared_values_preserves_previous_snapshots_and_other_keys() {
    let mut original = Preferences::default();
    let key = stringPreferencesKey("record");
    let keep = stringPreferencesKey("keep");
    let before = json!({"source": ["old".repeat(1024)], "status": "pending"});
    original.set(&key, before.clone());
    original.set(&keep, json!({"unchanged": true}));
    let mut edited = original.clone();
    let after = json!({"source": ["new".repeat(1024)], "status": "approved"});
    edited.set(&key, after.clone());
    assert_eq!(original.get(&key), Some(&before));
    assert_eq!(edited.get(&key), Some(&after));
    assert_eq!(original.get(&keep), edited.get(&keep));
    let mut removed = edited.clone();
    removed.remove(&key);
    assert!(removed.get(&key).is_none());
    assert_eq!(edited.get(&key), Some(&after));
    assert_eq!(removed.get(&keep), original.get(&keep));
    removed.set(&key, json!("restored"));
    assert_eq!(removed.get(&key), Some(&json!("restored")));
}

#[test]
fn borrowed_snapshot_encoding_matches_existing_sorted_json_bytes() {
    for constrained in [false, true] {
    let host = Arc::new(MemoryStorageHost { releasePreferencesCache: constrained, ..Default::default() });
    let path = "runtime/nvs-encoding.preferences.json";
    let store = super::CoreNodeStateStore::newWithStorage(host.clone(), path);
    let mut preferences = Preferences::default();
    preferences.set(&stringPreferencesKey("z-last"), json!({"nested": [true, null, "设备"]}));
    preferences.set(&stringPreferencesKey("a-first"), json!("unchanged bytes"));
    let values = serde_json::to_value(&preferences).unwrap();
    let expected = if constrained { serde_json::to_vec(&values).unwrap() }
        else { serde_json::to_vec_pretty(&values).unwrap() };
    store.replaceRecoverably(preferences.clone()).unwrap();
    assert_eq!(host.readBytes(path).unwrap(), expected);
    assert_eq!(store.data().unwrap(), preferences);
    }
}

#[test]
fn deferred_set_mutations_export_final_values_after_composed_edits() {
    let mut preferences = Preferences::default();
    let key = stringPreferencesKey("record");
    preferences.set(&key, json!({"field": 1}));
    preferences.setObjectField(&key, "field", json!(2)).unwrap();
    let mutation = PreferencesDataStore::entryMutation("record", &preferences).unwrap();
    assert_eq!(mutation.operation, "set");
    assert_eq!(mutation.value, json!({"field": 2}));
    preferences.set(&key, json!(10));
    preferences.incrementInteger(&key, 2).unwrap();
    preferences.incrementInteger(&key, 3).unwrap();
    let mutation = PreferencesDataStore::entryMutation("record", &preferences).unwrap();
    assert_eq!(mutation.operation, "set");
    assert_eq!(mutation.value, json!(15));
    let previous = preferences.clone();
    preferences.remove(&key);
    let mutation = PreferencesDataStore::entryMutation("record", &preferences).unwrap();
    assert_eq!(mutation.operation, "delete");
    assert_eq!(mutation.value, Value::Null);
    preferences.set(&key, json!(["final"]));
    assert_eq!(PreferencesDataStore::entryMutation("record", &preferences).unwrap().value, json!(["final"]));
    assert_eq!(PreferencesDataStore::entryMutation("record", &previous).unwrap().value, json!(15));
}

#[test]
fn constrained_host_shares_live_transactions_and_reloads_after_last_handle_closes() {
    let host = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let path = "runtime/constrained-cache.preferences.json";
    let first = super::CoreNodeStateStore::newWithStorage(host.clone(), path);
    let second = super::CoreNodeStateStore::newWithStorage(host.clone(), path);
    assert!(Arc::ptr_eq(&first.inner.sharedState, &second.inner.sharedState));
    let weak = Arc::downgrade(&first.inner.sharedState);
    first.edit(|p| p.set(&stringPreferencesKey("record"), json!({"status": "pending"}))).unwrap();
    let snapshot = second.data().unwrap();
    assert_eq!(snapshot.get(&stringPreferencesKey("record")), Some(&json!({"status": "pending"})));
    assert!(second.inner.loadedPreferences().is_none(), "long-lived node stores must release their decoded cache after reads");
    drop(first);
    assert!(weak.upgrade().is_some());
    second.edit(|p| p.set(&stringPreferencesKey("record"), json!({"status": "approved"}))).unwrap();
    assert_eq!(snapshot.get(&stringPreferencesKey("record")), Some(&json!({"status": "pending"})));
    drop(second);
    assert!(weak.upgrade().is_none(), "closed constrained stores must release parsed JSON");
    let reads = host.readCount();
    let reopened = super::CoreNodeStateStore::newWithStorage(host.clone(), path);
    assert_eq!(reopened.data().unwrap().get(&stringPreferencesKey("record")), Some(&json!({"status": "approved"})));
    assert!(host.readCount() > reads, "reopen must read the durable snapshot");
}

#[test]
fn constrained_typed_scan_checks_schema_migrates_and_rejects_bad_records_without_writes() {
    #[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq)]
    struct Record { status: String }
    let host = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let path = "runtime/typed-scan.preferences.json";
    let make = || super::CoreNodeStateStore::newWithStorage(host.clone(), path).withSchema(1, |_, p| {
        p.set(&stringPreferencesKey("record"), json!({"status":"migrated"}));
        Ok(())
    });
    host.writeBytes(path, b"{}").unwrap();
    assert_eq!(make().typedEntries::<Record>().unwrap(), vec![("record".into(), Record { status: "migrated".into() })]);
    let writes = host.writeCount();
    let store = make();
    assert_eq!(store.typedEntries::<Record>().unwrap(), vec![("record".into(), Record { status: "migrated".into() })]);
    assert!(store.inner.loadedPreferences().is_none(), "typed scans must not construct a parsed cache");
    assert_eq!(host.writeCount(), writes);
    drop(store);
    host.writeBytes(path, &serde_json::to_vec(&json!({PREFERENCES_SCHEMA_VERSION_KEY_NAME: 2, "record":{"status":"future"}})).unwrap()).unwrap();
    let writes = host.writeCount();
    assert!(matches!(make().typedEntries::<Record>(), Err(PreferencesDataStoreError::SchemaVersionTooNew { .. })));
    assert_eq!(host.writeCount(), writes);
    host.writeBytes(path, &serde_json::to_vec(&json!({PREFERENCES_SCHEMA_VERSION_KEY_NAME: 1, "record":{"invalid":true}})).unwrap()).unwrap();
    let writes = host.writeCount();
    assert!(make().typedEntries::<Record>().is_err());
    assert_eq!(host.writeCount(), writes);
}
#[test]
fn schema_and_first_record_commit_atomically_on_failed_edit_and_replacement() {
    for replace in [false, true] {
        let host = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
        let path = "runtime/atomic-schema.preferences.json";
        let make = || super::CoreNodeStateStore::newWithStorage(host.clone(), path).withSchema(1, |_, _| Ok(()));
        host.failedWrites.lock().unwrap().insert(path.into());
        let mut record = super::emptyPreferences();
        record.set(&stringPreferencesKey("record"), json!({"status":"complete"}));
        let result = if replace { make().replaceRecoverably(record.clone()) }
            else { make().edit(|p| p.set(&stringPreferencesKey("record"), json!({"status":"complete"}))) };
        assert!(result.is_err());
        assert!(!host.exists(path).unwrap(), "failed first write left a schema-only record");
        host.failedWrites.lock().unwrap().clear();
        if replace { make().replaceRecoverably(record).unwrap(); }
        else { make().edit(|p| p.set(&stringPreferencesKey("record"), json!({"status":"complete"}))).unwrap(); }
        assert_eq!(make().data().unwrap().get(&stringPreferencesKey("record")), Some(&json!({"status":"complete"})));
        assert_eq!(host.writeCount(), 1, "schema and record require one atomic publication");
    }
}
#[test]
fn constrained_failed_record_update_reloads_durable_values_for_all_live_handles() {
    let host = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let path = "runtime/evicted-edit.preferences.json";
    let first = super::CoreNodeStateStore::newWithStorage(host.clone(), path);
    let second = super::CoreNodeStateStore::newWithStorage(host.clone(), path);
    let key = stringPreferencesKey("record");
    first.edit(|p| p.set(&key, json!({"status":"pending"}))).unwrap();
    assert!(first.inner.loadedPreferences().is_none(), "committed node-local edits must release parsed state even with live handles");
    let before = second.data().unwrap();
    host.failedWrites.lock().unwrap().insert(path.into());
    assert!(first.edit(|p| p.set(&key, json!({"status":"approved"}))).is_err());
    assert!(first.inner.loadedPreferences().is_none());
    assert_eq!(second.data().unwrap(), before);
    host.failedWrites.lock().unwrap().clear();
    first.edit(|p| p.set(&key, json!({"status":"approved"}))).unwrap();
    assert!(first.inner.loadedPreferences().is_none());
    first.replaceRecoverably(second.data().unwrap()).unwrap();
    assert!(first.inner.loadedPreferences().is_none(), "unchanged replacement must preserve the uncached read policy");
    assert_eq!(before.get(&key), Some(&json!({"status":"pending"})));
    assert_eq!(second.data().unwrap().get(&key), Some(&json!({"status":"approved"})));
    host.failedWrites.lock().unwrap().insert(path.into());
    assert!(first.replaceRecoverably(before).is_err());
    assert_eq!(second.data().unwrap().get(&key), Some(&json!({"status":"approved"})));
}

#[test]
fn constrained_policy_replay_matches_full_replay_and_rejects_immutable_payload_conflicts() {
    use crate::NetworkControlStore::{NetworkControlStore, NetworkControlIdentityAssignment};
    let host = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let store = NetworkControlStore::newNodeLocal(host.clone()).unwrap();
    let initial = store.bootstrapCurrentSpace().unwrap();
    store.admitMember("peer".into()).unwrap();
    store.setIdentity(NetworkControlIdentityAssignment { nodeId: "peer".into(), roleId: "relay".into() }).unwrap();
    let compare = || {
        let operations = store.currentSpaceOperations().unwrap();
        assert_eq!(store.currentState().unwrap(), store.validateSpacePolicy(&initial.spaceId, &operations).unwrap());
    };
    compare();
    assert!(!store.nodeHasCapability("peer", "runtime.execute", None).unwrap());
    store.clearIdentity("peer".into()).unwrap();
    compare();
    assert!(store.nodeHasCapability("peer", "runtime.execute", None).unwrap());
    store.disconnectNode("peer".into()).unwrap();
    compare();
    assert!(NetworkControlStore::new(host.clone()).unwrap().nodeIsDisconnected("peer").unwrap());
    let mut conflict = store.currentSpaceOperations().unwrap().remove(0);
    conflict.payload["unrecognized"] = json!("immutable evidence must still be compared");
    NetworkControlStore::projectionStore(host).edit(|p| {
        p.set(&stringPreferencesKey("duplicate"), serde_json::to_value(conflict).unwrap());
    }).unwrap();
    assert!(store.currentState().unwrap_err().contains("Conflicting immutable"));
    assert!(store.currentSpaceOperations().unwrap_err().contains("Conflicting immutable"));
}

#[test]
fn constrained_record_insert_moves_retained_payload_and_skips_unchanged_edits() {
    let host = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let store = super::CoreNodeStateStore::newWithStorage(host.clone(), "runtime/owned-edit.json");
    let large = stringPreferencesKey("large");
    store.edit(|p| p.set(&large, "retained payload".repeat(1000))).unwrap();
    store.edit(|p| {
        let pointer = p.get(&large).unwrap().as_str().unwrap().as_ptr();
        p.set(&stringPreferencesKey("another"), json!({"status":"pending"}));
        assert_eq!(p.get(&large).unwrap().as_str().unwrap().as_ptr(), pointer, "unshared retained values must not be copied during insertion");
    }).unwrap();
    let writes = host.writeCount();
    store.edit(|_| ()).unwrap();
    store.edit(|p| p.set(&large, "retained payload".repeat(1000))).unwrap();
    assert_eq!(host.writeCount(), writes, "unchanged polling must not publish NVS again");
    let old = store.data().unwrap();
    store.edit(|p| p.set(&large, "changed")).unwrap();
    assert_eq!(old.get(&large).unwrap().as_str().unwrap(), "retained payload".repeat(1000));
    assert_eq!(store.data().unwrap().get(&large).unwrap(), "changed");
}

#[test]
fn constrained_single_record_read_skips_unrelated_payloads_and_rejects_future_schema() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Record { status: String }
    let host = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let path = "runtime/single-scan.json";
    let make = || super::CoreNodeStateStore::newWithStorage(host.clone(), path).withSchema(1, |_, p| {
        p.set(&stringPreferencesKey("wanted"), json!({"status":"migrated"}));
        Ok(())
    });
    host.writeBytes(path, b"{}").unwrap();
    assert_eq!(make().typedEntry::<Record>("wanted").unwrap().unwrap().status, "migrated");
    make().edit(|p| p.set(&stringPreferencesKey("unrelated"), json!({"large":"payload".repeat(1000)}))).unwrap();
    let writes = host.writeCount();
    assert_eq!(make().typedEntry::<Record>("wanted").unwrap().unwrap().status, "migrated");
    assert!(make().typedEntry::<Record>("absent").unwrap().is_none());
    assert!(make().typedEntries::<Record>().is_err(), "whole scans still validate every requested record");
    assert_eq!(host.writeCount(), writes);
    host.writeBytes(path, &serde_json::to_vec(&json!({PREFERENCES_SCHEMA_VERSION_KEY_NAME: 2, "wanted":{"status":"future"}})).unwrap()).unwrap();
    assert!(matches!(make().typedEntry::<Record>("wanted"), Err(PreferencesDataStoreError::SchemaVersionTooNew { .. })));
}

#[test]
fn bounded_json_buffer_measures_utf8_escapes_and_fails_without_growing() {
    use serde::Serialize;
    let value = json!({"text":"设备\n\"\t", "nested":[true, null, -12, 1.25, {"a":"b"}]});
    for indent in [b"".as_slice(), b"  ".as_slice()] {
        let encode = |writer: &mut super::PreferencesJsonBuffer| {
            value.serialize(&mut serde_json::Serializer::with_formatter(writer,
                serde_json::ser::PrettyFormatter::with_indent(indent)))
        };
        let mut writer = super::PreferencesJsonBuffer { bytes: Vec::new(), measured: 0, countOnly: true, bounded: true };
        encode(&mut writer).unwrap();
        let measured = writer.measured;
        writer.bytes.try_reserve_exact(measured).unwrap();
        writer.countOnly = false;
        let capacity = writer.bytes.capacity();
        encode(&mut writer).unwrap();
        assert_eq!(writer.bytes.len(), measured);
        assert_eq!(writer.bytes.capacity(), capacity);
        let mut expected = Vec::new();
        value.serialize(&mut serde_json::Serializer::with_formatter(&mut expected,
            serde_json::ser::PrettyFormatter::with_indent(indent))).unwrap();
        assert_eq!(writer.bytes, expected);
        let mut tooSmall = super::PreferencesJsonBuffer { bytes: Vec::new(), measured: 0, countOnly: false, bounded: true };
        assert!(encode(&mut tooSmall).is_err());
        assert_eq!(tooSmall.bytes.capacity(), 0);
    }
}

#[test]
fn constrained_typed_replace_preserves_raw_payloads_and_rolls_back_failed_publication() {
    let host = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let path = "runtime/raw-replace.json";
    let make = || super::CoreNodeStateStore::newWithStorage(host.clone(), path).withSchema(1, |_, _| Ok(()));
    // Escaped keys, unknown nested fields and number spellings must survive.
    let original = br#"{"__operit_preferences_schema_version":1,"retained\u0022key":{"unknown":[1e2,{"x":true}]},"request":{"status":"pending"}}"#;
    host.writeBytes(path, original).unwrap();
    let snapshot = make().data().unwrap();
    let approved = json!({"status":"approved", "text":"设备\n\""});
    host.failedWrites.lock().unwrap().insert(path.into());
    assert!(make().setTypedEntry("request", &approved).is_err());
    assert_eq!(host.readBytes(path).unwrap(), original);
    assert_eq!(make().data().unwrap(), snapshot);
    host.failedWrites.lock().unwrap().clear();
    make().setTypedEntry("request", &approved).unwrap();
    let committed = host.readBytes(path).unwrap();
    assert!(std::str::from_utf8(&committed).unwrap().contains("1e2"));
    assert_eq!(make().typedEntry::<serde_json::Value>("request").unwrap(), Some(approved.clone()));
    assert_eq!(snapshot.get(&stringPreferencesKey("request")).unwrap()["status"], "pending");
    let writes = host.writeCount();
    make().setTypedEntry("request", &approved).unwrap();
    assert_eq!(host.writeCount(), writes);
    assert!(make().inner.loadedPreferences().is_none());
    make().setTypedEntry("a", &17).unwrap();
    make().setTypedEntry("z", &19).unwrap();
    assert_eq!(make().typedEntry::<u32>("a").unwrap(), Some(17));
    assert_eq!(make().typedEntry::<u32>("z").unwrap(), Some(19));
    host.failedWrites.lock().unwrap().insert(path.into());
    assert!(make().removeEntry("a").is_err());
    assert_eq!(make().typedEntry::<u32>("a").unwrap(), Some(17));
    host.failedWrites.lock().unwrap().clear();
    make().removeEntry("a").unwrap();
    assert!(make().typedEntry::<u32>("a").unwrap().is_none());
    assert_eq!(make().typedEntry::<u32>("z").unwrap(), Some(19));
    let writes = host.writeCount();
    make().removeEntry("absent").unwrap();
    assert_eq!(host.writeCount(), writes);
}

#[test]
fn constrained_typed_replace_migrates_atomically_and_rejects_future_or_corrupt_files() {
    let host = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let path = "runtime/raw-replace-schema.json";
    let make = || super::CoreNodeStateStore::newWithStorage(host.clone(), path).withSchema(1, |_, p| {
        p.set(&stringPreferencesKey("migrated"), true);
        Ok(())
    });
    host.failedWrites.lock().unwrap().insert(path.into());
    assert!(make().setTypedEntry("request", &"pending").is_err());
    assert!(!host.exists(path).unwrap());
    host.failedWrites.lock().unwrap().clear();
    make().setTypedEntry("request", &"pending").unwrap();
    assert_eq!(make().typedEntry::<bool>("migrated").unwrap(), Some(true));
    let future = br#"{"__operit_preferences_schema_version":2}"#;
    host.writeBytes(path, future).unwrap();
    assert!(matches!(make().setTypedEntry("request", &true), Err(PreferencesDataStoreError::SchemaVersionTooNew { .. })));
    assert_eq!(host.readBytes(path).unwrap(), future);
    for corrupt in [b"[]".as_slice(), b"{broken", b"{\"bad\":null} trailing"] {
        host.writeBytes(path, corrupt).unwrap();
        assert!(make().setTypedEntry("request", &true).is_err());
        assert_eq!(host.readBytes(path).unwrap(), corrupt);
    }
}

#[test]
fn constrained_whole_replace_preserves_replication_and_atomic_failure_semantics() {
    let host = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let path = "runtime/compact-whole-replace.json";
    let make = || super::CoreNodeStateStore::newWithStorage(host.clone(), path).withSchema(1, |_, _| Ok(()));
    let original = json!({PREFERENCES_SCHEMA_VERSION_KEY_NAME:1, "old":{"status":"pending"},
        "__operit_preferences_replication":{"initialized":true,"logicalTime":17,"sequences":{"peer":3}}});
    host.writeBytes(path, &serde_json::to_vec(&original).unwrap()).unwrap();
    let before = make().data().unwrap();
    let mut next = Preferences::default();
    next.set(&stringPreferencesKey("new"), json!({"status":"approved"}));
    host.failedWrites.lock().unwrap().insert(path.into());
    assert!(make().replace(next.clone()).is_err());
    assert_eq!(make().data().unwrap(), before);
    host.failedWrites.lock().unwrap().clear();
    make().replace(next.clone()).unwrap();
    let saved: Value = serde_json::from_slice(&host.readBytes(path).unwrap()).unwrap();
    assert_eq!(saved["__operit_preferences_replication"]["logicalTime"], 17);
    assert_eq!(saved["__operit_preferences_replication"]["sequences"]["peer"], 3);
    assert!(saved.get("old").is_none());
    assert_eq!(saved[PREFERENCES_SCHEMA_VERSION_KEY_NAME], 1);
    assert!(make().inner.loadedPreferences().is_none());
    let writes = host.writeCount();
    make().replace(next).unwrap();
    assert_eq!(host.writeCount(), writes);
    assert!(before.get(&stringPreferencesKey("old")).is_some());
}

#[test]
fn constrained_encrypted_whole_replace_keeps_secrets_encrypted() {
    let host = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let path = "runtime/compact-secret-replace.json";
    let make = || CoreNodeSecretStore::newWithStorage(host.clone(), path);
    for token in ["first-secret-token", "second-secret-token"] {
        let mut preferences = Preferences::default();
        preferences.set(&stringPreferencesKey("token"), token);
        make().replace(preferences).unwrap();
        assert_eq!(make().data().unwrap().get(&stringPreferencesKey("token")), Some(&json!(token)));
        let bytes = host.readBytes(path).unwrap();
        let encoded = std::str::from_utf8(&bytes).unwrap();
        assert!(encoded.contains("operit.preferences.encrypted"));
        assert!(!encoded.contains(token));
    }
}

#[test]
fn immutable_record_batch_validates_all_conflicts_before_one_atomic_publication() {
    for release in [false, true] {
        let host = Arc::new(MemoryStorageHost { releasePreferencesCache: release, ..Default::default() });
        let path = "runtime/immutable-batch.json";
        let make = || super::CoreNodeStateStore::newWithStorage(host.clone(), path).withSchema(1, |_, _| Ok(()));
        let old = json!({"status":"kept", "unknown":{"signed":true}});
        make().setTypedEntry("b", &old).unwrap();
        let snapshot = make().data().unwrap();
        let new = json!({"status":"new"});
        let conflict = json!({"status":"kept", "unknown":{"signed":false}});
        let writes = host.writeCount();
        assert!(make().mergeImmutableEntries(&[("a", &new), ("b", &conflict)]).is_err());
        assert!(make().mergeImmutableEntries(&[("a", &new), ("a", &conflict)]).is_err());
        assert_eq!(host.writeCount(), writes);
        assert_eq!(make().data().unwrap(), snapshot);
        host.failedWrites.lock().unwrap().insert(path.into());
        assert!(make().mergeImmutableEntries(&[("a", &new), ("中\"", &new)]).is_err());
        assert!(make().typedEntry::<Value>("a").unwrap().is_none());
        host.failedWrites.lock().unwrap().clear();
        let writes = host.writeCount();
        make().mergeImmutableEntries(&[("a", &new), ("b", &old), ("中\"", &new), ("a", &new)]).unwrap();
        assert_eq!(host.writeCount(), writes + 1);
        assert_eq!(make().typedEntry::<Value>("a").unwrap(), Some(new.clone()));
        assert_eq!(make().typedEntry::<Value>("中\"").unwrap(), Some(new.clone()));
        assert_eq!(make().typedEntry::<Value>("b").unwrap(), Some(old.clone()));
        let writes = host.writeCount();
        make().mergeImmutableEntries(&[("b", &old), ("a", &new)]).unwrap();
        assert_eq!(host.writeCount(), writes);
        assert!(snapshot.get(&stringPreferencesKey("a")).is_none());
    }
}

#[test]
fn immutable_record_batch_keeps_schema_migration_atomic_and_rejects_future_versions() {
    let host = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let path = "runtime/immutable-batch-schema.json";
    let make = || super::CoreNodeStateStore::newWithStorage(host.clone(), path).withSchema(1, |_, p| {
        p.set(&stringPreferencesKey("migrated"), true);
        Ok(())
    });
    host.writeBytes(path, br#"{"kept":true}"#).unwrap();
    assert!(make().mergeImmutableEntries(&[("new", &true), ("kept", &false)]).is_err());
    assert_eq!(host.readBytes(path).unwrap(), br#"{"kept":true}"#);
    make().mergeImmutableEntries(&[("new", &true)]).unwrap();
    assert_eq!(make().typedEntry::<bool>("migrated").unwrap(), Some(true));
    host.writeBytes(path, br#"{"__operit_preferences_schema_version":2}"#).unwrap();
    assert!(matches!(make().mergeImmutableEntries(&[("new", &true)]), Err(PreferencesDataStoreError::SchemaVersionTooNew { .. })));
}


#[test]
fn encoded_node_snapshot_is_immutable_and_releases_the_transaction_before_callback() {
    for release in [false, true] {
        let host = Arc::new(MemoryStorageHost { releasePreferencesCache: release, ..Default::default() });
        let store = super::CoreNodeStateStore::newWithStorage(host, "runtime/borrowed-node.json")
            .withSchema(1, |_, _| Ok(()));
        let original = json!({"unknown": {"value": u64::MAX}, "array": [true, null, -1]});
        store.setTypedEntry("中\"", &original).unwrap();
        let returned = store.withEncodedEntries(|entries| {
            assert_eq!(entries.len(), 1);
            store.setTypedEntry("中\"", &json!({"replacement":true})).unwrap();
            serde_json::from_str::<Value>(entries["中\""].get()).unwrap()
        }).unwrap();
        assert_eq!(returned, original);
        assert_eq!(store.typedEntry::<Value>("中\"").unwrap(), Some(json!({"replacement":true})));
    }
}

#[test]
fn encoded_node_snapshot_migrates_old_schema_and_rejects_future_or_corrupt_records() {
    let host = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let path = "runtime/borrowed-schema.json";
    let store = super::CoreNodeStateStore::newWithStorage(host.clone(), path).withSchema(1, |_, preferences| {
        preferences.set(&stringPreferencesKey("migrated"), true);
        Ok(())
    });
    host.writeBytes(path, br#"{"kept":{"unknown":1}}"#).unwrap();
    let entries = store.withEncodedEntries(|entries| entries.into_iter()
        .map(|(key, value)| (key, serde_json::from_str::<Value>(value.get()).unwrap())).collect::<BTreeMap<_, _>>()).unwrap();
    assert_eq!(entries["kept"], json!({"unknown":1}));
    assert_eq!(entries["migrated"], json!(true));
    for invalid in [br#"{"__operit_preferences_schema_version":2}"#.as_slice(), br#"{"broken": [}"#] {
        host.writeBytes(path, invalid).unwrap();
        assert!(store.withEncodedEntries(|_| panic!("invalid schema/JSON reached callback")).is_err());
    }
}

/// Compact publication must continue reading legacy pretty files and preserve
/// unknown nested data and whitespace inside strings without a schema migration.
#[test]
fn constrained_compact_json_preserves_legacy_records_and_unknown_fields() {
    let host = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let path = "runtime/legacy-pretty-node.json";
    let legacy = json!({"keep":{"future":[null, {"text":"设备  \ntext"}]}, "change":{"value":1}});
    host.writeBytes(path, &serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();
    let store = super::CoreNodeStateStore::newWithStorage(host.clone(), path);
    let next = json!({"value":2,"future":{"unknown":true}});
    store.setTypedEntry("change", &next).unwrap();
    let bytes = host.readBytes(path).unwrap();
    let actual: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(actual["keep"], legacy["keep"]);
    assert_eq!(actual["change"], next);
    assert_eq!(store.typedEntry::<Value>("keep").unwrap(), Some(legacy["keep"].clone()));
    let writes = host.writeCount();
    store.setTypedEntry("change", &next).unwrap();
    assert_eq!(host.writeCount(), writes);
}

/// Mutable receipt normalization must preserve raw JSON tokens and rollback,
/// including escaped quotes/backslashes split across serializer writes.
#[test]
fn constrained_opt_in_compaction_preserves_tokens_and_atomic_retry() {
    let host = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let path = "runtime/mutable-receipts.json";
    let legacy = br#"{
        "keep" : { "number" : 1.00e+02, "text" : "a  \"quote\" \\ end\n", "unicode" : "\u4e2d" },
        "change" : { "status" : "pending" }
    }"#;
    host.writeBytes(path, legacy).unwrap();
    let store = super::CoreNodeStateStore::newWithStorage(host.clone(), path);
    let next = json!({"status":"approved"});
    host.failedWrites.lock().unwrap().insert(path.into());
    assert!(store.setTypedEntryCompactingJson("change", &next).is_err());
    assert_eq!(host.readBytes(path).unwrap(), legacy);
    host.failedWrites.lock().unwrap().clear();
    store.setTypedEntryCompactingJson("change", &next).unwrap();
    let bytes = host.readBytes(path).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.contains(r#""number":1.00e+02"#));
    assert!(text.contains(r#""unicode":"\u4e2d""#));
    let actual: Value = serde_json::from_slice(&bytes).unwrap();
    let old: Value = serde_json::from_slice(legacy).unwrap();
    assert_eq!(actual["keep"], old["keep"]);
    assert_eq!(actual["change"], next);
    assert!(bytes.len() < legacy.len());
    let writes = host.writeCount();
    store.setTypedEntryCompactingJson("change", &next).unwrap();
    assert_eq!(host.writeCount(), writes);
    // Existing typed writes keep immutable neighbours byte-for-byte.
    host.writeBytes(path, legacy).unwrap();
    store.setTypedEntry("change", &next).unwrap();
    let normal = host.readBytes(path).unwrap();
    assert!(std::str::from_utf8(&normal).unwrap().contains(r#""number" : 1.00e+02"#));
}

#[test]
fn encoded_entry_stream_checks_schema_and_whole_json_before_callbacks() {
    let host = Arc::new(MemoryStorageHost { releasePreferencesCache: true, ..Default::default() });
    let path = "runtime/streamed-entries.json";
    let store = super::CoreNodeStateStore::newWithStorage(host.clone(), path).withSchema(2, |_, _| Ok(()));
    for invalid in [br#"{"entry":true,"__operit_preferences_schema_version":3}"#.as_slice(),
        br#"{"__operit_preferences_schema_version":2,"entry":true} trailing"#] {
        host.writeBytes(path, invalid).unwrap();
        let mut calls = 0;
        assert!(store.visitEncodedEntries(|_, _| { calls += 1; Ok(()) }).is_err());
        assert_eq!(calls, 0);
        assert_eq!(host.readBytes(path).unwrap(), invalid);
    }
}
