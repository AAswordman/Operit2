use super::ThemeConfigManager::{validateConfig, ThemeConfig, THEME_CONFIG_PREFIX};
use super::ThemePreferenceSnapshot::{migrateLegacyThemePreferences, migrateLegacyThemeSnapshot};
use operit_host_api::RuntimeStorageHost;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, Preferences, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use operit_util::LocaleUtils::LanguageCodes;
use operit_util::OperitPaths;
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
pub struct UserPreferencesManager {
    dataStore: PreferencesDataStore,
}

#[derive(Clone)]
pub struct PreferencesManager {
    inner: UserPreferencesManager,
}

impl UserPreferencesManager {
    pub const DEFAULT_LANGUAGE: &'static str = LanguageCodes::AUTO;

    /// Opens user preferences from the default runtime preference path.
    pub fn getInstance() -> Self {
        Self {
            dataStore: Self::store(defaultRuntimeStorageHost()),
        }
    }

    /// Opens the single versioned owner of global user and appearance preferences.
    pub(crate) fn store(storage: Arc<dyn RuntimeStorageHost>) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(storage, OperitPaths::USER_PREFERENCES_PATH)
            .withMigration(5, Self::migration)
    }

    /// Applies user preference migrations through the shared transaction context.
    fn migration(
        from: u32,
        context: &mut operit_store::Transaction::migration::Context,
    ) -> Result<(), PreferencesDataStoreError> {
        if from == 2 {
            operit_store::Transaction::Rule::SplitRecords {
                key: "theme_configs".into(),
                prefix: THEME_CONFIG_PREFIX.into(),
                identity: "id".into(),
            }
            .apply(context)
            .map_err(PreferencesDataStoreError::Message)
        } else {
            context.map_preferences(from, &Self::migratePreferences)
        }
    }

    /// Applies each versioned migration owned by the user preferences file.
    pub(crate) fn migratePreferences(
        version: u32,
        preferences: &mut Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        match version {
            0 => Ok(()),
            1 => {
                for key in ["use_system_theme", "long_pasted_text_input_enabled"] {
                    let key = stringPreferencesKey(key);
                    if let Some(encoded) = preferences.getTyped::<String>(&key)? {
                        preferences.set(&key, serde_json::from_str::<bool>(&encoded)?);
                    }
                }
                let key = stringPreferencesKey("long_pasted_text_input_threshold");
                if let Some(encoded) = preferences.getTyped::<String>(&key)? {
                    preferences.set(&key, serde_json::from_str::<i32>(&encoded)?);
                }
                let key = stringPreferencesKey("theme_configs");
                if let Some(encoded) = preferences.getTyped::<String>(&key)? {
                    let configs: Vec<ThemeConfig> = serde_json::from_str(&encoded)?;
                    preferences.set(&key, serde_json::to_value(configs)?);
                }
                let key = stringPreferencesKey("active_theme_config_id");
                if let Some(encoded) = preferences.getTyped::<String>(&key)? {
                    let selection: Option<String> = serde_json::from_str(&encoded)?;
                    preferences.set(&key, serde_json::to_value(selection)?);
                }
                Ok(())
            }
            2 => operit_store::Transaction::migration::Definition::new(Self::migration)
                .state(2, preferences),
            3 => {
                migrateLegacyThemePreferences(preferences)?;
                let records = preferences
                    .iterEntries()
                    .filter(|(key, _)| key.starts_with(THEME_CONFIG_PREFIX))
                    .map(|(key, value)| (key.to_string(), value.clone()))
                    .collect::<Vec<_>>();
                for (key, value) in records {
                    let mut config: ThemeConfig = serde_json::from_value(value)?;
                    migrateLegacyThemeSnapshot(&mut config.snapshot)?;
                    validateConfig(&config)?;
                    preferences.set(&stringPreferencesKey(&key), serde_json::to_value(config)?);
                }
                Ok(())
            }
            4 => {
                if let Some(style) =
                    preferences.getTyped::<String>(&stringPreferencesKey("chat_style"))?
                {
                    for name in ["user_chat_style", "ai_chat_style"] {
                        let key = stringPreferencesKey(name);
                        if !preferences.contains(&key) {
                            preferences.set(&key, style.clone());
                        }
                    }
                }
                let records = preferences
                    .iterEntries()
                    .filter(|(key, _)| key.starts_with(THEME_CONFIG_PREFIX))
                    .map(|(key, value)| (key.to_string(), value.clone()))
                    .collect::<Vec<_>>();
                for (key, value) in records {
                    let mut config: ThemeConfig = serde_json::from_value(value)?;
                    if let Some(style) = config.snapshot.get("chatStyle").cloned() {
                        for name in ["userChatStyle", "aiChatStyle"] {
                            config
                                .snapshot
                                .entry(name.to_string())
                                .or_insert_with(|| style.clone());
                        }
                    }
                    validateConfig(&config)?;
                    preferences.set(&stringPreferencesKey(&key), serde_json::to_value(config)?);
                }
                Ok(())
            }
            from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
        }
    }

    #[allow(non_snake_case)]
    /// Observes the selected application language code.
    pub fn appLanguage(&self) -> Flow<String> {
        self.dataStore.dataFlow().mapResult(|preferences| {
            Ok(preferences
                .getTyped::<String>(&stringPreferencesKey("app_language"))?
                .unwrap_or_else(|| Self::DEFAULT_LANGUAGE.to_string()))
        })
    }

    #[allow(non_snake_case)]
    /// Saves the selected application language code.
    pub fn saveAppLanguage(&self, languageCode: String) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.edit(|preferences| {
            preferences.set(&stringPreferencesKey("app_language"), languageCode.clone());
        })
    }

    #[allow(non_snake_case)]
    /// Reads the selected application language code.
    pub fn getCurrentLanguage(&self) -> Result<String, PreferencesDataStoreError> {
        self.appLanguage().first()
    }
}

impl PreferencesManager {
    /// Opens the preferences facade backed by user preferences.
    pub fn getInstance() -> Self {
        Self {
            inner: UserPreferencesManager::getInstance(),
        }
    }

    #[allow(non_snake_case)]
    /// Observes the selected application language code.
    pub fn appLanguage(&self) -> Flow<String> {
        self.inner.appLanguage()
    }

    #[allow(non_snake_case)]
    /// Saves the selected application language code.
    pub fn saveAppLanguage(&self, languageCode: String) -> Result<(), PreferencesDataStoreError> {
        self.inner.saveAppLanguage(languageCode)
    }

    #[allow(non_snake_case)]
    /// Reads the selected application language code.
    pub fn getCurrentLanguage(&self) -> Result<String, PreferencesDataStoreError> {
        self.inner.getCurrentLanguage()
    }
}
