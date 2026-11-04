use std::collections::BTreeMap;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{Map, Value};

use operit_tools::tools::mcp::MCPManager::MCPManager;
use operit_tools::tools::mcp::MCPToolParameter::MCPToolParameter;
use operit_tools::tools::ToolExecutionLimits::ToolExecutionLimits;
use operit_tools::tools::ToolResultDataClasses::stringResultData;
use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{
    AITool, ToolAccessSpec, ToolBoundary, ToolEffect, AsyncToolExecutor, ToolInvocationFuture, ToolValidationResult,
};
use operit_util::AppLogger::AppLogger;
use operit_util::ImagePoolManager::ImagePoolManager;
use operit_util::MediaLinkBuilder::MediaLinkBuilder;

const TAG: &str = "MCPToolExecutor";
const REPLACEMENT_CHARACTER: char = '\u{FFFD}';

struct ArgumentIntegrityViolation {
    parameter_name: String,
    replacement_character_count: usize,
    character_offset: usize,
    utf8_byte_offset: usize,
}

#[derive(Clone)]
/// Tool executor that dispatches package-qualified calls to MCP servers.
pub struct MCPToolExecutor {
    mcpManager: MCPManager,
}

impl MCPToolExecutor {
    /// Creates an MCP tool executor backed by the shared MCP manager.
    pub fn new(mcpManager: MCPManager) -> Self {
        Self { mcpManager }
    }

    #[allow(non_snake_case)]
    fn truncateResult(&self, result: String) -> String {
        let maxResultLength = ToolExecutionLimits::MAX_TEXT_RESULT_LENGTH;
        if result.len() <= maxResultLength {
            return result;
        }
        let truncated = result.chars().take(maxResultLength).collect::<String>();
        let remainingLength = result.chars().count().saturating_sub(maxResultLength);
        format!(
            "{truncated}\n\n[... Result too long, truncated {remainingLength} characters. Recommend using file operations or pagination.]"
        )
    }

