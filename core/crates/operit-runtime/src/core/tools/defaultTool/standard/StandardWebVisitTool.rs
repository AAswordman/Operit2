use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use operit_host_api::{WebVisitHost, WebVisitRequest, WebVisitResult};
use reqwest::Url;

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::{AITool, ToolExecutor, ToolValidationResult};
use crate::core::tools::ToolResultDataClasses::{
    stringResultData, LinkData, ToolResultData, VisitWebResultData,
};

#[derive(Clone)]
pub struct StandardWebVisitTool {
    pub webVisitHost: Option<Arc<dyn WebVisitHost>>,
}

static VISIT_CACHE: OnceLock<Mutex<HashMap<String, VisitWebResultData>>> = OnceLock::new();

const USER_AGENT_DESKTOP: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36";
const USER_AGENT_ANDROID: &str = "Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36";
const MAX_INLINE_VISIT_CONTENT_CHARS: usize = 12_000;
const MAX_INLINE_VISIT_CONTENT_PREVIEW_CHARS: usize = 8_000;

#[allow(non_snake_case)]
fn webVisitResultToVisitData(value: WebVisitResult) -> VisitWebResultData {
    VisitWebResultData {
        url: value.url,
        title: value.title,
        content: value.content,
        metadata: value.metadata.into_iter().collect(),
        links: value.links.into_iter().map(LinkData::from).collect(),
        imageLinks: value.imageLinks,
        visitKey: None,
        contentSavedTo: None,
        contentTruncated: false,
        originalContentLength: None,
    }
}

impl StandardWebVisitTool {
    pub fn new(webVisitHost: Option<Arc<dyn WebVisitHost>>) -> Self {
        Self { webVisitHost }
    }

    #[allow(non_snake_case)]
    pub fn getCachedVisitResult(visitKey: &str) -> Option<VisitWebResultData> {
        if visitKey.trim().is_empty() {
            return None;
        }
        VISIT_CACHE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .expect("visit cache mutex poisoned")
            .get(visitKey)
            .cloned()
    }

    #[allow(non_snake_case)]
    fn putCachedVisitResult(result: VisitWebResultData) {
        if let Some(key) = result.visitKey.clone() {
            VISIT_CACHE
                .get_or_init(|| Mutex::new(HashMap::new()))
                .lock()
                .expect("visit cache mutex poisoned")
                .insert(key, result);
        }
    }

