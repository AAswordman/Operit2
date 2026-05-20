use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::Value;

use crate::api::chat::llmprovider::AIService::{AIService, AiServiceError};
use crate::api::chat::llmprovider::AIServiceFactory::{
    AIServiceFactory, ApiKeyProviderSpec, ProviderCreateParams, ProviderCreateRequest, ProviderServiceKind,
    ProviderServiceSpec,
};
use crate::api::chat::llmprovider::ClaudeProvider::ClaudeProvider;
use crate::api::chat::llmprovider::DeepseekProvider::DeepseekProvider;
use crate::api::chat::llmprovider::DoubaoAIProvider::DoubaoAIProvider;
use crate::api::chat::llmprovider::FourRouterProvider::FourRouterProvider;
use crate::api::chat::llmprovider::GeminiProvider::GeminiProvider;
use crate::api::chat::llmprovider::KimiProvider::KimiProvider;
use crate::api::chat::llmprovider::LlamaProvider::LlamaProvider;
use crate::api::chat::llmprovider::MimoProvider::MimoProvider;
use crate::api::chat::llmprovider::MistralProvider::MistralProvider;
use crate::api::chat::llmprovider::MNNProvider::MNNProvider;
use crate::api::chat::llmprovider::NvidiaAIProvider::NvidiaAIProvider;
use crate::api::chat::llmprovider::NousPortalProvider::NousPortalProvider;
use crate::api::chat::llmprovider::OllamaProvider::OllamaProvider;
use crate::api::chat::llmprovider::OpenAIResponsesProvider::OpenAIResponsesProvider;
use crate::api::chat::llmprovider::OpenRouterProvider::OpenRouterProvider;
use crate::api::chat::llmprovider::OpenAIProvider::OpenAIProvider;
use crate::api::chat::llmprovider::QwenAIProvider::QwenAIProvider;
use crate::api::chat::llmprovider::RateLimitedAIService::RateLimitedAIService;
use crate::api::chat::llmprovider::RateLimiterRegistry::RateLimiterRegistry;
use crate::api::chat::llmprovider::RequestConcurrencyRegistry::RequestConcurrencyRegistry;
use crate::data::model::FunctionType::FunctionType;
use crate::data::model::ModelConfigData::{
    getModelByIndex, getValidModelIndex, ApiProviderType, ModelConfigData,
};
use crate::data::model::ModelParameter::ModelParameter;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;

pub struct MultiServiceManager {
    pub functionalConfigManager: FunctionalConfigManager,
    pub modelConfigManager: ModelConfigManager,
    serviceInstances: HashMap<FunctionType, Box<dyn AIService>>,
    customServiceInstances: HashMap<String, Box<dyn AIService>>,
    isInitialized: bool,
    defaultServiceKey: Option<FunctionType>,
}

impl MultiServiceManager {
    pub fn new(root_dir: PathBuf) -> Self {
        Self {
            functionalConfigManager: FunctionalConfigManager::new(root_dir.clone()),
            modelConfigManager: ModelConfigManager::new(root_dir),
            serviceInstances: HashMap::new(),
            customServiceInstances: HashMap::new(),
            isInitialized: false,
            defaultServiceKey: None,
        }
    }

    pub fn default() -> Self {
        Self::new(ApiPreferences::data_dir())
    }

    pub fn initialize(&mut self) -> Result<(), AiServiceError> {
        self.ensureInitialized()
    }

    fn ensureInitialized(&mut self) -> Result<(), AiServiceError> {
        if self.isInitialized {
            return Ok(());
        }
        self.functionalConfigManager
            .initializeIfNeeded()
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        self.isInitialized = true;
        Ok(())
    }

    pub fn getServiceForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<&mut dyn AIService, AiServiceError> {
        self.ensureInitialized()?;
        if !self.serviceInstances.contains_key(&functionType) {
            let configMapping = self
                .functionalConfigManager
                .getConfigMappingForFunction(functionType.clone())
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
            let config = self
                .modelConfigManager
                .getModelConfigFlow(&configMapping.configId)
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
            let service = self.createServiceFromConfig(config, configMapping.modelIndex)?;
            self.serviceInstances.insert(functionType.clone(), service);
            if functionType == FunctionType::CHAT {
                self.defaultServiceKey = Some(FunctionType::CHAT);
            }
        }
        let service = self
            .serviceInstances
            .get_mut(&functionType)
            .expect("service must exist after creation");
        Ok(service.as_mut())
    }

