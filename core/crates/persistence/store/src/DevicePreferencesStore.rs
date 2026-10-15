//! Small device-owned preferences, independent of SQL/business replication.
//! Each key is a Lamport register; deletions remain tombstones so a returning
//! peer cannot resurrect them. Only the current bounded projection is stored.
#![allow(non_snake_case)]
use crate::PreferencesDataStore::{stringPreferencesKey, CoreNodeSecretStore};
use operit_host_api::AtomicCounter::AtomicU64;
use operit_host_api::RuntimeStorageHost;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

pub const MAX_DEVICE_PREFERENCES_BYTES: usize = 6144;
const MAX_KEYS: usize = 24;
static MUTATION: Mutex<()> = Mutex::new(());
static PREFERENCE_MUTATION_REVISION: AtomicU64 = AtomicU64::new(0);

/// Process-local signal for sync gates; preferences never enter the business log.
pub fn devicePreferencesMutationRevision() -> u64 {
    PREFERENCE_MUTATION_REVISION.load(Ordering::Acquire)
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevicePreferenceEntry {
    pub revision: u64,
    pub writer: String,
    pub value: Option<Value>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevicePreferences {
    pub spaceId: String,
    pub nodeId: String,
    pub entries: BTreeMap<String, DevicePreferenceEntry>,
}
impl DevicePreferences {
    pub fn values(&self) -> BTreeMap<String, Value> {
        self.entries
            .iter()
            .filter_map(|(key, entry)| entry.value.clone().map(|value| (key.clone(), value)))
            .collect()
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.nodeId.is_empty()
            || self.nodeId.len() > 160
            || self.spaceId.is_empty()
            || self.spaceId.len() > 160
            || self.entries.len() > MAX_KEYS
            || serde_json::to_vec_pretty(&BTreeMap::from([("state", self)]))
                .map_err(|e| e.to_string())?
                .len()
                > MAX_DEVICE_PREFERENCES_BYTES
        {
            return Err("Device preferences exceed the device budget".into());
        }
        for (key, entry) in &self.entries {
            if key.is_empty()
                || key.len() > 64
                || !key
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
                || entry.writer.is_empty()
                || entry.writer.len() > 160
                || entry.revision == 0
            {
                return Err("Invalid device preference entry".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct DevicePreferencesStore {
    storage: Arc<dyn RuntimeStorageHost>,
}
impl DevicePreferencesStore {
    pub fn new(storage: Arc<dyn RuntimeStorageHost>) -> Self {
        Self { storage }
    }
    fn file(&self, space: &str, node: &str) -> CoreNodeSecretStore {
        let hash = Sha256::digest(format!("{}:{space}{node}", space.len()).as_bytes());
        CoreNodeSecretStore::newWithStorage(
            self.storage.clone(),
            format!("runtime/node_preferences/{hash:x}.preferences.json"),
        )
    }
    pub fn read(&self, space: &str, node: &str) -> Result<DevicePreferences, String> {
        let data = self.file(space, node).data().map_err(|e| e.to_string())?;
        let state = match data.get(&stringPreferencesKey("state")) {
            Some(value) => serde_json::from_value::<DevicePreferences>(value.clone())
                .map_err(|e| e.to_string())?,
            None => DevicePreferences {
                spaceId: space.into(),
                nodeId: node.into(),
                entries: BTreeMap::new(),
            },
        };
        state.validate()?;
        if state.spaceId != space || state.nodeId != node {
            return Err("Device preferences scope mismatch".into());
        }
        Ok(state)
    }
    fn write(&self, state: &DevicePreferences) -> Result<(), String> {
        state.validate()?;
        self.file(&state.spaceId, &state.nodeId)
            .edit(|data| {
                data.set(
                    &stringPreferencesKey("state"),
                    serde_json::to_value(state).expect("preference DTO"),
                );
            })
            .map_err(|e| e.to_string())
    }
    pub fn set(
        &self,
        space: &str,
        node: &str,
        writer: &str,
        key: &str,
        value: Option<Value>,
    ) -> Result<DevicePreferences, String> {
        let _lock = MUTATION.lock().map_err(|e| e.to_string())?;
        let mut state = self.read(space, node)?;
        let revision = state
            .entries
            .values()
            .map(|entry| entry.revision)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("Preference revision exhausted")?;
        state.entries.insert(
            key.into(),
            DevicePreferenceEntry {
                revision,
                writer: writer.into(),
                value,
            },
        );
        self.write(&state)?;
        PREFERENCE_MUTATION_REVISION.fetch_add(1, Ordering::AcqRel);
        drop(_lock);
        crate::SyncOperationStore::publishSyncMutation();
        Ok(state)
    }
    pub fn merge(&self, incoming: &DevicePreferences) -> Result<DevicePreferences, String> {
        incoming.validate()?;
        let _lock = MUTATION.lock().map_err(|e| e.to_string())?;
        let mut state = self.read(&incoming.spaceId, &incoming.nodeId)?;
        let previous = state.clone();
        for (key, next) in &incoming.entries {
            if let Some(current) = state.entries.get(key) {
                let order = (next.revision, &next.writer).cmp(&(current.revision, &current.writer));
                if order.is_eq() && next != current {
                    return Err("Conflicting preference revision".into());
                }
                if !order.is_gt() {
                    continue;
                }
            }
            state.entries.insert(key.clone(), next.clone());
        }
        state.validate()?; // Validate the union before writing anything.
        let changed = state != previous;
        if changed {
            self.write(&state)?;
            PREFERENCE_MUTATION_REVISION.fetch_add(1, Ordering::AcqRel);
        }
        drop(_lock);
        if changed {
            crate::SyncOperationStore::publishSyncMutation();
        }
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    struct Fixture {
        root: std::path::PathBuf,
        store: DevicePreferencesStore,
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "operit-device-preferences-{}",
                uuid::Uuid::new_v4()
            ));
            let host = Arc::new(operit_host_native_storage::NativeRuntimeStorageHost::new(
                root.join("runtime"),
                root.join("workspaces"),
            ));
            Self {
                root,
                store: DevicePreferencesStore::new(host),
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
    #[test]
    fn preference_replicas_converge_after_concurrent_edits_and_deletion() {
        let a = Fixture::new();
        let b = Fixture::new();
        let first = a
            .store
            .set("space", "esp", "a", "volume", Some(json!(20)))
            .unwrap();
        b.store.merge(&first).unwrap();
        let left = a
            .store
            .set("space", "esp", "a", "volume", Some(json!(30)))
            .unwrap();
        let right = b
            .store
            .set("space", "esp", "b", "brightness", Some(json!(80)))
            .unwrap();
        let ab = a.store.merge(&right).unwrap();
        let ba = b.store.merge(&left).unwrap();
        assert_eq!(ab, ba);
        let deletion = a.store.set("space", "esp", "a", "volume", None).unwrap();
        b.store.merge(&deletion).unwrap();
        let restored = b.store.merge(&first).unwrap();
        assert!(!restored.values().contains_key("volume"));
        assert_eq!(restored.values()["brightness"], json!(80));
    }
    #[test]
    fn concurrent_same_key_edits_have_a_deterministic_winner() {
        let a = Fixture::new();
        let b = Fixture::new();
        let left = a
            .store
            .set("space", "esp", "a", "volume", Some(json!(10)))
            .unwrap();
        let right = b
            .store
            .set("space", "esp", "b", "volume", Some(json!(90)))
            .unwrap();
        assert_eq!(
            a.store.merge(&right).unwrap(),
            b.store.merge(&left).unwrap()
        );
    }
    #[test]
    fn rejects_oversized_union_and_conflicting_versions_before_writes() {
        let f = Fixture::new();
        let original = f
            .store
            .set(
                "space",
                "esp",
                "a",
                "chat",
                Some(json!({"apiKey":"private-token"})),
            )
            .unwrap();
        let mut forged = original.clone();
        forged.entries.get_mut("chat").unwrap().value = Some(json!("different"));
        assert!(f.store.merge(&forged).is_err());
        assert!(f
            .store
            .set(
                "space",
                "esp",
                "a",
                "large",
                Some(json!("x".repeat(MAX_DEVICE_PREFERENCES_BYTES)))
            )
            .is_err());
        assert_eq!(f.store.read("space", "esp").unwrap(), original);
        let mut incoming = DevicePreferences {
            spaceId: "space".into(),
            nodeId: "esp".into(),
            entries: BTreeMap::new(),
        };
        for n in 0..MAX_KEYS {
            incoming.entries.insert(
                format!("k{n}"),
                DevicePreferenceEntry {
                    revision: 1,
                    writer: "b".into(),
                    value: None,
                },
            );
        }
        assert!(f.store.merge(&incoming).is_err());
        assert_eq!(f.store.read("space", "esp").unwrap(), original);
    }
    #[test]
    fn device_and_space_scopes_are_isolated_and_secrets_are_encrypted() {
        let f = Fixture::new();
        let state = f
            .store
            .set(
                "space-a",
                "esp",
                "host",
                "chat",
                Some(json!({"apiKey":"private-token"})),
            )
            .unwrap();
        assert!(f.store.read("space-b", "esp").unwrap().entries.is_empty());
        assert!(f
            .store
            .read("space-a", "another-device")
            .unwrap()
            .entries
            .is_empty());
        let reopened = DevicePreferencesStore::new(f.store.storage.clone());
        assert_eq!(reopened.read("space-a", "esp").unwrap(), state);
        let directory = f.root.join("runtime/node_preferences");
        for file in std::fs::read_dir(directory).unwrap() {
            let content = std::fs::read_to_string(file.unwrap().path()).unwrap();
            assert!(!content.contains("private-token"));
        }
        assert!(!f.root.join("runtime/sync").exists());
        assert!(!f.root.join("runtime/chat").exists());
    }
}
