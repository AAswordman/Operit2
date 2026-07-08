use serde::{Deserialize, Serialize};

use super::ApiKeyInfo::ApiKeyInfo;
use super::BillingMode::BillingMode;
use super::ModelParameter::ModelParameter;

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
        match providerTypeId.trim().to_ascii_uppercase().as_str() {
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
    pub const DEFAULT_MAX_CONTEXT_LENGTH: f32 = 200.0;
    pub const DEFAULT_ENABLE_MAX_CONTEXT_MODE: bool = false;
    pub const DEFAULT_SUMMARY_TOKEN_THRESHOLD: f32 = 0.70;
    pub const DEFAULT_ENABLE_SUMMARY: bool = true;
    pub const DEFAULT_ENABLE_SUMMARY_BY_MESSAGE_COUNT: bool = true;
    pub const DEFAULT_SUMMARY_MESSAGE_COUNT_THRESHOLD: i32 = 16;
    pub const DEFAULT_DEEPSEEK_ENDPOINT: &'static str =
        "https://api.deepseek.com/v1/chat/completions";
    pub const DEFAULT_DEEPSEEK_MODEL: &'static str = "deepseek-v4-flash";
    pub const DEFAULT_PROVIDER_ID: &'static str = "DEEPSEEK";
    pub const DEFAULT_MODEL_ID: &'static str = "deepseek-v4-flash";
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PricingCurrency {
    CNY,
    USD,
}

impl PricingCurrency {
    pub fn code(&self) -> &'static str {
        match self {
            Self::CNY => "CNY",
            Self::USD => "USD",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelPricing {
    pub billingMode: BillingMode,
    pub inputPricePerMillion: f64,
    pub cachedInputPricePerMillion: Option<f64>,
    pub outputPricePerMillion: f64,
    pub pricePerRequest: f64,
    pub currency: PricingCurrency,
}

impl ModelPricing {
    pub fn token(
        inputPricePerMillion: f64,
        cachedInputPricePerMillion: Option<f64>,
        outputPricePerMillion: f64,
        currency: PricingCurrency,
    ) -> Self {
        Self {
            billingMode: BillingMode::TOKEN,
            inputPricePerMillion,
            cachedInputPricePerMillion,
            outputPricePerMillion,
            pricePerRequest: 0.0,
            currency,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelCapabilities {
    pub directImage: bool,
    pub directAudio: bool,
    pub directVideo: bool,
    pub toolCall: bool,
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            directImage: false,
            directAudio: false,
            directVideo: false,
            toolCall: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuiltinToolType {
    WebSearch,
    CodeExecution,
    UrlContext,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuiltinToolRequestFormat {
    GeminiGoogleSearch,
    AnthropicWebSearch,
    OpenAiWebSearch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuiltinToolExclusivity {
    CanMixWithExternalTools,
    ExclusiveWithExternalTools,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelBuiltinTool {
    pub toolType: BuiltinToolType,
    pub displayName: String,
    pub enabled: bool,
    pub requestFormat: BuiltinToolRequestFormat,
    pub exclusivity: BuiltinToolExclusivity,
    pub config: serde_json::Value,
}

impl ModelBuiltinTool {
    pub fn disabled(
        toolType: BuiltinToolType,
        displayName: String,
        requestFormat: BuiltinToolRequestFormat,
        exclusivity: BuiltinToolExclusivity,
    ) -> Self {
        Self {
            toolType,
            displayName,
            enabled: false,
            requestFormat,
            exclusivity,
            config: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelContextSpec {
    pub maxContextLength: f32,
    pub enableMaxContextMode: bool,
}

impl Default for ModelContextSpec {
    fn default() -> Self {
        Self {
            maxContextLength: ModelConfigDefaults::DEFAULT_MAX_CONTEXT_LENGTH,
            enableMaxContextMode: ModelConfigDefaults::DEFAULT_ENABLE_MAX_CONTEXT_MODE,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelSummarySettings {
    pub enableSummary: bool,
    pub summaryTokenThreshold: f32,
    pub enableSummaryByMessageCount: bool,
    pub summaryMessageCountThreshold: i32,
}

impl Default for ModelSummarySettings {
    fn default() -> Self {
        Self {
            enableSummary: ModelConfigDefaults::DEFAULT_ENABLE_SUMMARY,
            summaryTokenThreshold: ModelConfigDefaults::DEFAULT_SUMMARY_TOKEN_THRESHOLD,
            enableSummaryByMessageCount:
                ModelConfigDefaults::DEFAULT_ENABLE_SUMMARY_BY_MESSAGE_COUNT,
            summaryMessageCountThreshold:
                ModelConfigDefaults::DEFAULT_SUMMARY_MESSAGE_COUNT_THRESHOLD,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalModelRuntimeSettings {
    pub mnnForwardType: i32,
    pub mnnThreadCount: i32,
    pub llamaThreadCount: i32,
    pub llamaContextSize: i32,
    pub llamaBatchSize: i32,
    pub llamaUBatchSize: i32,
    pub llamaGpuLayers: i32,
    pub llamaUseMmap: bool,
    pub llamaFlashAttention: bool,
    pub llamaKvUnified: bool,
    pub llamaOffloadKqv: bool,
}

impl Default for LocalModelRuntimeSettings {
    fn default() -> Self {
        Self {
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
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelRequestSpec {
    pub supportsStructuredTools: bool,
}

impl Default for ModelRequestSpec {
    fn default() -> Self {
        Self {
            supportsStructuredTools: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ProviderOperationResultSpec {
    pub itemsJsonPath: Option<String>,
    pub itemIdJsonPath: Option<String>,
    pub inputPricePerTokenJsonPath: Option<String>,
    pub cachedInputPricePerTokenJsonPath: Option<String>,
    pub outputPricePerTokenJsonPath: Option<String>,
    pub pricePerRequestJsonPath: Option<String>,
    pub currencyJsonPath: Option<String>,
    pub maxContextLengthJsonPath: Option<String>,
    pub directImageJsonPath: Option<String>,
    pub directAudioJsonPath: Option<String>,
    pub directVideoJsonPath: Option<String>,
    pub toolCallJsonPath: Option<String>,
    pub supportsStructuredToolsJsonPath: Option<String>,
    pub amountJsonPath: Option<String>,
    pub amountCurrencyJsonPath: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ProviderOperationSpec {
    pub operationType: String,
    pub handlerId: String,
    pub method: String,
    pub path: String,
    pub requiresApiKey: bool,
    pub result: ProviderOperationResultSpec,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelCatalogKey {
    pub providerTypeId: String,
    pub modelId: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelCatalogEntry {
    pub providerTypeId: String,
    pub modelId: String,
    pub aliases: Vec<String>,
    pub pricing: Option<ModelPricing>,
    pub context: Option<ModelContextSpec>,
    pub capabilities: Option<ModelCapabilities>,
    pub builtinTools: Vec<ModelBuiltinTool>,
    pub request: Option<ModelRequestSpec>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ProviderCatalogEntry {
    pub providerTypeId: String,
    pub displayName: String,
    pub defaultEndpoint: String,
    pub operations: Vec<ProviderOperationSpec>,
    pub models: Vec<ModelCatalogEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AvailableProviderModelSource {
    Catalog,
    Remote,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct AvailableProviderModel {
    pub modelId: String,
    pub source: AvailableProviderModelSource,
    pub pricing: Option<ModelPricing>,
    pub context: Option<ModelContextSpec>,
    pub capabilities: Option<ModelCapabilities>,
    pub builtinTools: Vec<ModelBuiltinTool>,
    pub request: Option<ModelRequestSpec>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelProfile {
    pub id: String,
    pub catalogKey: Option<ModelCatalogKey>,
    pub pricingOverride: Option<ModelPricing>,
    pub contextOverride: Option<ModelContextSpec>,
    pub capabilitiesOverride: Option<ModelCapabilities>,
    pub builtinToolsOverride: Option<Vec<ModelBuiltinTool>>,
    pub requestOverride: Option<ModelRequestSpec>,
    pub parameters: Vec<ModelParameter<serde_json::Value>>,
    pub summary: ModelSummarySettings,
    pub localRuntime: LocalModelRuntimeSettings,
}

impl ModelProfile {
    pub fn new(id: String) -> Self {
        Self {
            id,
            catalogKey: None,
            pricingOverride: None,
            contextOverride: None,
            capabilitiesOverride: None,
            builtinToolsOverride: None,
            requestOverride: None,
            parameters: Vec::new(),
            summary: ModelSummarySettings::default(),
            localRuntime: LocalModelRuntimeSettings::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ProviderProfile {
    pub id: String,
    pub name: String,
    pub providerTypeId: String,
    pub providerType: ApiProviderType,
    pub endpoint: String,
    pub apiKey: String,
    pub useMultipleApiKeys: bool,
    pub apiKeyPool: Vec<ApiKeyInfo>,
    pub currentKeyIndex: i32,
    pub keyRotationMode: String,
    pub customHeaders: String,
    pub requestLimitPerMinute: i32,
    pub maxConcurrentRequests: i32,
    pub models: Vec<ModelProfile>,
}

impl ProviderProfile {
    pub fn new(id: String, name: String, providerType: ApiProviderType, endpoint: String) -> Self {
        Self {
            id,
            name,
            providerTypeId: providerType.name().to_string(),
            providerType,
            endpoint,
            apiKey: String::new(),
            useMultipleApiKeys: false,
            apiKeyPool: Vec::new(),
            currentKeyIndex: 0,
            keyRotationMode: "ROUND_ROBIN".to_string(),
            customHeaders: "{}".to_string(),
            requestLimitPerMinute: 0,
            maxConcurrentRequests: 0,
            models: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ResolvedModelConfig {
    pub providerId: String,
    pub providerName: String,
    pub modelId: String,
    pub apiKey: String,
    pub apiEndpoint: String,
    pub apiProviderType: ApiProviderType,
    pub apiProviderTypeId: String,
    pub useMultipleApiKeys: bool,
    pub apiKeyPool: Vec<ApiKeyInfo>,
    pub currentKeyIndex: i32,
    pub keyRotationMode: String,
    pub customHeaders: String,
    pub requestLimitPerMinute: i32,
    pub maxConcurrentRequests: i32,
    pub pricing: Option<ModelPricing>,
    pub context: ModelContextSpec,
    pub capabilities: ModelCapabilities,
    pub builtinTools: Vec<ModelBuiltinTool>,
    pub request: ModelRequestSpec,
    pub parameters: Vec<ModelParameter<serde_json::Value>>,
    pub summary: ModelSummarySettings,
    pub localRuntime: LocalModelRuntimeSettings,
}

impl ResolvedModelConfig {
    pub fn providerModelLabel(&self) -> String {
        format!("{}:{}", self.apiProviderTypeId, self.modelId)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ProviderModelSummary {
    pub providerId: String,
    pub providerName: String,
    pub providerTypeId: String,
    pub endpoint: String,
    pub modelId: String,
    pub capabilities: ModelCapabilities,
    pub pricing: Option<ModelPricing>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelConnectionTestItem {
    pub r#type: String,
    pub success: bool,
    pub error: Option<String>,
}

pub fn default_deepseek_provider() -> ProviderProfile {
    let mut provider = ProviderProfile::new(
        ModelConfigDefaults::DEFAULT_PROVIDER_ID.to_string(),
        "DeepSeek".to_string(),
        ApiProviderType::DEEPSEEK,
        ModelConfigDefaults::DEFAULT_DEEPSEEK_ENDPOINT.to_string(),
    );
    provider.models.push(default_deepseek_model());
    provider
}

pub fn default_deepseek_model() -> ModelProfile {
    let mut model = ModelProfile::new(ModelConfigDefaults::DEFAULT_MODEL_ID.to_string());
    model.catalogKey = Some(ModelCatalogKey {
        providerTypeId: ApiProviderType::DEEPSEEK.name().to_string(),
        modelId: ModelConfigDefaults::DEFAULT_MODEL_ID.to_string(),
    });
    model.contextOverride = Some(ModelContextSpec::default());
    model.capabilitiesOverride = Some(ModelCapabilities {
        toolCall: true,
        ..ModelCapabilities::default()
    });
    model.requestOverride = Some(ModelRequestSpec {
        supportsStructuredTools: true,
    });
    model
}
