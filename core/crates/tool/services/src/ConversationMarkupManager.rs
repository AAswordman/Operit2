use crate::tools::ToolExecutionLimits::ToolExecutionLimits;
use operit_plugin_sdk::js_sdk::results::ToolResultData;
use operit_util::ChatMarkupRegex::ChatMarkupRegex;
use serde::{Deserialize, Serialize};

const TOOL_RESULT_TRUNCATION_SUFFIX: &str = "\n[工具结果过长，已截断]";
pub const ENHANCED_PURE_THINKING_ONLY_WARNING: &str = "警告：请输出正文内容，禁止仅输出思考内容。";
pub const ENHANCED_TRUNCATED_TOOL_CALL_WARNING: &str = "警告：检测到工具调用输出被截断。本轮所有工具调用均已作废且不会执行。请尝试减少单次输出、拆分任务，或更换更合适的模型/供应商后重试。";

#[derive(Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub toolName: String,
    pub success: bool,
    pub result: ToolResultData,
    pub error: Option<String>,
}

pub struct ConversationMarkupManager;

impl ConversationMarkupManager {
    /// Creates an explicit tool-error status message.
    pub fn createToolErrorStatus(toolName: &str, errorMessage: &str) -> String {
        Self::createToolResultXml(
            toolName,
            "error",
            &format!("<content><error>{}</error></content>", errorMessage),
        )
    }

