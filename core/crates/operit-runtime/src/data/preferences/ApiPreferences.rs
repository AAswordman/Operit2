use std::collections::HashMap;
use std::path::PathBuf;

use crate::data::model::ModelConfigData::ApiProviderType;
use crate::util::OperitPaths;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, Preferences, PreferencesDataStore, PreferencesDataStoreError,
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

    #[allow(non_snake_case)]
    fn tokenInputKey(providerModel: &str) -> String {
        format!("token_input_{}", Self::encodeProviderModel(providerModel))
    }

    #[allow(non_snake_case)]
    fn tokenCachedInputKey(providerModel: &str) -> String {
        format!(
            "token_cached_input_{}",
            Self::encodeProviderModel(providerModel)
        )
    }

    #[allow(non_snake_case)]
    fn tokenOutputKey(providerModel: &str) -> String {
        format!("token_output_{}", Self::encodeProviderModel(providerModel))
    }

    #[allow(non_snake_case)]
    fn encodeProviderModel(providerModel: &str) -> String {
        providerModel.replace(':', "_")
    }

    #[allow(non_snake_case)]
    fn decodeProviderModelFromKeySuffix(encoded: &str) -> String {
        let mut providerNames = vec![
            ApiProviderType::OPENAI.name(),
            ApiProviderType::OPENAI_RESPONSES.name(),
            ApiProviderType::OPENAI_RESPONSES_GENERIC.name(),
            ApiProviderType::OPENAI_GENERIC.name(),
            ApiProviderType::ANTHROPIC.name(),
            ApiProviderType::ANTHROPIC_GENERIC.name(),
            ApiProviderType::GOOGLE.name(),
            ApiProviderType::GEMINI_GENERIC.name(),
            ApiProviderType::BAIDU.name(),
            ApiProviderType::ALIYUN.name(),
            ApiProviderType::XUNFEI.name(),
            ApiProviderType::ZHIPU.name(),
            ApiProviderType::BAICHUAN.name(),
            ApiProviderType::MOONSHOT.name(),
            ApiProviderType::MIMO.name(),
            ApiProviderType::DEEPSEEK.name(),
            ApiProviderType::MISTRAL.name(),
            ApiProviderType::SILICONFLOW.name(),
            ApiProviderType::IFLOW.name(),
            ApiProviderType::OPENROUTER.name(),
            ApiProviderType::FOUR_ROUTER.name(),
            ApiProviderType::NOUS_PORTAL.name(),
            ApiProviderType::INFINIAI.name(),
            ApiProviderType::ALIPAY_BAILING.name(),
            ApiProviderType::DOUBAO.name(),
            ApiProviderType::NVIDIA.name(),
            ApiProviderType::LMSTUDIO.name(),
            ApiProviderType::OLLAMA.name(),
            ApiProviderType::OPENAI_LOCAL.name(),
            ApiProviderType::MNN.name(),
            ApiProviderType::LLAMA_CPP.name(),
            ApiProviderType::PPINFRA.name(),
            ApiProviderType::NOVITA.name(),
            ApiProviderType::OTHER.name(),
        ];
        providerNames.sort_by_key(|name| std::cmp::Reverse(name.len()));

        for providerName in providerNames {
            if encoded == providerName {
                return providerName.to_string();
            }
            let prefix = format!("{providerName}_");
            if let Some(modelName) = encoded.strip_prefix(&prefix) {
                return format!("{providerName}:{modelName}");
            }
        }

        encoded.replace('_', ":")
    }

    #[allow(non_snake_case)]
    fn readOptionalTokenCount(
        preferences: &Preferences,
        keyName: &str,
    ) -> Result<Option<i64>, PreferencesDataStoreError> {
        preferences
            .get(&stringPreferencesKey(keyName))
            .map(|value| {
                value.parse::<i64>().map_err(|error| {
                    PreferencesDataStoreError::Message(format!(
                        "invalid token count for preference key {keyName}: {error}"
                    ))
                })
            })
            .transpose()
    }

    #[allow(non_snake_case)]
    fn readRequiredTokenCount(
        preferences: &Preferences,
        keyName: &str,
    ) -> Result<i64, PreferencesDataStoreError> {
        Self::readOptionalTokenCount(preferences, keyName)?.ok_or_else(|| {
            PreferencesDataStoreError::Message(format!(
                "token count preference key missing: {keyName}"
            ))
        })
    }

    #[allow(non_snake_case)]
    fn readRecordedTokenCount(
        preferences: &Preferences,
        keyName: &str,
    ) -> Result<i64, PreferencesDataStoreError> {
        match Self::readOptionalTokenCount(preferences, keyName)? {
            Some(value) => Ok(value),
            None => Ok(0),
        }
    }

    #[allow(non_snake_case)]
    fn providerModelTokensFromPreferences(
        preferences: &Preferences,
    ) -> Result<HashMap<String, Vec<i64>>, PreferencesDataStoreError> {
        let mut result = HashMap::new();
        for (keyName, value) in preferences.entries() {
            let Some(encoded) = keyName.strip_prefix("token_input_") else {
                continue;
            };
            let providerModel = Self::decodeProviderModelFromKeySuffix(encoded);
            let inputTokens = value.parse::<i64>().map_err(|error| {
                PreferencesDataStoreError::Message(format!(
                    "invalid token count for preference key {keyName}: {error}"
                ))
            })?;
            let outputTokens =
                Self::readRequiredTokenCount(preferences, &Self::tokenOutputKey(&providerModel))?;
            let cachedInputTokens = Self::readRequiredTokenCount(
                preferences,
                &Self::tokenCachedInputKey(&providerModel),
            )?;
            if inputTokens > 0 || outputTokens > 0 || cachedInputTokens > 0 {
                result.insert(
                    providerModel,
                    vec![inputTokens, outputTokens, cachedInputTokens],
                );
            }
        }
        Ok(result)
    }

    #[allow(non_snake_case)]
    pub fn updateTokensForProviderModel(
        &self,
        providerModel: &str,
        inputTokens: i32,
        outputTokens: i32,
        cachedInputTokens: i32,
    ) -> Result<(), PreferencesDataStoreError> {
        let result = self.apiDataStore.edit_result(|preferences| {
            let inputKey = Self::tokenInputKey(providerModel);
            let cachedInputKey = Self::tokenCachedInputKey(providerModel);
            let outputKey = Self::tokenOutputKey(providerModel);

            let currentInputTokens = Self::readRecordedTokenCount(preferences, &inputKey);
            let currentCachedInputTokens =
                Self::readRecordedTokenCount(preferences, &cachedInputKey);
            let currentOutputTokens = Self::readRecordedTokenCount(preferences, &outputKey);
            let (currentInputTokens, currentCachedInputTokens, currentOutputTokens) = (
                currentInputTokens?,
                currentCachedInputTokens?,
                currentOutputTokens?,
            );

            preferences.set(
                &stringPreferencesKey(&inputKey),
                (currentInputTokens + inputTokens as i64).to_string(),
            );
            preferences.set(
                &stringPreferencesKey(&cachedInputKey),
                (currentCachedInputTokens + cachedInputTokens as i64).to_string(),
            );
            preferences.set(
                &stringPreferencesKey(&outputKey),
                (currentOutputTokens + outputTokens as i64).to_string(),
            );
            Ok(())
        })?;
        result
    }

    #[allow(non_snake_case)]
    pub fn getInputTokensForProviderModel(
        &self,
        providerModel: &str,
    ) -> Result<i64, PreferencesDataStoreError> {
        let preferences = self.apiDataStore.data()?;
        Ok(Self::readRecordedTokenCount(
            &preferences,
            &Self::tokenInputKey(providerModel),
        )?)
    }

    #[allow(non_snake_case)]
    pub fn getCachedInputTokensForProviderModel(
        &self,
        providerModel: &str,
    ) -> Result<i64, PreferencesDataStoreError> {
        let preferences = self.apiDataStore.data()?;
        Ok(Self::readRecordedTokenCount(
            &preferences,
            &Self::tokenCachedInputKey(providerModel),
        )?)
    }

    #[allow(non_snake_case)]
    pub fn getOutputTokensForProviderModel(
        &self,
        providerModel: &str,
    ) -> Result<i64, PreferencesDataStoreError> {
        let preferences = self.apiDataStore.data()?;
        Ok(Self::readRecordedTokenCount(
            &preferences,
            &Self::tokenOutputKey(providerModel),
        )?)
    }

    #[allow(non_snake_case)]
    pub fn getAllProviderModelTokens(
        &self,
    ) -> Result<HashMap<String, Vec<i64>>, PreferencesDataStoreError> {
        Self::providerModelTokensFromPreferences(&self.apiDataStore.data()?)
    }

    #[allow(non_snake_case)]
    pub fn allProviderModelTokensFlow(&self) -> Flow<HashMap<String, Vec<i64>>> {
        self.apiDataStore
            .dataFlow()
            .mapResult(|preferences| Self::providerModelTokensFromPreferences(&preferences))
    }

    #[allow(non_snake_case)]
    pub fn resetAllProviderModelTokenCounts(&self) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            let keysToRemove = preferences
                .entries()
                .into_iter()
                .filter_map(|(keyName, _)| {
                    let isStatsKey = keyName.starts_with("token_input_")
                        || keyName.starts_with("token_output_")
                        || keyName.starts_with("token_cached_input_");
                    isStatsKey.then_some(keyName)
                })
                .collect::<Vec<_>>();
            for keyName in keysToRemove {
                preferences.remove(&stringPreferencesKey(&keyName));
            }
        })
    }

    #[allow(non_snake_case)]
    pub fn resetProviderModelTokenCounts(
        &self,
        providerModel: &str,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey(&Self::tokenInputKey(providerModel)),
                "0".to_string(),
            );
            preferences.set(
                &stringPreferencesKey(&Self::tokenCachedInputKey(providerModel)),
                "0".to_string(),
            );
            preferences.set(
                &stringPreferencesKey(&Self::tokenOutputKey(providerModel)),
                "0".to_string(),
            );
        })
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
