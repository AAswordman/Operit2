use std::collections::HashMap;
use std::path::PathBuf;

use crate::util::OperitPaths;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, PreferencesDataStore, PreferencesDataStoreError,
};

pub struct ApiPreferences {
    apiDataStore: PreferencesDataStore,
}

impl ApiPreferences {
    pub const DEFAULT_API_KEY: &'static str = "";
    pub const DEFAULT_API_ENDPOINT: &'static str = "https://api.deepseek.com/v1/chat/completions";
    pub const DEFAULT_MODEL_NAME: &'static str = "deepseek-v4-flash";
    pub const DEFAULT_CONFIG_ID: &'static str = "default";
    pub const DEFAULT_CONFIG_NAME: &'static str = "model_config_default_name";
    pub const DEFAULT_ENABLE_THINKING_MODE: bool = false;
    pub const DEFAULT_THINKING_QUALITY_LEVEL: i32 = 2;
    pub const DEFAULT_FEATURE_TOGGLE_STATE: bool = false;
    pub const DEFAULT_ENABLE_MEMORY_AUTO_UPDATE: bool = true;
    pub const DEFAULT_ENABLE_TOOLS: bool = true;
    pub const DEFAULT_DISABLE_STREAM_OUTPUT: bool = false;
    pub const DEFAULT_DISABLE_USER_PREFERENCE_DESCRIPTION: bool = false;
    pub const DEFAULT_MCP_STARTUP_TIMEOUT_SECONDS: i32 = 10;
    pub const DEFAULT_TOOL_PROMPT_VISIBILITY_JSON: &'static str = "{}";
    pub const DEFAULT_FEATURE_TOGGLES_JSON: &'static str = "{}";

    pub fn data_dir() -> PathBuf {
        OperitPaths::operitRootDir().expect("Operit root dir must be available")
    }

    pub fn getInstance() -> Self {
        Self::new(Self::data_dir())
    }

    pub fn new(root_dir: PathBuf) -> Self {
        let path = root_dir.join(OperitPaths::API_PREFERENCES_PATH);
        Self {
            apiDataStore: PreferencesDataStore::new(path),
        }
    }

    pub fn enableThinkingModeFlow(&self) -> Flow<bool> {
        self.apiDataStore.dataFlow().map(|preferences| {
            preferences
                .get(&stringPreferencesKey("enable_thinking_mode"))
                .and_then(|value| value.parse::<bool>().ok())
                .unwrap_or(Self::DEFAULT_ENABLE_THINKING_MODE)
        })
    }

    pub fn featureTogglesFlow(&self) -> Flow<HashMap<String, bool>> {
        self.apiDataStore.dataFlow().map(|preferences| {
            preferences
                .get(&stringPreferencesKey("feature_toggles_json"))
                .map(|value| {
                    serde_json::from_str::<HashMap<String, bool>>(value)
                        .expect("feature_toggles_json must be a boolean map")
                })
                .unwrap_or_else(|| {
                    serde_json::from_str::<HashMap<String, bool>>(
                        Self::DEFAULT_FEATURE_TOGGLES_JSON,
                    )
                    .expect("DEFAULT_FEATURE_TOGGLES_JSON must be a boolean map")
                })
        })
    }

    pub fn featureToggleFlow(&self, featureKey: &str, defaultValue: bool) -> Flow<bool> {
        let normalizedKey = featureKey.trim().to_string();
        self.featureTogglesFlow().map(move |toggles| {
            if normalizedKey.is_empty() {
                defaultValue
            } else {
                toggles.get(&normalizedKey).copied().unwrap_or(defaultValue)
            }
        })
    }

    pub fn thinkingQualityLevelFlow(&self) -> Flow<i32> {
        self.apiDataStore.dataFlow().map(|preferences| {
            preferences
                .get(&stringPreferencesKey("thinking_quality_level"))
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or(Self::DEFAULT_THINKING_QUALITY_LEVEL)
                .clamp(1, 4)
        })
    }