    #[allow(non_snake_case)]
    fn visitWebPage(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
        userAgent: &str,
        includeImageLinks: bool,
    ) -> Result<VisitWebResultData, String> {
        let Some(host) = self.webVisitHost.as_ref() else {
            return Err("WebVisitHost is not registered for this runtime.".to_string());
        };
        host.visitWeb(WebVisitRequest {
            url: url.to_string(),
            headers: headers
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            userAgent: userAgent.to_string(),
            includeImageLinks,
        })
        .map(webVisitResultToVisitData)
        .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    fn persistVisitContentIfNeeded(&self, resultData: VisitWebResultData) -> VisitWebResultData {
        if resultData.content.len() <= MAX_INLINE_VISIT_CONTENT_CHARS {
            return resultData;
        }
        let outputFile = writeVisitResultToFile(&resultData);
        let preview = buildInlineContentPreview(&resultData.content, &outputFile);
        resultData.copy_with_preview(preview, outputFile)
    }

    #[allow(non_snake_case)]
    fn buildFullVisitResultText(resultData: &VisitWebResultData) -> String {
        let mut sb = String::new();
        if let Some(visitKey) = &resultData.visitKey {
            sb.push_str(&format!("Visit key: {visitKey}\n"));
        }
        if !resultData.title.trim().is_empty() {
            sb.push_str(&format!("Title: {}\n", resultData.title));
        }
        sb.push_str(&format!("URL: {}\n", resultData.url));
        if !resultData.metadata.is_empty() {
            sb.push('\n');
            sb.push_str("Metadata:\n");
            let mut entries = resultData.metadata.iter().collect::<Vec<_>>();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            for (key, value) in entries {
                sb.push_str(&format!("{key}: {value}\n"));
            }
        }
        if !resultData.links.is_empty() {
            sb.push('\n');
            sb.push_str("Results:\n");
            for (index, link) in resultData.links.iter().enumerate() {
                sb.push_str(&format!("[{}] {}\n", index + 1, link.text));
                sb.push_str(&format!("    URL: {}\n", link.url));
            }
        }
        if !resultData.imageLinks.is_empty() {
            sb.push('\n');
            sb.push_str("Images:\n");
            for (index, link) in resultData.imageLinks.iter().enumerate() {
                let name = link
                    .rsplit('/')
                    .next()
                    .and_then(|part| part.split('?').next())
                    .filter(|part| !part.is_empty())
                    .unwrap_or("image");
                sb.push_str(&format!("[{}] {}\n", index + 1, name));
                sb.push_str(&format!("    URL: {}\n", link));
            }
        }
        sb.push('\n');
        sb.push_str("Content:\n");
        sb.push_str(&resultData.content);
        sb
    }
}

impl ToolExecutor for StandardWebVisitTool {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        let url = parameterValue(tool, "url");
        let visitKey = parameterValue(tool, "visit_key");
        let linkNumber = parameterValue(tool, "link_number");
        let isUrlVisit = !url.trim().is_empty();
        let isKeyVisit = !visitKey.trim().is_empty() && !linkNumber.trim().is_empty();
        if isUrlVisit || isKeyVisit {
            ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            }
        } else {
            ToolValidationResult {
                valid: false,
                errorMessage:
                    "Either 'url' or both 'visit_key' and 'link_number' must be provided."
                        .to_string(),
            }
        }
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        vec![self.invoke(tool)]
    }
}

impl StandardWebVisitTool {
    #[allow(non_snake_case)]
    pub fn invoke(&self, tool: &AITool) -> ToolResult {
        let url = parameterValue(tool, "url");
        let visitKey = parameterValue(tool, "visit_key");
        let linkNumberStr = parameterValue(tool, "link_number");
        let includeImageLinks =
            parseBoolean(optionalParameterValue(tool, "include_image_links").as_deref());
        let headersParam = optionalParameterValue(tool, "headers");
        let userAgentParam = optionalParameterValue(tool, "user_agent");
        let userAgentPresetParam = optionalParameterValue(tool, "user_agent_preset");

        let targetUrl = match resolveTargetUrl(&url, &visitKey, &linkNumberStr) {
            Ok(targetUrl) => targetUrl,
            Err(message) => {
                return toolError(tool, String::new(), message);
            }
        };

        let Some(parsedUrl) = Url::parse(&targetUrl).ok() else {
            return toolError(
                tool,
                String::new(),
                format!("Unsupported URL scheme: unknown"),
            );
        };
        if parsedUrl.scheme() != "http" && parsedUrl.scheme() != "https" {
            return toolError(
                tool,
                String::new(),
                format!("Unsupported URL scheme: {}", parsedUrl.scheme()),
            );
        }

        let headers = match parseHeaders(headersParam.as_deref()) {
            Ok(headers) => headers,
            Err(error) => return toolError(tool, String::new(), error),
        };
        let userAgent =
            resolveUserAgent(userAgentPresetParam.as_deref(), userAgentParam.as_deref());

        match self.visitWebPage(&targetUrl, &headers, &userAgent, includeImageLinks) {
            Ok(resultData) => {
                let visitKey = uuid::Uuid::new_v4().to_string();
                let mut stored = resultData.clone();
                stored.visitKey = Some(visitKey.clone());
                let stored = self.persistVisitContentIfNeeded(stored);
                Self::putCachedVisitResult(stored.clone());
                toolResultFromVisitWebData(tool, stored)
            }
            Err(error) => toolError(
                tool,
                String::new(),
                format!("Error visiting web page: {error}"),
            ),
        }
    }
}