    #[allow(non_snake_case)]
    fn extractContentFromResult(&self, resultData: Option<&Value>) -> String {
        let Some(resultData) = resultData else {
            return "{}".to_string();
        };
        let mut imageBudget = ImagePoolManager::max_pool_size();
        let contentText = resultData
            .get("content")
            .and_then(Value::as_array)
            .filter(|items| !items.is_empty())
            .map(|items| {
                items
                    .iter()
                    .map(|contentItem| self.extractContentItem(contentItem, &mut imageBudget))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();

        let metadataText = resultData
            .as_object()
            .map(|object| {
                let metadata = object
                    .iter()
                    .filter(|(key, _)| key.as_str() != "content")
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect::<Map<String, Value>>();
                if metadata.is_empty() {
                    String::new()
                } else {
                    Value::Object(metadata).to_string()
                }
            })
            .unwrap_or_default();

        match (!metadataText.is_empty(), !contentText.is_empty()) {
            (true, true) => format!("{metadataText}\n\n{contentText}"),
            (true, false) => metadataText,
            (false, true) => contentText,
            (false, false) => resultData.to_string(),
        }
    }

    #[allow(non_snake_case)]
    fn extractContentItem(&self, contentItem: &Value, imageBudget: &mut usize) -> String {
        let contentType = contentItem
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("text");
        match contentType {
            "text" => {
                let text = contentItem
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                match serde_json::from_str::<Value>(text) {
                    Ok(value @ (Value::Object(_) | Value::Array(_))) => value.to_string(),
                    _ => text.to_string(),
                }
            }
            "image" if *imageBudget == 0 => "[Image omitted: attachment limit reached]".to_string(),
            "image" => {
                let mimeType = contentItem
                    .get("mimeType")
                    .and_then(Value::as_str)
                    .unwrap_or("image/png");
                let data = contentItem
                    .get("data")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                Self::registerImageContent(data).inspect(|_| *imageBudget -= 1).unwrap_or_else(|| {
                    format!(
                        "[Image: {mimeType}, Size: {} bytes; the image data could not be attached]",
                        data.len()
                    )
                })
            }
            "resource" => {
                let Some(resource) = contentItem.get("resource") else {
                    return "[Resource: ]".to_string();
                };
                if let Some(text) = resource.get("text").and_then(Value::as_str) {
                    if !text.is_empty() {
                        return text.to_string();
                    }
                }
                let uri = resource
                    .get("uri")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                format!("[Resource: {uri}]")
            }
            other => format!("[Unknown content type '{other}': {contentItem}]"),
        }
    }

    #[allow(non_snake_case)]
    fn registerImageContent(data: &str) -> Option<String> {
        let bytes = STANDARD.decode(data.trim()).ok()?;
        let format = image::guess_format(&bytes).ok()?;
        match format {
            image::ImageFormat::Png | image::ImageFormat::Jpeg => {
                image::ImageReader::with_format(std::io::Cursor::new(&bytes), format)
                    .into_dimensions().ok()?;
            }
            image::ImageFormat::Gif | image::ImageFormat::WebP => {}
            _ => return None,
        }
        let id = ImagePoolManager::add_image_bytes(&bytes, Some(format.to_mime_type()), None);
        (id != "error").then(|| MediaLinkBuilder::image(&id))
    }

    #[allow(non_snake_case)]
    fn getToolInfo(&self, serverName: &str, toolName: &str) -> Option<Value> {
        let client = self.mcpManager.getClientForMetadata(serverName)?;
        client
            .getTools()
            .into_iter()
            .find(|tool| tool.get("name").and_then(Value::as_str) == Some(toolName))
    }

    #[allow(non_snake_case)]
    /// Filters top-level arguments against the tool definition before converting their types.
    /// Without usable properties, preserve arguments rather than guessing which are valid.
    fn convertParameterTypes(
        parameters: BTreeMap<String, Value>,
        toolInfo: Option<&Value>,
    ) -> BTreeMap<String, Value> {
        let properties = toolInfo
            .and_then(|tool| tool.get("inputSchema"))
            .and_then(|schema| schema.get("properties"))
            .and_then(Value::as_object);
        let mut result = BTreeMap::new();
        let mut filteredNames = Vec::new();
        for (name, value) in parameters {
            if properties.is_some_and(|properties| !properties.contains_key(&name)) {
                filteredNames.push(name);
                continue;
            }
            let expectedType = properties
                .and_then(|properties| properties.get(&name))
                .and_then(|parameter| parameter.get("type"))
                .and_then(Value::as_str);
            result.insert(name, MCPToolParameter::smartConvert(value, expectedType));
        }
        if !filteredNames.is_empty() {
            AppLogger::v_with_level(
                TAG,
                &format!(
                    "Filtered undeclared MCP arguments: tool={}, parameters={}",
                    toolInfo
                        .and_then(|tool| tool.get("name"))
                        .and_then(Value::as_str)
                        .unwrap_or("unknown"),
                    filteredNames.join(", "),
                ),
                operit_util::AppLogger::VERBOSE_LEVEL_3,
            );
        }
        result
    }

    /// Returns the first tool parameter whose text already contains replacement characters.
    fn findArgumentIntegrityViolation(&self, tool: &AITool) -> Option<ArgumentIntegrityViolation> {
        let parameter = tool
            .parameters
            .iter()
            .find(|parameter| parameter.value.contains(REPLACEMENT_CHARACTER))?;
        let utf8_byte_offset = parameter.value.find(REPLACEMENT_CHARACTER)?;

        Some(ArgumentIntegrityViolation {
            parameter_name: parameter.name.clone(),
            replacement_character_count: parameter
                .value
                .chars()
                .filter(|character| *character == REPLACEMENT_CHARACTER)
                .count(),
            character_offset: parameter.value[..utf8_byte_offset].chars().count(),
            utf8_byte_offset,
        })
    }

    #[allow(non_snake_case)]
    /// Invokes one MCP tool and converts its structured response into tool result text.
    pub async fn invoke(&self, tool: &AITool) -> ToolResult {
        let toolNameParts = tool.name.split(':').collect::<Vec<_>>();
        if toolNameParts.len() < 2 {
            return ToolResult {
                toolName: tool.name.clone(),
                success: false,
                result: stringResultData(""),
                error: Some(
                    "Invalid MCP tool name format, should be 'server_name:tool_name'".to_string(),
                ),
            };
        }
        let serverName = toolNameParts[0];
        let actualToolName = toolNameParts[1..].join(":");
        let Some(mcpClient) = self.mcpManager.getOrCreateClient(serverName).await else {
            let error = self
                .mcpManager
                .getLastConnectionFailureReason(serverName)
                .map(|reason| format!("Cannot connect to MCP server '{serverName}': {reason}"))
                .unwrap_or_else(|| format!("Cannot connect to MCP server: {serverName}"));
            return ToolResult {
                toolName: tool.name.clone(),
                success: false,
                result: stringResultData(""),
                error: Some(error),
            };
        };
        if !mcpClient.isActive() {
            return ToolResult {
                toolName: tool.name.clone(),
                success: false,
                result: stringResultData(""),
                error: Some(format!(
                    "MCP service '{serverName}' is not activated. Please use the 'use_package' tool with the package name '{serverName}' to activate it first."
                )),
            };
        }
        let parameters = tool
            .parameters
            .iter()
            .map(|parameter| {
                (
                    parameter.name.clone(),
                    Value::String(parameter.value.clone()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let toolInfo = self.getToolInfo(serverName, &actualToolName);
        let convertedParameters = Self::convertParameterTypes(parameters, toolInfo.as_ref());
        let response = mcpClient.callToolWithArguments(&actualToolName, convertedParameters).await;
        if response
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let extractedContent = self.extractContentFromResult(response.get("result"));
            return ToolResult {
                toolName: tool.name.clone(),
                success: true,
                result: stringResultData(self.truncateResult(extractedContent)),
                error: None,
            };
        }
        let errorMessage = response
            .get("error")
            .map(|error| {
                let code = error.get("code").and_then(Value::as_i64).unwrap_or(-1);
                let message = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Unknown error");
                format!("[{code}] {message}")
            })
            .unwrap_or_else(|| "Tool call failed but no error message returned".to_string());
        ToolResult {
            toolName: tool.name.clone(),
            success: false,
            result: stringResultData(""),
            error: Some(errorMessage),
        }
    }
}

impl AsyncToolExecutor for MCPToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        let toolNameParts = tool.name.split(':').collect::<Vec<_>>();
        if toolNameParts.len() < 2 {
            return ToolValidationResult {
                valid: false,
                errorMessage: "Invalid MCP tool name format, should be 'server_name:tool_name'"
                    .to_string(),
            };
        }

        if let Some(violation) = self.findArgumentIntegrityViolation(tool) {
            // U+FFFD means the original argument bytes have already been lost. Sending a write
            // request would silently persist corrupted user content on the remote MCP server.
            AppLogger::e(
                TAG,
                &format!(
                    "Blocked MCP tool call with corrupted argument: tool={}, parameter={}, replacementCount={}, characterOffset={}, utf8ByteOffset={}",
                    tool.name,
                    violation.parameter_name,
                    violation.replacement_character_count,
                    violation.character_offset,
                    violation.utf8_byte_offset,
                ),
            );
            return ToolValidationResult {
                valid: false,
                errorMessage: format!(
                    "MCP tool argument '{}' contains {} invalid UTF-8 replacement character(s) (U+FFFD); the call was blocked before it reached the MCP server. First UTF-8 byte offset: {}",
                    violation.parameter_name,
                    violation.replacement_character_count,
                    violation.utf8_byte_offset,
                ),
            };
        }

        ToolValidationResult {
            valid: true,
            errorMessage: String::new(),
        }
    }

    fn accessSpec(&self, _tool: &AITool) -> Result<ToolAccessSpec, String> {
        Ok(ToolAccessSpec {
            effect: ToolEffect::WRITE,
            boundary: ToolBoundary::None,
        })
    }

    fn invokeAndStreamAsync<'a>(&'a mut self, tool: &'a AITool, context: &'a crate::ToolExecutionManager::ToolInvocationContext) -> ToolInvocationFuture<'a> {
        Box::pin(async move { vec![self.invoke(tool).await] })
    }
}

#[cfg(test)]
mod tests {
    use super::MCPToolExecutor;
    use crate::tools::mcp::MCPManager::MCPManager;
    use operit_host_api::HostManager::HostManager;
    use operit_util::ImagePoolManager::ImagePoolManager;
    use serde_json::{json, Value};
    use std::collections::BTreeMap;

