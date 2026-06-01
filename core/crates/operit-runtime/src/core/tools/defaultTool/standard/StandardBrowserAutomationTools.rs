use std::sync::Arc;

use operit_host_api::{BrowserAutomationHost, BrowserAutomationRequest};
use serde_json::{Map, Value};

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::{
    AITool, ToolExecutionManager, ToolExecutor, ToolValidationResult,
};

#[derive(Clone)]
pub struct StandardBrowserAutomationTools {
    browserHost: Arc<dyn BrowserAutomationHost>,
}

pub struct BrowserAutomationToolExecutor {
    pub tools: StandardBrowserAutomationTools,
}

impl StandardBrowserAutomationTools {
    pub fn new(browserHost: Arc<dyn BrowserAutomationHost>) -> Self {
        Self { browserHost }
    }

    #[allow(non_snake_case)]
    pub fn invoke(&self, tool: &AITool) -> ToolResult {
        let Some(chatId) = resolveChatId(tool) else {
            return toolError(
                tool,
                "chat_id is required for browser automation.".to_string(),
            );
        };
        let parametersJson = browserParametersJson(tool, &chatId);
        let request = BrowserAutomationRequest {
            requestId: uuid::Uuid::new_v4().to_string(),
            toolName: tool.name.clone(),
            chatId,
            parametersJson,
        };
        match self.browserHost.executeBrowserTool(request) {
            Ok(response) => ToolResult {
                toolName: tool.name.clone(),
                success: true,
                result: response.output,
                error: None,
            },
            Err(error) => toolError(tool, error.message),
        }
    }
}

impl ToolExecutor for BrowserAutomationToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        if resolveChatId(tool).is_none() {
            return invalid("chat_id is required for browser automation.");
        }

        let required = requiredParameters(tool.name.as_str());
        for name in required {
            if parameterValue(tool, name).trim().is_empty() {
                return invalid(&format!("{name} is required."));
            }
        }

        match tool.name.as_str() {
            "browser_click" => {
                if parameterValue(tool, "ref").trim().is_empty()
                    && parameterValue(tool, "selector").trim().is_empty()
                {
                    return invalid("ref or selector is required.");
                }
            }
            "browser_wait_for" => {
                if parameterValue(tool, "time").trim().is_empty()
                    && parameterValue(tool, "text").trim().is_empty()
                    && parameterValue(tool, "textGone").trim().is_empty()
                {
                    return invalid("time, text, or textGone is required.");
                }
            }
            _ => {}
        }

        ToolValidationResult {
            valid: true,
            errorMessage: String::new(),
        }
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        vec![self.tools.invoke(tool)]
    }
}

#[allow(non_snake_case)]
fn resolveChatId(tool: &AITool) -> Option<String> {
    if let Some(value) = optionalParameterValue(tool, "chat_id")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return Some(value);
    }
    ToolExecutionManager::currentToolRuntimeContext().and_then(|context| context.callerChatId)
}

#[allow(non_snake_case)]
fn browserParametersJson(tool: &AITool, chatId: &str) -> String {
    let mut object = Map::new();
    for parameter in &tool.parameters {
        object.insert(
            parameter.name.clone(),
            Value::String(parameter.value.clone()),
        );
    }
    object.insert("chat_id".to_string(), Value::String(chatId.to_string()));
    Value::Object(object).to_string()
}

#[allow(non_snake_case)]
fn requiredParameters(toolName: &str) -> &'static [&'static str] {
    match toolName {
        "browser_navigate" => &["url"],
        "browser_drag" => &["startRef", "endRef"],
        "browser_evaluate" => &["function"],
        "browser_fill_form" => &["fields"],
        "browser_handle_dialog" => &["accept"],
        "browser_hover" => &["ref"],
        "browser_press_key" => &["key"],
        "browser_resize" => &["width", "height"],
        "browser_run_code" => &["code"],
        "browser_select_option" => &["ref", "values"],
        "browser_type" => &["ref", "text"],
        "browser_tabs" => &["action"],
        _ => &[],
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

fn invalid(message: &str) -> ToolValidationResult {
    ToolValidationResult {
        valid: false,
        errorMessage: message.to_string(),
    }
}

#[allow(non_snake_case)]
fn toolError(tool: &AITool, message: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result: String::new(),
        error: Some(message),
    }
}
