use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use operit_model::FunctionType::FunctionType;
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
    /// Creates a model binding for one function using the supplied provider and model.
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
    const PREFERENCES_VERSION: u32 = 2;

    /// Returns the preference key that stores function-to-model bindings.
    pub fn FUNCTION_MODEL_BINDING() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("function_model_binding")
    }

    /// Creates a manager rooted at the directory that owns functional configuration data.
    pub fn new(root_dir: PathBuf) -> Self {
        let path = RuntimeStorePaths::runtime_storage_path_from_root(
            &root_dir,
            operit_util::RuntimeStorageLayout::FUNCTIONAL_CONFIGS_PREFERENCES_PATH,
        );
        Self {
            functionalConfigDataStore: PreferencesDataStore::new(path)
                .withSchema(Self::PREFERENCES_VERSION, Self::migratePreferences),
            modelConfigManager: ModelConfigManager::new(root_dir),
        }
    }

    /// Creates a manager using the runtime data directory from API preferences.
    pub fn default() -> Self {
        Self::new(ApiPreferences::data_dir())
    }

    /// Observes the full mapping from runtime functions to provider model bindings.
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

    /// Saves the complete function-to-model binding map.
    pub fn saveFunctionModelBinding(
        &self,
        binding: HashMap<FunctionType, FunctionModelBinding>,
    ) -> Result<(), FunctionalConfigError> {
        self.functionalConfigDataStore
            .try_edit_result(|preferences| Self::writeFunctionModelBinding(preferences, binding))?;
        Ok(())
    }

    /// Writes the complete function binding map into one preferences snapshot.
    fn writeFunctionModelBinding(
        preferences: &mut Preferences,
        binding: HashMap<FunctionType, FunctionModelBinding>,
    ) -> Result<(), PreferencesDataStoreError> {
        let stringBinding: HashMap<String, FunctionModelBinding> = binding
            .into_iter()
            .map(|(functionType, value)| (Self::functionTypeName(functionType).to_string(), value))
            .collect();
        let encoded = serde_json::to_string(&stringBinding)?;
        preferences.set(&Self::FUNCTION_MODEL_BINDING(), encoded);
        Ok(())
    }

    /// Migrates functional preferences one schema version at a time.
    fn migratePreferences(
        version: u32,
        preferences: &mut Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        match version {
            0 => {
                let binding = Self::readFunctionModelBinding(preferences)?;
                if binding.is_empty() {
                    Self::writeFunctionModelBinding(preferences, Self::defaultBinding())?;
                }
                Ok(())
            }
            1 => {
                let mut binding = Self::readFunctionModelBinding(preferences)?;
                binding.insert(
                    FunctionType::TITLE_GENERATION,
                    FunctionModelBinding::default(),
                );
                Self::writeFunctionModelBinding(preferences, binding)
            }
            from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
        }
    }

    /// Reads the model binding currently assigned to one runtime function.
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

    /// Assigns one runtime function to the specified provider and model.
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

    /// Restores one runtime function to the default provider and model.
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

    /// Restores every runtime function to the default provider and model map.
    pub fn resetAllFunctionConfigs(&self) -> Result<(), FunctionalConfigError> {
        self.saveFunctionModelBinding(Self::defaultBinding())
    }

    /// Builds the complete default binding map for every functional model role.
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

    /// Lists every functional model role persisted in the binding map.
    fn functionTypeValues() -> Vec<FunctionType> {
        vec![
            FunctionType::CHAT,
            FunctionType::SUMMARY,
            FunctionType::TITLE_GENERATION,
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

    /// Serializes a functional model role for preference storage.
    fn functionTypeName(functionType: FunctionType) -> &'static str {
        match functionType {
            FunctionType::CHAT => "CHAT",
            FunctionType::SUMMARY => "SUMMARY",
            FunctionType::TITLE_GENERATION => "TITLE_GENERATION",
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

    /// Parses a persisted functional model role name.
    fn parseFunctionType(value: &str) -> Result<FunctionType, FunctionalConfigError> {
        match value {
            "CHAT" => Ok(FunctionType::CHAT),
            "SUMMARY" => Ok(FunctionType::SUMMARY),
            "TITLE_GENERATION" => Ok(FunctionType::TITLE_GENERATION),
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
