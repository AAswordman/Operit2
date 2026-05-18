use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::data::model::FunctionType::FunctionType;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionConfigMapping {
    #[serde(default = "FunctionalConfigManager::defaultConfigId")]
    pub configId: String,
    #[serde(default)]
    pub modelIndex: i32,
}

impl Default for FunctionConfigMapping {
    fn default() -> Self {
        Self {
            configId: FunctionalConfigManager::DEFAULT_CONFIG_ID.to_string(),
            modelIndex: 0,
        }
    }
}

impl FunctionConfigMapping {
    pub fn new(configId: String, modelIndex: i32) -> Self {
        Self {
            configId,
            modelIndex,
        }
    }
}

#[derive(Debug, Error)]
pub enum FunctionalConfigError {
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("store error: {0}")]
    Store(#[from] PreferencesDataStoreError),
    #[error("model config manager error: {0}")]
    ModelConfigManager(String),
    #[error("unknown FunctionType: {0}")]
    UnknownFunctionType(String),
}

pub struct FunctionalConfigManager {
    functionalConfigDataStore: PreferencesDataStore,
    modelConfigManager: ModelConfigManager,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StoredFunctionConfigMapping {
    WithIndex(FunctionConfigMapping),
    ConfigId(String),
}

impl FunctionalConfigManager {
    pub const DEFAULT_CONFIG_ID: &'static str = "default";

    pub fn FUNCTION_CONFIG_MAPPING() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("function_config_mapping")
    }

    pub fn new(root_dir: PathBuf) -> Self {
        let paths = RuntimeStorePaths::new(root_dir.clone());
        Self {
            functionalConfigDataStore: PreferencesDataStore::new(
                paths.functional_configs_preferences_path(),
            ),
            modelConfigManager: ModelConfigManager::new(root_dir),
        }
    }

    pub fn default() -> Self {
        Self::new(ApiPreferences::data_dir())
    }

    pub fn initializeIfNeeded(&self) -> Result<(), FunctionalConfigError> {
        let mapping = self.functionConfigMappingWithIndexFlow()?;
        if mapping.is_empty() {
            self.saveFunctionConfigMappingWithIndex(Self::defaultMapping())?;
        }

        self.modelConfigManager
            .initializeIfNeeded()
            .map_err(|error| FunctionalConfigError::ModelConfigManager(error.to_string()))?;
        Ok(())
    }

    pub fn functionConfigMappingFlow(
        &self,
    ) -> Result<HashMap<FunctionType, String>, FunctionalConfigError> {
        let mapping = self.functionConfigMappingWithIndexFlow()?;
        Ok(mapping
            .into_iter()
            .map(|(functionType, mapping)| (functionType, mapping.configId))
            .collect())
    }

    pub fn functionConfigMappingWithIndexFlow(
        &self,
    ) -> Result<HashMap<FunctionType, FunctionConfigMapping>, FunctionalConfigError> {
        let preferences = self.functionalConfigDataStore.data()?;
        let mappingJson = preferences
            .get(&Self::FUNCTION_CONFIG_MAPPING())
            .cloned()
            .unwrap_or_else(|| "{}".to_string());

        if mappingJson == "{}" {
            return Ok(Self::defaultMapping());
        }

        let rawMap: HashMap<String, StoredFunctionConfigMapping> =
            serde_json::from_str(&mappingJson)?;
        rawMap
            .into_iter()
            .map(|(key, storedMapping)| {
                let functionType = Self::parseFunctionType(&key)?;
                let mapping = match storedMapping {
                    StoredFunctionConfigMapping::WithIndex(mapping) => mapping,
                    StoredFunctionConfigMapping::ConfigId(configId) => {
                        FunctionConfigMapping::new(configId, 0)
                    }
                };
                Ok((functionType, mapping))
            })
            .collect()
    }

    pub fn saveFunctionConfigMapping(
        &self,
        mapping: HashMap<FunctionType, String>,
    ) -> Result<(), FunctionalConfigError> {
        let mappingWithIndex = mapping
            .into_iter()
            .map(|(functionType, configId)| {
                (functionType, FunctionConfigMapping::new(configId, 0))
            })
            .collect();
        self.saveFunctionConfigMappingWithIndex(mappingWithIndex)
    }

    pub fn saveFunctionConfigMappingWithIndex(
        &self,
        mapping: HashMap<FunctionType, FunctionConfigMapping>,
    ) -> Result<(), FunctionalConfigError> {
        let stringMapping: HashMap<String, FunctionConfigMapping> = mapping
            .into_iter()
            .map(|(functionType, value)| (Self::functionTypeName(functionType).to_string(), value))
            .collect();
        let encoded = serde_json::to_string(&stringMapping)?;
        self.functionalConfigDataStore.edit(|preferences| {
            preferences.set(&Self::FUNCTION_CONFIG_MAPPING(), encoded);
        })?;
        Ok(())
    }

