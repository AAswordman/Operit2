//! Catalog-driven model response parsing and metadata extraction.

use operit_model::BillingMode::BillingMode;
use operit_model::ModelConfigData::{
    AvailableProviderModel, AvailableProviderModelSource, ModelCapabilities, ModelContextSpec,
    ModelPricing, ModelRequestSpec, PricingCurrency, ProviderOperationSpec,
};
use serde_json::Value;

/// Selects the model array declared by the catalog operation.
pub(super) fn model_items<'a>(
    response: &'a Value,
    operation: &ProviderOperationSpec,
) -> Result<&'a [Value], String> {
    let items_path = operation
        .result
        .itemsJsonPath
        .as_deref()
        .ok_or_else(|| "list_models operation missing itemsJsonPath".to_string())?;
    let items = select_json_path(response, items_path)
        .ok_or_else(|| "list_models response items not found".to_string())?
        .as_array()
        .ok_or_else(|| "list_models response items is not an array".to_string())?;
    Ok(items)
}

/// Parses model options declared by the catalog operation.
pub(super) fn parse_items(
    items: &[Value],
    operation: &ProviderOperationSpec,
) -> Result<Vec<AvailableProviderModel>, String> {
    items
        .iter()
        .map(|item| parse_item(item, operation))
        .collect()
}

/// Parses one provider model item into a runtime model option.
pub(super) fn parse_item(
    item: &Value,
    operation: &ProviderOperationSpec,
) -> Result<AvailableProviderModel, String> {
    let model_id = read_required_string(
        item,
        operation
            .result
            .itemIdJsonPath
            .as_deref()
            .ok_or_else(|| "list_models operation missing itemIdJsonPath".to_string())?,
    )?;
    let capabilities = read_capabilities(item, operation)?;
    let request = read_request(item, operation)?;
    Ok(AvailableProviderModel {
        modelId: model_id,
        source: AvailableProviderModelSource::Remote,
        catalogModelId: None,
        pricing: read_pricing(item, operation)?,
        context: read_context(item, operation)?,
        capabilities,
        builtinTools: Vec::new(),
        request,
        listed: None,
    })
}

/// Reads pricing metadata from one provider model item.
fn read_pricing(
    item: &Value,
    operation: &ProviderOperationSpec,
) -> Result<Option<ModelPricing>, String> {
    let input = read_optional_f64(item, operation.result.inputPricePerTokenJsonPath.as_deref())?;
    let output = read_optional_f64(
        item,
        operation.result.outputPricePerTokenJsonPath.as_deref(),
    )?;
    let currency = read_optional_string(item, operation.result.currencyJsonPath.as_deref())?;
    match (input, output, currency) {
        (Some(input), Some(output), Some(currency)) => Ok(Some(ModelPricing {
            billingMode: BillingMode::TOKEN,
            inputPricePerMillion: input * 1_000_000.0,
            cachedInputPricePerMillion: read_optional_f64(
                item,
                operation.result.cachedInputPricePerTokenJsonPath.as_deref(),
            )?
            .map(|value| value * 1_000_000.0),
            cacheWritePricePerMillion: None,
            outputPricePerMillion: output * 1_000_000.0,
            pricePerRequest: read_optional_f64(
                item,
                operation.result.pricePerRequestJsonPath.as_deref(),
            )?
            .unwrap_or(0.0),
            currency: parse_currency(&currency)?,
        })),
        _ => Ok(None),
    }
}

/// Reads context-window metadata from one provider model item.
fn read_context(
    item: &Value,
    operation: &ProviderOperationSpec,
) -> Result<Option<ModelContextSpec>, String> {
    let max_context_length =
        read_optional_f32(item, operation.result.maxContextLengthJsonPath.as_deref())?;
    match max_context_length {
        Some(max_context_length) => Ok(Some(ModelContextSpec {
            maxContextLength: max_context_length / 1000.0,
        })),
        None => Ok(None),
    }
}

/// Reads model capability metadata from one provider model item.
fn read_capabilities(
    item: &Value,
    operation: &ProviderOperationSpec,
) -> Result<Option<ModelCapabilities>, String> {
    let values = [
        read_optional_bool(item, operation.result.directImageJsonPath.as_deref())?,
        read_optional_bool(item, operation.result.directAudioJsonPath.as_deref())?,
        read_optional_bool(item, operation.result.directVideoJsonPath.as_deref())?,
        read_optional_bool(item, operation.result.toolCallJsonPath.as_deref())?,
    ];
    if values.iter().all(Option::is_none) {
        return Ok(None);
    }
    Ok(Some(ModelCapabilities {
        directImage: values[0].unwrap_or(false),
        directAudio: values[1].unwrap_or(false),
        directVideo: values[2].unwrap_or(false),
        toolCall: values[3].unwrap_or(false),
    }))
}

