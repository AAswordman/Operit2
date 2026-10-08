use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_host_api::TimeUtils::tryCurrentTimeMillis;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, Preferences, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_util::OperitPaths;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::ThemePreferenceSnapshot::{
    assertThemePreferenceSnapshot, validateThemePreferenceSnapshot, writeThemePreferenceSnapshot,
};

const THEME_CONFIGS_KEY: &str = "theme_configs";
const ACTIVE_THEME_CONFIG_ID_KEY: &str = "active_theme_config_id";

/// Stores one independent named appearance configuration with a complete Flutter snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeConfig {
    pub id: String,
    pub name: String,
    pub snapshot: BTreeMap<String, Value>,
    pub createdAt: i64,
    pub updatedAt: i64,
}

/// Observes the named catalog and its nullable active selection from one committed preference state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeConfigState {
    pub configs: Vec<ThemeConfig>,
    pub activeThemeConfigId: Option<String>,
}

/// Owns named themes in the same runtime-bound store as ordinary application appearance.
#[derive(Clone)]
pub struct ThemeConfigManager {
    dataStore: PreferencesDataStore,
}

impl ThemeConfigManager {
    /// Opens ordinary user preferences through the owning runtime's actual storage host.
    pub fn new(storageHost: Arc<dyn RuntimeStorageHost>) -> Self {
        Self {
            dataStore: PreferencesDataStore::newWithStorage(
                storageHost,
                OperitPaths::USER_PREFERENCES_PATH,
            ),
        }
    }

    /// Lists every complete validated named theme without creating initial catalog entries.
    pub fn list(&self) -> Result<Vec<ThemeConfig>, PreferencesDataStoreError> {
        Ok(readState(&self.dataStore.data()?)?.configs)
    }

    /// Reads one existing named theme and rejects unknown or deleted identifiers.
    pub fn get(&self, id: String) -> Result<ThemeConfig, PreferencesDataStoreError> {
        let state = readState(&self.dataStore.data()?)?;
        Ok(findConfig(&state, &id)?.clone())
    }

    /// Creates a complete named theme without changing the current application appearance.
    pub fn create(
        &self,
        name: String,
        snapshot: BTreeMap<String, Value>,
    ) -> Result<ThemeConfig, PreferencesDataStoreError> {
        let now = tryCurrentTimeMillis().map_err(PreferencesDataStoreError::Message)?;
        let config = ThemeConfig {
            id: Uuid::new_v4().to_string(),
            name,
            snapshot,
            createdAt: now,
            updatedAt: now,
        };
        validateConfig(&config)?;
        self.dataStore.try_edit_result(|preferences| {
            let mut state = readState(preferences)?;
            if state
                .configs
                .iter()
                .any(|existing| existing.id == config.id)
            {
                return Err(PreferencesDataStoreError::Message(format!(
                    "Theme configuration identifier already exists: {}",
                    config.id
                )));
            }
            state.configs.push(config.clone());
            writeState(preferences, &state)?;
            Ok(config)
        })
    }

    /// Updates a named theme and its active ordinary appearance in one preference commit.
    pub fn update(
        &self,
        id: String,
        name: String,
        snapshot: BTreeMap<String, Value>,
    ) -> Result<ThemeConfig, PreferencesDataStoreError> {
        validateThemePreferenceSnapshot(&snapshot)?;
        self.dataStore.try_edit_result(|preferences| {
            let mut state = readState(preferences)?;
            let index = findConfigIndex(&state, &id)?;
            let config = ThemeConfig {
                id: id.clone(),
                name,
                snapshot,
                createdAt: state.configs[index].createdAt,
                updatedAt: tryCurrentTimeMillis().map_err(PreferencesDataStoreError::Message)?,
            };
            validateConfig(&config)?;
            if state.activeThemeConfigId.as_deref() == Some(id.as_str()) {
                writeThemePreferenceSnapshot(preferences, &config.snapshot)?;
            }
            state.configs[index] = config.clone();
            writeState(preferences, &state)?;
            Ok(config)
        })
    }

