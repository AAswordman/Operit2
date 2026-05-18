use std::path::PathBuf;

use thiserror::Error;

use crate::data::model::ApiKeyInfo::ApiKeyInfo;
use crate::data::model::ModelConfigData::{ApiProviderType, ModelConfigData, ModelConfigSummary};
use crate::data::model::ModelParameter::{
    CustomParameterData, ModelParameter, ParameterCategory, ParameterValueType,
};
use crate::data::model::StandardModelParameters::StandardModelParameters;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

#[derive(Debug, Error)]
pub enum ModelConfigError {
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("store error: {0}")]
    Store(#[from] PreferencesDataStoreError),
    #[error("model index out of range: {modelIndex}, available model count: {modelCount}")]
    ModelIndexOutOfRange { modelIndex: usize, modelCount: usize },
    #[error("model name list is empty")]
    EmptyModelNameList,
    #[error("custom parameter value type error: {0}")]
    CustomParameterValueType(String),
    #[error("custom parameter category error: {0}")]
    CustomParameterCategory(String),
    #[error("custom parameter conversion error: {0}")]
    CustomParameterConversion(String),
}

pub struct ModelConfigManager {
    paths: RuntimeStorePaths,
    modelConfigDataStore: PreferencesDataStore,
}

impl ModelConfigManager {
    pub const DEFAULT_CONFIG_ID: &'static str = "default";
    pub const DEFAULT_CONFIG_NAME: &'static str = "model_config_default_name";