    pub fn getServiceForConfig(
        &mut self,
        configId: String,
        modelIndex: i32,
    ) -> Result<&mut dyn AIService, AiServiceError> {
        self.ensureInitialized()?;
        let normalizedIndex = modelIndex.max(0);
        let cacheKey = format!("{configId}#{normalizedIndex}");
        if !self.customServiceInstances.contains_key(&cacheKey) {
            let config = self
                .modelConfigManager
                .getModelConfigFlow(&configId)
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
            let service = self.createServiceFromConfig(config, normalizedIndex)?;
            self.customServiceInstances.insert(cacheKey.clone(), service);
        }
        let service = self
            .customServiceInstances
            .get_mut(&cacheKey)
            .expect("custom service must exist after creation");
        Ok(service.as_mut())
    }

    pub fn getDefaultService(&mut self) -> Result<&mut dyn AIService, AiServiceError> {
        self.ensureInitialized()?;
        self.getServiceForFunction(FunctionType::CHAT)
    }

    pub fn createOwnedServiceBundleForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<(ModelConfigData, Vec<ModelParameter<Value>>, Box<dyn AIService>), AiServiceError> {
        self.ensureInitialized()?;
        let configMapping = self
            .functionalConfigManager
            .getConfigMappingForFunction(functionType.clone())
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let config = self
            .modelConfigManager
            .getModelConfigFlow(&configMapping.configId)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let modelParameters = self
            .modelConfigManager
            .getModelParametersForConfig(&configMapping.configId)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let service = self.createServiceFromConfig(config.clone(), configMapping.modelIndex)?;
        Ok((config, modelParameters, service))
    }

    pub fn createOwnedServiceBundleForConfig(
        &mut self,
        configId: String,
        modelIndex: i32,
    ) -> Result<(ModelConfigData, Vec<ModelParameter<Value>>, Box<dyn AIService>), AiServiceError> {
        self.ensureInitialized()?;
        let config = self
            .modelConfigManager
            .getModelConfigFlow(&configId)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let modelParameters = self
            .modelConfigManager
            .getModelParametersForConfig(&configId)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let service = self.createServiceFromConfig(config.clone(), modelIndex)?;
        Ok((config, modelParameters, service))
    }

    pub fn cancelAllStreaming(&mut self) {
        for service in self.serviceInstances.values_mut() {
            service.cancel_streaming();
        }
        for service in self.customServiceInstances.values_mut() {
            service.cancel_streaming();
        }
    }

    pub fn resetAllTokenCounters(&mut self) {
        for service in self.serviceInstances.values_mut() {
            service.reset_token_counts();
        }
        for service in self.customServiceInstances.values_mut() {
            service.reset_token_counts();
        }
    }

    pub fn resetTokenCountersForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<(), AiServiceError> {
        let service = self.getServiceForFunction(functionType)?;
        service.reset_token_counts();
        Ok(())
    }

    pub fn refreshServiceForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<(), AiServiceError> {
        self.ensureInitialized()?;
        if let Some(mut oldService) = self.serviceInstances.remove(&functionType) {
            oldService.cancel_streaming();
            oldService.release();
        }
        if functionType == FunctionType::CHAT {
            self.defaultServiceKey = None;
            for (_, mut service) in self.customServiceInstances.drain() {
                service.cancel_streaming();
                service.release();
            }
        }
        Ok(())
    }

    pub fn refreshAllServices(&mut self) -> Result<(), AiServiceError> {
        self.ensureInitialized()?;
        for (_, mut service) in self.serviceInstances.drain() {
            service.cancel_streaming();
            service.release();
        }
        for (_, mut service) in self.customServiceInstances.drain() {
            service.cancel_streaming();
            service.release();
        }
        self.defaultServiceKey = None;
        Ok(())
    }

