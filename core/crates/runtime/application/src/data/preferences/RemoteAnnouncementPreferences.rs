use std::sync::Arc;
use operit_host_api::RuntimeStorageHost;
use operit_store::PreferencesDataStore::{PreferencesDataStore, PreferencesDataStoreError, stringPreferencesKey};
use operit_util::OperitPaths;

/// Owns the versioned RemoteAnnouncementPreferences preference values.
pub struct RemoteAnnouncementPreferences;

impl RemoteAnnouncementPreferences {
    /// Opens this module's preference file and applies its declared migrations.
    pub(crate) fn store(storage: Arc<dyn RuntimeStorageHost>) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(storage, OperitPaths::customPreferenceStoragePath("remote_announcement_preferences").expect("declared preference path"))
            .withSchema(2, |version, preferences| match version {
                0 => Ok(()),
                1 => {
                    let key = stringPreferencesKey("acknowledged_version");
                if let Some(encoded) = preferences.getTyped::<String>(&key)? {
                    preferences.set(&key, serde_json::from_str::<i32>(&encoded)?);
                }
                    Ok(())
                }
                from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
            })
    }
}
