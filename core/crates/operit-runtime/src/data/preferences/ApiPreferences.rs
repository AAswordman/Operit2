use std::path::PathBuf;

use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::default_data_dir;

pub struct ApiPreferences {
    apiDataStore: PreferencesDataStore,
}

impl ApiPreferences {
    pub const DEFAULT_API_KEY: &'static str = "";
    pub const DEFAULT_API_ENDPOINT: &'static str = "https://api.deepseek.com/v1/chat/completions";
    pub const DEFAULT_MODEL_NAME: &'static str = "deepseek-chat";
    pub const DEFAULT_CONFIG_ID: &'static str = "default";
    pub const DEFAULT_CONFIG_NAME: &'static str = "model_config_default_name";
    pub const DEFAULT_ENABLE_THINKING_MODE: bool = false;
    pub const DEFAULT_THINKING_QUALITY_LEVEL: i32 = 2;

    pub fn data_dir() -> PathBuf {
        default_data_dir()
    }

    pub fn getInstance() -> Self {
        Self::new(Self::data_dir())
    }

    pub fn new(root_dir: PathBuf) -> Self {
        let path = root_dir.join("api_settings.json");
        Self {
            apiDataStore: PreferencesDataStore::new(path),
        }
    }

    pub fn enableThinkingModeFlow(&self) -> Flow<bool> {
        let store = self.apiDataStore.clone();
        Flow::new(move || {
            let preferences = store.data()?;
            Ok(preferences
                .get(&stringPreferencesKey("enable_thinking_mode"))
                .and_then(|value| value.parse::<bool>().ok())
                .unwrap_or(Self::DEFAULT_ENABLE_THINKING_MODE))
        })
    }

    pub fn thinkingQualityLevelFlow(&self) -> Flow<i32> {
        let store = self.apiDataStore.clone();
        Flow::new(move || {
            let preferences = store.data()?;
            Ok(preferences
                .get(&stringPreferencesKey("thinking_quality_level"))
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or(Self::DEFAULT_THINKING_QUALITY_LEVEL)
                .clamp(1, 4))
        })
    }

    pub fn disableStreamOutputFlow(&self) -> Flow<bool> {
        let store = self.apiDataStore.clone();
        Flow::new(move || {
            let preferences = store.data()?;
            Ok(preferences
                .get(&stringPreferencesKey("disable_stream_output"))
                .and_then(|value| value.parse::<bool>().ok())
                .unwrap_or(false))
        })
    }

    pub fn maxImageHistoryUserTurnsFlow(&self) -> Flow<i32> {
        let store = self.apiDataStore.clone();
        Flow::new(move || {
            let preferences = store.data()?;
            Ok(preferences
                .get(&stringPreferencesKey("max_image_history_user_turns"))
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or(2))
        })
    }

    pub fn maxMediaHistoryUserTurnsFlow(&self) -> Flow<i32> {
        let store = self.apiDataStore.clone();
        Flow::new(move || {
            let preferences = store.data()?;
            Ok(preferences
                .get(&stringPreferencesKey("max_media_history_user_turns"))
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or(1))
        })
    }

    pub fn saveEnableThinkingMode(&self, isEnabled: bool) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(&stringPreferencesKey("enable_thinking_mode"), isEnabled.to_string());
        })
    }

    pub fn saveThinkingQualityLevel(&self, level: i32) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey("thinking_quality_level"),
                level.clamp(1, 4).to_string(),
            );
        })
    }

    pub fn updateThinkingSettings(
        &self,
        enableThinkingMode: Option<bool>,
        thinkingQualityLevel: Option<i32>,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            if let Some(enableThinkingMode) = enableThinkingMode {
                preferences.set(
                    &stringPreferencesKey("enable_thinking_mode"),
                    enableThinkingMode.to_string(),
                );
            }
            if let Some(thinkingQualityLevel) = thinkingQualityLevel {
                preferences.set(
                    &stringPreferencesKey("thinking_quality_level"),
                    thinkingQualityLevel.clamp(1, 4).to_string(),
                );
            }
        })
    }
}
