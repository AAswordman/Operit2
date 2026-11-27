#![allow(non_snake_case)]

use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_model::ProviderQuota::{
    mask_api_key_fingerprint, ProviderLocalBudget, ProviderQuotaProbeConfig, ProviderQuotaSnapshot,
};
use operit_store::PreferencesDataStore::{
    Preferences, PreferencesDataStore, PreferencesDataStoreError, stringPreferencesKey,
};
use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use operit_util::RuntimeStorageLayout::PROVIDER_QUOTA_PREFERENCES_PATH;

const SNAPSHOT_KEY_PREFIX: &str = "snapshot_";
const BUDGET_KEY_PREFIX: &str = "budget_";
const PROBE_CONFIG_KEY_PREFIX: &str = "probe_config_";

/// Owns Space-synced sanitized provider quota snapshots, probe configs, and local budget caps.
pub struct ProviderQuotaPreferences {
    dataStore: PreferencesDataStore,
}

impl ProviderQuotaPreferences {
    /// Opens the default runtime-backed provider quota preference store.
    pub fn default() -> Self {
        Self::withStorage(defaultRuntimeStorageHost())
    }

    /// Opens the provider quota preference store backed by an explicit runtime storage host.
    pub fn withStorage(storage: Arc<dyn RuntimeStorageHost>) -> Self {
        Self {
            dataStore: Self::store(storage),
        }
    }

    /// Opens this module's Space-synced preference store.
    pub(crate) fn store(storage: Arc<dyn RuntimeStorageHost>) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(storage, PROVIDER_QUOTA_PREFERENCES_PATH)
    }

    /// Lists every persisted provider quota snapshot stored in this Space.
    pub fn listSnapshots(&self) -> Result<Vec<ProviderQuotaSnapshot>, String> {
        self.readPrefixedValues(SNAPSHOT_KEY_PREFIX)
    }

    /// Saves one sanitized provider quota snapshot without storing raw API keys.
    pub fn saveSnapshot(&self, snapshot: &ProviderQuotaSnapshot) -> Result<(), String> {
        let mut sanitized = snapshot.clone();
        for key in &mut sanitized.keySnapshots {
            key.keyFingerprint = mask_or_keep_fingerprint(&key.keyFingerprint);
        }
        let key = stringPreferencesKey(&format!("{SNAPSHOT_KEY_PREFIX}{}", sanitized.providerId));
        let encoded = serde_json::to_string(&sanitized).map_err(|error| error.to_string())?;
        self.dataStore
            .edit(|preferences: &mut Preferences| {
                preferences.set(&key, encoded);
            })
            .map_err(|error: PreferencesDataStoreError| error.to_string())
    }

    /// Lists every user-configured provider local spending budget in this Space.
    pub fn listBudgets(&self) -> Result<Vec<ProviderLocalBudget>, String> {
        self.readPrefixedValues(BUDGET_KEY_PREFIX)
    }

    /// Saves or updates a provider local budget cap.
    pub fn saveBudget(&self, budget: &ProviderLocalBudget) -> Result<(), String> {
        let key = stringPreferencesKey(&format!("{BUDGET_KEY_PREFIX}{}", budget.providerId));
        let encoded = serde_json::to_string(budget).map_err(|error| error.to_string())?;
        self.dataStore
            .edit(|preferences: &mut Preferences| {
                preferences.set(&key, encoded);
            })
            .map_err(|error: PreferencesDataStoreError| error.to_string())
    }

    /// Removes a provider local budget cap.
    pub fn deleteBudget(&self, providerId: &str) -> Result<(), String> {
        let key = stringPreferencesKey(&format!("{BUDGET_KEY_PREFIX}{providerId}"));
        self.dataStore
            .edit(|preferences: &mut Preferences| {
                preferences.remove(&key);
            })
            .map_err(|error: PreferencesDataStoreError| error.to_string())
    }

    /// Lists every per-provider quota probe configuration.
    pub fn listProbeConfigs(&self) -> Result<Vec<ProviderQuotaProbeConfig>, String> {
        self.readPrefixedValues(PROBE_CONFIG_KEY_PREFIX)
    }

    /// Saves one per-provider quota probe configuration.
    pub fn saveProbeConfig(&self, config: &ProviderQuotaProbeConfig) -> Result<(), String> {
        let key = stringPreferencesKey(&format!("{PROBE_CONFIG_KEY_PREFIX}{}", config.providerId));
        let encoded = serde_json::to_string(config).map_err(|error| error.to_string())?;
        self.dataStore
            .edit(|preferences: &mut Preferences| {
                preferences.set(&key, encoded);
            })
            .map_err(|error: PreferencesDataStoreError| error.to_string())
    }

    fn readPrefixedValues<T: serde::de::DeserializeOwned>(
        &self,
        prefix: &str,
    ) -> Result<Vec<T>, String> {
        let prefs = self
            .dataStore
            .data()
            .map_err(|error| error.to_string())?;
        let mut items = Vec::new();
        for (key, value) in prefs.iterEntries() {
            if !key.starts_with(prefix) {
                continue;
            }
            if let Some(encoded) = value.as_str() {
                if let Ok(item) = serde_json::from_str::<T>(encoded) {
                    items.push(item);
                }
            } else if let Ok(item) = serde_json::from_value::<T>(value.clone()) {
                items.push(item);
            }
        }
        Ok(items)
    }
}

fn mask_or_keep_fingerprint(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.contains("...") || trimmed.starts_with("***") {
        return trimmed.to_string();
    }
    mask_api_key_fingerprint(trimmed)
}
