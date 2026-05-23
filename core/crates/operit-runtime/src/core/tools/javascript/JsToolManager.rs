use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::AITool;
use crate::core::tools::AIToolHandler::AIToolHandler;
use crate::core::tools::javascript::JsEngine::JsEngine;
use crate::core::tools::javascript::JsExecutionResultProtocol::{
    extractJsExecutionFailure, JsExecutionFailure,
};
use crate::core::tools::packTool::PackageManager::PackageManager;

#[derive(Clone)]
pub struct JsToolManager {
    packageManager: Arc<Mutex<PackageManager>>,
    toolHandler: AIToolHandler,
}

#[derive(Debug)]
struct ToolParameterConversionException {
    message: String,
}

impl JsToolManager {
    #[allow(non_snake_case)]
    pub fn getInstance(packageManager: Arc<Mutex<PackageManager>>, toolHandler: AIToolHandler) -> Self {
        Self {
            packageManager,
            toolHandler,
        }
    }

    #[allow(non_snake_case)]
    fn parseDotCall(toolName: &str) -> Option<(String, String)> {
        let separatorIndex = toolName.rfind('.')?;
        if separatorIndex == 0 || separatorIndex >= toolName.len() - 1 {
            return None;
        }
        Some((
            toolName[..separatorIndex].to_string(),
            toolName[separatorIndex + 1..].to_string(),
        ))
    }

    #[allow(non_snake_case)]
    fn parsePackageToolName(toolName: &str) -> Option<(String, String)> {
        let separatorIndex = toolName.find(':')?;
        if separatorIndex == 0 || separatorIndex >= toolName.len() - 1 {
            return None;
        }
        Some((
            toolName[..separatorIndex].to_string(),
            toolName[separatorIndex + 1..].to_string(),
        ))
    }

    #[allow(non_snake_case)]
    fn buildRuntimeParams(&self, packageName: &str, params: BTreeMap<String, Value>) -> BTreeMap<String, Value> {
        let mut runtimeParams = params;
        if let Some(stateId) = self
            .packageManager
            .lock()
            .expect("package manager mutex poisoned")
            .getActivePackageStateId(packageName)
        {
            runtimeParams.insert("__operit_package_state".to_string(), Value::String(stateId));
        }

        for key in [
            "__operit_package_caller_name",
            "__operit_package_chat_id",
            "__operit_package_caller_card_id",
        ] {
            let value = runtimeParams
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            match value {
                Some(value) => {
                    runtimeParams.insert(key.to_string(), Value::String(value));
                }
                None => {
                    runtimeParams.remove(key);
                }
            }
        }

        runtimeParams.insert(
            "__operit_package_name".to_string(),
            Value::String(packageName.to_string()),
        );
        runtimeParams.insert(
            "__operit_toolpkg_runtime_kind".to_string(),
            Value::String("sandbox".to_string()),
        );
        runtimeParams
    }

