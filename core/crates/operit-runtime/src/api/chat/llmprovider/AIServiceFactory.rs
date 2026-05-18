use std::collections::BTreeMap;

use super::AIService::AiServiceError;
use crate::data::model::ModelConfigData::{ApiProviderType, ModelConfigData};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlmRequestTraceContext {
    pub request_id: String,
    pub provider: String,
    pub model: String,
    pub stream: bool,
    pub attempt: i32,
    pub endpoint_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderServiceKind {
    OpenAIProvider,
    OpenAIResponsesProvider,
    ClaudeProvider,
    GeminiProvider,
    OllamaProvider,
    MNNProvider,
    LlamaProvider,
    QwenAIProvider,
    KimiProvider,
    MimoProvider,
    DeepseekProvider,
    MistralProvider,
    OpenRouterProvider,
    FourRouterProvider,
    NousPortalProvider,
    DoubaoAIProvider,
    NvidiaAIProvider,
    ToolPkgJsAiProviderService,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApiKeyProviderSpec {
    SingleApiKeyProvider { api_key: String },
    MultiApiKeyProvider { config_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlamaSessionConfig {
    pub n_threads: i32,
    pub n_ctx: i32,
    pub n_batch: i32,
    pub n_ubatch: i32,
    pub n_gpu_layers: i32,
    pub use_mmap: bool,
    pub flash_attention: bool,
    pub kv_unified: bool,
    pub offload_kqv: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderCreateParams {
    OpenAIProvider {
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    },
    OpenAIResponsesProvider {
        responses_api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        responses_provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    },
    ClaudeProvider {
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        enable_tool_call: bool,
    },
    GeminiProvider {
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        enable_google_search: bool,
        enable_tool_call: bool,
    },
    OllamaProvider {
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    },
    MNNProvider {
        model_name: String,
        forward_type: String,
        thread_count: i32,
        provider_type: ApiProviderType,
        enable_tool_call: bool,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
    },
    LlamaProvider {
        model_name: String,
        session_config: LlamaSessionConfig,
        provider_type: ApiProviderType,
        enable_tool_call: bool,
    },
    QwenAIProvider {
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        qwen_provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    },
    KimiProvider {
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    },
    MimoProvider {
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    },
    DeepseekProvider {
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    },
    MistralProvider {
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    },
    OpenRouterProvider {
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    },
    FourRouterProvider {
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    },
    NousPortalProvider {
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    },
    DoubaoAIProvider {
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    },
    NvidiaAIProvider {
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    },
    ToolPkgJsAiProviderService {
        provider_type_id: String,
        config_id: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderServiceSpec {
    pub kind: ProviderServiceKind,
    pub params: ProviderCreateParams,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProviderCreateRequest {
    pub config: ModelConfigData,
    pub selected_model_name: String,
    pub provider_type: ApiProviderType,
    pub provider_type_id: String,
    pub tool_pkg_provider_registered: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApiKeyMode {
    Single,
    Multiple,
}

pub struct AIServiceFactory;

impl AIServiceFactory {
    pub fn create_service(request: ProviderCreateRequest) -> Result<ProviderServiceSpec, AiServiceError> {
        let config = request.config;
        let provider_type_id = request.provider_type_id.trim().to_string();

        if request.tool_pkg_provider_registered {
            return Ok(ProviderServiceSpec {
                kind: ProviderServiceKind::ToolPkgJsAiProviderService,
                params: ProviderCreateParams::ToolPkgJsAiProviderService {
                    provider_type_id,
                    config_id: config.id,
                },
            });
        }

        let custom_headers = Self::parse_custom_headers(&config.customHeaders)?;
        let api_key_provider = Self::api_key_provider(&config);
        let supports_vision = config.enableDirectImageProcessing;
        let supports_audio = config.enableDirectAudioProcessing;
        let supports_video = config.enableDirectVideoProcessing;
        let enable_tool_call = config.enableToolCall;
        let model_name = request.selected_model_name;
        let provider_type = request.provider_type;

        let spec = match provider_type {
            ApiProviderType::OPENAI | ApiProviderType::OPENAI_GENERIC | ApiProviderType::OPENAI_LOCAL => {
                Self::open_ai_provider(
                    config.apiEndpoint,
                    api_key_provider,
                    model_name,
                    custom_headers,
                    provider_type,
                    supports_vision,
                    supports_audio,
                    supports_video,
                    enable_tool_call,
                )
            }
            ApiProviderType::OPENAI_RESPONSES | ApiProviderType::OPENAI_RESPONSES_GENERIC => {
                Self::not_implemented(provider_type, ProviderServiceKind::OpenAIResponsesProvider)
            }
            ApiProviderType::ANTHROPIC | ApiProviderType::ANTHROPIC_GENERIC => {
                Self::not_implemented(provider_type, ProviderServiceKind::ClaudeProvider)
            }
            ApiProviderType::GOOGLE | ApiProviderType::GEMINI_GENERIC => {
                Self::not_implemented(provider_type, ProviderServiceKind::GeminiProvider)
            }
            ApiProviderType::LMSTUDIO => Self::open_ai_provider(
                config.apiEndpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ),
            ApiProviderType::OLLAMA => Self::not_implemented(provider_type, ProviderServiceKind::OllamaProvider),
            ApiProviderType::MNN => Self::not_implemented(provider_type, ProviderServiceKind::MNNProvider),
            ApiProviderType::LLAMA_CPP => Self::not_implemented(provider_type, ProviderServiceKind::LlamaProvider),
            ApiProviderType::ALIYUN => Self::not_implemented(provider_type, ProviderServiceKind::QwenAIProvider),
            ApiProviderType::BAIDU
            | ApiProviderType::XUNFEI
            | ApiProviderType::ZHIPU
            | ApiProviderType::BAICHUAN
            | ApiProviderType::IFLOW
            | ApiProviderType::INFINIAI
            | ApiProviderType::ALIPAY_BAILING
            | ApiProviderType::PPINFRA
            | ApiProviderType::NOVITA
            | ApiProviderType::OTHER => Self::open_ai_provider(
                config.apiEndpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ),
            ApiProviderType::MOONSHOT => Self::not_implemented(provider_type, ProviderServiceKind::KimiProvider),
            ApiProviderType::MIMO => Self::not_implemented(provider_type, ProviderServiceKind::MimoProvider),
            ApiProviderType::DEEPSEEK => Self::deepseek_provider(
                config.apiEndpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ),
            ApiProviderType::MISTRAL => Self::not_implemented(provider_type, ProviderServiceKind::MistralProvider),
            ApiProviderType::SILICONFLOW => Self::not_implemented(provider_type, ProviderServiceKind::QwenAIProvider),
            ApiProviderType::OPENROUTER => Self::not_implemented(provider_type, ProviderServiceKind::OpenRouterProvider),
            ApiProviderType::FOUR_ROUTER => Self::not_implemented(provider_type, ProviderServiceKind::FourRouterProvider),
            ApiProviderType::NOUS_PORTAL => Self::not_implemented(provider_type, ProviderServiceKind::NousPortalProvider),
            ApiProviderType::DOUBAO => Self::not_implemented(provider_type, ProviderServiceKind::DoubaoAIProvider),
            ApiProviderType::NVIDIA => Self::not_implemented(provider_type, ProviderServiceKind::NvidiaAIProvider),
        }?;

        Ok(spec)
    }

    pub fn parse_custom_headers(custom_headers_json: &str) -> Result<BTreeMap<String, String>, AiServiceError> {
        let trimmed = custom_headers_json.trim();
        if trimmed.is_empty() || trimmed == "{}" {
            return Ok(BTreeMap::new());
        }

        let value: serde_json::Value = serde_json::from_str(trimmed)
            .map_err(|error| AiServiceError::RequestFailed(format!("parse custom headers failed: {error}")))?;
        let object = value
            .as_object()
            .ok_or_else(|| AiServiceError::RequestFailed("customHeaders is not a JSON object".to_string()))?;

        object
            .iter()
            .map(|(key, value)| {
                value
                    .as_str()
                    .map(|header_value| (key.clone(), header_value.to_string()))
                    .ok_or_else(|| {
                        AiServiceError::RequestFailed(format!("customHeaders value for {key} is not a string"))
                    })
            })
            .collect()
    }

    pub fn api_key_mode(config: &ModelConfigData) -> ApiKeyMode {
        if config.useMultipleApiKeys {
            ApiKeyMode::Multiple
        } else {
            ApiKeyMode::Single
        }
    }

    pub fn api_key_provider(config: &ModelConfigData) -> ApiKeyProviderSpec {
        if config.useMultipleApiKeys {
            ApiKeyProviderSpec::MultiApiKeyProvider {
                config_id: config.id.clone(),
            }
        } else {
            ApiKeyProviderSpec::SingleApiKeyProvider {
                api_key: config.apiKey.clone(),
            }
        }
    }

    pub fn build_android_llama_session_config(config: &ModelConfigData, available_processors: i32) -> LlamaSessionConfig {
        let processor_count = available_processors.max(1);
        let safe_thread_count = config.llamaThreadCount.max(1).min(processor_count);
        LlamaSessionConfig {
            n_threads: safe_thread_count,
            n_ctx: config.llamaContextSize.max(1),
            n_batch: 512,
            n_ubatch: 512,
            n_gpu_layers: config.llamaGpuLayers.max(0),
            use_mmap: false,
            flash_attention: false,
            kv_unified: true,
            offload_kqv: false,
        }
    }

    fn open_ai_provider(
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    ) -> Result<ProviderServiceSpec, AiServiceError> {
        Ok(ProviderServiceSpec {
            kind: ProviderServiceKind::OpenAIProvider,
            params: ProviderCreateParams::OpenAIProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            },
        })
    }

    fn deepseek_provider(
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    ) -> Result<ProviderServiceSpec, AiServiceError> {
        Ok(ProviderServiceSpec {
            kind: ProviderServiceKind::DeepseekProvider,
            params: ProviderCreateParams::DeepseekProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            },
        })
    }

    fn not_implemented(
        provider_type: ApiProviderType,
        kind: ProviderServiceKind,
    ) -> Result<ProviderServiceSpec, AiServiceError> {
        Err(AiServiceError::ProviderNotImplemented(format!(
            "provider_type={provider_type:?}, service_kind={kind:?}"
        )))
    }
}
