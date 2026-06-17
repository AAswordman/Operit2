use std::path::PathBuf;

use thiserror::Error;

use crate::api::chat::llmprovider::ModelConfigConnectionTester::{
    ModelConfigConnectionTester, ModelConnectionTestReport,
};
use crate::api::chat::llmprovider::ModelListFetcher::ModelListFetcher;
use crate::data::model::ModelCatalog::ModelCatalog;
use crate::data::model::ModelConfigData::{
    default_deepseek_provider, ApiProviderType, AvailableProviderModel,
    AvailableProviderModelSource, ModelCapabilities, ModelCatalogKey, ModelConfigDefaults,
    ModelContextSpec, ModelProfile, ModelRequestSpec, ModelSummarySettings, ProviderModelSummary,
    ProviderProfile, ResolvedModelConfig,
};
use crate::data::model::ModelParameter::ModelParameter;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, Preferences, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

#[derive(Debug, Error)]
pub enum ModelConfigError {
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("store error: {0}")]
    Store(#[from] PreferencesDataStoreError),
    #[error("provider not found: {0}")]
    ProviderNotFound(String),
    #[error("model not found: {0}")]
    ModelNotFound(String),
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

#[derive(Clone)]
pub struct ModelConfigManager {
    paths: RuntimeStorePaths,
    modelConfigDataStore: PreferencesDataStore,
}

impl ModelConfigManager {
    pub const DEFAULT_PROVIDER_ID: &'static str = ModelConfigDefaults::DEFAULT_PROVIDER_ID;
    pub const DEFAULT_MODEL_ID: &'static str = ModelConfigDefaults::DEFAULT_MODEL_ID;

    pub fn PROVIDER_LIST_KEY() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("provider_list")
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
        let providerIds = self.providerListFlow()?.first()?;
        if providerIds.is_empty() {
            let provider = default_deepseek_provider();
            self.saveProviderToDataStore(&provider)?;
            self.saveProviderList(vec![provider.id.clone()])?;
        }
        Ok(())
    }

    pub fn providerListFlow(&self) -> Result<Flow<Vec<String>>, ModelConfigError> {
        Ok(self
            .modelConfigDataStore
            .dataFlow()
            .mapResult(|preferences| Self::readProviderList(&preferences)))
    }

    pub fn getProviderIds(&self) -> Result<Vec<String>, ModelConfigError> {
        Ok(self.providerListFlow()?.first()?)
    }

    pub fn getProviderProfilesFlow(&self) -> Result<Flow<Vec<ProviderProfile>>, ModelConfigError> {
        let manager = self.clone();
        Ok(self.modelConfigDataStore.dataFlow().mapResult(move |_| {
            manager
                .getProviderProfiles()
                .map_err(|error| PreferencesDataStoreError::Message(error.to_string()))
        }))
    }

    pub fn getProviderProfiles(&self) -> Result<Vec<ProviderProfile>, ModelConfigError> {
        self.getProviderIds()?
            .iter()
            .map(|providerId| self.getProviderProfile(providerId))
            .collect()
    }

    pub fn getProviderProfile(
        &self,
        providerId: &str,
    ) -> Result<ProviderProfile, ModelConfigError> {
        self.loadProviderFromDataStore(providerId)
    }

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

    pub fn getProviderCatalogEntries(
        &self,
    ) -> Result<Vec<crate::data::model::ModelConfigData::ProviderCatalogEntry>, ModelConfigError>
    {
        ModelCatalog::providers().map_err(ModelConfigError::ModelListFetch)
    }

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
        let mut providerIds = self.getProviderIds()?;
        providerIds.push(providerId.clone());
        self.saveProviderToDataStore(&provider)?;
        self.saveProviderList(providerIds)?;
        Ok(providerId)
    }

    pub fn updateProviderProfile(
        &self,
        provider: ProviderProfile,
    ) -> Result<ProviderProfile, ModelConfigError> {
        self.assertProviderExists(&provider.id)?;
        self.saveProviderToDataStore(&provider)?;
        Ok(provider)
    }

    pub fn deleteProvider(&self, providerId: &str) -> Result<(), ModelConfigError> {
        self.assertProviderExists(providerId)?;
        let mut providerIds = self.getProviderIds()?;
        providerIds.retain(|id| id != providerId);
        let providerKey = self.providerKey(providerId);
        let encodedProviderIds = serde_json::to_string(&providerIds)?;
        self.modelConfigDataStore.edit(|preferences| {
            preferences.remove(&providerKey);
            preferences.set(&Self::PROVIDER_LIST_KEY(), encodedProviderIds);
        })?;
        Ok(())
    }

    pub fn createProviderModel(
        &self,
        providerId: &str,
        modelId: String,
    ) -> Result<String, ModelConfigError> {
        self.updateProviderInternal(providerId, |mut provider| {
            provider.models.push(ModelProfile::new(modelId.clone()));
            provider
        })?;
        Ok(modelId)
    }

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
            if !models
                .iter()
                .any(|model| model.modelId.eq_ignore_ascii_case(&remoteModel.modelId))
            {
                models.push(remoteModel);
            }
        }
        Ok(models)
    }

    pub fn addProviderModelFromAvailable(
        &self,
        providerId: &str,
        modelId: String,
    ) -> Result<String, ModelConfigError> {
        let provider = self.getProviderProfile(providerId)?;
        let availableModel = self.findAvailableProviderModel(&provider, &modelId)?;
        self.updateProviderInternal(providerId, |mut provider| {
            let mut model = ModelProfile::new(modelId.clone());
            match availableModel.source {
                AvailableProviderModelSource::Catalog => {
                    model.catalogKey = Some(ModelCatalogKey {
                        providerTypeId: provider.providerTypeId.clone(),
                        modelId: modelId.clone(),
                    });
                }
                AvailableProviderModelSource::Remote => {
                    model.pricingOverride = availableModel.pricing.clone();
                    model.contextOverride = availableModel.context.clone();
                    model.capabilitiesOverride = availableModel.capabilities.clone();
                    model.builtinToolsOverride = Some(availableModel.builtinTools.clone());
                    model.requestOverride = availableModel.request.clone();
                }
            }
            provider.models.push(model);
            provider
        })?;
        Ok(modelId)
    }

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

    pub fn deleteModel(&self, providerId: &str, modelId: &str) -> Result<(), ModelConfigError> {
        self.updateProviderInternal(providerId, |mut provider| {
            provider.models.retain(|model| model.id != modelId);
            provider
        })?;
        Ok(())
    }

    pub fn getModelProfile(
        &self,
        providerId: &str,
        modelId: &str,
    ) -> Result<ModelProfile, ModelConfigError> {
        let (_, model) = self.findModel(providerId, modelId)?;
        Ok(model)
    }

    pub fn getResolvedModelConfig(
        &self,
        providerId: &str,
        modelId: &str,
    ) -> Result<ResolvedModelConfig, ModelConfigError> {
        let (provider, model) = self.findModel(providerId, modelId)?;
        self.resolveFromProfiles(&provider, &model)
    }

    pub fn getModelParametersForModel(
        &self,
        providerId: &str,
        modelId: &str,
    ) -> Result<Vec<ModelParameter<serde_json::Value>>, ModelConfigError> {
        let (_, model) = self.findModel(providerId, modelId)?;
        Ok(model.parameters)
    }

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

    pub fn updateBuiltinToolsForModel(
        &self,
        providerId: &str,
        modelId: &str,
        builtinTools: Vec<crate::data::model::ModelConfigData::ModelBuiltinTool>,
    ) -> Result<ModelProfile, ModelConfigError> {
        let mut model = self.getModelProfile(providerId, modelId)?;
        model.builtinToolsOverride = Some(builtinTools);
        self.updateModelProfile(providerId, model)
    }

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

    pub async fn testModelConnection(
        &self,
        providerId: &str,
        modelId: &str,
    ) -> Result<ModelConnectionTestReport, ModelConfigError> {
        ModelConfigConnectionTester::run(self.paths.root_dir().to_path_buf(), providerId, modelId)
            .await
            .map_err(ModelConfigError::ConnectionTest)
    }

    pub fn exportAllProviders(&self) -> Result<String, String> {
        serde_json::to_string_pretty(
            &self
                .getProviderProfiles()
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())
    }

    fn resolveFromProfiles(
        &self,
        provider: &ProviderProfile,
        model: &ModelProfile,
    ) -> Result<ResolvedModelConfig, ModelConfigError> {
        let catalogModel = match &model.catalogKey {
            Some(key) => Some(
                ModelCatalog::model(&key.providerTypeId, &key.modelId).map_err(|_| {
                    ModelConfigError::CatalogModelNotFound {
                        providerTypeId: key.providerTypeId.clone(),
                        modelId: key.modelId.clone(),
                    }
                })?,
            ),
            None => None,
        };

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
                Some(entry) => entry
                    .context
                    .clone()
                    .ok_or_else(|| ModelConfigError::MissingModelContext(model.id.clone()))?,
                None => return Err(ModelConfigError::MissingModelContext(model.id.clone())),
            },
        };
        let capabilities = match &model.capabilitiesOverride {
            Some(capabilities) => capabilities.clone(),
            None => match &catalogModel {
                Some(entry) => entry
                    .capabilities
                    .clone()
                    .ok_or_else(|| ModelConfigError::MissingModelCapabilities(model.id.clone()))?,
                None => return Err(ModelConfigError::MissingModelCapabilities(model.id.clone())),
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
                Some(entry) => entry
                    .request
                    .clone()
                    .ok_or_else(|| ModelConfigError::MissingModelRequestSpec(model.id.clone()))?,
                None => return Err(ModelConfigError::MissingModelRequestSpec(model.id.clone())),
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

    fn saveProviderToDataStore(&self, provider: &ProviderProfile) -> Result<(), ModelConfigError> {
        let providerKey = self.providerKey(&provider.id);
        let encodedProvider = serde_json::to_string(provider)?;
        self.modelConfigDataStore.edit(|preferences| {
            preferences.set(&providerKey, encodedProvider);
        })?;
        Ok(())
    }

    fn saveProviderList(&self, providerIds: Vec<String>) -> Result<(), ModelConfigError> {
        let encoded = serde_json::to_string(&providerIds)?;
        self.modelConfigDataStore.edit(|preferences| {
            preferences.set(&Self::PROVIDER_LIST_KEY(), encoded);
        })?;
        Ok(())
    }

    fn updateProviderInternal<F>(
        &self,
        providerId: &str,
        transform: F,
    ) -> Result<ProviderProfile, ModelConfigError>
    where
        F: FnOnce(ProviderProfile) -> ProviderProfile,
    {
        let provider = self.loadProviderFromDataStore(providerId)?;
        let updated = transform(provider);
        self.saveProviderToDataStore(&updated)?;
        Ok(updated)
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
            .ok_or_else(|| ModelConfigError::AvailableProviderModelNotFound {
                providerId: provider.id.clone(),
                modelId: modelId.to_string(),
            })
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
    use super::ModelConfigManager;
    use crate::data::model::ModelConfigData::ModelConfigDefaults;

    #[test]
    fn default_ids_are_model_ids() {
        assert_eq!(
            ModelConfigManager::DEFAULT_MODEL_ID,
            ModelConfigDefaults::DEFAULT_MODEL_ID
        );
    }
}
