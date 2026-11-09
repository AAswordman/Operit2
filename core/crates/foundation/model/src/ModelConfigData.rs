use serde::{Deserialize, Serialize};

use super::ApiKeyInfo::ApiKeyInfo;
use super::BillingMode::BillingMode;
use super::ModelParameter::ModelParameter;
use crate::ModelThinking::{ModelThinkingSpec, ThinkingLevel};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum ApiProviderType {
    OPENAI,
    XAI,
    OPENAI_RESPONSES,
    OPENAI_CODEX,
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
    OPENROUTER,
    OPENCODE,
    FOUR_ROUTER,
    NOUS_PORTAL,
    INFINIAI,
    ALIPAY_BAILING,
    DOUBAO,
    NVIDIA,
    LMSTUDIO,
    OLLAMA,
    OPENAI_LOCAL,
    LOCAL_MODEL,
    PPINFRA,
    NOVITA,
    MINIMAX,
    OTHER,
}

impl ApiProviderType {
    #[allow(non_snake_case)]
    /// Parses a stable provider type id into its enum representation.
    pub fn fromProviderTypeId(providerTypeId: &str) -> Option<Self> {
        match providerTypeId.trim().to_ascii_uppercase().as_str() {
            "OPENAI" => Some(Self::OPENAI),
            "XAI" => Some(Self::XAI),
            "OPENAI_RESPONSES" => Some(Self::OPENAI_RESPONSES),
            "OPENAI_CODEX" => Some(Self::OPENAI_CODEX),
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
            "OPENROUTER" => Some(Self::OPENROUTER),
            "OPENCODE" => Some(Self::OPENCODE),
            "FOUR_ROUTER" => Some(Self::FOUR_ROUTER),
            "NOUS_PORTAL" => Some(Self::NOUS_PORTAL),
            "INFINIAI" => Some(Self::INFINIAI),
            "ALIPAY_BAILING" => Some(Self::ALIPAY_BAILING),
            "DOUBAO" => Some(Self::DOUBAO),
            "NVIDIA" => Some(Self::NVIDIA),
            "LMSTUDIO" => Some(Self::LMSTUDIO),
            "OLLAMA" => Some(Self::OLLAMA),
            "OPENAI_LOCAL" => Some(Self::OPENAI_LOCAL),
            "LOCAL_MODEL" => Some(Self::LOCAL_MODEL),
            "PPINFRA" => Some(Self::PPINFRA),
            "NOVITA" => Some(Self::NOVITA),
            "MINIMAX" => Some(Self::MINIMAX),
            "OTHER" => Some(Self::OTHER),
            _ => None,
        }
    }

