use std::path::PathBuf;

use thiserror::Error;

use crate::data::preferences::ApiPreferences::ApiPreferences;
use operit_model::ModelCatalog::ModelCatalog;
use operit_model::ModelConfigData::{
    default_deepseek_provider, ApiProviderType, AvailableProviderModel,
    AvailableProviderModelSource, ModelCapabilities, ModelCatalogKey, ModelConfigDefaults,
    ModelContextSpec, ModelProfile, ModelRequestSpec, ModelSummarySettings, ProviderModelSummary,
    ProviderProfile, ResolvedModelConfig,
};
use operit_model::ModelParameter::ModelParameter;
use operit_providers::chat::llmprovider::ModelConfigConnectionTester::{
    ModelConfigConnectionTester, ModelConnectionTestReport,
};
use operit_providers::chat::llmprovider::ModelListFetcher::ModelListFetcher;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, Preferences, PreferencesDataStore, PreferencesDataStoreError,
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
    #[error("catalog model not found: {providerTypeId}:{modelId}")]
    CatalogModelNotFound {
        providerTypeId: String,
        modelId: String,
    },
    #[error("missing model context: {0}")]
    MissingModelContext(String),
    #[error("missing model capabilities: {0}")]
    MissingModelCapabilities(String),
    #[error("missing model request spec: {0}")]
    MissingModelRequestSpec(String),
    #[error("invalid provider type: {0}")]
    InvalidProviderType(String),
    #[error("available provider model not found: {providerId}:{modelId}")]
    AvailableProviderModelNotFound { providerId: String, modelId: String },
    #[error("model list fetch error: {0}")]
    ModelListFetch(String),
    #[error("connection test error: {0}")]
    ConnectionTest(String),
}

/// Stores provider profiles, model profiles, and resolved model configuration.
#[derive(Clone)]
pub struct ModelConfigManager {
    paths: RuntimeStorePaths,
    modelConfigDataStore: PreferencesDataStore,
}

impl ModelConfigManager {
    pub const DEFAULT_PROVIDER_ID: &'static str = ModelConfigDefaults::DEFAULT_PROVIDER_ID;
    pub const DEFAULT_MODEL_ID: &'static str = ModelConfigDefaults::DEFAULT_MODEL_ID;