    pub fn CONFIG_LIST_KEY() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("config_list")
    }

    pub fn new(root_dir: PathBuf) -> Self {
        let paths = RuntimeStorePaths::new(root_dir);
        let modelConfigDataStore =
            PreferencesDataStore::new(paths.model_configs_preferences_path());
        Self {
            paths,
            modelConfigDataStore,
        }
    }

    pub fn default() -> Self {
        Self::new(ApiPreferences::data_dir())
    }

    pub fn initializeIfNeeded(&self) -> Result<(), ModelConfigError> {
        let configList = self.configListFlow()?;
        if configList.is_empty() {
            let defaultConfig = self.createFreshDefaultConfig();
            self.saveConfigToDataStore(&defaultConfig)?;
            let encoded = serde_json::to_string(&vec![Self::DEFAULT_CONFIG_ID.to_string()])?;
            self.modelConfigDataStore.edit(|preferences| {
                preferences.set(&Self::CONFIG_LIST_KEY(), encoded);
            })?;
        }
        Ok(())
    }

    pub fn createFreshDefaultConfig(&self) -> ModelConfigData {
        let mut config = ModelConfigData::new(
            Self::DEFAULT_CONFIG_ID.to_string(),
            Self::DEFAULT_CONFIG_NAME.to_string(),
        );
        config.apiKey = ApiPreferences::DEFAULT_API_KEY.to_string();
        config.apiEndpoint = ApiPreferences::DEFAULT_API_ENDPOINT.to_string();
        config.modelName = ApiPreferences::DEFAULT_MODEL_NAME.to_string();
        config.apiProviderType = ApiProviderType::DEEPSEEK;
        config.apiProviderTypeId = ApiProviderType::DEEPSEEK.name().to_string();
        config.hasCustomParameters = false;
        config.maxTokensEnabled = false;
        config.temperatureEnabled = false;
        config.topPEnabled = false;
        config.topKEnabled = false;
        config.presencePenaltyEnabled = false;
        config.frequencyPenaltyEnabled = false;
        config.repetitionPenaltyEnabled = false;
        config.customParameters = "[]".to_string();
        config
    }

    pub fn configListFlow(&self) -> Result<Vec<String>, ModelConfigError> {
        let preferences = self.modelConfigDataStore.data()?;
        match preferences.get(&Self::CONFIG_LIST_KEY()) {
            Some(configList) if !configList.is_empty() => Ok(serde_json::from_str(configList)?),
            _ => Ok(Vec::new()),
        }
    }

    pub fn saveModelConfig(&self, config: ModelConfigData) -> Result<(), ModelConfigError> {
        let configKey = self.configKey(&config.id);
        let encodedConfig = serde_json::to_string(&config)?;
        self.modelConfigDataStore.edit(|preferences| {
            preferences.set(&configKey, encodedConfig);
        })?;
        Ok(())
    }

    pub fn getConfigIds(&self) -> Result<Vec<String>, ModelConfigError> {
        self.configListFlow()
    }

    pub fn getModelConfigFlow(&self, configId: &str) -> Result<ModelConfigData, ModelConfigError> {
        self.loadConfigFromDataStore(configId)
    }

    pub fn getModelConfig(&self, configId: &str) -> Result<ModelConfigData, ModelConfigError> {
        self.loadConfigFromDataStore(configId)
    }

    pub fn getAllConfigSummaries(&self) -> Result<Vec<ModelConfigSummary>, ModelConfigError> {
        let configIds = self.configListFlow()?;
        let mut summaries = Vec::new();
        for id in configIds {
            let config = self.getModelConfigFlow(&id)?;
            summaries.push(ModelConfigSummary {
                id: config.id.clone(),
                name: config.name.clone(),
                modelName: self.modelNameByIndexFromConfig(&config, 0)?,
                apiEndpoint: config.apiEndpoint.clone(),
                apiProviderType: config.apiProviderType.clone(),
                modelIndex: 0,
            });
        }
        Ok(summaries)
    }

    pub fn createConfig(&self, name: String) -> Result<String, ModelConfigError> {
        let configId = self.createConfigId();
        let mut configList = self.configListFlow()?;
        let mut newConfig = ModelConfigData::new(configId.clone(), name);
        newConfig.apiProviderType = ApiProviderType::OPENAI_GENERIC;
        newConfig.apiProviderTypeId = ApiProviderType::OPENAI_GENERIC.name().to_string();
        newConfig.enableToolCall = true;
        self.saveConfigToDataStore(&newConfig)?;
        configList.push(configId.clone());
        self.saveConfigList(configList)?;
        Ok(configId)
    }

    pub fn deleteConfig(&self, configId: &str) -> Result<(), ModelConfigError> {
        if configId == Self::DEFAULT_CONFIG_ID {
            return Ok(());
        }
        let mut configList = self.configListFlow()?;
        configList.retain(|id| id != configId);
        let configKey = self.configKey(configId);
        let encodedList = serde_json::to_string(&configList)?;
        self.modelConfigDataStore.edit(|preferences| {
            preferences.remove(&configKey);
            preferences.set(&Self::CONFIG_LIST_KEY(), encodedList);
        })?;
        Ok(())
    }

    pub fn saveConfigList(&self, configList: Vec<String>) -> Result<(), ModelConfigError> {
        let encoded = serde_json::to_string(&configList)?;
        self.modelConfigDataStore.edit(|preferences| {
            preferences.set(&Self::CONFIG_LIST_KEY(), encoded);
        })?;
        Ok(())
    }

    pub fn updateConfigBase(&self, configId: &str, name: String) -> Result<ModelConfigData, ModelConfigError> {
        self.updateConfigInternal(configId, |mut config| {
            config.name = name;
            config
        })
    }

    pub fn updateModelConfig(
        &self,
        configId: &str,
        apiKey: String,
        apiEndpoint: String,
        modelName: String,
    ) -> Result<ModelConfigData, ModelConfigError> {
        self.updateConfigInternal(configId, |mut config| {
            config.apiKey = apiKey;
            config.apiEndpoint = apiEndpoint;
            config.modelName = modelName;
            config
        })
    }

    pub fn updateModelConfigWithModelIndex(
        &self,
        configId: &str,
        apiKey: String,
        apiEndpoint: String,
        modelName: String,
        modelIndex: usize,
    ) -> Result<ModelConfigData, ModelConfigError> {
        let config = self.loadConfigFromDataStore(configId)?;
        let modelNames = Self::modelNameListFromConfig(&config);
        if modelNames.is_empty() {
            return Err(ModelConfigError::EmptyModelNameList);
        }
        if modelIndex >= modelNames.len() {
            return Err(ModelConfigError::ModelIndexOutOfRange {
                modelIndex,
                modelCount: modelNames.len(),
            });
        }

        self.updateConfigInternal(configId, |mut config| {
            config.apiKey = apiKey;
            config.apiEndpoint = apiEndpoint;
            let mut modelNames = Self::modelNameListFromConfig(&config);
            modelNames[modelIndex] = modelName;
            config.modelName = Self::joinModelNameList(modelNames);
            config
        })
    }

    pub fn updateModelConfigWithProvider(
        &self,
        configId: &str,
        apiKey: String,
        apiEndpoint: String,
        modelName: String,
        apiProviderType: ApiProviderType,
        apiProviderTypeId: String,
    ) -> Result<ModelConfigData, ModelConfigError> {
        self.updateConfigInternal(configId, |mut config| {
            config.apiKey = apiKey;
            config.apiEndpoint = apiEndpoint;
            config.modelName = modelName;
            config.apiProviderType = apiProviderType;
            config.apiProviderTypeId = apiProviderTypeId;
            config
        })
    }

    pub fn updateModelConfigWithProviderAndMnn(
        &self,
        configId: &str,
        apiKey: String,
        apiEndpoint: String,
        modelName: String,
        apiProviderType: ApiProviderType,
        apiProviderTypeId: String,
        mnnForwardType: i32,
        mnnThreadCount: i32,
    ) -> Result<ModelConfigData, ModelConfigError> {
        self.updateConfigInternal(configId, |mut config| {
            config.apiKey = apiKey;
            config.apiEndpoint = apiEndpoint;
            config.modelName = modelName;
            config.apiProviderType = apiProviderType;
            config.apiProviderTypeId = apiProviderTypeId;
            config.mnnForwardType = mnnForwardType;
            config.mnnThreadCount = mnnThreadCount;
            config
        })
    }

    pub fn updateConfigKeyIndex(&self, configId: &str, newIndex: i32) -> Result<ModelConfigData, ModelConfigError> {
        self.updateConfigInternal(configId, |mut config| {
            config.currentKeyIndex = newIndex;
            config
        })
    }

    pub fn updateApiKey(&self, configId: &str, apiKey: String) -> Result<ModelConfigData, ModelConfigError> {
        self.updateConfigInternal(configId, |mut config| {
            config.apiKey = apiKey;
            config
        })
    }

    pub fn updateApiKeyPool(
        &self,
        configId: &str,
        apiKeyPool: Vec<ApiKeyInfo>,
    ) -> Result<ModelConfigData, ModelConfigError> {
        self.updateConfigInternal(configId, |mut config| {
            config.apiKeyPool = apiKeyPool;
            config.useMultipleApiKeys = true;
            config
        })
    }

    pub fn updateModelName(&self, configId: &str, modelName: String) -> Result<ModelConfigData, ModelConfigError> {
        self.updateConfigInternal(configId, |mut config| {
            config.modelName = modelName;
            config
        })
    }

    pub fn updateModelNameAtIndex(
        &self,
        configId: &str,
        modelIndex: usize,
        modelName: String,
    ) -> Result<ModelConfigData, ModelConfigError> {
        let config = self.loadConfigFromDataStore(configId)?;
        let modelNames = Self::modelNameListFromConfig(&config);
        if modelNames.is_empty() {
            return Err(ModelConfigError::EmptyModelNameList);
        }
        if modelIndex >= modelNames.len() {
            return Err(ModelConfigError::ModelIndexOutOfRange {
                modelIndex,
                modelCount: modelNames.len(),
            });
        }

        self.updateConfigInternal(configId, |mut config| {
            let mut modelNames = Self::modelNameListFromConfig(&config);
            modelNames[modelIndex] = modelName;
            config.modelName = Self::joinModelNameList(modelNames);
            config
        })
    }

    pub fn updateModelNames(
        &self,
        configId: &str,
        modelNames: Vec<String>,
    ) -> Result<ModelConfigData, ModelConfigError> {
        if modelNames.is_empty() {
            return Err(ModelConfigError::EmptyModelNameList);
        }
        self.updateConfigInternal(configId, |mut config| {
            config.modelName = Self::joinModelNameList(modelNames);
            config
        })
    }

    pub fn getModelNameByIndex(
        &self,
        configId: &str,
        modelIndex: usize,
    ) -> Result<String, ModelConfigError> {
        let config = self.loadConfigFromDataStore(configId)?;
        self.modelNameByIndexFromConfig(&config, modelIndex)
    }

    pub fn getModelNames(&self, configId: &str) -> Result<Vec<String>, ModelConfigError> {
        let config = self.loadConfigFromDataStore(configId)?;
        let modelNames = Self::modelNameListFromConfig(&config);
        if modelNames.is_empty() {
            return Err(ModelConfigError::EmptyModelNameList);
        }
        Ok(modelNames)
    }

    pub fn getModelParametersForConfig(
        &self,
        configId: &str,
    ) -> Result<Vec<ModelParameter<serde_json::Value>>, ModelConfigError> {
        let config = self.getModelConfigFlow(configId)?;
        let mut parameters = Vec::new();

        for def in StandardModelParameters::DEFINITIONS() {
            let (currentValue, isEnabled) = match def.id {
                "max_tokens" => (serde_json::json!(config.maxTokens), config.maxTokensEnabled),
                "temperature" => (serde_json::json!(config.temperature), config.temperatureEnabled),
                "top_p" => (serde_json::json!(config.topP), config.topPEnabled),
                "top_k" => (serde_json::json!(config.topK), config.topKEnabled),
                "presence_penalty" => (serde_json::json!(config.presencePenalty), config.presencePenaltyEnabled),
                "frequency_penalty" => (serde_json::json!(config.frequencyPenalty), config.frequencyPenaltyEnabled),
                "repetition_penalty" => (serde_json::json!(config.repetitionPenalty), config.repetitionPenaltyEnabled),
                other => return Err(ModelConfigError::CustomParameterConversion(other.to_string())),
            };

            parameters.push(ModelParameter {
                id: def.id.to_string(),
                name: def.name.to_string(),
                apiName: def.apiName.to_string(),
                description: def.description.to_string(),
                defaultValue: def.defaultValue,
                currentValue,
                isEnabled,
                valueType: def.valueType,
                minValue: def.minValue,
                maxValue: def.maxValue,
                category: def.category,
                isCustom: false,
            });
        }

        if config.hasCustomParameters
            && !config.customParameters.trim().is_empty()
            && config.customParameters.trim() != "[]"
        {
            let customParamsData: Vec<CustomParameterData> =
                serde_json::from_str(&config.customParameters)?;
            for data in customParamsData {
                parameters.push(self.convertCustomParameterData(data)?);
            }
        }

        Ok(parameters)
    }

    pub fn updateEndpoint(&self, configId: &str, apiEndpoint: String) -> Result<ModelConfigData, ModelConfigError> {
        self.updateConfigInternal(configId, |mut config| {
            config.apiEndpoint = apiEndpoint;
            config
        })
    }

    fn loadConfigFromDataStore(&self, configId: &str) -> Result<ModelConfigData, ModelConfigError> {
        let preferences = self.modelConfigDataStore.data()?;
        let configKey = self.configKey(configId);
        match preferences.get(&configKey) {
            Some(configJson) => Ok(serde_json::from_str(configJson)?),
            None => Ok(self.createConfigForId(configId)),
        }
    }

    fn saveConfigToDataStore(&self, config: &ModelConfigData) -> Result<(), ModelConfigError> {
        let configKey = self.configKey(&config.id);
        let encodedConfig = serde_json::to_string(config)?;
        self.modelConfigDataStore.edit(|preferences| {
            preferences.set(&configKey, encodedConfig);
        })?;
        Ok(())
    }

    fn updateConfigInternal<F>(&self, configId: &str, transform: F) -> Result<ModelConfigData, ModelConfigError>
    where
        F: FnOnce(ModelConfigData) -> ModelConfigData,
    {
        let current = self.loadConfigFromDataStore(configId)?;
        let newConfig = transform(current);
        let configKey = self.configKey(configId);
        let encodedConfig = serde_json::to_string(&newConfig)?;
        self.modelConfigDataStore.edit(|preferences| {
            preferences.set(&configKey, encodedConfig);
        })?;
        Ok(newConfig)
    }

    fn configKey(&self, configId: &str) -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey(&format!("config_{configId}"))
    }

    fn createConfigForId(&self, configId: &str) -> ModelConfigData {
        if configId == Self::DEFAULT_CONFIG_ID {
            return self.createFreshDefaultConfig();
        }
        ModelConfigData::new(
            configId.to_string(),
            format!("model_config_config_id_{configId}"),
        )
    }

    fn modelNameByIndexFromConfig(
        &self,
        config: &ModelConfigData,
        modelIndex: usize,
    ) -> Result<String, ModelConfigError> {
        let modelNames = Self::modelNameListFromConfig(config);
        if modelNames.is_empty() {
            return Err(ModelConfigError::EmptyModelNameList);
        }
        match modelNames.get(modelIndex) {
            Some(modelName) => Ok(modelName.clone()),
            None => Err(ModelConfigError::ModelIndexOutOfRange {
                modelIndex,
                modelCount: modelNames.len(),
            }),
        }
    }

    fn modelNameListFromConfig(config: &ModelConfigData) -> Vec<String> {
        config
            .modelName
            .split(',')
            .map(str::trim)
            .filter(|modelName| !modelName.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    }

    fn joinModelNameList(modelNames: Vec<String>) -> String {
        modelNames
            .into_iter()
            .map(|modelName| modelName.trim().to_string())
            .filter(|modelName| !modelName.is_empty())
            .collect::<Vec<_>>()
            .join(",")
    }

    fn createConfigId(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before UNIX_EPOCH")
            .as_millis();
        format!("config_{now}")
    }

    fn convertCustomParameterData(
        &self,
        data: CustomParameterData,
    ) -> Result<ModelParameter<serde_json::Value>, ModelConfigError> {
        let valueType = Self::parseParameterValueType(&data.valueType)?;
        let category = Self::parseParameterCategory(&data.category)?;
        let defaultValue = Self::parseCustomParameterValue(&data.defaultValue, &valueType)?;
        let currentValue = Self::parseCustomParameterValue(&data.currentValue, &valueType)?;
        let minValue = data
            .minValue
            .map(|value| Self::parseCustomParameterValue(&value, &valueType))
            .transpose()?;
        let maxValue = data
            .maxValue
            .map(|value| Self::parseCustomParameterValue(&value, &valueType))
            .transpose()?;

        Ok(ModelParameter {
            id: data.id,
            name: data.name,
            apiName: data.apiName,
            description: data.description,
            defaultValue,
            currentValue,
            isEnabled: data.isEnabled,
            valueType,
            minValue,
            maxValue,
            category,
            isCustom: true,
        })
    }

    fn parseParameterValueType(value: &str) -> Result<ParameterValueType, ModelConfigError> {
        match value {
            "INT" => Ok(ParameterValueType::INT),
            "FLOAT" => Ok(ParameterValueType::FLOAT),
            "STRING" => Ok(ParameterValueType::STRING),
            "BOOLEAN" => Ok(ParameterValueType::BOOLEAN),
            "OBJECT" => Ok(ParameterValueType::OBJECT),
            other => Err(ModelConfigError::CustomParameterValueType(other.to_string())),
        }
    }

    fn parseParameterCategory(value: &str) -> Result<ParameterCategory, ModelConfigError> {
        match value {
            "GENERATION" => Ok(ParameterCategory::GENERATION),
            "CREATIVITY" => Ok(ParameterCategory::CREATIVITY),
            "REPETITION" => Ok(ParameterCategory::REPETITION),
            "OTHER" => Ok(ParameterCategory::OTHER),
            other => Err(ModelConfigError::CustomParameterCategory(other.to_string())),
        }
    }

    fn parseCustomParameterValue(
        value: &str,
        valueType: &ParameterValueType,
    ) -> Result<serde_json::Value, ModelConfigError> {
        match valueType {
            ParameterValueType::INT => value
                .parse::<i32>()
                .map(serde_json::Value::from)
                .map_err(|error| ModelConfigError::CustomParameterConversion(error.to_string())),
            ParameterValueType::FLOAT => value
                .parse::<f32>()
                .map(|parsed| serde_json::json!(parsed))
                .map_err(|error| ModelConfigError::CustomParameterConversion(error.to_string())),
            ParameterValueType::STRING => Ok(serde_json::Value::String(value.to_string())),
            ParameterValueType::BOOLEAN => value
                .parse::<bool>()
                .map(serde_json::Value::from)
                .map_err(|error| ModelConfigError::CustomParameterConversion(error.to_string())),
            ParameterValueType::OBJECT => serde_json::from_str(value).map_err(ModelConfigError::Json),
        }
    }
}
