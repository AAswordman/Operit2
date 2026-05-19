use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::AITool;

pub trait AIToolHook: Send + Sync {
    fn id(&self) -> &str;

    fn onToolCallRequested(&self, _tool: &AITool) {}

    fn onToolPermissionChecked(&self, _tool: &AITool, _granted: bool, _reason: Option<&str>) {}

    fn onToolExecutionStarted(&self, _tool: &AITool) {}

    fn onToolExecutionResult(&self, _tool: &AITool, _result: &ToolResult) {}

    fn onToolExecutionError(&self, _tool: &AITool, _message: &str) {}

    fn onToolExecutionFinished(&self, _tool: &AITool) {}
}