    pub fn enableMemoryAutoUpdateFlow(&self) -> Flow<bool> {
        self.apiDataStore.dataFlow().map(|preferences| {
            preferences
                .get(&stringPreferencesKey("enable_memory_auto_update"))
                .and_then(|value| value.parse::<bool>().ok())
                .unwrap_or(Self::DEFAULT_ENABLE_MEMORY_AUTO_UPDATE)
        })
    }

    pub fn enableToolsFlow(&self) -> Flow<bool> {
        self.apiDataStore.dataFlow().map(|preferences| {
            preferences
                .get(&stringPreferencesKey("enable_tools"))
                .and_then(|value| value.parse::<bool>().ok())
                .unwrap_or(Self::DEFAULT_ENABLE_TOOLS)
        })
    }

    pub fn toolPromptVisibilityFlow(&self) -> Flow<HashMap<String, bool>> {
        self.apiDataStore.dataFlow().map(|preferences| {
            preferences
                .get(&stringPreferencesKey("tool_prompt_visibility_json"))
                .map(|value| {
                    serde_json::from_str::<HashMap<String, bool>>(value)
                        .expect("tool_prompt_visibility_json must be a boolean map")
                })
                .unwrap_or_else(|| {
                    serde_json::from_str::<HashMap<String, bool>>(
                        Self::DEFAULT_TOOL_PROMPT_VISIBILITY_JSON,
                    )
                    .expect("DEFAULT_TOOL_PROMPT_VISIBILITY_JSON must be a boolean map")
                })
        })
    }

    pub fn disableStreamOutputFlow(&self) -> Flow<bool> {
        self.apiDataStore.dataFlow().map(|preferences| {
            preferences
                .get(&stringPreferencesKey("disable_stream_output"))
                .and_then(|value| value.parse::<bool>().ok())
                .unwrap_or(Self::DEFAULT_DISABLE_STREAM_OUTPUT)
        })
    }

    pub fn disableUserPreferenceDescriptionFlow(&self) -> Flow<bool> {
        self.apiDataStore.dataFlow().map(|preferences| {
            preferences
                .get(&stringPreferencesKey("disable_user_preference_description"))
                .and_then(|value| value.parse::<bool>().ok())
                .unwrap_or(Self::DEFAULT_DISABLE_USER_PREFERENCE_DESCRIPTION)
        })
    }

    pub fn maxImageHistoryUserTurnsFlow(&self) -> Flow<i32> {
        self.apiDataStore.dataFlow().map(|preferences| {
            preferences
                .get(&stringPreferencesKey("max_image_history_user_turns"))
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or(2)
        })
    }

    pub fn maxMediaHistoryUserTurnsFlow(&self) -> Flow<i32> {
        self.apiDataStore.dataFlow().map(|preferences| {
            preferences
                .get(&stringPreferencesKey("max_media_history_user_turns"))
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or(1)
        })
    }

    pub fn mcpStartupTimeoutSecondsFlow(&self) -> Flow<i32> {
        self.apiDataStore.dataFlow().map(|preferences| {
            preferences
                .get(&stringPreferencesKey("mcp_startup_timeout_seconds"))
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or(Self::DEFAULT_MCP_STARTUP_TIMEOUT_SECONDS)
                .clamp(1, 10)
        })
    }

