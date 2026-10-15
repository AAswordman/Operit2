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
    /// Marks a binding that dynamically follows the chat model binding.
    #[serde(default)]
    pub followsChat: bool,
}

impl Default for FunctionModelBinding {
    fn default() -> Self {
        Self {
            providerId: ModelConfigManager::DEFAULT_PROVIDER_ID.to_string(),
            modelId: ModelConfigManager::DEFAULT_MODEL_ID.to_string(),
            followsChat: false,
        }
    }
}

impl FunctionModelBinding {
    /// Creates a model binding for one function using the supplied provider and model.
    pub fn new(providerId: String, modelId: String) -> Self {
        Self {
            providerId,
            modelId,
            followsChat: false,
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
    const PREFERENCES_VERSION: u32 = 3;

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

    /// Reads native JSON function bindings using their declared function identities.
    fn readFunctionModelBinding(
        preferences: &Preferences,
    ) -> Result<HashMap<FunctionType, FunctionModelBinding>, PreferencesDataStoreError> {
        let Some(bindingJson) = preferences.get(&Self::FUNCTION_MODEL_BINDING()) else {
            return Ok(HashMap::new());
        };

        let rawMap: HashMap<String, FunctionModelBinding> =
            serde_json::from_value(bindingJson.clone())?;
        let mut binding = HashMap::new();
        for (key, value) in rawMap {
            let functionType = Self::parseFunctionType(&key)
                .map_err(|error| PreferencesDataStoreError::Message(error.to_string()))?;
            binding.insert(functionType, value);
        }
        Ok(binding)
    }

    /// Saves the complete function-to-model binding map after normalizing follow-chat links.
    pub fn saveFunctionModelBinding(
        &self,
        mut binding: HashMap<FunctionType, FunctionModelBinding>,
    ) -> Result<(), FunctionalConfigError> {
        Self::normalizeBinding(&mut binding);
        self.functionalConfigDataStore
            .try_edit_result(|preferences| Self::writeFunctionModelBinding(preferences, binding))?;
        Ok(())
    }

    /// Keeps stored follow-chat bindings consistent with the chat binding.
    fn normalizeBinding(binding: &mut HashMap<FunctionType, FunctionModelBinding>) {
        let Some(chat) = binding.get_mut(&FunctionType::CHAT) else {
            for value in binding.values_mut() {
                value.followsChat = false;
            }
            return;
        };
        chat.followsChat = false;
        let providerId = chat.providerId.clone();
        let modelId = chat.modelId.clone();
        for value in binding.values_mut() {
            if value.followsChat {
                value.providerId = providerId.clone();
                value.modelId = modelId.clone();
            }
        }
    }

    /// Returns the effective binding for one function, resolving follow-chat links.
    fn resolveBinding(
        binding: &HashMap<FunctionType, FunctionModelBinding>,
        functionType: &FunctionType,
    ) -> Option<FunctionModelBinding> {
        let value = binding.get(functionType)?;
        if value.followsChat && *functionType != FunctionType::CHAT {
            let mut chat = binding.get(&FunctionType::CHAT)?.clone();
            chat.followsChat = false;
            return Some(chat);
        }
        Some(value.clone())
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
        let encoded = serde_json::to_value(&stringBinding)?;
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
                let binding = preferences
                    .getTyped::<String>(&Self::FUNCTION_MODEL_BINDING())?
                    .map(|encoded| {
                        serde_json::from_str::<HashMap<String, FunctionModelBinding>>(&encoded)
                    })
                    .transpose()?
                    .unwrap_or_default();
                if binding.is_empty() {
                    let defaults = Self::defaultBinding()
                        .into_iter()
                        .map(|(function, binding)| {
                            (Self::functionTypeName(function).to_owned(), binding)
                        })
                        .collect::<HashMap<_, _>>();
                    preferences.set(
                        &Self::FUNCTION_MODEL_BINDING(),
                        serde_json::to_string(&defaults)?,
                    );
                }
                Ok(())
            }
            1 => {
                let encoded = preferences
                    .getTyped::<String>(&Self::FUNCTION_MODEL_BINDING())?
                    .ok_or_else(|| {
                        PreferencesDataStoreError::Message(
                            "Function bindings missing at schema version 1".into(),
                        )
                    })?;
                let mut binding: HashMap<String, FunctionModelBinding> =
                    serde_json::from_str(&encoded)?;
                binding.insert(
                    Self::functionTypeName(FunctionType::TITLE_GENERATION).to_owned(),
                    FunctionModelBinding::default(),
                );
                preferences.set(
                    &Self::FUNCTION_MODEL_BINDING(),
                    serde_json::to_string(&binding)?,
                );
                Ok(())
            }
            2 => {
                if let Some(value) = preferences.get(&Self::FUNCTION_MODEL_BINDING()) {
                    let encoded = serde_json::from_value::<String>(value.clone())?;
                    let binding: HashMap<String, FunctionModelBinding> =
                        serde_json::from_str(&encoded)?;
                    preferences.set(
                        &Self::FUNCTION_MODEL_BINDING(),
                        serde_json::to_value(binding)?,
                    );
                }
                Ok(())
            }
            from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
        }
    }