    /// Standard 1x1 PNG payload used to exercise real image registration.
    const ONE_PIXEL_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

    fn executor() -> MCPToolExecutor {
        MCPToolExecutor::new(MCPManager::getInstance(HostManager::default()))
    }

    fn arguments(value: Value) -> BTreeMap<String, Value> {
        value.as_object().unwrap().clone().into_iter().collect()
    }

    /// Drops unknown top-level arguments while preserving schema-based scalar conversion.
    #[test]
    fn undeclared_arguments_are_filtered_before_conversion() {
        let tool = json!({
            "name": "write_file",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "content": {"type": "string"},
                    "count": {"type": "integer"},
                    "enabled": {"type": "boolean"}
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }
        });
        let actual = MCPToolExecutor::convertParameterTypes(
            arguments(json!({
                "path": "test.txt", "content": "00123", "count": "3", "enabled": "true",
                "extra": "[1,2]", "tool_name": "server:write_file"
            })),
            Some(&tool),
        );
        assert_eq!(
            actual,
            arguments(json!({
                "path": "test.txt", "content": "00123", "count": 3, "enabled": true
            }))
        );
    }

    /// A tool with an explicit empty property map receives no arguments.
    #[test]
    fn parameterless_tool_filters_all_arguments() {
        let tool = json!({"inputSchema": {"type": "object", "properties": {}}});
        assert!(MCPToolExecutor::convertParameterTypes(
            arguments(json!({"extra": "value"})),
            Some(&tool)
        )
        .is_empty());
    }

