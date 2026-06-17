use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::{AITool, ToolExecutor, ToolValidationResult};
use crate::core::tools::mcp::MCPManager::MCPManager;
use crate::core::tools::mcp::MCPToolParameter::MCPToolParameter;
use crate::core::tools::ToolExecutionLimits::ToolExecutionLimits;
use crate::core::tools::ToolResultDataClasses::stringResultData;

#[derive(Clone)]
pub struct MCPToolExecutor {
    mcpManager: MCPManager,
}

impl MCPToolExecutor {
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
        let contentText = resultData
            .get("content")
            .and_then(Value::as_array)
            .filter(|items| !items.is_empty())
            .map(|items| {
                items
                    .iter()
                    .map(|contentItem| self.extractContentItem(contentItem))
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
    fn extractContentItem(&self, contentItem: &Value) -> String {
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
                if self.isJsonString(text) {
                    self.formatJson(text)
                } else {
                    text.to_string()
                }
            }
            "image" => {
                let mimeType = contentItem
                    .get("mimeType")
                    .and_then(Value::as_str)
                    .unwrap_or("image/png");
                let dataSize = contentItem
                    .get("data")
                    .and_then(Value::as_str)
                    .map(|data| data.len())
                    .unwrap_or(0);
                format!("[Image: {mimeType}, Size: {dataSize} bytes]")
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
    fn isJsonString(&self, text: &str) -> bool {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return false;
        }
        let isJsonObject = trimmed.starts_with('{') && trimmed.ends_with('}');
        let isJsonArray = trimmed.starts_with('[') && trimmed.ends_with(']');
        if !isJsonObject && !isJsonArray {
            return false;
        }
        serde_json::from_str::<Value>(trimmed).is_ok()
    }

    #[allow(non_snake_case)]
    fn formatJson(&self, jsonString: &str) -> String {
        serde_json::from_str::<Value>(jsonString.trim())
            .map(|value| value.to_string())
            .unwrap_or_else(|_| jsonString.to_string())
    }

    #[allow(non_snake_case)]
    fn getToolInfo(&self, serverName: &str, toolName: &str) -> Option<Value> {
        let client = self.mcpManager.getOrCreateClient(serverName)?;
        client
            .getTools()
            .into_iter()
            .find(|tool| tool.get("name").and_then(Value::as_str) == Some(toolName))
    }

    #[allow(non_snake_case)]
    fn convertParameterTypes(
        &self,
        parameters: BTreeMap<String, Value>,
        toolInfo: Option<&Value>,
    ) -> BTreeMap<String, Value> {
        let mut result = BTreeMap::new();
        for (name, value) in parameters {
            let expectedType = toolInfo
                .and_then(|tool| tool.get("inputSchema"))
                .and_then(|schema| schema.get("properties"))
                .and_then(|properties| properties.get(&name))
                .and_then(|parameter| parameter.get("type"))
                .and_then(Value::as_str);
            result.insert(name, MCPToolParameter::smartConvert(value, expectedType));
        }
        result
    }

    #[allow(non_snake_case)]
    pub fn invoke(&self, tool: &AITool) -> ToolResult {
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
        let Some(mcpClient) = self.mcpManager.getOrCreateClient(serverName) else {
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
        let convertedParameters = self.convertParameterTypes(parameters, toolInfo.as_ref());
        let response = mcpClient.callToolSync(&actualToolName, convertedParameters);
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

impl ToolExecutor for MCPToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        let toolNameParts = tool.name.split(':').collect::<Vec<_>>();
        if toolNameParts.len() < 2 {
            return ToolValidationResult {
                valid: false,
                errorMessage: "Invalid MCP tool name format, should be 'server_name:tool_name'"
                    .to_string(),
            };
        }
        ToolValidationResult {
            valid: true,
            errorMessage: String::new(),
        }
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        vec![self.invoke(tool)]
    }
}
