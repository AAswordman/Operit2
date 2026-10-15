//! Configured model summaries exposed to plugins.
use super::{JsFuture, JsNullable};
use super::results::ModelsListSummariesResultData;
use serde::{Deserialize, Serialize};

/// Describes the actual configured model capability flags.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelsCapabilities {
    pub directImage: bool,
    pub directAudio: bool,
    pub directVideo: bool,
    pub toolCall: bool,
}
/// Selects the configured billing unit without changing its wire spelling.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ModelsBillingMode {
    TOKEN,
    COUNT,
}
/// Selects the configured pricing currency.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ModelsPricingCurrency {
    CNY,
    USD,
}
/// Preserves all configured model pricing fields, including explicit nullable prices.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelsPricing {
    pub billingMode: ModelsBillingMode,
    pub inputPricePerMillion: f64,
    pub cachedInputPricePerMillion: JsNullable<f64>,
    pub cacheWritePricePerMillion: JsNullable<f64>,
    pub outputPricePerMillion: f64,
    pub pricePerRequest: f64,
    pub currency: ModelsPricingCurrency,
}
/// Preserves a complete real provider/model summary returned by the host configuration manager.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelsSummary {
    pub providerId: String,
    pub providerName: String,
    pub providerTypeId: String,
    pub endpoint: String,
    pub modelId: String,
    pub capabilities: ModelsCapabilities,
    pub pricing: JsNullable<ModelsPricing>,
}
/// Reads configured model summaries from the canonical host manager.
pub trait ModelsHost: Send + Sync {
    /// Returns complete provider/model summaries, capabilities and pricing.
    fn listSummaries(&self) -> JsFuture<ModelsListSummariesResultData>;
}