    /// Creates a conversation warning without fabricating a tool result.
    pub fn createWarningStatus(warningMessage: &str) -> String {
        format!(r#"<status type="warning">{}</status>"#, warningMessage)
    }

    /// Formats one result within a single Unicode-scalar budget.
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

    /// Bounds each result independently while preserving every result slot in invocation order.
    pub fn buildToolResultMessage(results: &[ToolResult]) -> String {
        results
            .iter()
            .map(Self::formatToolResultForMessage)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Creates the error message for an unavailable tool.
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

    /// Wraps a tool payload in one uniquely named result element.
    fn createToolResultXml(toolName: &str, status: &str, content: &str) -> String {
        let tagName = ChatMarkupRegex::generate_random_tool_result_tag_name();
        format!(r#"<{tagName} name="{toolName}" status="{status}">{content}</{tagName}>"#)
    }

    /// Reserves wrapper and truncation-marker space using the same character unit.
    fn createBoundedToolResultXml(
        toolName: &str,
        status: &str,
        rawPayload: &str,
        bodyBuilder: impl Fn(&str) -> String,
    ) -> String {
        let tagName = ChatMarkupRegex::generate_random_tool_result_tag_name();
        let wrap = |payload: &str| {
            let content = bodyBuilder(payload);
            format!(r#"<{tagName} name="{toolName}" status="{status}">{content}</{tagName}>"#)
        };
        let maxPayloadChars = ToolExecutionLimits::MAX_SINGLE_TOOL_RESULT_MESSAGE_CHARS
            .saturating_sub(wrap("").chars().count());
        let boundedPayload = Self::truncatePayload(rawPayload, maxPayloadChars);
        wrap(&boundedPayload)
    }

    /// Truncates text on Unicode scalar boundaries while retaining the explicit marker.
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
    use operit_tools::tools::ToolResultDataClasses::{
        DirectoryListingData, FileEntry, StringResultData, TerminalCommandResultData,
        TerminalImplementation, TerminalType, ToolResultData,
    };
    use operit_tools::ConversationMarkupManager::{ConversationMarkupManager, ToolResult};

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
                platform: "windows".to_string(),
                terminal: TerminalImplementation::Native,
                terminalType: TerminalType::Powershell,
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
        let payload = r#"{"command":"Write-Output direct-package-ok","output":"direct-package-ok\n","exitCode":0,"sessionId":"super_admin_default_session","timedOut":false,"terminalEnvironment":{"__type":"TerminalInfoResultData","platform":"windows","terminal":"native","terminalType":"powershell","types":[]}}"#;
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

    /// Preserves every result when a batch exceeds the former shared limit.
    #[test]
    fn keeps_every_large_result_in_order() {
        let results = ["first_tool", "second_tool"].map(|name| ToolResult {
            toolName: name.to_string(),
            success: true,
            result: ToolResultData::StringResultData(StringResultData { value: "a".repeat(63_000) }),
            error: None,
        });
        let message = ConversationMarkupManager::buildToolResultMessage(&results);
        let blocks = operit_util::ChatMarkupRegex::ChatMarkupRegex::tool_result_blocks(&message);
        assert_eq!(blocks.len(), results.len());
        for (block, result) in blocks.iter().zip(&results) {
            assert_eq!(operit_util::ChatMarkupRegex::attr_value(&block.raw, "name"), Some(result.toolName.clone()));
            assert!(block.raw.chars().count() <= crate::tools::ToolExecutionLimits::ToolExecutionLimits::MAX_SINGLE_TOOL_RESULT_MESSAGE_CHARS);
        }
    }

    /// Uses one character unit for ASCII, CJK, emoji, wrapper names, and truncation markers.
    #[test]
    fn bounds_unicode_results_without_dropping_result_slots() {
        let limit = crate::tools::ToolExecutionLimits::ToolExecutionLimits::MAX_SINGLE_TOOL_RESULT_MESSAGE_CHARS;
        for (name, payload) in [
            ("grep_code", "a".repeat(364_096)),
            ("中文工具", "证".repeat(100_000)),
            ("emoji_tool", "🦀".repeat(100_000)),
        ] {
            let results = vec![
                ToolResult { toolName: name.to_string(), success: true, result: ToolResultData::StringResultData(StringResultData { value: payload }), error: None },
                ToolResult { toolName: "read_file_part".to_string(), success: true, result: ToolResultData::StringResultData(StringResultData { value: "read-ok".to_string() }), error: None },
            ];
            let message = ConversationMarkupManager::buildToolResultMessage(&results);
            let blocks = operit_util::ChatMarkupRegex::ChatMarkupRegex::tool_result_blocks(&message);
            assert_eq!(blocks.len(), 2);
            assert_eq!(blocks[0].raw.chars().count(), limit);
            assert!(blocks[0].body.ends_with(&format!("{}</content>", super::TOOL_RESULT_TRUNCATION_SUFFIX)));
            assert_eq!(blocks[1].body, "<content>read-ok</content>");
        }
    }

    /// Keeps the error envelope valid when a Unicode failure detail needs truncation.
    #[test]
    fn bounds_error_results_with_the_same_character_budget() {
        let result = ToolResult {
            toolName: "失败工具".to_string(),
            success: false,
            result: ToolResultData::StringResultData(StringResultData { value: "错误🦀".repeat(50_000) }),
            error: Some("execution failed".to_string()),
        };
        let message = ConversationMarkupManager::formatToolResultForMessage(&result);
        let blocks = operit_util::ChatMarkupRegex::ChatMarkupRegex::tool_result_blocks(&message);
        assert_eq!(blocks.len(), 1);
        assert_eq!(message.chars().count(), crate::tools::ToolExecutionLimits::ToolExecutionLimits::MAX_SINGLE_TOOL_RESULT_MESSAGE_CHARS);
        assert!(blocks[0].body.starts_with("<content><error>execution failed"));
        assert!(blocks[0].body.ends_with("</error></content>"));
    }

    /// Leaves the only truly empty batch empty instead of inventing a result.
    #[test]
    fn empty_batch_has_no_result_message() {
        assert_eq!(ConversationMarkupManager::buildToolResultMessage(&[]), "");
    }

    /// Includes XML and marker lengths without truncating payloads already exactly at the budget.
    #[test]
    fn truncation_respects_exact_payload_boundaries() {
        let result = |value: String| ToolResult {
            toolName: "grep_code".to_string(), success: true,
            result: ToolResultData::StringResultData(StringResultData { value }), error: None,
        };
        let empty = ConversationMarkupManager::formatToolResultForMessage(&result(String::new()));
        let limit = crate::tools::ToolExecutionLimits::ToolExecutionLimits::MAX_SINGLE_TOOL_RESULT_MESSAGE_CHARS;
        let payload_limit = limit - empty.chars().count();
        for size in [0, payload_limit - 1, payload_limit, payload_limit + 1] {
            let message = ConversationMarkupManager::formatToolResultForMessage(&result("x".repeat(size)));
            let blocks = operit_util::ChatMarkupRegex::ChatMarkupRegex::tool_result_blocks(&message);
            assert_eq!(blocks.len(), 1);
            assert!(message.chars().count() <= limit);
            if size <= payload_limit {
                assert_eq!(blocks[0].body, format!("<content>{}</content>", "x".repeat(size)));
            } else {
                assert!(blocks[0].body.ends_with(&format!("{}</content>", super::TOOL_RESULT_TRUNCATION_SUFFIX)));
            }
        }
    }
}