    pub fn saveEnableThinkingMode(&self, isEnabled: bool) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey("enable_thinking_mode"),
                isEnabled.to_string(),
            );
        })
    }

    pub fn saveFeatureToggle(
        &self,
        featureKey: &str,
        isEnabled: bool,
    ) -> Result<(), PreferencesDataStoreError> {
        let normalizedKey = featureKey.trim().to_string();
        if normalizedKey.is_empty() {
            return Ok(());
        }
        self.apiDataStore.edit(|preferences| {
            let mut currentMap = preferences
                .get(&stringPreferencesKey("feature_toggles_json"))
                .map(|value| {
                    serde_json::from_str::<HashMap<String, bool>>(value)
                        .expect("feature_toggles_json must be a boolean map")
                })
                .unwrap_or_else(|| {
                    serde_json::from_str::<HashMap<String, bool>>(
                        Self::DEFAULT_FEATURE_TOGGLES_JSON,
                    )
                    .expect("DEFAULT_FEATURE_TOGGLES_JSON must be a boolean map")
                });
            currentMap.insert(normalizedKey.clone(), isEnabled);
            preferences.set(
                &stringPreferencesKey("feature_toggles_json"),
                serde_json::to_string(&currentMap).expect("feature toggle map must serialize"),
            );
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

    pub fn saveEnableMemoryAutoUpdate(
        &self,
        isEnabled: bool,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey("enable_memory_auto_update"),
                isEnabled.to_string(),
            );
        })
    }

    pub fn saveEnableTools(&self, isEnabled: bool) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(&stringPreferencesKey("enable_tools"), isEnabled.to_string());
        })
    }

    pub fn saveToolPromptVisibility(
        &self,
        toolName: &str,
        isVisible: bool,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            let mut currentMap = preferences
                .get(&stringPreferencesKey("tool_prompt_visibility_json"))
                .map(|value| {
                    serde_json::from_str::<HashMap<String, bool>>(value)
                        .expect("tool_prompt_visibility_json must be a boolean map")
                })
                .unwrap_or_else(|| {
                    serde_json::from_str::<HashMap<String, bool>>(
                        Self::DEFAULT_TOOL_PROMPT_VISIBILITY_JSON,
                    )
                    .expect("DEFAULT_TOOL_PROMPT_VISIBILITY_JSON must be a boolean map")
                });
            currentMap.insert(toolName.to_string(), isVisible);
            preferences.set(
                &stringPreferencesKey("tool_prompt_visibility_json"),
                serde_json::to_string(&currentMap)
                    .expect("tool prompt visibility map must serialize"),
            );
        })
    }

    pub fn saveToolPromptVisibilityMap(
        &self,
        visibilityMap: HashMap<String, bool>,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey("tool_prompt_visibility_json"),
                serde_json::to_string(&visibilityMap)
                    .expect("tool prompt visibility map must serialize"),
            );
        })
    }

    pub fn getToolPromptVisibilityMap(
        &self,
    ) -> Result<HashMap<String, bool>, PreferencesDataStoreError> {
        let preferences = self.apiDataStore.data()?;
        let map = preferences
            .get(&stringPreferencesKey("tool_prompt_visibility_json"))
            .map(|value| {
                serde_json::from_str::<HashMap<String, bool>>(value)
                    .expect("tool_prompt_visibility_json must be a boolean map")
            })
            .unwrap_or_else(|| {
                serde_json::from_str::<HashMap<String, bool>>(
                    Self::DEFAULT_TOOL_PROMPT_VISIBILITY_JSON,
                )
                .expect("DEFAULT_TOOL_PROMPT_VISIBILITY_JSON must be a boolean map")
            });
        Ok(map)
    }

    pub fn saveDisableStreamOutput(
        &self,
        isDisabled: bool,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey("disable_stream_output"),
                isDisabled.to_string(),
            );
        })
    }

    pub fn saveDisableUserPreferenceDescription(
        &self,
        isDisabled: bool,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey("disable_user_preference_description"),
                isDisabled.to_string(),
            );
        })
    }

    pub fn updateMediaHistorySettings(
        &self,
        maxImageHistoryUserTurns: i32,
        maxMediaHistoryUserTurns: i32,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey("max_image_history_user_turns"),
                maxImageHistoryUserTurns.to_string(),
            );
            preferences.set(
                &stringPreferencesKey("max_media_history_user_turns"),
                maxMediaHistoryUserTurns.to_string(),
            );
        })
    }

    pub fn saveMcpStartupTimeoutSeconds(
        &self,
        seconds: i32,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey("mcp_startup_timeout_seconds"),
                seconds.clamp(1, 10).to_string(),
            );
        })
    }

    pub fn getMcpStartupTimeoutSeconds(&self) -> Result<i32, PreferencesDataStoreError> {
        let preferences = self.apiDataStore.data()?;
        Ok(preferences
            .get(&stringPreferencesKey("mcp_startup_timeout_seconds"))
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(Self::DEFAULT_MCP_STARTUP_TIMEOUT_SECONDS)
            .clamp(1, 10))
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
