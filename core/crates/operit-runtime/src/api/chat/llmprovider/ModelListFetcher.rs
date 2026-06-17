#[cfg(not(target_arch = "wasm32"))]
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use std::thread;

use crate::data::model::BillingMode::BillingMode;
use crate::data::model::ModelConfigData::{
    AvailableProviderModel, AvailableProviderModelSource, ModelCapabilities, ModelContextSpec,
    ModelPricing, ModelRequestSpec, PricingCurrency, ProviderCatalogEntry, ProviderOperationSpec,
    ProviderProfile,
};

pub struct ModelListFetcher;

impl ModelListFetcher {
    #[allow(non_snake_case)]
    pub fn fetch(
        provider: &ProviderProfile,
        providerCatalog: &ProviderCatalogEntry,
    ) -> Result<Vec<AvailableProviderModel>, String> {
        let operation = match providerCatalog
            .operations
            .iter()
            .find(|operation| operation.operationType == "list_models")
        {
            Some(operation) => operation,
            None => return Ok(Vec::new()),
        };

        let response = Self::requestJson(provider, operation)?;
        let items = selectJsonPath(
            &response,
            operation
                .result
                .itemsJsonPath
                .as_deref()
                .ok_or_else(|| "list_models operation missing itemsJsonPath".to_string())?,
        )
        .ok_or_else(|| "list_models response items not found".to_string())?
        .as_array()
        .ok_or_else(|| "list_models response items is not an array".to_string())?;

        items
            .iter()
            .map(|item| Self::parseItem(item, operation))
            .collect()
    }

    #[allow(non_snake_case)]
    fn requestJson(
        provider: &ProviderProfile,
        operation: &ProviderOperationSpec,
    ) -> Result<Value, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = provider;
            let _ = operation;
            return Err(
                "list_models http_json operation is not available in wasm runtime".to_string(),
            );
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if operation.handlerId != "http_json" {
                return Err(format!(
                    "unsupported provider operation handler: {}",
                    operation.handlerId
                ));
            }
            if operation.method != "GET" {
                return Err(format!(
                    "unsupported provider operation method: {}",
                    operation.method
                ));
            }

            let url = operationUrl(&provider.endpoint, &operation.path)?;
            let headers = headers(provider, operation)?;
            let response = thread::spawn(move || {
                let response = Client::new()
                    .get(url)
                    .headers(headers)
                    .send()
                    .map_err(|error| error.to_string())?;
                let status = response.status();
                let body = response.text().map_err(|error| error.to_string())?;
                Ok::<(reqwest::StatusCode, String), String>((status, body))
            })
            .join()
            .map_err(|_| "list_models request thread panicked".to_string())??;
            let (status, body) = response;
            if !status.is_success() {
                return Err(format!("list_models request failed: {status} {body}"));
            }
            serde_json::from_str(&body).map_err(|error| error.to_string())
        }
    }

    #[allow(non_snake_case)]
    fn parseItem(
        item: &Value,
        operation: &ProviderOperationSpec,
    ) -> Result<AvailableProviderModel, String> {
        let modelId = readRequiredString(
            item,
            operation
                .result
                .itemIdJsonPath
                .as_deref()
                .ok_or_else(|| "list_models operation missing itemIdJsonPath".to_string())?,
        )?;
        let capabilities = readCapabilities(item, operation)?;
        let request = readRequest(item, operation)?;
        Ok(AvailableProviderModel {
            modelId,
            source: AvailableProviderModelSource::Remote,
            pricing: readPricing(item, operation)?,
            context: readContext(item, operation)?,
            capabilities,
            builtinTools: Vec::new(),
            request: Some(request),
        })
    }
}

#[allow(non_snake_case)]
fn operationUrl(endpoint: &str, path: &str) -> Result<String, String> {
    let mut url = url::Url::parse(endpoint).map_err(|error| error.to_string())?;
    url.set_path(path);
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string())
}