    /// Deletes an inactive named theme and rejects deletion of the active selection.
    pub fn delete(&self, id: String) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.try_edit_result(|preferences| {
            let mut state = readState(preferences)?;
            let index = findConfigIndex(&state, &id)?;
            if state.activeThemeConfigId.as_deref() == Some(id.as_str()) {
                return Err(PreferencesDataStoreError::Message(format!(
                    "Cannot delete active theme configuration {id}; explicitly select another theme or custom appearance first"
                )));
            }
            state.configs.remove(index);
            writeState(preferences, &state)
        })
    }

    /// Applies a complete named theme and its selection in one ordinary preference commit.
    pub fn apply(&self, id: String) -> Result<ThemeConfig, PreferencesDataStoreError> {
        self.dataStore.try_edit_result(|preferences| {
            let mut state = readState(preferences)?;
            let config = findConfig(&state, &id)?.clone();
            writeThemePreferenceSnapshot(preferences, &config.snapshot)?;
            state.activeThemeConfigId = Some(config.id.clone());
            writeState(preferences, &state)?;
            Ok(config)
        })
    }

    /// Reads the active named theme, or null for explicitly ordinary custom appearance.
    pub fn getActive(&self) -> Result<Option<ThemeConfig>, PreferencesDataStoreError> {
        let state = readState(&self.dataStore.data()?)?;
        match &state.activeThemeConfigId {
            Some(id) => Ok(Some(findConfig(&state, id)?.clone())),
            None => Ok(None),
        }
    }

    /// Observes committed catalog and selection changes from the shared preference store.
    pub fn watch(&self) -> Flow<ThemeConfigState> {
        self.dataStore
            .dataFlow()
            .mapResult(|preferences| readState(&preferences))
    }

    /// Saves complete ordinary appearance and synchronizes an active named theme in the same commit.
    pub fn saveAppearance(
        &self,
        snapshot: BTreeMap<String, Value>,
    ) -> Result<Option<ThemeConfig>, PreferencesDataStoreError> {
        validateThemePreferenceSnapshot(&snapshot)?;
        self.dataStore.try_edit_result(|preferences| {
            let mut state = readState(preferences)?;
            let updated = match state.activeThemeConfigId.clone() {
                Some(id) => {
                    let index = findConfigIndex(&state, &id)?;
                    let config = ThemeConfig {
                        snapshot: snapshot.clone(),
                        updatedAt: tryCurrentTimeMillis()
                            .map_err(PreferencesDataStoreError::Message)?,
                        ..state.configs[index].clone()
                    };
                    validateConfig(&config)?;
                    state.configs[index] = config.clone();
                    Some(config)
                }
                None => None,
            };
            writeThemePreferenceSnapshot(preferences, &snapshot)?;
            writeState(preferences, &state)?;
            Ok(updated)
        })
    }

    /// Explicitly selects ordinary custom appearance without choosing or creating a named theme.
    pub fn useCustomAppearance(
        &self,
        snapshot: BTreeMap<String, Value>,
    ) -> Result<(), PreferencesDataStoreError> {
        validateThemePreferenceSnapshot(&snapshot)?;
        self.dataStore.try_edit_result(|preferences| {
            let mut state = readState(preferences)?;
            writeThemePreferenceSnapshot(preferences, &snapshot)?;
            state.activeThemeConfigId = None;
            writeState(preferences, &state)
        })
    }
}

