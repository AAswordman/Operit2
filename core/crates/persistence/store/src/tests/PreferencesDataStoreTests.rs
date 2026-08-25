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
                    .get(&stringPreferencesKey("first"))
                    .expect("version-one data");
                preferences.set(&stringPreferencesKey("second"), format!("{first}-migrated"));
                Ok(())
            }
            from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
        });

    let preferences = store.data().expect("schema migration");
    assert_eq!(
        preferences.get(&stringPreferencesKey("first")),
        Some(&"created".to_string())
    );
    assert_eq!(
        preferences.get(&stringPreferencesKey("second")),
        Some(&"created-migrated".to_string())
    );
    assert_eq!(
        preferences.get(&stringPreferencesKey(PREFERENCES_SCHEMA_VERSION_KEY_NAME)),
        Some(&"2".to_string())
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
        Some(&"first-host-token".to_string())
    );
    assert_eq!(
        secondStore
            .data()
            .expect("second host preferences read")
            .get(&stringPreferencesKey("access_token")),
        Some(&"second-host-token".to_string())
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
        Some(&"secret-token".to_string())
    );
    assert_eq!(
        roundTrip.get(&stringPreferencesKey("token_type")),
        Some(&"bearer".to_string())
    );
    assert_eq!(
        roundTrip.get(&stringPreferencesKey("user_info")),
        Some(&"{\"login\":\"codex\"}".to_string())
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
    assert_eq!(operations.len(), 2);
    let apiKeyOperation = operations
        .iter()
        .find(|operation| operation.payload["key"] == "api_key")
        .expect("API key operation");
    assert_eq!(apiKeyOperation.operation, "set");
    assert_eq!(apiKeyOperation.payload["value"], "sk-model-secret");
    assert_eq!(apiKeyOperation.payload["encrypted"], true);
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
        Some(&"dark".to_string())
    );
    assert_eq!(
        preferences.get(&stringPreferencesKey("language")),
        Some(&"zh-CN".to_string())
    );
}

#[test]
/// Verifies structured JSON sync sends leaf values and merges concurrent provider edits.
fn structured_json_preferences_merge_provider_fields_and_model_items() {
    let firstHost = Arc::new(MemoryStorageHost::default());
    let secondHost = Arc::new(MemoryStorageHost::default());
    let targetHost = Arc::new(MemoryStorageHost::default());
    let storagePath = format!("{CONFIG_PREFERENCES_DIR_PATH}/structured_merge.preferences.json");
    let providerKey = stringPreferencesKey("provider_test");
    let baselineProvider = json!({
        "id": "provider-test",
        "name": "Provider",
        "endpoint": "https://old.example.com",
        "apiKey": "sk-old",
        "optionalConfig": {"enabled": true},
        "models": [
            {
                "id": "model-a",
                "parameters": [
                    {
                        "id": "temperature",
                        "currentValue": 0.2,
                        "description": "x".repeat(2048)
                    }
                ]
            }
        ]
    });
    let baselineEncoded = serde_json::to_string(&baselineProvider).expect("baseline provider JSON");

    let firstStore = PreferencesDataStore::newWithStorage(firstHost.clone(), storagePath.clone())
        .withStructuredJsonSync();
    let secondStore = PreferencesDataStore::newWithStorage(secondHost.clone(), storagePath.clone())
        .withStructuredJsonSync();
    let targetStore = PreferencesDataStore::newSpaceWithResolvedPath(
        targetHost.clone(),
        std::path::PathBuf::from("D:/runtime/physical-structured-merge.preferences.json"),
        storagePath.clone(),
        false,
    )
    .withStructuredJsonSync();
    for store in [&firstStore, &secondStore, &targetStore] {
        store
            .edit(|preferences| preferences.set(&providerKey, baselineEncoded.clone()))
            .expect("baseline provider write");
    }
    let firstClock = SyncOperationStore::new(firstHost.clone(), RUNTIME_SYNC_DIR_PATH)
        .localClock()
        .expect("first baseline clock");
    let secondClock = SyncOperationStore::new(secondHost.clone(), RUNTIME_SYNC_DIR_PATH)
        .localClock()
        .expect("second baseline clock");

    firstStore
        .edit(|preferences| {
            let mut provider: Value = serde_json::from_str(
                preferences
                    .get(&providerKey)
                    .expect("first provider preference"),
            )
            .expect("first provider JSON");
            provider["apiKey"] = json!("sk-new");
            provider["optionalConfig"] = Value::Null;
            provider["models"][0]["parameters"][0]["currentValue"] = json!(0.7);
            preferences.set(
                &providerKey,
                serde_json::to_string(&provider).expect("updated first provider JSON"),
            );
        })
        .expect("first structured edit");
    secondStore
        .edit(|preferences| {
            let mut provider: Value = serde_json::from_str(
                preferences
                    .get(&providerKey)
                    .expect("second provider preference"),
            )
            .expect("second provider JSON");
            provider["endpoint"] = json!("https://new.example.com");
            preferences.set(
                &providerKey,
                serde_json::to_string(&provider).expect("updated second provider JSON"),
            );
        })
        .expect("second structured edit");

    let mut operations = SyncOperationStore::new(firstHost, RUNTIME_SYNC_DIR_PATH)
        .operationsSince(&firstClock, &["preferences".to_string()], 10)
        .expect("first structured operations");
    operations.extend(
        SyncOperationStore::new(secondHost, RUNTIME_SYNC_DIR_PATH)
            .operationsSince(&secondClock, &["preferences".to_string()], 10)
            .expect("second structured operations"),
    );
    assert_eq!(operations.len(), 4);
    let apiKeyOperation = operations
        .iter()
        .find(|operation| operation.payload["jsonMutation"]["value"] == "sk-new")
        .expect("API key leaf operation");
    assert!(
        serde_json::to_vec(&apiKeyOperation.payload)
            .expect("API key operation payload")
            .len()
            < baselineEncoded.len()
    );

    targetStore.data().expect("loaded target preferences");
    let changeVersionBeforeSync = *targetStore
        .changeSignal
        .version
        .lock()
        .expect("change version mutex");
    let writesBeforeSync = targetHost.writeCount();
    let entries = operations
        .iter()
        .map(PreferencesSyncedEntry::fromOperation)
        .collect::<Result<Vec<_>, _>>()
        .expect("decoded structured preference operations");
    PreferencesDataStore::applySyncedEntriesWithStorage(targetHost.clone(), &entries)
        .expect("structured preference operation application");
    assert_eq!(targetHost.writeCount(), writesBeforeSync + 1);
    assert_eq!(
        *targetStore
            .changeSignal
            .version
            .lock()
            .expect("change version mutex"),
        changeVersionBeforeSync + 1
    );
    let preferences = targetStore.data().expect("merged structured preferences");
    let provider: Value = serde_json::from_str(
        preferences
            .get(&providerKey)
            .expect("merged provider preference"),
    )
    .expect("merged provider JSON");
    assert_eq!(provider["apiKey"], "sk-new");
    assert_eq!(provider["endpoint"], "https://new.example.com");
    assert_eq!(provider["optionalConfig"], Value::Null);
    assert_eq!(provider["models"][0]["parameters"][0]["currentValue"], 0.7);
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
        Some(&"sk-test".to_string())
    );
    assert_eq!(
        preferences.get(&stringPreferencesKey("provider_list")),
        Some(&"[\"DEEPSEEK\"]".to_string())
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
        Some(&"light".to_string())
    );
    assert_eq!(
        store
            .data()
            .expect("updated preferences read")
            .get(&stringPreferencesKey("theme")),
        Some(&"dark".to_string())
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
