use crate::util::ChatMarkupRegex::ChatMarkupRegex;

const TOOL_RESULT_TRUNCATION_SUFFIX: &str = "\n[工具结果过长，已截断]";
const MAX_FINAL_TOOL_RESULT_MESSAGE_CHARS: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolResult {
    pub toolName: String,
    pub success: bool,
    pub result: String,
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
                &result.result,
                |payload| format!("<content>{payload}</content>"),
            )
        } else {
            let message = result.error.clone().unwrap_or_default().trim().to_string();
            let detail = result.result.trim().to_string();
            let errorPayload = if !message.is_empty() && !detail.is_empty() {
                format!("{message}\n\n{detail}")
            } else if !message.is_empty() {
                message
            } else {
                detail
            };
            Self::createBoundedToolResultXml(
                &result.toolName,
                "error",
                &errorPayload,
                |payload| format!("<content><error>{payload}</error></content>"),
            )
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

    pub fn createMultipleToolsWarning(toolName: &str) -> String {
        Self::createWarningStatus(&format!(
            "Multiple tool invocations were found; only `{}` will be processed.",
            toolName
        ))
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
        format!(
            r#"<{tagName} name="{toolName}" status="{status}">{content}</{tagName}>"#
        )
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
            return TOOL_RESULT_TRUNCATION_SUFFIX.chars().take(maxChars).collect();
        }
        let keep = maxChars - suffix_len;
        let mut truncated = payload.chars().take(keep).collect::<String>();
        truncated = truncated.trim_end().to_string();
        truncated.push_str(TOOL_RESULT_TRUNCATION_SUFFIX);
        truncated
    }
}
