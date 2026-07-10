use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use operit_host_api::{HostError, HostSecretStore, RuntimeStorageEntry, RuntimeStorageHost};
use serde_json::Value;

use crate::PreferencesEncryption::tests::{
    loadOrCreateWithSecretStoreForTest, ENCRYPTION_HOST_SECRET_KEY_FOR_TEST,
};
use crate::SyncOperationStore::{SyncClock, SyncOperationStore};
use operit_util::RuntimeStorageLayout::MODEL_CONFIGS_PREFERENCES_PATH;

use super::{
    combine2, combine5, mutableStateFlow, stringPreferencesKey, Preferences, PreferencesDataStore,
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
}

#[derive(Clone, Default)]
struct MemorySecretStore {
    secrets: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

impl RuntimeStorageHost for MemoryStorageHost {
    fn rootDir(&self) -> Option<std::path::PathBuf> {
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

    fn writeBytes(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
        let mut files = self
            .files
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        files.insert(path.to_string(), content.to_vec());
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

    host.writeBytes("secure/preferences_encryption_key.json", legacyKey)
        .expect("legacy key write");

    let encryption = loadOrCreateWithSecretStoreForTest(&host, Some(&secretStore))
        .expect("migrated encryption key");
    let encrypted = encryption
        .encrypt(
            "runtime/config/preferences/migration_test.json",
            b"secret preferences",
        )
        .expect("encrypted bytes");
    let decrypted = encryption
        .decrypt("runtime/config/preferences/migration_test.json", &encrypted)
        .expect("decrypted bytes");

    assert_eq!(decrypted, b"secret preferences");
    assert_eq!(
        secretStore
            .readSecret(ENCRYPTION_HOST_SECRET_KEY_FOR_TEST)
            .expect("host secret read"),
        Some(legacyKey.to_vec())
    );
    assert_eq!(
        host.exists("secure/preferences_encryption_key.json")
            .expect("legacy key exists check"),
        false
    );
}

#[test]
fn encrypted_store_round_trips_without_plaintext_file() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = PreferencesDataStore::newEncryptedWithStorage(
        host.clone(),
        "runtime/config/preferences/github_auth_preferences.json",
    );

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
        .readBytes("runtime/config/preferences/github_auth_preferences.json")
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
    let storagePath = "runtime/config/preferences/legacy_encrypted.preferences.json";
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
    host.writeBytes(storagePath, &plaintext)
        .expect("legacy plaintext preferences write");

    let store = PreferencesDataStore::newEncryptedWithStorage(host.clone(), storagePath);
    let loaded = store.data().expect("migrated preferences read");

    assert_eq!(loaded, legacyPreferences);
    let stored = host
        .readBytes(storagePath)
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
/// Verifies encrypted-only stores write no sync operations.
fn encrypted_store_does_not_record_sync_operations() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = PreferencesDataStore::newEncryptedWithStorage(
        host.clone(),
        "runtime/config/preferences/github_auth_preferences.json",
    );

    let mut preferences = Preferences::default();
    preferences.set(
        &stringPreferencesKey("access_token"),
        "github-secret-token".to_string(),
    );

    store.replace(preferences).expect("store write");

    assert_eq!(
        host.list("runtime/sync")
            .expect("sync directory list")
            .len(),
        0
    );
}

#[test]
/// Verifies encrypted synced stores hide secrets in files and sync logs.
fn encrypted_synced_store_records_decryptable_operation_without_plaintext_log() {
    let host = Arc::new(MemoryStorageHost::default());
    let store = PreferencesDataStore::newEncryptedSyncedWithStorage(
        host.clone(),
        MODEL_CONFIGS_PREFERENCES_PATH,
        "runtime/sync",
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

    let operationEntries = host
        .list("runtime/sync/operations")
        .expect("operation directory list");
    assert_eq!(operationEntries.len(), 1);
    let operationLog = String::from_utf8(
        host.readBytes(&operationEntries[0].path)
            .expect("operation log"),
    )
    .expect("operation log utf8");
    assert!(operationLog.find("sk-model-secret").is_none());
    assert!(operationLog.find("operit.preferences.encrypted").is_some());

    let operations = SyncOperationStore::new(host, "runtime/sync")
        .operationsSince(&SyncClock::empty(), &["preferences".to_string()], 10)
        .expect("decoded operations");
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].entityId, MODEL_CONFIGS_PREFERENCES_PATH);
    assert_eq!(operations[0].payload["api_key"], "sk-model-secret");
}

#[test]
fn stores_with_same_path_share_latest_in_memory_preferences() {
    let host = Arc::new(MemoryStorageHost::default());
    let first = PreferencesDataStore::newWithStorage(
        host.clone(),
        "runtime/config/preferences/shared_state_test.preferences.json",
    );
    let second = PreferencesDataStore::newWithStorage(
        host,
        "runtime/config/preferences/shared_state_test.preferences.json",
    );

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
