use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::AITool;

/// Decides whether a tool call may continue into permission checks and execution.
pub enum AIToolHookDecision {
    /// Allows the tool call to continue.
    Allow,
    /// Stops the tool call and supplies the user-visible error reason.
    Block(String),
}

pub trait AIToolHook: Send + Sync {
    /// Returns the stable identifier of this hook.
    fn id(&self) -> &str;

    /// Observes a tool call request before interception.
    fn onToolCallRequested(&self, _tool: &AITool) {}

    /// Decides whether a requested tool call may continue.
    fn onToolCallIntercept(&self, _tool: &AITool) -> AIToolHookDecision {
        AIToolHookDecision::Allow
    }

    /// Observes the completed permission check.
    fn onToolPermissionChecked(&self, _tool: &AITool, _granted: bool, _reason: Option<&str>) {}

    /// Observes the start of actual tool execution.
    fn onToolExecutionStarted(&self, _tool: &AITool) {}

    /// Observes a produced tool result.
    fn onToolExecutionResult(&self, _tool: &AITool, _result: &ToolResult) {}

    /// Observes an execution error.
    fn onToolExecutionError(&self, _tool: &AITool, _message: &str) {}

    /// Observes the end of the tool lifecycle.
    fn onToolExecutionFinished(&self, _tool: &AITool) {}
}
