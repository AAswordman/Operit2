#![allow(non_snake_case)]

use std::collections::BTreeMap;

use base64::Engine;
use operit_host_api::HttpRequestData;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::api::voice::HttpTtsResponsePipelineStep::HttpTtsResponsePipelineStepType;
use crate::api::voice::VoiceService::{normalizedAudioExtension, VoiceService};
use crate::core::application::OperitApplicationContext::defaultHttpHost;
use crate::data::model::TtsConfig::{TtsConfig, TtsHttpHeader, TtsHttpResponsePipelineStep};

pub struct HttpVoiceProvider;

impl HttpVoiceProvider {
    pub fn new() -> Self {
        Self
    }
}

impl VoiceService for HttpVoiceProvider {
    fn synthesize(&self, config: &TtsConfig, text: &str) -> Result<Vec<u8>, String> {
        let placeholders = placeholders(config, text);
        let initialPayload = executeConfiguredRequest(config, &placeholders)?;
        let payload = resolvePipelineAudio(initialPayload, &config.responsePipeline)?;
        Ok(payload.bytes)
    }

    fn outputExtension(&self, config: &TtsConfig) -> Result<&'static str, String> {
        normalizedAudioExtension(&config.responseFormat)
    }
}

#[derive(Clone)]
struct BinaryPayload {
    bytes: Vec<u8>,
    contentType: Option<String>,
}

enum PipelineValue {
    Binary(BinaryPayload),
    Json(Value),
}

enum JsonPathToken {
    Key(String),
    Index(usize),
}

enum PlaceholderValue {
    Text(String),
    Number(f64),
}

impl PlaceholderValue {
    fn asString(&self) -> String {
        match self {
            PlaceholderValue::Text(value) => value.clone(),
            PlaceholderValue::Number(value) => numberString(*value),
        }
    }

    fn asJsonLiteral(&self) -> Result<String, String> {
        match self {
            PlaceholderValue::Text(value) => {
                serde_json::to_string(value).map_err(|error| error.to_string())
            }
            PlaceholderValue::Number(value) => Ok(numberString(*value)),
        }
    }

    fn asJsonStringContent(&self) -> Result<String, String> {
        serde_json::to_string(&self.asString())
            .map(|encoded| encoded[1..encoded.len() - 1].to_string())
            .map_err(|error| error.to_string())
    }

    fn asXmlContent(&self) -> String {
        xmlEscape(&self.asString())
    }
}

fn placeholders(config: &TtsConfig, text: &str) -> BTreeMap<String, PlaceholderValue> {
    let mut values = BTreeMap::new();
    values.insert(
        "apiKey".to_string(),
        PlaceholderValue::Text(config.apiKey.clone()),
    );
    values.insert(
        "locale".to_string(),
        PlaceholderValue::Text(config.model.clone()),
    );
    values.insert(
        "model".to_string(),
        PlaceholderValue::Text(config.model.clone()),
    );
    values.insert("pitch".to_string(), PlaceholderValue::Number(1.0));
    values.insert("rate".to_string(), PlaceholderValue::Number(config.speed));
    values.insert(
        "responseFormat".to_string(),
        PlaceholderValue::Text(config.responseFormat.clone()),
    );
    values.insert("speed".to_string(), PlaceholderValue::Number(config.speed));
    values.insert("text".to_string(), PlaceholderValue::Text(text.to_string()));
    values.insert(
        "textXml".to_string(),
        PlaceholderValue::Text(text.to_string()),
    );
    values.insert(
        "uuid".to_string(),
        PlaceholderValue::Text(Uuid::new_v4().to_string()),
    );
    values.insert(
        "voice".to_string(),
        PlaceholderValue::Text(config.voice.clone()),
    );
    values
}

fn executeConfiguredRequest(
    config: &TtsConfig,
    placeholders: &BTreeMap<String, PlaceholderValue>,
) -> Result<BinaryPayload, String> {
    let method = normalizeMethod(&config.httpMethod)?;
    let url = replacePlaceholders(&config.endpoint, placeholders, PlaceholderMode::Url)?;
    let mut headers = replaceHeaders(&config.headers, placeholders)?;
    let bodyPlaceholderMode = placeholderModeFromContentType(&config.contentType);
    if method == "POST" {
        let contentType = config.contentType.trim();
        if !contentType.is_empty() {
            headers.push(("Content-Type".to_string(), contentType.to_string()));
        }
    }
    let body = if method == "POST" {
        replacePlaceholders(&config.requestBody, placeholders, bodyPlaceholderMode)?.into_bytes()
    } else {
        Vec::new()
    };
    executeBinaryRequest(method, url, headers, body, "HTTP TTS request")
}

