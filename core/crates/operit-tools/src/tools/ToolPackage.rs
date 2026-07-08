use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{
    AITool, ToolAccessSpec, ToolBoundary, ToolEffect, ToolExecutor, ToolValidationResult,
};
use operit_tools::tools::javascript::jsBridgeSupport;
use operit_tools::tools::packTool::PackageManager::PackageManager;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::tools::ToolResultDataClasses::stringResultData;

#[derive(Clone, Debug, Default, Serialize)]
/// Localized text value accepted from package manifests.
pub struct LocalizedText {
    pub values: HashMap<String, String>,
}

impl<'de> Deserialize<'de> for LocalizedText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if let Some(text) = value.as_str() {
            return Ok(LocalizedText {
                values: HashMap::from([("default".to_string(), text.to_string())]),
            });
        }
        if let Some(object) = value.as_object() {
            let source = object
                .get("values")
                .and_then(Value::as_object)
                .unwrap_or(object);
            let values = source
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|text| (key.clone(), text.to_string()))
                })
                .collect::<HashMap<_, _>>();
            return Ok(LocalizedText { values });
        }
        Ok(LocalizedText::default())
    }
}

impl LocalizedText {
    /// Resolves the localized text for the requested language mode.
    pub fn resolve(&self, useEnglish: bool) -> String {
        let primary = if useEnglish { "en" } else { "zh" };
        self.values
            .get(primary)
            .or_else(|| self.values.get("default"))
            .or_else(|| self.values.values().next())
            .cloned()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::LocalizedText;

    #[test]
    fn parses_operit1_locale_object() {
        let text: LocalizedText =
            serde_json::from_str(r#"{"zh":"思考引导","en":"Thinking Guidance"}"#).unwrap();
        assert_eq!(text.resolve(false), "思考引导");
        assert_eq!(text.resolve(true), "Thinking Guidance");
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
/// Environment variable declared by a tool package manifest.
pub struct EnvVar {
    pub name: String,
    pub description: LocalizedText,
    pub required: bool,
    pub default_value: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
/// Parsed package manifest containing metadata, tools, state, and environment needs.
pub struct ToolPackage {
    pub name: String,
    pub description: LocalizedText,
    pub tools: Vec<PackageTool>,
    pub states: Vec<ToolPackageState>,
    pub env: Vec<EnvVar>,
    pub is_built_in: bool,
    pub enabled_by_default: bool,
    pub display_name: LocalizedText,
    pub category: String,
    pub author: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
/// Conditional package state that can adjust the package's available tools.
pub struct ToolPackageState {
    pub id: String,
    pub condition: String,
    pub inherit_tools: bool,
    pub exclude_tools: Vec<String>,
    pub tools: Vec<PackageTool>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
/// Executable tool declaration inside a package manifest.
pub struct PackageTool {
    pub name: String,
    pub description: LocalizedText,
    pub parameters: Vec<PackageToolParameter>,
    pub script: String,
    pub advice: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
/// Parameter declaration for a packaged tool.
pub struct PackageToolParameter {
    pub name: String,
    pub description: LocalizedText,
    pub parameter_type: String,
    pub required: bool,
}

/// Serializer marker retained for compatibility with package schema code.
pub struct LocalizedTextSerializer;

/// Serializer marker for fields accepted as either a string or a string list.
pub struct StringOrStringListSerializer;

#[derive(Clone)]
/// Tool executor that dispatches package tool calls into the JavaScript runtime.
pub struct PackageToolExecutor {
    toolPackage: ToolPackage,
    packageManager: Arc<Mutex<PackageManager>>,
    toolHandler: AIToolHandler,
}

impl PackageToolExecutor {
    /// Creates an executor bound to one package and the shared package runtime.
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
    /// Invokes a package tool by resolving its package-qualified tool name.
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

        jsBridgeSupport()
            .executePackageTool(
                self.packageManager.clone(),
                self.toolHandler.clone(),
                &packageTool.script,
                tool,
            )
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

    fn accessSpec(&self, _tool: &AITool) -> Result<ToolAccessSpec, String> {
        Ok(ToolAccessSpec {
            effect: ToolEffect::READ,
            boundary: ToolBoundary::None,
        })
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

        jsBridgeSupport().executePackageTool(
            self.packageManager.clone(),
            self.toolHandler.clone(),
            &packageTool.script,
            tool,
        )
    }
}
