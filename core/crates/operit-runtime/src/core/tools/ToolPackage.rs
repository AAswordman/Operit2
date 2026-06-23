use std::sync::{Arc, Mutex};

use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{AITool, ToolExecutor, ToolValidationResult};
use crate::core::tools::AIToolHandler::AIToolHandler;
use crate::core::tools::javascript::JsToolManager::JsToolManager;
use crate::core::tools::packTool::PackageManager::PackageManager;
use operit_tools::ToolResultDataClasses::stringResultData;

pub use operit_tools::ToolPackage::*;

pub struct LocalizedTextSerializer;

pub struct StringOrStringListSerializer;

#[derive(Clone)]
pub struct PackageToolExecutor {
    toolPackage: ToolPackage,
    packageManager: Arc<Mutex<PackageManager>>,
    toolHandler: AIToolHandler,
}

impl PackageToolExecutor {
    pub fn new(
        toolPackage: ToolPackage,
        packageManager: Arc<Mutex<PackageManager>>,
        toolHandler: AIToolHandler,
    ) -> Self {
        Self {
            toolPackage,
            packageManager,
            toolHandler,
        }
    }

    #[allow(non_snake_case)]
    pub fn invoke(&self, tool: &AITool) -> ToolResult {
        let parts = tool.name.split(':').collect::<Vec<_>>();
        if parts.len() != 2 {
            return ToolResult {
                toolName: tool.name.clone(),
                success: false,
                result: stringResultData(""),
                error: Some(
                    "Invalid package tool format. Expected 'packageName:toolName'".to_string(),
                ),
            };
        }

        let packageName = parts[0];
        let toolName = parts[1];
        if packageName != self.toolPackage.name {
            return ToolResult {
                toolName: tool.name.clone(),
                success: false,
                result: stringResultData(""),
                error: Some(format!(
                    "Package mismatch: expected {}, got {}",
                    self.toolPackage.name, packageName
                )),
            };
        }

        let Some(packageTool) = self
            .toolPackage
            .tools
            .iter()
            .find(|item| item.name == toolName)
        else {
            return ToolResult {
                toolName: tool.name.clone(),
                success: false,
                result: stringResultData(""),
                error: Some(format!(
                    "Tool '{}' not found in package '{}'",
                    toolName, self.toolPackage.name
                )),
            };
        };

        let jsToolManager =
            JsToolManager::getInstance(self.packageManager.clone(), self.toolHandler.clone());
        jsToolManager
            .executeScript(&packageTool.script, tool)
            .last()
            .cloned()
            .unwrap_or_else(|| ToolResult {
                toolName: tool.name.clone(),
                success: false,
                result: stringResultData(""),
                error: Some("The tool execution returned no results.".to_string()),
            })
    }
}

impl ToolExecutor for PackageToolExecutor {
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

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        let toolName = tool.name.split(':').last().unwrap_or_default();
        let Some(packageTool) = self
            .toolPackage
            .tools
            .iter()
            .find(|item| item.name.ends_with(toolName))
        else {
            return vec![ToolResult {
                toolName: tool.name.clone(),
                success: false,
                result: stringResultData(""),
                error: Some("Tool not found in package for streaming".to_string()),
            }];
        };

        let jsToolManager =
            JsToolManager::getInstance(self.packageManager.clone(), self.toolHandler.clone());
        jsToolManager.executeScript(&packageTool.script, tool)
    }
}
