use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::sync::Mutex as AsyncMutex;

use crate::api::chat::llmprovider::AIService::{AIService, AiServiceError};
use crate::api::chat::llmprovider::AIServiceFactory::{
    AIServiceFactory, ApiKeyProviderSpec, ProviderCreateParams, ProviderCreateRequest,
    ProviderServiceKind, ProviderServiceSpec,
};
use crate::api::chat::llmprovider::ClaudeProvider::ClaudeProvider;
use crate::api::chat::llmprovider::DeepseekProvider::DeepseekProvider;
use crate::api::chat::llmprovider::DoubaoAIProvider::DoubaoAIProvider;
use crate::api::chat::llmprovider::FourRouterProvider::FourRouterProvider;
use crate::api::chat::llmprovider::GeminiProvider::GeminiProvider;
use crate::api::chat::llmprovider::KimiProvider::KimiProvider;
use crate::api::chat::llmprovider::LlamaProvider::LlamaProvider;
use crate::api::chat::llmprovider::MNNProvider::MNNProvider;
use crate::api::chat::llmprovider::MimoProvider::MimoProvider;
use crate::api::chat::llmprovider::MistralProvider::MistralProvider;
use crate::api::chat::llmprovider::NousPortalProvider::NousPortalProvider;
use crate::api::chat::llmprovider::NvidiaAIProvider::NvidiaAIProvider;
use crate::api::chat::llmprovider::OllamaProvider::OllamaProvider;
use crate::api::chat::llmprovider::OpenAIProvider::OpenAIProvider;
use crate::api::chat::llmprovider::OpenAIResponsesProvider::OpenAIResponsesProvider;
use crate::api::chat::llmprovider::OpenRouterProvider::OpenRouterProvider;
use crate::api::chat::llmprovider::QwenAIProvider::QwenAIProvider;
use crate::api::chat::llmprovider::RateLimitedAIService::RateLimitedAIService;
use crate::api::chat::llmprovider::RateLimiterRegistry::RateLimiterRegistry;
use crate::api::chat::llmprovider::RequestConcurrencyRegistry::RequestConcurrencyRegistry;
use crate::api::chat::llmprovider::ToolPkgJsAiProviderService::ToolPkgJsAiProviderService;
use crate::data::model::FunctionType::FunctionType;
use crate::data::model::ModelConfigData::{
    getModelByIndex, getValidModelIndex, ApiProviderType, ModelConfigData,
};
use crate::data::model::ModelParameter::ModelParameter;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;

pub type SharedAIServiceHandle = Arc<AsyncMutex<Box<dyn AIService>>>;

#[derive(Clone)]
pub struct MultiServiceManager {
    inner: Arc<Mutex<MultiServiceManagerState>>,
}

struct MultiServiceManagerState {
    pub functionalConfigManager: FunctionalConfigManager,
    pub modelConfigManager: ModelConfigManager,
    serviceInstances: HashMap<FunctionType, SharedAIServiceHandle>,
    customServiceInstances: HashMap<String, SharedAIServiceHandle>,
    isInitialized: bool,
    defaultServiceKey: Option<FunctionType>,
}

impl MultiServiceManager {
    pub fn new(root_dir: PathBuf) -> Self {
        Self {
            inner: Arc::new(Mutex::new(MultiServiceManagerState {
                functionalConfigManager: FunctionalConfigManager::new(root_dir.clone()),
                modelConfigManager: ModelConfigManager::new(root_dir),
                serviceInstances: HashMap::new(),
                customServiceInstances: HashMap::new(),
                isInitialized: false,
                defaultServiceKey: None,
            })),
        }
    }

    pub fn default() -> Self {
        Self::new(ApiPreferences::data_dir())
    }

    pub fn initialize(&mut self) -> Result<(), AiServiceError> {
        self.ensureInitialized()
    }

