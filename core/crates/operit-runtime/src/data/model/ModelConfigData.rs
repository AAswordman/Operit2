use serde::{Deserialize, Serialize};

use super::ApiKeyInfo::ApiKeyInfo;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum ApiProviderType {
    OPENAI,
    OPENAI_RESPONSES,
    OPENAI_RESPONSES_GENERIC,
    OPENAI_GENERIC,
    ANTHROPIC,
    ANTHROPIC_GENERIC,
    GOOGLE,
    GEMINI_GENERIC,
    BAIDU,
    ALIYUN,
    XUNFEI,
    ZHIPU,
    BAICHUAN,
    MOONSHOT,
    MIMO,
    DEEPSEEK,
    MISTRAL,
    SILICONFLOW,
    IFLOW,
    OPENROUTER,
    FOUR_ROUTER,
    NOUS_PORTAL,
    INFINIAI,
    ALIPAY_BAILING,
    DOUBAO,
    NVIDIA,
    LMSTUDIO,
    OLLAMA,
    OPENAI_LOCAL,
    MNN,
    LLAMA_CPP,
    PPINFRA,
    NOVITA,
    OTHER,
}

impl ApiProviderType {
    #[allow(non_snake_case)]
    pub fn fromProviderTypeId(providerTypeId: &str) -> Option<Self> {
        let normalized = providerTypeId.trim();
        if normalized.is_empty() {
            return None;
        }

        match normalized.to_ascii_uppercase().as_str() {
            "OPENAI" => Some(Self::OPENAI),
            "OPENAI_RESPONSES" => Some(Self::OPENAI_RESPONSES),
            "OPENAI_RESPONSES_GENERIC" => Some(Self::OPENAI_RESPONSES_GENERIC),
            "OPENAI_GENERIC" => Some(Self::OPENAI_GENERIC),
            "ANTHROPIC" => Some(Self::ANTHROPIC),
            "ANTHROPIC_GENERIC" => Some(Self::ANTHROPIC_GENERIC),
            "GOOGLE" => Some(Self::GOOGLE),
            "GEMINI_GENERIC" => Some(Self::GEMINI_GENERIC),
            "BAIDU" => Some(Self::BAIDU),
            "ALIYUN" => Some(Self::ALIYUN),
            "XUNFEI" => Some(Self::XUNFEI),
            "ZHIPU" => Some(Self::ZHIPU),
            "BAICHUAN" => Some(Self::BAICHUAN),
            "MOONSHOT" => Some(Self::MOONSHOT),
            "MIMO" => Some(Self::MIMO),
            "DEEPSEEK" => Some(Self::DEEPSEEK),
            "MISTRAL" => Some(Self::MISTRAL),
            "SILICONFLOW" => Some(Self::SILICONFLOW),
            "IFLOW" => Some(Self::IFLOW),
            "OPENROUTER" => Some(Self::OPENROUTER),
            "FOUR_ROUTER" => Some(Self::FOUR_ROUTER),
            "NOUS_PORTAL" => Some(Self::NOUS_PORTAL),
            "INFINIAI" => Some(Self::INFINIAI),
            "ALIPAY_BAILING" => Some(Self::ALIPAY_BAILING),
            "DOUBAO" => Some(Self::DOUBAO),
            "NVIDIA" => Some(Self::NVIDIA),
            "LMSTUDIO" => Some(Self::LMSTUDIO),
            "OLLAMA" => Some(Self::OLLAMA),
            "OPENAI_LOCAL" => Some(Self::OPENAI_LOCAL),
            "MNN" => Some(Self::MNN),
            "LLAMA_CPP" => Some(Self::LLAMA_CPP),
            "PPINFRA" => Some(Self::PPINFRA),
            "NOVITA" => Some(Self::NOVITA),
            "OTHER" => Some(Self::OTHER),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::OPENAI => "OPENAI",
            Self::OPENAI_RESPONSES => "OPENAI_RESPONSES",
            Self::OPENAI_RESPONSES_GENERIC => "OPENAI_RESPONSES_GENERIC",
            Self::OPENAI_GENERIC => "OPENAI_GENERIC",
            Self::ANTHROPIC => "ANTHROPIC",
            Self::ANTHROPIC_GENERIC => "ANTHROPIC_GENERIC",
            Self::GOOGLE => "GOOGLE",
            Self::GEMINI_GENERIC => "GEMINI_GENERIC",
            Self::BAIDU => "BAIDU",
            Self::ALIYUN => "ALIYUN",
            Self::XUNFEI => "XUNFEI",
            Self::ZHIPU => "ZHIPU",
            Self::BAICHUAN => "BAICHUAN",
            Self::MOONSHOT => "MOONSHOT",
            Self::MIMO => "MIMO",
            Self::DEEPSEEK => "DEEPSEEK",
            Self::MISTRAL => "MISTRAL",
            Self::SILICONFLOW => "SILICONFLOW",
            Self::IFLOW => "IFLOW",
            Self::OPENROUTER => "OPENROUTER",
            Self::FOUR_ROUTER => "FOUR_ROUTER",
            Self::NOUS_PORTAL => "NOUS_PORTAL",
            Self::INFINIAI => "INFINIAI",
            Self::ALIPAY_BAILING => "ALIPAY_BAILING",
            Self::DOUBAO => "DOUBAO",
            Self::NVIDIA => "NVIDIA",
            Self::LMSTUDIO => "LMSTUDIO",
            Self::OLLAMA => "OLLAMA",
            Self::OPENAI_LOCAL => "OPENAI_LOCAL",
            Self::MNN => "MNN",
            Self::LLAMA_CPP => "LLAMA_CPP",
            Self::PPINFRA => "PPINFRA",
            Self::NOVITA => "NOVITA",
            Self::OTHER => "OTHER",
        }
    }
}

