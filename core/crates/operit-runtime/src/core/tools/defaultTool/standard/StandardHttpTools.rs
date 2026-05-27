use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::{
    HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE, COOKIE, SET_COOKIE, USER_AGENT,
};
use reqwest::{Method, Proxy, Url};
use serde_json::{json, Map, Value};

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::{AITool, ToolExecutor, ToolValidationResult};

#[derive(Clone, Debug)]
struct CookieRecord {
    name: String,
    value: String,
    domain: String,
    path: String,
    expiresAt: Option<i64>,
    secure: bool,
    httpOnly: bool,
}

#[derive(Clone)]
pub struct StandardHttpTools;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HttpToolOperation {
    HttpRequest,
    MultipartRequest,
    ManageCookies,
}

pub struct HttpToolExecutor {
    pub tools: StandardHttpTools,
    pub operation: HttpToolOperation,
}

static COOKIE_STORE: OnceLock<Mutex<HashMap<String, Vec<CookieRecord>>>> = OnceLock::new();

const USER_AGENT_VALUE: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";

impl StandardHttpTools {
    pub fn new() -> Self {
        Self
    }

    #[allow(non_snake_case)]
    pub fn httpRequest(&self, tool: &AITool) -> ToolResult {
        let toolClone = tool.clone();
        let toolName = tool.name.clone();
        runBlockingHttpThread(move || StandardHttpTools::new().httpRequestBlocking(&toolClone))
            .unwrap_or_else(|message| ToolResult {
                toolName,
                success: false,
                result: String::new(),
                error: Some(message),
            })
    }

