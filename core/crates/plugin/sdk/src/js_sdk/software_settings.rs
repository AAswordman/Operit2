//! Environment-variable access and core command execution exposed to plugins.
use super::results::*;
use super::{JsDate, JsFuture, JsNullable};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Describes the actual configured model capability flags.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct SoftwareModelCapabilities {
    pub directImage: bool,
    pub directAudio: bool,
    pub directVideo: bool,
    pub toolCall: bool,
}
/// Selects the configured billing unit without changing its wire spelling.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SoftwareModelBillingMode {
    TOKEN,
    COUNT,
}
/// Selects the configured pricing currency.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SoftwareModelPricingCurrency {
    CNY,
    USD,
}
/// Preserves all configured model pricing fields, including explicit nullable prices.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct SoftwareModelPricing {
    pub billingMode: SoftwareModelBillingMode,
    pub inputPricePerMillion: f64,
    pub cachedInputPricePerMillion: JsNullable<f64>,
    pub cacheWritePricePerMillion: JsNullable<f64>,
    pub outputPricePerMillion: f64,
    pub pricePerRequest: f64,
    pub currency: SoftwareModelPricingCurrency,
}
/// Preserves a complete real provider/model summary returned by the host configuration manager.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct SoftwareModelSummary {
    pub providerId: String,
    pub providerName: String,
    pub providerTypeId: String,
    pub endpoint: String,
    pub modelId: String,
    pub capabilities: SoftwareModelCapabilities,
    pub pricing: JsNullable<SoftwareModelPricing>,
}
/// Preserves one configured speech request or response-pipeline header.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoftwareTtsHeader {
    pub name: String,
    pub value: String,
}
/// Preserves every configured speech response-pipeline field.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct SoftwareTtsResponseStep {
    pub stepType: String,
    pub path: String,
    pub headers: Vec<SoftwareTtsHeader>,
}
/// Preserves the complete speech configuration returned by the host manager.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct SoftwareTtsConfig {
    pub id: String,
    pub name: String,
    pub providerType: String,
    pub endpoint: String,
    pub apiKey: String,
    pub model: String,
    pub voice: String,
    pub responseFormat: String,
    pub speed: f64,
    pub httpMethod: String,
    pub requestBody: String,
    pub contentType: String,
    pub headers: Vec<SoftwareTtsHeader>,
    pub responsePipeline: Vec<SoftwareTtsResponseStep>,
    pub createdAt: i64,
    pub updatedAt: i64,
}
/// Preserves one ordinary named appearance configuration from the canonical user-preference ledger.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct SoftwareThemeConfig {
    pub id: String,
    pub name: String,
    /// Contains every key and exact value validated by the ordinary host appearance snapshot contract.
    pub snapshot: BTreeMap<String, serde_json::Value>,
    pub createdAt: i64,
    pub updatedAt: i64,
}
/// Identifies one actual configured tool source or registered built-in tool.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct SoftwareToolSource {
    pub name: String,
    pub displayName: String,
    pub description: String,
}
/// Separates the actual canonical source registries rather than classifying names heuristically.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct SoftwareToolSourceCatalog {
    pub builtinTools: Vec<SoftwareToolSource>,
    pub packages: Vec<SoftwareToolSource>,
    pub skills: Vec<SoftwareToolSource>,
    pub mcpServers: Vec<SoftwareToolSource>,
}

/// Reads host configuration directories, manages environment variables and executes core commands.
pub trait SoftwareSettingsHost: Send + Sync {
    /// Returns complete configured model summaries directly from the canonical host manager.
    fn listModelSummaries(&self) -> JsFuture<Vec<SoftwareModelSummary>>;
    /// Returns complete configured speech records directly from the canonical host manager.
    fn listTtsConfigs(&self) -> JsFuture<Vec<SoftwareTtsConfig>>;
    /// Lists independent named themes from the same canonical ledger as ordinary appearance settings.
    fn listThemeConfigs(&self) -> JsFuture<Vec<SoftwareThemeConfig>>;
    /// Applies the complete existing named snapshot and active ID in the canonical preference transaction.
    fn applyThemeConfig(&self, id: String) -> JsFuture<SoftwareThemeConfig>;
    /// Reads the actual current canonical speech configuration without a plugin-usage lookup.
    fn getCurrentTtsConfigId(&self) -> JsFuture<String>;
    /// Selects an existing exact speech ID; invalid IDs reject without changing the current configuration.
    fn setCurrentTtsConfigId(&self, id: String) -> JsFuture<String>;
    /// Returns actual builtin, package, skill and MCP source identities from their own registries.
    fn readToolSourceCatalog(&self) -> JsFuture<SoftwareToolSourceCatalog>;
    ///
    ///Read current value of an environment variable.
    ///@param key - Environment variable key
    ///
    fn readEnvironmentVariable(&self, key: String) -> JsFuture<EnvironmentVariableReadResultData>;
    ///
    ///Write an environment variable; empty value clears the variable.
    ///@param key - Environment variable key
    ///@param value - Variable value (empty string clears)
    ///
    fn writeEnvironmentVariable(
        &self,
        key: String,
        value: Option<String>,
    ) -> JsFuture<EnvironmentVariableWriteResultData>;
    ///
    ///Execute a core command with CLI-style arguments.
    ///@param args - Command arguments, for example ['plugin', 'list']
    ///
    fn exec(&self, args: Vec<String>) -> JsFuture<String>;
}