impl Default for ApiProviderType {
    fn default() -> Self {
        Self::DEEPSEEK
    }
}

pub struct ModelConfigDefaults;

impl ModelConfigDefaults {
    pub const DEFAULT_CONTEXT_LENGTH: f32 = 64.0;
    pub const DEFAULT_MAX_CONTEXT_LENGTH: f32 = 200.0;
    pub const DEFAULT_ENABLE_MAX_CONTEXT_MODE: bool = false;
    pub const DEFAULT_SUMMARY_TOKEN_THRESHOLD: f32 = 0.70;
    pub const DEFAULT_ENABLE_SUMMARY: bool = true;
    pub const DEFAULT_ENABLE_SUMMARY_BY_MESSAGE_COUNT: bool = true;
    pub const DEFAULT_SUMMARY_MESSAGE_COUNT_THRESHOLD: i32 = 16;
    pub const DEFAULT_DEEPSEEK_ENDPOINT: &'static str = "https://api.deepseek.com/v1/chat/completions";
    pub const DEFAULT_DEEPSEEK_MODEL: &'static str = "deepseek-chat";
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelConfigData {
    pub id: String,
    pub name: String,

    #[serde(default)]
    pub apiKey: String,
    #[serde(default = "default_deepseek_endpoint")]
    pub apiEndpoint: String,
    #[serde(default = "default_deepseek_model")]
    pub modelName: String,
    #[serde(default)]
    pub apiProviderType: ApiProviderType,
    #[serde(default = "default_api_provider_type_id")]
    pub apiProviderTypeId: String,

    #[serde(default)]
    pub useMultipleApiKeys: bool,
    #[serde(default)]
    pub apiKeyPool: Vec<ApiKeyInfo>,
    #[serde(default)]
    pub currentKeyIndex: i32,
    #[serde(default = "default_key_rotation_mode")]
    pub keyRotationMode: String,

    #[serde(default)]
    pub hasCustomParameters: bool,

    #[serde(default)]
    pub maxTokensEnabled: bool,
    #[serde(default)]
    pub temperatureEnabled: bool,
    #[serde(default)]
    pub topPEnabled: bool,
    #[serde(default)]
    pub topKEnabled: bool,
    #[serde(default)]
    pub presencePenaltyEnabled: bool,
    #[serde(default)]
    pub frequencyPenaltyEnabled: bool,
    #[serde(default)]
    pub repetitionPenaltyEnabled: bool,

    #[serde(default = "default_max_tokens")]
    pub maxTokens: i32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_top_p")]
    pub topP: f32,
    #[serde(default)]
    pub topK: i32,
    #[serde(default)]
    pub presencePenalty: f32,
    #[serde(default)]
    pub frequencyPenalty: f32,
    #[serde(default = "default_repetition_penalty")]
    pub repetitionPenalty: f32,

    #[serde(default = "default_custom_parameters")]
    pub customParameters: String,
    #[serde(default = "default_custom_headers")]
    pub customHeaders: String,

    #[serde(default = "default_context_length")]
    pub contextLength: f32,
    #[serde(default = "default_max_context_length")]
    pub maxContextLength: f32,
    #[serde(default)]
    pub enableMaxContextMode: bool,
    #[serde(default = "default_summary_token_threshold")]
    pub summaryTokenThreshold: f32,
    #[serde(default = "default_true")]
    pub enableSummary: bool,
    #[serde(default = "default_true")]
    pub enableSummaryByMessageCount: bool,
    #[serde(default = "default_summary_message_count_threshold")]
    pub summaryMessageCountThreshold: i32,

    #[serde(default)]
    pub mnnForwardType: i32,
    #[serde(default = "default_thread_count")]
    pub mnnThreadCount: i32,

    #[serde(default = "default_thread_count")]
    pub llamaThreadCount: i32,
    #[serde(default = "default_llama_context_size")]
    pub llamaContextSize: i32,
    #[serde(default = "default_llama_batch_size")]
    pub llamaBatchSize: i32,
    #[serde(default = "default_llama_batch_size")]
    pub llamaUBatchSize: i32,
    #[serde(default)]
    pub llamaGpuLayers: i32,
    #[serde(default)]
    pub llamaUseMmap: bool,
    #[serde(default)]
    pub llamaFlashAttention: bool,
    #[serde(default = "default_true")]
    pub llamaKvUnified: bool,
    #[serde(default)]
    pub llamaOffloadKqv: bool,

    #[serde(default)]
    pub enableDirectImageProcessing: bool,
    #[serde(default)]
    pub enableDirectAudioProcessing: bool,
    #[serde(default)]
    pub enableDirectVideoProcessing: bool,

    #[serde(default)]
    pub enableGoogleSearch: bool,
    #[serde(default)]
    pub enableToolCall: bool,

    #[serde(default)]
    pub requestLimitPerMinute: i32,
    #[serde(default)]
    pub maxConcurrentRequests: i32,
}

impl ModelConfigData {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            ..Self::default()
        }
    }
}

