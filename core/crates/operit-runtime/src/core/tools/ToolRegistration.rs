use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::{AITool, ToolValidationResult};
use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::AIToolHandler::{AIToolHandler, FnToolExecutor};
use crate::core::tools::defaultTool::ToolGetter::ToolGetter;
use crate::core::tools::defaultTool::standard::StandardFileSystemTools::{
    FileSystemToolExecutor, FileSystemToolOperation, StandardFileSystemTools,
};

#[allow(non_snake_case)]
pub fn registerAllTools(handler: &mut AIToolHandler, context: &OperitApplicationContext) {
    handler.registerTool(
        "sleep".to_string(),
        Box::new(FnToolExecutor {
            name: "sleep".to_string(),
            validate: validateSleep,
            invoke: executeSleep,
        }),
    );
    if let Some(fileSystemTools) = ToolGetter::getFileSystemTools(context) {
        registerFileSystemTools(handler, fileSystemTools);
    }
}

#[allow(non_snake_case)]
fn registerFileSystemTools(handler: &mut AIToolHandler, fileSystemTools: StandardFileSystemTools) {
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "list_files",
        FileSystemToolOperation::ListFiles,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "read_file",
        FileSystemToolOperation::ReadFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "read_file_part",
        FileSystemToolOperation::ReadFilePart,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "read_file_full",
        FileSystemToolOperation::ReadFileFull,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "read_file_binary",
        FileSystemToolOperation::ReadFileBinary,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "write_file",
        FileSystemToolOperation::WriteFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "write_file_binary",
        FileSystemToolOperation::WriteFileBinary,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "delete_file",
        FileSystemToolOperation::DeleteFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "file_exists",
        FileSystemToolOperation::FileExists,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "move_file",
        FileSystemToolOperation::MoveFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "copy_file",
        FileSystemToolOperation::CopyFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "make_directory",
        FileSystemToolOperation::MakeDirectory,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "find_files",
        FileSystemToolOperation::FindFiles,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "file_info",
        FileSystemToolOperation::FileInfo,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "grep_code",
        FileSystemToolOperation::GrepCode,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "create_file",
        FileSystemToolOperation::CreateFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "edit_file",
        FileSystemToolOperation::EditFile,
    );
}

#[allow(non_snake_case)]
fn registerFileSystemTool(
    handler: &mut AIToolHandler,
    fileSystemTools: &StandardFileSystemTools,
    name: &str,
    operation: FileSystemToolOperation,
) {
    handler.registerTool(
        name.to_string(),
        Box::new(FileSystemToolExecutor {
            tools: fileSystemTools.clone(),
            operation,
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
