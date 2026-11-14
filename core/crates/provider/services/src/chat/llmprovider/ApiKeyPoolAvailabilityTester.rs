use super::ModelConfigConnectionTester::ModelConfigConnectionTester;
use crate::runtime_support::ProviderRuntimeContext;
use operit_model::ApiKeyInfo::{ApiKeyAvailabilityStatus, ApiKeyInfo};
use operit_model::ModelConfigData::ResolvedModelConfig;

/// Tests one draft pool entry without mutating the saved provider or preferred key.
pub async fn test_pool_entry(
    mut entry: ApiKeyInfo,
    mut config: ResolvedModelConfig,
    context: ProviderRuntimeContext,
) -> ApiKeyInfo {
    config.apiKey = entry.key.trim().to_string();
    config.useMultipleApiKeys = false;
    config.apiKeyPool.clear();
    entry.availabilityStatus = if ModelConfigConnectionTester::testApiKey(config, context)
        .await
        .is_ok()
    {
        ApiKeyAvailabilityStatus::AVAILABLE
    } else {
        ApiKeyAvailabilityStatus::UNAVAILABLE
    };
    entry
}
