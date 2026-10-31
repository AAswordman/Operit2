use std::collections::{BTreeMap, HashMap};
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
    /// Opens this owner's versioned store through the supplied runtime storage host.
    pub(crate) fn preferenceStore(
        storage: std::sync::Arc<dyn operit_host_api::RuntimeStorageHost>,
        path: impl Into<String>,
    ) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(storage, path)
            .withSchema(Self::PREFERENCES_VERSION, Self::migratePreferences)
    }

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
            functionalConfigDataStore: Self::preferenceStore(
                operit_store::RuntimeStorageHost::defaultRuntimeStorageHost(),
                operit_store::RuntimeStorageHost::runtimeStoragePath(&path),
            ),
            modelConfigManager: ModelConfigManager::new(root_dir),
        }
    }

    /// Creates a manager using the runtime data directory from API preferences.
    pub fn default() -> Self {
        Self::new(ApiPreferences::data_dir())
    }

    /// Observes persisted bindings after initializing missing functions and their default model.
    pub fn functionModelBindingFlow(
        &self,
    ) -> Result<Flow<HashMap<FunctionType, FunctionModelBinding>>, FunctionalConfigError> {
        let manager = self.clone();
        Ok(self
            .functionalConfigDataStore
            .dataFlow()
            .mapResult(move |preferences| manager.readOrInitializeFunctionModelBinding(&preferences)))
    }

    /// Reads only bindings present in the persisted snapshot.
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

    /// Commits the backing default model before publishing any binding that refers to it.
    fn readOrInitializeFunctionModelBinding(
        &self,
        preferences: &Preferences,
    ) -> Result<HashMap<FunctionType, FunctionModelBinding>, PreferencesDataStoreError> {
        let binding = Self::readFunctionModelBinding(preferences)?;
        let missingFunctions = Self::functionTypeValues()
            .iter()
            .any(|function| !binding.contains_key(function));
        let usesDefaultModel = binding.values().any(|value| {
            value.providerId == ModelConfigManager::DEFAULT_PROVIDER_ID
                && value.modelId == ModelConfigManager::DEFAULT_MODEL_ID
        });
        if missingFunctions || usesDefaultModel {
            self.modelConfigManager
                .ensureDefaultModelConfig()
                .map_err(|error| PreferencesDataStoreError::Message(error.to_string()))?;
        }
        if !missingFunctions {
            return Ok(binding);
        }

        self.functionalConfigDataStore.try_edit_result(|preferences| {
            // Re-read under the store lock so a concurrent user edit keeps precedence.
            let mut binding = Self::readFunctionModelBinding(preferences)?;
            for (function, default) in Self::defaultBinding() {
                binding.entry(function).or_insert(default);
            }
            Self::normalizeBinding(&mut binding);
            Self::writeFunctionModelBinding(preferences, binding.clone())?;
            Ok(binding)
        })
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
        fn read(
            preferences: &Preferences,
        ) -> Result<BTreeMap<String, FunctionModelBinding>, PreferencesDataStoreError> {
            match preferences.get(&FunctionalConfigManager::FUNCTION_MODEL_BINDING()) {
                Some(serde_json::Value::String(encoded)) => Ok(serde_json::from_str(encoded)?),
                Some(value) => Ok(serde_json::from_value(value.clone())?),
                None => Ok(BTreeMap::new()),
            }
        }
        fn write(
            preferences: &mut Preferences,
            binding: BTreeMap<String, FunctionModelBinding>,
        ) -> Result<(), PreferencesDataStoreError> {
            let key = FunctionalConfigManager::FUNCTION_MODEL_BINDING();
            let value = match preferences.get(&key) {
                Some(value) if !value.is_string() => serde_json::to_value(binding)?,
                _ => serde_json::Value::String(serde_json::to_string(&binding)?),
            };
            preferences.set(&key, value);
            Ok(())
        }
        match version {
            0 => {
                let mut binding = read(preferences)?;
                if binding.is_empty() {
                    binding = Self::defaultBinding()
                        .into_iter()
                        .map(|(function, binding)| {
                            (Self::functionTypeName(function).to_owned(), binding)
                        })
                        .collect();
                }
                write(preferences, binding)
            }
            1 => {
                let mut binding = read(preferences)?;
                if !preferences.contains(&Self::FUNCTION_MODEL_BINDING()) {
                    return Ok(());
                }
                binding
                    .entry(Self::functionTypeName(FunctionType::TITLE_GENERATION).to_owned())
                    .or_default();
                write(preferences, binding)
            }
            2 => {
                if preferences.contains(&Self::FUNCTION_MODEL_BINDING()) {
                    let binding = read(preferences)?;
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

    #[test]
    fn migration_encoding_is_deterministic_and_keeps_custom_title_binding() {
        use operit_store::PreferencesDataStore::emptyPreferences;
        let key = FunctionalConfigManager::FUNCTION_MODEL_BINDING();
        for version in [0, 1] {
            let mut source = emptyPreferences();
            if version == 1 {
                source.set(
                    &key,
                    serde_json::json!({"CHAT":binding("p","chat",false),
                    "TITLE_GENERATION":binding("p","title",false)})
                    .to_string(),
                );
            }
            let mut expected = source.clone();
            FunctionalConfigManager::migratePreferences(version, &mut expected).unwrap();
            for _ in 0..32 {
                let mut actual = source.clone();
                FunctionalConfigManager::migratePreferences(version, &mut actual).unwrap();
                assert_eq!(actual, expected);
            }
            if version == 1 {
                FunctionalConfigManager::migratePreferences(2, &mut expected).unwrap();
                assert_eq!(
                    expected.get(&key).unwrap()["TITLE_GENERATION"]["modelId"],
                    "title"
                );
            }
        }
    }

    struct ConfigFixture {
        root: std::path::PathBuf,
        storage: std::sync::Arc<dyn operit_host_api::RuntimeStorageHost>,
        manager: FunctionalConfigManager,
    }

    impl ConfigFixture {
        fn new() -> Self {
            use operit_host_native_storage::NativeRuntimeStorageHost;
            use operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost;
            use operit_util::RuntimeStoreRoot::{
                setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig,
            };

            let root = std::env::temp_dir()
                .join(format!("operit-default-config-{}", uuid::Uuid::new_v4()));
            let runtime = root.join("runtime");
            let workspaces = root.join("workspaces");
            let storage: std::sync::Arc<dyn operit_host_api::RuntimeStorageHost> =
                std::sync::Arc::new(NativeRuntimeStorageHost::new(
                    runtime.clone(),
                    workspaces.clone(),
                ));
            setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(
                runtime.clone(),
                workspaces,
            ));
            setDefaultRuntimeStorageHost(storage.clone());
            let manager = FunctionalConfigManager::new(runtime);
            Self {
                root,
                storage,
                manager,
            }
        }

        fn modelStore(&self) -> operit_store::PreferencesDataStore::PreferencesDataStore {
            super::ModelConfigManager::preferenceStore(
                self.storage.clone(),
                operit_util::RuntimeStorageLayout::MODEL_CONFIGS_PREFERENCES_PATH,
            )
        }
    }

    impl Drop for ConfigFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn missing_bindings_persist_default_provider_model_and_editable_bindings() {
        use super::ModelConfigManager;
        use operit_model::ModelConfigData::ProviderProfile;
        use operit_store::PreferencesDataStore::stringPreferencesKey;

        for empty_map in [false, true] {
            let fixture = ConfigFixture::new();
            let manager = &fixture.manager;
            manager
                .functionalConfigDataStore
                .edit(|preferences| {
                    let key = FunctionalConfigManager::FUNCTION_MODEL_BINDING();
                    if empty_map {
                        preferences.set(&key, serde_json::json!({}));
                    } else {
                        preferences.remove(&key);
                    }
                })
                .unwrap();
            manager
                .modelConfigManager
                .deleteProvider(ModelConfigManager::DEFAULT_PROVIDER_ID)
                .unwrap();
            let store = fixture.modelStore();
            let key = stringPreferencesKey(&format!(
                "provider_{}",
                ModelConfigManager::DEFAULT_PROVIDER_ID
            ));
            assert!(!store.data().unwrap().contains(&key));

            let chat = manager
                .getModelBindingForFunction(FunctionType::CHAT)
                .unwrap();
            let persisted = manager.functionalConfigDataStore.data().unwrap();
            assert_eq!(
                FunctionalConfigManager::readFunctionModelBinding(&persisted).unwrap(),
                FunctionalConfigManager::defaultBinding(),
            );
            let mut provider: ProviderProfile =
                serde_json::from_value(store.data().unwrap().get(&key).unwrap().clone()).unwrap();
            assert!(provider.models.iter().any(|model| model.id == chat.modelId));
            let model_manager = &manager.modelConfigManager;
            model_manager
                .getResolvedModelConfig(&chat.providerId, &chat.modelId)
                .unwrap();
            model_manager
                .getThinkingSettingsForProvider(&chat.providerId, &chat.modelId)
                .unwrap();
            assert!(model_manager
                .getProviderProfiles()
                .unwrap()
                .iter()
                .any(|value| value.id == chat.providerId));

            provider.apiKey = "test-persisted-key".into();
            model_manager.updateProviderProfile(provider).unwrap();
            let before = manager
                .functionalConfigDataStore
                .syncSnapshot()
                .unwrap()
                .token()
                .unwrap();
            let model_before = store.syncSnapshot().unwrap().token().unwrap();
            // Open fresh storage hosts so these reads cannot use either manager's cache.
            let cold_storage: std::sync::Arc<dyn operit_host_api::RuntimeStorageHost> =
                std::sync::Arc::new(operit_host_native_storage::NativeRuntimeStorageHost::new(
                    fixture.root.join("runtime"),
                    fixture.root.join("workspaces"),
                ));
            let cold_bindings = FunctionalConfigManager::preferenceStore(
                cold_storage.clone(),
                operit_util::RuntimeStorageLayout::FUNCTIONAL_CONFIGS_PREFERENCES_PATH,
            )
            .data()
            .unwrap();
            assert_eq!(cold_bindings, persisted);
            let cold_models = ModelConfigManager::preferenceStore(
                cold_storage,
                operit_util::RuntimeStorageLayout::MODEL_CONFIGS_PREFERENCES_PATH,
            )
            .data()
            .unwrap();
            assert_eq!(cold_models.get(&key).unwrap()["apiKey"], "test-persisted-key");
            let reopened = FunctionalConfigManager::new(fixture.root.join("runtime"));
            assert_eq!(
                reopened
                    .getModelBindingForFunction(FunctionType::CHAT)
                    .unwrap(),
                chat
            );
            assert_eq!(
                reopened
                    .modelConfigManager
                    .getResolvedModelConfig(&chat.providerId, &chat.modelId)
                    .unwrap()
                    .apiKey,
                "test-persisted-key",
            );
            assert_eq!(
                manager
                    .functionalConfigDataStore
                    .syncSnapshot()
                    .unwrap()
                    .token()
                    .unwrap(),
                before
            );
            assert_eq!(store.syncSnapshot().unwrap().token().unwrap(), model_before);
        }
    }

    #[test]
    fn partial_bindings_are_completed_in_storage_without_replacing_user_choices() {
        let fixture = ConfigFixture::new();
        let manager = &fixture.manager;
        manager
            .functionalConfigDataStore
            .edit(|preferences| {
                preferences.set(
                    &FunctionalConfigManager::FUNCTION_MODEL_BINDING(),
                    serde_json::json!({
                        "CHAT": binding("custom-provider", "custom-chat", false),
                        "SUMMARY": binding("custom-provider", "custom-chat", true),
                        "TRANSLATION": binding("translation-provider", "translation-model", false),
                    }),
                );
            })
            .unwrap();

        let bindings = manager.functionModelBindingFlow().unwrap().first().unwrap();
        let persisted = manager.functionalConfigDataStore.data().unwrap();
        assert_eq!(
            FunctionalConfigManager::readFunctionModelBinding(&persisted).unwrap(),
            bindings
        );
        assert_eq!(
            bindings.len(),
            FunctionalConfigManager::functionTypeValues().len()
        );
        assert_eq!(
            bindings[&FunctionType::CHAT],
            binding("custom-provider", "custom-chat", false)
        );
        assert_eq!(
            bindings[&FunctionType::TRANSLATION],
            binding("translation-provider", "translation-model", false)
        );
        assert!(bindings[&FunctionType::SUMMARY].followsChat);
        assert_eq!(
            manager
                .getModelBindingForFunction(FunctionType::SUMMARY)
                .unwrap(),
            bindings[&FunctionType::CHAT]
        );
        assert_eq!(
            bindings[&FunctionType::MEMORY],
            FunctionModelBinding::default()
        );
    }

    #[test]
    fn missing_default_model_is_persisted_without_resetting_provider_settings() {
        use super::ModelConfigManager;

        let fixture = ConfigFixture::new();
        let manager = &fixture.manager;
        let model_manager = &manager.modelConfigManager;
        let mut provider = model_manager
            .getProviderProfile(ModelConfigManager::DEFAULT_PROVIDER_ID)
            .unwrap();
        provider.apiKey = "test-existing-key".into();
        provider.endpoint = "https://example.test/v1/chat/completions".into();
        model_manager.updateProviderProfile(provider).unwrap();
        model_manager
            .deleteModel(
                ModelConfigManager::DEFAULT_PROVIDER_ID,
                ModelConfigManager::DEFAULT_MODEL_ID,
            )
            .unwrap();
        assert!(model_manager
            .getModelProfile(
                ModelConfigManager::DEFAULT_PROVIDER_ID,
                ModelConfigManager::DEFAULT_MODEL_ID
            )
            .is_err());

        let flow = manager.functionModelBindingFlow().unwrap();
        let bindings = flow.first().unwrap();
        let chat = &bindings[&FunctionType::CHAT];
        let config = model_manager
            .getResolvedModelConfig(&chat.providerId, &chat.modelId)
            .unwrap();
        assert_eq!(config.apiKey, "test-existing-key");
        assert_eq!(
            config.apiEndpoint,
            "https://example.test/v1/chat/completions"
        );

        // An existing flow also repairs a later deleted binding on its next read.
        manager
            .functionalConfigDataStore
            .edit(|preferences| {
                preferences.remove(&FunctionalConfigManager::FUNCTION_MODEL_BINDING());
            })
            .unwrap();
        assert_eq!(flow.first().unwrap(), bindings);
        assert!(manager
            .functionalConfigDataStore
            .data()
            .unwrap()
            .contains(&FunctionalConfigManager::FUNCTION_MODEL_BINDING()));
    }

    #[test]
    fn read_invalid_bindings_still_reports_errors() {
        use operit_store::PreferencesDataStore::emptyPreferences;

        for invalid in [
            serde_json::json!({"CHAT": {"providerId": "p"}}),
            serde_json::json!({"UNKNOWN": binding("p", "m", false)}),
        ] {
            let mut preferences = emptyPreferences();
            preferences.set(&FunctionalConfigManager::FUNCTION_MODEL_BINDING(), invalid);
            assert!(FunctionalConfigManager::readFunctionModelBinding(&preferences).is_err());
        }
    }

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
