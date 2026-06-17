use crate::util::OperitPaths;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError,
};
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct EnvPreferences {
    dataStore: PreferencesDataStore,
}

impl EnvPreferences {
    const PREFS_FILE_NAME: &'static str = "env_preferences.preferences.json";

    #[allow(non_snake_case)]
    pub fn getInstance() -> Self {
        Self {
            dataStore: PreferencesDataStore::new(
                OperitPaths::envPreferencesPath().expect("env preferences path must be available"),
            ),
        }
    }

    #[allow(non_snake_case)]
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
    pub fn setEnv(&self, key: &str, value: &str) -> Result<(), PreferencesDataStoreError> {
        let name = key.trim();
        if name.is_empty() {
            return Ok(());
        }
        self.dataStore
            .edit(|preferences| preferences.set(&stringPreferencesKey(name), value.to_string()))
    }

    #[allow(non_snake_case)]
    pub fn removeEnv(&self, key: &str) -> Result<(), PreferencesDataStoreError> {
        let name = key.trim();
        if name.is_empty() {
            return Ok(());
        }
        self.dataStore
            .edit(|preferences| preferences.remove(&stringPreferencesKey(name)))
    }

    #[allow(non_snake_case)]
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
