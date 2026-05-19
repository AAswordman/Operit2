use std::sync::{Arc, Mutex};

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::FileBindingService::{
    FileBindingService, StructuredEditAction, StructuredEditOperation,
};
use crate::api::chat::enhance::ToolExecutionManager::{AITool, ToolValidationResult};
use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::AIToolHandler::{AIToolHandler, FnToolExecutor};
use crate::core::tools::defaultTool::ToolGetter::ToolGetter;
use crate::core::tools::defaultTool::standard::StandardFileSystemTools::{
    FileSystemToolExecutor, FileSystemToolOperation, StandardFileSystemTools,
};
use crate::core::tools::packTool::PackageManager::PackageManager;
use operit_host_api::FileSystemHost;

#[allow(non_snake_case)]
pub fn registerAllTools(handler: &mut AIToolHandler, context: &OperitApplicationContext) {
    registerPublicTools(handler, context);
    registerInternalTools(handler, context);
}

#[allow(non_snake_case)]
fn registerPublicTools(handler: &mut AIToolHandler, context: &OperitApplicationContext) {
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

    let packageManager = Arc::new(Mutex::new(PackageManager::default()));
    handler.registerTool(
        "use_package".to_string(),
        Box::new(UsePackageToolExecutor { packageManager }),
    );
}

#[allow(non_snake_case)]
fn registerInternalTools(handler: &mut AIToolHandler, context: &OperitApplicationContext) {
    if let Some(fileSystemHost) = context.fileSystemHost.clone() {
        handler.registerInternalTool(
            "apply_file".to_string(),
            Box::new(ApplyFileToolExecutor {
                fileBindingService: FileBindingService,
                fileSystemHost,
            }),
        );
    }

    handler.registerInternalTool(
        "package_proxy".to_string(),
        Box::new(FnToolExecutor {
            name: "package_proxy".to_string(),
            validate: validatePackageProxy,
            invoke: executePackageProxy,
        }),
    );
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
        "create_file",
        FileSystemToolOperation::CreateFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "edit_file",
        FileSystemToolOperation::EditFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "zip_files",
        FileSystemToolOperation::ZipFiles,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "unzip_files",
        FileSystemToolOperation::UnzipFiles,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "open_file",
        FileSystemToolOperation::OpenFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "share_file",
        FileSystemToolOperation::ShareFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "grep_code",
        FileSystemToolOperation::GrepCode,
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

struct ApplyFileToolExecutor {
    fileBindingService: FileBindingService,
    fileSystemHost: std::sync::Arc<dyn FileSystemHost>,
}

struct UsePackageToolExecutor {
    packageManager: Arc<Mutex<PackageManager>>,
}

impl crate::api::chat::enhance::ToolExecutionManager::ToolExecutor for ApplyFileToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        validateApplyFile(tool)
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        vec![executeApplyFile(
            &self.fileBindingService,
            self.fileSystemHost.as_ref(),
            tool,
        )]
    }
}

impl crate::api::chat::enhance::ToolExecutionManager::ToolExecutor for UsePackageToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        validateUsePackage(tool)
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        vec![executeUsePackage(&self.packageManager, tool)]
    }
}

#[allow(non_snake_case)]
fn validateApplyFile(tool: &AITool) -> ToolValidationResult {
    let path = requiredParameterValue(tool, "path");
    let operationType = requiredParameterValue(tool, "type").to_ascii_lowercase();
    if path.trim().is_empty() {
        return invalidToolValidation("path is required.");
    }
    match operationType.as_str() {
        "create" => {
            if requiredParameterValue(tool, "new").trim().is_empty() {
                return invalidToolValidation("new is required for type=create.");
            }
        }
        "replace" => {
            if requiredParameterValue(tool, "old").trim().is_empty() {
                return invalidToolValidation("old is required for type=replace.");
            }
            if requiredParameterValue(tool, "new").trim().is_empty() {
                return invalidToolValidation("new is required for type=replace.");
            }
        }
        "delete" => {
            if requiredParameterValue(tool, "old").trim().is_empty() {
                return invalidToolValidation("old is required for type=delete.");
            }
        }
        _ => {
            return invalidToolValidation("type must be create, replace, or delete.");
        }
    }
    ToolValidationResult {
        valid: true,
        errorMessage: String::new(),
    }
}

