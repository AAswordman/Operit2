use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use operit_host_api::{
    FileSystemHost, HttpFilePart, HttpHost, HttpRequestData,
    HttpResponseData as HostHttpResponseData,
};
use serde_json::{json, Map, Value};
use url::Url;

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::{AITool, ToolExecutor, ToolValidationResult};
use crate::core::tools::ToolResultDataClasses::{
    stringResultData, HttpResponseData, ToolResultData,
};

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
pub struct StandardHttpTools {
    httpHost: Arc<dyn HttpHost>,
    fileSystemHost: Option<Arc<dyn FileSystemHost>>,
}

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
    pub fn new(
        httpHost: Arc<dyn HttpHost>,
        fileSystemHost: Option<Arc<dyn FileSystemHost>>,
    ) -> Self {
        Self {
            httpHost,
            fileSystemHost,
        }
    }

    #[allow(non_snake_case)]
    pub fn httpRequest(&self, tool: &AITool) -> ToolResult {
        match self.prepareHttpRequest(tool) {
            Ok(request) => self.executeRequest(tool, request, "Error executing HTTP request"),
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
        match self.prepareMultipartRequest(tool) {
            Ok(request) => {
                self.executeRequest(tool, request, "Error executing multipart form request")
            }
            Err(error) => toolError(tool, String::new(), error),
        }
    }

    #[allow(non_snake_case)]
    fn executeRequest(
        &self,
        tool: &AITool,
        request: HttpRequestData,
        messagePrefix: &str,
    ) -> ToolResult {
        let url = request.url.clone();
        match self.httpHost.executeHttpRequest(request) {
            Ok(response) => successData(tool, responseToData(&url, response)),
            Err(error) => toolError(tool, String::new(), format!("{messagePrefix}: {error}")),
        }
    }

    #[allow(non_snake_case)]
    fn prepareHttpRequest(&self, tool: &AITool) -> Result<HttpRequestData, String> {
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

        validateRequestBase(&url, &method)?;
        applyCustomCookies(tool, useCookies, &url);

        let mut headers = parseHeaders(&headersParam)?;
        headers.push(("User-Agent".to_string(), USER_AGENT_VALUE.to_string()));
        if useCookies {
            pushCookieHeader(&mut headers, &url);
        }

        let mut body = Vec::new();
        if method != "GET" && method != "HEAD" && !bodyParam.trim().is_empty() {
            match bodyType.as_str() {
                "json" => {
                    headers.push((
                        "Content-Type".to_string(),
                        "application/json; charset=utf-8".to_string(),
                    ));
                    body = bodyParam.into_bytes();
                }
                "form" => {
                    headers.push((
                        "Content-Type".to_string(),
                        "application/x-www-form-urlencoded; charset=utf-8".to_string(),
                    ));
                    let formObject = serde_json::from_str::<Value>(&bodyParam)
                        .map_err(|error| error.to_string())?;
                    let Some(object) = formObject.as_object() else {
                        return Err("form body must be a JSON object".to_string());
                    };
                    body = formUrlEncoded(object).into_bytes();
                }
                "text" => {
                    headers.push((
                        "Content-Type".to_string(),
                        "text/plain; charset=utf-8".to_string(),
                    ));
                    body = bodyParam.into_bytes();
                }
                "xml" => {
                    headers.push((
                        "Content-Type".to_string(),
                        "application/xml; charset=utf-8".to_string(),
                    ));
                    body = bodyParam.into_bytes();
                }
                "multipart" => {
                    return Err(
                        "multipart request body type requires dedicated multipart_request tool"
                            .to_string(),
                    )
                }
                _ => return Err(format!("Unsupported request body type: {bodyType}")),
            }
        }

        Ok(buildHostRequest(
            tool,
            url,
            method,
            headers,
            body,
            Vec::new(),
            Vec::new(),
        ))
    }

    #[allow(non_snake_case)]
    fn prepareMultipartRequest(&self, tool: &AITool) -> Result<HttpRequestData, String> {
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
            return Err("URL parameter cannot be empty".to_string());
        }
        if !isValidUrl(&url) {
            return Err(format!("Invalid URL format: {url}"));
        }
        if method != "POST" && method != "PUT" {
            return Err(format!(
                "Multipart form requests only support POST and PUT methods, not supported: {method}"
            ));
        }

        let mut headers = parseHeaders(&headersParam)?;
        headers.push(("User-Agent".to_string(), USER_AGENT_VALUE.to_string()));
        let useCookies = optionalParameterValue(tool, "use_cookies")
            .map(|value| value.to_lowercase() != "false")
            .unwrap_or(true);
        applyCustomCookies(tool, useCookies, &url);
        if useCookies {
            pushCookieHeader(&mut headers, &url);
        }

        let formFields = parseFormFields(&formDataParam)?;
        let fileParts = self.parseFileParts(&filesParam)?;

        Ok(buildHostRequest(
            tool,
            url,
            method,
            headers,
            Vec::new(),
            formFields,
            fileParts,
        ))
    }

    #[allow(non_snake_case)]
    fn parseFileParts(&self, filesParam: &str) -> Result<Vec<HttpFilePart>, String> {
        let files = match serde_json::from_str::<Value>(filesParam) {
            Ok(Value::Array(array)) => array,
            Ok(_) => return Err("Error parsing file data: files must be a JSON array".to_string()),
            Err(error) => return Err(format!("Error parsing file data: {error}")),
        };
        let mut parts = Vec::new();
        for value in files {
            let Some(object) = value.as_object() else {
                return Err("Error parsing file data: file entry must be a JSON object".to_string());
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
                return Err(
                    "Error parsing file data: field_name and file_path are required".to_string(),
                );
            }
            let content = match &self.fileSystemHost {
                Some(host) => host.readFileBytes(filePath).map_err(|error| {
                    format!("File does not exist or cannot be read: {filePath}: {error}")
                })?,
                None => fs::read(filePath)
                    .map_err(|_| format!("File does not exist or cannot be read: {filePath}"))?,
            };
            parts.push(HttpFilePart {
                fieldName: fieldName.to_string(),
                fileName,
                contentType: contentType.to_string(),
                content,
            });
        }
        Ok(parts)
    }
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
fn validateRequestBase(url: &str, method: &str) -> Result<(), String> {
    if url.trim().is_empty() {
        return Err("URL parameter cannot be empty".to_string());
    }
    if !isValidUrl(url) {
        return Err(format!("Invalid URL format: {url}"));
    }
    if !matches!(
        method,
        "GET" | "POST" | "PUT" | "DELETE" | "HEAD" | "OPTIONS" | "PATCH" | "TRACE"
    ) {
        return Err(format!("Unsupported HTTP method: {method}"));
    }
    Ok(())
}