    /// Returns the stable provider type id used in persisted configuration.
    pub fn name(&self) -> &'static str {
        match self {
            Self::OPENAI => "OPENAI",
            Self::XAI => "XAI",
            Self::OPENAI_RESPONSES => "OPENAI_RESPONSES",
            Self::OPENAI_CODEX => "OPENAI_CODEX",
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
            Self::OPENROUTER => "OPENROUTER",
            Self::OPENCODE => "OPENCODE",
            Self::FOUR_ROUTER => "FOUR_ROUTER",
            Self::NOUS_PORTAL => "NOUS_PORTAL",
            Self::INFINIAI => "INFINIAI",
            Self::ALIPAY_BAILING => "ALIPAY_BAILING",
            Self::DOUBAO => "DOUBAO",
            Self::NVIDIA => "NVIDIA",
            Self::LMSTUDIO => "LMSTUDIO",
            Self::OLLAMA => "OLLAMA",
            Self::OPENAI_LOCAL => "OPENAI_LOCAL",
            Self::LOCAL_MODEL => "LOCAL_MODEL",
            Self::PPINFRA => "PPINFRA",
            Self::NOVITA => "NOVITA",
            Self::MINIMAX => "MINIMAX",
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
    pub const DEFAULT_SUMMARY_TOKEN_THRESHOLD: f32 = 0.85;
    pub const DEFAULT_ENABLE_SUMMARY: bool = true;
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
    pub cacheWritePricePerMillion: Option<f64>,
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
            cacheWritePricePerMillion: None,
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

/// Capability values the user set for one model; `None` takes what the model catalog says.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelCapabilityOverrides {
    #[serde(default)]
    pub directImage: Option<bool>,
    #[serde(default)]
    pub directAudio: Option<bool>,
    #[serde(default)]
    pub directVideo: Option<bool>,
    #[serde(default)]
    pub toolCall: Option<bool>,
}

impl ModelCapabilityOverrides {
    /// Returns whether the user set none of the capabilities.
    #[allow(non_snake_case)]
    pub fn isEmpty(&self) -> bool {
        self.directImage.is_none()
            && self.directAudio.is_none()
            && self.directVideo.is_none()
            && self.toolCall.is_none()
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
}

impl Default for ModelContextSpec {
    fn default() -> Self {
        Self {
            maxContextLength: ModelConfigDefaults::DEFAULT_MAX_CONTEXT_LENGTH,
        }
    }
}

impl ModelContextSpec {
    /// Converts the persisted kilo-token limit into an absolute token budget.
    #[allow(non_snake_case)]
    pub fn maxContextTokens(&self) -> i32 {
        (f64::from(self.maxContextLength) * 1000.0).clamp(0.0, f64::from(i32::MAX)) as i32
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelSummarySettings {
    pub enableSummary: bool,
    pub summaryTokenThreshold: f32,
}

impl Default for ModelSummarySettings {
    fn default() -> Self {
        Self {
            enableSummary: ModelConfigDefaults::DEFAULT_ENABLE_SUMMARY,
            summaryTokenThreshold: ModelConfigDefaults::DEFAULT_SUMMARY_TOKEN_THRESHOLD,
        }
    }
}

impl ModelSummarySettings {
    /// Returns the threshold as a context ratio, accepting legacy percent values.
    #[allow(non_snake_case)]
    pub fn tokenThresholdRatio(&self) -> f64 {
        let threshold = f64::from(self.summaryTokenThreshold);
        if threshold > 1.0 {
            (threshold / 100.0).clamp(0.0, 1.0)
        } else {
            threshold.clamp(0.0, 1.0)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalModelRuntimeSettings {
    /// Opaque Binding selecting the device that owns the installed model weights.
    #[serde(default)]
    pub inferenceBindingKey: String,
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
            inferenceBindingKey: String::new(),
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
    /// Uses Kotlin's optional one-hour TTL for Claude prompt-cache breakpoints.
    #[serde(default)]
    pub enableClaude1hPromptCache: bool,
}

impl Default for ModelRequestSpec {
    fn default() -> Self {
        Self {
            supportsStructuredTools: false,
            enableClaude1hPromptCache: false,
        }
    }
}

/// Request settings the user set for one model; `None` takes the catalog's value or the default.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelRequestOverrides {
    #[serde(default)]
    pub supportsStructuredTools: Option<bool>,
    #[serde(default)]
    pub enableClaude1hPromptCache: Option<bool>,
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
pub struct ProviderEndpointOption {
    pub endpoint: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ProviderCatalogEntry {
    pub providerTypeId: String,
    pub displayName: String,
    pub defaultEndpoint: String,
    pub endpointOptions: Vec<ProviderEndpointOption>,
    pub operations: Vec<ProviderOperationSpec>,
    pub models: Vec<ModelCatalogEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AvailableProviderModelSource {
    Catalog,
    Remote,
    Local,
}

impl AvailableProviderModelSource {
    /// Returns whether this source is a live listing rather than catalog history.
    pub fn isFetched(&self) -> bool {
        !matches!(self, Self::Catalog)
    }
}

/// What a provider's own model list said of one model.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelListedSpec {
    pub pricing: Option<ModelPricing>,
    pub context: Option<ModelContextSpec>,
    pub capabilities: Option<ModelCapabilities>,
    pub supportsStructuredTools: Option<bool>,
}

/// One model a provider offers, described as it would be once added.
///
/// `pricing`, `context`, `capabilities` and `request` are `None` where neither the model
/// catalog nor the provider's list says. `catalogModelId` names the catalog model it was
/// recognized as, and `listed` keeps what the provider's list itself said.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct AvailableProviderModel {
    pub modelId: String,
    pub source: AvailableProviderModelSource,
    #[serde(default)]
    pub catalogModelId: Option<String>,
    pub pricing: Option<ModelPricing>,
    pub context: Option<ModelContextSpec>,
    pub capabilities: Option<ModelCapabilities>,
    pub builtinTools: Vec<ModelBuiltinTool>,
    pub request: Option<ModelRequestSpec>,
    #[serde(default)]
    pub listed: Option<ModelListedSpec>,
}

impl AvailableProviderModel {
    /// Returns whether this option came from a live provider listing or local runtime.
    pub fn isFetched(&self) -> bool {
        self.source.isFetched()
    }
}

/// One configured model.
///
/// The `*Override` fields hold only what the user set. Every field left unset takes what
/// the model catalog says of the model `id` names, then what the provider's list said in
/// `listed`, then the defaults.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelProfile {
    pub id: String,
    pub pricingOverride: Option<ModelPricing>,
    pub contextOverride: Option<ModelContextSpec>,
    #[serde(default)]
    pub capabilitiesOverride: ModelCapabilityOverrides,
    pub builtinToolsOverride: Option<Vec<ModelBuiltinTool>>,
    #[serde(default)]
    pub requestOverride: ModelRequestOverrides,
    /// What the provider's model list said of the model when it was added.
    #[serde(default)]
    pub listed: Option<ModelListedSpec>,
    /// The thinking controls the user described the model with, replacing the catalog's.
    #[serde(default)]
    pub thinkingOverride: Option<ModelThinkingSpec>,
    /// The thinking level the user set on this model; `None` carries over the level last set
    /// on any model.
    #[serde(default)]
    pub thinkingLevel: Option<ThinkingLevel>,
    pub parameters: Vec<ModelParameter<serde_json::Value>>,
    pub summary: ModelSummarySettings,
    pub localRuntime: LocalModelRuntimeSettings,
}

impl ModelProfile {
    pub fn new(id: String) -> Self {
        Self {
            id,
            pricingOverride: None,
            contextOverride: None,
            capabilitiesOverride: ModelCapabilityOverrides::default(),
            builtinToolsOverride: None,
            requestOverride: ModelRequestOverrides::default(),
            listed: None,
            thinkingOverride: None,
            thinkingLevel: None,
            parameters: Vec::new(),
            summary: ModelSummarySettings::default(),
            localRuntime: LocalModelRuntimeSettings::default(),
        }
    }

    /// Returns whether the user set any pricing, context, capability, tool-format or
    /// thinking-control value.
    #[allow(non_snake_case)]
    pub fn hasSpecOverrides(&self) -> bool {
        self.pricingOverride.is_some()
            || self.contextOverride.is_some()
            || !self.capabilitiesOverride.isEmpty()
            || self.requestOverride.supportsStructuredTools.is_some()
            || self.thinkingOverride.is_some()
    }
}

/// Freshness of the model catalog downloaded on this device.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelCatalogStatus {
    /// When the catalog in use was downloaded, in Unix milliseconds; `None` before the first download.
    pub updatedAt: Option<i64>,
    /// Why the last download failed, when it did.
    pub lastError: Option<String>,
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
    /// Creates a provider profile with no models.
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
    /// The thinking controls the model accepts.
    pub thinking: ModelThinkingSpec,
    /// The thinking level requests ask for; `None` when requests cannot change how the
    /// model thinks.
    pub thinkingLevel: Option<ThinkingLevel>,
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
    pub context: ModelContextSpec,
    pub pricing: Option<ModelPricing>,
    /// The thinking controls the model accepts.
    pub thinking: ModelThinkingSpec,
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

/// Builds the default DeepSeek model profile with optional Responses search.
///
/// Tool calling is set up front so the first chat can call tools before the model
/// catalog has been downloaded; the rest follows the catalog.
pub fn default_deepseek_model() -> ModelProfile {
    let mut model = ModelProfile::new(ModelConfigDefaults::DEFAULT_MODEL_ID.to_string());
    model.capabilitiesOverride.toolCall = Some(true);
    model.requestOverride.supportsStructuredTools = Some(true);
    model.builtinToolsOverride = Some(vec![ModelBuiltinTool::disabled(
        BuiltinToolType::WebSearch,
        "内置联网搜索".to_string(),
        BuiltinToolRequestFormat::OpenAiWebSearch,
        BuiltinToolExclusivity::CanMixWithExternalTools,
    )]);
    model
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_budget_uses_decimal_kilo_tokens() {
        assert_eq!(
            ModelContextSpec {
                maxContextLength: 128.0
            }
            .maxContextTokens(),
            128_000
        );
        assert_eq!(
            ModelContextSpec {
                maxContextLength: 1000.0
            }
            .maxContextTokens(),
            1_000_000
        );
    }

    #[test]
    fn summary_threshold_accepts_ratio_and_legacy_percent() {
        let ratio = ModelSummarySettings {
            summaryTokenThreshold: 0.7,
            ..ModelSummarySettings::default()
        };
        assert!((ratio.tokenThresholdRatio() - 0.7).abs() < 1e-6);

        let percent = ModelSummarySettings {
            summaryTokenThreshold: 70.0,
            ..ModelSummarySettings::default()
        };
        assert!((percent.tokenThresholdRatio() - 0.7).abs() < 1e-6);
    }
}
