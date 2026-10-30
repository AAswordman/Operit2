//! Retires the native character namespace after its verified plugin import and source cleanup.
use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_store::{
    PreferenceLayout::PreferenceLayout,
    PreferencesDataStore::{stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError},
};
use operit_util::RuntimeStorageLayout::{
    CHARACTER_CARDS_PREFERENCES_PATH, CHARACTER_GROUPS_PREFERENCES_PATH,
    DATA_MEMORY_CHARACTERS_DIR_PATH, DATA_MEMORY_SHARED_DIR_PATH, PROMPT_TAGS_PREFERENCES_PATH,
    SHARED_MEMORY_STORES_PREFERENCES_PATH,
};

const STATE: &str = "runtime/sync/character_plugin_layout.preferences.json";

/// Existing installations adopt the same local layout progress as a newly completed import.
pub(crate) fn prepare(storage: Arc<dyn RuntimeStorageHost>) -> Result<(), String> {
    let directory = operit_store::ExtensionStore::ExtensionStore::spaceDataPath(
        super::CharacterPluginMigration::PLUGIN_ID,
    )?;
    let path = format!("{directory}/meta.preferences.json");
    if !storage.exists(&path).map_err(|e| e.to_string())? {
        return Ok(());
    }
    let meta = PreferencesDataStore::newWithStorage(storage.clone(), &path)
        .data()
        .map_err(|e| e.to_string())?;
    let Some(marker) = meta.get(&stringPreferencesKey("import_operit2-v27")) else {
        return Ok(());
    };
    if marker["complete"] != true || marker["cleanupComplete"] != true {
        return Ok(());
    }
    // Cleanup commits its marker before deleting the final startup trigger.
    // Retained or recreated documents under old memory directories do not delay retirement.
    if storage
        .exists(CHARACTER_CARDS_PREFERENCES_PATH)
        .map_err(|e| e.to_string())?
    {
        return Ok(());
    }
    for name in ["cards.preferences.json", "memory.sqlite"] {
        if !storage
            .exists(&format!("{directory}/{name}"))
            .map_err(|e| e.to_string())?
        {
            return Err(format!(
                "Character layout destination is missing: {directory}/{name}"
            ));
        }
    }
    PreferenceLayout::new(storage, STATE)
        .withSchema(1, |from, migration| {
            if from != 0 {
                return Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 });
            }
            for path in [
                CHARACTER_CARDS_PREFERENCES_PATH,
                CHARACTER_GROUPS_PREFERENCES_PATH,
                PROMPT_TAGS_PREFERENCES_PATH,
                SHARED_MEMORY_STORES_PREFERENCES_PATH,
            ] {
                migration.retireFile(path);
            }
            for path in [DATA_MEMORY_CHARACTERS_DIR_PATH, DATA_MEMORY_SHARED_DIR_PATH] {
                migration.retireDirectory(path);
            }
            Ok(())
        })
        .prepare()
}
