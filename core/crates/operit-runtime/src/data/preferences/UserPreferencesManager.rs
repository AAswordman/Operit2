use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_util::LocaleUtils::LanguageCodes;
use operit_util::OperitPaths;

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
            dataStore: PreferencesDataStore::new(
                OperitPaths::userPreferencesPath()
                    .expect("user preferences path must be available"),
            ),
        }
    }

    #[allow(non_snake_case)]
    /// Initializes the user preference store.
    pub fn initializeIfNeeded(
        &self,
        _defaultProfileName: &str,
    ) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.data()?;
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Observes the selected application language code.
    pub fn appLanguage(&self) -> Flow<String> {
        self.dataStore.dataFlow().map(|preferences| {
            preferences
                .get(&stringPreferencesKey("app_language"))
                .cloned()
                .unwrap_or_else(|| Self::DEFAULT_LANGUAGE.to_string())
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

    #[allow(non_snake_case)]
    /// Initializes the user preference store through the preferences facade.
    pub fn initializeIfNeeded(
        &self,
        defaultProfileName: &str,
    ) -> Result<(), PreferencesDataStoreError> {
        self.inner.initializeIfNeeded(defaultProfileName)
    }
}