impl Default for ModelConfigData {
    fn default() -> Self {
        let apiProviderType = ApiProviderType::DEEPSEEK;
        Self {
            id: String::new(),
            name: String::new(),
            apiKey: String::new(),
            apiEndpoint: ModelConfigDefaults::DEFAULT_DEEPSEEK_ENDPOINT.to_string(),
            modelName: ModelConfigDefaults::DEFAULT_DEEPSEEK_MODEL.to_string(),
            apiProviderTypeId: apiProviderType.name().to_string(),
            apiProviderType,
            useMultipleApiKeys: false,
            apiKeyPool: Vec::new(),
            currentKeyIndex: 0,
            keyRotationMode: "ROUND_ROBIN".to_string(),
            hasCustomParameters: false,
            maxTokensEnabled: false,
            temperatureEnabled: false,
            topPEnabled: false,
            topKEnabled: false,
            presencePenaltyEnabled: false,
            frequencyPenaltyEnabled: false,
            repetitionPenaltyEnabled: false,
            maxTokens: 4096,
            temperature: 1.0,
            topP: 1.0,
            topK: 0,
            presencePenalty: 0.0,
            frequencyPenalty: 0.0,
            repetitionPenalty: 1.0,
            customParameters: "[]".to_string(),
            customHeaders: "{}".to_string(),
            contextLength: ModelConfigDefaults::DEFAULT_CONTEXT_LENGTH,
            maxContextLength: ModelConfigDefaults::DEFAULT_MAX_CONTEXT_LENGTH,
            enableMaxContextMode: ModelConfigDefaults::DEFAULT_ENABLE_MAX_CONTEXT_MODE,
            summaryTokenThreshold: ModelConfigDefaults::DEFAULT_SUMMARY_TOKEN_THRESHOLD,
            enableSummary: ModelConfigDefaults::DEFAULT_ENABLE_SUMMARY,
            enableSummaryByMessageCount: ModelConfigDefaults::DEFAULT_ENABLE_SUMMARY_BY_MESSAGE_COUNT,
            summaryMessageCountThreshold: ModelConfigDefaults::DEFAULT_SUMMARY_MESSAGE_COUNT_THRESHOLD,
            mnnForwardType: 0,
            mnnThreadCount: 4,
            llamaThreadCount: 4,
            llamaContextSize: 2048,
            llamaBatchSize: 512,
            llamaUBatchSize: 512,
            llamaGpuLayers: 0,
            llamaUseMmap: false,
            llamaFlashAttention: false,
            llamaKvUnified: true,
            llamaOffloadKqv: false,
            enableDirectImageProcessing: false,
            enableDirectAudioProcessing: false,
            enableDirectVideoProcessing: false,
            enableGoogleSearch: false,
            enableToolCall: false,
            requestLimitPerMinute: 0,
            maxConcurrentRequests: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelConfigSummary {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub modelName: String,
    #[serde(default)]
    pub apiEndpoint: String,
    #[serde(default)]
    pub apiProviderType: ApiProviderType,
    #[serde(default)]
    pub modelIndex: i32,
}

impl ModelConfigSummary {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            modelName: String::new(),
            apiEndpoint: String::new(),
            apiProviderType: ApiProviderType::DEEPSEEK,
            modelIndex: 0,
        }
    }
}

#[allow(non_snake_case)]
pub fn getModelByIndex(modelName: &str, index: i32) -> String {
    if modelName.is_empty() {
        return String::new();
    }
    let models = getModelList(modelName);
    if index >= 0 && (index as usize) < models.len() {
        models[index as usize].clone()
    } else {
        models.get(0).cloned().unwrap_or_default()
    }
}

#[allow(non_snake_case)]
pub fn getModelList(modelName: &str) -> Vec<String> {
    if modelName.is_empty() {
        return Vec::new();
    }
    modelName
        .split(',')
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[allow(non_snake_case)]
pub fn getValidModelIndex(modelName: &str, requestedIndex: i32) -> i32 {
    let modelList = getModelList(modelName);
    if requestedIndex >= 0 && (requestedIndex as usize) < modelList.len() {
        requestedIndex
    } else {
        0
    }
}

fn default_true() -> bool {
    true
}

fn default_deepseek_endpoint() -> String {
    ModelConfigDefaults::DEFAULT_DEEPSEEK_ENDPOINT.to_string()
}

fn default_deepseek_model() -> String {
    ModelConfigDefaults::DEFAULT_DEEPSEEK_MODEL.to_string()
}

fn default_api_provider_type_id() -> String {
    ApiProviderType::DEEPSEEK.name().to_string()
}

fn default_key_rotation_mode() -> String {
    "ROUND_ROBIN".to_string()
}

fn default_max_tokens() -> i32 {
    4096
}

fn default_temperature() -> f32 {
    1.0
}

fn default_top_p() -> f32 {
    1.0
}

fn default_repetition_penalty() -> f32 {
    1.0
}

fn default_custom_parameters() -> String {
    "[]".to_string()
}

fn default_custom_headers() -> String {
    "{}".to_string()
}

fn default_context_length() -> f32 {
    ModelConfigDefaults::DEFAULT_CONTEXT_LENGTH
}

fn default_max_context_length() -> f32 {
    ModelConfigDefaults::DEFAULT_MAX_CONTEXT_LENGTH
}

fn default_summary_token_threshold() -> f32 {
    ModelConfigDefaults::DEFAULT_SUMMARY_TOKEN_THRESHOLD
}

fn default_summary_message_count_threshold() -> i32 {
    ModelConfigDefaults::DEFAULT_SUMMARY_MESSAGE_COUNT_THRESHOLD
}

fn default_thread_count() -> i32 {
    4
}

fn default_llama_context_size() -> i32 {
    2048
}

fn default_llama_batch_size() -> i32 {
    512
}