#[allow(non_snake_case)]
fn resolveTargetUrl(url: &str, visitKey: &str, linkNumberStr: &str) -> Result<String, String> {
    if !visitKey.trim().is_empty() && !linkNumberStr.trim().is_empty() {
        let linkNumber = linkNumberStr
            .trim()
            .parse::<usize>()
            .map_err(|_| "Invalid link number.".to_string())?;
        let cached = StandardWebVisitTool::getCachedVisitResult(visitKey)
            .ok_or_else(|| "Invalid visit key.".to_string())?;
        let link = cached
            .links
            .get(linkNumber.saturating_sub(1))
            .ok_or_else(|| "Link number out of bounds.".to_string())?;
        return Ok(link.url.clone());
    }
    if !url.trim().is_empty() {
        return Ok(url.trim().to_string());
    }
    Err("Either 'url' or both 'visit_key' and 'link_number' must be provided.".to_string())
}

#[allow(non_snake_case)]
fn toolResultFromVisitWebData(tool: &AITool, result: VisitWebResultData) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result: ToolResultData::VisitWebResultData(result),
        error: None,
    }
}

#[allow(non_snake_case)]
fn parseHeaders(headersJson: Option<&str>) -> Result<HashMap<String, String>, String> {
    let Some(raw) = headersJson.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(HashMap::new());
    };
    let value = serde_json::from_str::<serde_json::Value>(raw)
        .map_err(|error| format!("Invalid headers JSON: {error}"))?;
    let Some(object) = value.as_object() else {
        return Err("headers must be a JSON object string".to_string());
    };
    let mut headers = HashMap::new();
    for (key, value) in object {
        let Some(text) = value.as_str() else {
            return Err(format!("headers.{key} must be a string"));
        };
        headers.insert(key.clone(), text.to_string());
    }
    Ok(headers)
}

#[allow(non_snake_case)]
fn resolveUserAgent(preset: Option<&str>, overrideValue: Option<&str>) -> String {
    if let Some(value) = overrideValue
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return value.to_string();
    }
    match preset.map(|value| value.trim().to_lowercase()).as_deref() {
        Some("android") | Some("mobile_android") | Some("mobile") => USER_AGENT_ANDROID.to_string(),
        _ => USER_AGENT_DESKTOP.to_string(),
    }
}

#[allow(non_snake_case)]
fn parseBoolean(raw: Option<&str>) -> bool {
    matches!(
        raw.map(|value| value.trim().to_lowercase()),
        Some(value) if value == "true" || value == "1" || value == "yes" || value == "y" || value == "on"
    )
}

#[allow(non_snake_case)]
fn writeVisitResultToFile(resultData: &VisitWebResultData) -> PathBuf {
    let mut outputDir = std::env::temp_dir();
    outputDir.push("operit2");
    outputDir.push("visit_web");
    let _ = fs::create_dir_all(&outputDir);
    let hostRaw = Url::parse(&resultData.url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_else(|| "page".to_string())
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let host = hostRaw.trim_matches('_').to_string();
    let filePath = outputDir.join(format!(
        "visit_web_{}_{}_{}.txt",
        host,
        operit_host_api::TimeUtils::currentTimeMillis(),
        &uuid::Uuid::new_v4().to_string()[..8]
    ));
    let text = StandardWebVisitTool::buildFullVisitResultText(resultData);
    let _ = fs::write(&filePath, text);
    filePath
}

#[allow(non_snake_case)]
fn buildInlineContentPreview(fullContent: &str, savedPath: &PathBuf) -> String {
    let preview = fullContent
        .chars()
        .take(MAX_INLINE_VISIT_CONTENT_PREVIEW_CHARS)
        .collect::<String>();
    format!(
        "{preview}\n\n[Content truncated. Full content saved to file: {}]",
        savedPath.display()
    )
}

impl VisitWebResultData {
    fn copy_with_preview(mut self, preview: String, savedPath: PathBuf) -> Self {
        let originalLength = self.content.chars().count();
        self.content = preview;
        self.contentSavedTo = Some(savedPath.display().to_string());
        self.contentTruncated = true;
        self.originalContentLength = Some(originalLength);
        self
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