    /// Returns the preference key storing provider profile order.
    pub fn PROVIDER_LIST_KEY() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("provider_list")
    }

    /// Creates a manager rooted at a runtime data directory.
    pub fn new(root_dir: PathBuf) -> Self {
        let paths = RuntimeStorePaths::new(root_dir);
        let modelConfigDataStore =
            PreferencesDataStore::new(paths.model_configs_preferences_path());
        Self {
            paths,
            modelConfigDataStore,
        }
    }

    /// Creates a manager using the default API data directory.
    pub fn default() -> Self {
        Self::new(ApiPreferences::data_dir())
    }

    /// Ensures the default provider profile exists in preferences.
    pub fn initializeIfNeeded(&self) -> Result<(), ModelConfigError> {
        self.modelConfigDataStore.try_edit_result(|preferences| {
            let providerIds = Self::readProviderList(preferences)?;
            if providerIds.is_empty() {
                let provider = default_deepseek_provider();
                Self::writeProvider(preferences, &provider)?;
                Self::writeProviderList(preferences, &[provider.id.clone()])?;
            }
            Ok::<(), ModelConfigError>(())
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
        self.getProviderIds()?
            .iter()
            .map(|providerId| self.getProviderProfile(providerId))
            .collect()
    }

    /// Reads one provider profile by id.
    pub fn getProviderProfile(
        &self,
        providerId: &str,
    ) -> Result<ProviderProfile, ModelConfigError> {
        self.loadProviderFromDataStore(providerId)
    }

    /// Builds summary rows for every configured provider model.
    pub fn getAllModelSummaries(&self) -> Result<Vec<ProviderModelSummary>, ModelConfigError> {
        let providers = self.getProviderProfiles()?;
        let mut summaries = Vec::new();
        for provider in providers {
            for model in &provider.models {
                let resolved = self.resolveFromProfiles(&provider, model)?;
                summaries.push(ProviderModelSummary {
                    providerId: provider.id.clone(),
                    providerName: provider.name.clone(),
                    providerTypeId: provider.providerTypeId.clone(),
                    endpoint: provider.endpoint.clone(),
                    modelId: model.id.clone(),
                    capabilities: resolved.capabilities,
                    pricing: resolved.pricing,
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
        let providerId = self.createProviderId();
        let provider = ProviderProfile::new(providerId.clone(), name, providerType, endpoint);
        self.modelConfigDataStore.try_edit_result(|preferences| {
            Self::assertProviderNameUniqueInPreferences(preferences, &provider.name, None)?;
            let mut providerIds = Self::readProviderList(preferences)?;
            providerIds.push(providerId.clone());
            Self::writeProvider(preferences, &provider)?;
            Self::writeProviderList(preferences, &providerIds)?;
            Ok::<(), ModelConfigError>(())
        })?;
        Ok(providerId)
    }

    /// Replaces an existing provider profile after validation.
    pub fn updateProviderProfile(
        &self,
        provider: ProviderProfile,
    ) -> Result<ProviderProfile, ModelConfigError> {
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
            let mut providerIds = Self::readProviderList(preferences)?;
            if !providerIds.iter().any(|id| id == Self::DEFAULT_PROVIDER_ID) {
                providerIds.insert(0, Self::DEFAULT_PROVIDER_ID.to_string());
            }
            Self::writeProvider(preferences, &provider)?;
            Self::writeProviderList(preferences, &providerIds)?;
            Ok::<(), ModelConfigError>(())
        })?;
        Ok(provider)
    }

    /// Deletes one provider profile and removes it from provider order.
    pub fn deleteProvider(&self, providerId: &str) -> Result<(), ModelConfigError> {
        let providerKey = self.providerKey(providerId);
        self.modelConfigDataStore.try_edit_result(|preferences| {
            self.assertProviderExistsInPreferences(preferences, providerId)?;
            let mut providerIds = Self::readProviderList(preferences)?;
            providerIds.retain(|id| id != providerId);
            preferences.remove(&providerKey);
            Self::writeProviderList(preferences, &providerIds)?;
            Ok::<(), ModelConfigError>(())
        })?;
        Ok(())
    }

    /// Creates a model profile under an existing provider.
    pub fn createProviderModel(
        &self,
        providerId: &str,
        modelId: String,
    ) -> Result<String, ModelConfigError> {
        self.updateProviderInternalResult(providerId, |mut provider| {
            Self::assertProviderModelDoesNotExist(&provider, &modelId)?;
            let model = Self::newModelProfileWithResolvedSpecs(&provider, modelId.clone());
            provider.models.push(model);
            Ok(provider)
        })?;
        Ok(modelId)
    }

    /// Fetches provider models available for import into a provider profile.
    pub fn getAvailableProviderModels(
        &self,
        providerId: &str,
    ) -> Result<Vec<AvailableProviderModel>, ModelConfigError> {
        let provider = self.getProviderProfile(providerId)?;
        let providerCatalog = ModelCatalog::provider(&provider.providerTypeId)
            .map_err(ModelConfigError::ModelListFetch)?;
        let mut models: Vec<AvailableProviderModel> = providerCatalog
            .models
            .iter()
            .map(|model| AvailableProviderModel {
                modelId: model.modelId.clone(),
                source: AvailableProviderModelSource::Catalog,
                pricing: model.pricing.clone(),
                context: model.context.clone(),
                capabilities: model.capabilities.clone(),
                builtinTools: model.builtinTools.clone(),
                request: model.request.clone(),
            })
            .collect();
        let remoteModels = ModelListFetcher::fetch(&provider, &providerCatalog)
            .map_err(ModelConfigError::ModelListFetch)?;
        for remoteModel in remoteModels {
            let remoteModel = Self::completeAvailableProviderModel(remoteModel);
            if !models
                .iter()
                .any(|model| model.modelId.eq_ignore_ascii_case(&remoteModel.modelId))
            {
                models.push(remoteModel);
            }
        }
        Ok(models)
    }

    /// Adds a provider model using catalog or remote availability metadata.
    pub fn addProviderModelFromAvailable(
        &self,
        providerId: &str,
        modelId: String,
    ) -> Result<String, ModelConfigError> {
        let provider = self.getProviderProfile(providerId)?;
        let availableModel = self.findAvailableProviderModel(&provider, &modelId)?;
        self.updateProviderInternalResult(providerId, |mut provider| {
            Self::assertProviderModelDoesNotExist(&provider, &modelId)?;
            let mut model = ModelProfile::new(modelId.clone());
            match availableModel.source {
                AvailableProviderModelSource::Catalog => {
                    model.catalogKey = Some(ModelCatalogKey {
                        providerTypeId: provider.providerTypeId.clone(),
                        modelId: modelId.clone(),
                    });
                }
                AvailableProviderModelSource::Remote => {
                    let availableModel =
                        Self::completeAvailableProviderModel(availableModel.clone());
                    model.pricingOverride = availableModel.pricing.clone();
                    model.contextOverride = availableModel.context.clone();
                    model.capabilitiesOverride = availableModel.capabilities.clone();
                    model.builtinToolsOverride = Some(availableModel.builtinTools.clone());
                    model.requestOverride = availableModel.request.clone();
                }
            }
            provider.models.push(model);
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

    /// Updates model capabilities for one provider/model pair.
    pub fn updateCapabilitiesForModel(
        &self,
        providerId: &str,
        modelId: &str,
        capabilities: ModelCapabilities,
    ) -> Result<ModelProfile, ModelConfigError> {
        let mut model = self.getModelProfile(providerId, modelId)?;
        model.capabilitiesOverride = Some(capabilities);
        self.updateModelProfile(providerId, model)
    }

    /// Updates model context settings for one provider/model pair.
    pub fn updateContextForModel(
        &self,
        providerId: &str,
        modelId: &str,
        context: ModelContextSpec,
    ) -> Result<ModelProfile, ModelConfigError> {
        let mut model = self.getModelProfile(providerId, modelId)?;
        model.contextOverride = Some(context);
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

    /// Updates request settings for one provider/model pair.
    pub fn updateRequestForModel(
        &self,
        providerId: &str,
        modelId: &str,
        request: ModelRequestSpec,
    ) -> Result<ModelProfile, ModelConfigError> {
        let mut model = self.getModelProfile(providerId, modelId)?;
        model.requestOverride = Some(request);
        self.updateModelProfile(providerId, model)
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

    /// Tests connectivity for one provider/model configuration.
    pub async fn testModelConnection(
        &self,
        providerId: &str,
        modelId: &str,
    ) -> Result<ModelConnectionTestReport, ModelConfigError> {
        ModelConfigConnectionTester::run(self.paths.root_dir().to_path_buf(), providerId, modelId)
            .await
            .map_err(ModelConfigError::ConnectionTest)
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

    /// Resolves a model profile against its owning provider profile.
    fn resolveFromProfiles(
        &self,
        provider: &ProviderProfile,
        model: &ModelProfile,
    ) -> Result<ResolvedModelConfig, ModelConfigError> {
        let catalogModel = Self::catalogModelForProfile(provider, model)?;

        let pricing = match &model.pricingOverride {
            Some(pricing) => Some(pricing.clone()),
            None => match &catalogModel {
                Some(entry) => entry.pricing.clone(),
                None => None,
            },
        };
        let context = match &model.contextOverride {
            Some(context) => context.clone(),
            None => match &catalogModel {
                Some(entry) => entry.context.clone().unwrap_or_default(),
                None => ModelContextSpec::default(),
            },
        };
        let capabilities = match &model.capabilitiesOverride {
            Some(capabilities) => capabilities.clone(),
            None => match &catalogModel {
                Some(entry) => entry.capabilities.clone().unwrap_or_default(),
                None => ModelCapabilities::default(),
            },
        };
        let builtinTools = match &model.builtinToolsOverride {
            Some(builtinTools) => builtinTools.clone(),
            None => match &catalogModel {
                Some(entry) => entry.builtinTools.clone(),
                None => Vec::new(),
            },
        };
        let request = match &model.requestOverride {
            Some(request) => request.clone(),
            None => match &catalogModel {
                Some(entry) => entry.request.clone().unwrap_or_default(),
                None => ModelRequestSpec::default(),
            },
        };

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
            summary: model.summary.clone(),
            localRuntime: model.localRuntime.clone(),
        })
    }

    /// Reads provider ids from the preferences object.
    fn readProviderList(
        preferences: &Preferences,
    ) -> Result<Vec<String>, PreferencesDataStoreError> {
        match preferences.get(&Self::PROVIDER_LIST_KEY()) {
            Some(providerList) if !providerList.is_empty() => {
                Ok(serde_json::from_str(providerList)?)
            }
            _ => Ok(Vec::new()),
        }
    }

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
        Ok(serde_json::from_str(providerJson)?)
    }

    /// Saves one provider profile to preferences.
    fn saveProviderToDataStore(&self, provider: &ProviderProfile) -> Result<(), ModelConfigError> {
        self.modelConfigDataStore.try_edit_result(|preferences| {
            Self::writeProvider(preferences, provider)?;
            Ok::<(), ModelConfigError>(())
        })
    }

    /// Saves provider order to preferences.
    fn saveProviderList(&self, providerIds: Vec<String>) -> Result<(), ModelConfigError> {
        self.modelConfigDataStore.try_edit_result(|preferences| {
            Self::writeProviderList(preferences, &providerIds)?;
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

    fn readProviderFromPreferences(
        &self,
        preferences: &Preferences,
        providerId: &str,
    ) -> Result<ProviderProfile, ModelConfigError> {
        let providerKey = self.providerKey(providerId);
        let providerJson = preferences
            .get(&providerKey)
            .ok_or_else(|| ModelConfigError::ProviderNotFound(providerId.to_string()))?;
        Ok(serde_json::from_str(providerJson)?)
    }

    fn writeProvider(
        preferences: &mut Preferences,
        provider: &ProviderProfile,
    ) -> Result<(), ModelConfigError> {
        let providerKey = stringPreferencesKey(&format!("provider_{}", provider.id));
        let encodedProvider = serde_json::to_string(provider)?;
        preferences.set(&providerKey, encodedProvider);
        Ok(())
    }

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
            let provider: ProviderProfile = serde_json::from_str(providerJson)?;
            if provider.name.trim() == normalizedName {
                return Err(ModelConfigError::ProviderNameAlreadyExists(
                    normalizedName.to_string(),
                ));
            }
        }
        Ok(())
    }
    fn writeProviderList(
        preferences: &mut Preferences,
        providerIds: &[String],
    ) -> Result<(), ModelConfigError> {
        let encoded = serde_json::to_string(providerIds)?;
        preferences.set(&Self::PROVIDER_LIST_KEY(), encoded);
        Ok(())
    }

    fn findAvailableProviderModel(
        &self,
        provider: &ProviderProfile,
        modelId: &str,
    ) -> Result<AvailableProviderModel, ModelConfigError> {
        let providerCatalog = ModelCatalog::provider(&provider.providerTypeId)
            .map_err(ModelConfigError::ModelListFetch)?;
        if let Some(model) = providerCatalog
            .models
            .iter()
            .find(|model| model.modelId.eq_ignore_ascii_case(modelId))
        {
            return Ok(AvailableProviderModel {
                modelId: model.modelId.clone(),
                source: AvailableProviderModelSource::Catalog,
                pricing: model.pricing.clone(),
                context: model.context.clone(),
                capabilities: model.capabilities.clone(),
                builtinTools: model.builtinTools.clone(),
                request: model.request.clone(),
            });
        }
        ModelListFetcher::fetch(provider, &providerCatalog)
            .map_err(ModelConfigError::ModelListFetch)?
            .into_iter()
            .find(|model| model.modelId.eq_ignore_ascii_case(modelId))
            .map(Self::completeAvailableProviderModel)
            .ok_or_else(|| ModelConfigError::AvailableProviderModelNotFound {
                providerId: provider.id.clone(),
                modelId: modelId.to_string(),
            })
    }

    fn completeAvailableProviderModel(mut model: AvailableProviderModel) -> AvailableProviderModel {
        if let Some(catalogModel) = ModelCatalog::modelByTerminalId(&model.modelId) {
            if model.pricing.is_none() {
                model.pricing = catalogModel.pricing;
            }
            if model.context.is_none() {
                model.context = catalogModel.context;
            }
            if model.capabilities.is_none() {
                model.capabilities = catalogModel.capabilities;
            }
            if model.builtinTools.is_empty() {
                model.builtinTools = catalogModel.builtinTools;
            }
            if model.request.is_none() {
                model.request = catalogModel.request;
            }
        }
        if model.context.is_none() {
            model.context = Some(ModelContextSpec::default());
        }
        if model.capabilities.is_none() {
            model.capabilities = Some(ModelCapabilities::default());
        }
        if model.request.is_none() {
            model.request = Some(ModelRequestSpec::default());
        }
        model
    }

    fn newModelProfileWithResolvedSpecs(
        provider: &ProviderProfile,
        modelId: String,
    ) -> ModelProfile {
        let mut model = ModelProfile::new(modelId.clone());
        if let Ok(catalogModel) = ModelCatalog::model(&provider.providerTypeId, &modelId) {
            model.pricingOverride = catalogModel.pricing;
            model.contextOverride = catalogModel.context;
            model.capabilitiesOverride = catalogModel.capabilities;
            model.builtinToolsOverride = Some(catalogModel.builtinTools);
            model.requestOverride = catalogModel.request;
        }
        if model.contextOverride.is_none() {
            model.contextOverride = Some(ModelContextSpec::default());
        }
        if model.capabilitiesOverride.is_none() {
            model.capabilitiesOverride = Some(ModelCapabilities::default());
        }
        if model.requestOverride.is_none() {
            model.requestOverride = Some(ModelRequestSpec::default());
        }
        model
    }

    fn catalogModelForProfile(
        provider: &ProviderProfile,
        model: &ModelProfile,
    ) -> Result<Option<operit_model::ModelConfigData::ModelCatalogEntry>, ModelConfigError> {
        match &model.catalogKey {
            Some(key) => ModelCatalog::model(&key.providerTypeId, &key.modelId)
                .map(Some)
                .map_err(|_| ModelConfigError::CatalogModelNotFound {
                    providerTypeId: key.providerTypeId.clone(),
                    modelId: key.modelId.clone(),
                }),
            None => Ok(ModelCatalog::model(&provider.providerTypeId, &model.id).ok()),
        }
    }

    fn assertProviderExists(&self, providerId: &str) -> Result<(), ModelConfigError> {
        let providerIds = self.getProviderIds()?;
        if providerIds.iter().any(|id| id == providerId) {
            Ok(())
        } else {
            Err(ModelConfigError::ProviderNotFound(providerId.to_string()))
        }
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

    fn createProviderId(&self) -> String {
        format!(
            "provider_{}",
            operit_host_api::TimeUtils::currentTimeMillis()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{ModelConfigError, ModelConfigManager};
    use operit_host_api::{HostError, HostResult, RuntimeStorageEntry, RuntimeStorageHost};
    use operit_model::ModelConfigData::ModelConfigDefaults;
    use operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost;
    use operit_util::RuntimeStoreRoot::setDefaultRuntimeStoreRoot;
    use std::fs;
    use std::path::{Component, Path, PathBuf};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn default_ids_are_model_ids() {
        assert_eq!(
            ModelConfigManager::DEFAULT_MODEL_ID,
            ModelConfigDefaults::DEFAULT_MODEL_ID
        );
    }

    #[test]
    fn provider_model_update_keeps_existing_api_key() {
        let root = unique_test_root("provider_internal_update_keeps_latest_api_key");
        setup_test_runtime(root.clone());
        let manager = ModelConfigManager::new(root.clone());
        manager
            .initializeIfNeeded()
            .expect("initialize model config");

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

    #[test]
    fn create_provider_model_rejects_existing_model_id() {
        let root = unique_test_root("create_provider_model_rejects_existing_model_id");
        setup_test_runtime(root.clone());
        let manager = ModelConfigManager::new(root.clone());
        manager
            .initializeIfNeeded()
            .expect("initialize model config");

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
        manager
            .initializeIfNeeded()
            .expect("initialize model config");

        let error = manager
            .addProviderModelFromAvailable(
                ModelConfigManager::DEFAULT_PROVIDER_ID,
                ModelConfigManager::DEFAULT_MODEL_ID.to_string(),
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

    fn unique_test_root(name: &str) -> std::path::PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("operit_model_config_test_{name}_{now}"))
    }

    fn setup_test_runtime(root: PathBuf) {
        setDefaultRuntimeStoreRoot(root.clone());
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
        fn rootDir(&self) -> Option<PathBuf> {
            Some(self.root.clone())
        }

        fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
            Ok(fs::read(self.resolve(path)?)?)
        }

        fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
            let path = self.resolve(path)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, content)?;
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
