use std::sync::Arc;
use operit_host_api::RuntimeStorageHost;
use operit_store::PreferencesDataStore::{PreferencesDataStore, PreferencesDataStoreError, stringPreferencesKey};
use operit_util::OperitPaths;

/// Owns the versioned SidebarDockPreferences preference values.
pub struct SidebarDockPreferences;

impl SidebarDockPreferences {
    /// Opens this module's preference file and applies its declared migrations.
    pub(crate) fn store(storage: Arc<dyn RuntimeStorageHost>) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(storage, OperitPaths::customPreferenceStoragePath("sidebar_dock.preferences.json").expect("declared preference path"))
.withSchema(2, Self::migratePreferences)
    }
    /// Registers this owner's typed preference contract for background synchronization.
    pub(crate) fn spaceMergeStore() -> operit_store::PreferenceMerge::PreferenceMergeStore {
        operit_store::PreferenceMerge::PreferenceMergeStore::values(OperitPaths::customPreferenceStoragePath("sidebar_dock.preferences.json").expect("declared preference path"), false, |storage, _| Self::store(storage), Self::decodeSyncedEntry)
    }

    /// Decodes historical scalar encodings without creating unrelated default entries.
    fn decodeSyncedEntry(entry: &mut operit_store::PreferencesDataStore::PreferencesSyncedEntry) -> Result<(), PreferencesDataStoreError> {
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
                    let key = stringPreferencesKey("layout");
                if let Some(encoded) = preferences.getTyped::<String>(&key)? {
                    let layout: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&encoded)?;
                    preferences.set(&key, serde_json::Value::Object(layout));
                }
                    Ok(())
                }
                from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
            }
    }

}
