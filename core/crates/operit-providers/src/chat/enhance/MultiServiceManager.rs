use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::sync::Mutex as AsyncMutex;

use crate::chat::llmprovider::AIService::{AIService, AiServiceError};
use crate::chat::llmprovider::AIServiceFactory::{
    AIServiceFactory, ApiKeyProviderSpec, ProviderCreateParams, ProviderCreateRequest,
    ProviderServiceKind, ProviderServiceSpec,
};
use crate::chat::llmprovider::ClaudeProvider::ClaudeProvider;
use crate::chat::llmprovider::DeepseekProvider::DeepseekProvider;
use crate::chat::llmprovider::DoubaoAIProvider::DoubaoAIProvider;
use crate::chat::llmprovider::FourRouterProvider::FourRouterProvider;
use crate::chat::llmprovider::GeminiProvider::GeminiProvider;
use crate::chat::llmprovider::KimiProvider::KimiProvider;
use crate::chat::llmprovider::MimoProvider::MimoProvider;
use crate::chat::llmprovider::MistralProvider::MistralProvider;
use crate::chat::llmprovider::NousPortalProvider::NousPortalProvider;
use crate::chat::llmprovider::NvidiaAIProvider::NvidiaAIProvider;
use crate::chat::llmprovider::OllamaProvider::OllamaProvider;
use crate::chat::llmprovider::OpenAIProvider::OpenAIProvider;
use crate::chat::llmprovider::OpenAIResponsesProvider::OpenAIResponsesProvider;
use crate::chat::llmprovider::OpenRouterProvider::OpenRouterProvider;
use crate::chat::llmprovider::QwenAIProvider::QwenAIProvider;
use crate::chat::llmprovider::RateLimitedAIService::RateLimitedAIService;
use crate::chat::llmprovider::RateLimiterRegistry::RateLimiterRegistry;
use crate::chat::llmprovider::RequestConcurrencyRegistry::RequestConcurrencyRegistry;
use crate::chat::llmprovider::ToolPkgJsAiProviderService::ToolPkgJsAiProviderService;
use crate::runtime_support::ProviderRuntimeContext;
use operit_model::FunctionType::FunctionType;
use operit_model::ModelConfigData::{ApiProviderType, ResolvedModelConfig};
use operit_model::ModelParameter::ModelParameter;

/// Shared, asynchronously locked handle around a concrete model provider service.
pub type SharedAIServiceHandle = Arc<AsyncMutex<Box<dyn AIService>>>;

/// Owns model-provider service instances for function bindings and explicit model requests.
#[derive(Clone)]
pub struct MultiServiceManager {
    inner: Arc<Mutex<MultiServiceManagerState>>,
}

/// Tracks the provider and model backing a function-bound service instance.
struct FunctionServiceInstance {
    providerId: String,
    modelId: String,
    service: SharedAIServiceHandle,
}

/// Mutable service registry state protected by the manager mutex.
struct MultiServiceManagerState {
    pub rootDir: PathBuf,
    runtime_context: ProviderRuntimeContext,
    serviceInstances: HashMap<FunctionType, FunctionServiceInstance>,
    modelServiceInstances: HashMap<String, SharedAIServiceHandle>,
    isInitialized: bool,
    defaultServiceKey: Option<FunctionType>,
}

impl MultiServiceManager {
    /// Creates a manager rooted at the supplied runtime data directory.
    pub fn new(root_dir: PathBuf, runtime_context: ProviderRuntimeContext) -> Self {
        Self {
            inner: Arc::new(Mutex::new(MultiServiceManagerState {
                rootDir: root_dir,
                runtime_context,
                serviceInstances: HashMap::new(),
                modelServiceInstances: HashMap::new(),
                isInitialized: false,
                defaultServiceKey: None,
            })),
        }
    }

    /// Creates a manager rooted at the runtime support data directory.
    pub fn from_runtime_context(
        runtime_context: ProviderRuntimeContext,
    ) -> Result<Self, AiServiceError> {
        let root_dir = runtime_context
            .support()
            .dataDir()
            .map_err(AiServiceError::RequestFailed)?;
        Ok(Self::new(root_dir, runtime_context))
    }

    /// Initializes preference-backed model bindings before service lookup.
    pub fn initialize(&mut self) -> Result<(), AiServiceError> {
        self.ensureInitialized()
    }

