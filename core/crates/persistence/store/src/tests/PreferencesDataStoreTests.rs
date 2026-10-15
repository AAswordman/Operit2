use std::collections::BTreeMap;
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
use operit_util::RuntimeStorageLayout::{
    CONFIG_PREFERENCES_DIR_PATH, GITHUB_AUTH_PREFERENCES_PATH, MODEL_CONFIGS_PREFERENCES_PATH,
    PREFERENCES_ENCRYPTION_KEY_PATH, RUNTIME_SYNC_DIR_PATH,
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
    files: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    readCount: Arc<AtomicUsize>,
    writeCount: Arc<AtomicUsize>,
    blockedWrite: Arc<Mutex<Option<BlockedWrite>>>,
}

struct BlockedWrite {
    started: Sender<()>,
    release: Receiver<()>,
}

impl MemoryStorageHost {
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

    fn writeBytes(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
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
        Ok(())
    }

    /// Appends bytes to one in-memory preferences storage entry.
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
    let store =
        PreferencesDataStore::newEncryptedWithStorage(host.clone(), MODEL_CONFIGS_PREFERENCES_PATH);

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
    let entries = operations
        .iter()
        .map(PreferencesSyncedEntry::fromOperation)
        .collect::<Result<Vec<_>, _>>()
        .expect("decoded preference operations");
    PreferencesDataStore::applySyncedEntriesWithStorage(targetHost.clone(), &entries)
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
    let targetStore = PreferencesDataStore::newWithStorage(targetHost.clone(), path);
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
    let entries = operations
        .iter()
        .map(PreferencesSyncedEntry::fromOperation)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    PreferencesDataStore::applySyncedEntriesWithStorage(targetHost, &entries).unwrap();
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
    let entries = operations
        .iter()
        .map(PreferencesSyncedEntry::fromOperation)
        .collect::<Result<Vec<_>, _>>()
        .expect("decode source operations");
    PreferencesDataStore::applySyncedEntriesWithStorage(targetHost, &entries)
        .expect("new device must receive the configured model");
    let preferences = targetStore.data().expect("synced provider");
    let actual = preferences.get(&providerKey).unwrap().clone();
    assert_eq!(actual, provider);
}

#[test]
fn preference_sync_batch_validation_keeps_chat_binding_when_model_file_fails() {
    use operit_util::RuntimeStorageLayout::FUNCTIONAL_CONFIGS_PREFERENCES_PATH;
    let sourceHost = Arc::new(MemoryStorageHost::default());
    let targetHost = Arc::new(MemoryStorageHost::default());
    let bindingKey = stringPreferencesKey("function_model_binding");
    let providerKey = stringPreferencesKey("provider_DEEPSEEK");
    let sourceBindings = PreferencesDataStore::newWithStorage(
        sourceHost.clone(),
        FUNCTIONAL_CONFIGS_PREFERENCES_PATH,
    );
    let sourceModels =
        PreferencesDataStore::newWithStorage(sourceHost.clone(), MODEL_CONFIGS_PREFERENCES_PATH);
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
    let entries = operations
        .iter()
        .map(PreferencesSyncedEntry::fromOperation)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    targetHost
        .writeBytes(MODEL_CONFIGS_PREFERENCES_PATH, b"{")
        .unwrap();
    let writesBefore = targetHost.writeCount();
    let error =
        PreferencesDataStore::applySyncedPreferencesWithStorage(targetHost.clone(), &entries)
            .expect_err("a malformed model preference file cannot be applied");
    assert!(matches!(error, PreferencesDataStoreError::Json(_)));
    assert_eq!(targetHost.writeCount(), writesBefore);
    assert_eq!(
        targetBindings.data().unwrap().get(&bindingKey),
        Some(&originalBinding)
    );
}