#[allow(non_snake_case)]
fn executeApplyFile(
    fileBindingService: &FileBindingService,
    fileSystemHost: &dyn FileSystemHost,
    tool: &AITool,
) -> ToolResult {
    let path = requiredParameterValue(tool, "path");
    if let Err(error) = fileSystemHost.validatePath(&path, "path") {
        return toolErrorResult(tool, error.message);
    }

    let operationType = requiredParameterValue(tool, "type").to_ascii_lowercase();
    let oldContent = requiredParameterValue(tool, "old");
    let newContent = requiredParameterValue(tool, "new");
    let existence = match fileSystemHost.fileExists(&path) {
        Ok(value) => value,
        Err(error) => return toolErrorResult(tool, error.message),
    };

    match operationType.as_str() {
        "create" => {
            if existence.exists {
                return toolErrorResult(
                    tool,
                    "If you want to rewrite an entire existing file: please delete_file first then use apply_file with type=create (do not overwrite directly).".to_string(),
                );
            }
            match fileSystemHost.writeFile(&path, &newContent, false) {
                Ok(()) => ToolResult {
                    toolName: tool.name.clone(),
                    success: true,
                    result: format!("Created file: {path}"),
                    error: None,
                },
                Err(error) => toolErrorResult(tool, error.message),
            }
        }
        "replace" | "delete" => {
            if !existence.exists {
                return toolErrorResult(tool, format!("File does not exist: {path}"));
            }
            if existence.isDirectory {
                return toolErrorResult(tool, format!("Path is not a file: {path}"));
            }
            let originalContent = match fileSystemHost.readFile(&path) {
                Ok(value) => value,
                Err(error) => return toolErrorResult(tool, error.message),
            };
            let operation = StructuredEditOperation {
                action: if operationType == "replace" {
                    StructuredEditAction::REPLACE
                } else {
                    StructuredEditAction::DELETE
                },
                oldContent,
                newContent,
            };
            let (updatedContent, diffResult) =
                fileBindingService.processFileBindingOperations(&originalContent, &[operation]);
            if diffResult.starts_with("Error:") {
                return toolErrorResult(tool, diffResult);
            }
            match fileSystemHost.writeFile(&path, &updatedContent, false) {
                Ok(()) => ToolResult {
                    toolName: tool.name.clone(),
                    success: true,
                    result: diffResult,
                    error: None,
                },
                Err(error) => toolErrorResult(tool, error.message),
            }
        }
        _ => toolErrorResult(tool, "type must be create, replace, or delete.".to_string()),
    }
}

fn requiredParameterValue(tool: &AITool, name: &str) -> String {
    tool.parameters
        .iter()
        .find(|parameter| parameter.name == name)
        .map(|parameter| parameter.value.trim().to_string())
        .unwrap_or_default()
}

fn validateUsePackage(tool: &AITool) -> ToolValidationResult {
    if requiredParameterValue(tool, "package_name").trim().is_empty() {
        return invalidToolValidation("package_name is required.");
    }
    ToolValidationResult {
        valid: true,
        errorMessage: String::new(),
    }
}

fn executeUsePackage(packageManager: &Arc<Mutex<PackageManager>>, tool: &AITool) -> ToolResult {
    let packageName = requiredParameterValue(tool, "package_name");
    let mut guard = packageManager.lock().expect("package manager mutex poisoned");
    let newlyActivated = guard.activatePackage(&packageName);
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result: if newlyActivated {
            format!("Package activated: {packageName}")
        } else {
            format!("Package already active: {packageName}")
        },
        error: None,
    }
}

fn validatePackageProxy(tool: &AITool) -> ToolValidationResult {
    if requiredParameterValue(tool, "tool_name").trim().is_empty() {
        return invalidToolValidation("tool_name is required.");
    }
    if requiredParameterValue(tool, "params").trim().is_empty() {
        return invalidToolValidation("params is required.");
    }
    ToolValidationResult {
        valid: true,
        errorMessage: String::new(),
    }
}

fn executePackageProxy(tool: &AITool) -> ToolResult {
    toolErrorResult(
        tool,
        "Package proxy execution is not implemented in the current Rust runtime.".to_string(),
    )
}

fn invalidToolValidation(message: &str) -> ToolValidationResult {
    ToolValidationResult {
        valid: false,
        errorMessage: message.to_string(),
    }
}

fn toolErrorResult(tool: &AITool, error: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result: String::new(),
        error: Some(error),
    }
}
