use operit_model::MemorySearchConfig::MemorySearchConfig;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use operit_util::OperitPaths;

#[derive(Clone)]
pub struct MemorySearchSettingsPreferences {
    dataStore: PreferencesDataStore,
}

impl MemorySearchSettingsPreferences {
    /// Creates a memory search preference store backed by runtime storage.
    pub fn new(profileId: impl AsRef<str>) -> Self {
        Self {
            dataStore: PreferencesDataStore::newWithStorage(
                defaultRuntimeStorageHost(),
                OperitPaths::memorySearchSettingsStoragePath(profileId.as_ref())
                    .expect("memory search settings storage path must be valid"),
            ),
        }
    }

    /// Loads one persisted memory search configuration.
    pub fn load(&self) -> Result<MemorySearchConfig, PreferencesDataStoreError> {
        let preferences = self.dataStore.data()?;
        let Some(encoded) = preferences.get(&stringPreferencesKey("memory_search_config")) else {
            return Ok(MemorySearchConfig::default());
        };
        serde_json::from_str(encoded).map_err(PreferencesDataStoreError::from)
    }

    /// Saves one memory search configuration.
    pub fn save(&self, config: &MemorySearchConfig) -> Result<(), PreferencesDataStoreError> {
        let encoded = serde_json::to_string(config)?;
        self.dataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey("memory_search_config"),
                encoded.clone(),
            );
        })
    }
}