/// Decodes and validates one committed catalog together with its active ordinary appearance.
fn readState(preferences: &Preferences) -> Result<ThemeConfigState, PreferencesDataStoreError> {
    // Absent new-domain keys mean an untouched, genuinely empty catalog and custom appearance.
    // Malformed stored values never enter these initial-state branches.
    let configs = match preferences.get(&stringPreferencesKey(THEME_CONFIGS_KEY)) {
        Some(value) => serde_json::from_str::<Vec<ThemeConfig>>(value)?,
        None => Vec::new(),
    };
    let activeThemeConfigId =
        match preferences.get(&stringPreferencesKey(ACTIVE_THEME_CONFIG_ID_KEY)) {
            Some(value) => serde_json::from_str::<Option<String>>(value)?,
            None => None,
        };
    let state = ThemeConfigState {
        configs,
        activeThemeConfigId,
    };
    validateState(&state)?;
    if let Some(id) = &state.activeThemeConfigId {
        assertThemePreferenceSnapshot(preferences, &findConfig(&state, id)?.snapshot)?;
    }
    Ok(state)
}

/// Writes the catalog and selection without storing a second active appearance snapshot.
fn writeState(
    preferences: &mut Preferences,
    state: &ThemeConfigState,
) -> Result<(), PreferencesDataStoreError> {
    validateState(state)?;
    if let Some(id) = &state.activeThemeConfigId {
        assertThemePreferenceSnapshot(preferences, &findConfig(state, id)?.snapshot)?;
    }
    preferences.set(
        &stringPreferencesKey(THEME_CONFIGS_KEY),
        serde_json::to_string(&state.configs)?,
    );
    preferences.set(
        &stringPreferencesKey(ACTIVE_THEME_CONFIG_ID_KEY),
        serde_json::to_string(&state.activeThemeConfigId)?,
    );
    Ok(())
}

/// Rejects duplicate configuration IDs, invalid records and dangling active selections.
fn validateState(state: &ThemeConfigState) -> Result<(), PreferencesDataStoreError> {
    let mut ids = BTreeSet::new();
    for config in &state.configs {
        validateConfig(config)?;
        if !ids.insert(&config.id) {
            return Err(PreferencesDataStoreError::Message(format!(
                "Duplicate theme configuration identifier: {}",
                config.id
            )));
        }
    }
    if let Some(id) = &state.activeThemeConfigId {
        findConfig(state, id)?;
    }
    Ok(())
}

/// Validates the complete saved snapshot and host-assigned record metadata.
fn validateConfig(config: &ThemeConfig) -> Result<(), PreferencesDataStoreError> {
    validateIdentifier(&config.id)?;
    if config.name.trim().is_empty() {
        return Err(PreferencesDataStoreError::Message(
            "Theme configuration name must not be blank".to_string(),
        ));
    }
    if config.createdAt < 0 || config.updatedAt < config.createdAt {
        return Err(PreferencesDataStoreError::Message(format!(
            "Invalid theme configuration timestamps: {}",
            config.id
        )));
    }
    validateThemePreferenceSnapshot(&config.snapshot)
}

/// Rejects blank configuration IDs without changing the caller's identifier.
fn validateIdentifier(id: &str) -> Result<(), PreferencesDataStoreError> {
    if id.trim().is_empty() {
        return Err(PreferencesDataStoreError::Message(
            "Theme configuration identifier must not be blank".to_string(),
        ));
    }
    Ok(())
}

/// Locates exactly one validated configuration index or reports its missing identifier.
fn findConfigIndex(state: &ThemeConfigState, id: &str) -> Result<usize, PreferencesDataStoreError> {
    validateIdentifier(id)?;
    state
        .configs
        .iter()
        .position(|config| config.id == id)
        .ok_or_else(|| {
            PreferencesDataStoreError::Message(format!("Theme configuration not found: {id}"))
        })
}

/// Borrows one existing configuration without substituting another theme.
fn findConfig<'a>(
    state: &'a ThemeConfigState,
    id: &str,
) -> Result<&'a ThemeConfig, PreferencesDataStoreError> {
    Ok(&state.configs[findConfigIndex(state, id)?])
}

#[cfg(test)]
#[path = "ThemeConfigManager.tests.rs"]
mod tests;
