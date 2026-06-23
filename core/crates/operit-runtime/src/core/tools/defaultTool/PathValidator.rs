use operit_host_api::FileSystemHost;

use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolResultDataClasses::stringResultData;

pub struct PathValidator;

impl PathValidator {
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
