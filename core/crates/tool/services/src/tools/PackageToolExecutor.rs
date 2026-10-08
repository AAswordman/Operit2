use std::sync::{Arc, Mutex};

use operit_plugin_sdk::javascript::{JsPackageToolCallRequest, JsPackageToolCallResult};
use operit_plugin_sdk::package::ToolPackage;

use crate::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use crate::tools::AIToolHandler::AIToolHandler;
use crate::tools::ToolJsRuntime::{JsPackageExecutor, PackageManagerJsRuntime};
use crate::tools::ToolResultDataClasses::stringResultData;
use crate::ConversationMarkupManager::ToolResult;
use crate::ToolExecutionManager::{
    AITool, AsyncToolExecutor, ToolAccessSpec, ToolBoundary, ToolEffect, ToolInvocationFuture,
    ToolValidationResult,
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

    /// Delegates one qualified request to the authoritative SDK selection and execution path.
    #[allow(non_snake_case)]
    pub async fn invoke(&self, tool: &AITool) -> ToolResult {
        let validation = self.validateParameters(tool);
        if !validation.valid {
            return failedToolResult(tool, validation.errorMessage);
        }
        let request = packageToolCallRequest(tool);
        let result = self.packageExecutor.execute_package_tool(&request).await;
        packageToolResult(result)
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
        if parts[0] != self.toolPackage.name {
            return ToolValidationResult {
                valid: false,
                errorMessage: format!(
                    "Package mismatch: expected {}, got {}",
                    self.toolPackage.name, parts[0]
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

    /// Declares the access boundary for package tools.
    #[allow(non_snake_case)]
    fn accessSpec(&self, _tool: &AITool) -> Result<ToolAccessSpec, String> {
        Ok(ToolAccessSpec {
            effect: ToolEffect::READ,
            boundary: ToolBoundary::None,
        })
    }

    /// Uses the same exact SDK request dispatch for streaming and ordinary calls.
    #[allow(non_snake_case)]
    fn invokeAndStreamAsync<'a>(&'a mut self, tool: &'a AITool) -> ToolInvocationFuture<'a> {
        Box::pin(async move { vec![self.invoke(tool).await] })
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