    /// Ensures manager state is initialized through the outer lock.
    fn ensureInitialized(&mut self) -> Result<(), AiServiceError> {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)
    }

    /// Ensures manager state is initialized while the state lock is already held.
    fn ensureInitializedLocked(inner: &mut MultiServiceManagerState) -> Result<(), AiServiceError> {
        if inner.isInitialized {
            return Ok(());
        }
        inner
            .runtime_context
            .support()
            .initializeFunctionModelBindings(inner.rootDir.clone())
            .map_err(AiServiceError::RequestFailed)?;
        inner.isInitialized = true;
        Ok(())
    }

    /// Returns a cached or newly created service for a functional model binding.
    pub fn getServiceForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<SharedAIServiceHandle, AiServiceError> {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)?;
        let binding = inner
            .runtime_context
            .support()
            .modelBindingForFunction(inner.rootDir.clone(), functionType.clone())
            .map_err(AiServiceError::RequestFailed)?;
        if !Self::functionServiceMatchesBinding(
            inner.serviceInstances.get(&functionType),
            &binding.providerId,
            &binding.modelId,
        ) {
            let config = inner
                .runtime_context
                .support()
                .resolvedModelConfig(inner.rootDir.clone(), &binding.providerId, &binding.modelId)
                .map_err(AiServiceError::RequestFailed)?;
            let service = Self::createServiceFromResolvedConfigLocked(&inner, config)?;
            inner.serviceInstances.insert(
                functionType.clone(),
                FunctionServiceInstance {
                    providerId: binding.providerId.clone(),
                    modelId: binding.modelId.clone(),
                    service: Arc::new(AsyncMutex::new(service)),
                },
            );
            if functionType == FunctionType::CHAT {
                inner.defaultServiceKey = Some(FunctionType::CHAT);
            }
        }
        let service = inner
            .serviceInstances
            .get(&functionType)
            .expect("service must exist after creation")
            .service
            .clone();
        Ok(service)
    }

    /// Returns a cached or newly created service for a concrete provider and model.
    pub fn getServiceForModel(
        &mut self,
        providerId: String,
        modelId: String,
    ) -> Result<SharedAIServiceHandle, AiServiceError> {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)?;
        let serviceKey = Self::modelServiceKey(&providerId, &modelId);
        if !inner.modelServiceInstances.contains_key(&serviceKey) {
            let config = inner
                .runtime_context
                .support()
                .resolvedModelConfig(inner.rootDir.clone(), &providerId, &modelId)
                .map_err(AiServiceError::RequestFailed)?;
            let service = Self::createServiceFromResolvedConfigLocked(&inner, config)?;
            inner
                .modelServiceInstances
                .insert(serviceKey.clone(), Arc::new(AsyncMutex::new(service)));
        }
        let service = inner
            .modelServiceInstances
            .get(&serviceKey)
            .expect("model service must exist after creation")
            .clone();
        Ok(service)
    }

    /// Returns the chat function service.
    pub fn getDefaultService(&mut self) -> Result<SharedAIServiceHandle, AiServiceError> {
        self.getServiceForFunction(FunctionType::CHAT)
    }

    /// Returns the resolved config, parameters, and service for a function binding.
    pub fn getServiceBundleForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<
        (
            ResolvedModelConfig,
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
        let binding = inner
            .runtime_context
            .support()
            .modelBindingForFunction(inner.rootDir.clone(), functionType.clone())
            .map_err(AiServiceError::RequestFailed)?;
        let config = inner
            .runtime_context
            .support()
            .resolvedModelConfig(inner.rootDir.clone(), &binding.providerId, &binding.modelId)
            .map_err(AiServiceError::RequestFailed)?;
        let modelParameters = config.parameters.clone();
        if !Self::functionServiceMatchesBinding(
            inner.serviceInstances.get(&functionType),
            &binding.providerId,
            &binding.modelId,
        ) {
            let service = Self::createServiceFromResolvedConfigLocked(&inner, config.clone())?;
            inner.serviceInstances.insert(
                functionType.clone(),
                FunctionServiceInstance {
                    providerId: binding.providerId.clone(),
                    modelId: binding.modelId.clone(),
                    service: Arc::new(AsyncMutex::new(service)),
                },
            );
            if functionType == FunctionType::CHAT {
                inner.defaultServiceKey = Some(FunctionType::CHAT);
            }
        }
        let service = inner
            .serviceInstances
            .get(&functionType)
            .expect("service must exist after creation")
            .service
            .clone();
        Ok((config, modelParameters, service))
    }

    /// Returns the resolved config, parameters, and service for a concrete model.
    pub fn getServiceBundleForModel(
        &mut self,
        providerId: String,
        modelId: String,
    ) -> Result<
        (
            ResolvedModelConfig,
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
        let serviceKey = Self::modelServiceKey(&providerId, &modelId);
        let config = inner
            .runtime_context
            .support()
            .resolvedModelConfig(inner.rootDir.clone(), &providerId, &modelId)
            .map_err(AiServiceError::RequestFailed)?;
        let modelParameters = config.parameters.clone();
        if !inner.modelServiceInstances.contains_key(&serviceKey) {
            let service = Self::createServiceFromResolvedConfigLocked(&inner, config.clone())?;
            inner
                .modelServiceInstances
                .insert(serviceKey.clone(), Arc::new(AsyncMutex::new(service)));
        }
        let service = inner
            .modelServiceInstances
            .get(&serviceKey)
            .expect("model service must exist after creation")
            .clone();
        Ok((config, modelParameters, service))
    }

    /// Creates a non-cached service bundle for a concrete model.
    pub fn createTransientServiceBundleForModel(
        &mut self,
        providerId: String,
        modelId: String,
    ) -> Result<
        (
            ResolvedModelConfig,
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
        let config = inner
            .runtime_context
            .support()
            .resolvedModelConfig(inner.rootDir.clone(), &providerId, &modelId)
            .map_err(AiServiceError::RequestFailed)?;
        let modelParameters = config.parameters.clone();
        let service = Self::createServiceFromResolvedConfigLocked(&inner, config.clone())?;
        Ok((config, modelParameters, service))
    }

    /// Cancels all active streams across cached services.
    pub async fn cancelAllStreaming(&mut self) {
        let services = {
            let inner = self
                .inner
                .lock()
                .expect("MultiServiceManager mutex poisoned");
            Self::collectServiceHandlesLocked(&inner)
        };
        for service in services {
            service.lock().await.cancel_streaming();
        }
    }

    /// Resets token counters across cached services.
    pub async fn resetAllTokenCounters(&mut self) {
        let services = {
            let inner = self
                .inner
                .lock()
                .expect("MultiServiceManager mutex poisoned");
            Self::collectServiceHandlesLocked(&inner)
        };
        for service in services {
            service.lock().await.reset_token_counts();
        }
    }

    /// Resets token counters for the service bound to one function.
    pub async fn resetTokenCountersForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<(), AiServiceError> {
        let service = self.getServiceForFunction(functionType)?;
        service.lock().await.reset_token_counts();
        Ok(())
    }

    /// Drops and releases the cached service for one function binding.
    pub async fn refreshServiceForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<(), AiServiceError> {
        let (oldService, oldModelServices) = {
            let mut inner = self
                .inner
                .lock()
                .expect("MultiServiceManager mutex poisoned");
            Self::ensureInitializedLocked(&mut inner)?;
            let oldService = inner
                .serviceInstances
                .remove(&functionType)
                .map(|instance| instance.service);
            let oldModelServices = if functionType == FunctionType::CHAT {
                inner.defaultServiceKey = None;
                inner
                    .modelServiceInstances
                    .drain()
                    .map(|(_, oldService)| oldService)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            (oldService, oldModelServices)
        };
        if let Some(oldService) = oldService {
            let mut service = oldService.lock().await;
            service.cancel_streaming();
            service.release();
        }
        for oldService in oldModelServices {
            let mut service = oldService.lock().await;
            service.cancel_streaming();
            service.release();
        }
        Ok(())
    }

    /// Drops and releases the cached service for one concrete provider/model pair.
    pub async fn refreshServiceForModel(
        &mut self,
        providerId: String,
        modelId: String,
    ) -> Result<(), AiServiceError> {
        let oldService = {
            let mut inner = self
                .inner
                .lock()
                .expect("MultiServiceManager mutex poisoned");
            Self::ensureInitializedLocked(&mut inner)?;
            let serviceKey = Self::modelServiceKey(&providerId, &modelId);
            inner.modelServiceInstances.remove(&serviceKey)
        };
        if let Some(oldService) = oldService {
            let mut service = oldService.lock().await;
            service.cancel_streaming();
            service.release();
        }
        Ok(())
    }

    /// Drops and releases every cached service.
    pub async fn refreshAllServices(&mut self) -> Result<(), AiServiceError> {
        let oldServices = {
            let mut inner = self
                .inner
                .lock()
                .expect("MultiServiceManager mutex poisoned");
            Self::ensureInitializedLocked(&mut inner)?;
            let mut oldServices = inner
                .serviceInstances
                .drain()
                .map(|(_, oldService)| oldService.service)
                .collect::<Vec<_>>();
            oldServices.extend(
                inner
                    .modelServiceInstances
                    .drain()
                    .map(|(_, oldService)| oldService),
            );
            inner.defaultServiceKey = None;
            oldServices
        };
        for oldService in oldServices {
            let mut service = oldService.lock().await;
            service.cancel_streaming();
            service.release();
        }
        Ok(())
    }

    /// Checks whether a cached function service still matches the configured binding.
    fn functionServiceMatchesBinding(
        instance: Option<&FunctionServiceInstance>,
        providerId: &str,
        modelId: &str,
    ) -> bool {
        match instance {
            Some(instance) => instance.providerId == providerId && instance.modelId == modelId,
            None => false,
        }
    }

    /// Collects unique cached service handles while the state lock is held.
    fn collectServiceHandlesLocked(inner: &MultiServiceManagerState) -> Vec<SharedAIServiceHandle> {
        let mut services = Vec::new();
        for instance in inner.serviceInstances.values() {
            if !services
                .iter()
                .any(|existing| Arc::ptr_eq(existing, &instance.service))
            {
                services.push(instance.service.clone());
            }
        }
        for service in inner.modelServiceInstances.values() {
            if !services
                .iter()
                .any(|existing| Arc::ptr_eq(existing, service))
            {
                services.push(service.clone());
            }
        }
        services
    }

    /// Builds an AI service from a resolved model config and wraps configured limits.
    fn createServiceFromResolvedConfigLocked(
        inner: &MultiServiceManagerState,
        config: ResolvedModelConfig,
    ) -> Result<Box<dyn AIService>, AiServiceError> {
        let providerTypeId = config.apiProviderTypeId.trim().to_string();
        let toolPkgProviderRegistered = inner
            .runtime_context
            .support()
            .hasToolPkgAiProvider(&providerTypeId);
        let providerType = match toolPkgProviderRegistered {
            true => config.apiProviderType.clone(),
            false => ApiProviderType::fromProviderTypeId(&providerTypeId)
                .expect("apiProviderTypeId must map to ApiProviderType"),
        };
        let requestLimitPerMinute = config.requestLimitPerMinute.max(0);
        let maxConcurrentRequests = config.maxConcurrentRequests.max(0);
        let modelId = config.modelId.clone();
        let spec = AIServiceFactory::create_service(ProviderCreateRequest {
            config,
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
                &modelId,
                requestLimitPerMinute,
            ))
        } else {
            None
        };

        let concurrencySemaphore = if maxConcurrentRequests > 0 {
            Some(RequestConcurrencyRegistry::getOrCreate(
                &modelId,
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

    /// Instantiates the concrete provider implementation described by a factory spec.
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
                inner.runtime_context.clone(),
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
                inner.runtime_context.clone(),
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
                builtin_tools,
            } => Ok(Box::new(GeminiProvider::new(
                api_endpoint,
                Self::resolveApiKeyProviderLocked(inner, api_key_provider)?,
                model_name,
                provider_type.name().to_string(),
                custom_headers.into_iter().collect(),
                builtin_tools,
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
                inner.runtime_context.clone(),
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
                inner.runtime_context.clone(),
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
                inner.runtime_context.clone(),
            ))),
            ProviderCreateParams::ToolPkgJsAiProviderService {
                provider_type_id,
                provider_id,
                model_id,
            } => {
                let provider = inner
                    .runtime_context
                    .support()
                    .toolPkgAiProvider(&provider_type_id)
                    .ok_or_else(|| {
                        AiServiceError::ProviderNotImplemented(provider_type_id.clone())
                    })?;
                let config = inner
                    .runtime_context
                    .support()
                    .resolvedModelConfig(inner.rootDir.clone(), &provider_id, &model_id)
                    .map_err(AiServiceError::RequestFailed)?;
                Ok(Box::new(ToolPkgJsAiProviderService::new(
                    config,
                    provider,
                    inner.runtime_context.clone(),
                )))
            }
        }
    }

    /// Resolves the API-key provider spec into the concrete key source used by providers.
    fn resolveApiKeyProviderLocked(
        inner: &MultiServiceManagerState,
        apiKeyProvider: ApiKeyProviderSpec,
    ) -> Result<String, AiServiceError> {
        match apiKeyProvider {
            ApiKeyProviderSpec::SingleApiKeyProvider { api_key } => Ok(api_key),
            ApiKeyProviderSpec::MultiApiKeyProvider { provider_id } => {
                let provider = inner
                    .runtime_context
                    .support()
                    .providerProfile(inner.rootDir.clone(), &provider_id)
                    .map_err(AiServiceError::RequestFailed)?;
                let index = usize::try_from(provider.currentKeyIndex)
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
                let keyInfo = provider.apiKeyPool.get(index).ok_or_else(|| {
                    AiServiceError::RequestFailed(format!(
                        "apiKeyPool index out of range: providerId={provider_id}, index={index}"
                    ))
                })?;
                if !keyInfo.isEnabled {
                    return Err(AiServiceError::RequestFailed(format!(
                        "apiKeyPool entry disabled: providerId={provider_id}, index={index}"
                    )));
                }
                Ok(keyInfo.key.clone())
            }
        }
    }

    /// Returns model parameters for a function-bound model.
    pub fn getModelParametersForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<Vec<ModelParameter<Value>>, AiServiceError> {
        let config = self.getModelConfigForFunction(functionType)?;
        Ok(config.parameters)
    }

    /// Returns resolved model config for a function binding.
    pub fn getModelConfigForFunction(
        &mut self,
        functionType: FunctionType,
    ) -> Result<ResolvedModelConfig, AiServiceError> {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)?;
        let binding = inner
            .runtime_context
            .support()
            .modelBindingForFunction(inner.rootDir.clone(), functionType)
            .map_err(AiServiceError::RequestFailed)?;
        inner
            .runtime_context
            .support()
            .resolvedModelConfig(inner.rootDir.clone(), &binding.providerId, &binding.modelId)
            .map_err(AiServiceError::RequestFailed)
    }

    /// Returns resolved model config for a concrete provider/model pair.
    pub fn getModelConfigForModel(
        &mut self,
        providerId: String,
        modelId: String,
    ) -> Result<ResolvedModelConfig, AiServiceError> {
        let mut inner = self
            .inner
            .lock()
            .expect("MultiServiceManager mutex poisoned");
        Self::ensureInitializedLocked(&mut inner)?;
        inner
            .runtime_context
            .support()
            .resolvedModelConfig(inner.rootDir.clone(), &providerId, &modelId)
            .map_err(AiServiceError::RequestFailed)
    }

    /// Returns model parameters for a concrete provider/model pair.
    pub fn getModelParametersForModel(
        &mut self,
        providerId: String,
        modelId: String,
    ) -> Result<Vec<ModelParameter<Value>>, AiServiceError> {
        let config = self.getModelConfigForModel(providerId, modelId)?;
        Ok(config.parameters)
    }

    /// Builds the cache key for a concrete provider/model service.
    fn modelServiceKey(providerId: &str, modelId: &str) -> String {
        format!("{providerId}:{modelId}")
    }

    /// Reports whether the chat model binding supports direct image input.
    pub fn hasImageRecognitionConfigured(&mut self) -> Result<bool, AiServiceError> {
        let config = self.getModelConfigForFunction(FunctionType::IMAGE_RECOGNITION)?;
        Ok(config.capabilities.directImage)
    }

    /// Reports whether the chat model binding supports direct audio input.
    pub fn hasAudioRecognitionConfigured(&mut self) -> Result<bool, AiServiceError> {
        let config = self.getModelConfigForFunction(FunctionType::AUDIO_RECOGNITION)?;
        Ok(config.capabilities.directAudio)
    }

    /// Reports whether the chat model binding supports direct video input.
    pub fn hasVideoRecognitionConfigured(&mut self) -> Result<bool, AiServiceError> {
        let config = self.getModelConfigForFunction(FunctionType::VIDEO_RECOGNITION)?;
        Ok(config.capabilities.directVideo)
    }
}
