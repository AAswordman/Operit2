use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

use operit_host_api::RuntimeStorageHost;
use operit_model::ModelConfigData::ApiProviderType;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, Preferences, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use operit_util::OperitPaths;

/// Stores API, token accounting, tool visibility, and runtime preference values.
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
    pub const DEFAULT_ENABLE_TOOLS: bool = true;
    pub const DEFAULT_DISABLE_STREAM_OUTPUT: bool = false;
    pub const DEFAULT_DISABLE_USER_PREFERENCE_DESCRIPTION: bool = false;
    pub const DEFAULT_MCP_STARTUP_TIMEOUT_SECONDS: i32 = 10;
    pub const DEFAULT_TOOLPKG_PRE_HOOK_TIMEOUT_SECONDS: i32 = 10;
    pub const DEFAULT_TOOL_PROMPT_VISIBILITY_JSON: &'static str = "{}";
    pub const DEFAULT_FEATURE_TOGGLES_JSON: &'static str = "{}";

    /// Returns the root directory used for API preference storage.
    pub fn data_dir() -> PathBuf {
        defaultRuntimeStorageHost()
            .runtimeRootDir()
            .expect("runtime storage host must provide an API preference root")
    }

    /// Creates preferences using the default runtime data directory.
    pub fn getInstance() -> Self {
        Self {
            apiDataStore: Self::store(defaultRuntimeStorageHost()),
        }
    }

    /// Opens this module's single preference owner using an explicit runtime storage host.
    pub(crate) fn store(storage: std::sync::Arc<dyn RuntimeStorageHost>) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(storage, OperitPaths::API_PREFERENCES_PATH)
            .withSchema(2, Self::migratePreferences)
    }



    /// Converts historical API values to the current typed value contract.
    pub(crate) fn decodeSyncedEntry(entry: &mut operit_store::PreferencesDataStore::PreferencesSyncedEntry) -> Result<(), PreferencesDataStoreError> {
        entry.decodeHistoricalValues(|version, values| {
            if version > 2 { return Err(PreferencesDataStoreError::SchemaVersionTooNew { actual: version, expected: 2 }); }
            if version < 2 { Self::migratePreferences(1, values)?; }
            Ok(())
        })
    }

    /// Reads the committed typed values for another owning runtime module.
    pub(crate) fn snapshot(&self) -> Result<Preferences, PreferencesDataStoreError> {
        self.apiDataStore.data()
    }

    /// Creates preferences rooted at the supplied directory.
    pub fn new(root_dir: PathBuf) -> Self {
        let path = OperitPaths::runtimePathFromRoot(&root_dir, OperitPaths::API_PREFERENCES_PATH)
            .expect("API preferences path must use the runtime storage prefix");
        Self {
            apiDataStore: PreferencesDataStore::new(path).withSchema(2, Self::migratePreferences),
        }
    }

    /// Converts the API module's declared scalar and structured values at version one.
    pub(crate) fn migratePreferences(
        version: u32,
        preferences: &mut Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        match version {
            0 => Ok(()),
            1 => {
                for key in [
                    "enable_thinking_mode",
                    "enable_tools",
                    "disable_stream_output",
                    "disable_user_preference_description",
                    "enable_memory_auto_update",
                ] {
                    let key = stringPreferencesKey(key);
                    if let Some(encoded) = preferences.getTyped::<String>(&key)? {
                        let value: bool = serde_json::from_str(&encoded)?;
                        preferences.set(&key, value);
                    }
                }
                let counters = preferences
                    .iterEntries()
                    .filter_map(|(key, _)| {
                        (key.starts_with("token_input_")
                            || key.starts_with("token_cached_input_")
                            || key.starts_with("token_output_"))
                        .then(|| key.to_owned())
                    })
                    .chain(
                        [
                            "thinking_quality_level",
                            "max_image_history_user_turns",
                            "max_media_history_user_turns",
                            "mcp_startup_timeout_seconds",
                            "toolpkg_pre_hook_timeout_seconds",
                        ]
                        .into_iter()
                        .map(str::to_owned),
                    )
                    .collect::<Vec<_>>();
                for key in counters {
                    let key = stringPreferencesKey(&key);
                    if let Some(encoded) = preferences.getTyped::<String>(&key)? {
                        let value: i64 = serde_json::from_str(&encoded)?;
                        preferences.set(&key, value);
                    }
                }
                for key in ["feature_toggles_json", "tool_prompt_visibility_json"] {
                    let key = stringPreferencesKey(key);
                    if let Some(encoded) = preferences.getTyped::<String>(&key)? {
                        let value: HashMap<String, bool> = serde_json::from_str(&encoded)?;
                        preferences.set(&key, serde_json::to_value(value)?);
                    }
                }
                Ok(())
            }
            from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
        }
    }

    #[allow(non_snake_case)]
    /// Builds the key for uncached input tokens for one provider/model.
    fn tokenInputKey(providerModel: &str) -> String {
        format!("token_input_{}", Self::encodeProviderModel(providerModel))
    }

    #[allow(non_snake_case)]
    /// Builds the key for cached input tokens for one provider/model.
    fn tokenCachedInputKey(providerModel: &str) -> String {
        format!(
            "token_cached_input_{}",
            Self::encodeProviderModel(providerModel)
        )
    }

    #[allow(non_snake_case)]
    /// Builds the key for output tokens for one provider/model.
    fn tokenOutputKey(providerModel: &str) -> String {
        format!("token_output_{}", Self::encodeProviderModel(providerModel))
    }

    #[allow(non_snake_case)]
    /// Encodes a provider/model identifier for safe preference keys.
    fn encodeProviderModel(providerModel: &str) -> String {
        providerModel.replace(':', "_")
    }

    #[allow(non_snake_case)]
    /// Decodes a provider/model identifier from a preference-key suffix.
    fn decodeProviderModelFromKeySuffix(encoded: &str) -> String {
        let mut providerNames = vec![
            ApiProviderType::OPENAI.name(),
            ApiProviderType::XAI.name(),
            ApiProviderType::OPENAI_RESPONSES.name(),
            ApiProviderType::OPENAI_CODEX.name(),
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
            ApiProviderType::OPENROUTER.name(),
            ApiProviderType::OPENCODE.name(),
            ApiProviderType::FOUR_ROUTER.name(),
            ApiProviderType::NOUS_PORTAL.name(),
            ApiProviderType::INFINIAI.name(),
            ApiProviderType::ALIPAY_BAILING.name(),
            ApiProviderType::DOUBAO.name(),
            ApiProviderType::NVIDIA.name(),
            ApiProviderType::LMSTUDIO.name(),
            ApiProviderType::OLLAMA.name(),
            ApiProviderType::OPENAI_LOCAL.name(),
            ApiProviderType::LOCAL_MODEL.name(),
            ApiProviderType::PPINFRA.name(),
            ApiProviderType::NOVITA.name(),
            ApiProviderType::MINIMAX.name(),
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
                serde_json::from_value::<i64>(value.clone()).map_err(|error| {
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
            let inputTokens = serde_json::from_value::<i64>(value.clone()).map_err(|error| {
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
    /// Adds token counts to the cumulative total for one provider/model.
    pub fn updateTokensForProviderModel(
        &self,
        providerModel: &str,
        inputTokens: i64,
        outputTokens: i64,
        cachedInputTokens: i64,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.try_edit_result(|preferences| {
            preferences.incrementInteger(
                &stringPreferencesKey(&Self::tokenInputKey(providerModel)),
                inputTokens,
            )?;
            preferences.incrementInteger(
                &stringPreferencesKey(&Self::tokenCachedInputKey(providerModel)),
                cachedInputTokens,
            )?;
            preferences.incrementInteger(
                &stringPreferencesKey(&Self::tokenOutputKey(providerModel)),
                outputTokens,
            )?;
            Ok(())
        })
    }

    #[allow(non_snake_case)]
    /// Reads uncached input token count for one provider/model.
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
    /// Reads cached input token count for one provider/model.
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
    /// Reads output token count for one provider/model.
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
    /// Reads all provider/model token counters.
    pub fn getAllProviderModelTokens(
        &self,
    ) -> Result<HashMap<String, Vec<i64>>, PreferencesDataStoreError> {
        Self::providerModelTokensFromPreferences(&self.apiDataStore.data()?)
    }

    #[allow(non_snake_case)]
    /// Observes all provider/model token counters.
    pub fn allProviderModelTokensFlow(&self) -> Flow<HashMap<String, Vec<i64>>> {
        self.apiDataStore
            .dataFlow()
            .mapResult(|preferences| Self::providerModelTokensFromPreferences(&preferences))
    }

    #[allow(non_snake_case)]
    /// Clears every provider/model token counter.
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
    /// Clears token counters for one provider/model.
    pub fn resetProviderModelTokenCounts(
        &self,
        providerModel: &str,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey(&Self::tokenInputKey(providerModel)),
                0,
            );
            preferences.set(
                &stringPreferencesKey(&Self::tokenCachedInputKey(providerModel)),
                0,
            );
            preferences.set(
                &stringPreferencesKey(&Self::tokenOutputKey(providerModel)),
                0,
            );
        })
    }

    /// Observes whether thinking mode is enabled.
    pub fn enableThinkingModeFlow(&self) -> Flow<bool> {
        self.apiDataStore.dataFlow().mapResult(|preferences| {
            Ok(preferences
                .getTyped::<bool>(&stringPreferencesKey("enable_thinking_mode"))?
                .unwrap_or(Self::DEFAULT_ENABLE_THINKING_MODE))
        })
    }

    /// Observes the persisted feature toggle map.
    pub fn featureTogglesFlow(&self) -> Flow<HashMap<String, bool>> {
        self.apiDataStore.dataFlow().map(|preferences| {
            preferences
                .get(&stringPreferencesKey("feature_toggles_json"))
                .map(|value| {
                    serde_json::from_value::<HashMap<String, bool>>(value.clone())
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

    /// Observes one feature toggle with an explicit default.
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

    /// Observes the current thinking quality level.
    pub fn thinkingQualityLevelFlow(&self) -> Flow<i32> {
        self.apiDataStore.dataFlow().mapResult(|preferences| {
            Ok(preferences
                .getTyped::<i32>(&stringPreferencesKey("thinking_quality_level"))?
                .unwrap_or(Self::DEFAULT_THINKING_QUALITY_LEVEL)
                .clamp(1, 4))
        })
    }

    /// Observes whether AI tools are enabled.
    pub fn enableToolsFlow(&self) -> Flow<bool> {
        self.apiDataStore.dataFlow().mapResult(|preferences| {
            Ok(preferences
                .getTyped::<bool>(&stringPreferencesKey("enable_tools"))?
                .unwrap_or(Self::DEFAULT_ENABLE_TOOLS))
        })
    }

    /// Observes per-tool prompt visibility settings.
    pub fn toolPromptVisibilityFlow(&self) -> Flow<HashMap<String, bool>> {
        self.apiDataStore.dataFlow().map(|preferences| {
            preferences
                .get(&stringPreferencesKey("tool_prompt_visibility_json"))
                .map(|value| {
                    serde_json::from_value::<HashMap<String, bool>>(value.clone())
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

    /// Observes whether streaming output is disabled.
    pub fn disableStreamOutputFlow(&self) -> Flow<bool> {
        self.apiDataStore.dataFlow().mapResult(|preferences| {
            Ok(preferences
                .getTyped::<bool>(&stringPreferencesKey("disable_stream_output"))?
                .unwrap_or(Self::DEFAULT_DISABLE_STREAM_OUTPUT))
        })
    }

    /// Observes whether user preference descriptions are hidden from prompts.
    pub fn disableUserPreferenceDescriptionFlow(&self) -> Flow<bool> {
        self.apiDataStore.dataFlow().mapResult(|preferences| {
            Ok(preferences
                .getTyped::<bool>(&stringPreferencesKey("disable_user_preference_description"))?
                .unwrap_or(Self::DEFAULT_DISABLE_USER_PREFERENCE_DESCRIPTION))
        })
    }

    /// Observes the image-history turn limit.
    pub fn maxImageHistoryUserTurnsFlow(&self) -> Flow<i32> {
        self.apiDataStore.dataFlow().mapResult(|preferences| {
            Ok(preferences
                .getTyped::<i32>(&stringPreferencesKey("max_image_history_user_turns"))?
                .unwrap_or(2))
        })
    }

    /// Observes the media-history turn limit.
    pub fn maxMediaHistoryUserTurnsFlow(&self) -> Flow<i32> {
        self.apiDataStore.dataFlow().mapResult(|preferences| {
            Ok(preferences
                .getTyped::<i32>(&stringPreferencesKey("max_media_history_user_turns"))?
                .unwrap_or(1))
        })
    }

    /// Observes the MCP startup timeout in seconds.
    pub fn mcpStartupTimeoutSecondsFlow(&self) -> Flow<i32> {
        self.apiDataStore.dataFlow().mapResult(|preferences| {
            Ok(preferences
                .getTyped::<i32>(&stringPreferencesKey("mcp_startup_timeout_seconds"))?
                .unwrap_or(Self::DEFAULT_MCP_STARTUP_TIMEOUT_SECONDS)
                .clamp(1, 10))
        })
    }

    /// Observes the total timeout for one ToolPkg pre-hook dispatch chain in seconds.
    #[allow(non_snake_case)]
    pub fn toolPkgPreHookTimeoutSecondsFlow(&self) -> Flow<i32> {
        self.apiDataStore.dataFlow().mapResult(|preferences| {
            Ok(preferences
                .getTyped::<i32>(&stringPreferencesKey("toolpkg_pre_hook_timeout_seconds"))?
                .unwrap_or(Self::DEFAULT_TOOLPKG_PRE_HOOK_TIMEOUT_SECONDS)
                .clamp(1, 60))
        })
    }

    /// Saves the thinking-mode toggle.
    pub fn saveEnableThinkingMode(&self, isEnabled: bool) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(&stringPreferencesKey("enable_thinking_mode"), isEnabled);
        })
    }

    /// Saves one feature toggle value.
    pub fn saveFeatureToggle(
        &self,
        featureKey: &str,
        isEnabled: bool,
    ) -> Result<(), PreferencesDataStoreError> {
        let normalizedKey = featureKey.trim().to_string();
        if normalizedKey.is_empty() {
            return Ok(());
        }
        self.apiDataStore.try_edit_result(|preferences| {
            preferences.setObjectField(
                &stringPreferencesKey("feature_toggles_json"),
                &normalizedKey,
                Value::Bool(isEnabled),
            )
        })
    }

    /// Saves the thinking quality level.
    pub fn saveThinkingQualityLevel(&self, level: i32) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey("thinking_quality_level"),
                level.clamp(1, 4),
            );
        })
    }

    /// Saves the global tools toggle.
    pub fn saveEnableTools(&self, isEnabled: bool) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(&stringPreferencesKey("enable_tools"), isEnabled);
        })
    }

    /// Saves prompt visibility for one tool.
    pub fn saveToolPromptVisibility(
        &self,
        toolName: &str,
        isVisible: bool,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.try_edit_result(|preferences| {
            preferences.setObjectField(
                &stringPreferencesKey("tool_prompt_visibility_json"),
                toolName,
                Value::Bool(isVisible),
            )
        })
    }

    /// Replaces the full tool prompt visibility map.
    pub fn saveToolPromptVisibilityMap(
        &self,
        visibilityMap: HashMap<String, bool>,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey("tool_prompt_visibility_json"),
                serde_json::to_value(&visibilityMap)
                    .expect("tool prompt visibility map must serialize"),
            );
        })
    }

    /// Reads the full tool prompt visibility map.
    pub fn getToolPromptVisibilityMap(
        &self,
    ) -> Result<HashMap<String, bool>, PreferencesDataStoreError> {
        let preferences = self.apiDataStore.data()?;
        let map = preferences
            .get(&stringPreferencesKey("tool_prompt_visibility_json"))
            .map(|value| {
                serde_json::from_value::<HashMap<String, bool>>(value.clone())
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

    /// Saves whether streaming output should be disabled.
    pub fn saveDisableStreamOutput(
        &self,
        isDisabled: bool,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(&stringPreferencesKey("disable_stream_output"), isDisabled);
        })
    }

    /// Saves whether user preference descriptions should be hidden from prompts.
    pub fn saveDisableUserPreferenceDescription(
        &self,
        isDisabled: bool,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey("disable_user_preference_description"),
                isDisabled,
            );
        })
    }

    /// Saves image and media history turn limits.
    pub fn updateMediaHistorySettings(
        &self,
        maxImageHistoryUserTurns: i32,
        maxMediaHistoryUserTurns: i32,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey("max_image_history_user_turns"),
                maxImageHistoryUserTurns,
            );
            preferences.set(
                &stringPreferencesKey("max_media_history_user_turns"),
                maxMediaHistoryUserTurns,
            );
        })
    }

    /// Saves the MCP startup timeout in seconds.
    pub fn saveMcpStartupTimeoutSeconds(
        &self,
        seconds: i32,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey("mcp_startup_timeout_seconds"),
                seconds.clamp(1, 10),
            );
        })
    }

    /// Saves the total timeout for one ToolPkg pre-hook dispatch chain in seconds.
    #[allow(non_snake_case)]
    pub fn saveToolPkgPreHookTimeoutSeconds(
        &self,
        seconds: i32,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey("toolpkg_pre_hook_timeout_seconds"),
                seconds.clamp(1, 60),
            );
        })
    }

    /// Reads the MCP startup timeout in seconds.
    pub fn getMcpStartupTimeoutSeconds(&self) -> Result<i32, PreferencesDataStoreError> {
        let preferences = self.apiDataStore.data()?;
        Ok(preferences
            .getTyped::<i32>(&stringPreferencesKey("mcp_startup_timeout_seconds"))?
            .unwrap_or(Self::DEFAULT_MCP_STARTUP_TIMEOUT_SECONDS)
            .clamp(1, 10))
    }

    /// Reads the total timeout for one ToolPkg pre-hook dispatch chain in seconds.
    #[allow(non_snake_case)]
    pub fn getToolPkgPreHookTimeoutSeconds(&self) -> Result<i32, PreferencesDataStoreError> {
        let preferences = self.apiDataStore.data()?;
        Ok(preferences
            .getTyped::<i32>(&stringPreferencesKey("toolpkg_pre_hook_timeout_seconds"))?
            .unwrap_or(Self::DEFAULT_TOOLPKG_PRE_HOOK_TIMEOUT_SECONDS)
            .clamp(1, 60))
    }

    /// Updates thinking settings in one edit transaction.
    pub fn updateThinkingSettings(
        &self,
        enableThinkingMode: Option<bool>,
        thinkingQualityLevel: Option<i32>,
    ) -> Result<(), PreferencesDataStoreError> {
        self.apiDataStore.edit(|preferences| {
            if let Some(enableThinkingMode) = enableThinkingMode {
                preferences.set(
                    &stringPreferencesKey("enable_thinking_mode"),
                    enableThinkingMode,
                );
            }
            if let Some(thinkingQualityLevel) = thinkingQualityLevel {
                preferences.set(
                    &stringPreferencesKey("thinking_quality_level"),
                    thinkingQualityLevel.clamp(1, 4),
                );
            }
        })
    }
}
