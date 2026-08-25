use operit_host_api::HostManager::defaultHttpHost;
use operit_host_api::HttpRequestData;
use serde_json::Value;

use operit_model::BillingMode::BillingMode;
use operit_model::ModelConfigData::{
    AvailableProviderModel, AvailableProviderModelSource, ModelCapabilities, ModelContextSpec,
    ModelPricing, ModelRequestSpec, PricingCurrency, ProviderCatalogEntry, ProviderOperationSpec,
    ProviderProfile,
};

pub struct ModelListFetcher;

impl ModelListFetcher {
    /// Fetches provider models through the configured catalog operation.
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

    /// Executes the provider model list request through the configured HTTP host.
    #[allow(non_snake_case)]
    fn requestJson(
        provider: &ProviderProfile,
        operation: &ProviderOperationSpec,
    ) -> Result<Value, String> {
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

        let response = defaultHttpHost()
            .executeHttpRequest(HttpRequestData {
                url: operationUrl(&provider.endpoint, &operation.path)?,
                method: operation.method.clone(),
                headers: headers(provider, operation)?,
                body: Vec::new(),
                formFields: Vec::new(),
                fileParts: Vec::new(),
                connectTimeoutSeconds: 30,
                readTimeoutSeconds: 120,
                followRedirects: true,
                ignoreSsl: false,
                proxyHost: String::new(),
                proxyPort: 0,
            })
            .map_err(|error| error.to_string())?;
        let body = String::from_utf8(response.body)
            .map_err(|error| format!("list_models response body is not UTF-8: {error}"))?;
        if response.statusCode < 200 || response.statusCode >= 300 {
            return Err(format!(
                "list_models request failed: {} {body}",
                response.statusCode
            ));
        }
        serde_json::from_str(&body).map_err(|error| error.to_string())
    }

    /// Parses one provider model item into a runtime model option.
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

/// Builds the provider operation URL from the chat endpoint and operation path.
#[allow(non_snake_case)]
fn operationUrl(endpoint: &str, path: &str) -> Result<String, String> {
    let mut url = url::Url::parse(endpoint).map_err(|error| error.to_string())?;
    url.set_path(path);
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string())
}

/// Builds HTTP headers required by the provider operation.
#[allow(non_snake_case)]
fn headers(
    provider: &ProviderProfile,
    operation: &ProviderOperationSpec,
) -> Result<Vec<(String, String)>, String> {
    let mut headers = vec![("Content-Type".to_string(), "application/json".to_string())];
    if operation.requiresApiKey {
        headers.push((
            "Authorization".to_string(),
            format!("Bearer {}", apiKey(provider)?),
        ));
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
        headers.push((name.clone(), headerValue.to_string()));
    }
    Ok(headers)
}

/// Selects the API key used by the provider model list request.
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

/// Reads pricing metadata from one provider model item.
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

/// Reads context-window metadata from one provider model item.
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

/// Reads model capability metadata from one provider model item.
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

/// Reads request-shape metadata from one provider model item.
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

/// Reads a required string from one JSON path or literal spec.
#[allow(non_snake_case)]
fn readRequiredString(item: &Value, spec: &str) -> Result<String, String> {
    readOptionalString(item, Some(spec))?.ok_or_else(|| format!("required value not found: {spec}"))
}

/// Reads an optional string from one JSON path or literal spec.
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

/// Reads an optional f64 value from one JSON path or literal spec.
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

/// Reads an optional f32 value from one JSON path or literal spec.
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

/// Reads an optional boolean from one JSON path, literal spec, or containment spec.
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

/// Selects a JSON value using the catalog's dot-and-index path syntax.
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

/// Selects one named or indexed segment from a JSON value.
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

/// Converts scalar JSON values into strings used by catalog readers.
#[allow(non_snake_case)]
fn jsonValueString(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

/// Tests whether a JSON scalar or array contains the expected catalog value.
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

/// Parses a provider pricing currency literal.
#[allow(non_snake_case)]
fn parseCurrency(value: &str) -> Result<PricingCurrency, String> {
    match value.trim().to_ascii_uppercase().as_str() {
        "CNY" => Ok(PricingCurrency::CNY),
        "USD" => Ok(PricingCurrency::USD),
        other => Err(format!("invalid pricing currency: {other}")),
    }
}
