use std::sync::Arc;
use operit_host_api::RuntimeStorageHost;
use operit_store::PreferencesDataStore::{PreferencesDataStore, PreferencesDataStoreError, stringPreferencesKey};

/// Owns the versioned RemoteAnnouncementPreferences preference values.
pub struct RemoteAnnouncementPreferences;

impl RemoteAnnouncementPreferences {
    /// Opens this module's preference file and applies its declared migrations.
    pub(crate) fn store(storage: Arc<dyn RuntimeStorageHost>) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(storage, operit_util::RuntimeStorageLayout::REMOTE_ANNOUNCEMENT_PREFERENCES_PATH)
.withSchema(2, Self::migratePreferences)
    }


    /// Decodes historical scalar encodings without creating unrelated default entries.
    pub(crate) fn decodeSyncedEntry(entry: &mut operit_store::PreferencesDataStore::PreferencesSyncedEntry) -> Result<(), PreferencesDataStoreError> {
        entry.decodeHistoricalValues(|version, values| {
            if version > 2 { return Err(PreferencesDataStoreError::SchemaVersionTooNew { actual: version, expected: 2 }); }
            if version < 2 { Self::migratePreferences(1, values)?; }
            Ok(())
        })
    }

    /// Applies the local storage migration owned by this preference module.
    fn migratePreferences(version: u32, preferences: &mut operit_store::PreferencesDataStore::Preferences) -> Result<(), PreferencesDataStoreError> {
        match version {
                0 => Ok(()),
                1 => {
                    let key = stringPreferencesKey("acknowledged_version");
                if let Some(encoded) = preferences.getTyped::<String>(&key)? {
                    preferences.set(&key, serde_json::from_str::<i32>(&encoded)?);
                }
                    Ok(())
                }
                from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
            }
    }

}