    #[allow(non_snake_case)]
    fn httpRequestBlocking(&self, tool: &AITool) -> ToolResult {
        match self.prepareHttpRequest(tool) {
            Ok(spec) => match spec.request.send() {
                Ok(response) => {
                    let url = spec.url.clone();
                    let statusCode = response.status().as_u16() as i32;
                    let statusMessage = response
                        .status()
                        .canonical_reason()
                        .unwrap_or("")
                        .to_string();
                    let headers = responseHeadersMap(response.headers());
                    let contentType = response
                        .headers()
                        .get(CONTENT_TYPE)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or("")
                        .to_string();
                    saveResponseCookies(&url, response.headers());
                    match response.bytes() {
                        Ok(bodyBytes) => {
                            let content =
                                String::from_utf8(bodyBytes.to_vec()).unwrap_or_else(|_| {
                                    "[Binary Content, decoding failed]".to_string()
                                });
                            let responseText = buildHttpResponseData(
                                &url,
                                statusCode,
                                &statusMessage,
                                &headers,
                                &contentType,
                                &content,
                                Some(&STANDARD.encode(&bodyBytes)),
                                bodyBytes.len(),
                                &cookiesMapForUrl(&url),
                            );
                            success(tool, responseText)
                        }
                        Err(error) => errorResult(
                            tool.name.as_str(),
                            &format!("Error executing HTTP request: {error}"),
                        ),
                    }
                }
                Err(error) => errorResult(
                    tool.name.as_str(),
                    &format!("Error executing HTTP request: {error}"),
                ),
            },
            Err(error) => errorResult(
                tool.name.as_str(),
                &format!("Error executing HTTP request: {error}"),
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn manageCookies(&self, tool: &AITool) -> ToolResult {
        let action = parameterValue(tool, "action").to_lowercase();
        let action = if action.trim().is_empty() {
            "get".to_string()
        } else {
            action
        };
        let domain = parameterValue(tool, "domain");
        let cookiesJson =
            optionalParameterValue(tool, "cookies").unwrap_or_else(|| "{}".to_string());

        match action.as_str() {
            "get" => {
                let cookies = {
                    let store = cookieStore().lock().expect("cookie store mutex poisoned");
                    if domain.trim().is_empty() {
                        store.values().flatten().cloned().collect::<Vec<_>>()
                    } else {
                        store.get(domain.trim()).cloned().unwrap_or_default()
                    }
                };
                let mut object = Map::new();
                for cookie in cookies {
                    object.insert(
                        cookie.name,
                        json!({
                            "value": cookie.value,
                            "domain": cookie.domain,
                            "path": cookie.path,
                            "expires": cookie.expiresAt,
                            "secure": cookie.secure,
                            "httpOnly": cookie.httpOnly
                        }),
                    );
                }
                let jsonResult = serde_json::to_string_pretty(&Value::Object(object))
                    .unwrap_or_else(|_| "{}".to_string());
                success(tool, format!("Current cookie status:\n{jsonResult}"))
            }
            "set" => {
                if domain.trim().is_empty() {
                    return toolError(
                        tool,
                        String::new(),
                        "setCookie requires domain parameter".to_string(),
                    );
                }
                match parseCookies(&cookiesJson, &format!("https://{}", domain.trim())) {
                    Some(cookies) => {
                        let count = cookies.len();
                        cookieStore()
                            .lock()
                            .expect("cookie store mutex poisoned")
                            .insert(domain.trim().to_string(), cookies);
                        success(
                            tool,
                            format!(
                                "Successfully set {count} cookies to domain {}",
                                domain.trim()
                            ),
                        )
                    }
                    None => toolError(
                        tool,
                        String::new(),
                        "Cookie format error, cannot parse".to_string(),
                    ),
                }
            }
            "clear" => {
                if domain.trim().is_empty() {
                    cookieStore()
                        .lock()
                        .expect("cookie store mutex poisoned")
                        .clear();
                    success(tool, "Cleared all cookies".to_string())
                } else {
                    cookieStore()
                        .lock()
                        .expect("cookie store mutex poisoned")
                        .remove(domain.trim());
                    success(
                        tool,
                        format!("Cleared cookies for domain {}", domain.trim()),
                    )
                }
            }
            _ => toolError(
                tool,
                String::new(),
                format!("Unsupported action: {action}, supported actions are: get, set, clear"),
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn multipartRequest(&self, tool: &AITool) -> ToolResult {
        let toolClone = tool.clone();
        let toolName = tool.name.clone();
        runBlockingHttpThread(move || StandardHttpTools::new().multipartRequestBlocking(&toolClone))
            .unwrap_or_else(|message| ToolResult {
                toolName,
                success: false,
                result: String::new(),
                error: Some(message),
            })
    }

    #[allow(non_snake_case)]
    fn multipartRequestBlocking(&self, tool: &AITool) -> ToolResult {
        let url = parameterValue(tool, "url");
        let method = optionalParameterValue(tool, "method")
            .map(|value| value.to_uppercase())
            .unwrap_or_else(|| "POST".to_string());
        let headersParam =
            optionalParameterValue(tool, "headers").unwrap_or_else(|| "{}".to_string());
        let formDataParam =
            optionalParameterValue(tool, "form_data").unwrap_or_else(|| "{}".to_string());
        let filesParam = optionalParameterValue(tool, "files").unwrap_or_else(|| "[]".to_string());

        if url.trim().is_empty() {
            return toolError(
                tool,
                String::new(),
                "URL parameter cannot be empty".to_string(),
            );
        }
        if !isValidUrl(&url) {
            return toolError(tool, String::new(), format!("Invalid URL format: {url}"));
        }
        if method != "POST" && method != "PUT" {
            return toolError(
                tool,
                String::new(),
                format!("Multipart form requests only support POST and PUT methods, not supported: {method}"),
            );
        }

        let headers = match parseHeaders(&headersParam) {
            Ok(headers) => headers,
            Err(error) => return toolError(tool, String::new(), error),
        };
        let useCookies = optionalParameterValue(tool, "use_cookies")
            .map(|value| value.to_lowercase() != "false")
            .unwrap_or(true);
        if let Some(customCookies) =
            optionalParameterValue(tool, "custom_cookies").filter(|value| !value.trim().is_empty())
        {
            if useCookies {
                if let Some(cookies) = parseCookies(&customCookies, &url) {
                    if let Some(host) = Url::parse(&url)
                        .ok()
                        .and_then(|parsed| parsed.host_str().map(str::to_string))
                    {
                        cookieStore()
                            .lock()
                            .expect("cookie store mutex poisoned")
                            .insert(host, cookies);
                    }
                }
            }
        }

        let client = match buildConfigurableClient(tool, useCookies) {
            Ok(client) => client,
            Err(error) => return toolError(tool, String::new(), error),
        };

        let mut form = Form::new();
        let formData = match serde_json::from_str::<Value>(&formDataParam) {
            Ok(Value::Object(object)) => object,
            Ok(_) => {
                return toolError(
                    tool,
                    String::new(),
                    "Error parsing form data: form_data must be a JSON object".to_string(),
                )
            }
            Err(error) => {
                return toolError(
                    tool,
                    String::new(),
                    format!("Error parsing form data: {error}"),
                )
            }
        };
        for (key, value) in formData {
            form = form.text(key, jsonValueToString(&value));
        }

        let files = match serde_json::from_str::<Value>(&filesParam) {
            Ok(Value::Array(array)) => array,
            Ok(_) => {
                return toolError(
                    tool,
                    String::new(),
                    "Error parsing file data: files must be a JSON array".to_string(),
                )
            }
            Err(error) => {
                return toolError(
                    tool,
                    String::new(),
                    format!("Error parsing file data: {error}"),
                )
            }
        };
        for value in files {
            let Some(object) = value.as_object() else {
                return toolError(
                    tool,
                    String::new(),
                    "Error parsing file data: file entry must be a JSON object".to_string(),
                );
            };
            let fieldName = object
                .get("field_name")
                .and_then(Value::as_str)
                .unwrap_or("");
            let filePath = object
                .get("file_path")
                .and_then(Value::as_str)
                .unwrap_or("");
            let contentType = object
                .get("content_type")
                .and_then(Value::as_str)
                .unwrap_or("application/octet-stream");
            let fileName = object
                .get("file_name")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| {
                    Path::new(filePath)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("")
                        .to_string()
                });
            if fieldName.is_empty() || filePath.is_empty() {
                return toolError(
                    tool,
                    String::new(),
                    "Error parsing file data: field_name and file_path are required".to_string(),
                );
            }
            let bytes = match fs::read(filePath) {
                Ok(bytes) => bytes,
                Err(_) => {
                    return toolError(
                        tool,
                        String::new(),
                        format!("File does not exist or cannot be read: {filePath}"),
                    )
                }
            };
            let part = match Part::bytes(bytes).file_name(fileName).mime_str(contentType) {
                Ok(part) => part,
                Err(error) => {
                    return toolError(
                        tool,
                        String::new(),
                        format!("Error parsing file data: {error}"),
                    )
                }
            };
            form = form.part(fieldName.to_string(), part);
        }

        let methodValue = Method::from_bytes(method.as_bytes()).unwrap_or(Method::POST);
        let mut request = client.request(methodValue, url.trim()).multipart(form);
        request = request.header(USER_AGENT, USER_AGENT_VALUE);
        request = applyHeaders(request, &headers);
        if useCookies {
            request = applyCookieHeader(request, &url);
        }

        match request.send() {
            Ok(response) => {
                let statusCode = response.status().as_u16() as i32;
                let statusMessage = response
                    .status()
                    .canonical_reason()
                    .unwrap_or("")
                    .to_string();
                let responseHeaders = responseHeadersMap(response.headers());
                let contentType = response
                    .headers()
                    .get(CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                saveResponseCookies(&url, response.headers());
                match response.bytes() {
                    Ok(bodyBytes) => {
                        let content = String::from_utf8(bodyBytes.to_vec())
                            .unwrap_or_else(|_| "[Binary Content, decoding failed]".to_string());
                        success(
                            tool,
                            buildHttpResponseData(
                                &url,
                                statusCode,
                                &statusMessage,
                                &responseHeaders,
                                &contentType,
                                &content,
                                Some(&STANDARD.encode(&bodyBytes)),
                                bodyBytes.len(),
                                &cookiesMapForUrl(&url),
                            ),
                        )
                    }
                    Err(error) => toolError(
                        tool,
                        String::new(),
                        format!("Error executing multipart form request: {error}"),
                    ),
                }
            }
            Err(error) => toolError(
                tool,
                String::new(),
                format!("Error executing multipart form request: {error}"),
            ),
        }
    }

    #[allow(non_snake_case)]
    fn prepareHttpRequest(&self, tool: &AITool) -> Result<PreparedHttpRequest, String> {
        let url = parameterValue(tool, "url");
        let method = optionalParameterValue(tool, "method")
            .map(|value| value.to_uppercase())
            .unwrap_or_else(|| "GET".to_string());
        let headersParam =
            optionalParameterValue(tool, "headers").unwrap_or_else(|| "{}".to_string());
        let bodyParam = parameterValue(tool, "body");
        let bodyType = optionalParameterValue(tool, "body_type")
            .map(|value| value.to_lowercase())
            .unwrap_or_else(|| "json".to_string());
        let useCookies = optionalParameterValue(tool, "use_cookies")
            .map(|value| value.to_lowercase() != "false")
            .unwrap_or(true);

        if url.trim().is_empty() {
            return Err("URL parameter cannot be empty".to_string());
        }
        if !isValidUrl(&url) {
            return Err(format!("Invalid URL format: {url}"));
        }
        if !matches!(
            method.as_str(),
            "GET" | "POST" | "PUT" | "DELETE" | "HEAD" | "OPTIONS" | "PATCH" | "TRACE"
        ) {
            return Err(format!("Unsupported HTTP method: {method}"));
        }

        if let Some(customCookies) =
            optionalParameterValue(tool, "custom_cookies").filter(|value| !value.trim().is_empty())
        {
            if useCookies {
                if let Some(cookies) = parseCookies(&customCookies, &url) {
                    if let Some(host) = Url::parse(&url)
                        .ok()
                        .and_then(|parsed| parsed.host_str().map(str::to_string))
                    {
                        cookieStore()
                            .lock()
                            .expect("cookie store mutex poisoned")
                            .insert(host, cookies);
                    }
                }
            }
        }

        let client = buildConfigurableClient(tool, useCookies)?;
        let headers = parseHeaders(&headersParam)?;
        let methodValue =
            Method::from_bytes(method.as_bytes()).map_err(|error| error.to_string())?;
        let mut request = client.request(methodValue.clone(), url.trim());
        request = request.header(USER_AGENT, USER_AGENT_VALUE);
        request = applyHeaders(request, &headers);
        if useCookies {
            request = applyCookieHeader(request, &url);
        }

        if method != "GET" && method != "HEAD" && !bodyParam.trim().is_empty() {
            request = match bodyType.as_str() {
                "json" => request
                    .header(CONTENT_TYPE, "application/json; charset=utf-8")
                    .body(bodyParam),
                "form" => {
                    let formObject = serde_json::from_str::<Value>(&bodyParam)
                        .map_err(|error| error.to_string())?;
                    let Some(object) = formObject.as_object() else {
                        return Err("form body must be a JSON object".to_string());
                    };
                    let mut formPairs = Vec::new();
                    for (key, value) in object {
                        formPairs.push((key.clone(), jsonValueToString(value)));
                    }
                    request.form(&formPairs)
                }
                "text" => request
                    .header(CONTENT_TYPE, "text/plain; charset=utf-8")
                    .body(bodyParam),
                "xml" => request
                    .header(CONTENT_TYPE, "application/xml; charset=utf-8")
                    .body(bodyParam),
                "multipart" => {
                    return Err(
                        "multipart request body type requires dedicated multipart_request tool"
                            .to_string(),
                    )
                }
                _ => return Err(format!("Unsupported request body type: {bodyType}")),
            };
        }

        Ok(PreparedHttpRequest { url, request })
    }
}

struct PreparedHttpRequest {
    url: String,
    request: RequestBuilder,
}

impl ToolExecutor for HttpToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        let required = match self.operation {
            HttpToolOperation::HttpRequest => &["url", "method"][..],
            HttpToolOperation::MultipartRequest => &["url", "method"][..],
            HttpToolOperation::ManageCookies => &["action"][..],
        };
        for name in required {
            if parameterValue(tool, name).trim().is_empty() {
                return ToolValidationResult {
                    valid: false,
                    errorMessage: format!("{name} parameter is required"),
                };
            }
        }
        ToolValidationResult {
            valid: true,
            errorMessage: String::new(),
        }
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        let result = match self.operation {
            HttpToolOperation::HttpRequest => self.tools.httpRequest(tool),
            HttpToolOperation::MultipartRequest => self.tools.multipartRequest(tool),
            HttpToolOperation::ManageCookies => self.tools.manageCookies(tool),
        };
        vec![result]
    }
}

#[allow(non_snake_case)]
fn buildConfigurableClient(tool: &AITool, _useCookies: bool) -> Result<Client, String> {
    let connectTimeout = optionalParameterValue(tool, "connect_timeout")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(15);
    let readTimeout = optionalParameterValue(tool, "read_timeout")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(20);
    let followRedirects = optionalParameterValue(tool, "follow_redirects")
        .map(|value| value.to_lowercase() != "false")
        .unwrap_or(true);
    let ignoreSsl = optionalParameterValue(tool, "ignore_ssl")
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let mut builder = Client::builder()
        .connect_timeout(Duration::from_secs(connectTimeout))
        .timeout(Duration::from_secs(readTimeout))
        .danger_accept_invalid_certs(ignoreSsl);
    if !followRedirects {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }
    let proxyHost = parameterValue(tool, "proxy_host");
    let proxyPort = optionalParameterValue(tool, "proxy_port")
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(0);
    if !proxyHost.trim().is_empty() && proxyPort > 0 {
        let proxyUrl = format!("http://{}:{proxyPort}", proxyHost.trim());
        let proxy = Proxy::http(&proxyUrl).map_err(|error| error.to_string())?;
        builder = builder.proxy(proxy);
    }
    builder.build().map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
fn isValidUrl(urlString: &str) -> bool {
    Url::parse(urlString)
        .map(|url| url.scheme() == "http" || url.scheme() == "https")
        .unwrap_or(false)
}

#[allow(non_snake_case)]
fn parseHeaders(headersJson: &str) -> Result<HeaderMap, String> {
    if headersJson.trim().is_empty() {
        return Ok(HeaderMap::new());
    }
    let value = serde_json::from_str::<Value>(headersJson)
        .map_err(|error| format!("Invalid headers JSON: {error}"))?;
    let Some(object) = value.as_object() else {
        return Err("headers must be a JSON object string".to_string());
    };
    let mut headers = HeaderMap::new();
    for (key, value) in object {
        let headerName = HeaderName::from_bytes(key.as_bytes())
            .map_err(|error| format!("Invalid header name '{key}': {error}"))?;
        let headerValue = HeaderValue::from_str(&jsonValueToString(value))
            .map_err(|error| format!("Invalid header value for '{key}': {error}"))?;
        headers.insert(headerName, headerValue);
    }
    Ok(headers)
}

#[allow(non_snake_case)]
fn parseCookies(cookiesJson: &str, urlString: &str) -> Option<Vec<CookieRecord>> {
    let url = Url::parse(urlString).ok()?;
    let host = url.host_str()?.to_string();
    let value = serde_json::from_str::<Value>(cookiesJson).ok()?;
    let object = value.as_object()?;
    let mut cookies = Vec::new();
    for (name, value) in object {
        if let Some(text) = value.as_str() {
            cookies.push(CookieRecord {
                name: name.clone(),
                value: text.to_string(),
                domain: host.clone(),
                path: "/".to_string(),
                expiresAt: None,
                secure: false,
                httpOnly: false,
            });
        } else if let Some(cookieObject) = value.as_object() {
            cookies.push(CookieRecord {
                name: name.clone(),
                value: cookieObject
                    .get("value")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                domain: cookieObject
                    .get("domain")
                    .and_then(Value::as_str)
                    .filter(|domain| !domain.trim().is_empty())
                    .unwrap_or(&host)
                    .to_string(),
                path: cookieObject
                    .get("path")
                    .and_then(Value::as_str)
                    .filter(|path| !path.trim().is_empty())
                    .unwrap_or("/")
                    .to_string(),
                expiresAt: cookieObject.get("expiresAt").and_then(Value::as_i64),
                secure: cookieObject
                    .get("secure")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                httpOnly: cookieObject
                    .get("httpOnly")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            });
        }
    }
    Some(cookies)
}

#[allow(non_snake_case)]
fn saveResponseCookies(url: &str, headers: &HeaderMap) {
    let Ok(parsedUrl) = Url::parse(url) else {
        return;
    };
    let Some(host) = parsedUrl.host_str() else {
        return;
    };
    let mut cookies = Vec::new();
    for headerValue in headers.get_all(SET_COOKIE).iter() {
        let Ok(text) = headerValue.to_str() else {
            continue;
        };
        if let Some(cookie) = parseSetCookie(text, host) {
            cookies.push(cookie);
        }
    }
    if !cookies.is_empty() {
        cookieStore()
            .lock()
            .expect("cookie store mutex poisoned")
            .insert(host.to_string(), cookies);
    }
}

#[allow(non_snake_case)]
fn parseSetCookie(raw: &str, host: &str) -> Option<CookieRecord> {
    let mut parts = raw.split(';').map(str::trim);
    let first = parts.next()?;
    let (name, value) = first.split_once('=')?;
    let mut cookie = CookieRecord {
        name: name.to_string(),
        value: value.to_string(),
        domain: host.to_string(),
        path: "/".to_string(),
        expiresAt: None,
        secure: false,
        httpOnly: false,
    };
    for part in parts {
        if part.eq_ignore_ascii_case("secure") {
            cookie.secure = true;
        } else if part.eq_ignore_ascii_case("httponly") {
            cookie.httpOnly = true;
        } else if let Some((key, value)) = part.split_once('=') {
            match key.trim().to_lowercase().as_str() {
                "domain" => cookie.domain = value.trim().trim_start_matches('.').to_string(),
                "path" => cookie.path = value.trim().to_string(),
                "max-age" => cookie.expiresAt = value.trim().parse::<i64>().ok(),
                _ => {}
            }
        }
    }
    Some(cookie)
}

#[allow(non_snake_case)]
fn applyHeaders(mut request: RequestBuilder, headers: &HeaderMap) -> RequestBuilder {
    for (name, value) in headers.iter() {
        request = request.header(name, value);
    }
    request
}

#[allow(non_snake_case)]
fn applyCookieHeader(request: RequestBuilder, url: &str) -> RequestBuilder {
    let cookieHeader = cookiesForUrl(url)
        .into_iter()
        .map(|cookie| format!("{}={}", cookie.name, cookie.value))
        .collect::<Vec<_>>()
        .join("; ");
    if cookieHeader.is_empty() {
        request
    } else {
        request.header(COOKIE, cookieHeader)
    }
}

#[allow(non_snake_case)]
fn cookiesForUrl(url: &str) -> Vec<CookieRecord> {
    let Some(host) = Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_string))
    else {
        return Vec::new();
    };
    cookieStore()
        .lock()
        .expect("cookie store mutex poisoned")
        .get(&host)
        .cloned()
        .unwrap_or_default()
}

#[allow(non_snake_case)]
fn cookiesMapForUrl(url: &str) -> HashMap<String, String> {
    cookiesForUrl(url)
        .into_iter()
        .map(|cookie| (cookie.name, cookie.value))
        .collect()
}

#[allow(non_snake_case)]
fn responseHeadersMap(headers: &HeaderMap) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for (name, value) in headers.iter() {
        result.insert(name.to_string(), value.to_str().unwrap_or("").to_string());
    }
    result
}

#[allow(non_snake_case)]
fn buildHttpResponseData(
    url: &str,
    statusCode: i32,
    statusMessage: &str,
    _headers: &HashMap<String, String>,
    contentType: &str,
    content: &str,
    _contentBase64: Option<&str>,
    size: usize,
    cookies: &HashMap<String, String>,
) -> String {
    let mut text = String::new();
    text.push_str("HTTP Response:\n");
    text.push_str(&format!("URL: {url}\n"));
    text.push_str(&format!("Status: {statusCode} {statusMessage}\n"));
    text.push_str(&format!("Content-Type: {contentType}\n"));
    text.push_str(&format!("Size: {size} bytes\n"));
    if !cookies.is_empty() {
        text.push_str(&format!("Cookies: {}\n", cookies.len()));
        let mut entries = cookies.iter().collect::<Vec<_>>();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        for (name, value) in entries.into_iter().take(5) {
            let suffix = if value.chars().count() > 30 {
                "..."
            } else {
                ""
            };
            let preview = value.chars().take(30).collect::<String>();
            text.push_str(&format!("  {name}: {preview}{suffix}\n"));
        }
        if cookies.len() > 5 {
            text.push_str(&format!("  ... and {} more cookies\n", cookies.len() - 5));
        }
    }
    text.push('\n');
    text.push_str("Content Summary:\n");
    text.push_str(content);
    text
}

#[allow(non_snake_case)]
fn cookieStore() -> &'static Mutex<HashMap<String, Vec<CookieRecord>>> {
    COOKIE_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[allow(non_snake_case)]
fn jsonValueToString(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

fn success(tool: &AITool, result: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result,
        error: None,
    }
}

#[allow(non_snake_case)]
fn errorResult(toolName: &str, message: &str) -> ToolResult {
    ToolResult {
        toolName: toolName.to_string(),
        success: false,
        result: String::new(),
        error: Some(message.to_string()),
    }
}

#[allow(non_snake_case)]
fn toolError(tool: &AITool, result: String, message: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result,
        error: Some(message),
    }
}

#[allow(non_snake_case)]
fn parameterValue(tool: &AITool, name: &str) -> String {
    optionalParameterValue(tool, name).unwrap_or_default()
}

#[allow(non_snake_case)]
fn optionalParameterValue(tool: &AITool, name: &str) -> Option<String> {
    tool.parameters
        .iter()
        .find(|parameter| parameter.name == name)
        .map(|parameter| parameter.value.clone())
}

#[allow(non_snake_case)]
fn runBlockingHttpThread<T, F>(operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    thread::spawn(operation)
        .join()
        .map_err(|_| "HTTP worker thread panicked".to_string())
}
