use std::sync::Arc;

use operit_host_api::{BrowserAutomationHost, BrowserAutomationRequest};
use serde_json::{Map, Value};

use operit_tools::tools::ToolResultDataClasses::stringResultData;
use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{
    AITool, ToolAccessSpec, ToolBoundary, ToolEffect, ToolExecutor, ToolValidationResult,
};

#[derive(Clone)]
pub struct StandardBrowserAutomationTools {
    browserAutomationHost: Arc<dyn BrowserAutomationHost>,
}

pub struct BrowserAutomationToolExecutor {
    pub tools: StandardBrowserAutomationTools,
}

impl StandardBrowserAutomationTools {
    /// Creates browser tools bound to the configured browser automation host.
    pub fn new(browserAutomationHost: Arc<dyn BrowserAutomationHost>) -> Self {
        Self {
            browserAutomationHost,
        }
    }

    #[allow(non_snake_case)]
    /// Forwards a browser invocation without interpreting its operation parameters.
    pub fn invoke(&self, tool: &AITool) -> ToolResult {
        let parametersJson = browserParametersJson(tool);
        let request = BrowserAutomationRequest {
            requestId: uuid::Uuid::new_v4().to_string(),
            toolName: tool.name.clone(),
            parametersJson,
        };
        match self.browserAutomationHost.executeBrowserTool(request) {
            Ok(response) => ToolResult {
                toolName: tool.name.clone(),
                success: true,
                result: stringResultData(response.output),
                error: None,
            },
            Err(error) => toolError(tool, error.message),
        }
    }
}

impl ToolExecutor for BrowserAutomationToolExecutor {
    /// Validates required fields and the exact upload-path array contract.
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        let required = requiredParameters(tool.name.as_str());
        for name in required {
            if parameterValue(tool, name).trim().is_empty() {
                return invalid(&format!("{name} is required."));
            }
        }

        match tool.name.as_str() {
            "browser_file_upload" => {
                if let Err(error) = browserUploadPaths(tool) {
                    return invalid(&error);
                }
            }
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

    fn accessSpec(&self, tool: &AITool) -> Result<ToolAccessSpec, String> {
        let effect = browserToolEffect(tool);
        Ok(ToolAccessSpec {
            effect,
            boundary: ToolBoundary::None,
        })
    }

    /// Executes an independent request through the owning resource services.
    fn invokeAndStream(&self, tool: &AITool) -> Vec<ToolResult> {
        vec![self.tools.invoke(tool)]
    }
}

fn browserToolEffect(tool: &AITool) -> ToolEffect {
    match tool.name.as_str() {
        "browser_console_messages"
        | "browser_network_requests"
        | "browser_snapshot"
        | "browser_take_screenshot"
        | "browser_wait_for" => ToolEffect::READ,
        "browser_tabs" => match parameterValue(tool, "action").trim() {
            "list" => ToolEffect::READ,
            _ => ToolEffect::WRITE,
        },
        _ => ToolEffect::WRITE,
    }
}

#[allow(non_snake_case)]
/// Serializes every browser operation's parameters without reading or adding data.
fn browserParametersJson(tool: &AITool) -> String {
    let object = tool
        .parameters
        .iter()
        .map(|parameter| {
            (
                parameter.name.clone(),
                Value::String(parameter.value.clone()),
            )
        })
        .collect::<Map<String, Value>>();
    Value::Object(object).to_string()
}

/// Parses upload paths without conflating cancellation with an empty selection.
fn browserUploadPaths(tool: &AITool) -> Result<Option<Vec<String>>, String> {
    let Some(raw) = optionalParameterValue(tool, "paths") else {
        return Ok(None);
    };
    let paths: Vec<String> = serde_json::from_str(&raw)
        .map_err(|error| format!("paths must be a JSON array of file paths: {error}"))?;
    if paths.iter().any(|path| path.trim().is_empty()) {
        return Err("Browser upload paths must not be empty".to_string());
    }
    Ok(Some(paths))
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
        result: stringResultData(""),
        error: Some(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolExecutionManager::ToolParameter;

    /// Constructs an upload invocation with an explicitly optional paths parameter.
    fn upload_tool(paths: Option<&str>) -> AITool {
        AITool {
            name: "browser_file_upload".to_string(),
            parameters: paths
                .into_iter()
                .map(|value| ToolParameter {
                    name: "paths".to_string(),
                    value: value.to_string(),
                })
                .collect(),
        }
    }

    /// Serializes identical parameters identically regardless of the browser operation.
    #[test]
    fn browser_parameters_do_not_specialize_by_tool_name() {
        let mut tool = upload_tool(Some(r#"["/not-opened-by-the-serializer.txt"]"#));
        let expected = serde_json::json!({ "paths": r#"["/not-opened-by-the-serializer.txt"]"# });
        for name in ["browser_file_upload", "browser_click", "browser_snapshot"] {
            tool.name = name.to_string();
            let serialized: Value = serde_json::from_str(&browserParametersJson(&tool)).unwrap();
            assert_eq!(serialized, expected);
        }
    }

    /// Leaves cancellation and clearing in the public paths protocol for the host to interpret.
    #[test]
    fn browser_parameters_do_not_inject_upload_payloads() {
        let cancellation: Value =
            serde_json::from_str(&browserParametersJson(&upload_tool(None))).unwrap();
        let clearing: Value =
            serde_json::from_str(&browserParametersJson(&upload_tool(Some("[]")))).unwrap();
        assert_eq!(cancellation, serde_json::json!({}));
        assert_eq!(clearing, serde_json::json!({ "paths": "[]" }));
    }

    /// Preserves optional upload intent across the generated plugin host boundary.
    #[test]
    fn sdk_upload_options_preserve_absence() {
        use operit_plugin_sdk::js_sdk::network::NetHostBrowserFileUploadOptions;
        let cancellation = NetHostBrowserFileUploadOptions { paths: None };
        let clear_selection = NetHostBrowserFileUploadOptions {
            paths: Some(Vec::new()),
        };
        assert_eq!(
            serde_json::to_value(cancellation).unwrap(),
            serde_json::json!({})
        );
        assert_eq!(
            serde_json::to_value(clear_selection).unwrap(),
            serde_json::json!({ "paths": [] })
        );
    }

    /// Keeps cancellation distinct from an explicit empty selection.
    #[test]
    fn upload_paths_preserve_selection_intent() {
        assert_eq!(browserUploadPaths(&upload_tool(None)).unwrap(), None);
        assert_eq!(
            browserUploadPaths(&upload_tool(Some("[]"))).unwrap(),
            Some(Vec::new())
        );
    }

    /// Rejects malformed arrays and empty paths rather than interpreting them as cancellation.
    #[test]
    fn upload_paths_reject_invalid_parameters() {
        for raw in ["", "null", "{}", "42", r#"[1]"#, r#"[""]"#, r#"["  "]"#] {
            assert!(
                browserUploadPaths(&upload_tool(Some(raw))).is_err(),
                "{raw}"
            );
        }
    }

    /// Preserves path spelling, spaces, Unicode, and caller-defined file ordering.
    #[test]
    fn upload_paths_preserve_file_identity() {
        let paths = vec![
            "/tmp/中文 file.txt".to_string(),
            r"D:\uploads\second.bin".to_string(),
        ];
        let json = serde_json::to_string(&paths).unwrap();
        assert_eq!(
            browserUploadPaths(&upload_tool(Some(&json))).unwrap(),
            Some(paths)
        );
    }
}