    fn createServiceFromConfig(
        &self,
        config: ModelConfigData,
        modelIndex: i32,
    ) -> Result<Box<dyn AIService>, AiServiceError> {
        let actualIndex = getValidModelIndex(&config.modelName, modelIndex);
        let selectedModelName = getModelByIndex(&config.modelName, actualIndex);
        let providerType = ApiProviderType::fromProviderTypeId(&config.apiProviderTypeId)
            .expect("apiProviderTypeId must map to ApiProviderType");
        let requestLimitPerMinute = config.requestLimitPerMinute.max(0);
        let maxConcurrentRequests = config.maxConcurrentRequests.max(0);
        let configId = config.id.clone();
        let spec = AIServiceFactory::create_service(ProviderCreateRequest {
            config,
            selected_model_name: selectedModelName,
            provider_type: providerType.clone(),
            provider_type_id: providerType.name().to_string(),
            tool_pkg_provider_registered: false,
        })?;
        let rawService = self.instantiateService(spec)?;

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

    fn instantiateService(
        &self,
        spec: ProviderServiceSpec,
    ) -> Result<Box<dyn AIService>, AiServiceError> {
        match spec.params {
            ProviderCreateParams::OpenAIProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                enable_tool_call,
                ..
            } => Ok(Box::new(OpenAIProvider::new(
                api_endpoint,
                self.resolveApiKeyProvider(api_key_provider)?,
                model_name,
                provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
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
                self.resolveApiKeyProvider(api_key_provider)?,
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
                self.resolveApiKeyProvider(api_key_provider)?,
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
                self.resolveApiKeyProvider(api_key_provider)?,
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
                self.resolveApiKeyProvider(api_key_provider)?,
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
                self.resolveApiKeyProvider(api_key_provider)?,
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
                self.resolveApiKeyProvider(api_key_provider)?,
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
                self.resolveApiKeyProvider(api_key_provider)?,
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
                self.resolveApiKeyProvider(api_key_provider)?,
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
                self.resolveApiKeyProvider(api_key_provider)?,
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
                self.resolveApiKeyProvider(api_key_provider)?,
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
                self.resolveApiKeyProvider(api_key_provider)?,
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
                self.resolveApiKeyProvider(api_key_provider)?,
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
                self.resolveApiKeyProvider(api_key_provider)?,
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
                self.resolveApiKeyProvider(api_key_provider)?,
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
            _ => Err(AiServiceError::ProviderNotImplemented(format!("{:?}", spec.kind))),
        }
    }

    fn resolveApiKeyProvider(
        &self,
        apiKeyProvider: ApiKeyProviderSpec,
    ) -> Result<String, AiServiceError> {
        match apiKeyProvider {
            ApiKeyProviderSpec::SingleApiKeyProvider { api_key } => Ok(api_key),
            ApiKeyProviderSpec::MultiApiKeyProvider { config_id } => {
                let config = self
                    .modelConfigManager
                    .getModelConfigFlow(&config_id)
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
        self.ensureInitialized()?;
        let configMapping = self
            .functionalConfigManager
            .getConfigMappingForFunction(functionType)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        self.modelConfigManager
            .getModelParametersForConfig(&configMapping.configId)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))
    }

    pub fn getModelConfigForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<ModelConfigData, AiServiceError> {
        self.ensureInitialized()?;
        let configMapping = self
            .functionalConfigManager
            .getConfigMappingForFunction(functionType)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        self.modelConfigManager
            .getModelConfigFlow(&configMapping.configId)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))
    }

    pub fn getModelConfigForConfig(
        &mut self,
        configId: String,
    ) -> Result<ModelConfigData, AiServiceError> {
        self.ensureInitialized()?;
        self.modelConfigManager
            .getModelConfigFlow(&configId)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))
    }

    pub fn getModelParametersForConfig(
        &mut self,
        configId: String,
    ) -> Result<Vec<ModelParameter<Value>>, AiServiceError> {
        self.ensureInitialized()?;
        self.modelConfigManager
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
