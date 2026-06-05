use std::collections::BTreeMap;

use operit_store::PreferencesDataStore::{
    stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::default_data_dir;

pub struct PreferenceStorageManager {}

impl PreferenceStorageManager {
    pub fn getInstance() -> Self {
        Self {}
    }

    pub fn getPreference(
        &self,
        fileName: &str,
        key: &str,
    ) -> Result<Option<String>, PreferencesDataStoreError> {
        let fileName = normalizePreferenceFileName(fileName)?;
        let key = normalizePreferenceKey(key)?;
        Ok(preferencesDataStore(&fileName)
            .data()?
            .get(&stringPreferencesKey(&key))
            .cloned())
    }

    pub fn getPreferences(
        &self,
        fileName: &str,
        keys: Vec<String>,
    ) -> Result<BTreeMap<String, String>, PreferencesDataStoreError> {
        let fileName = normalizePreferenceFileName(fileName)?;
        let preferences = preferencesDataStore(&fileName).data()?;
        let mut values = BTreeMap::new();
        for key in keys {
            let key = normalizePreferenceKey(&key)?;
            if let Some(value) = preferences.get(&stringPreferencesKey(&key)).cloned() {
                values.insert(key, value);
            }
        }
        Ok(values)
    }

    pub fn setPreference(
        &self,
        fileName: &str,
        key: &str,
        value: &str,
    ) -> Result<(), PreferencesDataStoreError> {
        let fileName = normalizePreferenceFileName(fileName)?;
        let key = normalizePreferenceKey(key)?;
        preferencesDataStore(&fileName).edit(|preferences| {
            preferences.set(&stringPreferencesKey(&key), value.to_string());
        })
    }

    pub fn setPreferences(
        &self,
        fileName: &str,
        values: BTreeMap<String, String>,
    ) -> Result<(), PreferencesDataStoreError> {
        let fileName = normalizePreferenceFileName(fileName)?;
        let mut normalizedValues = BTreeMap::new();
        for (key, value) in values {
            normalizedValues.insert(normalizePreferenceKey(&key)?, value);
        }
        preferencesDataStore(&fileName).edit(|preferences| {
            for (key, value) in normalizedValues {
                preferences.set(&stringPreferencesKey(&key), value);
            }
        })
    }

    pub fn removePreference(
        &self,
        fileName: &str,
        key: &str,
    ) -> Result<(), PreferencesDataStoreError> {
        let fileName = normalizePreferenceFileName(fileName)?;
        let key = normalizePreferenceKey(key)?;
        preferencesDataStore(&fileName).edit(|preferences| {
            preferences.remove(&stringPreferencesKey(&key));
        })
    }

    pub fn removePreferences(
        &self,
        fileName: &str,
        keys: Vec<String>,
    ) -> Result<(), PreferencesDataStoreError> {
        let fileName = normalizePreferenceFileName(fileName)?;
        let mut normalizedKeys = Vec::new();
        for key in keys {
            normalizedKeys.push(normalizePreferenceKey(&key)?);
        }
        preferencesDataStore(&fileName).edit(|preferences| {
            for key in normalizedKeys {
                preferences.remove(&stringPreferencesKey(&key));
            }
        })
    }
}

fn preferencesDataStore(fileName: &str) -> PreferencesDataStore {
    PreferencesDataStore::new(default_data_dir().join(fileName))
}

fn normalizePreferenceFileName(fileName: &str) -> Result<String, PreferencesDataStoreError> {
    let fileName = fileName.trim();
    if fileName.is_empty() {
        return Err(PreferencesDataStoreError::Message(
            "preference file name must not be blank".to_string(),
        ));
    }
    if fileName.contains('/') || fileName.contains('\\') || fileName == "." || fileName == ".." {
        return Err(PreferencesDataStoreError::Message(
            "preference file name must be a plain file name".to_string(),
        ));
    }
    Ok(fileName.to_string())
}

fn normalizePreferenceKey(key: &str) -> Result<String, PreferencesDataStoreError> {
    let key = key.trim();
    if key.is_empty() {
        return Err(PreferencesDataStoreError::Message(
            "preference key must not be blank".to_string(),
        ));
    }
    Ok(key.to_string())
}