    #[allow(non_snake_case)]
    fn convertToolParameters(
        &self,
        tool: &AITool,
        packageName: &str,
        functionName: &str,
    ) -> Result<BTreeMap<String, Value>, ToolParameterConversionException> {
        let packageTools = self
            .packageManager
            .lock()
            .expect("package manager mutex poisoned")
            .getPackageTools(packageName);
        let toolDefinition = packageTools
            .as_ref()
            .and_then(|package| package.tools.iter().find(|item| item.name == functionName));

        let missingRequiredParameters = toolDefinition
            .map(|definition| {
                definition
                    .parameters
                    .iter()
                    .filter(|parameter| {
                        parameter.required
                            && !tool.parameters.iter().any(|item| item.name == parameter.name)
                    })
                    .map(|parameter| parameter.name.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !missingRequiredParameters.is_empty() {
            return Err(ToolParameterConversionException {
                message: format!(
                    "Missing required parameters: {}",
                    missingRequiredParameters.join(", ")
                ),
            });
        }

        let mut converted = BTreeMap::new();
        for parameter in &tool.parameters {
            let parameterType = toolDefinition
                .and_then(|definition| {
                    definition
                        .parameters
                        .iter()
                        .find(|item| item.name == parameter.name)
                })
                .map(|item| item.parameter_type.to_ascii_lowercase())
                .unwrap_or_else(|| "string".to_string());
            let value = self.convertToolParameterValue(
                &tool.name,
                &parameter.name,
                &parameter.value,
                &parameterType,
            )?;
            converted.insert(parameter.name.clone(), value);
        }

        Ok(self.buildRuntimeParams(packageName, converted))
    }

    #[allow(non_snake_case)]
    fn convertToolParameterValue(
        &self,
        toolName: &str,
        parameterName: &str,
        rawValue: &str,
        parameterType: &str,
    ) -> Result<Value, ToolParameterConversionException> {
        let normalizedValue = rawValue.trim();
        match parameterType {
            "number" => normalizedValue
                .parse::<f64>()
                .map(|value| serde_json::json!(value))
                .map_err(|_| self.invalidParameterType(toolName, parameterName, parameterType)),
            "integer" => normalizedValue
                .parse::<i64>()
                .map(|value| serde_json::json!(value))
                .map_err(|_| self.invalidParameterType(toolName, parameterName, parameterType)),
            "boolean" => match normalizedValue.to_ascii_lowercase().as_str() {
                "true" | "1" => Ok(Value::Bool(true)),
                "false" | "0" => Ok(Value::Bool(false)),
                _ => Err(self.invalidParameterType(toolName, parameterName, parameterType)),
            },
            "array" | "object" => serde_json::from_str::<Value>(rawValue)
                .map_err(|_| self.invalidParameterType(toolName, parameterName, parameterType)),
            _ => Ok(Value::String(rawValue.to_string())),
        }
    }

    #[allow(non_snake_case)]
    fn invalidParameterType(
        &self,
        toolName: &str,
        parameterName: &str,
        expectedType: &str,
    ) -> ToolParameterConversionException {
        ToolParameterConversionException {
            message: format!(
                "Invalid parameter '{}' for tool '{}': expected {}",
                parameterName, toolName, expectedType
            ),
        }
    }

    fn success(toolName: &str, value: Option<String>) -> ToolResult {
        ToolResult {
            toolName: toolName.to_string(),
            success: true,
            result: value.unwrap_or_else(|| "null".to_string()),
            error: None,
        }
    }

    fn failure(toolName: &str, message: String) -> ToolResult {
        ToolResult {
            toolName: toolName.to_string(),
            success: false,
            result: String::new(),
            error: Some(message),
        }
    }

    #[allow(non_snake_case)]
    fn failureFromJs(toolName: &str, failure: JsExecutionFailure) -> ToolResult {
        ToolResult {
            toolName: toolName.to_string(),
            success: false,
            result: failure.dataText,
            error: Some(failure.message),
        }
    }

    #[allow(non_snake_case)]
    pub fn executeScriptByName(&self, toolName: &str, params: BTreeMap<String, String>) -> String {
        let Some((packageName, functionName)) = Self::parseDotCall(toolName) else {
            return format!("Invalid tool name format: {toolName}. Expected format: packageName.functionName");
        };
        let script = self
            .packageManager
            .lock()
            .expect("package manager mutex poisoned")
            .getPackageScript(&packageName);
        let Some(script) = script else {
            return format!("Package not found: {packageName}");
        };
        let params = params
            .into_iter()
            .map(|(key, value)| (key, Value::String(value)))
            .collect::<BTreeMap<_, _>>();
        let runtimeParams = self.buildRuntimeParams(&packageName, params);
        let engine = JsEngine::new(self.toolHandler.clone());
        engine
            .executeScriptFunction(&script, &functionName, &runtimeParams)
            .unwrap_or_else(|| "null".to_string())
    }

    #[allow(non_snake_case)]
    pub fn executeScript(&self, script: &str, tool: &AITool) -> Vec<ToolResult> {
        let Some((packageName, functionName)) = Self::parsePackageToolName(&tool.name) else {
            return vec![Self::failure(
                &tool.name,
                "Invalid tool name format. Expected 'packageName:toolName'".to_string(),
            )];
        };

        let runtimeParams = match self.convertToolParameters(tool, &packageName, &functionName) {
            Ok(value) => value,
            Err(error) => return vec![Self::failure(&tool.name, error.message)],
        };

        let engine = JsEngine::new(self.toolHandler.clone());
        let result = engine.executeScriptFunction(script, &functionName, &runtimeParams);
        if let Some(failure) = extractJsExecutionFailure(result.as_deref()) {
            vec![Self::failureFromJs(&tool.name, failure)]
        } else {
            vec![Self::success(&tool.name, result)]
        }
    }

    pub fn destroy(&self) {}
}
