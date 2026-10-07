use operit_store::PreferencesDataStore::{
    Flow, PreferencesDataStore, PreferencesDataStoreError, stringPreferencesKey,
};
use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use operit_util::OperitPaths;

/// Preference key storing the thinking-persistence switch as "true"/"false".
const PERSIST_THINKING_PARTS_KEY: &str = "persist_thinking_parts";

/// Assistant thinking parts persist unless the user switches them off.
pub const DEFAULT_PERSIST_THINKING_PARTS: bool = true;

/// Owns the user switch controlling whether assistant thinking parts persist.
pub struct ThinkingPersistencePreferences {
    dataStore: PreferencesDataStore,
}

impl ThinkingPersistencePreferences {
    /// Opens the thinking-persistence facade from the default runtime path.
    #[allow(non_snake_case)]
    pub fn getInstance() -> Self {
        Self {
            dataStore: PreferencesDataStore::newWithStorage(
                defaultRuntimeStorageHost(),
                OperitPaths::USER_PREFERENCES_PATH,
            ),
        }
    }

    /// Observes the thinking-persistence switch as "true"/"false".
    #[allow(non_snake_case)]
    pub fn persistThinkingPartsFlow(&self) -> Flow<String> {
        self.dataStore.dataFlow().map(|preferences| {
            preferences
                .get(&stringPreferencesKey(PERSIST_THINKING_PARTS_KEY))
                .cloned()
                .unwrap_or_else(|| DEFAULT_PERSIST_THINKING_PARTS.to_string())
        })
    }

    /// Reads whether assistant thinking parts persist.
    #[allow(non_snake_case)]
    pub fn persistThinkingParts(&self) -> bool {
        self.persistThinkingPartsFlow()
            .first()
            .map(|value| value == "true")
            .unwrap_or(DEFAULT_PERSIST_THINKING_PARTS)
    }

    /// Saves whether assistant thinking parts persist.
    #[allow(non_snake_case)]
    pub fn savePersistThinkingParts(
        &self,
        persist: bool,
    ) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey(PERSIST_THINKING_PARTS_KEY),
                persist.to_string(),
            );
        })
    }
}