    fn ensureInitialized(&mut self) -> Result<(), AiServiceError> {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)
    }

    fn ensureInitializedLocked(inner: &mut MultiServiceManagerState) -> Result<(), AiServiceError> {
        if inner.isInitialized {
            return Ok(());
        }
        inner
            .functionalConfigManager
            .initializeIfNeeded()
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        inner.isInitialized = true;
        Ok(())
    }

    pub fn getServiceForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<SharedAIServiceHandle, AiServiceError> {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)?;
        if !inner.serviceInstances.contains_key(&functionType) {
            let configMapping = inner
                .functionalConfigManager
                .getConfigMappingForFunction(functionType.clone())
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
            let config = inner
                .modelConfigManager
                .getModelConfigFlow(&configMapping.configId)
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?
                .first()
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
            let service =
                Self::createServiceFromConfigLocked(&inner, config, configMapping.modelIndex)?;
            inner
                .serviceInstances
                .insert(functionType.clone(), Arc::new(AsyncMutex::new(service)));
            if functionType == FunctionType::CHAT {
                inner.defaultServiceKey = Some(FunctionType::CHAT);
            }
        }
        let service = inner
            .serviceInstances
            .get(&functionType)
            .expect("service must exist after creation")
            .clone();
        Ok(service)
    }

    pub fn getServiceForConfig(
        &mut self,
        configId: String,
        modelIndex: i32,
    ) -> Result<SharedAIServiceHandle, AiServiceError> {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)?;
        let normalizedIndex = modelIndex.max(0);
        let cacheKey = format!("{configId}#{normalizedIndex}");
        if !inner.customServiceInstances.contains_key(&cacheKey) {
            let config = inner
                .modelConfigManager
                .getModelConfigFlow(&configId)
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?
                .first()
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
            let service = Self::createServiceFromConfigLocked(&inner, config, normalizedIndex)?;
            inner
                .customServiceInstances
                .insert(cacheKey.clone(), Arc::new(AsyncMutex::new(service)));
        }
        let service = inner
            .customServiceInstances
            .get(&cacheKey)
            .expect("custom service must exist after creation")
            .clone();
        Ok(service)
    }

    pub fn getDefaultService(&mut self) -> Result<SharedAIServiceHandle, AiServiceError> {
        self.getServiceForFunction(FunctionType::CHAT)
    }

    pub fn getServiceBundleForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<
        (
            ModelConfigData,
            Vec<ModelParameter<Value>>,
            SharedAIServiceHandle,
        ),
        AiServiceError,
    > {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)?;
        let configMapping = inner
            .functionalConfigManager
            .getConfigMappingForFunction(functionType.clone())
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let config = inner
            .modelConfigManager
            .getModelConfigFlow(&configMapping.configId)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?
            .first()
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let modelParameters = inner
            .modelConfigManager
            .getModelParametersForConfig(&configMapping.configId)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        if !inner.serviceInstances.contains_key(&functionType) {
            let service = Self::createServiceFromConfigLocked(
                &inner,
                config.clone(),
                configMapping.modelIndex,
            )?;
            inner
                .serviceInstances
                .insert(functionType.clone(), Arc::new(AsyncMutex::new(service)));
            if functionType == FunctionType::CHAT {
                inner.defaultServiceKey = Some(FunctionType::CHAT);
            }
        }
        let service = inner
            .serviceInstances
            .get(&functionType)
            .expect("service must exist after creation")
            .clone();
        Ok((config, modelParameters, service))
    }

    pub fn getServiceBundleForConfig(
        &mut self,
        configId: String,
        modelIndex: i32,
    ) -> Result<
        (
            ModelConfigData,
            Vec<ModelParameter<Value>>,
            SharedAIServiceHandle,
        ),
        AiServiceError,
    > {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)?;
        let config = inner
            .modelConfigManager
            .getModelConfigFlow(&configId)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?
            .first()
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let modelParameters = inner
            .modelConfigManager
            .getModelParametersForConfig(&configId)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let normalizedIndex = modelIndex.max(0);
        let cacheKey = format!("{configId}#{normalizedIndex}");
        if !inner.customServiceInstances.contains_key(&cacheKey) {
            let service =
                Self::createServiceFromConfigLocked(&inner, config.clone(), normalizedIndex)?;
            inner
                .customServiceInstances
                .insert(cacheKey.clone(), Arc::new(AsyncMutex::new(service)));
        }
        let service = inner
            .customServiceInstances
            .get(&cacheKey)
            .expect("custom service must exist after creation")
            .clone();
        Ok((config, modelParameters, service))
    }

    pub fn createTransientServiceBundleForConfigData(
        &mut self,
        config: ModelConfigData,
        modelIndex: i32,
    ) -> Result<
        (
            ModelConfigData,
            Vec<ModelParameter<Value>>,
            Box<dyn AIService>,
        ),
        AiServiceError,
    > {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)?;
        let modelParameters = inner
            .modelConfigManager
            .getModelParametersForConfig(&config.id)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let normalizedIndex = modelIndex.max(0);
        let service = Self::createServiceFromConfigLocked(&inner, config.clone(), normalizedIndex)?;
        Ok((config, modelParameters, service))
    }

    pub fn cancelAllStreaming(&mut self) {
        let inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        for service in inner.serviceInstances.values() {
            service.blocking_lock().cancel_streaming();
        }
        for service in inner.customServiceInstances.values() {
            service.blocking_lock().cancel_streaming();
        }
    }

    pub fn resetAllTokenCounters(&mut self) {
        let inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        for service in inner.serviceInstances.values() {
            service.blocking_lock().reset_token_counts();
        }
        for service in inner.customServiceInstances.values() {
            service.blocking_lock().reset_token_counts();
        }
    }

    pub fn resetTokenCountersForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<(), AiServiceError> {
        let service = self.getServiceForFunction(functionType)?;
        service.blocking_lock().reset_token_counts();
        Ok(())
    }

    pub fn refreshServiceForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<(), AiServiceError> {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)?;
        if let Some(oldService) = inner.serviceInstances.remove(&functionType) {
            let mut service = oldService.blocking_lock();
            service.cancel_streaming();
            service.release();
        }
        if functionType == FunctionType::CHAT {
            inner.defaultServiceKey = None;
            for (_, oldService) in inner.customServiceInstances.drain() {
                let mut service = oldService.blocking_lock();
                service.cancel_streaming();
                service.release();
            }
        }
        Ok(())
    }

    pub fn refreshAllServices(&mut self) -> Result<(), AiServiceError> {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)?;
        for (_, oldService) in inner.serviceInstances.drain() {
            let mut service = oldService.blocking_lock();
            service.cancel_streaming();
            service.release();
        }
        for (_, oldService) in inner.customServiceInstances.drain() {
            let mut service = oldService.blocking_lock();
            service.cancel_streaming();
            service.release();
        }
        inner.defaultServiceKey = None;
        Ok(())
    }

    fn createServiceFromConfigLocked(
        inner: &MultiServiceManagerState,
        config: ModelConfigData,
        modelIndex: i32,
    ) -> Result<Box<dyn AIService>, AiServiceError> {
        let actualIndex = getValidModelIndex(&config.modelName, modelIndex);
        let selectedModelName = getModelByIndex(&config.modelName, actualIndex);
        let providerTypeId = config.apiProviderTypeId.trim().to_string();
        let toolPkgProviderRegistered =
            crate::plugins::toolpkg::ToolPkgAiProviderRegistry::ToolPkgAiProviderRegistry::get(
                &providerTypeId,
            )
            .is_some();
        let providerType = if toolPkgProviderRegistered {
            config.apiProviderType.clone()
        } else {
            ApiProviderType::fromProviderTypeId(&providerTypeId)
                .expect("apiProviderTypeId must map to ApiProviderType")
        };
        let requestLimitPerMinute = config.requestLimitPerMinute.max(0);
        let maxConcurrentRequests = config.maxConcurrentRequests.max(0);
        let configId = config.id.clone();
        let spec = AIServiceFactory::create_service(ProviderCreateRequest {
            config,
            selected_model_name: selectedModelName,
            provider_type: providerType.clone(),
            provider_type_id: providerTypeId,
            tool_pkg_provider_registered: toolPkgProviderRegistered,
        })?;
        let rawService = Self::instantiateServiceLocked(inner, spec)?;

        if requestLimitPerMinute == 0 && maxConcurrentRequests == 0 {
            return Ok(rawService);
        }

        let limiter = if requestLimitPerMinute > 0 {
            Some(RateLimiterRegistry::getOrCreate(
                &configId,
                requestLimitPerMinute,
            ))
        } else {
            None
        };

        let concurrencySemaphore = if maxConcurrentRequests > 0 {
            Some(RequestConcurrencyRegistry::getOrCreate(
                &configId,
                maxConcurrentRequests,
            ))
        } else {
            None
        };

        Ok(Box::new(RateLimitedAIService::new(
            rawService,
            limiter,
            concurrencySemaphore,
        )))
    }

    fn instantiateServiceLocked(
        inner: &MultiServiceManagerState,
        spec: ProviderServiceSpec,
    ) -> Result<Box<dyn AIService>, AiServiceError> {
        match spec.params {
            ProviderCreateParams::OpenAIProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            } => Ok(Box::new(OpenAIProvider::new_with_capabilities(
                api_endpoint,
                Self::resolveApiKeyProviderLocked(inner, api_key_provider)?,
                model_name,
                provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ))),
            ProviderCreateParams::DeepseekProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                enable_tool_call,
                ..
            } => Ok(Box::new(DeepseekProvider::new(
                api_endpoint,
                Self::resolveApiKeyProviderLocked(inner, api_key_provider)?,
                model_name,
                provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
                enable_tool_call,
            ))),
            ProviderCreateParams::OpenAIResponsesProvider {
                responses_api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                responses_provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            } => Ok(Box::new(OpenAIResponsesProvider::new(
                responses_api_endpoint,
                Self::resolveApiKeyProviderLocked(inner, api_key_provider)?,
                model_name,
                responses_provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ))),
            ProviderCreateParams::ClaudeProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                enable_tool_call,
            } => Ok(Box::new(ClaudeProvider::new(
                api_endpoint,
                Self::resolveApiKeyProviderLocked(inner, api_key_provider)?,
                model_name,
                provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
                enable_tool_call,
            ))),
            ProviderCreateParams::GeminiProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                enable_tool_call,
                enable_google_search,
            } => Ok(Box::new(GeminiProvider::new(
                api_endpoint,
                Self::resolveApiKeyProviderLocked(inner, api_key_provider)?,
                model_name,
                provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
                enable_google_search,
                enable_tool_call,
            ))),
            ProviderCreateParams::OllamaProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
                ..
            } => Ok(Box::new(OllamaProvider::new(
                api_endpoint,
                Self::resolveApiKeyProviderLocked(inner, api_key_provider)?,
                model_name,
                provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ))),
            ProviderCreateParams::KimiProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
                ..
            } => Ok(Box::new(KimiProvider::new(
                api_endpoint,
                Self::resolveApiKeyProviderLocked(inner, api_key_provider)?,
                model_name,
                provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ))),
            ProviderCreateParams::MimoProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
                ..
            } => Ok(Box::new(MimoProvider::new(
                api_endpoint,
                Self::resolveApiKeyProviderLocked(inner, api_key_provider)?,
                model_name,
                provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ))),
            ProviderCreateParams::MistralProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
                ..
            } => Ok(Box::new(MistralProvider::new(
                api_endpoint,
                Self::resolveApiKeyProviderLocked(inner, api_key_provider)?,
                model_name,
                provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ))),
            ProviderCreateParams::OpenRouterProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
                ..
            } => Ok(Box::new(OpenRouterProvider::new(
                api_endpoint,
                Self::resolveApiKeyProviderLocked(inner, api_key_provider)?,
                model_name,
                provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ))),
            ProviderCreateParams::FourRouterProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
                ..
            } => Ok(Box::new(FourRouterProvider::new(
                api_endpoint,
                Self::resolveApiKeyProviderLocked(inner, api_key_provider)?,
                model_name,
                provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ))),
            ProviderCreateParams::NousPortalProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
                ..
            } => Ok(Box::new(NousPortalProvider::new(
                api_endpoint,
                Self::resolveApiKeyProviderLocked(inner, api_key_provider)?,
                model_name,
                provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ))),
            ProviderCreateParams::DoubaoAIProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
                ..
            } => Ok(Box::new(DoubaoAIProvider::new(
                api_endpoint,
                Self::resolveApiKeyProviderLocked(inner, api_key_provider)?,
                model_name,
                provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ))),
            ProviderCreateParams::NvidiaAIProvider {
                api_endpoint,
                model_name,
                api_key_provider,
                custom_headers,
                provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
                ..
            } => Ok(Box::new(NvidiaAIProvider::new(
                api_endpoint,
                Self::resolveApiKeyProviderLocked(inner, api_key_provider)?,
                model_name,
                provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ))),
            ProviderCreateParams::QwenAIProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                qwen_provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
                ..
            } => Ok(Box::new(QwenAIProvider::new(
                api_endpoint,
                Self::resolveApiKeyProviderLocked(inner, api_key_provider)?,
                model_name,
                qwen_provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ))),
            ProviderCreateParams::MNNProvider { .. } => Ok(Box::new(MNNProvider)),
            ProviderCreateParams::LlamaProvider { .. } => Ok(Box::new(LlamaProvider)),
            ProviderCreateParams::ToolPkgJsAiProviderService {
                provider_type_id,
                config_id,
            } => {
                let provider = crate::plugins::toolpkg::ToolPkgAiProviderRegistry::ToolPkgAiProviderRegistry::get(&provider_type_id)
                    .ok_or_else(|| AiServiceError::ProviderNotImplemented(provider_type_id.clone()))?;
                let config = inner
                    .modelConfigManager
                    .getModelConfigFlow(&config_id)
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?
                    .first()
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
                Ok(Box::new(ToolPkgJsAiProviderService::new(config, provider)))
            }
            _ => Err(AiServiceError::ProviderNotImplemented(format!(
                "{:?}",
                spec.kind
            ))),
        }
    }

    fn resolveApiKeyProviderLocked(
        inner: &MultiServiceManagerState,
        apiKeyProvider: ApiKeyProviderSpec,
    ) -> Result<String, AiServiceError> {
        match apiKeyProvider {
            ApiKeyProviderSpec::SingleApiKeyProvider { api_key } => Ok(api_key),
            ApiKeyProviderSpec::MultiApiKeyProvider { config_id } => {
                let config = inner
                    .modelConfigManager
                    .getModelConfigFlow(&config_id)
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?
                    .first()
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
                let index = usize::try_from(config.currentKeyIndex)
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
                let keyInfo = config.apiKeyPool.get(index).ok_or_else(|| {
                    AiServiceError::RequestFailed(format!(
                        "apiKeyPool index out of range: configId={config_id}, index={index}"
                    ))
                })?;
                if !keyInfo.isEnabled {
                    return Err(AiServiceError::RequestFailed(format!(
                        "apiKeyPool entry disabled: configId={config_id}, index={index}"
                    )));
                }
                Ok(keyInfo.key.clone())
            }
        }
    }

    pub fn getModelParametersForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<Vec<ModelParameter<Value>>, AiServiceError> {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)?;
        let configMapping = inner
            .functionalConfigManager
            .getConfigMappingForFunction(functionType)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        inner
            .modelConfigManager
            .getModelParametersForConfig(&configMapping.configId)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))
    }

    pub fn getModelConfigForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<ModelConfigData, AiServiceError> {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)?;
        let configMapping = inner
            .functionalConfigManager
            .getConfigMappingForFunction(functionType)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        inner
            .modelConfigManager
            .getModelConfigFlow(&configMapping.configId)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?
            .first()
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))
    }

    pub fn getModelConfigForConfig(
        &mut self,
        configId: String,
    ) -> Result<ModelConfigData, AiServiceError> {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)?;
        inner
            .modelConfigManager
            .getModelConfigFlow(&configId)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?
            .first()
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))
    }

    pub fn getModelParametersForConfig(
        &mut self,
        configId: String,
    ) -> Result<Vec<ModelParameter<Value>>, AiServiceError> {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)?;
        inner
            .modelConfigManager
            .getModelParametersForConfig(&configId)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))
    }

    pub fn hasImageRecognitionConfigured(&mut self) -> Result<bool, AiServiceError> {
        let config = self.getModelConfigForFunction(FunctionType::IMAGE_RECOGNITION)?;
        Ok(config.enableDirectImageProcessing)
    }

    pub fn hasAudioRecognitionConfigured(&mut self) -> Result<bool, AiServiceError> {
        let config = self.getModelConfigForFunction(FunctionType::AUDIO_RECOGNITION)?;
        Ok(config.enableDirectAudioProcessing)
    }

    pub fn hasVideoRecognitionConfigured(&mut self) -> Result<bool, AiServiceError> {
        let config = self.getModelConfigForFunction(FunctionType::VIDEO_RECOGNITION)?;
        Ok(config.enableDirectVideoProcessing)
    }
}
