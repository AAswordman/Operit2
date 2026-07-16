use operit_store::PreferencesDataStore::{
    stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use operit_util::OperitPaths;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct EnvPreferences {
    dataStore: PreferencesDataStore,
}

impl EnvPreferences {
    const PREFS_FILE_NAME: &'static str = "env_preferences.preferences.json";

    #[allow(non_snake_case)]
    /// Opens the persistent environment preference store.
    pub fn getInstance() -> Self {
        Self {
            dataStore: PreferencesDataStore::newWithStorage(
                defaultRuntimeStorageHost(),
                OperitPaths::ENV_PREFERENCES_PATH,
            ),
        }
    }

    #[allow(non_snake_case)]
    /// Reads an environment value from persistent preferences or the process environment.
    pub fn getEnv(&self, key: &str) -> Result<Option<String>, PreferencesDataStoreError> {
        let name = key.trim();
        if name.is_empty() {
            return Ok(None);
        }

        let fromPrefs = self
            .dataStore
            .data()?
            .get(&stringPreferencesKey(name))
            .cloned();
        if fromPrefs.as_ref().is_some_and(|value| !value.is_empty()) {
            return Ok(fromPrefs);
        }

        Ok(std::env::var(name).ok())
    }

    #[allow(non_snake_case)]
    /// Stores an environment value in persistent preferences.
    pub fn setEnv(&self, key: &str, value: &str) -> Result<(), PreferencesDataStoreError> {
        let name = key.trim();
        if name.is_empty() {
            return Ok(());
        }
        self.dataStore
            .edit(|preferences| preferences.set(&stringPreferencesKey(name), value.to_string()))
    }

    #[allow(non_snake_case)]
    /// Removes a persisted environment value by key.
    pub fn removeEnv(&self, key: &str) -> Result<(), PreferencesDataStoreError> {
        let name = key.trim();
        if name.is_empty() {
            return Ok(());
        }
        self.dataStore
            .edit(|preferences| preferences.remove(&stringPreferencesKey(name)))
    }

    #[allow(non_snake_case)]
    /// Returns all persisted environment values keyed by variable name.
    pub fn getAllEnv(&self) -> Result<BTreeMap<String, String>, PreferencesDataStoreError> {
        Ok(self
            .dataStore
            .data()?
            .entries()
            .into_iter()
            .filter(|(key, value)| !key.trim().is_empty() && !value.is_empty())
            .collect())
    }

    #[allow(non_snake_case)]
    /// Replaces the persisted environment values with the provided map.
    pub fn setAllEnv(
        &self,
        variables: BTreeMap<String, String>,
    ) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.edit(|preferences| {
            for (key, _) in preferences.entries() {
                preferences.remove(&stringPreferencesKey(&key));
            }
            for (key, value) in variables {
                let name = key.trim();
                if !name.is_empty() {
                    preferences.set(&stringPreferencesKey(name), value);
                }
            }
        })
    }
}