    /// Missing or unusable definitions must not silently erase all supplied arguments.
    #[test]
    fn unavailable_properties_preserve_arguments() {
        let inputs = arguments(json!({"path": "test.txt", "count": "3"}));
        let expected = arguments(json!({"path": "test.txt", "count": 3}));
        assert_eq!(
            MCPToolExecutor::convertParameterTypes(inputs.clone(), None),
            expected
        );
        for tool in [
            json!({}),
            json!({"inputSchema": {"type": "object"}}),
            json!({"inputSchema": {"properties": null}}),
            json!({"inputSchema": {"properties": []}}),
        ] {
            assert_eq!(
                MCPToolExecutor::convertParameterTypes(inputs.clone(), Some(&tool)),
                expected
            );
        }
    }

    /// Only argument names are filtered; nested payload keys and strings remain intact.
    #[test]
    fn declared_optional_and_nested_arguments_are_preserved() {
        let payload = json!({"extra": "00123", "items": ["true", {"unknown": "false"}]});
        let tool = json!({"inputSchema": {
            "properties": {"payload": {"type": "object"}, "optional": {}},
            "required": ["payload"]
        }});
        assert_eq!(
            MCPToolExecutor::convertParameterTypes(
                arguments(
                    json!({"payload": payload.to_string(), "optional": "note", "extra": "drop"})
                ),
                Some(&tool),
            ),
            arguments(json!({"payload": payload, "optional": "note"})),
        );
    }

    /// Image payloads register into the image pool and surface as canonical
    /// media-link tags that providers resolve into multimodal image parts.
    #[test]
    fn image_content_registers_payload_and_omits_excess() {
        let executor = executor();
        let item = json!({"type": "image", "mimeType": "image/jpeg", "data": ONE_PIXEL_PNG_BASE64});
        let mut budget = 1;
        let result = executor.extractContentItem(&item, &mut budget);
        assert_eq!(executor.extractContentItem(&item, &mut budget), "[Image omitted: attachment limit reached]");
        let id = result
            .split("id=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or_default()
            .to_string();
        assert!(result.contains("<link type=\"image\""), "tag: {result}");
        let image = ImagePoolManager::get_image(&id).expect("registered image");
        assert_eq!(image.mime_type, "image/png");
        assert_eq!(image.width, 1);
        assert_eq!(image.height, 1);
        assert!(!image.base64.is_empty());
        ImagePoolManager::remove_image(&id);
    }

    /// Undecodable image data degrades to the informative placeholder instead
    /// of emitting a dead media link.
    #[test]
    fn invalid_image_data_falls_back_to_placeholder() {
        let executor = executor();
        for data in [
            "!!!!!!".to_string(),
            format!("!{ONE_PIXEL_PNG_BASE64}"),
            "aGVsbG8=".to_string(),
            "iVBORw0KGgo=".to_string(),
        ] {
            let item = json!({"type": "image", "mimeType": "image/png", "data": data});
            let result = executor.extractContentItem(&item, &mut 1);
            assert!(result.contains("could not be attached"), "tag: {result}");
            assert!(!result.contains("<link"), "tag: {result}");
        }
    }

    /// Empty image data never reaches the image pool.
    #[test]
    fn empty_image_data_falls_back_to_placeholder() {
        let executor = executor();
        let item = json!({"type": "image", "mimeType": "image/png", "data": "   "});
        let result = executor.extractContentItem(&item, &mut 1);
        assert!(result.contains("[Image: image/png, Size: 3 bytes"), "tag: {result}");
    }

    /// Mixed content keeps text payloads alongside registered image links.
    #[test]
    fn mixed_content_preserves_text_and_image_items() {
        let executor = executor();
        let resultData = json!({
            "content": [
                {"type": "text", "text": "screenshot ready"},
                {"type": "text", "text": " { \"ok\": true } "},
                {"type": "text", "text": " 1 "},
                {"type": "image", "mimeType": "image/png", "data": ONE_PIXEL_PNG_BASE64},
            ]
        });
        let result = executor.extractContentFromResult(Some(&resultData));
        assert!(result.starts_with("screenshot ready\n{\"ok\":true}\n 1 \n"), "tag: {result}");
        assert!(result.contains("<link type=\"image\""), "tag: {result}");
        let id = result
            .split("id=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or_default()
            .to_string();
        ImagePoolManager::remove_image(&id);
    }
}