/// Reads request-shape metadata from one provider model item; `None` when the list says nothing.
fn read_request(
    item: &Value,
    operation: &ProviderOperationSpec,
) -> Result<Option<ModelRequestSpec>, String> {
    let supports_structured_tools = read_optional_bool(
        item,
        operation.result.supportsStructuredToolsJsonPath.as_deref(),
    )?;
    Ok(supports_structured_tools.map(|supports_structured_tools| ModelRequestSpec {
        supportsStructuredTools: supports_structured_tools,
        ..ModelRequestSpec::default()
    }))
}

/// Reads a required string from one JSON path or literal spec.
fn read_required_string(item: &Value, spec: &str) -> Result<String, String> {
    read_optional_string(item, Some(spec))?
        .ok_or_else(|| format!("required value not found: {spec}"))
}

/// Reads an optional string from one JSON path or literal spec.
fn read_optional_string(item: &Value, spec: Option<&str>) -> Result<Option<String>, String> {
    let Some(spec) = spec else {
        return Ok(None);
    };
    if !spec.starts_with('$') {
        return Ok(Some(spec.to_string()));
    }
    Ok(select_json_path(item, spec).and_then(json_value_string))
}

/// Reads an optional f64 value from one JSON path or literal spec.
fn read_optional_f64(item: &Value, spec: Option<&str>) -> Result<Option<f64>, String> {
    let Some(value) = read_optional_string(item, spec)? else {
        return Ok(None);
    };
    value
        .parse::<f64>()
        .map(Some)
        .map_err(|error| error.to_string())
}

/// Reads an optional f32 value from one JSON path or literal spec.
fn read_optional_f32(item: &Value, spec: Option<&str>) -> Result<Option<f32>, String> {
    let Some(value) = read_optional_string(item, spec)? else {
        return Ok(None);
    };
    value
        .parse::<f32>()
        .map(Some)
        .map_err(|error| error.to_string())
}

/// Reads an optional boolean from one JSON path, literal spec, or containment spec.
fn read_optional_bool(item: &Value, spec: Option<&str>) -> Result<Option<bool>, String> {
    let Some(spec) = spec else {
        return Ok(None);
    };
    if let Some((path, expected)) = spec.split_once('~') {
        let Some(value) = select_json_path(item, path) else {
            return Ok(Some(false));
        };
        return Ok(Some(json_contains(value, expected)));
    }
    if !spec.starts_with('$') {
        return spec
            .parse::<bool>()
            .map(Some)
            .map_err(|error| error.to_string());
    }
    Ok(select_json_path(item, spec).and_then(Value::as_bool))
}

/// Selects a JSON value using the catalog's dot-and-index path syntax.
fn select_json_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    if path == "$" {
        return Some(value);
    }
    let mut current = value;
    let path = path.strip_prefix("$.")?;
    for segment in path.split('.') {
        current = select_segment(current, segment)?;
    }
    Some(current)
}

/// Selects one named or indexed segment from a JSON value.
fn select_segment<'a>(value: &'a Value, segment: &str) -> Option<&'a Value> {
    let mut current = value;
    let mut rest = segment;
    let name_end = rest.find('[').unwrap_or(rest.len());
    let name = &rest[..name_end];
    if !name.is_empty() {
        current = current.get(name)?;
    }
    rest = &rest[name_end..];
    while !rest.is_empty() {
        let end = rest.find(']')?;
        let index = rest[1..end].parse::<usize>().ok()?;
        current = current.as_array()?.get(index)?;
        rest = &rest[end + 1..];
    }
    Some(current)
}

/// Converts scalar JSON values into strings used by catalog readers.
fn json_value_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

/// Tests whether a JSON scalar or array contains the expected catalog value.
fn json_contains(value: &Value, expected: &str) -> bool {
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(json_value_string)
            .any(|value| value == expected),
        _ => json_value_string(value)
            .map(|value| value == expected)
            .unwrap_or(false),
    }
}

/// Parses a provider pricing currency literal.
fn parse_currency(value: &str) -> Result<PricingCurrency, String> {
    match value.trim().to_ascii_uppercase().as_str() {
        "CNY" => Ok(PricingCurrency::CNY),
        "USD" => Ok(PricingCurrency::USD),
        other => Err(format!("invalid pricing currency: {other}")),
    }
}
