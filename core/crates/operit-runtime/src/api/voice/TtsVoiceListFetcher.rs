#![allow(non_snake_case)]

use std::collections::BTreeMap;

use operit_host_api::HttpRequestData;
use serde_json::Value;

use operit_context::OperitApplicationContext::defaultHttpHost;
use crate::data::model::TtsCatalog::TtsCatalog;
use crate::data::model::TtsConfig::{
    AvailableTtsVoice, TtsConfig, TtsHttpHeader, TtsProviderCatalogEntry, TtsProviderOperationSpec,
};

pub struct TtsVoiceListFetcher;

impl TtsVoiceListFetcher {
    pub fn fetch(providerConfig: &TtsConfig) -> Result<Vec<AvailableTtsVoice>, String> {
        let providerCatalog = TtsCatalog::provider(&providerConfig.providerType)?;
        let operation = match providerCatalog
            .operations
            .iter()
            .find(|operation| operation.operationType == "list_voices")
        {
            Some(operation) => operation,
            None => return Ok(Vec::new()),
        };
        let response = requestJson(providerConfig, &providerCatalog, operation)?;
        parseVoiceItems(providerConfig, operation, &response)
    }
}

#[allow(non_snake_case)]
fn requestJson(
    providerConfig: &TtsConfig,
    providerCatalog: &TtsProviderCatalogEntry,
    operation: &TtsProviderOperationSpec,
) -> Result<Value, String> {
    if operation.handlerId != "http_json" {
        return Err(format!(
            "unsupported tts provider operation handler: {}",
            operation.handlerId
        ));
    }
    let method = normalizeMethod(&operation.method)?;
    let placeholders = operationPlaceholders(providerConfig);
    let url = operationUrl(&providerConfig.endpoint, &operation.path)?;
    let url = replaceOperationPlaceholders(&url, &placeholders)?;
    let mut headers = requestHeaders(providerConfig, &placeholders)?;
    let contentType = providerConfig.contentType.trim();
    if !contentType.is_empty() {
        pushHeaderIfAbsent(&mut headers, "Content-Type", contentType.to_string());
    }
    if operation.requiresApiKey {
        let headerName = operation.authHeaderName.trim();
        if headerName.is_empty() {
            return Err("tts provider operation auth header name is empty".to_string());
        }
        let headerValue = replaceOperationPlaceholders(&operation.authHeaderValue, &placeholders)?;
        pushHeaderIfAbsent(&mut headers, headerName, headerValue);
    }
    let body = if method == "POST" {
        replaceOperationPlaceholders(&operation.body, &placeholders)?.into_bytes()
    } else {
        Vec::new()
    };
    executeJsonRequest(
        method,
        url,
        headers,
        body,
        &format!("TTS provider operation {}", operation.operationType),
    )
}

#[allow(non_snake_case)]
fn requestHeaders(
    providerConfig: &TtsConfig,
    placeholders: &BTreeMap<String, String>,
) -> Result<Vec<(String, String)>, String> {
    providerConfig
        .headers
        .iter()
        .map(|header| operationHeader(header, placeholders))
        .collect()
}

#[allow(non_snake_case)]
fn operationHeader(
    header: &TtsHttpHeader,
    placeholders: &BTreeMap<String, String>,
) -> Result<(String, String), String> {
    let name = header.name.trim();
    if name.is_empty() {
        return Err("tts provider operation header name is empty".to_string());
    }
    Ok((
        name.to_string(),
        replaceOperationPlaceholders(&header.value, placeholders)?,
    ))
}

#[allow(non_snake_case)]
fn pushHeaderIfAbsent(headers: &mut Vec<(String, String)>, name: &str, value: String) {
    if headers
        .iter()
        .any(|(currentName, _)| currentName.eq_ignore_ascii_case(name))
    {
        return;
    }
    headers.push((name.to_string(), value));
}

#[allow(non_snake_case)]
fn operationPlaceholders(providerConfig: &TtsConfig) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    values.insert(
        "apiKey".to_string(),
        providerConfig.apiKey.trim().to_string(),
    );
    values.insert("model".to_string(), providerConfig.model.trim().to_string());
    values.insert(
        "responseFormat".to_string(),
        providerConfig.responseFormat.trim().to_string(),
    );
    values.insert("voice".to_string(), providerConfig.voice.trim().to_string());
    values
}

#[allow(non_snake_case)]
fn parseVoiceItems(
    providerConfig: &TtsConfig,
    operation: &TtsProviderOperationSpec,
    response: &Value,
) -> Result<Vec<AvailableTtsVoice>, String> {
    let itemsJsonPath = operation
        .result
        .itemsJsonPath
        .as_deref()
        .ok_or_else(|| "tts list_voices operation missing itemsJsonPath".to_string())?;
    let mut voices = Vec::new();
    for item in selectJsonPathArray(response, itemsJsonPath)? {
        voices.push(parseVoiceItem(providerConfig, operation, item)?);
    }
    Ok(voices)
}

