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
            .withSchema(2, |version, preferences| match version {
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
            })
    }
}