#[test]
fn preference_sync_batch_publishes_models_before_binding_observers_run() {
    operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(Arc::new(
        ImmediatePreferenceObserverScheduler,
    ));
    use operit_util::RuntimeStorageLayout::FUNCTIONAL_CONFIGS_PREFERENCES_PATH;
    let sourceHost = Arc::new(MemoryStorageHost::default());
    let targetHost = Arc::new(MemoryStorageHost::default());
    let bindingKey = stringPreferencesKey("function_model_binding");
    let providerKey = stringPreferencesKey("provider_DEEPSEEK");
    let sourceBindings = PreferencesDataStore::newWithStorage(
        sourceHost.clone(),
        FUNCTIONAL_CONFIGS_PREFERENCES_PATH,
    );
    let sourceModels =
        PreferencesDataStore::newWithStorage(sourceHost.clone(), MODEL_CONFIGS_PREFERENCES_PATH);
    let targetBindings = PreferencesDataStore::newWithStorage(
        targetHost.clone(),
        FUNCTIONAL_CONFIGS_PREFERENCES_PATH,
    );
    let targetModels =
        PreferencesDataStore::newWithStorage(targetHost.clone(), MODEL_CONFIGS_PREFERENCES_PATH);
    let binding = json!({"CHAT": {"providerId": "DEEPSEEK", "modelId": "deepseek-flash"}});
    let provider = json!({"id": "DEEPSEEK", "models": [{"id": "deepseek-flash"}]});
    sourceBindings
        .edit(|preferences| preferences.set(&bindingKey, binding.clone()))
        .unwrap();
    sourceModels
        .edit(|preferences| preferences.set(&providerKey, provider.clone()))
        .unwrap();
    let observed = Arc::new(Mutex::new(Vec::new()));
    let observedForSubscription = observed.clone();
    let bindingKeyForSubscription = bindingKey.clone();
    let providerKeyForSubscription = providerKey.clone();
    let _subscription = targetBindings
        .dataFlow()
        .subscribeWithCancellation(super::FlowCancellation::new(), move |preferences| {
            if preferences.get(&bindingKeyForSubscription).is_some() {
                observedForSubscription.lock().unwrap().push(
                    targetModels
                        .data()
                        .unwrap()
                        .get(&providerKeyForSubscription)
                        .cloned(),
                );
            }
        })
        .unwrap();
    let operations = SyncOperationStore::new(sourceHost, RUNTIME_SYNC_DIR_PATH)
        .operationsSince(&SyncClock::empty(), &["preferences".to_string()], 512)
        .unwrap();
    let entries = operations
        .iter()
        .map(PreferencesSyncedEntry::fromOperation)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    PreferencesDataStore::applySyncedPreferencesWithStorage(targetHost, &entries).unwrap();
    assert_eq!(*observed.lock().unwrap(), vec![Some(provider.clone())]);
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

fn synchronizedPreferenceSet(path: &str, encrypted: bool) -> PreferencesSyncedEntry {
    PreferencesSyncedEntry {
        storagePath: path.to_string(),
        encrypted,
        schemaVersion: None,
        formatVersion: 2,
        mutation: super::PreferencesSyncedMutation::SetEntry {
            key: "setting".to_string(),
            value: json!("synced-value"),
        },
    }
}

#[test]
fn preference_sync_replay_skips_unchanged_files_and_notifications() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = PreferencesDataStore::newWithStorage(host.clone(), MODEL_CONFIGS_PREFERENCES_PATH);
    let entries = [synchronizedPreferenceSet(
        MODEL_CONFIGS_PREFERENCES_PATH,
        false,
    )];
    PreferencesDataStore::applySyncedPreferencesWithStorage(host.clone(), &entries).unwrap();
    let writes = host.writeCount();
    let version = *store.changeSignal.version.lock().unwrap();
    PreferencesDataStore::applySyncedPreferencesWithStorage(host.clone(), &entries).unwrap();
    PreferencesDataStore::applySyncedPreferencesWithStorage(host.clone(), &[]).unwrap();
    assert_eq!(host.writeCount(), writes);
    assert_eq!(*store.changeSignal.version.lock().unwrap(), version);
}

