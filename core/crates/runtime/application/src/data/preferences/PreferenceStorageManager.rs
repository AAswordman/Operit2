use serde_json::Value;
use std::collections::BTreeMap;

use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use operit_util::OperitPaths;

pub struct PreferenceStorageManager {}

impl PreferenceStorageManager {
    /// Creates a manager for named custom preference files.
    pub fn getInstance() -> Self {
        Self {}
    }

    /// Reads one value from a named custom preference file.
    pub fn getPreference(
        &self,
        fileName: &str,
        key: &str,
    ) -> Result<Option<Value>, PreferencesDataStoreError> {
        let fileName = normalizePreferenceFileName(fileName)?;
        let key = normalizePreferenceKey(key)?;
        Ok(preferencesDataStore(&fileName)?
            .data()?
            .get(&stringPreferencesKey(&key))
            .cloned())
    }

    /// Reads selected values from a named custom preference file.
    pub fn getPreferences(
        &self,
        fileName: &str,
        keys: Vec<String>,
    ) -> Result<BTreeMap<String, Value>, PreferencesDataStoreError> {
        let fileName = normalizePreferenceFileName(fileName)?;
        let preferences = preferencesDataStore(&fileName)?.data()?;
        let mut values = BTreeMap::new();
        for key in keys {
            let key = normalizePreferenceKey(&key)?;
            if let Some(value) = preferences.get(&stringPreferencesKey(&key)).cloned() {
                values.insert(key, value);
            }
        }
        Ok(values)
    }

    /// Observes committed local and synchronized values from a named custom preference file.
    pub fn preferencesFlow(
        &self,
        fileName: &str,
    ) -> Result<Flow<BTreeMap<String, Value>>, PreferencesDataStoreError> {
        let fileName = normalizePreferenceFileName(fileName)?;
        Ok(preferencesDataStore(&fileName)?
            .dataFlow()
            .map(|preferences| preferences.entries().into_iter().filter(|(key, _)| key != operit_store::PreferencesDataStore::PREFERENCES_SCHEMA_VERSION_KEY_NAME).collect()))
    }

    /// Writes one value to a named custom preference file.
    pub fn setPreference(
        &self,
        fileName: &str,
        key: &str,
        value: Value,
    ) -> Result<(), PreferencesDataStoreError> {
        let fileName = normalizePreferenceFileName(fileName)?;
        let key = normalizePreferenceKey(key)?;
        preferencesDataStore(&fileName)?.edit(|preferences| {
            preferences.set(&stringPreferencesKey(&key), value);
        })
    }

    /// Writes multiple values to a named custom preference file.
    pub fn setPreferences(
        &self,
        fileName: &str,
        values: BTreeMap<String, Value>,
    ) -> Result<(), PreferencesDataStoreError> {
        let fileName = normalizePreferenceFileName(fileName)?;
        let mut normalizedValues = BTreeMap::new();
        for (key, value) in values {
            normalizedValues.insert(normalizePreferenceKey(&key)?, value);
        }
        preferencesDataStore(&fileName)?.edit(|preferences| {
            for (key, value) in normalizedValues {
                preferences.set(&stringPreferencesKey(&key), value);
            }
        })
    }

    /// Removes one key from a named custom preference file.
    pub fn removePreference(
        &self,
        fileName: &str,
        key: &str,
    ) -> Result<(), PreferencesDataStoreError> {
        let fileName = normalizePreferenceFileName(fileName)?;
        let key = normalizePreferenceKey(key)?;
        preferencesDataStore(&fileName)?.edit(|preferences| {
            preferences.remove(&stringPreferencesKey(&key));
        })
    }

    /// Removes selected keys from a named custom preference file.
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
        preferencesDataStore(&fileName)?.edit(|preferences| {
            for key in normalizedKeys {
                preferences.remove(&stringPreferencesKey(&key));
            }
        })
    }

    /// Removes every key from a named custom preference file.
    pub fn clearPreferences(&self, fileName: &str) -> Result<(), PreferencesDataStoreError> {
        let fileName = normalizePreferenceFileName(fileName)?;
        preferencesDataStore(&fileName)?.edit(|preferences| {
            for (key, _) in preferences.entries() {
                if key == operit_store::PreferencesDataStore::PREFERENCES_SCHEMA_VERSION_KEY_NAME { continue; }
                preferences.remove(&stringPreferencesKey(&key));
            }
        })
    }
}

/// Opens the shared host-backed store for one validated custom preference file.
fn preferencesDataStore(fileName: &str) -> Result<PreferencesDataStore, PreferencesDataStoreError> {
    let path = OperitPaths::customPreferenceStoragePath(fileName)
        .map_err(PreferencesDataStoreError::Message)?;
    super::PreferenceSpaceMerge::open(defaultRuntimeStorageHost(), &path)
}

/// Validates and normalizes a plain custom preference file name.
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

/// Validates and normalizes one custom preference key.
fn normalizePreferenceKey(key: &str) -> Result<String, PreferencesDataStoreError> {
    let key = key.trim();
    if key == operit_store::PreferencesDataStore::PREFERENCES_SCHEMA_VERSION_KEY_NAME {
        return Err(PreferencesDataStoreError::Message("The schema version is owned by its preference module".into()));
    }
    if key.is_empty() {
        return Err(PreferencesDataStoreError::Message(
            "preference key must not be blank".to_string(),
        ));
    }
    Ok(key.to_string())
}