fn headers(
    provider: &ProviderProfile,
    operation: &ProviderOperationSpec,
) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    if operation.requiresApiKey {
        let value = HeaderValue::from_str(&format!("Bearer {}", apiKey(provider)?))
            .map_err(|error| error.to_string())?;
        headers.insert(AUTHORIZATION, value);
    }
    let customHeaders = serde_json::from_str::<serde_json::Value>(&provider.customHeaders)
        .map_err(|error| error.to_string())?;
    let object = customHeaders
        .as_object()
        .ok_or_else(|| "customHeaders is not a JSON object".to_string())?;
    for (name, value) in object {
        let headerValue = value
            .as_str()
            .ok_or_else(|| format!("customHeaders value for {name} is not a string"))?;
        headers.insert(
            HeaderName::from_bytes(name.as_bytes()).map_err(|error| error.to_string())?,
            HeaderValue::from_str(headerValue).map_err(|error| error.to_string())?,
        );
    }
    Ok(headers)
}

#[allow(non_snake_case)]
fn apiKey(provider: &ProviderProfile) -> Result<String, String> {
    if provider.useMultipleApiKeys {
        let apiKeys: Vec<&str> = provider
            .apiKeyPool
            .iter()
            .filter(|info| info.isEnabled && !info.key.trim().is_empty())
            .map(|info| info.key.trim())
            .collect();
        if apiKeys.is_empty() {
            return Err("provider api key is required".to_string());
        }
        let index = provider.currentKeyIndex.rem_euclid(apiKeys.len() as i32) as usize;
        return Ok(apiKeys[index].to_string());
    }
    let apiKey = provider.apiKey.trim();
    if apiKey.is_empty() {
        return Err("provider api key is required".to_string());
    }
    Ok(apiKey.to_string())
}

#[allow(non_snake_case)]
fn readPricing(
    item: &Value,
    operation: &ProviderOperationSpec,
) -> Result<Option<ModelPricing>, String> {
    let input = readOptionalF64(item, operation.result.inputPricePerTokenJsonPath.as_deref())?;
    let output = readOptionalF64(
        item,
        operation.result.outputPricePerTokenJsonPath.as_deref(),
    )?;
    let currency = readOptionalString(item, operation.result.currencyJsonPath.as_deref())?;
    match (input, output, currency) {
        (Some(input), Some(output), Some(currency)) => Ok(Some(ModelPricing {
            billingMode: BillingMode::TOKEN,
            inputPricePerMillion: input * 1_000_000.0,
            cachedInputPricePerMillion: readOptionalF64(
                item,
                operation.result.cachedInputPricePerTokenJsonPath.as_deref(),
            )?
            .map(|value| value * 1_000_000.0),
            outputPricePerMillion: output * 1_000_000.0,
            pricePerRequest: readOptionalF64(
                item,
                operation.result.pricePerRequestJsonPath.as_deref(),
            )?
            .unwrap_or(0.0),
            currency: parseCurrency(&currency)?,
        })),
        _ => Ok(None),
    }
}

#[allow(non_snake_case)]
fn readContext(
    item: &Value,
    operation: &ProviderOperationSpec,
) -> Result<Option<ModelContextSpec>, String> {
    let maxContextLength =
        readOptionalF32(item, operation.result.maxContextLengthJsonPath.as_deref())?;
    match maxContextLength {
        Some(maxContextLength) => Ok(Some(ModelContextSpec {
            maxContextLength: maxContextLength / 1000.0,
            enableMaxContextMode: false,
        })),
        None => Ok(None),
    }
}