fn replaceHeaders(
    headers: &[TtsHttpHeader],
    placeholders: &BTreeMap<String, PlaceholderValue>,
) -> Result<Vec<(String, String)>, String> {
    let mut result = Vec::new();
    for header in headers {
        let name = header.name.trim();
        if name.is_empty() {
            return Err("http tts header name is empty".to_string());
        }
        result.push((
            name.to_string(),
            replacePlaceholders(&header.value, placeholders, PlaceholderMode::Plain)?,
        ));
    }
    Ok(result)
}

#[derive(Clone, Copy)]
enum PlaceholderMode {
    Plain,
    Url,
    JsonBody,
    XmlBody,
}

fn replacePlaceholders(
    template: &str,
    placeholders: &BTreeMap<String, PlaceholderValue>,
    mode: PlaceholderMode,
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
            let replacement = match mode {
                PlaceholderMode::Plain => value.asString(),
                PlaceholderMode::Url => percentEncode(&value.asString()),
                PlaceholderMode::JsonBody => {
                    if isInsideJsonString(template, index) {
                        value.asJsonStringContent()?
                    } else {
                        value.asJsonLiteral()?
                    }
                }
                PlaceholderMode::XmlBody => value.asXmlContent(),
            };
            output.push_str(&replacement);
            index += tokenLen;
        } else {
            let ch = remaining
                .chars()
                .next()
                .ok_or_else(|| "http tts placeholder scan reached invalid boundary".to_string())?;
            output.push(ch);
            index += ch.len_utf8();
        }
    }
    Ok(output)
}

fn isInsideJsonString(template: &str, targetIndex: usize) -> bool {
    let mut inside = false;
    let mut escaped = false;
    for (index, ch) in template.char_indices() {
        if index >= targetIndex {
            break;
        }
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            inside = !inside;
        }
    }
    inside
}

fn percentEncode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn placeholderModeFromContentType(contentType: &str) -> PlaceholderMode {
    let normalized = contentType
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if normalized == "application/json" || normalized.ends_with("+json") {
        PlaceholderMode::JsonBody
    } else if normalized == "application/xml"
        || normalized == "text/xml"
        || normalized == "application/ssml+xml"
        || normalized.ends_with("+xml")
    {
        PlaceholderMode::XmlBody
    } else {
        PlaceholderMode::Plain
    }
}

fn resolvePipelineAudio(
    initialPayload: BinaryPayload,
    steps: &[TtsHttpResponsePipelineStep],
) -> Result<BinaryPayload, String> {
    let mut current = PipelineValue::Binary(initialPayload);
    for (index, step) in steps.iter().enumerate() {
        let stepType = HttpTtsResponsePipelineStepType::normalize(&step.stepType)?;
        current = match stepType.as_str() {
            HttpTtsResponsePipelineStepType::PARSE_JSON => {
                PipelineValue::Json(parseJsonPayload(current, index)?)
            }
            HttpTtsResponsePipelineStepType::PICK => {
                PipelineValue::Json(pickJsonValue(current, &step.path, index)?)
            }
            HttpTtsResponsePipelineStepType::PARSE_JSON_STRING => {
                PipelineValue::Json(parseJsonStringValue(current, index)?)
            }
            HttpTtsResponsePipelineStepType::HTTP_GET => {
                PipelineValue::Binary(followHttpUrl(current, step, index)?)
            }
            HttpTtsResponsePipelineStepType::HTTP_REQUEST_FROM_OBJECT => {
                PipelineValue::Binary(followHttpRequestObject(current, index)?)
            }
            HttpTtsResponsePipelineStepType::BASE64_DECODE => {
                PipelineValue::Binary(decodeBase64Value(current, index)?)
            }
            HttpTtsResponsePipelineStepType::HEX_DECODE => {
                PipelineValue::Binary(decodeHexValue(current, index)?)
            }
            _ => {
                return Err(format!(
                    "unsupported http tts response pipeline step: {stepType}"
                ))
            }
        };
    }
    match current {
        PipelineValue::Binary(payload) => Ok(payload),
        PipelineValue::Json(_) => {
            Err("http tts response pipeline result is not binary audio".to_string())
        }
    }
}

fn parseJsonPayload(current: PipelineValue, stepIndex: usize) -> Result<Value, String> {
    let text = scalarTextFromPipeline(current, stepIndex, "parse_json")?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "http tts response pipeline parse_json step {} failed: {error}",
            stepIndex + 1
        )
    })
}

