use std::sync::{Arc, Mutex};

use operit_plugin_sdk::javascript::{JsPackageToolCallRequest, JsPackageToolCallResult};

use crate::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use crate::tools::ToolJsRuntime::PackageManagerJsRuntime;
use crate::tools::ToolResultDataClasses::{stringResultData, JsonResultData, ToolResultData};
use crate::ConversationMarkupManager::ToolResult;
use crate::ToolExecutionManager::{
    AITool, AsyncToolExecutor, ToolAccessSpec, ToolBoundary, ToolEffect, ToolInvocationFuture,
    ToolValidationResult,
};

#[derive(Clone)]
/// Executes one SDK package through the host JavaScript and tool runtimes.
pub struct PackageToolExecutor {
    packageName: String,
    packageManager: Arc<Mutex<RuntimePackageManager>>,
}

impl PackageToolExecutor {
    /// Creates an executor bound to one package and the shared package runtime.
    pub fn new(
        packageName: String,
        packageManager: Arc<Mutex<RuntimePackageManager>>,
    ) -> Self {
        Self { packageName, packageManager }
    }

}

impl AsyncToolExecutor for PackageToolExecutor {
    /// Validates the generic namespace boundary without selecting from stale package metadata.
    #[allow(non_snake_case)]
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        let parts = tool.name.split(':').collect::<Vec<_>>();
        if parts.len() != 2 || parts.iter().any(|part| part.trim().is_empty()) {
            return ToolValidationResult {
                valid: false,
                errorMessage: "Invalid package tool format. Expected 'packageName:toolName'"
                    .to_string(),
            };
        }
        if parts[0] != self.packageName {
            return ToolValidationResult {
                valid: false,
                errorMessage: format!(
                    "Package mismatch: expected {}, got {}",
                    self.packageName, parts[0]
                ),
            };
        }
        let mut names = std::collections::HashSet::new();
        for parameter in &tool.parameters {
            if !names.insert(&parameter.name) {
                return ToolValidationResult {
                    valid: false,
                    errorMessage: format!("Duplicate package tool parameter: {}", parameter.name),
                };
            }
        }
        ToolValidationResult {
            valid: true,
            errorMessage: String::new(),
        }
    }

    /// Treats every outer package invocation as a write operation requiring approval in restricted modes.
    #[allow(non_snake_case)]
    fn accessSpec(&self, _tool: &AITool) -> Result<ToolAccessSpec, String> {
        Ok(ToolAccessSpec {
            effect: ToolEffect::WRITE,
            boundary: ToolBoundary::None,
        })
    }

    /// Uses the same exact SDK request dispatch for streaming and ordinary calls.
    #[allow(non_snake_case)]
    fn invokeAndStreamAsync<'a>(&'a self, tool: &'a AITool, context: &'a crate::ToolExecutionManager::ToolInvocationContext) -> ToolInvocationFuture<'a> {
        let host = context.host().clone();
        let runtime = Arc::new(PackageManagerJsRuntime::new(self.packageManager.clone(), host.clone()));
        let executor = host.runtimeDependencies().js_execution_provider()
            .create_package_executor(runtime, Arc::new(host));
        let onOutput = crate::PackageExecution::PackageExecution::outputCallback();
        Box::pin(async move {
            let request = packageToolCallRequest(tool);
            vec![packageToolResult(executor.execute_package_tool_streaming(&request, onOutput).await)]
        })
    }
}

/// Converts an Operit tool invocation into the stable SDK package request.
#[allow(non_snake_case)]
fn packageToolCallRequest(tool: &AITool) -> JsPackageToolCallRequest {
    JsPackageToolCallRequest {
        tool_name: tool.name.clone(),
        parameters: tool
            .parameters
            .iter()
            .map(|parameter| (parameter.name.clone(), parameter.value.clone()))
            .collect(),
    }
}

/// Converts the stable SDK package result into the Operit tool result.
#[allow(non_snake_case)]
fn packageToolResult(result: JsPackageToolCallResult) -> ToolResult {
    let data = if result.success {
        match serde_json::from_str(&result.result) {
            Ok(value) => ToolResultData::JsonResultData(JsonResultData { value }),
            Err(error) => return ToolResult { toolName: result.tool_name, success: false,
                result: stringResultData(""), error: Some(format!("Invalid package result JSON: {error}")) },
        }
    } else { stringResultData(result.result) };
    ToolResult {
        toolName: result.tool_name,
        success: result.success,
        result: data,
        error: result.error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Decodes exactly the transport JSON layer and preserves every business value and key.
    #[test]
    fn package_results_decode_objects_arrays_scalars_and_json_looking_strings_once() {
        for value in [json!({"__type":"BusinessType", "nested":[null,false]}), json!([1,2]),
            json!(null), json!(false), json!(42), json!("{\"looks\":\"like JSON\"}")] {
            let result = packageToolResult(JsPackageToolCallResult { tool_name: "example:read".into(),
                success: true, result: value.to_string(), error: None });
            assert!(result.success);
            let ToolResultData::JsonResultData(data) = result.result else { panic!("Expected structured JSON") };
            assert_eq!(data.value, value);
        }
    }

    /// Rejects a broken transport result explicitly instead of guessing an alternative representation.
    #[test]
    fn malformed_package_result_is_an_explicit_failure() {
        let result = packageToolResult(JsPackageToolCallResult { tool_name: "example:read".into(),
            success: true, result: "invalid JSON".into(), error: None });
        assert!(!result.success);
        assert!(result.error.unwrap().starts_with("Invalid package result JSON:"));
        assert_eq!(result.result.toString(), "");
    }

    /// Keeps the original package error without attempting to decode a failure as JSON.
    #[test]
    fn package_failure_preserves_its_error() {
        let result = packageToolResult(JsPackageToolCallResult { tool_name: "example:read".into(),
            success: false, result: String::new(), error: Some("setting does not exist".into()) });
        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some("setting does not exist"));
        assert_eq!(result.result.toString(), "");
    }
}

