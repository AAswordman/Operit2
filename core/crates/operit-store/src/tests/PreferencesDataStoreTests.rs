use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use operit_host_api::{HostError, RuntimeStorageEntry, RuntimeStorageHost};
use serde_json::Value;

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