#[allow(non_snake_case)]
fn buildHostRequest(
    tool: &AITool,
    url: String,
    method: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    formFields: Vec<(String, String)>,
    fileParts: Vec<HttpFilePart>,
) -> HttpRequestData {
    HttpRequestData {
        url,
        method,
        headers,
        body,
        formFields,
        fileParts,
        connectTimeoutSeconds: optionalParameterValue(tool, "connect_timeout")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(15),
        readTimeoutSeconds: optionalParameterValue(tool, "read_timeout")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(20),
        followRedirects: optionalParameterValue(tool, "follow_redirects")
            .map(|value| value.to_lowercase() != "false")
            .unwrap_or(true),
        ignoreSsl: optionalParameterValue(tool, "ignore_ssl")
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
        proxyHost: parameterValue(tool, "proxy_host"),
        proxyPort: optionalParameterValue(tool, "proxy_port")
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(0),
    }
}

#[allow(non_snake_case)]
fn isValidUrl(urlString: &str) -> bool {
    Url::parse(urlString)
        .map(|url| url.scheme() == "http" || url.scheme() == "https")
        .unwrap_or(false)
}

#[allow(non_snake_case)]
fn parseHeaders(headersJson: &str) -> Result<Vec<(String, String)>, String> {
    if headersJson.trim().is_empty() {
        return Ok(Vec::new());
    }
    let value = serde_json::from_str::<Value>(headersJson)
        .map_err(|error| format!("Invalid headers JSON: {error}"))?;
    let Some(object) = value.as_object() else {
        return Err("headers must be a JSON object string".to_string());
    };
    let mut headers = Vec::new();
    for (key, value) in object {
        if key.trim().is_empty() {
            return Err("Invalid header name: empty".to_string());
        }
        headers.push((key.clone(), jsonValueToString(value)));
    }
    Ok(headers)
}

