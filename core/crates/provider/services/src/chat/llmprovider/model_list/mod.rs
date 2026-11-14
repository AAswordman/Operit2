//! Provider protocol routing for model discovery.

mod endpoint;
mod gemini;
mod request;
mod response;

#[cfg(test)]
mod tests;

use operit_model::ModelConfigData::{
    ApiProviderType, AvailableProviderModel, ProviderOperationSpec, ProviderProfile,
};
use serde_json::Value;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum DiscoveryProtocol {
    Bearer,
    Anthropic,
    Gemini,
    OpenCode,
}

impl DiscoveryProtocol {
    /// Declares where the shared transport must encode the request's credential.
    pub(super) fn authentication(
        self,
    ) -> crate::chat::llmprovider::ProviderHttp::KeyAuthentication {
        use crate::chat::llmprovider::ProviderHttp::KeyAuthentication;
        match self {
            Self::Gemini => KeyAuthentication::Query("key".into()),
            Self::Anthropic => KeyAuthentication::Header("x-api-key".into()),
            Self::Bearer | Self::OpenCode => KeyAuthentication::Bearer,
        }
    }
    /// Selects the model-discovery protocol once from the configured provider type.
    pub(super) fn for_provider(provider_type: &ApiProviderType) -> Self {
        match provider_type {
            ApiProviderType::GOOGLE | ApiProviderType::GEMINI_GENERIC => Self::Gemini,
            ApiProviderType::ANTHROPIC | ApiProviderType::ANTHROPIC_GENERIC => Self::Anthropic,
            ApiProviderType::OPENCODE => Self::OpenCode,
            _ => Self::Bearer,
        }
    }

    /// Builds the URL using the selected discovery protocol and catalog operation.
    pub(super) fn request_url(
        self,
        provider: &ProviderProfile,
        operation: &ProviderOperationSpec,
    ) -> Result<String, String> {
        match self {
            Self::Gemini => gemini::request_url(provider, operation),
            Self::OpenCode if operation.operationType == "list_models" => {
                request::opencode_url(provider)
            }
            _ => request::catalog_url(provider, operation),
        }
    }

    /// Builds protocol authentication headers with explicit user header configuration.
    pub(super) fn request_headers(
        self,
        provider: &ProviderProfile,
    ) -> Result<Vec<(String, String)>, String> {
        let custom_headers = request::custom_headers(provider)?;
        match self {
            Self::Gemini => request::merge_custom_headers(request::json_headers(), &custom_headers),
            Self::Anthropic => request::anthropic_headers(&custom_headers),
            Self::Bearer | Self::OpenCode => {
                request::bearer_headers(self, provider, &custom_headers)
            }
        }
    }

    /// Selects catalog-declared response items and parses them with the discovery protocol.
    pub(super) fn parse_response(
        self,
        response: &Value,
        operation: &ProviderOperationSpec,
    ) -> Result<Vec<AvailableProviderModel>, String> {
        self.parse_items(response::model_items(response, operation)?, operation)
    }

    /// Parses models using protocol-specific discovery semantics.
    fn parse_items(
        self,
        items: &[Value],
        operation: &ProviderOperationSpec,
    ) -> Result<Vec<AvailableProviderModel>, String> {
        match self {
            Self::Gemini => gemini::parse_items(items, operation),
            _ => response::parse_items(items, operation),
        }
    }
}