#[allow(non_snake_case)]
fn readCapabilities(
    item: &Value,
    operation: &ProviderOperationSpec,
) -> Result<Option<ModelCapabilities>, String> {
    let values = [
        readOptionalBool(item, operation.result.directImageJsonPath.as_deref())?,
        readOptionalBool(item, operation.result.directAudioJsonPath.as_deref())?,
        readOptionalBool(item, operation.result.directVideoJsonPath.as_deref())?,
        readOptionalBool(item, operation.result.toolCallJsonPath.as_deref())?,
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

#[allow(non_snake_case)]
fn readRequest(
    item: &Value,
    operation: &ProviderOperationSpec,
) -> Result<ModelRequestSpec, String> {
    let supportsStructuredTools = readOptionalBool(
        item,
        operation.result.supportsStructuredToolsJsonPath.as_deref(),
    )?
    .unwrap_or(false);
    Ok(ModelRequestSpec {
        supportsStructuredTools,
    })
}

#[allow(non_snake_case)]
fn readRequiredString(item: &Value, spec: &str) -> Result<String, String> {
    readOptionalString(item, Some(spec))?.ok_or_else(|| format!("required value not found: {spec}"))
}

#[allow(non_snake_case)]
fn readOptionalString(item: &Value, spec: Option<&str>) -> Result<Option<String>, String> {
    let Some(spec) = spec else {
        return Ok(None);
    };
    if !spec.starts_with('$') {
        return Ok(Some(spec.to_string()));
    }
    Ok(selectJsonPath(item, spec).and_then(jsonValueString))
}

#[allow(non_snake_case)]
fn readOptionalF64(item: &Value, spec: Option<&str>) -> Result<Option<f64>, String> {
    let Some(value) = readOptionalString(item, spec)? else {
        return Ok(None);
    };
    value
        .parse::<f64>()
        .map(Some)
        .map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
fn readOptionalF32(item: &Value, spec: Option<&str>) -> Result<Option<f32>, String> {
    let Some(value) = readOptionalString(item, spec)? else {
        return Ok(None);
    };
    value
        .parse::<f32>()
        .map(Some)
        .map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
fn readOptionalBool(item: &Value, spec: Option<&str>) -> Result<Option<bool>, String> {
    let Some(spec) = spec else {
        return Ok(None);
    };
    if let Some((path, expected)) = spec.split_once('~') {
        let Some(value) = selectJsonPath(item, path) else {
            return Ok(Some(false));
        };
        return Ok(Some(jsonContains(value, expected)));
    }
    if !spec.starts_with('$') {
        return spec
            .parse::<bool>()
            .map(Some)
            .map_err(|error| error.to_string());
    }
    Ok(selectJsonPath(item, spec).and_then(Value::as_bool))
}

#[allow(non_snake_case)]
fn selectJsonPath<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    if path == "$" {
        return Some(value);
    }
    let mut current = value;
    let path = path.strip_prefix("$.")?;
    for segment in path.split('.') {
        current = selectSegment(current, segment)?;
    }
    Some(current)
}

#[allow(non_snake_case)]
fn selectSegment<'a>(value: &'a Value, segment: &str) -> Option<&'a Value> {
    let mut current = value;
    let mut rest = segment;
    let nameEnd = rest.find('[').unwrap_or(rest.len());
    let name = &rest[..nameEnd];
    if !name.is_empty() {
        current = current.get(name)?;
    }
    rest = &rest[nameEnd..];
    while !rest.is_empty() {
        let end = rest.find(']')?;
        let index = rest[1..end].parse::<usize>().ok()?;
        current = current.as_array()?.get(index)?;
        rest = &rest[end + 1..];
    }
    Some(current)
}

#[allow(non_snake_case)]
fn jsonValueString(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

#[allow(non_snake_case)]
fn jsonContains(value: &Value, expected: &str) -> bool {
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(jsonValueString)
            .any(|value| value == expected),
        _ => jsonValueString(value)
            .map(|value| value == expected)
            .unwrap_or(false),
    }
}

#[allow(non_snake_case)]
fn parseCurrency(value: &str) -> Result<PricingCurrency, String> {
    match value.trim().to_ascii_uppercase().as_str() {
        "CNY" => Ok(PricingCurrency::CNY),
        "USD" => Ok(PricingCurrency::USD),
        other => Err(format!("invalid pricing currency: {other}")),
    }
}