#[allow(non_snake_case)]
fn parseFormFields(formDataParam: &str) -> Result<Vec<(String, String)>, String> {
    let formData = match serde_json::from_str::<Value>(formDataParam) {
        Ok(Value::Object(object)) => object,
        Ok(_) => return Err("Error parsing form data: form_data must be a JSON object".to_string()),
        Err(error) => return Err(format!("Error parsing form data: {error}")),
    };
    Ok(formData
        .into_iter()
        .map(|(key, value)| (key, jsonValueToString(&value)))
        .collect())
}

#[allow(non_snake_case)]
fn formUrlEncoded(object: &Map<String, Value>) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in object {
        serializer.append_pair(key, &jsonValueToString(value));
    }
    serializer.finish()
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
fn applyCustomCookies(tool: &AITool, useCookies: bool, url: &str) {
    if let Some(customCookies) =
        optionalParameterValue(tool, "custom_cookies").filter(|value| !value.trim().is_empty())
    {
        if useCookies {
            if let Some(cookies) = parseCookies(&customCookies, url) {
                if let Some(host) = Url::parse(url)
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
}

#[allow(non_snake_case)]
fn saveResponseCookies(url: &str, headers: &[(String, String)]) {
    let Ok(parsedUrl) = Url::parse(url) else {
        return;
    };
    let Some(host) = parsedUrl.host_str() else {
        return;
    };
    let mut cookies = Vec::new();
    for (name, value) in headers {
        if name.eq_ignore_ascii_case("set-cookie") {
            if let Some(cookie) = parseSetCookie(value, host) {
                cookies.push(cookie);
            }
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
fn pushCookieHeader(headers: &mut Vec<(String, String)>, url: &str) {
    let cookieHeader = cookiesForUrl(url)
        .into_iter()
        .map(|cookie| format!("{}={}", cookie.name, cookie.value))
        .collect::<Vec<_>>()
        .join("; ");
    if !cookieHeader.is_empty() {
        headers.push(("Cookie".to_string(), cookieHeader));
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
fn responseToData(requestUrl: &str, response: HostHttpResponseData) -> ToolResultData {
    saveResponseCookies(requestUrl, &response.headers);
    let contentType = response
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.clone())
        .unwrap_or_default();
    let content = String::from_utf8(response.body.clone())
        .unwrap_or_else(|_| "[Binary Content, decoding failed]".to_string());
    ToolResultData::HttpResponseData(HttpResponseData {
        url: response.finalUrl,
        statusCode: response.statusCode,
        statusMessage: response.statusMessage,
        headers: response.headers.into_iter().collect(),
        contentType,
        content,
        contentBase64: Some(STANDARD.encode(&response.body)),
        size: response.body.len() as i32,
        cookies: cookiesMapForUrl(requestUrl),
    })
}

fn successData(tool: &AITool, data: ToolResultData) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result: data,
        error: None,
    }
}

#[allow(non_snake_case)]
fn buildHttpResponseData(
    url: &str,
    statusCode: i32,
    statusMessage: &str,
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
        result: stringResultData(result),
        error: None,
    }
}

#[allow(non_snake_case)]
fn errorResult(toolName: &str, message: &str) -> ToolResult {
    ToolResult {
        toolName: toolName.to_string(),
        success: false,
        result: stringResultData(""),
        error: Some(message.to_string()),
    }
}

#[allow(non_snake_case)]
fn toolError(tool: &AITool, result: String, message: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result: stringResultData(result),
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
