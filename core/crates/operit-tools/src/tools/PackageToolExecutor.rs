use std::sync::{Arc, Mutex};

use operit_plugin_sdk::javascript::{JsPackageToolCallRequest, JsPackageToolCallResult};
use operit_plugin_sdk::package::ToolPackage;

use crate::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use crate::tools::AIToolHandler::AIToolHandler;
use crate::tools::ToolJsRuntime::{JsPackageExecutor, PackageManagerJsRuntime};
use crate::tools::ToolResultDataClasses::stringResultData;
use crate::ConversationMarkupManager::ToolResult;
use crate::ToolExecutionManager::{
    AITool, ToolAccessSpec, ToolBoundary, ToolEffect, ToolExecutor, ToolValidationResult,
};

#[derive(Clone)]
/// Executes one SDK package through the host JavaScript and tool runtimes.
pub struct PackageToolExecutor {
    toolPackage: ToolPackage,
    packageExecutor: Arc<dyn JsPackageExecutor>,
}

impl PackageToolExecutor {
    /// Creates an executor bound to one package and the shared package runtime.
    pub fn new(
        toolPackage: ToolPackage,
        packageManager: Arc<Mutex<RuntimePackageManager>>,
        toolHandler: AIToolHandler,
    ) -> Self {
        let packageRuntime = Arc::new(PackageManagerJsRuntime::new(
            packageManager,
            toolHandler.clone(),
        ));
        let runtimeDependencies = toolHandler.runtimeDependencies();
        Self {
            toolPackage,
            packageExecutor: runtimeDependencies
                .js_execution_provider()
                .create_package_executor(packageRuntime, Arc::new(toolHandler)),
        }
    }

    /// Invokes a package tool by its package-qualified tool name.
    #[allow(non_snake_case)]
    pub fn invoke(&self, tool: &AITool) -> ToolResult {
        let parts = tool.name.split(':').collect::<Vec<_>>();
        if parts.len() != 2 {
            return failedToolResult(
                tool,
                "Invalid package tool format. Expected 'packageName:toolName'".to_string(),
            );
        }

        let packageName = parts[0];
        let toolName = parts[1];
        if packageName != self.toolPackage.name {
            return failedToolResult(
                tool,
                format!(
                    "Package mismatch: expected {}, got {}",
                    self.toolPackage.name, packageName
                ),
            );
        }

        let Some(packageTool) = self
            .toolPackage
            .tools
            .iter()
            .find(|item| item.name == toolName)
        else {
            return failedToolResult(
                tool,
                format!(
                    "Tool '{}' not found in package '{}'",
                    toolName, self.toolPackage.name
                ),
            );
        };

        let request = packageToolCallRequest(tool);
        let result = self
            .packageExecutor
            .execute_package_tool(&packageTool.script, &request);
        packageToolResult(result)
    }
}

impl ToolExecutor for PackageToolExecutor {
    /// Validates the package-qualified name and required parameters.
    #[allow(non_snake_case)]
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        let parts = tool.name.split(':').collect::<Vec<_>>();
        if parts.len() != 2 {
            return ToolValidationResult {
                valid: false,
                errorMessage: "Invalid package tool format. Expected 'packageName:toolName'"
                    .to_string(),
            };
        }

        let packageName = parts[0];
        let toolName = parts[1];
        if packageName != self.toolPackage.name {
            return ToolValidationResult {
                valid: false,
                errorMessage: format!(
                    "Package mismatch: expected {}, got {}",
                    self.toolPackage.name, packageName
                ),
            };
        }

        let Some(packageTool) = self
            .toolPackage
            .tools
            .iter()
            .find(|item| item.name == toolName)
        else {
            return ToolValidationResult {
                valid: false,
                errorMessage: format!(
                    "Tool '{}' not found in package '{}'",
                    toolName, self.toolPackage.name
                ),
            };
        };

        let missingParams = packageTool
            .parameters
            .iter()
            .filter(|parameter| parameter.required)
            .map(|parameter| parameter.name.clone())
            .filter(|paramName| tool.parameters.iter().all(|item| item.name != *paramName))
            .collect::<Vec<_>>();

        if !missingParams.is_empty() {
            return ToolValidationResult {
                valid: false,
                errorMessage: format!("Missing required parameters: {}", missingParams.join(", ")),
            };
        }

        ToolValidationResult {
            valid: true,
            errorMessage: String::new(),
        }
    }

    /// Declares the access boundary for package tools.
    #[allow(non_snake_case)]
    fn accessSpec(&self, _tool: &AITool) -> Result<ToolAccessSpec, String> {
        Ok(ToolAccessSpec {
            effect: ToolEffect::READ,
            boundary: ToolBoundary::None,
        })
    }

    /// Invokes a package tool and returns every emitted result.
    #[allow(non_snake_case)]
    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        let toolName = tool.name.split(':').last().unwrap_or_default();
        let Some(packageTool) = self
            .toolPackage
            .tools
            .iter()
            .find(|item| item.name.ends_with(toolName))
        else {
            return vec![failedToolResult(
                tool,
                "Tool not found in package for streaming".to_string(),
            )];
        };

        let request = packageToolCallRequest(tool);
        let result = self
            .packageExecutor
            .execute_package_tool(&packageTool.script, &request);
        vec![packageToolResult(result)]
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
    ToolResult {
        toolName: result.tool_name,
        success: result.success,
        result: stringResultData(result.result),
        error: result.error,
    }
}

/// Builds a failed tool result with an empty string payload.
#[allow(non_snake_case)]
fn failedToolResult(tool: &AITool, error: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result: stringResultData(""),
        error: Some(error),
    }
}