fn pickJsonValue(current: PipelineValue, path: &str, stepIndex: usize) -> Result<Value, String> {
    let value = match current {
        PipelineValue::Json(value) => value,
        PipelineValue::Binary(_) => {
            return Err(format!(
                "http tts response pipeline pick step {} expects json",
                stepIndex + 1
            ))
        }
    };
    let tokens = parseJsonPath(path)?;
    let mut current = &value;
    for token in tokens {
        current = match token {
            JsonPathToken::Key(key) => current.get(&key).ok_or_else(|| {
                format!(
                    "http tts response pipeline pick step {} missing key: {key}",
                    stepIndex + 1
                )
            })?,
            JsonPathToken::Index(index) => current.get(index).ok_or_else(|| {
                format!(
                    "http tts response pipeline pick step {} missing index: {index}",
                    stepIndex + 1
                )
            })?,
        };
    }
    Ok(current.clone())
}

fn parseJsonStringValue(current: PipelineValue, stepIndex: usize) -> Result<Value, String> {
    let text = scalarTextFromPipeline(current, stepIndex, "parse_json_string")?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "http tts response pipeline parse_json_string step {} failed: {error}",
            stepIndex + 1
        )
    })
}

fn followHttpUrl(
    current: PipelineValue,
    step: &TtsHttpResponsePipelineStep,
    stepIndex: usize,
) -> Result<BinaryPayload, String> {
    let url = scalarTextFromPipeline(current, stepIndex, "http_get")?;
    let headers = step
        .headers
        .iter()
        .map(|header| {
            let name = header.name.trim();
            if name.is_empty() {
                Err("http tts pipeline header name is empty".to_string())
            } else {
                Ok((name.to_string(), header.value.trim().to_string()))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    executeBinaryRequest(
        "GET".to_string(),
        url,
        headers,
        Vec::new(),
        &format!("HTTP TTS pipeline http_get step {}", stepIndex + 1),
    )
}

fn followHttpRequestObject(
    current: PipelineValue,
    stepIndex: usize,
) -> Result<BinaryPayload, String> {
    let value = match current {
        PipelineValue::Json(Value::Object(object)) => object,
        PipelineValue::Json(_) => {
            return Err(format!(
                "http tts response pipeline http_request_from_object step {} expects object",
                stepIndex + 1
            ))
        }
        PipelineValue::Binary(_) => {
            return Err(format!(
                "http tts response pipeline http_request_from_object step {} expects json",
                stepIndex + 1
            ))
        }
    };
    let url = requiredObjectString(&value, "url", stepIndex)?;
    let method = normalizeMethod(&requiredObjectString(&value, "method", stepIndex)?)?;
    let headers = requestObjectHeaders(&value)?;
    let body = requestObjectBody(&value)?;
    executeBinaryRequest(
        method,
        url,
        headers,
        body,
        &format!(
            "HTTP TTS pipeline http_request_from_object step {}",
            stepIndex + 1
        ),
    )
}

fn requestObjectHeaders(object: &Map<String, Value>) -> Result<Vec<(String, String)>, String> {
    let mut headers = Vec::new();
    let Some(Value::Object(headersObject)) = object.get("headers") else {
        return Ok(headers);
    };
    for (name, value) in headersObject {
        let value = scalarString(value).ok_or_else(|| {
            format!("http tts pipeline request header value is not scalar: {name}")
        })?;
        headers.push((name.to_string(), value));
    }
    Ok(headers)
}

fn requestObjectBody(object: &Map<String, Value>) -> Result<Vec<u8>, String> {
    let Some(value) = object.get("body") else {
        return Ok(Vec::new());
    };
    match value {
        Value::String(text) => Ok(text.as_bytes().to_vec()),
        Value::Object(_) | Value::Array(_) => {
            serde_json::to_vec(value).map_err(|error| error.to_string())
        }
        Value::Bool(_) | Value::Number(_) => Ok(value.to_string().into_bytes()),
        Value::Null => Ok(Vec::new()),
    }
}

fn decodeBase64Value(current: PipelineValue, stepIndex: usize) -> Result<BinaryPayload, String> {
    let raw = scalarTextFromPipeline(current, stepIndex, "base64_decode")?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .map_err(|error| {
            format!(
                "http tts response pipeline base64_decode step {} failed: {error}",
                stepIndex + 1
            )
        })?;
    Ok(BinaryPayload {
        bytes,
        contentType: None,
    })
}

fn decodeHexValue(current: PipelineValue, stepIndex: usize) -> Result<BinaryPayload, String> {
    let raw = scalarTextFromPipeline(current, stepIndex, "hex_decode")?;
    let text = raw.trim();
    if text.len() % 2 != 0 {
        return Err(format!(
            "http tts response pipeline hex_decode step {} received odd length data",
            stepIndex + 1
        ));
    }
    let bytes = text
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| {
            let hex = std::str::from_utf8(chunk).map_err(|error| {
                format!(
                    "http tts response pipeline hex_decode step {} received invalid utf-8: {error}",
                    stepIndex + 1
                )
            })?;
            u8::from_str_radix(hex, 16).map_err(|error| {
                format!(
                    "http tts response pipeline hex_decode step {} failed: {error}",
                    stepIndex + 1
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BinaryPayload {
        bytes,
        contentType: None,
    })
}

fn scalarTextFromPipeline(
    current: PipelineValue,
    stepIndex: usize,
    stepType: &str,
) -> Result<String, String> {
    match current {
        PipelineValue::Binary(payload) => String::from_utf8(payload.bytes).map_err(|error| {
            format!(
                "http tts response pipeline {stepType} step {} expected utf-8 text: {error}",
                stepIndex + 1
            )
        }),
        PipelineValue::Json(value) => scalarString(&value).ok_or_else(|| {
            format!(
                "http tts response pipeline {stepType} step {} expects scalar value",
                stepIndex + 1
            )
        }),
    }
}

fn scalarString(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

fn requiredObjectString(
    object: &Map<String, Value>,
    key: &str,
    stepIndex: usize,
) -> Result<String, String> {
    let value = object.get(key).ok_or_else(|| {
        format!(
            "http tts response pipeline http_request_from_object step {} missing field: {key}",
            stepIndex + 1
        )
    })?;
    scalarString(value).ok_or_else(|| {
        format!(
            "http tts response pipeline http_request_from_object step {} field is not scalar: {key}",
            stepIndex + 1
        )
    })
}

fn executeBinaryRequest(
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    label: &str,
) -> Result<BinaryPayload, String> {
    validateHttpUrl(&url)?;
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
    Ok(BinaryPayload {
        contentType: responseHeader(&response.headers, "content-type"),
        bytes: response.body,
    })
}

fn responseHeader(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(headerName, _)| headerName.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.clone())
}

fn normalizeMethod(method: &str) -> Result<String, String> {
    let trimmed = method.trim();
    if trimmed.eq_ignore_ascii_case("GET") {
        Ok("GET".to_string())
    } else if trimmed.eq_ignore_ascii_case("POST") {
        Ok("POST".to_string())
    } else {
        Err(format!("unsupported http tts method: {trimmed}"))
    }
}

fn validateHttpUrl(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|error| format!("invalid http tts url: {error}"))?;
    match parsed.scheme() {
        "http" | "https" => Ok(()),
        scheme => Err(format!("unsupported http tts url scheme: {scheme}")),
    }
}

fn parseJsonPath(rawPath: &str) -> Result<Vec<JsonPathToken>, String> {
    let chars = rawPath.trim().chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Err("http tts json path is empty".to_string());
    }
    let mut tokens = Vec::new();
    let mut index = 0usize;
    if chars[index] == '$' {
        index += 1;
    }
    while index < chars.len() {
        if chars[index] == '.' {
            index += 1;
            if index == chars.len() {
                return Err("http tts json path ends with dot".to_string());
            }
        }
        if chars[index] == '[' {
            index += 1;
            let start = index;
            while index < chars.len() && chars[index].is_ascii_digit() {
                index += 1;
            }
            if start == index {
                return Err("http tts json path index is empty".to_string());
            }
            if index >= chars.len() || chars[index] != ']' {
                return Err("http tts json path index is not closed".to_string());
            }
            let number = chars[start..index].iter().collect::<String>();
            let parsed = number
                .parse::<usize>()
                .map_err(|error| format!("invalid http tts json path index: {error}"))?;
            tokens.push(JsonPathToken::Index(parsed));
            index += 1;
            continue;
        }
        let start = index;
        while index < chars.len() && chars[index] != '.' && chars[index] != '[' {
            index += 1;
        }
        if start == index {
            return Err("http tts json path token is empty".to_string());
        }
        tokens.push(JsonPathToken::Key(chars[start..index].iter().collect()));
    }
    if tokens.is_empty() {
        return Err("http tts json path has no token".to_string());
    }
    Ok(tokens)
}

fn numberString(value: f64) -> String {
    serde_json::Number::from_f64(value)
        .expect("tts numeric placeholder must be finite")
        .to_string()
}

fn xmlEscape(value: &str) -> String {
    let mut output = String::new();
    for ch in value.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            other => output.push(other),
        }
    }
    output
}