#[test]
fn preference_sync_rejects_mixed_encryption_before_loading_or_writing_files() {
    let host = Arc::new(MemoryStorageHost::default());
    let entries = [
        synchronizedPreferenceSet(MODEL_CONFIGS_PREFERENCES_PATH, false),
        synchronizedPreferenceSet(MODEL_CONFIGS_PREFERENCES_PATH, true),
    ];
    PreferencesDataStore::applySyncedPreferencesWithStorage(host.clone(), &entries)
        .expect_err("a preferences file must have one encryption mode");
    assert_eq!(host.writeCount(), 0);
    assert_eq!(host.readCount(), 0);
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
                "storagePath":path, "encrypted":false, "formatVersion":2,
                "entries":entries.into_iter().map(|(key, value)| json!({"key":key,"operation":"set","value":value})).collect::<Vec<_>>()
            }),
        )
    } else {
        (
            super::preferenceMutationEntityId(path, entries[0].0, &[]).unwrap(),
            json!({
                "storagePath":path, "encrypted":false, "formatVersion":2, "key":entries[0].0,"value":entries[0].1
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
    PreferencesDataStore::applySyncedOperationsWithStorage(forward.clone(), &[], &[first.clone()])
        .unwrap();
    PreferencesDataStore::applySyncedOperationsWithStorage(
        forward.clone(),
        &[first.clone()],
        &[later.clone()],
    )
    .unwrap();
    PreferencesDataStore::applySyncedOperationsWithStorage(reverse.clone(), &[], &[later.clone()])
        .unwrap();
    PreferencesDataStore::applySyncedOperationsWithStorage(reverse.clone(), &[later], &[first])
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

/// Rejects a duplicate business key before reading or publishing any preference file.
#[test]
fn preference_transaction_validation_rejects_the_entire_batch() {
    let path = "runtime/config/preferences/invalid-transaction.preferences.json";
    let host = Arc::new(MemoryStorageHost::default());
    let valid = nativePreferenceOperation(path, vec![("first", json!(true))], 1);
    let mut invalid = nativePreferenceOperation(path, vec![("A", json!(1)), ("B", json!(2))], 2);
    invalid.payload["entries"][1]["key"] = json!("A");
    assert!(PreferencesDataStore::applySyncedOperationsWithStorage(
        host.clone(),
        &[],
        &[valid, invalid]
    )
    .is_err());
    assert_eq!(host.writeCount(), 0);
    assert_eq!(host.readCount(), 0);
}

/// Keeps schema metadata inside a complete migration payload so a new peer does not rerun legacy conversion.
#[test]
fn preference_migration_sync_initializes_the_receiving_schema() {
    let path = "runtime/config/preferences/schema-sync.preferences.json";
    let source = Arc::new(MemoryStorageHost::default());
    source.writeBytes(path, &serde_json::to_vec(&json!({
        PREFERENCES_SCHEMA_VERSION_KEY_NAME:"1", "A":"{\"name\":\"legacy\"}", "obsolete_list":"[\"A\"]"
    })).unwrap()).unwrap();
    let sourceStore =
        PreferencesDataStore::newWithStorage(source.clone(), path).withSchema(2, |version, p| {
            match version {
                0 => Ok(()),
                1 => {
                    let key = stringPreferencesKey("A");
                    let value: Value = serde_json::from_str(&p.getTyped::<String>(&key)?.unwrap())?;
                    p.set(&key, value);
                    p.remove(&stringPreferencesKey("obsolete_list"));
                    Ok(())
                }
                from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
            }
        });
    sourceStore.data().unwrap();
    let operations = SyncOperationStore::new(source, RUNTIME_SYNC_DIR_PATH)
        .operationsSince(&SyncClock::empty(), &["preferences".into()], 100)
        .unwrap();
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].payload["schemaVersion"], 2);
    assert!(operations[0].payload["entries"]
        .as_array()
        .unwrap()
        .iter()
        .all(|entry| entry["key"] != PREFERENCES_SCHEMA_VERSION_KEY_NAME));
    let target = Arc::new(MemoryStorageHost::default());
    PreferencesDataStore::applySyncedOperationsWithStorage(target.clone(), &[], &operations)
        .unwrap();
    let targetStore = PreferencesDataStore::newWithStorage(target, path).withSchema(2, |_, _| {
        panic!("native synchronized schema must not execute a legacy migration")
    });
    assert_eq!(targetStore.schemaVersion().unwrap(), 2);
    assert_eq!(
        targetStore.data().unwrap().get(&stringPreferencesKey("A")),
        Some(&json!({"name":"legacy"}))
    );
}

/// A migration supersedes historical encoded values even if a stale operation arrives after it.
#[test]
fn preference_migration_replay_supersedes_legacy_encoded_root_values() {
    let path = "runtime/config/preferences/legacy-replay.preferences.json";
    let mut old = nativePreferenceOperation(path, vec![("A", json!("{\"name\":\"old\"}"))], 1);
    old.payload.as_object_mut().unwrap().remove("formatVersion");
    let native = nativePreferenceOperation(path, vec![("A", json!({"name":"native"}))], 2);
    let host = Arc::new(MemoryStorageHost::default());
    PreferencesDataStore::applySyncedOperationsWithStorage(host.clone(), &[], &[native.clone()])
        .unwrap();
    PreferencesDataStore::applySyncedOperationsWithStorage(host.clone(), &[native], &[old])
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
    PreferencesDataStore::applySyncedOperationsWithStorageAndFilter(
        host.clone(),
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
    PreferencesDataStore::applySyncedOperationsWithStorage(host, &history, &[remote]).unwrap();
    assert_eq!(
        data.data().unwrap().get(&stringPreferencesKey("A")),
        Some(&json!({"name":"local"}))
    );
}

/// A stale legacy writer must not replace native values in an already migrated file.
#[test]
fn preference_sync_rejects_legacy_payloads_on_a_migrated_owner() {
    let path = "runtime/config/preferences/stale-owner.preferences.json";
    let host = Arc::new(MemoryStorageHost::default());
    let store = PreferencesDataStore::newNodeLocalWithStorage(host.clone(), path, false)
        .withSchema(2, |_, _| Ok(()));
    store
        .edit(|p| p.set(&stringPreferencesKey("A"), json!({"name":"native"})))
        .unwrap();
    let before = host.writeCount();
    let mut old = nativePreferenceOperation(path, vec![("A", json!("encoded"))], 1);
    old.payload.as_object_mut().unwrap().remove("formatVersion");
    assert!(
        PreferencesDataStore::applySyncedOperationsWithStorage(host.clone(), &[], &[old]).is_err()
    );
    assert_eq!(host.writeCount(), before);
    assert_eq!(
        store.data().unwrap().get(&stringPreferencesKey("A")),
        Some(&json!({"name":"native"}))
    );
}

/// Replaying a complete newer schema snapshot never executes older native schema operations on the current file.
#[test]
fn preference_replay_supersedes_older_native_schema_history() {
    let path = "runtime/config/preferences/native-version-replay.preferences.json";
    let host = Arc::new(MemoryStorageHost::default());
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
    PreferencesDataStore::applySyncedOperationsWithStorage(host.clone(), &[], &[new.clone()])
        .unwrap();
    PreferencesDataStore::applySyncedOperationsWithStorage(host.clone(), &[new], &[old]).unwrap();
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
