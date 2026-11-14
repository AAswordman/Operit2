//! Gemini model discovery, query-key authentication, and generation-model filtering.

use operit_model::ModelConfigData::{
    AvailableProviderModel, ProviderOperationSpec, ProviderProfile,
};
use serde_json::Value;

use super::response;

/// Builds Gemini's versioned model URL with the configured API key as a query parameter.
pub(super) fn request_url(
    provider: &ProviderProfile,
    operation: &ProviderOperationSpec,
) -> Result<String, String> {
    let mut url = url::Url::parse(&provider.endpoint).map_err(|error| error.to_string())?;
    let segments: Vec<_> = url
        .path_segments()
        .ok_or_else(|| "Google model-list endpoint must have URL path segments".to_string())?
        .collect();
    let path = match segments.as_slice() {
        [] | [""] => operation.path.clone(),
        [version @ ("v1" | "v1beta")]
        | [version @ ("v1" | "v1beta"), ""]
        | [version @ ("v1" | "v1beta"), "models", ..] => format!("/{version}/models"),
        _ => {
            return Err(format!(
                "unsupported Google model-list endpoint path: {}",
                url.path()
            ))
        }
    };
    url.set_path(&path);
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string())
}

/// Parses declared generation models using Kotlin's base model IDs and ordering.
pub(super) fn parse_items(
    items: &[Value],
    operation: &ProviderOperationSpec,
) -> Result<Vec<AvailableProviderModel>, String> {
    let mut models = Vec::new();
    for item in items {
        if !supports_generation(item)? {
            continue;
        }
        let base_model_id = item
            .get("baseModelId")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| {
                "Google generation model baseModelId must be a non-empty string".to_string()
            })?;
        let mut model = response::parse_item(item, operation)?;
        model.modelId = base_model_id.to_string();
        models.push(model);
    }
    models.sort_by(|left, right| left.modelId.cmp(&right.modelId));
    Ok(models)
}

/// Checks declared generation methods without assuming support for missing metadata.
fn supports_generation(item: &Value) -> Result<bool, String> {
    let methods = item
        .get("supportedGenerationMethods")
        .and_then(Value::as_array)
        .ok_or_else(|| "Google model supportedGenerationMethods must be an array".to_string())?;
    let mut supports_generation = false;
    for method in methods {
        let method = method.as_str().ok_or_else(|| {
            "Google model supportedGenerationMethods entries must be strings".to_string()
        })?;
        supports_generation |= method == "generateContent";
    }
    Ok(supports_generation)
}
