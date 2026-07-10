use operit_host_api::FileSystemHost;

use operit_tools::tools::ToolResultDataClasses::stringResultData;
use operit_tools::ConversationMarkupManager::ToolResult;

/// Converts host path validation errors into tool results.
pub struct PathValidator;

impl PathValidator {
    /// Validates a host path parameter before a tool invokes file-system operations.
    #[allow(non_snake_case)]
    pub fn validateHostPath(
        host: &dyn FileSystemHost,
        path: &str,
        toolName: &str,
        paramName: &str,
    ) -> Option<ToolResult> {
        match host.validatePath(path, paramName) {
            Ok(()) => None,
            Err(error) => Some(ToolResult {
                toolName: toolName.to_string(),
                success: false,
                result: stringResultData(""),
                error: Some(error.message),
            }),
        }
    }
}
