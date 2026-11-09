//! One-time startup migration from loose appearance preferences to ordinary saved themes.
//! This is a business transaction so the catalog, selection and migration version sync atomically.
use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_store::PreferencesDataStore::{stringPreferencesKey, PreferencesDataStoreError};

const VERSION_KEY: &str = "theme_appearance_migration_version";
const VERSION: u32 = 1;

/// Runs after legacy character-theme import and before the runtime becomes ready.
/// Ordinary preference readers and the theme library never invoke this migration.
pub(crate) fn migrate(
    storage: Arc<dyn RuntimeStorageHost>,
) -> Result<(), PreferencesDataStoreError> {
    let store = super::UserPreferencesManager::UserPreferencesManager::store(storage);
    store.try_edit_result(|preferences| {
        let key = stringPreferencesKey(VERSION_KEY);
        let version = preferences.getTyped::<u32>(&key)?.unwrap_or(0);
        match version {
            VERSION => Ok(()),
            0 => {
                super::ThemeConfigManager::migrateUnboundAppearance(preferences)?;
                preferences.set(&key, VERSION);
                Ok(())
            }
            from => Err(PreferencesDataStoreError::Message(format!(
                "Unsupported theme appearance migration version: {from}"
            ))),
        }
    })
}