#[allow(non_snake_case)]
fn parseVoiceItem(
    providerConfig: &TtsConfig,
    operation: &TtsProviderOperationSpec,
    item: &Value,
) -> Result<AvailableTtsVoice, String> {
    let model = match operation.result.modelJsonPath.as_deref() {
        Some(path) => readRequiredString(item, path)?,
        None => providerConfig.model.trim().to_string(),
    };
    let voice = readRequiredString(
        item,
        operation
            .result
            .voiceJsonPath
            .as_deref()
            .ok_or_else(|| "tts list_voices operation missing voiceJsonPath".to_string())?,
    )?;
    let displayName = readOptionalString(item, operation.result.displayNameJsonPath.as_deref())?
        .unwrap_or_else(String::new);
    let description = readOptionalString(item, operation.result.descriptionJsonPath.as_deref())?
        .unwrap_or_else(String::new);
    Ok(AvailableTtsVoice {
        model,
        voice,
        displayName,
        description,
        responseFormat: providerConfig.responseFormat.trim().to_string(),
        speed: providerConfig.speed,
    })
}

#[allow(non_snake_case)]
fn executeJsonRequest(
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    label: &str,
) -> Result<Value, String> {
    let response = defaultHttpHost()
        .executeHttpRequest(HttpRequestData {
            url,
            method,
            headers,
            body,
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
    if response.statusCode < 200 || response.statusCode >= 300 {
        let body = String::from_utf8_lossy(&response.body);
        return Err(format!(
            "{label} failed with status {}: {body}",
            response.statusCode
        ));
    }
    serde_json::from_slice(&response.body).map_err(|error| format!("{label} json error: {error}"))
}

#[allow(non_snake_case)]
fn operationUrl(endpoint: &str, path: &str) -> Result<String, String> {
    if path.starts_with("http://") || path.starts_with("https://") {
        return Ok(path.to_string());
    }
    let mut url = url::Url::parse(endpoint).map_err(|error| error.to_string())?;
    let (pathPart, queryPart) = path
        .split_once('?')
        .map(|(pathPart, queryPart)| (pathPart, Some(queryPart)))
        .unwrap_or((path, None));
    url.set_path(pathPart);
    url.set_query(queryPart);
    url.set_fragment(None);
    Ok(url.to_string())
}

#[allow(non_snake_case)]
fn replaceOperationPlaceholders(
    template: &str,
    placeholders: &BTreeMap<String, String>,
) -> Result<String, String> {
    let mut output = String::new();
    let mut index = 0usize;
    while index < template.len() {
        let remaining = &template[index..];
        let matched = placeholders.iter().find_map(|(name, value)| {
            let token = format!("{{{name}}}");
            remaining
                .starts_with(&token)
                .then_some((token.len(), value))
        });
        if let Some((tokenLen, value)) = matched {
            output.push_str(value);
            index += tokenLen;
        } else {
            let ch = remaining.chars().next().ok_or_else(|| {
                "tts operation placeholder scan reached invalid boundary".to_string()
            })?;
            output.push(ch);
            index += ch.len_utf8();
        }
    }
    Ok(output)
}

#[allow(non_snake_case)]
fn normalizeMethod(method: &str) -> Result<String, String> {
    let trimmed = method.trim();
    if trimmed.eq_ignore_ascii_case("GET") {
        Ok("GET".to_string())
    } else if trimmed.eq_ignore_ascii_case("POST") {
        Ok("POST".to_string())
    } else {
        Err(format!(
            "unsupported tts provider operation method: {trimmed}"
        ))
    }
}

#[allow(non_snake_case)]
fn selectJsonPathArray<'a>(value: &'a Value, paths: &str) -> Result<Vec<&'a Value>, String> {
    let mut items = Vec::new();
    for path in paths.split('+') {
        let path = path.trim();
        if path.is_empty() {
            return Err("tts json path is empty".to_string());
        }
        let selected = selectJsonPath(value, path)
            .ok_or_else(|| format!("tts json path not found: {path}"))?;
        let array = selected
            .as_array()
            .ok_or_else(|| format!("tts json path is not an array: {path}"))?;
        for item in array {
            items.push(item);
        }
    }
    Ok(items)
}

#[allow(non_snake_case)]
fn readRequiredString(item: &Value, spec: &str) -> Result<String, String> {
    readOptionalString(item, Some(spec))?
        .ok_or_else(|| format!("required tts value not found: {spec}"))
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
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}
