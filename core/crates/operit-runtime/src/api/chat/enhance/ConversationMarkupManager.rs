use crate::core::tools::ToolResultDataClasses::ToolResultData;
use crate::util::ChatMarkupRegex::ChatMarkupRegex;
use serde::{Deserialize, Serialize};

const TOOL_RESULT_TRUNCATION_SUFFIX: &str = "\n[工具结果过长，已截断]";
const MAX_FINAL_TOOL_RESULT_MESSAGE_CHARS: usize = 64 * 1024;
pub const ENHANCED_PURE_THINKING_ONLY_WARNING: &str = "警告：请输出正文内容，禁止仅输出思考内容。";
pub const ENHANCED_TRUNCATED_TOOL_CALL_WARNING: &str =
    "警告：检测到工具调用输出被截断。本轮所有工具调用均已作废且不会执行。请尝试减少单次输出、拆分任务，或更换更合适的模型/供应商后重试。";

#[derive(Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub toolName: String,
    pub success: bool,
    pub result: ToolResultData,
    pub error: Option<String>,
}

pub struct ConversationMarkupManager;

impl ConversationMarkupManager {
    pub fn createToolErrorStatus(toolName: &str, errorMessage: &str) -> String {
        Self::createToolResultXml(
            toolName,
            "error",
            &format!("<content><error>{}</error></content>", errorMessage),
        )
    }

