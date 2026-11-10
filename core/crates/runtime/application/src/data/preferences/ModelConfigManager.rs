
use std::path::PathBuf;

use std::collections::HashSet;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::ModelSpecResolver::{ModelSpecResolver, ResolvedThinking};
use crate::services::ModelCatalogService;
use operit_local_models::LocalEngineManifest::LocalPlatformTarget;
use operit_local_models::LocalModelManifest::LocalModelKind;
use operit_local_models::LocalModelRegistryStore::LocalModelRegistryStore;
use operit_model::ModelCatalog::ModelCatalog;
use operit_model::ModelConfigData::{
    default_deepseek_model, default_deepseek_provider, ApiProviderType,
    AvailableProviderModel, AvailableProviderModelSource, ModelCapabilities,
    ModelCapabilityOverrides, ModelCatalogStatus, ModelConfigDefaults, ModelContextSpec,
    ModelProfile, ModelRequestSpec, ModelSummarySettings, ProviderModelSummary, ProviderProfile,
    ResolvedModelConfig,
};
use operit_model::ModelParameter::ModelParameter;
use operit_model::ModelThinking::{
    ModelThinkingSettings, ModelThinkingSpec, ThinkingLevel, ThinkingLevelOption,
};
use operit_providers::chat::llmprovider::ModelConfigConnectionTester::{
    ModelConfigConnectionTester, ModelConnectionTestReport,
};
use operit_providers::chat::llmprovider::ModelListFetcher::ModelListFetcher;
use operit_providers::chat::llmprovider::ThinkingDialect::ThinkingDialect;
use operit_providers::runtime_support::ProviderRuntimeContext;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, CoreNodeStateStore, Flow, Preferences, PreferencesDataStore,
    PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

/// Error surface for provider and model configuration operations.
#[derive(Debug, Error)]
pub enum ModelConfigError {
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("store error: {0}")]
    Store(#[from] PreferencesDataStoreError),
    #[error("provider not found: {0}")]
    ProviderNotFound(String),
    #[error("provider name already exists: {0}")]
    ProviderNameAlreadyExists(String),
    #[error("model not found: {0}")]
    ModelNotFound(String),
    #[error("model already exists: {providerId}:{modelId}")]
    ModelAlreadyExists { providerId: String, modelId: String },
    #[error("missing model context: {0}")]
    MissingModelContext(String),
    #[error("missing model capabilities: {0}")]
    MissingModelCapabilities(String),
    #[error("missing model request spec: {0}")]
    MissingModelRequestSpec(String),
    #[error("invalid provider type: {0}")]
    InvalidProviderType(String),
    #[error("model list fetch error: {0}")]
    ModelListFetch(String),
    #[error("model capability catalog error: {0}")]
    ModelCatalog(String),
    #[error("connection test error: {0}")]
    ConnectionTest(String),
    #[error("invalid thinking level: {0}")]
    InvalidThinkingLevel(String),
}

/// Reports the result of importing model configuration backup data.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[allow(non_snake_case)]
pub struct ModelConfigImportResult {
    pub new: i32,
    pub updated: i32,
    pub skipped: i32,
    pub total: i32,
}

/// The last completed connection check, including errors that prevented it from running.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[allow(non_snake_case)]
pub struct ModelConnectionTestResult {
    pub providerId: String,
    pub modelId: String,
    pub report: Option<ModelConnectionTestReport>,
    pub error: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct StoredModelConnectionTest {
    fingerprint: String,
    result: ModelConnectionTestResult,
}

/// Stores provider profiles, model profiles, and resolved model configuration.
#[derive(Clone)]
pub struct ModelConfigManager {
    runtimeRoot: PathBuf,
    modelConfigDataStore: PreferencesDataStore,
    connectionTestDataStore: CoreNodeStateStore,
}

impl ModelConfigManager {
    /// Opens this owner's versioned store through the supplied runtime storage host.
    pub(crate) fn preferenceStore(
        storage: std::sync::Arc<dyn operit_host_api::RuntimeStorageHost>,
        path: impl Into<String>,
    ) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(storage, path)
            .withMigration(Self::PREFERENCES_VERSION, |from,context| {
                if from == 3 {
                    operit_store::Transaction::Rule::DropKey {key:"provider_list".into()}.apply(context).map_err(PreferencesDataStoreError::Message)?;
                }
                context.map_preferences(from,&Self::migratePreferences)
            })
    }

    const PREFERENCES_VERSION: u32 = 8;
    pub const DEFAULT_PROVIDER_ID: &'static str = ModelConfigDefaults::DEFAULT_PROVIDER_ID;
    pub const DEFAULT_MODEL_ID: &'static str = ModelConfigDefaults::DEFAULT_MODEL_ID;

