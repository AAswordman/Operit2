use crate::data::model::MemorySearchConfig::MemorySearchConfig;
use crate::util::OperitPaths;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError,
};

#[derive(Clone)]
pub struct MemorySearchSettingsPreferences {
    dataStore: PreferencesDataStore,
}

impl MemorySearchSettingsPreferences {
    pub fn new(profileId: impl AsRef<str>) -> Self {
        let path = OperitPaths::memorySearchSettingsPath(profileId.as_ref())
            .expect("memory search settings path must be available");
        Self {
            dataStore: PreferencesDataStore::new(path),
        }
    }

    pub fn load(&self) -> Result<MemorySearchConfig, PreferencesDataStoreError> {
        let preferences = self.dataStore.data()?;
        let Some(encoded) = preferences.get(&stringPreferencesKey("memory_search_config")) else {
            return Ok(MemorySearchConfig::default());
        };
        serde_json::from_str(encoded).map_err(PreferencesDataStoreError::from)
    }

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
