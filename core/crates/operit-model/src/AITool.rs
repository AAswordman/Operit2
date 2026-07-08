use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AITool {
    pub name: String,
    pub parameters: Vec<ToolParameter>,
    pub description: String,
}

impl AITool {
    pub fn new(name: String) -> Self {
        Self {
            name,
            parameters: Vec::new(),
            description: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntRange {
    pub start: i32,
    pub endInclusive: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub tool: AITool,
    pub rawText: String,
    pub responseLocation: IntRange,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    pub toolName: String,
    pub success: bool,
    pub result: ToolResultData,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolResultData {
    pub value: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolValidationResult {
    pub valid: bool,
    pub errorMessage: String,
}

impl ToolValidationResult {
    pub fn new(valid: bool) -> Self {
        Self {
            valid,
            errorMessage: String::new(),
        }
    }
}
