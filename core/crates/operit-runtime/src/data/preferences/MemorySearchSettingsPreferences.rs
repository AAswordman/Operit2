use crate::data::model::MemorySearchConfig::MemorySearchConfig;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::default_data_dir;

#[derive(Clone)]
pub struct MemorySearchSettingsPreferences {
    dataStore: PreferencesDataStore,
}

impl MemorySearchSettingsPreferences {
    pub fn new(profileId: impl AsRef<str>) -> Self {
        let path = default_data_dir()
            .join("memory")
            .join(sanitizeProfileId(profileId.as_ref()))
            .join("memory_search_settings.preferences.json");
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
            preferences.set(&stringPreferencesKey("memory_search_config"), encoded.clone());
        })
    }
}

fn sanitizeProfileId(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "default".to_string()
    } else {
        out
    }
}
