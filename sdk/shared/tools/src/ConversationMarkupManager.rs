use serde::{Deserialize, Serialize};

use crate::ToolResultDataClasses::ToolResultData;

#[derive(Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolResult {
    pub toolName: String,
    pub success: bool,
    pub result: ToolResultData,
    pub error: Option<String>,
}

pub struct ConversationMarkupManager;

impl ConversationMarkupManager {
    #[allow(non_snake_case)]
    pub fn formatToolResultForMessage(result: &ToolResult) -> String {
        if result.success {
            format!(
                r#"<tool_result name="{}" status="success"><content>{}</content></tool_result>"#,
                result.toolName,
                result.result.toString()
            )
        } else {
            let message = result.error.clone().unwrap_or_default();
            format!(
                r#"<tool_result name="{}" status="error"><content><error>{}</error></content></tool_result>"#,
                result.toolName, message
            )
        }
    }
}