    pub fn getConfigIdForFunction(
        &self,
        functionType: FunctionType,
    ) -> Result<String, FunctionalConfigError> {
        let mapping = self.functionConfigMappingFlow()?;
        Ok(mapping
            .get(&functionType)
            .cloned()
            .unwrap_or_else(|| Self::DEFAULT_CONFIG_ID.to_string()))
    }

    pub fn getConfigMappingForFunction(
        &self,
        functionType: FunctionType,
    ) -> Result<FunctionConfigMapping, FunctionalConfigError> {
        let mapping = self.functionConfigMappingWithIndexFlow()?;
        Ok(mapping
            .get(&functionType)
            .cloned()
            .unwrap_or_default())
    }

    pub fn setConfigForFunction(
        &self,
        functionType: FunctionType,
        configId: String,
    ) -> Result<(), FunctionalConfigError> {
        self.setConfigForFunctionWithIndex(functionType, configId, 0)
    }

    pub fn setConfigForFunctionWithIndex(
        &self,
        functionType: FunctionType,
        configId: String,
        modelIndex: i32,
    ) -> Result<(), FunctionalConfigError> {
        let mut mapping = self.functionConfigMappingWithIndexFlow()?;
        mapping.insert(functionType, FunctionConfigMapping::new(configId, modelIndex));
        self.saveFunctionConfigMappingWithIndex(mapping)
    }

    pub fn resetFunctionConfig(
        &self,
        functionType: FunctionType,
    ) -> Result<(), FunctionalConfigError> {
        self.setConfigForFunction(functionType, Self::DEFAULT_CONFIG_ID.to_string())
    }

    pub fn resetAllFunctionConfigs(&self) -> Result<(), FunctionalConfigError> {
        self.saveFunctionConfigMappingWithIndex(Self::defaultMapping())
    }

    fn defaultConfigId() -> String {
        Self::DEFAULT_CONFIG_ID.to_string()
    }

    fn defaultMapping() -> HashMap<FunctionType, FunctionConfigMapping> {
        Self::functionTypeValues()
            .into_iter()
            .map(|functionType| {
                (
                    functionType,
                    FunctionConfigMapping::new(Self::DEFAULT_CONFIG_ID.to_string(), 0),
                )
            })
            .collect()
    }

    fn functionTypeValues() -> Vec<FunctionType> {
        vec![
            FunctionType::CHAT,
            FunctionType::SUMMARY,
            FunctionType::MEMORY,
            FunctionType::UI_CONTROLLER,
            FunctionType::TRANSLATION,
            FunctionType::GREP,
            FunctionType::ROLE_RESPONSE_PLANNER,
            FunctionType::IMAGE_RECOGNITION,
            FunctionType::AUDIO_RECOGNITION,
            FunctionType::VIDEO_RECOGNITION,
        ]
    }

    fn functionTypeName(functionType: FunctionType) -> &'static str {
        match functionType {
            FunctionType::CHAT => "CHAT",
            FunctionType::SUMMARY => "SUMMARY",
            FunctionType::MEMORY => "MEMORY",
            FunctionType::UI_CONTROLLER => "UI_CONTROLLER",
            FunctionType::TRANSLATION => "TRANSLATION",
            FunctionType::GREP => "GREP",
            FunctionType::ROLE_RESPONSE_PLANNER => "ROLE_RESPONSE_PLANNER",
            FunctionType::IMAGE_RECOGNITION => "IMAGE_RECOGNITION",
            FunctionType::AUDIO_RECOGNITION => "AUDIO_RECOGNITION",
            FunctionType::VIDEO_RECOGNITION => "VIDEO_RECOGNITION",
        }
    }

    fn parseFunctionType(value: &str) -> Result<FunctionType, FunctionalConfigError> {
        match value {
            "CHAT" => Ok(FunctionType::CHAT),
            "SUMMARY" => Ok(FunctionType::SUMMARY),
            "MEMORY" => Ok(FunctionType::MEMORY),
            "UI_CONTROLLER" => Ok(FunctionType::UI_CONTROLLER),
            "TRANSLATION" => Ok(FunctionType::TRANSLATION),
            "GREP" => Ok(FunctionType::GREP),
            "ROLE_RESPONSE_PLANNER" => Ok(FunctionType::ROLE_RESPONSE_PLANNER),
            "IMAGE_RECOGNITION" => Ok(FunctionType::IMAGE_RECOGNITION),
            "AUDIO_RECOGNITION" => Ok(FunctionType::AUDIO_RECOGNITION),
            "VIDEO_RECOGNITION" => Ok(FunctionType::VIDEO_RECOGNITION),
            _ => Err(FunctionalConfigError::UnknownFunctionType(value.to_string())),
        }
    }
}
