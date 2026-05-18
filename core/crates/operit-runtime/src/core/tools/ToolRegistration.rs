use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::{AITool, ToolValidationResult};
use crate::core::tools::AIToolHandler::{AIToolHandler, FnToolExecutor};

#[allow(non_snake_case)]
pub fn registerAllTools(handler: &mut AIToolHandler) {
    handler.registerTool(
        "sleep".to_string(),
        Box::new(FnToolExecutor {
            name: "sleep".to_string(),
            validate: validateSleep,
            invoke: executeSleep,
        }),
    );
}

#[allow(non_snake_case)]
fn validateSleep(tool: &AITool) -> ToolValidationResult {
    let duration = tool
        .parameters
        .iter()
        .find(|parameter| parameter.name == "duration_ms")
        .map(|parameter| parameter.value.trim().to_string());
    match duration {
        Some(value) if value.parse::<u64>().is_err() => ToolValidationResult {
            valid: false,
            errorMessage: "duration_ms must be an integer.".to_string(),
        },
        _ => ToolValidationResult {
            valid: true,
            errorMessage: String::new(),
        },
    }
}

#[allow(non_snake_case)]
fn executeSleep(tool: &AITool) -> ToolResult {
    let durationMs = tool
        .parameters
        .iter()
        .find(|parameter| parameter.name == "duration_ms")
        .and_then(|parameter| parameter.value.trim().parse::<u64>().ok())
        .unwrap_or(1000);
    std::thread::sleep(std::time::Duration::from_millis(durationMs));
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result: format!("Slept for {durationMs} ms."),
        error: None,
    }
}