    pub fn createWarningStatus(warningMessage: &str) -> String {
        format!(r#"<status type="warning">{}</status>"#, warningMessage)
    }

    pub fn formatToolResultForMessage(result: &ToolResult) -> String {
        if result.success {
            Self::createBoundedToolResultXml(
                &result.toolName,
                "success",
                &result.result.toString(),
                |payload| format!("<content>{payload}</content>"),
            )
        } else {
            let message = result.error.clone().unwrap_or_default().trim().to_string();
            let detail = result.result.toString().trim().to_string();
            let errorPayload = if !message.is_empty() && !detail.is_empty() {
                format!("{message}\n\n{detail}")
            } else if !message.is_empty() {
                message
            } else {
                detail
            };
            Self::createBoundedToolResultXml(&result.toolName, "error", &errorPayload, |payload| {
                format!("<content><error>{payload}</error></content>")
            })
        }
    }

    pub fn buildBoundedToolResultMessage(results: &[ToolResult]) -> String {
        if results.is_empty() {
            return String::new();
        }
        let separator = "\n";
        let mut builder = String::new();
        for result in results {
            let formatted = Self::formatToolResultForMessage(result);
            let additionalLength = if builder.is_empty() {
                formatted.len()
            } else {
                separator.len() + formatted.len()
            };
            if builder.len() + additionalLength > MAX_FINAL_TOOL_RESULT_MESSAGE_CHARS {
                break;
            }
            if !builder.is_empty() {
                builder.push_str(separator);
            }
            builder.push_str(&formatted);
        }
        builder
    }

    pub fn createToolNotAvailableError(toolName: &str, details: Option<&str>) -> String {
        let owned;
        let errorMessage = match details {
            Some(value) => value,
            None => {
                owned = format!("The tool `{}` is not available.", toolName);
                &owned
            }
        };
        Self::createToolErrorStatus(toolName, errorMessage)
    }

    fn createToolResultXml(toolName: &str, status: &str, content: &str) -> String {
        let tagName = ChatMarkupRegex::generate_random_tool_result_tag_name();
        format!(r#"<{tagName} name="{toolName}" status="{status}">{content}</{tagName}>"#)
    }

    fn createBoundedToolResultXml(
        toolName: &str,
        status: &str,
        rawPayload: &str,
        bodyBuilder: impl Fn(&str) -> String,
    ) -> String {
        let emptyXml = Self::createToolResultXml(toolName, status, &bodyBuilder(""));
        let maxPayloadChars = MAX_FINAL_TOOL_RESULT_MESSAGE_CHARS.saturating_sub(emptyXml.len());
        let boundedPayload = Self::truncatePayload(rawPayload, maxPayloadChars);
        Self::createToolResultXml(toolName, status, &bodyBuilder(&boundedPayload))
    }

    fn truncatePayload(payload: &str, maxChars: usize) -> String {
        if payload.chars().count() <= maxChars {
            return payload.to_string();
        }
        if maxChars == 0 {
            return String::new();
        }
        let suffix_len = TOOL_RESULT_TRUNCATION_SUFFIX.chars().count();
        if suffix_len >= maxChars {
            return TOOL_RESULT_TRUNCATION_SUFFIX
                .chars()
                .take(maxChars)
                .collect();
        }
        let keep = maxChars - suffix_len;
        let mut truncated = payload.chars().take(keep).collect::<String>();
        truncated = truncated.trim_end().to_string();
        truncated.push_str(TOOL_RESULT_TRUNCATION_SUFFIX);
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::{ConversationMarkupManager, ToolResult};
    use crate::core::tools::ToolResultDataClasses::{
        DirectoryListingData, FileEntry, StringResultData, TerminalCommandResultData,
        ToolResultData,
    };

    #[test]
    fn formats_runtime_tool_result_data_with_to_string() {
        let result = ToolResult {
            toolName: "execute_in_terminal_session".to_string(),
            success: true,
            result: ToolResultData::TerminalCommandResultData(TerminalCommandResultData {
                command: "Write-Output direct-package-ok".to_string(),
                output: "direct-package-ok\n".to_string(),
                exitCode: 0,
                sessionId: "session-1".to_string(),
                timedOut: false,
            }),
            error: None,
        };

        let formatted = ConversationMarkupManager::formatToolResultForMessage(&result);

        assert!(formatted.contains("Terminal Command Execution Result:"));
        assert!(formatted.contains("Command: Write-Output direct-package-ok"));
        assert!(formatted.contains("Session: session-1"));
        assert!(formatted.contains("Output:"));
        assert!(!formatted.contains(r#""__type":"TerminalCommandResultData""#));
    }

    #[test]
    fn formats_file_collection_tool_result_data_with_to_string() {
        let result = ToolResult {
            toolName: "list_files".to_string(),
            success: true,
            result: ToolResultData::DirectoryListingData(DirectoryListingData {
                path: "C:\\work".to_string(),
                entries: vec![FileEntry {
                    name: "notes.txt".to_string(),
                    isDirectory: false,
                    size: 42,
                    permissions: "rw-r--r--".to_string(),
                    lastModified: "2026-06-16T00:00:00Z".to_string(),
                }],
            }),
            error: None,
        };

        let formatted = ConversationMarkupManager::formatToolResultForMessage(&result);

        assert!(formatted.contains("Directory listing for C:\\work:"));
        assert!(formatted.contains("notes.txt"));
        assert!(!formatted.contains(r#""__type":"DirectoryListingData""#));
    }

    #[test]
    fn package_payload_keeps_nested_type_field_as_text() {
        let payload = r#"{"command":"Write-Output direct-package-ok","output":"direct-package-ok\n","exitCode":0,"sessionId":"super_admin_default_session","timedOut":false,"terminalEnvironment":{"__type":"TerminalInfoResultData","platform":"windows","defaultType":"powershell","types":[]}}"#;
        let result = ToolResult {
            toolName: "super_admin:terminal".to_string(),
            success: true,
            result: ToolResultData::StringResultData(StringResultData {
                value: payload.to_string(),
            }),
            error: None,
        };

        let formatted = ConversationMarkupManager::formatToolResultForMessage(&result);

        assert!(formatted.contains(payload));
    }

    #[test]
    fn package_payload_keeps_top_level_type_field_as_text() {
        let payload = r#"{"__type":"PackageOwnedType","value":"plain package json"}"#;
        let result = ToolResult {
            toolName: "example_package:tool".to_string(),
            success: true,
            result: ToolResultData::StringResultData(StringResultData {
                value: payload.to_string(),
            }),
            error: None,
        };

        let formatted = ConversationMarkupManager::formatToolResultForMessage(&result);

        assert!(formatted.contains(payload));
    }

    #[test]
    fn package_proxy_payload_keeps_top_level_type_field_as_text() {
        let payload = r#"{"__type":"PackageOwnedType","value":"plain package json"}"#;
        let result = ToolResult {
            toolName: "package_proxy".to_string(),
            success: true,
            result: ToolResultData::StringResultData(StringResultData {
                value: payload.to_string(),
            }),
            error: None,
        };

        let formatted = ConversationMarkupManager::formatToolResultForMessage(&result);

        assert!(formatted.contains(payload));
    }
}
