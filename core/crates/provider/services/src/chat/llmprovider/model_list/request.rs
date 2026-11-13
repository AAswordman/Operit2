//! Shared request configuration and non-Gemini discovery authentication.

use operit_model::ModelConfigData::{ProviderOperationSpec, ProviderProfile};
use serde_json::{Map, Value};

use super::{endpoint, DiscoveryProtocol};
use crate::chat::llmprovider::OpenCodeProvider::OpenCodeRouting;

/// Builds a catalog operation URL while preserving the model endpoint's base path.
pub(super) fn catalog_url(
    provider: &ProviderProfile,
    operation: &ProviderOperationSpec,
) -> Result<String, String> {
    let mut url = url::Url::parse(&provider.endpoint).map_err(|error| error.to_string())?;
    let path = match operation.operationType.as_str() {
        "list_models" => endpoint::models_path(&url, &operation.path)?,
        _ => operation.path.clone(),
    };
    url.set_path(&path);
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string())
}

/// Builds OpenCode's model URL below its configured Zen or Go endpoint.
pub(super) fn opencode_url(provider: &ProviderProfile) -> Result<String, String> {
    let mut url = url::Url::parse(&OpenCodeRouting::models_endpoint(&provider.endpoint))
        .map_err(|error| error.to_string())?;
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string())
}

/// Parses explicitly configured request headers as a JSON object.
pub(super) fn custom_headers(provider: &ProviderProfile) -> Result<Map<String, Value>, String> {
    match serde_json::from_str(&provider.customHeaders).map_err(|error| error.to_string())? {
        Value::Object(headers) => Ok(headers),
        _ => Err("customHeaders is not a JSON object".to_string()),
    }
}

/// Creates the content-type header shared by JSON discovery requests.
pub(super) fn json_headers() -> Vec<(String, String)> {
    vec![("Content-Type".to_string(), "application/json".to_string())]
}

/// Applies explicit custom headers by case-insensitive header name.
pub(super) fn merge_custom_headers(
    mut headers: Vec<(String, String)>,
    custom_headers: &Map<String, Value>,
) -> Result<Vec<(String, String)>, String> {
    for (name, value) in custom_headers {
        let header_value = value
            .as_str()
            .ok_or_else(|| format!("customHeaders value for {name} is not a string"))?;
        headers.retain(|(existing_name, _)| !existing_name.eq_ignore_ascii_case(name));
        headers.push((name.clone(), header_value.to_string()));
    }
    Ok(headers)
}

/// Builds Bearer discovery headers and the OpenCode-owned client identity.
pub(super) fn bearer_headers(
    protocol: DiscoveryProtocol,
    provider: &ProviderProfile,
    operation: &ProviderOperationSpec,
    custom_headers: &Map<String, Value>,
) -> Result<Vec<(String, String)>, String> {
    let mut headers = json_headers();
    let is_opencode = protocol == DiscoveryProtocol::OpenCode;
    let mut has_authorization = false;
    for (name, value) in custom_headers {
        let header_value = value
            .as_str()
            .ok_or_else(|| format!("customHeaders value for {name} is not a string"))?;
        if name.eq_ignore_ascii_case("Authorization") {
            if (operation.requiresApiKey || is_opencode) && header_value.trim().is_empty() {
                // Preserve the existing contract for empty legacy authorization headers.
                continue;
            }
            has_authorization = !header_value.trim().is_empty();
        }
        headers.push((name.clone(), header_value.to_string()));
    }
    if !has_authorization && (operation.requiresApiKey || is_opencode) {
        if let Some(api_key) = selected_api_key(provider) {
            headers.push(("Authorization".to_string(), bearer_authorization(api_key)));
        } else if operation.requiresApiKey {
            return Err("provider api key is required".to_string());
        }
    }
    if is_opencode {
        headers = OpenCodeRouting::request_headers(&provider.endpoint, &provider.id, headers);
    }
    Ok(headers)
}

/// Builds Anthropic's version and key headers with explicit custom overrides.
pub(super) fn anthropic_headers(
    provider: &ProviderProfile,
    operation: &ProviderOperationSpec,
    custom_headers: &Map<String, Value>,
) -> Result<Vec<(String, String)>, String> {
    let mut headers = json_headers();
    headers.push(("anthropic-version".to_string(), "2023-06-01".to_string()));
    if let Some(api_key) = selected_api_key(provider) {
        headers.push(("x-api-key".to_string(), api_key.to_string()));
    }
    let headers = merge_custom_headers(headers, custom_headers)?;
    if !headers.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case("anthropic-version") && !value.trim().is_empty()
    }) {
        return Err("Anthropic anthropic-version header is required".to_string());
    }
    if operation.requiresApiKey
        && !headers
            .iter()
            .any(|(name, value)| name.eq_ignore_ascii_case("x-api-key") && !value.trim().is_empty())
    {
        return Err("Anthropic x-api-key header is required".to_string());
    }
    Ok(headers)
}

/// Preserves an explicit Bearer prefix while normalizing a raw API key.
pub(super) fn bearer_authorization(api_key: &str) -> String {
    let api_key = api_key.trim();
    let mut parts = api_key.split_whitespace();
    if matches!(
        (parts.next(), parts.next()),
        (Some(scheme), Some(_)) if scheme.eq_ignore_ascii_case("Bearer")
    ) {
        return api_key.to_string();
    }
    format!("Bearer {api_key}")
}

/// Selects the configured API key, including rotation through enabled pool entries.
pub(super) fn selected_api_key(provider: &ProviderProfile) -> Option<&str> {
    if provider.useMultipleApiKeys {
        let api_keys: Vec<&str> = provider
            .apiKeyPool
            .iter()
            .filter(|info| info.isEnabled && !info.key.trim().is_empty())
            .map(|info| info.key.trim())
            .collect();
        if api_keys.is_empty() {
            return None;
        }
        let index = provider.currentKeyIndex.rem_euclid(api_keys.len() as i32) as usize;
        return Some(api_keys[index]);
    }
    let api_key = provider.apiKey.trim();
    if api_key.is_empty() {
        return None;
    }
    Some(api_key)
}
