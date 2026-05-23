use std::collections::BTreeMap;

use crate::api::chat::enhance::ToolExecutionManager::{AITool, ToolParameter};
use crate::core::tools::AIToolHandler::AIToolHandler;

#[derive(Clone, Debug)]
struct ParsedToolCall {
    params: BTreeMap<String, String>,
    fullToolName: String,
    aiTool: AITool,
}

#[allow(non_snake_case)]
fn buildToolErrorJson(message: &str) -> String {
    serde_json::json!({
        "success": false,
        "message": message
    })
    .to_string()
}

#[allow(non_snake_case)]
fn parseToolCall(toolType: &str, toolName: &str, paramsJson: &str) -> Result<ParsedToolCall, String> {
    let normalizedToolName = toolName.trim();
    if normalizedToolName.is_empty() {
        return Err("Tool name cannot be empty".to_string());
    }

    let value = serde_json::from_str::<serde_json::Value>(paramsJson)
        .map_err(|error| error.to_string())?;
    let object = value
        .as_object()
        .ok_or_else(|| "Tool params must be a JSON object".to_string())?;

    let mut params = BTreeMap::new();
    for (key, value) in object {
        let text = match value {
            serde_json::Value::Null => String::new(),
            serde_json::Value::String(value) => value.clone(),
            _ => value.to_string(),
        };
        params.insert(key.clone(), text);
    }

    let fullToolName = if !toolType.is_empty() && toolType != "default" {
        format!("{toolType}:{normalizedToolName}")
    } else {
        normalizedToolName.to_string()
    };
    let toolParameters = params
        .iter()
        .map(|(name, value)| ToolParameter {
            name: name.clone(),
            value: value.clone(),
        })
        .collect();

    Ok(ParsedToolCall {
        params,
        fullToolName: fullToolName.clone(),
        aiTool: AITool {
            name: fullToolName,
            parameters: toolParameters,
        },
    })
}

#[allow(non_snake_case)]
fn serializeToolExecutionResult(result: &crate::api::chat::enhance::ConversationMarkupManager::ToolResult) -> String {
    let mut object = serde_json::Map::new();
    object.insert("success".to_string(), serde_json::Value::Bool(result.success));
    if !result.success {
        object.insert(
            "message".to_string(),
            serde_json::Value::String(result.error.clone().unwrap_or_default()),
        );
    }
    object.insert(
        "data".to_string(),
        serde_json::Value::String(result.result.clone()),
    );
    serde_json::Value::Object(object).to_string()
}

#[allow(non_snake_case)]
pub fn callToolSync(
    toolHandler: &AIToolHandler,
    toolType: &str,
    toolName: &str,
    paramsJson: &str,
) -> String {
    if toolName.trim().is_empty() {
        return buildToolErrorJson("Tool name cannot be empty");
    }

    let parsed = match parseToolCall(toolType, toolName, paramsJson) {
        Ok(value) => value,
        Err(error) => return buildToolErrorJson(&error),
    };
    let _ = parsed.params.len();
    let _ = parsed.fullToolName.as_str();

    let mut handler = toolHandler.clone();
    let result = handler.executeTool(parsed.aiTool);
    serializeToolExecutionResult(&result)
}
