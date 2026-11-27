#![allow(non_camel_case_types, non_snake_case)]

use serde::{Deserialize, Serialize};

use crate::ModelConfigData::ApiProviderType;

/// Classifies how a provider's remaining capacity or usage cost is billed and tracked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuotaAccountKind {
    BALANCE,
    SUBSCRIPTION,
    LOCAL_COMPUTE,
    UNSUPPORTED,
}

impl Default for QuotaAccountKind {
    fn default() -> Self {
        Self::UNSUPPORTED
    }
}

/// Identifies the rolling reset cadence or ledger category of one quota window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuotaWindowKind {
    HOUR_5,
    DAY_1,
    DAY_7,
    MONTH,
    BALANCE,
    TOTAL,
    CUSTOM,
}

impl Default for QuotaWindowKind {
    fn default() -> Self {
        Self::CUSTOM
    }
}

/// Reports the latest probe status for one provider profile or pooled API key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuotaProbeStatus {
    OK,
    UNSUPPORTED,
    UNAUTHORIZED,
    ERROR,
    STALE,
}

impl Default for QuotaProbeStatus {
    fn default() -> Self {
        Self::STALE
    }
}

/// Selects the explicit remote balance or quota wire protocol without implicit fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderBalanceProtocol {
    AUTO,
    DEEPSEEK_BALANCE,
    OPENROUTER_KEY,
    OPENROUTER_CREDITS,
    ZHIPU_CODING_PLAN,
    ZHIPU_ACCOUNT_REPORT,
    MINIMAX_TOKEN_PLAN,
    MOONSHOT_BALANCE,
    KIMI_CODING_USAGES,
    SILICONFLOW_USER_INFO,
    CODEX_WHAM_USAGE,
    NEW_API_TOKEN,
    OPENAI_LEGACY_BILLING,
    CUSTOM_HTTP_JSON,
    LOCAL_COMPUTE,
    UNSUPPORTED,
}

impl Default for ProviderBalanceProtocol {
    fn default() -> Self {
        Self::AUTO
    }
}

/// Defines whether clearing usage history affects only this node or the entire Space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsageResetScope {
    LOCAL_ONLY,
    ALL_DEVICES_IN_SPACE,
}

impl Default for UsageResetScope {
    fn default() -> Self {
        Self::LOCAL_ONLY
    }
}

/// Captures the terminal execution outcome of a single LLM request attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsageRequestStatus {
    COMPLETED,
    CANCELLED,
    FAILED,
}

impl Default for UsageRequestStatus {
    fn default() -> Self {
        Self::COMPLETED
    }
}

/// Represents one cyclical quota window or ledger limit modelled after Magpie's `QuotaWindow`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuotaWindow {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub kind: QuotaWindowKind,
    #[serde(default)]
    pub display: String,
    #[serde(default)]
    pub used: Option<f64>,
    #[serde(default)]
    pub remaining: Option<f64>,
    #[serde(default)]
    pub limit: Option<f64>,
    #[serde(default)]
    pub usedPercent: Option<f64>,
    #[serde(default)]
    pub unlimited: bool,
    #[serde(default)]
    pub resetsAtMs: Option<i64>,
    #[serde(default)]
    pub resetSecs: Option<i64>,
    #[serde(default)]
    pub spanLabel: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
}

/// Represents normalized currency or credit balance fields returned by pay-as-you-go providers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuotaBalanceBreakdown {
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub totalBalance: Option<f64>,
    #[serde(default)]
    pub cashBalance: Option<f64>,
    #[serde(default)]
    pub grantedBalance: Option<f64>,
    #[serde(default)]
    pub usedAmount: Option<f64>,
    #[serde(default)]
    pub dailyUsed: Option<f64>,
    #[serde(default)]
    pub monthlyUsed: Option<f64>,
    #[serde(default)]
    pub isAvailable: bool,
    #[serde(default)]
    pub isFreeTier: bool,
}

/// Sanitized balance/window snapshot for one enabled key in a provider pool without raw secrets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyQuotaSnapshot {
    #[serde(default)]
    pub keyId: String,
    #[serde(default)]
    pub keyName: Option<String>,
    #[serde(default)]
    pub keyFingerprint: String,
    #[serde(default)]
    pub isEnabled: bool,
    #[serde(default)]
    pub status: QuotaProbeStatus,
    #[serde(default)]
    pub balance: Option<QuotaBalanceBreakdown>,
    #[serde(default)]
    pub windows: Vec<QuotaWindow>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub asOfMs: i64,
}