    /// Reads the effective model binding for one runtime function.
    pub fn getModelBindingForFunction(
        &self,
        functionType: FunctionType,
    ) -> Result<FunctionModelBinding, FunctionalConfigError> {
        let binding = self.functionModelBindingFlow()?.first()?;
        Self::resolveBinding(&binding, &functionType).ok_or_else(|| {
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

    /// Assigns one runtime function to dynamically follow the chat model binding.
    pub fn setFunctionFollowChat(
        &self,
        functionType: FunctionType,
    ) -> Result<(), FunctionalConfigError> {
        if functionType == FunctionType::CHAT {
            return Err(FunctionalConfigError::ModelConfigManager(
                "chat binding cannot follow itself".to_string(),
            ));
        }
        let mut binding = self.functionModelBindingFlow()?.first()?;
        let chat = binding.get(&FunctionType::CHAT).cloned().ok_or_else(|| {
            FunctionalConfigError::ModelConfigManager("missing model binding: CHAT".to_string())
        })?;
        self.modelConfigManager
            .getModelProfile(&chat.providerId, &chat.modelId)
            .map_err(|error| FunctionalConfigError::ModelConfigManager(error.to_string()))?;
        binding.insert(
            functionType,
            FunctionModelBinding {
                providerId: chat.providerId,
                modelId: chat.modelId,
                followsChat: true,
            },
        );
        self.saveFunctionModelBinding(binding)
    }

    /// Assigns every non-chat runtime function to follow the chat model binding.
    pub fn setAllFunctionsFollowChat(&self) -> Result<(), FunctionalConfigError> {
        let mut binding = self.functionModelBindingFlow()?.first()?;
        let chat = binding.get(&FunctionType::CHAT).cloned().ok_or_else(|| {
            FunctionalConfigError::ModelConfigManager("missing model binding: CHAT".to_string())
        })?;
        self.modelConfigManager
            .getModelProfile(&chat.providerId, &chat.modelId)
            .map_err(|error| FunctionalConfigError::ModelConfigManager(error.to_string()))?;
        for (functionType, value) in binding.iter_mut() {
            if *functionType == FunctionType::CHAT {
                continue;
            }
            value.providerId = chat.providerId.clone();
            value.modelId = chat.modelId.clone();
            value.followsChat = true;
        }
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

#[cfg(test)]
mod tests {
    use super::{FunctionModelBinding, FunctionType, FunctionalConfigManager};
    use std::collections::HashMap;

    fn binding(provider: &str, model: &str, follows_chat: bool) -> FunctionModelBinding {
        FunctionModelBinding {
            providerId: provider.to_string(),
            modelId: model.to_string(),
            followsChat: follows_chat,
        }
    }

    #[test]
    fn normalize_syncs_followers_to_chat() {
        let mut map = HashMap::new();
        map.insert(
            FunctionType::CHAT,
            binding("chat-provider", "chat-model", true),
        );
        map.insert(
            FunctionType::SUMMARY,
            binding("old-provider", "old-model", true),
        );
        map.insert(
            FunctionType::TRANSLATION,
            binding("explicit-provider", "explicit-model", false),
        );

        FunctionalConfigManager::normalizeBinding(&mut map);

        assert!(!map[&FunctionType::CHAT].followsChat);
        assert_eq!(map[&FunctionType::SUMMARY].providerId, "chat-provider");
        assert_eq!(map[&FunctionType::SUMMARY].modelId, "chat-model");
        assert!(map[&FunctionType::SUMMARY].followsChat);
        assert_eq!(
            map[&FunctionType::TRANSLATION].providerId,
            "explicit-provider"
        );
        assert_eq!(map[&FunctionType::TRANSLATION].modelId, "explicit-model");
    }

    #[test]
    fn normalize_strips_followers_without_chat() {
        let mut map = HashMap::new();
        map.insert(
            FunctionType::SUMMARY,
            binding("old-provider", "old-model", true),
        );

        FunctionalConfigManager::normalizeBinding(&mut map);

        assert!(!map[&FunctionType::SUMMARY].followsChat);
    }

    #[test]
    fn resolve_returns_chat_binding_for_followers() {
        let mut map = HashMap::new();
        map.insert(
            FunctionType::CHAT,
            binding("chat-provider", "chat-model", false),
        );
        map.insert(
            FunctionType::SUMMARY,
            binding("chat-provider", "chat-model", true),
        );

        let resolved = FunctionalConfigManager::resolveBinding(&map, &FunctionType::SUMMARY)
            .expect("resolved summary");

        assert_eq!(resolved.providerId, "chat-provider");
        assert_eq!(resolved.modelId, "chat-model");
        assert!(!resolved.followsChat);
    }

    #[test]
    fn resolve_keeps_explicit_bindings() {
        let mut map = HashMap::new();
        map.insert(
            FunctionType::CHAT,
            binding("chat-provider", "chat-model", false),
        );
        map.insert(
            FunctionType::SUMMARY,
            binding("summary-provider", "summary-model", false),
        );

        let resolved = FunctionalConfigManager::resolveBinding(&map, &FunctionType::SUMMARY)
            .expect("resolved summary");

        assert_eq!(resolved.providerId, "summary-provider");
        assert_eq!(resolved.modelId, "summary-model");
    }
}
