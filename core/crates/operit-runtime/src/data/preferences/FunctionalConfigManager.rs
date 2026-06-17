use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::data::model::FunctionType::FunctionType;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, Preferences, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct FunctionModelBinding {
    pub providerId: String,
    pub modelId: String,
}

impl Default for FunctionModelBinding {
    fn default() -> Self {
        Self {
            providerId: ModelConfigManager::DEFAULT_PROVIDER_ID.to_string(),
            modelId: ModelConfigManager::DEFAULT_MODEL_ID.to_string(),
        }
    }
}

impl FunctionModelBinding {
    pub fn new(providerId: String, modelId: String) -> Self {
        Self {
            providerId,
            modelId,
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

#[derive(Clone)]
pub struct FunctionalConfigManager {
    functionalConfigDataStore: PreferencesDataStore,
    modelConfigManager: ModelConfigManager,
}

impl FunctionalConfigManager {
    pub fn FUNCTION_MODEL_BINDING() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("function_model_binding")
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
        self.modelConfigManager
            .initializeIfNeeded()
            .map_err(|error| FunctionalConfigError::ModelConfigManager(error.to_string()))?;

        let binding = self.functionModelBindingFlow()?.first()?;
        if binding.is_empty() {
            self.saveFunctionModelBinding(Self::defaultBinding())?;
        }
        Ok(())
    }

    pub fn functionModelBindingFlow(
        &self,
    ) -> Result<Flow<HashMap<FunctionType, FunctionModelBinding>>, FunctionalConfigError> {
        Ok(self
            .functionalConfigDataStore
            .dataFlow()
            .mapResult(|preferences| Self::readFunctionModelBinding(&preferences)))
    }

    fn readFunctionModelBinding(
        preferences: &Preferences,
    ) -> Result<HashMap<FunctionType, FunctionModelBinding>, PreferencesDataStoreError> {
        let Some(bindingJson) = preferences.get(&Self::FUNCTION_MODEL_BINDING()) else {
            return Ok(HashMap::new());
        };
        if bindingJson.is_empty() {
            return Ok(HashMap::new());
        }

        let rawMap: HashMap<String, FunctionModelBinding> = serde_json::from_str(bindingJson)?;
        let mut binding = HashMap::new();
        for (key, value) in rawMap {
            let functionType = Self::parseFunctionType(&key)
                .map_err(|error| PreferencesDataStoreError::Message(error.to_string()))?;
            binding.insert(functionType, value);
        }
        Ok(binding)
    }

    pub fn saveFunctionModelBinding(
        &self,
        binding: HashMap<FunctionType, FunctionModelBinding>,
    ) -> Result<(), FunctionalConfigError> {
        let stringBinding: HashMap<String, FunctionModelBinding> = binding
            .into_iter()
            .map(|(functionType, value)| (Self::functionTypeName(functionType).to_string(), value))
            .collect();
        let encoded = serde_json::to_string(&stringBinding)?;
        self.functionalConfigDataStore.edit(|preferences| {
            preferences.set(&Self::FUNCTION_MODEL_BINDING(), encoded);
        })?;
        Ok(())
    }

    pub fn getModelBindingForFunction(
        &self,
        functionType: FunctionType,
    ) -> Result<FunctionModelBinding, FunctionalConfigError> {
        let binding = self.functionModelBindingFlow()?.first()?;
        binding.get(&functionType).cloned().ok_or_else(|| {
            FunctionalConfigError::ModelConfigManager(format!(
                "missing model binding: {}",
                Self::functionTypeName(functionType)
            ))
        })
    }

    pub fn setModelForFunction(
        &self,
        functionType: FunctionType,
        providerId: String,
        modelId: String,
    ) -> Result<(), FunctionalConfigError> {
        self.modelConfigManager
            .getModelProfile(&providerId, &modelId)
            .map_err(|error| FunctionalConfigError::ModelConfigManager(error.to_string()))?;
        let mut binding = self.functionModelBindingFlow()?.first()?;
        binding.insert(functionType, FunctionModelBinding::new(providerId, modelId));
        self.saveFunctionModelBinding(binding)
    }

    pub fn resetFunctionConfig(
        &self,
        functionType: FunctionType,
    ) -> Result<(), FunctionalConfigError> {
        self.setModelForFunction(
            functionType,
            ModelConfigManager::DEFAULT_PROVIDER_ID.to_string(),
            ModelConfigManager::DEFAULT_MODEL_ID.to_string(),
        )
    }

    pub fn resetAllFunctionConfigs(&self) -> Result<(), FunctionalConfigError> {
        self.saveFunctionModelBinding(Self::defaultBinding())
    }

    fn defaultBinding() -> HashMap<FunctionType, FunctionModelBinding> {
        Self::functionTypeValues()
            .into_iter()
            .map(|functionType| {
                (
                    functionType,
                    FunctionModelBinding::new(
                        ModelConfigManager::DEFAULT_PROVIDER_ID.to_string(),
                        ModelConfigManager::DEFAULT_MODEL_ID.to_string(),
                    ),
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
            _ => Err(FunctionalConfigError::UnknownFunctionType(
                value.to_string(),
            )),
        }
    }
}
