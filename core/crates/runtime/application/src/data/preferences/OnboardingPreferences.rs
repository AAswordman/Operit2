use std::sync::Arc;
use operit_host_api::RuntimeStorageHost;
use operit_store::PreferencesDataStore::{PreferencesDataStore, PreferencesDataStoreError, stringPreferencesKey};
use operit_util::OperitPaths;

/// Owns the versioned OnboardingPreferences preference values.
pub struct OnboardingPreferences;

impl OnboardingPreferences {
    /// Opens this module's preference file and applies its declared migrations.
    pub(crate) fn store(storage: Arc<dyn RuntimeStorageHost>) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(storage, OperitPaths::customPreferenceStoragePath("onboarding_preferences").expect("declared preference path"))
            .withSchema(2, |version, preferences| match version {
                0 => Ok(()),
                1 => {
                    let key = stringPreferencesKey("ai_setup_guide_seen");
                if let Some(encoded) = preferences.getTyped::<String>(&key)? {
                    preferences.set(&key, serde_json::from_str::<bool>(&encoded)?);
                }
                    Ok(())
                }
                from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
            })
    }
}
