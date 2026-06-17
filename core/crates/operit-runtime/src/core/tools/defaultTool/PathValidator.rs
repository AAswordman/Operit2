use operit_host_api::FileSystemHost;

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::core::tools::ToolResultDataClasses::stringResultData;

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
