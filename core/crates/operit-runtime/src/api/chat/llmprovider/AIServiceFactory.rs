use std::collections::BTreeMap;

use super::AIService::AiServiceError;
use crate::data::model::ModelConfigData::{ApiProviderType, ModelBuiltinTool, ResolvedModelConfig};

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
    MultiApiKeyProvider { provider_id: String },
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

#[derive(Clone, Debug, PartialEq)]
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
        builtin_tools: Vec<ModelBuiltinTool>,
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
        provider_id: String,
        model_id: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProviderServiceSpec {
    pub kind: ProviderServiceKind,
    pub params: ProviderCreateParams,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProviderCreateRequest {
    pub config: ResolvedModelConfig,
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
    pub fn create_service(
        request: ProviderCreateRequest,
    ) -> Result<ProviderServiceSpec, AiServiceError> {
        let config = request.config;
        let provider_type_id = request.provider_type_id.trim().to_string();

        if request.tool_pkg_provider_registered {
            return Ok(ProviderServiceSpec {
                kind: ProviderServiceKind::ToolPkgJsAiProviderService,
                params: ProviderCreateParams::ToolPkgJsAiProviderService {
                    provider_type_id,
                    provider_id: config.providerId,
                    model_id: config.modelId,
                },
            });
        }

        let custom_headers = Self::parse_custom_headers(&config.customHeaders)?;
        let api_key_provider = Self::api_key_provider(&config);
        let supports_vision = config.capabilities.directImage;
        let supports_audio = config.capabilities.directAudio;
        let supports_video = config.capabilities.directVideo;
        let enable_tool_call = config.capabilities.toolCall;
        let builtin_tools = config.builtinTools.clone();
        let model_name = config.modelId.clone();
        let provider_type = request.provider_type;

        let spec = match provider_type {
            ApiProviderType::OPENAI
            | ApiProviderType::OPENAI_GENERIC
            | ApiProviderType::OPENAI_LOCAL => Self::open_ai_provider(
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
            ApiProviderType::OPENAI_RESPONSES | ApiProviderType::OPENAI_RESPONSES_GENERIC => {
                Self::open_ai_responses_provider(
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
            ApiProviderType::ANTHROPIC | ApiProviderType::ANTHROPIC_GENERIC => {
                Self::claude_provider(
                    config.apiEndpoint,
                    api_key_provider,
                    model_name,
                    custom_headers,
                    provider_type,
                    enable_tool_call,
                )
            }
            ApiProviderType::GOOGLE | ApiProviderType::GEMINI_GENERIC => Self::gemini_provider(
                config.apiEndpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                builtin_tools,
                enable_tool_call,
            ),
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
            ApiProviderType::OLLAMA => Self::ollama_provider(
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
            ApiProviderType::MNN => Self::mnn_provider(
                model_name,
                config.localRuntime.mnnForwardType.to_string(),
                config.localRuntime.mnnThreadCount,
                provider_type,
                enable_tool_call,
                supports_vision,
                supports_audio,
                supports_video,
            ),
            ApiProviderType::LLAMA_CPP => Self::llama_provider(
                model_name,
                Self::build_android_llama_session_config(&config, 1),
                provider_type,
                enable_tool_call,
            ),
            ApiProviderType::ALIYUN => Self::qwen_provider(
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
            ApiProviderType::MOONSHOT => Self::kimi_provider(
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
            ApiProviderType::MIMO => Self::mimo_provider(
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
            ApiProviderType::MISTRAL => Self::mistral_provider(
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
            ApiProviderType::SILICONFLOW => Self::qwen_provider(
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
            ApiProviderType::OPENROUTER => Self::open_router_provider(
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
            ApiProviderType::FOUR_ROUTER => Self::four_router_provider(
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
            ApiProviderType::NOUS_PORTAL => Self::nous_portal_provider(
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
            ApiProviderType::DOUBAO => Self::doubao_provider(
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
            ApiProviderType::NVIDIA => Self::nvidia_provider(
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
        }?;

        Ok(spec)
    }

    pub fn parse_custom_headers(
        custom_headers_json: &str,
    ) -> Result<BTreeMap<String, String>, AiServiceError> {
        let trimmed = custom_headers_json.trim();
        if trimmed.is_empty() || trimmed == "{}" {
            return Ok(BTreeMap::new());
        }

        let value: serde_json::Value = serde_json::from_str(trimmed).map_err(|error| {
            AiServiceError::RequestFailed(format!("parse custom headers failed: {error}"))
        })?;
        let object = value.as_object().ok_or_else(|| {
            AiServiceError::RequestFailed("customHeaders is not a JSON object".to_string())
        })?;

        object
            .iter()
            .map(|(key, value)| {
                value
                    .as_str()
                    .map(|header_value| (key.clone(), header_value.to_string()))
                    .ok_or_else(|| {
                        AiServiceError::RequestFailed(format!(
                            "customHeaders value for {key} is not a string"
                        ))
                    })
            })
            .collect()
    }

    pub fn api_key_mode(config: &ResolvedModelConfig) -> ApiKeyMode {
        if config.useMultipleApiKeys {
            ApiKeyMode::Multiple
        } else {
            ApiKeyMode::Single
        }
    }

    pub fn api_key_provider(config: &ResolvedModelConfig) -> ApiKeyProviderSpec {
        if config.useMultipleApiKeys {
            ApiKeyProviderSpec::MultiApiKeyProvider {
                provider_id: config.providerId.clone(),
            }
        } else {
            ApiKeyProviderSpec::SingleApiKeyProvider {
                api_key: config.apiKey.clone(),
            }
        }
    }

    pub fn build_android_llama_session_config(
        config: &ResolvedModelConfig,
        available_processors: i32,
    ) -> LlamaSessionConfig {
        let processor_count = available_processors.max(1);
        let thread_count = config
            .localRuntime
            .llamaThreadCount
            .max(1)
            .min(processor_count);
        LlamaSessionConfig {
            n_threads: thread_count,
            n_ctx: config.localRuntime.llamaContextSize.max(1),
            n_batch: config.localRuntime.llamaBatchSize.max(1),
            n_ubatch: config.localRuntime.llamaUBatchSize.max(1),
            n_gpu_layers: config.localRuntime.llamaGpuLayers.max(0),
            use_mmap: config.localRuntime.llamaUseMmap,
            flash_attention: config.localRuntime.llamaFlashAttention,
            kv_unified: config.localRuntime.llamaKvUnified,
            offload_kqv: config.localRuntime.llamaOffloadKqv,
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

    fn open_ai_responses_provider(
        responses_api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        responses_provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    ) -> Result<ProviderServiceSpec, AiServiceError> {
        Ok(ProviderServiceSpec {
            kind: ProviderServiceKind::OpenAIResponsesProvider,
            params: ProviderCreateParams::OpenAIResponsesProvider {
                responses_api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                responses_provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            },
        })
    }

    fn claude_provider(
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        enable_tool_call: bool,
    ) -> Result<ProviderServiceSpec, AiServiceError> {
        Ok(ProviderServiceSpec {
            kind: ProviderServiceKind::ClaudeProvider,
            params: ProviderCreateParams::ClaudeProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                enable_tool_call,
            },
        })
    }

    fn gemini_provider(
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        provider_type: ApiProviderType,
        builtin_tools: Vec<ModelBuiltinTool>,
        enable_tool_call: bool,
    ) -> Result<ProviderServiceSpec, AiServiceError> {
        Ok(ProviderServiceSpec {
            kind: ProviderServiceKind::GeminiProvider,
            params: ProviderCreateParams::GeminiProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                provider_type,
                builtin_tools,
                enable_tool_call,
            },
        })
    }

    fn ollama_provider(
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
            kind: ProviderServiceKind::OllamaProvider,
            params: ProviderCreateParams::OllamaProvider {
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

    fn mnn_provider(
        model_name: String,
        forward_type: String,
        thread_count: i32,
        provider_type: ApiProviderType,
        enable_tool_call: bool,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
    ) -> Result<ProviderServiceSpec, AiServiceError> {
        Ok(ProviderServiceSpec {
            kind: ProviderServiceKind::MNNProvider,
            params: ProviderCreateParams::MNNProvider {
                model_name,
                forward_type,
                thread_count,
                provider_type,
                enable_tool_call,
                supports_vision,
                supports_audio,
                supports_video,
            },
        })
    }

    fn llama_provider(
        model_name: String,
        session_config: LlamaSessionConfig,
        provider_type: ApiProviderType,
        enable_tool_call: bool,
    ) -> Result<ProviderServiceSpec, AiServiceError> {
        Ok(ProviderServiceSpec {
            kind: ProviderServiceKind::LlamaProvider,
            params: ProviderCreateParams::LlamaProvider {
                model_name,
                session_config,
                provider_type,
                enable_tool_call,
            },
        })
    }

    fn qwen_provider(
        api_endpoint: String,
        api_key_provider: ApiKeyProviderSpec,
        model_name: String,
        custom_headers: BTreeMap<String, String>,
        qwen_provider_type: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
    ) -> Result<ProviderServiceSpec, AiServiceError> {
        Ok(ProviderServiceSpec {
            kind: ProviderServiceKind::QwenAIProvider,
            params: ProviderCreateParams::QwenAIProvider {
                api_endpoint,
                api_key_provider,
                model_name,
                custom_headers,
                qwen_provider_type,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            },
        })
    }

    fn kimi_provider(
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
            kind: ProviderServiceKind::KimiProvider,
            params: ProviderCreateParams::KimiProvider {
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

    fn mimo_provider(
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
            kind: ProviderServiceKind::MimoProvider,
            params: ProviderCreateParams::MimoProvider {
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

    fn mistral_provider(
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
            kind: ProviderServiceKind::MistralProvider,
            params: ProviderCreateParams::MistralProvider {
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

    fn open_router_provider(
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
            kind: ProviderServiceKind::OpenRouterProvider,
            params: ProviderCreateParams::OpenRouterProvider {
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

    fn four_router_provider(
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
            kind: ProviderServiceKind::FourRouterProvider,
            params: ProviderCreateParams::FourRouterProvider {
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

    fn nous_portal_provider(
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
            kind: ProviderServiceKind::NousPortalProvider,
            params: ProviderCreateParams::NousPortalProvider {
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

    fn doubao_provider(
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
            kind: ProviderServiceKind::DoubaoAIProvider,
            params: ProviderCreateParams::DoubaoAIProvider {
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

    fn nvidia_provider(
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
            kind: ProviderServiceKind::NvidiaAIProvider,
            params: ProviderCreateParams::NvidiaAIProvider {
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
}
