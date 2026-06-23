use serde::{Deserialize, Serialize};

use crate::ConversationMarkupManager::ToolResult;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AITool {
    pub name: String,
    pub parameters: Vec<ToolParameter>,
}

pub trait ToolExecutor: Send {
    #[allow(non_snake_case)]
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult;

    #[allow(non_snake_case)]
    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct ToolValidationResult {
    pub valid: bool,
    pub errorMessage: String,
}