/// Sanitized balance and rolling quota snapshot for one provider profile across a Space.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProviderQuotaSnapshot {
    #[serde(default)]
    pub providerId: String,
    #[serde(default)]
    pub providerName: String,
    pub providerType: ApiProviderType,
    #[serde(default)]
    pub providerTypeId: String,
    #[serde(default)]
    pub accountKind: QuotaAccountKind,
    #[serde(default)]
    pub protocol: ProviderBalanceProtocol,
    #[serde(default)]
    pub status: QuotaProbeStatus,
    #[serde(default)]
    pub plan: Option<String>,
    #[serde(default)]
    pub accountLabel: Option<String>,
    #[serde(default)]
    pub balance: Option<QuotaBalanceBreakdown>,
    #[serde(default)]
    pub windows: Vec<QuotaWindow>,
    #[serde(default)]
    pub keySnapshots: Vec<KeyQuotaSnapshot>,
    #[serde(default)]
    pub consoleUrl: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub held: bool,
    #[serde(default)]
    pub asOfMs: i64,
    #[serde(default)]
    pub probedByNodeId: String,
    #[serde(default)]
    pub probedByDeviceName: String,
}

/// Per-provider balance probe configuration and preferred Space execution node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderQuotaProbeConfig {
    #[serde(default)]
    pub providerId: String,
    #[serde(default)]
    pub protocol: ProviderBalanceProtocol,
    #[serde(default)]
    pub customPath: Option<String>,
    #[serde(default)]
    pub customAmountJsonPath: Option<String>,
    #[serde(default)]
    pub customCurrencyJsonPath: Option<String>,
    #[serde(default)]
    pub customCurrency: Option<String>,
    #[serde(default)]
    pub preferredProbeNodeId: Option<String>,
}

/// User-defined spending cap for providers without public key-queryable balance APIs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProviderLocalBudget {
    #[serde(default)]
    pub providerId: String,
    #[serde(default)]
    pub monthlyBudget: f64,
    #[serde(default)]
    pub totalBudget: Option<f64>,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub resetDayOfMonth: u8,
    #[serde(default)]
    pub alertThresholdPercent: f64,
    #[serde(default)]
    pub updatedAtMs: i64,
}

/// Combined Space-synced quota center state read by the Settings Usage & Quota dashboard.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProviderQuotaCenterState {
    #[serde(default)]
    pub snapshots: Vec<ProviderQuotaSnapshot>,
    #[serde(default)]
    pub budgets: Vec<ProviderLocalBudget>,
    #[serde(default)]
    pub probeConfigs: Vec<ProviderQuotaProbeConfig>,
}

/// Normalized local balance/key probe response produced by `operit-providers`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProviderQuotaProbeOutcome {
    pub accountKind: QuotaAccountKind,
    pub protocol: ProviderBalanceProtocol,
    pub status: QuotaProbeStatus,
    pub plan: Option<String>,
    pub accountLabel: Option<String>,
    pub balance: Option<QuotaBalanceBreakdown>,
    pub windows: Vec<QuotaWindow>,
    pub consoleUrl: Option<String>,
    pub error: Option<String>,
}

/// Masks a secret token into a non-sensitive display fingerprint (`sk-...1234`).
pub fn mask_api_key_fingerprint(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= 8 {
        let tail: String = chars[chars.len().saturating_sub(2)..].iter().collect();
        return format!("***{tail}");
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}...{tail}")
}

/// Returns the canonical ISO currency code for one configured model pricing rule.
pub fn pricing_currency_code(currency: &crate::ModelConfigData::PricingCurrency) -> &'static str {
    match currency {
        crate::ModelConfigData::PricingCurrency::CNY => "CNY",
        crate::ModelConfigData::PricingCurrency::USD => "USD",
    }
}

/// Computes estimated request cost in the model's declared pricing currency.
pub fn calculate_model_cost(
    pricing: &crate::ModelConfigData::ModelPricing,
    input_tokens: i64,
    cached_input_tokens: i64,
    cache_write_tokens: i64,
    output_tokens: i64,
) -> f64 {
    match pricing.billingMode {
        crate::BillingMode::BillingMode::COUNT => pricing.pricePerRequest.max(0.0),
        crate::BillingMode::BillingMode::TOKEN => {
            let safe_input = input_tokens.max(0) as f64;
            let safe_cached = cached_input_tokens.max(0) as f64;
            let safe_write = cache_write_tokens.max(0) as f64;
            let safe_output = output_tokens.max(0) as f64;
            let uncached_input = (safe_input - safe_cached).max(0.0);
            let input_cost = uncached_input * pricing.inputPricePerMillion.max(0.0) / 1_000_000.0;
            let cached_cost = safe_cached
                * pricing
                    .cachedInputPricePerMillion
                    .unwrap_or(pricing.inputPricePerMillion)
                    .max(0.0)
                / 1_000_000.0;
            let write_cost =
                safe_write * pricing.cacheWritePricePerMillion.unwrap_or(0.0).max(0.0) / 1_000_000.0;
            let output_cost = safe_output * pricing.outputPricePerMillion.max(0.0) / 1_000_000.0;
            input_cost + cached_cost + write_cost + output_cost
        }
    }
}