    /// Returns the preference key storing provider profile order.
    pub fn PROVIDER_LIST_KEY() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("provider_list")
    }

    /// Creates a manager rooted at a runtime data directory.
    pub fn new(root_dir: PathBuf) -> Self {
        let path = RuntimeStorePaths::runtime_storage_path_from_root(
            &root_dir,
            operit_util::RuntimeStorageLayout::MODEL_CONFIGS_PREFERENCES_PATH,
        );
        let modelConfigDataStore = Self::preferenceStore(
            operit_store::RuntimeStorageHost::defaultRuntimeStorageHost(),
            operit_store::RuntimeStorageHost::runtimeStoragePath(&path),
        );
        let connectionTestPath = RuntimeStorePaths::runtime_storage_path_from_root(
            &root_dir,
            operit_util::RuntimeStorageLayout::RUNTIME_MODEL_CONNECTION_TESTS_PATH,
        );
        let connectionTestDataStore = CoreNodeStateStore::newWithStorage(
            operit_store::RuntimeStorageHost::defaultRuntimeStorageHost(),
            operit_store::RuntimeStorageHost::runtimeStoragePath(&connectionTestPath),
        );
        Self {
            runtimeRoot: root_dir,
            modelConfigDataStore,
            connectionTestDataStore,
        }
    }

    /// Creates a manager using the default API data directory.
    pub fn default() -> Self {
        Self::new(ApiPreferences::data_dir())
    }

    /// Persists the default provider and model before a default function binding is used.
    pub(crate) fn ensureDefaultModelConfig(&self) -> Result<(), ModelConfigError> {
        let preferences = self.modelConfigDataStore.data()?;
        match self.readProviderFromPreferences(&preferences, Self::DEFAULT_PROVIDER_ID) {
            Ok(provider)
                if provider
                    .models
                    .iter()
                    .any(|model| model.id == Self::DEFAULT_MODEL_ID) =>
            {
                return Ok(());
            }
            Ok(_) | Err(ModelConfigError::ProviderNotFound(_)) => {}
            Err(error) => return Err(error),
        }
        self.modelConfigDataStore.try_edit_result(|preferences| {
            let mut provider =
                match self.readProviderFromPreferences(preferences, Self::DEFAULT_PROVIDER_ID) {
                    Ok(provider) => provider,
                    Err(ModelConfigError::ProviderNotFound(_)) => default_deepseek_provider(),
                    Err(error) => return Err(error),
                };
            if !provider
                .models
                .iter()
                .any(|model| model.id == Self::DEFAULT_MODEL_ID)
            {
                provider.models.push(default_deepseek_model());
            }
            Self::writeProvider(preferences, &provider)?;
            Ok(())
        })
    }

    /// Observes the ordered provider id list.
    pub fn providerListFlow(&self) -> Result<Flow<Vec<String>>, ModelConfigError> {
        Ok(self
            .modelConfigDataStore
            .dataFlow()
            .mapResult(|preferences| Self::readProviderList(&preferences)))
    }

    /// Reads the ordered provider id list.
    pub fn getProviderIds(&self) -> Result<Vec<String>, ModelConfigError> {
        Ok(self.providerListFlow()?.first()?)
    }

    /// Observes all provider profiles in persisted order.
    pub fn getProviderProfilesFlow(&self) -> Result<Flow<Vec<ProviderProfile>>, ModelConfigError> {
        let manager = self.clone();
        Ok(self.modelConfigDataStore.dataFlow().mapResult(move |_| {
            manager
                .getProviderProfiles()
                .map_err(|error| PreferencesDataStoreError::Message(error.to_string()))
        }))
    }

    /// Reads all provider profiles in persisted order.
    pub fn getProviderProfiles(&self) -> Result<Vec<ProviderProfile>, ModelConfigError> {
        let providerIds = self.getProviderIds()?;
        let mut providers = Vec::new();
        for providerId in &providerIds {
            providers.push(self.getProviderProfile(providerId)?);
        }
        Ok(providers)
    }

    /// Reads one provider profile by id.
    pub fn getProviderProfile(
        &self,
        providerId: &str,
    ) -> Result<ProviderProfile, ModelConfigError> {
        let provider = self.loadProviderFromDataStore(providerId)?;
        self.hydrateLocalProviderModels(provider)
    }

    /// Builds summary rows for every configured provider model.
    pub fn getAllModelSummaries(&self) -> Result<Vec<ProviderModelSummary>, ModelConfigError> {
        let providers = self.getProviderProfiles()?;
        let mut summaries = Vec::new();
        for provider in providers {
            if provider.models.is_empty() {
                continue;
            }
            let resolver = ModelSpecResolver::new(&provider)?;
            for model in &provider.models {
                let resolved = resolver.resolve(model);
                summaries.push(ProviderModelSummary {
                    providerId: provider.id.clone(),
                    providerName: provider.name.clone(),
                    providerTypeId: provider.providerTypeId.clone(),
                    endpoint: provider.endpoint.clone(),
                    modelId: model.id.clone(),
                    capabilities: resolved.capabilities,
                    context: resolved.context,
                    pricing: resolved.pricing,
                    thinking: resolved.thinking.spec,
                });
            }
        }
        Ok(summaries)
    }

    /// Reads provider catalog entries from the built-in catalog.
    pub fn getProviderCatalogEntries(
        &self,
    ) -> Result<Vec<operit_model::ModelConfigData::ProviderCatalogEntry>, ModelConfigError> {
        ModelCatalog::providers().map_err(ModelConfigError::ModelListFetch)
    }

    /// Creates a new provider profile and stores it in provider order.
    pub fn createProvider(
        &self,
        name: String,
        providerTypeId: String,
        endpoint: String,
    ) -> Result<String, ModelConfigError> {
        let providerType = ApiProviderType::fromProviderTypeId(&providerTypeId)
            .ok_or_else(|| ModelConfigError::InvalidProviderType(providerTypeId.clone()))?;
        let endpoint = if providerType == ApiProviderType::OPENAI_CODEX {
            "https://chatgpt.com/backend-api/codex/responses".to_string()
        } else {
            endpoint
        };
        let providerId = self.createProviderId();
        let provider = ProviderProfile::new(providerId.clone(), name, providerType, endpoint);
        self.modelConfigDataStore.try_edit_result(|preferences| {
            Self::assertProviderNameUniqueInPreferences(preferences, &provider.name, None)?;
            Self::writeProvider(preferences, &provider)?;
            Ok::<(), ModelConfigError>(())
        })?;
        Ok(providerId)
    }

    /// Replaces an existing provider profile after validation.
    pub fn updateProviderProfile(
        &self,
        provider: ProviderProfile,
    ) -> Result<ProviderProfile, ModelConfigError> {
        let mut provider = provider;
        if provider.providerType == ApiProviderType::OPENAI_CODEX {
            provider.endpoint = "https://chatgpt.com/backend-api/codex/responses".to_string();
        }
        self.modelConfigDataStore.try_edit_result(|preferences| {
            self.assertProviderExistsInPreferences(preferences, &provider.id)?;
            Self::assertProviderNameUniqueInPreferences(
                preferences,
                &provider.name,
                Some(&provider.id),
            )?;
            Self::writeProvider(preferences, &provider)?;
            Ok::<(), ModelConfigError>(())
        })?;
        Ok(provider)
    }

    /// Replaces the default provider profile and preserves its id.
    pub fn replaceDefaultProviderProfile(
        &self,
        provider: ProviderProfile,
    ) -> Result<ProviderProfile, ModelConfigError> {
        if provider.id != Self::DEFAULT_PROVIDER_ID {
            return Err(ModelConfigError::ProviderNotFound(provider.id));
        }
        self.modelConfigDataStore.try_edit_result(|preferences| {
            Self::writeProvider(preferences, &provider)?;
            Ok::<(), ModelConfigError>(())
        })?;
        Ok(provider)
    }

    /// Deletes one provider profile and removes it from provider order.
    pub fn deleteProvider(&self, providerId: &str) -> Result<(), ModelConfigError> {
        let providerKey = self.providerKey(providerId);
        self.modelConfigDataStore.try_edit_result(|preferences| {
            self.assertProviderExistsInPreferences(preferences, providerId)?;
            preferences.remove(&providerKey);
            Ok::<(), ModelConfigError>(())
        })?;
        for (key, stored) in self
            .connectionTestDataStore
            .typedEntries::<StoredModelConnectionTest>()?
        {
            if stored.result.providerId == providerId {
                self.connectionTestDataStore.removeEntry(&key)?;
            }
        }
        Ok(())
    }

    /// Creates a model profile under an existing provider from a user-entered model id.
    pub fn createProviderModel(
        &self,
        providerId: &str,
        modelId: String,
    ) -> Result<String, ModelConfigError> {
        let provider = self.getProviderProfile(providerId)?;
        let builtinTools = ModelSpecResolver::new(&provider)?
            .catalogSpec(&modelId)
            .map(|spec| spec.builtinTools);
        self.updateProviderInternalResult(providerId, |mut provider| {
            Self::assertProviderModelDoesNotExist(&provider, &modelId)?;
            let mut model = ModelProfile::new(modelId.clone());
            model.builtinToolsOverride = builtinTools.clone();
            provider.models.push(model);
            Ok(provider)
        })?;
        Ok(modelId)
    }

    /// Lists the provider's live models first, then the other models the catalog lists for it.
    ///
    /// Each model is described as it would be once added. A catalog over a day old is
    /// downloaded again first; when that download fails the copy in use describes the models
    /// and `getModelCatalogStatus` reports the failure. A missing catalog must finish its
    /// first download successfully before any models are returned.
    pub fn getAvailableProviderModels(
        &self,
        providerId: &str,
    ) -> Result<Vec<AvailableProviderModel>, ModelConfigError> {
        let provider = self.getProviderProfile(providerId)?;
        if provider.providerType == ApiProviderType::LOCAL_MODEL {
            let resolver = ModelSpecResolver::new(&provider)?;
            return Ok(provider
                .models
                .iter()
                .map(|model| resolver.configured(model, AvailableProviderModelSource::Local))
                .collect());
        }
        ModelCatalogService::requireCatalog().map_err(ModelConfigError::ModelCatalog)?;
        ModelCatalogService::refreshCatalogIfStale();
        let resolver = ModelSpecResolver::new(&provider)?;
        let mut models: Vec<AvailableProviderModel> = Vec::new();
        let mut seenModelIds = HashSet::new();
        if provider.providerType != ApiProviderType::OPENAI_CODEX {
            let providerCatalog = ModelCatalog::provider(&provider.providerTypeId)
                .map_err(ModelConfigError::ModelListFetch)?;
            let remoteModels = ModelListFetcher::fetch(&provider, &providerCatalog)
                .map_err(ModelConfigError::ModelListFetch)?;
            for remoteModel in remoteModels {
                if seenModelIds.insert(remoteModel.modelId.to_ascii_lowercase()) {
                    let listed = ModelSpecResolver::listedSpec(&remoteModel);
                    models.push(resolver.available(
                        remoteModel.modelId,
                        AvailableProviderModelSource::Remote,
                        listed,
                    ));
                }
            }
        }
        // The Codex plan serves no model list; its fixed list is what it serves.
        let catalogSource = if provider.providerType == ApiProviderType::OPENAI_CODEX {
            AvailableProviderModelSource::Remote
        } else {
            AvailableProviderModelSource::Catalog
        };
        for modelId in resolver.catalogModelIds() {
            if seenModelIds.insert(modelId.to_ascii_lowercase()) {
                models.push(resolver.available(modelId, catalogSource.clone(), None));
            }
        }
        Ok(models)
    }

    /// Adds a model chosen from `getAvailableProviderModels`, keeping what the provider's
    /// model list said of it. Its settings follow the model catalog until the user sets them.
    pub fn addProviderModelFromAvailable(
        &self,
        providerId: &str,
        model: AvailableProviderModel,
    ) -> Result<String, ModelConfigError> {
        let modelId = model.modelId.clone();
        let provider = self.getProviderProfile(providerId)?;
        ModelSpecResolver::new(&provider)?;
        self.updateProviderInternalResult(providerId, |mut provider| {
            Self::assertProviderModelDoesNotExist(&provider, &modelId)?;
            let mut profile = ModelProfile::new(modelId.clone());
            profile.listed = model.listed.clone();
            profile.builtinToolsOverride = Some(model.builtinTools.clone());
            provider.models.push(profile);
            Ok(provider)
        })?;
        Ok(modelId)
    }

    /// Replaces a model profile under an existing provider.
    pub fn updateModelProfile(
        &self,
        providerId: &str,
        model: ModelProfile,
    ) -> Result<ModelProfile, ModelConfigError> {
        self.updateProviderInternal(providerId, |mut provider| {
            for current in &mut provider.models {
                if current.id == model.id {
                    *current = model.clone();
                }
            }
            provider
        })?;
        Ok(model)
    }

    /// Deletes one model profile from a provider.
    pub fn deleteModel(&self, providerId: &str, modelId: &str) -> Result<(), ModelConfigError> {
        self.updateProviderInternal(providerId, |mut provider| {
            provider.models.retain(|model| model.id != modelId);
            provider
        })?;
        self.connectionTestDataStore
            .removeEntry(&Self::connectionTestKey(providerId, modelId))?;
        Ok(())
    }

    /// Reads one model profile from a provider.
    pub fn getModelProfile(
        &self,
        providerId: &str,
        modelId: &str,
    ) -> Result<ModelProfile, ModelConfigError> {
        let (_, model) = self.findModel(providerId, modelId)?;
        Ok(model)
    }

    /// Resolves provider and model profile data into a runtime model config.
    pub fn getResolvedModelConfig(
        &self,
        providerId: &str,
        modelId: &str,
    ) -> Result<ResolvedModelConfig, ModelConfigError> {
        let (provider, model) = self.findModel(providerId, modelId)?;
        self.resolveFromProfiles(&provider, &model)
    }

    /// Reads model parameters for one provider/model pair.
    pub fn getModelParametersForModel(
        &self,
        providerId: &str,
        modelId: &str,
    ) -> Result<Vec<ModelParameter<serde_json::Value>>, ModelConfigError> {
        let (_, model) = self.findModel(providerId, modelId)?;
        Ok(model.parameters)
    }

    /// Updates model parameters for one provider/model pair.
    pub fn updateParametersForModel(
        &self,
        providerId: &str,
        modelId: &str,
        parameters: Vec<ModelParameter<serde_json::Value>>,
    ) -> Result<(), ModelConfigError> {
        let mut model = self.getModelProfile(providerId, modelId)?;
        model.parameters = parameters;
        self.updateModelProfile(providerId, model)?;
        Ok(())
    }

    /// Updates model capabilities for one provider/model pair. A value equal to what the
    /// model inherits is not kept, so that capability keeps following the model catalog.
    pub fn updateCapabilitiesForModel(
        &self,
        providerId: &str,
        modelId: &str,
        capabilities: ModelCapabilities,
    ) -> Result<ModelProfile, ModelConfigError> {
        let (provider, mut model) = self.findModel(providerId, modelId)?;
        let inherited = ModelSpecResolver::new(&provider)?.inherited(&model);
        model.capabilitiesOverride = ModelSpecResolver::capabilityOverrides(&inherited, &capabilities);
        self.updateModelProfile(providerId, model)
    }

    /// Updates the context window for one provider/model pair. The inherited window is not
    /// kept, so the window keeps following the model catalog.
    pub fn updateContextForModel(
        &self,
        providerId: &str,
        modelId: &str,
        context: ModelContextSpec,
    ) -> Result<ModelProfile, ModelConfigError> {
        let (provider, mut model) = self.findModel(providerId, modelId)?;
        let inherited = ModelSpecResolver::new(&provider)?.inherited(&model);
        model.contextOverride = ModelSpecResolver::contextOverride(&inherited, context);
        self.updateModelProfile(providerId, model)
    }

    /// Updates built-in tool settings for one provider/model pair.
    pub fn updateBuiltinToolsForModel(
        &self,
        providerId: &str,
        modelId: &str,
        builtinTools: Vec<operit_model::ModelConfigData::ModelBuiltinTool>,
    ) -> Result<ModelProfile, ModelConfigError> {
        let mut model = self.getModelProfile(providerId, modelId)?;
        model.builtinToolsOverride = Some(builtinTools);
        self.updateModelProfile(providerId, model)
    }

    /// Updates request settings for one provider/model pair, keeping the values that differ
    /// from what the model inherits.
    pub fn updateRequestForModel(
        &self,
        providerId: &str,
        modelId: &str,
        request: ModelRequestSpec,
    ) -> Result<ModelProfile, ModelConfigError> {
        let (provider, mut model) = self.findModel(providerId, modelId)?;
        let inherited = ModelSpecResolver::new(&provider)?.inherited(&model);
        model.requestOverride = ModelSpecResolver::requestOverrides(&inherited, &request);
        self.updateModelProfile(providerId, model)
    }

    /// Drops the pricing, context, capability, tool-format and thinking-control values the user
    /// set for one model, so it takes the catalog's again.
    pub fn clearModelSpecOverrides(
        &self,
        providerId: &str,
        modelId: &str,
    ) -> Result<ModelProfile, ModelConfigError> {
        let mut model = self.getModelProfile(providerId, modelId)?;
        Self::clearSpecOverrides(&mut model);
        self.updateModelProfile(providerId, model)
    }

    /// Reports when this device last downloaded the model catalog and whether the last
    /// download failed.
    pub fn getModelCatalogStatus(&self) -> Result<ModelCatalogStatus, ModelConfigError> {
        Ok(ModelCatalogService::catalogStatus())
    }

    /// Downloads the model catalog now and reports the result.
    pub fn refreshModelCatalog(&self) -> Result<ModelCatalogStatus, ModelConfigError> {
        Ok(ModelCatalogService::refreshCatalog())
    }

    /// Updates summary settings for one provider/model pair.
    pub fn updateSummaryForModel(
        &self,
        providerId: &str,
        modelId: &str,
        summary: ModelSummarySettings,
    ) -> Result<ModelProfile, ModelConfigError> {
        let mut model = self.getModelProfile(providerId, modelId)?;
        model.summary = summary;
        self.updateModelProfile(providerId, model)
    }

    /// Describes the thinking levels one model offers and the level its requests use.
    pub fn getThinkingSettingsForModel(
        &self,
        providerId: &str,
        modelId: &str,
    ) -> Result<ModelThinkingSettings, ModelConfigError> {
        let (provider, model) = self.findModel(providerId, modelId)?;
        let thinking = Self::resolvedThinking(&provider, &model)?;
        Ok(Self::thinkingSettings(
            &provider,
            &model,
            thinking,
            self.recentThinkingLevel()?,
        ))
    }

    /// Sets the thinking level of one model. Models the user never set one on carry the level
    /// set last over to the nearest level they offer.
    pub fn setThinkingLevelForModel(
        &self,
        providerId: &str,
        modelId: &str,
        level: ThinkingLevel,
    ) -> Result<ModelThinkingSettings, ModelConfigError> {
        // Resolve catalog readiness before entering the preferences transaction.
        let provider = self.getProviderProfile(providerId)?;
        let resolver = ModelSpecResolver::new(&provider)?;
        self.modelConfigDataStore.try_edit_result(|preferences| {
            let provider = self.readProviderFromPreferences(preferences, providerId)?;
            let mut provider = self.hydrateLocalProviderModels(provider)?;
            let index = provider
                .models
                .iter()
                .position(|model| model.id == modelId)
                .ok_or_else(|| {
                    ModelConfigError::ModelNotFound(format!("{providerId}:{modelId}"))
                })?;
            let model = &provider.models[index];
            let thinking = resolver.resolve(model).thinking;
            let offered = Self::thinkingLevels(&provider, model, &thinking.spec);
            if !offered.iter().any(|option| option.level == level) {
                return Err(ModelConfigError::InvalidThinkingLevel(format!(
                    "{providerId}:{modelId} does not take `{}`",
                    level.name()
                )));
            }
            provider.models[index].thinkingLevel = Some(level);
            Self::writeProvider(preferences, &provider)?;
            preferences.set(
                &Self::RECENT_THINKING_LEVEL_KEY(),
                serde_json::to_value(level)?,
            );
            Ok::<(), ModelConfigError>(())
        })?;
        self.getThinkingSettingsForModel(providerId, modelId)
    }

    /// Describes one model's thinking controls in place of what the catalog says, or with
    /// `None` lets the catalog describe them again.
    pub fn setThinkingOverrideForModel(
        &self,
        providerId: &str,
        modelId: &str,
        spec: Option<ModelThinkingSpec>,
    ) -> Result<ModelThinkingSettings, ModelConfigError> {
        let mut model = self.getModelProfile(providerId, modelId)?;
        model.thinkingOverride = spec;
        self.updateModelProfile(providerId, model)?;
        self.getThinkingSettingsForModel(providerId, modelId)
    }

    /// Reads completed checks whose connection settings still match this node's configuration.
    pub fn getModelConnectionTestResults(
        &self,
        providerId: &str,
    ) -> Result<Vec<ModelConnectionTestResult>, ModelConfigError> {
        let provider = self.getProviderProfile(providerId)?;
        let mut results = Vec::new();
        for (_, stored) in self
            .connectionTestDataStore
            .typedEntries::<StoredModelConnectionTest>()?
        {
            if stored.result.providerId != providerId {
                continue;
            }
            let Some(model) = provider
                .models
                .iter()
                .find(|model| model.id == stored.result.modelId)
            else {
                continue;
            };
            if stored.fingerprint == Self::connectionTestFingerprint(&provider, model)? {
                results.push(stored.result);
            }
        }
        Ok(results)
    }

    fn connectionTestKey(providerId: &str, modelId: &str) -> String {
        let identity = format!("{}:{providerId}{modelId}", providerId.len());
        format!("{:x}", Sha256::digest(identity.as_bytes()))
    }

    /// Hash credentials without copying them into the diagnostic record. Key usage
    /// counters and rotation position are deliberately excluded from configuration identity.
    fn connectionTestFingerprint(
        provider: &ProviderProfile,
        model: &ModelProfile,
    ) -> Result<String, ModelConfigError> {
        let keys = provider
            .apiKeyPool
            .iter()
            .map(|key| (&key.key, key.isEnabled))
            .collect::<Vec<_>>();
        let connection = serde_json::json!({
            "providerTypeId": provider.providerTypeId,
            "providerType": provider.providerType,
            "endpoint": provider.endpoint,
            "apiKey": provider.apiKey,
            "useMultipleApiKeys": provider.useMultipleApiKeys,
            "apiKeyPool": keys,
            "keyRotationMode": provider.keyRotationMode,
            "customHeaders": provider.customHeaders,
            "requestLimitPerMinute": provider.requestLimitPerMinute,
            "maxConcurrentRequests": provider.maxConcurrentRequests,
            "modelId": model.id,
            "request": model.requestOverride,
            "parameters": model.parameters,
            "localRuntime": model.localRuntime,
        });
        Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(&connection)?)))
    }

    fn saveConnectionTestResult(
        &self,
        provider: &ProviderProfile,
        model: &ModelProfile,
        outcome: &Result<ModelConnectionTestReport, String>,
    ) -> Result<(), ModelConfigError> {
        let stored = StoredModelConnectionTest {
            fingerprint: Self::connectionTestFingerprint(provider, model)?,
            result: ModelConnectionTestResult {
                providerId: provider.id.clone(),
                modelId: model.id.clone(),
                report: outcome.as_ref().ok().cloned(),
                error: outcome.as_ref().err().cloned(),
            },
        };
        self.connectionTestDataStore.setTypedEntry(
            &Self::connectionTestKey(&provider.id, &model.id),
            &stored,
        )?;
        Ok(())
    }

    /// Tests connectivity and commits the outcome before returning to any client.
    pub async fn testModelConnection(
        &self,
        providerId: &str,
        modelId: &str,
        providerRuntimeContext: ProviderRuntimeContext,
    ) -> Result<ModelConnectionTestReport, ModelConfigError> {
        let (provider, model) = self.findModel(providerId, modelId)?;
        let outcome = ModelConfigConnectionTester::run(
            self.runtimeRoot.clone(),
            providerId,
            modelId,
            providerRuntimeContext,
        )
        .await;
        self.saveConnectionTestResult(&provider, &model, &outcome)?;
        outcome.map_err(ModelConfigError::ConnectionTest)
    }

    /// Exports all provider profiles as formatted JSON.
    pub fn exportAllProviders(&self) -> Result<String, String> {
        serde_json::to_string_pretty(
            &self
                .getProviderProfiles()
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())
    }

    /// Imports provider profiles from the JSON format emitted by exportAllProviders.
    pub fn importAllProvidersFromBackupContent(
        &self,
        jsonContent: &str,
    ) -> Result<ModelConfigImportResult, String> {
        if jsonContent.trim().is_empty() {
            return Err("模型配置备份内容不能为空".to_string());
        }
        let providers = serde_json::from_str::<Vec<ProviderProfile>>(jsonContent)
            .map_err(|error| format!("模型配置备份 JSON 格式错误：{error}"))?;
        if providers.is_empty() {
            return Err("模型配置备份不包含任何配置".to_string());
        }

        let mut importedProviderIds = HashSet::new();
        let mut importedProviderNames = HashSet::new();
        for provider in &providers {
            let providerId = provider.id.trim();
            if providerId.is_empty() {
                return Err("模型配置备份包含缺少 ID 的配置".to_string());
            }
            if !importedProviderIds.insert(providerId) {
                return Err(format!("模型配置备份包含重复 ID：{providerId}"));
            }

            let providerName = provider.name.trim();
            if providerName.is_empty() {
                return Err(format!("模型配置备份中的配置「{providerId}」缺少名称"));
            }
            if !importedProviderNames.insert(providerName) {
                return Err(format!("模型配置备份包含重复名称：{providerName}"));
            }

            if provider.providerTypeId != provider.providerType.name() {
                return Err(format!(
                    "模型配置备份中的配置「{providerName}」类型不一致：{} 与 {}",
                    provider.providerTypeId,
                    provider.providerType.name(),
                ));
            }

            let mut modelIds = HashSet::new();
            for model in &provider.models {
                let modelId = model.id.trim();
                if modelId.is_empty() {
                    return Err(format!(
                        "模型配置备份中的配置「{providerName}」包含缺少 ID 的模型"
                    ));
                }
                if !modelIds.insert(modelId) {
                    return Err(format!(
                        "模型配置备份中的配置「{providerName}」包含重复模型 ID：{modelId}"
                    ));
                }
            }
        }

        self.modelConfigDataStore
            .try_edit_result(|preferences| {
                let providerIds = Self::readProviderList(preferences)?;
                let currentProviderIds = providerIds.iter().cloned().collect::<HashSet<_>>();
                for currentProviderId in &providerIds {
                    if importedProviderIds.contains(currentProviderId.as_str()) {
                        continue;
                    }
                    let currentProvider =
                        self.readProviderFromPreferences(preferences, currentProviderId)?;
                    if importedProviderNames.contains(currentProvider.name.trim()) {
                        return Err(ModelConfigError::ProviderNameAlreadyExists(
                            currentProvider.name.trim().to_string(),
                        ));
                    }
                }

                let mut newCount = 0;
                let mut updatedCount = 0;
                for provider in &providers {
                    if currentProviderIds.contains(&provider.id) {
                        updatedCount += 1;
                    } else {
                        newCount += 1;
                    }
                    Self::writeProvider(preferences, provider)?;
                }

                Ok(ModelConfigImportResult {
                    new: newCount,
                    updated: updatedCount,
                    skipped: 0,
                    total: newCount + updatedCount,
                })
            })
            .map_err(|error: ModelConfigError| format!("模型配置备份恢复失败：{error}"))
    }

    /// Resolves a model profile against its owning provider profile and the model catalog.
    fn resolveFromProfiles(
        &self,
        provider: &ProviderProfile,
        model: &ModelProfile,
    ) -> Result<ResolvedModelConfig, ModelConfigError> {
        let resolved = ModelSpecResolver::new(provider)?.resolve(model);
        let pricing = resolved.pricing;
        let context = resolved.context;
        let capabilities = resolved.capabilities;
        let builtinTools = model.builtinToolsOverride.clone().unwrap_or_default();
        let request = resolved.request;
        let thinkingLevel = Self::currentThinkingLevel(
            &Self::thinkingLevels(provider, model, &resolved.thinking.spec),
            model.thinkingLevel,
            self.recentThinkingLevel()?,
        );

        Ok(ResolvedModelConfig {
            providerId: provider.id.clone(),
            providerName: provider.name.clone(),
            modelId: model.id.clone(),
            apiKey: provider.apiKey.clone(),
            apiEndpoint: provider.endpoint.clone(),
            apiProviderType: provider.providerType.clone(),
            apiProviderTypeId: provider.providerTypeId.clone(),
            useMultipleApiKeys: provider.useMultipleApiKeys,
            apiKeyPool: provider.apiKeyPool.clone(),
            currentKeyIndex: provider.currentKeyIndex,
            keyRotationMode: provider.keyRotationMode.clone(),
            customHeaders: provider.customHeaders.clone(),
            requestLimitPerMinute: provider.requestLimitPerMinute,
            maxConcurrentRequests: provider.maxConcurrentRequests,
            pricing,
            context,
            capabilities,
            builtinTools,
            request,
            parameters: model.parameters.clone(),
            thinking: resolved.thinking.spec,
            thinkingLevel,
            summary: model.summary.clone(),
            localRuntime: model.localRuntime.clone(),
        })
    }

    /// Returns the preference key holding the thinking level the user set last, on any model.
    fn RECENT_THINKING_LEVEL_KEY() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("thinking_recent_level")
    }

    /// Reads the thinking level the user set last, `None` before the first one.
    fn recentThinkingLevel(&self) -> Result<Option<ThinkingLevel>, ModelConfigError> {
        let preferences = self.modelConfigDataStore.data()?;
        preferences
            .get(&Self::RECENT_THINKING_LEVEL_KEY())
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
            .map_err(ModelConfigError::from)
    }

    /// Resolves the thinking controls of a configured model.
    fn resolvedThinking(
        provider: &ProviderProfile,
        model: &ModelProfile,
    ) -> Result<ResolvedThinking, ModelConfigError> {
        Ok(ModelSpecResolver::new(provider)?.resolve(model).thinking)
    }

    /// Returns the levels a model offers through the API its provider speaks.
    fn thinkingLevels(
        provider: &ProviderProfile,
        model: &ModelProfile,
        spec: &ModelThinkingSpec,
    ) -> Vec<ThinkingLevelOption> {
        match ThinkingDialect::for_provider(&provider.providerTypeId, &provider.endpoint, &model.id)
        {
            Some(dialect) => dialect.levels(spec),
            None => Vec::new(),
        }
    }

    /// Picks the level requests use: the level set on the model, else the level set last on
    /// any model, else no thinking, each carried to the nearest level the model offers. The
    /// carry-over is the user's choice (2026-11-04) so a level need not be set model by model.
    fn currentThinkingLevel(
        levels: &[ThinkingLevelOption],
        chosen: Option<ThinkingLevel>,
        recent: Option<ThinkingLevel>,
    ) -> Option<ThinkingLevel> {
        let offered = levels.iter().map(|option| option.level).collect::<Vec<_>>();
        let wanted = match (chosen, recent) {
            (Some(level), _) | (None, Some(level)) => level,
            (None, None) => ThinkingLevel::Off,
        };
        wanted.nearestIn(&offered)
    }

    /// Describes a model's thinking for settings and the composer.
    fn thinkingSettings(
        provider: &ProviderProfile,
        model: &ModelProfile,
        thinking: ResolvedThinking,
        recent: Option<ThinkingLevel>,
    ) -> ModelThinkingSettings {
        let levels = Self::thinkingLevels(provider, model, &thinking.spec);
        let current = Self::currentThinkingLevel(&levels, model.thinkingLevel, recent);
        ModelThinkingSettings {
            spec: thinking.spec,
            source: thinking.source,
            catalog: thinking.catalog,
            levels,
            current,
            chosen: model.thinkingLevel.is_some(),
        }
    }

    /// Drops the spec values the user set; the prompt-cache choice has no catalog value and stays.
    fn clearSpecOverrides(model: &mut ModelProfile) {
        model.pricingOverride = None;
        model.contextOverride = None;
        model.capabilitiesOverride = ModelCapabilityOverrides::default();
        model.requestOverride.supportsStructuredTools = None;
        model.thinkingOverride = None;
    }

    /// Derives provider membership from complete records and validates their identities.
    fn readProviderList(
        preferences: &Preferences,
    ) -> Result<Vec<String>, PreferencesDataStoreError> {
        let mut providers = Vec::new();
        for (key, value) in preferences.iterEntries() {
            let Some(id) = key.strip_prefix("provider_") else {
                continue;
            };
            let provider: ProviderProfile = serde_json::from_value(value.clone())?;
            if provider.id != id {
                return Err(PreferencesDataStoreError::Message(format!(
                    "Provider record identity mismatch: {key}"
                )));
            }
            providers.push(provider);
        }
        providers.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(providers.into_iter().map(|provider| provider.id).collect())
    }

    /// Migrates model preferences one schema version at a time.
    fn migratePreferences(
        version: u32,
        preferences: &mut Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        match version {
            0 => {
                let mut providerIds = preferences
                    .getTyped::<String>(&Self::PROVIDER_LIST_KEY())?
                    .map(|encoded| serde_json::from_str::<Vec<String>>(&encoded))
                    .transpose()?
                    .unwrap_or_default();
                if providerIds.is_empty() {
                    let provider = default_deepseek_provider();
                    preferences.set(
                        &stringPreferencesKey(&format!("provider_{}", provider.id)),
                        serde_json::to_string(&provider)?,
                    );
                    providerIds.push(provider.id);
                }
                preferences.set(
                    &Self::PROVIDER_LIST_KEY(),
                    serde_json::to_string(&providerIds)?,
                );
                Ok(())
            }
            1 => Ok(()),
            // Provider thinking rules lost their provider selectors here; the step from
            // version six drops the rules altogether.
            2 => Ok(()),
            3 => {
                let records = preferences
                    .iterEntries()
                    .filter_map(|(key, value)| {
                        key.strip_prefix("provider_")?;
                        (key != "provider_list").then(|| (key.to_owned(), value.clone()))
                    })
                    .collect::<Vec<_>>();
                for (key, value) in records {
                    let mut record: serde_json::Value = match value {
                        serde_json::Value::String(encoded) => serde_json::from_str(&encoded)?,
                        record => record,
                    };
                    for field in ["providerTypeId", "providerType"] {
                        let removed = record
                            .get(field)
                            .and_then(|value| value.as_str())
                            .map(|value| {
                                Self::REMOVED_PROVIDER_TYPE_IDS
                                    .iter()
                                    .any(|removed| value.trim().eq_ignore_ascii_case(removed))
                            })
                            .unwrap_or(false);
                        if removed {
                            record[field] = serde_json::json!(ApiProviderType::OTHER.name());
                        }
                    }
                    let provider: ProviderProfile = serde_json::from_value(record.clone())?;
                    let id = key
                        .strip_prefix("provider_")
                        .expect("provider record prefix");
                    if provider.id != id {
                        return Err(PreferencesDataStoreError::Message(format!(
                            "Provider record identity mismatch: {key}"
                        )));
                    }
                    preferences.set(&stringPreferencesKey(&key), record);
                }
                preferences.remove(&Self::PROVIDER_LIST_KEY());
                Ok(())
            }
            4 => {
                // iFlow was removed as a provider type. A version-four record naming it
                // fails to deserialize in readProviderList, so rewrite it to OTHER here
                // before any later arm or the provider list reads it.
                let records = preferences
                    .iterEntries()
                    .filter_map(|(key, value)| {
                        (key != "provider_list" && key.starts_with("provider_"))
                            .then(|| (key.to_owned(), value.clone()))
                    })
                    .collect::<Vec<_>>();
                for (key, value) in records {
                    let mut record = value;
                    for field in ["providerTypeId", "providerType"] {
                        let removed = record
                            .get(field)
                            .and_then(|value| value.as_str())
                            .map(|value| {
                                Self::REMOVED_PROVIDER_TYPE_IDS
                                    .iter()
                                    .any(|removed| value.trim().eq_ignore_ascii_case(removed))
                            })
                            .unwrap_or(false);
                        if removed {
                            record[field] = serde_json::json!(ApiProviderType::OTHER.name());
                        }
                    }
                    preferences.set(&stringPreferencesKey(&key), record);
                }
                Ok(())
            }
            5 => {
                // Pricing, context, capability and request values became what the user set,
                // field by field, over the model catalog. Earlier builds copied them from the
                // catalog when a model was added, so they are dropped and every model takes
                // the catalog's again. The Claude prompt-cache choice has no catalog value
                // and is kept.
                let records = preferences
                    .iterEntries()
                    .filter_map(|(key, value)| {
                        (key != "provider_list" && key.starts_with("provider_"))
                            .then(|| (key.to_owned(), value.clone()))
                    })
                    .collect::<Vec<_>>();
                for (key, mut record) in records {
                    if let Some(models) = record
                        .get_mut("models")
                        .and_then(serde_json::Value::as_array_mut)
                    {
                        for model in models.iter_mut().filter_map(serde_json::Value::as_object_mut) {
                            let promptCache = model
                                .get("requestOverride")
                                .and_then(|request| request.get("enableClaude1hPromptCache"))
                                .and_then(serde_json::Value::as_bool)
                                == Some(true);
                            model.insert("pricingOverride".into(), serde_json::Value::Null);
                            model.insert("contextOverride".into(), serde_json::Value::Null);
                            model.insert("capabilitiesOverride".into(), serde_json::json!({}));
                            model.insert(
                                "requestOverride".into(),
                                if promptCache {
                                    serde_json::json!({ "enableClaude1hPromptCache": true })
                                } else {
                                    serde_json::json!({})
                                },
                            );
                        }
                    }
                    preferences.set(&stringPreferencesKey(&key), record);
                }
                Ok(())
            }
            6 => {
                // Thinking became a level set per model over the controls the catalog
                // describes. Earlier builds kept a rule list and a chosen option per provider;
                // both are dropped.
                let records = preferences
                    .iterEntries()
                    .filter_map(|(key, value)| {
                        (key != "provider_list" && key.starts_with("provider_"))
                            .then(|| (key.to_owned(), value.clone()))
                    })
                    .collect::<Vec<_>>();
                for (key, mut record) in records {
                    if let Some(provider) = record.as_object_mut() {
                        provider.remove("thinkingConfigurations");
                        provider.remove("thinkingOptionId");
                    }
                    preferences.set(&stringPreferencesKey(&key), record);
                }
                Ok(())
            }
            7 => {
                // Remove retired message-count controls from every model, preserving
                // each user's summary switch, token threshold and all other settings.
                let records = preferences
                    .iterEntries()
                    .filter_map(|(key, value)| {
                        (key != "provider_list" && key.starts_with("provider_"))
                            .then(|| (key.to_owned(), value.clone()))
                    })
                    .collect::<Vec<_>>();
                for (key, value) in records {
                    let mut record = match value {
                        serde_json::Value::String(encoded) => serde_json::from_str(&encoded)?,
                        record => record,
                    };
                    if let Some(models) = record
                        .get_mut("models")
                        .and_then(serde_json::Value::as_array_mut)
                    {
                        for model in models {
                            if let Some(summary) = model
                                .get_mut("summary")
                                .and_then(serde_json::Value::as_object_mut)
                            {
                                summary.remove("enableSummaryByMessageCount");
                                summary.remove("summaryMessageCountThreshold");
                            }
                        }
                    }
                    preferences.set(&stringPreferencesKey(&key), record);
                }
                Ok(())
            }
            from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
        }
    }

    /// Provider type ids that earlier builds could persist but that no longer exist in
    /// `ApiProviderType`. A provider record naming one fails to deserialize, which makes
    /// `readProviderList` reject the whole store. Migrations that can encounter such a
    /// record rewrite its two provider-type fields to the OpenAI-compatible catch-all
    /// before the provider list is read.
    const REMOVED_PROVIDER_TYPE_IDS: [&'static str; 1] = ["IFLOW"];

    /// Loads and decodes one provider profile from preferences.
    fn loadProviderFromDataStore(
        &self,
        providerId: &str,
    ) -> Result<ProviderProfile, ModelConfigError> {
        let preferences = self.modelConfigDataStore.data()?;
        let providerKey = self.providerKey(providerId);
        let providerJson = preferences
            .get(&providerKey)
            .ok_or_else(|| ModelConfigError::ProviderNotFound(providerId.to_string()))?;
        Ok(serde_json::from_value(providerJson.clone())?)
    }

    /// Saves one provider profile to preferences.
    fn saveProviderToDataStore(&self, provider: &ProviderProfile) -> Result<(), ModelConfigError> {
        self.modelConfigDataStore.try_edit_result(|preferences| {
            Self::writeProvider(preferences, provider)?;
            Ok::<(), ModelConfigError>(())
        })
    }

    fn updateProviderInternal<F>(
        &self,
        providerId: &str,
        transform: F,
    ) -> Result<ProviderProfile, ModelConfigError>
    where
        F: FnOnce(ProviderProfile) -> ProviderProfile,
    {
        self.updateProviderInternalResult(providerId, |provider| Ok(transform(provider)))
    }

    fn updateProviderInternalResult<F>(
        &self,
        providerId: &str,
        transform: F,
    ) -> Result<ProviderProfile, ModelConfigError>
    where
        F: FnOnce(ProviderProfile) -> Result<ProviderProfile, ModelConfigError>,
    {
        self.modelConfigDataStore.try_edit_result(|preferences| {
            let provider = self.readProviderFromPreferences(preferences, providerId)?;
            let provider = self.hydrateLocalProviderModels(provider)?;
            let updated = transform(provider)?;
            Self::writeProvider(preferences, &updated)?;
            Ok(updated)
        })
    }

    fn assertProviderModelDoesNotExist(
        provider: &ProviderProfile,
        modelId: &str,
    ) -> Result<(), ModelConfigError> {
        if provider.models.iter().any(|model| model.id == modelId) {
            Err(ModelConfigError::ModelAlreadyExists {
                providerId: provider.id.clone(),
                modelId: modelId.to_string(),
            })
        } else {
            Ok(())
        }
    }

    fn assertProviderExistsInPreferences(
        &self,
        preferences: &Preferences,
        providerId: &str,
    ) -> Result<(), ModelConfigError> {
        let providerIds = Self::readProviderList(preferences)?;
        if providerIds.iter().any(|id| id == providerId) {
            Ok(())
        } else {
            Err(ModelConfigError::ProviderNotFound(providerId.to_string()))
        }
    }

    /// Reads a complete native JSON provider record from the current snapshot.
    fn readProviderFromPreferences(
        &self,
        preferences: &Preferences,
        providerId: &str,
    ) -> Result<ProviderProfile, ModelConfigError> {
        let providerKey = self.providerKey(providerId);
        let providerJson = preferences
            .get(&providerKey)
            .ok_or_else(|| ModelConfigError::ProviderNotFound(providerId.to_string()))?;
        Ok(serde_json::from_value(providerJson.clone())?)
    }

    /// Writes the complete provider as the synchronized preference value.
    fn writeProvider(
        preferences: &mut Preferences,
        provider: &ProviderProfile,
    ) -> Result<(), PreferencesDataStoreError> {
        let providerKey = stringPreferencesKey(&format!("provider_{}", provider.id));
        let encodedProvider = serde_json::to_value(provider)?;
        preferences.set(&providerKey, encodedProvider);
        Ok(())
    }

    /// Checks provider name uniqueness against all stored records.
    fn assertProviderNameUniqueInPreferences(
        preferences: &Preferences,
        name: &str,
        currentProviderId: Option<&str>,
    ) -> Result<(), ModelConfigError> {
        let normalizedName = name.trim();
        let providerIds = Self::readProviderList(preferences)?;
        for providerId in providerIds {
            if currentProviderId == Some(providerId.as_str()) {
                continue;
            }
            let providerKey = stringPreferencesKey(&format!("provider_{}", providerId));
            let Some(providerJson) = preferences.get(&providerKey) else {
                continue;
            };
            let provider: ProviderProfile = serde_json::from_value(providerJson.clone())?;
            if provider.name.trim() == normalizedName {
                return Err(ModelConfigError::ProviderNameAlreadyExists(
                    normalizedName.to_string(),
                ));
            }
        }
        Ok(())
    }
    fn assertProviderExists(&self, providerId: &str) -> Result<(), ModelConfigError> {
        let providerIds = self.getProviderIds()?;
        if providerIds.iter().any(|id| id == providerId) {
            Ok(())
        } else {
            Err(ModelConfigError::ProviderNotFound(providerId.to_string()))
        }
    }

    /// Replaces a LOCAL_MODEL provider's model rows with the installed asset inventory.
    fn hydrateLocalProviderModels(
        &self,
        mut provider: ProviderProfile,
    ) -> Result<ProviderProfile, ModelConfigError> {
        if provider.providerType != ApiProviderType::LOCAL_MODEL {
            return Ok(provider);
        }
        let registry = LocalModelRegistryStore::forRuntimeStorage(
            operit_store::RuntimeStorageHost::defaultRuntimeStorageHost(),
        )
        .read()
        .map_err(|error| ModelConfigError::ModelListFetch(error.to_string()))?;
        let platform = LocalPlatformTarget::current().map_err(ModelConfigError::ModelListFetch)?;
        if registry
            .installedModels
            .iter()
            .all(|model| model.manifest.kind != LocalModelKind::Chat)
            && provider
                .models
                .iter()
                .all(|model| model.localRuntime.inferenceBindingKey.is_empty())
        {
            provider.models.clear();
            return Ok(provider);
        }
        let storage = operit_store::RuntimeStorageHost::defaultRuntimeStorageHost();
        let identity =
            operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::new(storage.clone())
                .initialize()
                .map_err(ModelConfigError::ModelListFetch)?;
        let bindings =
            operit_store::CoreNodeBindingStore::CoreNodeBindingStore::new(storage.clone())
                .map_err(ModelConfigError::ModelListFetch)?;
        let mut saved = provider
            .models
            .into_iter()
            .map(|model| (model.id.clone(), model))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut models = Vec::new();
        for installed in registry.installedModels.into_iter().filter(|installed| {
            installed.manifest.kind == LocalModelKind::Chat
                && installed.manifest.supportsPlatform(&platform.platform)
        }) {
            let id = installed.manifest.registryKey();
            let mut model = saved
                .remove(&id)
                .unwrap_or_else(|| ModelProfile::new(id.clone()));
            let binding_key = &model.localRuntime.inferenceBindingKey;
            let uses_remote_inference = !binding_key.is_empty()
                && bindings
                    .bindingOptional(binding_key)
                    .map_err(ModelConfigError::ModelListFetch)?
                    .is_some_and(|binding| binding.nodeId != identity.nodeId);
            // Installing matching weights here must preserve an explicitly selected remote owner.
            if !uses_remote_inference {
                model.localRuntime.inferenceBindingKey =
                    crate::services::LocalLlmService::ensureLocalLlmBinding(storage.clone(), &id)
                        .map_err(ModelConfigError::ModelListFetch)?;
            }
            if model.contextOverride.is_none() {
                model.contextOverride = Some(ModelContextSpec {
                    maxContextLength: model.localRuntime.llamaContextSize as f32 / 1000.0,
                });
            }
            if model.capabilitiesOverride.toolCall.is_none() {
                model.capabilitiesOverride.toolCall = Some(true);
            }
            if model.requestOverride.supportsStructuredTools.is_none() {
                model.requestOverride.supportsStructuredTools = Some(false);
            }
            models.push(model);
        }
        // Saved references can use another node's model without mirroring its installation inventory.
        for model in saved.into_values() {
            let key = &model.localRuntime.inferenceBindingKey;
            if !key.is_empty()
                && bindings
                    .bindingOptional(key)
                    .map_err(ModelConfigError::ModelListFetch)?
                    .is_some_and(|binding| binding.nodeId != identity.nodeId)
            {
                models.push(model);
            }
        }
        models.sort_by(|left, right| left.id.cmp(&right.id));
        provider.models = models;
        Ok(provider)
    }

    fn findModel(
        &self,
        providerId: &str,
        modelId: &str,
    ) -> Result<(ProviderProfile, ModelProfile), ModelConfigError> {
        let provider = self.getProviderProfile(providerId)?;
        for model in &provider.models {
            if model.id == modelId {
                return Ok((provider.clone(), model.clone()));
            }
        }
        Err(ModelConfigError::ModelNotFound(format!(
            "{providerId}:{modelId}"
        )))
    }

    fn providerKey(&self, providerId: &str) -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey(&format!("provider_{providerId}"))
    }

    /// Allocates a provider identity independently of device clocks and names.
    fn createProviderId(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{ModelConfigError, ModelConfigManager};
    use operit_host_api::{HostError, HostResult, RuntimeStorageEntry, RuntimeStorageHost};
    use operit_model::ModelConfigData::{ApiProviderType, ModelConfigDefaults};
    use operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost;
    use operit_util::RuntimeStorageLayout::WORKSPACE_DIR_PATH;
    use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};
    use std::fs;
    use std::path::{Component, Path, PathBuf};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn completed_connection_tests_survive_reopen_and_follow_model_lifecycle() {
        use operit_model::ModelConfigData::{ModelProfile, ProviderProfile};
        use operit_providers::chat::llmprovider::ModelConfigConnectionTester::{
            ModelConnectionTestItem, ModelConnectionTestReport, ModelConnectionTestType,
        };

        let root = unique_test_root("connection_results");
        setup_test_runtime(root.clone());
        let manager = ModelConfigManager::new(root.clone());
        let mut provider = ProviderProfile::new(
            "provider-a".into(),
            "Provider A".into(),
            ApiProviderType::OPENAI,
            "https://example.test/v1".into(),
        );
        provider.apiKey = "test-key-not-for-diagnostic-storage".into();
        provider.models = vec![
            ModelProfile::new("model/a".into()),
            ModelProfile::new("model-b".into()),
        ];
        manager.saveProviderToDataStore(&provider).unwrap();
        let mut other = provider.clone();
        other.id = "provider-b".into();
        other.name = "Provider B".into();
        manager.saveProviderToDataStore(&other).unwrap();
        let report = |provider: &ProviderProfile, success| ModelConnectionTestReport {
            providerId: provider.id.clone(),
            providerName: provider.name.clone(),
            providerType: provider.providerTypeId.clone(),
            modelId: provider.models[0].id.clone(),
            success,
            items: vec![ModelConnectionTestItem {
                r#type: ModelConnectionTestType::CHAT,
                success,
                error: if success {
                    None
                } else {
                    Some("HTTP 503".into())
                },
            }],
        };
        let passed = report(&provider, true);
        let failed = report(&other, false);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                manager
                    .saveConnectionTestResult(&provider, &provider.models[0], &Ok(passed.clone()))
                    .unwrap();
            });
            scope.spawn(|| {
                manager
                    .saveConnectionTestResult(
                        &provider,
                        &provider.models[1],
                        &Err("Request could not start".into()),
                    )
                    .unwrap();
            });
            scope.spawn(|| {
                manager
                    .saveConnectionTestResult(&other, &other.models[0], &Ok(failed.clone()))
                    .unwrap();
            });
        });
        let encoded = fs::read_to_string(
            root.join(operit_util::RuntimeStorageLayout::RUNTIME_MODEL_CONNECTION_TESTS_PATH),
        )
        .unwrap();
        assert!(!encoded.contains(&provider.apiKey));
        drop(manager);

        let manager = ModelConfigManager::new(root.clone());
        let results = manager
            .getModelConnectionTestResults(&provider.id)
            .unwrap();
        assert_eq!(results.len(), 2);
        assert!(results
            .iter()
            .any(|result| result.report.as_ref() == Some(&passed)));
        assert!(results.iter().any(|result| result.modelId == "model-b"
            && result.error.as_deref() == Some("Request could not start")));
        assert_eq!(
            manager.getModelConnectionTestResults(&other.id).unwrap()[0]
                .report,
            Some(failed),
        );

        let failed = report(&provider, false);
        manager
            .saveConnectionTestResult(&provider, &provider.models[0], &Ok(failed.clone()))
            .unwrap();
        let results = manager
            .getModelConnectionTestResults(&provider.id)
            .unwrap();
        assert_eq!(results.len(), 2);
        assert!(results
            .iter()
            .any(|result| result.report.as_ref() == Some(&failed)));

        provider.models[0].localRuntime.llamaThreadCount += 1;
        manager.saveProviderToDataStore(&provider).unwrap();
        let results = manager
            .getModelConnectionTestResults(&provider.id)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].modelId, "model-b");
        provider.endpoint = "https://changed.example.test/v1".into();
        manager.saveProviderToDataStore(&provider).unwrap();
        assert!(manager
            .getModelConnectionTestResults(&provider.id)
            .unwrap()
            .is_empty());

        manager.deleteModel(&provider.id, "model-b").unwrap();
        manager
            .createProviderModel(&provider.id, "model-b".into())
            .unwrap();
        assert!(manager
            .getModelConnectionTestResults(&provider.id)
            .unwrap()
            .is_empty());
        manager.deleteProvider(&other.id).unwrap();
        assert!(manager
            .connectionTestDataStore
            .typedEntries::<super::StoredModelConnectionTest>()
            .unwrap()
            .iter()
            .all(|(_, stored)| stored.result.providerId != other.id));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn connection_test_identity_ignores_key_usage_and_applied_capabilities() {
        use operit_model::ApiKeyInfo::{ApiKeyAvailabilityStatus, ApiKeyInfo};
        use operit_model::ModelConfigData::{ModelProfile, ProviderProfile};

        let mut provider = ProviderProfile::new(
            "provider".into(),
            "Provider".into(),
            ApiProviderType::OPENAI,
            "https://example.test/v1".into(),
        );
        provider.apiKeyPool = vec![ApiKeyInfo::new("key-id".into(), "secret".into())];
        let mut model = ModelProfile::new("model".into());
        let original = ModelConfigManager::connectionTestFingerprint(&provider, &model).unwrap();
        provider.name = "Renamed".into();
        provider.currentKeyIndex = 1;
        provider.apiKeyPool[0].usageCount = 5;
        provider.apiKeyPool[0].lastUsed = 123;
        provider.apiKeyPool[0].errorCount = 2;
        provider.apiKeyPool[0].availabilityStatus = ApiKeyAvailabilityStatus::AVAILABLE;
        model.capabilitiesOverride.directImage = Some(true);
        model.summary.enableSummary = false;
        assert_eq!(
            original,
            ModelConfigManager::connectionTestFingerprint(&provider, &model).unwrap(),
        );
        provider.apiKeyPool[0].key = "changed-secret".into();
        assert_ne!(
            original,
            ModelConfigManager::connectionTestFingerprint(&provider, &model).unwrap(),
        );
        provider.apiKeyPool[0].key = "secret".into();
        provider.apiKeyPool[0].isEnabled = false;
        assert_ne!(
            original,
            ModelConfigManager::connectionTestFingerprint(&provider, &model).unwrap(),
        );
        assert_ne!(
            ModelConfigManager::connectionTestKey("a:b", "c"),
            ModelConfigManager::connectionTestKey("a", "b:c"),
        );
    }

    /// Starts a persisted version-three file at its appended migration and preserves unindexed providers.
    #[test]
    fn schema_three_migration_does_not_execute_initialization() {
        use operit_model::ModelConfigData::ProviderProfile;
        use operit_store::PreferencesDataStore::{stringPreferencesKey, CoreNodeStateStore};
        let root = unique_test_root("schema_three_native_json");
        let host: Arc<dyn RuntimeStorageHost> =
            Arc::new(TestRuntimeStorageHost { root: root.clone() });
        let path = "runtime/config/model-schema-three.preferences.json";
        let first = ProviderProfile::new(
            "first".into(),
            "First".into(),
            ApiProviderType::OPENAI,
            "https://example.test".into(),
        );
        let second = ProviderProfile::new(
            "unindexed".into(),
            "Unindexed".into(),
            ApiProviderType::OPENAI,
            "https://example.test".into(),
        );
        let legacy = serde_json::json!({
            "__operit_preferences_schema_version": "3",
            "provider_list": "[\"first\"]",
            "provider_first": serde_json::to_string(&first).unwrap(),
            "provider_unindexed": serde_json::to_string(&second).unwrap()
        });
        host.writeBytes(path, &serde_json::to_vec(&legacy).unwrap())
            .unwrap();
        let expectedMigrationStep = std::sync::atomic::AtomicU32::new(3);
        let store = CoreNodeStateStore::newWithStorage(host, path).withSchema(
            ModelConfigManager::PREFERENCES_VERSION,
            move |version, preferences| {
                assert_eq!(
                    version,
                    expectedMigrationStep.load(std::sync::atomic::Ordering::SeqCst),
                    "an existing file must run its migrations in order"
                );
                expectedMigrationStep.store(version + 1, std::sync::atomic::Ordering::SeqCst);
                ModelConfigManager::migratePreferences(version, preferences)
            },
        );
        let migrated = store.data().unwrap();
        assert_eq!(
            ModelConfigManager::readProviderList(&migrated).unwrap(),
            vec!["first", "unindexed"]
        );
        assert!(migrated
            .get(&stringPreferencesKey("provider_first"))
            .unwrap()
            .is_object());
        assert!(migrated
            .get(&stringPreferencesKey("provider_list"))
            .is_none());
        assert_eq!(
            store.schemaVersion().unwrap(),
            ModelConfigManager::PREFERENCES_VERSION
        );
        assert_eq!(store.data().unwrap(), migrated);
    }

    /// Rewrites a persisted iFlow provider to the OpenAI-compatible catch-all so the list stays readable.
    #[test]
    fn iflow_provider_migrates_to_other_provider_type() {
        use operit_model::ModelConfigData::ProviderProfile;
        use operit_store::PreferencesDataStore::{stringPreferencesKey, CoreNodeStateStore};
        let root = unique_test_root("schema_four_iflow");
        let host: Arc<dyn RuntimeStorageHost> =
            Arc::new(TestRuntimeStorageHost { root: root.clone() });
        let path = "runtime/config/model-schema-four.preferences.json";
        let iflow = ProviderProfile::new(
            "iflow".into(),
            "iFlow".into(),
            ApiProviderType::OPENAI,
            "https://apis.iflow.cn/v1/chat/completions".into(),
        );
        let mut record = serde_json::to_value(&iflow).unwrap();
        record["providerTypeId"] = serde_json::json!("IFLOW");
        record["providerType"] = serde_json::json!("IFLOW");
        let openai = ProviderProfile::new(
            "openai".into(),
            "OpenAI".into(),
            ApiProviderType::OPENAI,
            "https://api.openai.com/v1/chat/completions".into(),
        );
        let legacy = serde_json::json!({
            "__operit_preferences_schema_version": "4",
            "provider_iflow": record,
            "provider_openai": serde_json::to_value(&openai).unwrap()
        });
        host.writeBytes(path, &serde_json::to_vec(&legacy).unwrap())
            .unwrap();
        let expectedMigrationStep = std::sync::atomic::AtomicU32::new(4);
        let store = CoreNodeStateStore::newWithStorage(host, path).withSchema(
            ModelConfigManager::PREFERENCES_VERSION,
            move |version, preferences| {
                assert_eq!(
                    version,
                    expectedMigrationStep.load(std::sync::atomic::Ordering::SeqCst),
                    "an existing version-four file starts at its own arm"
                );
                expectedMigrationStep.store(version + 1, std::sync::atomic::Ordering::SeqCst);
                ModelConfigManager::migratePreferences(version, preferences)
            },
        );
        let migrated = store.data().unwrap();
        let iflowRecord = migrated
            .get(&stringPreferencesKey("provider_iflow"))
            .unwrap();
        assert_eq!(iflowRecord["providerTypeId"], "OTHER");
        assert_eq!(iflowRecord["providerType"], "OTHER");
        assert_eq!(iflowRecord["name"], "iFlow");
        let openaiRecord = migrated
            .get(&stringPreferencesKey("provider_openai"))
            .unwrap();
        assert_eq!(openaiRecord["providerTypeId"], "OPENAI");
        assert_eq!(openaiRecord["providerType"], "OPENAI");
        assert_eq!(
            ModelConfigManager::readProviderList(&migrated).unwrap(),
            vec!["openai", "iflow"]
        );
        assert_eq!(
            store.schemaVersion().unwrap(),
            ModelConfigManager::PREFERENCES_VERSION
        );
    }

    /// Cleans an iFlow record on an older file whose arms still parse full provider profiles.
    #[test]
    fn iflow_provider_migrates_from_schema_three() {
        use operit_model::ModelConfigData::ProviderProfile;
        use operit_store::PreferencesDataStore::{stringPreferencesKey, CoreNodeStateStore};
        let root = unique_test_root("schema_three_iflow");
        let host: Arc<dyn RuntimeStorageHost> =
            Arc::new(TestRuntimeStorageHost { root: root.clone() });
        let path = "runtime/config/model-schema-three-iflow.preferences.json";
        let iflow = ProviderProfile::new(
            "iflow".into(),
            "iFlow".into(),
            ApiProviderType::OPENAI,
            "https://apis.iflow.cn/v1/chat/completions".into(),
        );
        let mut record = serde_json::to_value(&iflow).unwrap();
        record["providerTypeId"] = serde_json::json!("IFLOW");
        record["providerType"] = serde_json::json!("IFLOW");
        let legacy = serde_json::json!({
            "__operit_preferences_schema_version": "3",
            "provider_list": "[\"iflow\"]",
            "provider_iflow": serde_json::to_string(&record).unwrap()
        });
        host.writeBytes(path, &serde_json::to_vec(&legacy).unwrap())
            .unwrap();
        let expectedMigrationStep = std::sync::atomic::AtomicU32::new(3);
        let store = CoreNodeStateStore::newWithStorage(host, path).withSchema(
            ModelConfigManager::PREFERENCES_VERSION,
            move |version, preferences| {
                assert_eq!(
                    version,
                    expectedMigrationStep.load(std::sync::atomic::Ordering::SeqCst),
                    "an older file must run its migrations in order"
                );
                expectedMigrationStep.store(version + 1, std::sync::atomic::Ordering::SeqCst);
                ModelConfigManager::migratePreferences(version, preferences)
            },
        );
        let migrated = store.data().unwrap();
        let iflowRecord = migrated
            .get(&stringPreferencesKey("provider_iflow"))
            .unwrap();
        assert_eq!(iflowRecord["providerTypeId"], "OTHER");
        assert_eq!(iflowRecord["providerType"], "OTHER");
        assert_eq!(
            ModelConfigManager::readProviderList(&migrated).unwrap(),
            vec!["iflow"]
        );
        assert_eq!(
            store.schemaVersion().unwrap(),
            ModelConfigManager::PREFERENCES_VERSION
        );
    }

    #[test]
    fn default_ids_are_model_ids() {
        assert_eq!(
            ModelConfigManager::DEFAULT_MODEL_ID,
            ModelConfigDefaults::DEFAULT_MODEL_ID
        );
    }

    #[test]
    fn local_llm_settings_and_remote_inference_references_survive_hydration() {
        use operit_local_models::{
            LocalModelCatalog::LocalModelCatalog,
            LocalModelRegistry::{InstalledLocalModel, LocalModelRegistrySnapshot},
            LocalModelRegistryStore::LocalModelRegistryStore,
        };
        use operit_model::ModelConfigData::ModelProfile;
        let root = unique_test_root("local_llm_settings");
        setup_test_runtime(root.clone());
        let storage = operit_store::RuntimeStorageHost::defaultRuntimeStorageHost();
        let registry_store = LocalModelRegistryStore::forRuntimeStorage(storage.clone());
        let manifest = LocalModelCatalog::qwen3LlamaCpp();
        let model_id = manifest.registryKey();
        let mut registry = LocalModelRegistrySnapshot::empty();
        registry.installedModels.push(InstalledLocalModel {
            manifest,
            storagePath: "runtime/models/local/chat/qwen".into(),
            installedAtMs: 1,
            verifiedAtMs: Some(1),
        });
        registry_store.write(&registry).unwrap();
        let manager = ModelConfigManager::new(root.clone());
        let provider_id = local_provider(&manager);
        let mut model = manager.getModelProfile(&provider_id, &model_id).unwrap();
        assert!(!model.localRuntime.inferenceBindingKey.is_empty());
        model.localRuntime.llamaThreadCount = 7;
        model.localRuntime.llamaContextSize = 4096;
        manager
            .updateModelProfile(&provider_id, model.clone())
            .unwrap();
        assert_eq!(
            manager
                .getModelProfile(&provider_id, &model_id)
                .unwrap()
                .localRuntime,
            model.localRuntime
        );
        operit_store::CoreNodeBindingStore::CoreNodeBindingStore::new(storage)
            .unwrap()
            .create("remote-llm", "model-device")
            .unwrap();
        let mut provider = manager.getProviderProfile(&provider_id).unwrap();
        let mut remote = ModelProfile::new("remote-model@revision".into());
        remote.localRuntime.inferenceBindingKey = "remote-llm".into();
        provider.models.push(remote.clone());
        manager.saveProviderToDataStore(&provider).unwrap();
        registry.installedModels.clear();
        registry_store.write(&registry).unwrap();
        let models = manager.getProviderProfile(&provider_id).unwrap().models;
        assert_eq!(models, vec![remote]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn local_model_installation_preserves_an_explicit_remote_inference_binding() {
        use operit_local_models::{
            LocalModelCatalog::LocalModelCatalog,
            LocalModelRegistry::{InstalledLocalModel, LocalModelRegistrySnapshot},
            LocalModelRegistryStore::LocalModelRegistryStore,
        };
        let root = unique_test_root("local_remote_model_collision");
        setup_test_runtime(root.clone());
        let storage = operit_store::RuntimeStorageHost::defaultRuntimeStorageHost();
        let registry_store = LocalModelRegistryStore::forRuntimeStorage(storage.clone());
        let manifest = LocalModelCatalog::qwen3LlamaCpp();
        let model_id = manifest.registryKey();
        let mut registry = LocalModelRegistrySnapshot::empty();
        registry.installedModels.push(InstalledLocalModel {
            manifest,
            storagePath: "runtime/models/local/chat/qwen".into(),
            installedAtMs: 1,
            verifiedAtMs: Some(1),
        });
        registry_store.write(&registry).unwrap();
        let manager = ModelConfigManager::new(root.clone());
        let provider_id = local_provider(&manager);
        let local = manager.getModelProfile(&provider_id, &model_id).unwrap();
        assert!(!local.localRuntime.inferenceBindingKey.is_empty());
        operit_store::CoreNodeBindingStore::CoreNodeBindingStore::new(storage)
            .unwrap()
            .create("remote-same-model", "remote-model-device")
            .unwrap();
        let mut remote = local.clone();
        remote.localRuntime.inferenceBindingKey = "remote-same-model".into();
        remote.localRuntime.llamaThreadCount = 7;
        remote.localRuntime.llamaContextSize = 4096;
        manager
            .updateModelProfile(&provider_id, remote.clone())
            .unwrap();

        // A fresh read must preserve the selected device even with identical local weights.
        let reloaded = ModelConfigManager::new(root.clone());
        assert_eq!(
            reloaded.getModelProfile(&provider_id, &model_id).unwrap(),
            remote
        );

        // Removing and reinstalling this node's weights must not change the remote reference.
        registry_store
            .write(&LocalModelRegistrySnapshot::empty())
            .unwrap();
        assert_eq!(
            reloaded.getModelProfile(&provider_id, &model_id).unwrap(),
            remote
        );
        registry_store.write(&registry).unwrap();
        assert_eq!(
            reloaded.getModelProfile(&provider_id, &model_id).unwrap(),
            remote
        );

        // An explicit change back to this node remains possible.
        reloaded
            .updateModelProfile(&provider_id, local.clone())
            .unwrap();
        assert_eq!(
            reloaded.getModelProfile(&provider_id, &model_id).unwrap(),
            local
        );
        fs::remove_dir_all(root).unwrap();
    }

    /// Verifies local provider hydration does not create a nested HTTP runtime.
    #[test]
    fn local_provider_hydration_runs_inside_tokio_runtime() {
        let root = unique_test_root("local_provider_tokio_runtime");
        setup_test_runtime(root.clone());
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("build Tokio runtime");
        runtime.block_on(async {
            let manager = ModelConfigManager::new(root.clone());
            let provider_id = local_provider(&manager);
            let providers = manager.getProviderProfiles().expect("provider profiles");
            assert!(providers.iter().any(|provider| provider.id == provider_id));
        });
        fs::remove_dir_all(root).expect("remove model config test root");
    }

    #[test]
    fn create_local_provider_with_empty_endpoint() {
        let root = unique_test_root("create_local_provider_with_empty_endpoint");
        setup_test_runtime(root.clone());
        let manager = ModelConfigManager::new(root.clone());
        // A fresh install lists no local models until the user adds the provider.
        assert!(manager
            .getProviderProfiles()
            .expect("provider profiles")
            .iter()
            .all(|provider| provider.providerType != ApiProviderType::LOCAL_MODEL));
        let provider_id = manager
            .createProvider(
                "Additional Local Models".to_string(),
                ApiProviderType::LOCAL_MODEL.name().to_string(),
                String::new(),
            )
            .expect("create local provider");

        let provider = manager
            .getProviderProfile(&provider_id)
            .expect("created local provider");
        assert_eq!(provider.providerType, ApiProviderType::LOCAL_MODEL);
        assert!(provider.endpoint.is_empty());
        let ids = manager.getProviderIds().expect("provider ids");
        assert!(ids.contains(&provider_id));

        fs::remove_dir_all(root).expect("remove model config test root");
    }

    #[test]
    fn delete_and_recreate_local_provider() {
        let root = unique_test_root("delete_and_recreate_local_provider");
        setup_test_runtime(root.clone());
        let manager = ModelConfigManager::new(root.clone());
        let original_id = local_provider(&manager);
        manager
            .deleteProvider(&original_id)
            .expect("delete first local provider");
        assert!(matches!(
            manager.getProviderProfile(&original_id),
            Err(ModelConfigError::ProviderNotFound(_))
        ));

        let manager = ModelConfigManager::new(root.clone());
        assert!(!manager
            .getProviderIds()
            .expect("provider ids after reload")
            .contains(&original_id));
        let provider_id = manager
            .createProvider(
                "Local Models".to_string(),
                ApiProviderType::LOCAL_MODEL.name().to_string(),
                String::new(),
            )
            .expect("recreate local provider");
        let manager = ModelConfigManager::new(root.clone());
        let provider = manager
            .getProviderProfile(&provider_id)
            .expect("recreated local provider after reload");
        assert_eq!(provider.name, "Local Models");
        assert_eq!(provider.providerType, ApiProviderType::LOCAL_MODEL);
        manager
            .deleteProvider(&provider_id)
            .expect("delete recreated local provider");
        assert!(!manager
            .getProviderIds()
            .expect("provider ids after deletion")
            .contains(&provider_id));
        assert!(matches!(
            manager.getProviderProfile(&provider_id),
            Err(ModelConfigError::ProviderNotFound(_))
        ));

        fs::remove_dir_all(root).expect("remove model config test root");
    }

    #[test]
    fn provider_model_update_keeps_existing_api_key() {
        let root = unique_test_root("provider_internal_update_keeps_latest_api_key");
        setup_test_runtime(root.clone());
        let manager = ModelConfigManager::new(root.clone());

        let mut provider = manager
            .getProviderProfile(ModelConfigManager::DEFAULT_PROVIDER_ID)
            .expect("default provider");
        provider.apiKey = "sk-latest".to_string();
        manager
            .updateProviderProfile(provider)
            .expect("update api key");

        manager
            .createProviderModel(
                ModelConfigManager::DEFAULT_PROVIDER_ID,
                "provider-model-update-test".to_string(),
            )
            .expect("create provider model");

        let provider = manager
            .getProviderProfile(ModelConfigManager::DEFAULT_PROVIDER_ID)
            .expect("provider after model update");
        assert_eq!(provider.apiKey, "sk-latest");

        let _ = fs::remove_dir_all(root);
    }

    /// Verifies exported provider JSON restores a new provider with its API key.
    #[test]
    fn import_all_providers_from_backup_content_restores_provider() {
        use operit_model::ModelConfigData::{ModelSummarySettings, ProviderProfile};
        let root = unique_test_root("import_all_providers_from_backup_content_restores_provider");
        setup_test_runtime(root.clone());
        let manager = ModelConfigManager::new(root.clone());
        let mut provider = manager
            .getProviderProfile(ModelConfigManager::DEFAULT_PROVIDER_ID)
            .expect("default provider");
        provider.id = "restored_provider".to_string();
        provider.name = "Restored provider".to_string();
        provider.apiKey = "sk-restored".to_string();
        let mut backup = serde_json::to_value(vec![provider]).expect("serialize backup");
        backup[0]["models"][0]["summary"] = serde_json::json!({
            "enableSummary": false, "summaryTokenThreshold": 0.92,
            "enableSummaryByMessageCount": true, "summaryMessageCountThreshold": 1
        });
        let backup = backup.to_string();

        let result = manager
            .importAllProvidersFromBackupContent(&backup)
            .expect("import model configuration backup");

        assert_eq!(result.new, 1);
        assert_eq!(result.updated, 0);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.total, 1);
        assert_eq!(
            manager
                .getProviderProfile("restored_provider")
                .expect("restored provider")
                .apiKey,
            "sk-restored"
        );
        let restored = manager.getProviderProfile("restored_provider").unwrap();
        assert_eq!(
            serde_json::to_value(&restored.models[0].summary).unwrap(),
            serde_json::to_value(ModelSummarySettings {
                enableSummary: false,
                summaryTokenThreshold: 0.92,
            }).unwrap()
        );
        let exported: Vec<ProviderProfile> = serde_json::from_str(
            &manager.exportAllProviders().expect("export imported settings")
        ).unwrap();
        let exported = exported.iter().find(|provider| provider.id == "restored_provider").unwrap();
        assert_eq!(exported.models[0].summary, restored.models[0].summary);

        let _ = fs::remove_dir_all(root);
    }

    /// Verifies provider UUID allocation and rejects an occupied display name.
    #[test]
    fn create_provider_rejects_duplicate_name() {
        let root = unique_test_root("create_provider_rejects_duplicate_name");
        setup_test_runtime(root.clone());
        let manager = ModelConfigManager::new(root.clone());

        let providerId = manager
            .createProvider(
                "Same provider name".to_string(),
                "OPENAI".to_string(),
                "https://api.openai.com/v1".to_string(),
            )
            .expect("first provider with this name");
        assert!(uuid::Uuid::parse_str(&providerId).is_ok());
        let error = manager
            .createProvider(
                "Same provider name".to_string(),
                "OPENAI".to_string(),
                "https://api.openai.com/v1".to_string(),
            )
            .expect_err("duplicate provider name should be rejected");
        assert!(matches!(
            error,
            ModelConfigError::ProviderNameAlreadyExists(name) if name == "Same provider name"
        ));

        let _ = fs::remove_dir_all(root);
    }

    /// Verifies renaming a provider onto an occupied name is rejected.
    #[test]
    fn update_provider_profile_rejects_duplicate_name() {
        let root = unique_test_root("update_provider_profile_rejects_duplicate_name");
        setup_test_runtime(root.clone());
        let manager = ModelConfigManager::new(root.clone());

        let providerId = manager
            .createProvider(
                "First provider".to_string(),
                "OPENAI".to_string(),
                "https://api.openai.com/v1".to_string(),
            )
            .expect("first provider");
        manager
            .createProvider(
                "Second provider".to_string(),
                "OPENAI".to_string(),
                "https://api.openai.com/v1".to_string(),
            )
            .expect("second provider");

        let mut provider = manager
            .getProviderProfile(&providerId)
            .expect("created provider");
        provider.name = "Second provider".to_string();
        let error = manager
            .updateProviderProfile(provider)
            .expect_err("duplicate provider name should be rejected");
        assert!(matches!(
            error,
            ModelConfigError::ProviderNameAlreadyExists(name) if name == "Second provider"
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn create_provider_model_rejects_existing_model_id() {
        let root = unique_test_root("create_provider_model_rejects_existing_model_id");
        setup_test_runtime(root.clone());
        let manager = ModelConfigManager::new(root.clone());

        let error = manager
            .createProviderModel(
                ModelConfigManager::DEFAULT_PROVIDER_ID,
                ModelConfigManager::DEFAULT_MODEL_ID.to_string(),
            )
            .expect_err("duplicate model id should be rejected");
        assert!(matches!(error, ModelConfigError::ModelAlreadyExists { .. }));

        let provider = manager
            .getProviderProfile(ModelConfigManager::DEFAULT_PROVIDER_ID)
            .expect("default provider");
        let count = provider
            .models
            .iter()
            .filter(|model| model.id == ModelConfigManager::DEFAULT_MODEL_ID)
            .count();
        assert_eq!(count, 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn add_provider_model_from_available_rejects_existing_model_id() {
        let root = unique_test_root("add_provider_model_from_available_rejects_existing_model_id");
        setup_test_runtime(root.clone());
        let manager = ModelConfigManager::new(root.clone());

        let error = manager
            .addProviderModelFromAvailable(
                ModelConfigManager::DEFAULT_PROVIDER_ID,
                available_model(ModelConfigManager::DEFAULT_MODEL_ID, None),
            )
            .expect_err("duplicate available model id should be rejected");
        assert!(matches!(error, ModelConfigError::ModelAlreadyExists { .. }));

        let provider = manager
            .getProviderProfile(ModelConfigManager::DEFAULT_PROVIDER_ID)
            .expect("default provider");
        let count = provider
            .models
            .iter()
            .filter(|model| model.id == ModelConfigManager::DEFAULT_MODEL_ID)
            .count();
        assert_eq!(count, 1);

        let _ = fs::remove_dir_all(root);
    }

    /// Keeps only the capability values that differ from what the provider list said.
    #[test]
    fn capability_updates_keep_only_values_that_differ_from_inherited() {
        use operit_model::ModelConfigData::{ModelCapabilities, ModelListedSpec};
        let root = unique_test_root("capability_updates_keep_only_differing_values");
        setup_test_runtime(root.clone());
        let manager = ModelConfigManager::new(root.clone());
        let listed = ModelListedSpec {
            capabilities: Some(ModelCapabilities {
                directImage: true,
                toolCall: true,
                ..ModelCapabilities::default()
            }),
            ..ModelListedSpec::default()
        };
        manager
            .addProviderModelFromAvailable(
                ModelConfigManager::DEFAULT_PROVIDER_ID,
                available_model("listed-test-model", Some(listed)),
            )
            .expect("add listed model");

        let model = manager
            .updateCapabilitiesForModel(
                ModelConfigManager::DEFAULT_PROVIDER_ID,
                "listed-test-model",
                ModelCapabilities {
                    directImage: true,
                    toolCall: false,
                    ..ModelCapabilities::default()
                },
            )
            .expect("update capabilities");
        assert_eq!(model.capabilitiesOverride.directImage, None);
        assert_eq!(model.capabilitiesOverride.toolCall, Some(false));

        let resolved = manager
            .getResolvedModelConfig(ModelConfigManager::DEFAULT_PROVIDER_ID, "listed-test-model")
            .expect("resolve listed model");
        assert!(resolved.capabilities.directImage);
        assert!(!resolved.capabilities.toolCall);

        let model = manager
            .clearModelSpecOverrides(ModelConfigManager::DEFAULT_PROVIDER_ID, "listed-test-model")
            .expect("clear overrides");
        assert!(!model.hasSpecOverrides());

        let _ = fs::remove_dir_all(root);
    }

    /// Drops the spec values earlier builds copied from the catalog and keeps the prompt-cache choice.
    #[test]
    fn schema_five_migration_drops_copied_model_specs() {
        use operit_model::ModelConfigData::ProviderProfile;
        use operit_store::PreferencesDataStore::{stringPreferencesKey, CoreNodeStateStore};
        let root = unique_test_root("schema_five_model_specs");
        let host: Arc<dyn RuntimeStorageHost> =
            Arc::new(TestRuntimeStorageHost { root: root.clone() });
        let path = "runtime/config/model-schema-five.preferences.json";
        let provider = ProviderProfile::new(
            "anthropic".into(),
            "Anthropic".into(),
            ApiProviderType::ANTHROPIC,
            "https://api.anthropic.com/v1/messages".into(),
        );
        let mut record = serde_json::to_value(&provider).unwrap();
        record["models"] = serde_json::json!([{
            "id": "claude-sonnet-5",
            "pricingOverride": null,
            "contextOverride": {"maxContextLength": 200.0},
            "capabilitiesOverride": {"directImage": false, "directAudio": false, "directVideo": false, "toolCall": false},
            "builtinToolsOverride": [],
            "requestOverride": {"supportsStructuredTools": false, "enableClaude1hPromptCache": true},
            "parameters": [],
            "summary": serde_json::to_value(
                operit_model::ModelConfigData::ModelSummarySettings::default()
            ).unwrap(),
            "localRuntime": serde_json::to_value(
                operit_model::ModelConfigData::LocalModelRuntimeSettings::default()
            ).unwrap()
        }]);
        let legacy = serde_json::json!({
            "__operit_preferences_schema_version": "5",
            "provider_anthropic": record
        });
        host.writeBytes(path, &serde_json::to_vec(&legacy).unwrap())
            .unwrap();
        let expectedMigrationStep = std::sync::atomic::AtomicU32::new(5);
        let store = CoreNodeStateStore::newWithStorage(host, path).withSchema(
            ModelConfigManager::PREFERENCES_VERSION,
            move |version, preferences| {
                assert_eq!(
                    version,
                    expectedMigrationStep.load(std::sync::atomic::Ordering::SeqCst),
                    "a version-five file starts at the spec reset"
                );
                expectedMigrationStep.store(version + 1, std::sync::atomic::Ordering::SeqCst);
                ModelConfigManager::migratePreferences(version, preferences)
            },
        );
        let migrated = store.data().unwrap();
        let record = migrated
            .get(&stringPreferencesKey("provider_anthropic"))
            .unwrap();
        let provider: ProviderProfile = serde_json::from_value(record.clone()).unwrap();
        let model = &provider.models[0];
        assert!(!model.hasSpecOverrides());
        assert_eq!(model.requestOverride.enableClaude1hPromptCache, Some(true));
        assert_eq!(
            store.schemaVersion().unwrap(),
            ModelConfigManager::PREFERENCES_VERSION
        );
        let _ = fs::remove_dir_all(root);
    }

    /// Upgrades persisted settings once, keeping user choices and cleaning every provider.
    #[test]
    fn schema_seven_migration_removes_retired_summary_controls() {
        use operit_store::PreferencesDataStore::{stringPreferencesKey, PreferencesDataStore};
        let root = unique_test_root("schema_seven_summary_controls");
        let host: Arc<dyn RuntimeStorageHost> =
            Arc::new(TestRuntimeStorageHost { root: root.clone() });
        let path = "runtime/config/model-schema-seven.preferences.json";
        let provider = serde_json::json!({
            "id": "custom",
            "apiKey": "preserved-test-key",
            "models": [
                {"id": "enabled", "summary": {
                    "enableSummary": true, "summaryTokenThreshold": 0.92,
                    "enableSummaryByMessageCount": true, "summaryMessageCountThreshold": 1
                }},
                {"id": "disabled", "summary": {
                    "enableSummary": false, "summaryTokenThreshold": 70,
                    "enableSummaryByMessageCount": false, "summaryMessageCountThreshold": 16
                }},
                {"id": "without-summary"}
            ]
        });
        let legacy = serde_json::json!({
            "__operit_preferences_schema_version": "7",
            "provider_custom": provider,
            "provider_unindexed": provider.to_string(),
            "unrelated_preference": "preserved"
        });
        host.writeBytes(path, &serde_json::to_vec(&legacy).unwrap()).unwrap();
        let store = ModelConfigManager::preferenceStore(host.clone(), path);
        let migrated = store.data().unwrap();
        let mut expected = provider;
        for summary in expected["models"].as_array_mut().unwrap().iter_mut()
            .filter_map(|model| model.get_mut("summary"))
        {
            let summary = summary.as_object_mut().unwrap();
            summary.remove("enableSummaryByMessageCount");
            summary.remove("summaryMessageCountThreshold");
        }
        for key in ["provider_custom", "provider_unindexed"] {
            assert_eq!(migrated.get(&stringPreferencesKey(key)), Some(&expected));
        }
        assert_eq!(
            migrated.get(&stringPreferencesKey("unrelated_preference")),
            Some(&serde_json::json!("preserved"))
        );
        assert_eq!(store.schemaVersion().unwrap(), 8);
        let reopened = PreferencesDataStore::newEncryptedWithStorage(host, path).withMigration(
            ModelConfigManager::PREFERENCES_VERSION,
            |_, _| panic!("a migrated store must not run migration again"),
        );
        assert_eq!(
            reopened.data().unwrap().get(&stringPreferencesKey("provider_custom")),
            Some(&expected)
        );
        let _ = fs::remove_dir_all(root);
    }

    fn available_model(
        modelId: &str,
        listed: Option<operit_model::ModelConfigData::ModelListedSpec>,
    ) -> operit_model::ModelConfigData::AvailableProviderModel {
        operit_model::ModelConfigData::AvailableProviderModel {
            modelId: modelId.to_string(),
            source: operit_model::ModelConfigData::AvailableProviderModelSource::Remote,
            catalogModelId: None,
            pricing: None,
            context: None,
            capabilities: None,
            builtinTools: Vec::new(),
            request: None,
            listed,
        }
    }

    /// Creates a provider of installed local models, as the user does from New provider.
    fn local_provider(manager: &ModelConfigManager) -> String {
        manager
            .createProvider(
                "Local Models".to_string(),
                ApiProviderType::LOCAL_MODEL.name().to_string(),
                String::new(),
            )
            .expect("create local provider")
    }

    #[test]
    fn thinking_levels_stay_per_model_and_carry_to_unset_models() {
        use operit_model::ModelThinking::{ModelThinkingSpec, ThinkingLevel::*, ThinkingSpecSource};
        let root = unique_test_root("thinking_levels_per_model");
        setup_test_runtime(root.clone());
        let manager = ModelConfigManager::new(root.clone());
        let provider = ModelConfigManager::DEFAULT_PROVIDER_ID;
        let graded = ModelConfigManager::DEFAULT_MODEL_ID;
        let switch = manager
            .createProviderModel(provider, "switch-only".to_string())
            .expect("second model");
        manager
            .setThinkingOverrideForModel(
                provider,
                graded,
                Some(ModelThinkingSpec {
                    supported: true,
                    toggle: true,
                    efforts: vec![Low, High, Max],
                    budget: None,
                }),
            )
            .expect("describe graded model");
        let described = manager
            .setThinkingOverrideForModel(provider, &switch, Some(ModelThinkingSpec::switchable()))
            .expect("describe switch-only model");
        assert_eq!(described.source, ThinkingSpecSource::Declared);

        // Nothing set yet: no thinking.
        let fresh = manager.getThinkingSettingsForModel(provider, graded).unwrap();
        assert_eq!(
            fresh.levels.iter().map(|option| option.level).collect::<Vec<_>>(),
            vec![Off, Low, High, Max]
        );
        assert_eq!((fresh.current, fresh.chosen), (Some(Off), false));

        // A level set on one model carries to a model never set, as its nearest level.
        let graded_high = manager
            .setThinkingLevelForModel(provider, graded, High)
            .expect("set high");
        assert_eq!((graded_high.current, graded_high.chosen), (Some(High), true));
        let carried = manager.getThinkingSettingsForModel(provider, &switch).unwrap();
        assert_eq!((carried.current, carried.chosen), (Some(Enabled), false));

        // A level set on the second model stays there and leaves the first alone.
        manager
            .setThinkingLevelForModel(provider, &switch, Off)
            .expect("turn off");
        assert_eq!(
            manager
                .getResolvedModelConfig(provider, &switch)
                .unwrap()
                .thinkingLevel,
            Some(Off)
        );
        assert_eq!(
            manager
                .getResolvedModelConfig(provider, graded)
                .unwrap()
                .thinkingLevel,
            Some(High)
        );

        // A level the model does not offer is refused.
        assert!(matches!(
            manager.setThinkingLevelForModel(provider, graded, Medium),
            Err(ModelConfigError::InvalidThinkingLevel(_))
        ));

        // Clearing the description leaves a model the catalog does not know without levels.
        let cleared = manager
            .setThinkingOverrideForModel(provider, &switch, None)
            .expect("clear description");
        assert!(cleared.levels.is_empty());
        assert_eq!(cleared.current, None);
        fs::remove_dir_all(root).expect("remove model config test root");
    }

    #[test]
    fn local_models_switch_thinking_on_and_off() {
        use operit_model::ModelConfigData::{ModelProfile, ProviderProfile};
        use operit_model::ModelThinking::{ThinkingLevel::*, ThinkingSpecSource};
        let provider = ProviderProfile::new(
            "local".to_string(),
            "Local Models".to_string(),
            ApiProviderType::LOCAL_MODEL,
            String::new(),
        );
        let model = ModelProfile::new("qwen3@v1".to_string());
        let thinking = ModelConfigManager::resolvedThinking(&provider, &model).unwrap();
        assert_eq!(thinking.source, ThinkingSpecSource::LocalRuntime);
        assert_eq!(
            ModelConfigManager::thinkingLevels(&provider, &model, &thinking.spec)
                .iter()
                .map(|option| option.level)
                .collect::<Vec<_>>(),
            vec![Off, Enabled]
        );
    }

    fn unique_test_root(name: &str) -> std::path::PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("operit_model_config_test_{name}_{now}"))
    }

    /// Seeds a downloaded catalog so configuration tests never depend on network access.
    fn setup_test_runtime(root: PathBuf) {
        let catalogDir = root.join(
            operit_util::RuntimeStorageLayout::RUNTIME_MODELS_DEV_CACHE_DIR_PATH,
        );
        fs::create_dir_all(&catalogDir).unwrap();
        fs::write(catalogDir.join("api.json"), br#"{"fixture":{"models":{}}}"#).unwrap();
        fs::write(catalogDir.join("models.json"), b"{}").unwrap();
        fs::write(catalogDir.join("downloaded.json"), br#"{"updatedAt":1}"#).unwrap();
        setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(
            root.clone(),
            root.join(WORKSPACE_DIR_PATH),
        ));
        setDefaultRuntimeStorageHost(Arc::new(TestRuntimeStorageHost { root }));
    }

    struct TestRuntimeStorageHost {
        root: PathBuf,
    }

    impl TestRuntimeStorageHost {
        fn resolve(&self, path: &str) -> HostResult<PathBuf> {
            let path = Path::new(path);
            if path.is_absolute() {
                return Err(HostError::new(format!(
                    "runtime storage path must be relative: {}",
                    path.display()
                )));
            }
            let mut resolved = self.root.clone();
            for component in path.components() {
                match component {
                    Component::Normal(segment) => resolved.push(segment),
                    Component::CurDir => {}
                    _ => {
                        return Err(HostError::new(format!(
                            "invalid runtime storage path: {}",
                            path.display()
                        )));
                    }
                }
            }
            Ok(resolved)
        }
    }

    impl RuntimeStorageHost for TestRuntimeStorageHost {
        fn openTransactionDatabase(&self, path: &str) -> HostResult<Box<dyn operit_host_api::RuntimeSqliteConnection>> {
            operit_host_native_storage::NativeRuntimeStorageHost::new(self.root.join("runtime"), self.root.join(WORKSPACE_DIR_PATH))
                .openTransactionDatabase(path)
        }

        /// Flushes the existing test file through its backing filesystem.
        fn flushFile(&self, path: &str) -> operit_host_api::HostResult<()> {
            std::fs::OpenOptions::new().write(true).open(self.resolve(path)?)?.sync_all()?;
            Ok(())
        }

        /// Returns the temporary runtime root.
        fn runtimeRootDir(&self) -> Option<PathBuf> {
            Some(self.root.clone())
        }

        /// Returns the temporary workspace collection root.
        fn workspaceRootDir(&self) -> Option<PathBuf> {
            Some(self.root.join(WORKSPACE_DIR_PATH))
        }

        fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
            Ok(fs::read(self.resolve(path)?)?)
        }

        /// Atomically publishes one complete in-memory test entry.
        fn writeBytesAtomically(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
            self.writeBytes(path, content)
        }

        fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
            let path = self.resolve(path)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, content)?;
            Ok(())
        }

        /// Appends bytes to the temporary runtime root.
        fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
            let path = self.resolve(path)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;
            std::io::Write::write_all(&mut file, content)?;
            Ok(())
        }

        fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
            let path = self.resolve(path)?;
            if !path.exists() {
                return Ok(());
            }
            if path.is_dir() {
                if recursive {
                    fs::remove_dir_all(path)?;
                } else {
                    fs::remove_dir(path)?;
                }
            } else {
                fs::remove_file(path)?;
            }
            Ok(())
        }

        fn exists(&self, path: &str) -> HostResult<bool> {
            Ok(self.resolve(path)?.exists())
        }

        fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
            let root = self.resolve(prefix)?;
            if !root.exists() {
                return Ok(Vec::new());
            }
            let mut entries = Vec::new();
            for entry in fs::read_dir(root)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                entries.push(RuntimeStorageEntry {
                    path: entry.path().to_string_lossy().replace('\\', "/"),
                    isDirectory: metadata.is_dir(),
                    size: metadata.len() as i64,
                });
            }
            Ok(entries)
        }
    }
}
