use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::AITool;
use crate::core::tools::AIToolHandler::ExternalToolInvocationBridge;
use operit_tools::ToolResultDataClasses::stringResultData;

pub struct FunctionExternalToolBridge<F>
where
    F: Fn(&AITool) -> Vec<ToolResult> + Send + Sync,
{
    handler: F,
}

impl<F> FunctionExternalToolBridge<F>
where
    F: Fn(&AITool) -> Vec<ToolResult> + Send + Sync,
{
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

impl<F> ExternalToolInvocationBridge for FunctionExternalToolBridge<F>
where
    F: Fn(&AITool) -> Vec<ToolResult> + Send + Sync,
{
    #[allow(non_snake_case)]
    fn invokeTool(&self, tool: &AITool) -> Vec<ToolResult> {
        (self.handler)(tool)
    }
}

pub fn stringToolResult(toolName: &str, value: impl Into<String>) -> ToolResult {
    ToolResult {
        toolName: toolName.to_string(),
        success: true,
        result: stringResultData(value),
        error: None,
    }
}

pub fn errorToolResult(toolName: &str, message: impl Into<String>) -> ToolResult {
    ToolResult {
        toolName: toolName.to_string(),
        success: false,
        result: stringResultData(""),
        error: Some(message.into()),
    }
}
