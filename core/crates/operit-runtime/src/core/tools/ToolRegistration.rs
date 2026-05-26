use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::FileBindingService::{
    FileBindingService, StructuredEditAction, StructuredEditOperation,
};
use crate::api::chat::enhance::ToolExecutionManager::{
    AITool, ToolExecutionManager, ToolParameter, ToolValidationResult,
};
use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::AIToolHandler::{AIToolHandler, FnToolExecutor};
use crate::core::tools::ToolPackage::{PackageToolExecutor, ToolPackage};
use crate::core::tools::climode::CliToolModeSupport::{
    CliToolModeSupport, PACKAGE_PROXY_TOOL_NAME, PROXY_TOOL_NAME, SEARCH_TOOL_NAME,
};
use crate::core::tools::mcp::MCPManager::MCPManager;
use crate::core::tools::mcp::MCPToolExecutor::MCPToolExecutor;
use crate::core::tools::defaultTool::ToolGetter::ToolGetter;
use crate::core::tools::defaultTool::standard::StandardFileSystemTools::{
    FileSystemToolExecutor, FileSystemToolOperation, StandardFileSystemTools,
};
use crate::core::tools::defaultTool::standard::StandardHttpTools::{
    HttpToolExecutor, HttpToolOperation, StandardHttpTools,
};
use crate::core::tools::defaultTool::standard::StandardMemoryTools::{
    MemoryToolExecutor, MemoryToolOperation,
};
use crate::core::tools::defaultTool::standard::StandardSystemOperationTools::{
    StandardSystemOperationTools, SystemOperationToolExecutor, SystemOperationToolOperation,
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
    handler.registerTool(
        "visit_web".to_string(),
        Box::new(ToolGetter::getWebVisitTool(context)),
    );
    registerSystemOperationTools(handler, ToolGetter::getSystemOperationTools(context));
    registerMemoryPublicTools(handler);

    let packageManager = handler.getOrCreatePackageManager();
    handler.registerTool(
        "use_package".to_string(),
        Box::new(UsePackageToolExecutor {
            packageManager: packageManager.clone(),
            handler: handler.clone(),
        }),
    );
    handler.registerTool(
        SEARCH_TOOL_NAME.to_string(),
        Box::new(SearchHiddenToolCatalogExecutor {
            context: context.clone(),
            packageManager,
        }),
    );
    handler.registerTool(
        PROXY_TOOL_NAME.to_string(),
        Box::new(ProxyToolExecutor {
            handler: handler.clone(),
        }),
    );
}

#[allow(non_snake_case)]
fn registerSystemOperationTools(
    handler: &mut AIToolHandler,
    systemOperationTools: StandardSystemOperationTools,
) {
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        "toast",
        SystemOperationToolOperation::Toast,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        "send_notification",
        SystemOperationToolOperation::SendNotification,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        "modify_system_setting",
        SystemOperationToolOperation::ModifySystemSetting,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        "get_system_setting",
        SystemOperationToolOperation::GetSystemSetting,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        "install_app",
        SystemOperationToolOperation::InstallApp,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        "uninstall_app",
        SystemOperationToolOperation::UninstallApp,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        "list_installed_apps",
        SystemOperationToolOperation::ListInstalledApps,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        "start_app",
        SystemOperationToolOperation::StartApp,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        "stop_app",
        SystemOperationToolOperation::StopApp,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        "get_notifications",
        SystemOperationToolOperation::GetNotifications,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        "get_app_usage_time",
        SystemOperationToolOperation::GetAppUsageTime,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        "get_device_location",
        SystemOperationToolOperation::GetDeviceLocation,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        "device_info",
        SystemOperationToolOperation::GetDeviceInfo,
    );
}

#[allow(non_snake_case)]
fn registerSystemOperationTool(
    handler: &mut AIToolHandler,
    systemOperationTools: &StandardSystemOperationTools,
    name: &str,
    operation: SystemOperationToolOperation,
) {
    handler.registerInternalTool(
        name.to_string(),
        Box::new(SystemOperationToolExecutor {
            tools: systemOperationTools.clone(),
            operation,
        }),
    );
}

#[allow(non_snake_case)]
fn registerInternalTools(handler: &mut AIToolHandler, context: &OperitApplicationContext) {
    registerHttpTools(handler, ToolGetter::getHttpTools(context));
    registerMemoryInternalTools(handler);

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
        Box::new(PackageProxyToolExecutor {
            handler: handler.clone(),
        }),
    );
}

#[allow(non_snake_case)]
fn registerMemoryPublicTools(handler: &mut AIToolHandler) {
    registerMemoryTool(handler, "query_memory", MemoryToolOperation::QueryMemory, false);
    registerMemoryTool(handler, "get_memory_by_title", MemoryToolOperation::GetMemoryByTitle, false);
}

#[allow(non_snake_case)]
fn registerMemoryInternalTools(handler: &mut AIToolHandler) {
    registerMemoryTool(handler, "create_memory", MemoryToolOperation::CreateMemory, true);
    registerMemoryTool(handler, "update_memory", MemoryToolOperation::UpdateMemory, true);
    registerMemoryTool(handler, "delete_memory", MemoryToolOperation::DeleteMemory, true);
    registerMemoryTool(handler, "move_memory", MemoryToolOperation::MoveMemory, true);
    registerMemoryTool(
        handler,
        "update_user_preferences",
        MemoryToolOperation::UpdateUserPreferences,
        true,
    );
    registerMemoryTool(handler, "link_memories", MemoryToolOperation::LinkMemories, true);
    registerMemoryTool(handler, "query_memory_links", MemoryToolOperation::QueryMemoryLinks, true);
    registerMemoryTool(handler, "update_memory_link", MemoryToolOperation::UpdateMemoryLink, true);
    registerMemoryTool(handler, "delete_memory_link", MemoryToolOperation::DeleteMemoryLink, true);
}

#[allow(non_snake_case)]
fn registerMemoryTool(
    handler: &mut AIToolHandler,
    name: &str,
    operation: MemoryToolOperation,
    internal: bool,
) {
    let executor = Box::new(MemoryToolExecutor { operation });
    if internal {
        handler.registerInternalTool(name.to_string(), executor);
    } else {
        handler.registerTool(name.to_string(), executor);
    }
}

#[allow(non_snake_case)]
fn registerHttpTools(handler: &mut AIToolHandler, httpTools: StandardHttpTools) {
    registerHttpTool(
        handler,
        &httpTools,
        "http_request",
        HttpToolOperation::HttpRequest,
    );
    registerHttpTool(
        handler,
        &httpTools,
        "multipart_request",
        HttpToolOperation::MultipartRequest,
    );
    registerHttpTool(
        handler,
        &httpTools,
        "manage_cookies",
        HttpToolOperation::ManageCookies,
    );
}

#[allow(non_snake_case)]
fn registerHttpTool(
    handler: &mut AIToolHandler,
    httpTools: &StandardHttpTools,
    name: &str,
    operation: HttpToolOperation,
) {
    handler.registerInternalTool(
        name.to_string(),
        Box::new(HttpToolExecutor {
            tools: httpTools.clone(),
            operation,
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
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        "download_file",
        FileSystemToolOperation::DownloadFile,
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
    handler: AIToolHandler,
}

struct PackageProxyToolExecutor {
    handler: AIToolHandler,
}

struct ProxyToolExecutor {
    handler: AIToolHandler,
}

struct SearchHiddenToolCatalogExecutor {
    context: OperitApplicationContext,
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
        vec![executeUsePackage(&self.packageManager, &self.handler, tool)]
    }
}

impl crate::api::chat::enhance::ToolExecutionManager::ToolExecutor for SearchHiddenToolCatalogExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        if requiredParameterValue(tool, "query").trim().is_empty() {
            return invalidToolValidation("query is required.");
        }
        ToolValidationResult {
            valid: true,
            errorMessage: String::new(),
        }
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        vec![executeSearchHiddenToolCatalog(
            tool,
            &self.context,
            &self.packageManager,
        )]
    }
}

impl crate::api::chat::enhance::ToolExecutionManager::ToolExecutor for PackageProxyToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        validatePackageProxy(tool)
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        vec![executePackageProxy(&self.handler, tool)]
    }
}

impl crate::api::chat::enhance::ToolExecutionManager::ToolExecutor for ProxyToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        validateProxy(tool)
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        vec![executeProxy(&self.handler, tool)]
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

fn executeUsePackage(
    packageManager: &Arc<Mutex<PackageManager>>,
    handler: &AIToolHandler,
    tool: &AITool,
) -> ToolResult {
    let packageName = requiredParameterValue(tool, "package_name");
    let (result, selectedPackage) = {
        let mut guard = packageManager.lock().expect("package manager mutex poisoned");
        let result = guard.executeUsePackageTool(&tool.name, &packageName);
        let selectedPackage = if result.success {
            guard.getEffectivePackageTools(&packageName)
                .filter(|package| !guard.isToolPkgContainer(&package.name))
        } else {
            None
        };
        (result, selectedPackage)
    };
    if let Some(selectedPackage) = selectedPackage {
        registerPackageTools(handler, packageManager.clone(), selectedPackage);
    }
    result
}

#[allow(non_snake_case)]
fn registerPackageTools(
    handler: &AIToolHandler,
    packageManager: Arc<Mutex<PackageManager>>,
    toolPackage: ToolPackage,
) {
    let isMcpPackage = toolPackage.category == "MCP"
        || toolPackage
            .tools
            .first()
            .map(|tool| tool.script.contains("/* MCPJS"))
            .unwrap_or(false);
    let executableTools = toolPackage
        .tools
        .iter()
        .filter(|packageTool| !packageTool.advice)
        .cloned()
        .collect::<Vec<_>>();
    let context = handler.getContext();
    for packageTool in executableTools {
        let toolName = format!("{}:{}", toolPackage.name, packageTool.name);
        let mut clonedHandler = handler.clone();
        if isMcpPackage {
            clonedHandler.registerTool(
                toolName,
                Box::new(MCPToolExecutor::new(MCPManager::getInstance(context.clone()))),
            );
        } else {
            clonedHandler.registerTool(
                toolName,
                Box::new(PackageToolExecutor::new(
                    toolPackage.clone(),
                    packageManager.clone(),
                    handler.clone(),
                )),
            );
        }
    }
}

#[allow(non_snake_case)]
fn executeSearchHiddenToolCatalog(
    tool: &AITool,
    context: &OperitApplicationContext,
    packageManager: &Arc<Mutex<PackageManager>>,
) -> ToolResult {
    let useEnglish = false;
    let runtimeContext = ToolExecutionManager::currentToolRuntimeContext();
    if runtimeContext
        .as_ref()
        .map(|context| context.toolExposureMode.clone())
        != Some(crate::api::chat::enhance::ToolExecutionManager::ToolExposureMode::CLI)
    {
        return toolErrorResult(
            tool,
            CliToolModeSupport::buildCliModeUnavailableMessage(useEnglish),
        );
    }

    let query = requiredParameterValue(tool, "query");
    if query.trim().is_empty() {
        return toolErrorResult(tool, "Missing required parameter: query".to_string());
    }
    let limit = tool
        .parameters
        .iter()
        .find(|parameter| parameter.name == "limit")
        .map(|parameter| parameter.value.trim().to_string())
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or_else(CliToolModeSupport::defaultSearchLimit);

    let hostEnvironment = context.hostEnvironment.clone();
    let packageManagerGuard = packageManager
        .lock()
        .expect("package manager mutex poisoned");
    let roleCardToolAccess =
        crate::data::preferences::CharacterCardToolAccessResolver::CharacterCardToolAccessResolver::getInstance()
            .resolve(
                runtimeContext
                    .as_ref()
                    .and_then(|context| context.callerCardId.as_deref()),
                &packageManagerGuard,
                None,
            );
    let catalog = CliToolModeSupport::buildHiddenToolCatalog(
        context,
        &packageManagerGuard,
        useEnglish,
        &roleCardToolAccess,
        &hostEnvironment,
    );
    let results = CliToolModeSupport::searchHiddenToolCatalog(&catalog, &query, limit);
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result: CliToolModeSupport::formatSearchResults(&query, &results, useEnglish),
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

fn validateProxy(tool: &AITool) -> ToolValidationResult {
    validatePackageProxy(tool)
}

fn executeProxy(handler: &AIToolHandler, tool: &AITool) -> ToolResult {
    let useEnglish = false;
    let runtimeContext = ToolExecutionManager::currentToolRuntimeContext();
    if runtimeContext
        .as_ref()
        .map(|context| context.toolExposureMode.clone())
        != Some(crate::api::chat::enhance::ToolExecutionManager::ToolExposureMode::CLI)
    {
        return toolErrorResult(
            tool,
            CliToolModeSupport::buildCliModeUnavailableMessage(useEnglish),
        );
    }

    let (parsedInvocation, parseError) = parseProxyInvocation(tool, false);
    if let Some(error) = parseError {
        return error;
    }
    let Some(resolvedInvocation) = parsedInvocation else {
        return toolErrorResult(tool, "Missing required parameter: tool_name".to_string());
    };

    if CliToolModeSupport::isReservedProxyTarget(&resolvedInvocation.targetToolName) {
        return toolErrorResult(
            tool,
            CliToolModeSupport::buildReservedProxyTargetMessage(
                &resolvedInvocation.targetToolName,
                useEnglish,
            ),
        );
    }

    let packageManager = handler.getOrCreatePackageManager();
    let packageManagerGuard = packageManager
        .lock()
        .expect("package manager mutex poisoned");
    let roleCardToolAccess =
        crate::data::preferences::CharacterCardToolAccessResolver::CharacterCardToolAccessResolver::getInstance()
            .resolve(
                runtimeContext
                    .as_ref()
                    .and_then(|context| context.callerCardId.as_deref()),
                &packageManagerGuard,
                None,
            );
    drop(packageManagerGuard);

    let usePackageSourceName = if resolvedInvocation.targetToolName == "use_package" {
        resolvedInvocation
            .forwardedParameters
            .iter()
            .find(|parameter| parameter.name == "package_name")
            .map(|parameter| parameter.value.trim().to_string())
            .filter(|value| !value.is_empty())
    } else {
        None
    };
    if !CliToolModeSupport::isToolNameAllowedForRoleCard(
        &resolvedInvocation.targetToolName,
        usePackageSourceName.as_deref(),
        &roleCardToolAccess,
    ) {
        return ToolResult {
            toolName: resolvedInvocation.targetToolName,
            success: false,
            result: String::new(),
            error: Some(CliToolModeSupport::buildRoleAccessDeniedMessage(useEnglish)),
        };
    }

    let proxiedTool = AITool {
        name: resolvedInvocation.targetToolName,
        parameters: resolvedInvocation.forwardedParameters,
    };
    let permissionSystem = handler.getToolPermissionSystem();
    let hasPermission = match permissionSystem.checkToolPermission(&proxiedTool) {
        Ok(value) => value,
        Err(_) => false,
    };
    if !hasPermission {
        let errorMessage = "User cancelled the tool execution.".to_string();
        handler.notifyToolPermissionChecked(&proxiedTool, false, Some(&errorMessage));
        return ToolResult {
            toolName: proxiedTool.name,
            success: false,
            result: String::new(),
            error: Some(errorMessage),
        };
    }

    handler.notifyToolPermissionChecked(&proxiedTool, true, None);
    let mut clonedHandler = handler.clone();
    let proxiedResult = clonedHandler.executeTool(proxiedTool);
    ToolResult {
        toolName: proxiedResult.toolName,
        success: proxiedResult.success,
        result: proxiedResult.result,
        error: proxiedResult.error,
    }
}

fn executePackageProxy(handler: &AIToolHandler, tool: &AITool) -> ToolResult {
    let (parsedInvocation, parseError) = parseProxyInvocation(tool, true);
    if let Some(error) = parseError {
        return error;
    }
    let Some(resolvedInvocation) = parsedInvocation else {
        return toolErrorResult(tool, "Missing required parameter: tool_name".to_string());
    };
    if resolvedInvocation.targetToolName == PACKAGE_PROXY_TOOL_NAME {
        return toolErrorResult(tool, "tool_name cannot be package_proxy".to_string());
    }

    let proxiedTool = AITool {
        name: resolvedInvocation.targetToolName,
        parameters: resolvedInvocation.forwardedParameters,
    };
    let mut clonedHandler = handler.clone();
    let proxiedResult = clonedHandler.executeTool(proxiedTool);
    ToolResult {
        toolName: proxiedResult.toolName,
        success: proxiedResult.success,
        result: proxiedResult.result,
        error: proxiedResult.error,
    }
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

#[allow(non_snake_case)]
fn parseProxyInvocation(
    tool: &AITool,
    requireQualifiedTarget: bool,
) -> (Option<ParsedProxyInvocation>, Option<ToolResult>) {
    let allowedParamNames =
        BTreeSet::from_iter(["tool_name", "params", "__operit_package_caller_name", "__operit_package_chat_id", "__operit_package_caller_card_id"].into_iter().map(String::from));
    let unknownParamNames = tool
        .parameters
        .iter()
        .map(|parameter| parameter.name.clone())
        .filter(|name| !allowedParamNames.contains(name))
        .collect::<Vec<_>>();
    if !unknownParamNames.is_empty() {
        return (
            None,
            Some(toolErrorResult(
                tool,
                format!(
                    "Unexpected parameters: {}. Only tool_name, params, and supported system context parameters are allowed",
                    unknownParamNames.join(", ")
                ),
            )),
        );
    }

    let toolNameParams = tool
        .parameters
        .iter()
        .filter(|parameter| parameter.name == "tool_name")
        .collect::<Vec<_>>();
    if toolNameParams.len() != 1 {
        return (
            None,
            Some(toolErrorResult(
                tool,
                "Exactly one tool_name parameter is required".to_string(),
            )),
        );
    }
    let targetToolName = toolNameParams[0].value.trim().to_string();
    if targetToolName.is_empty() {
        return (
            None,
            Some(toolErrorResult(
                tool,
                "Missing required parameter: tool_name".to_string(),
            )),
        );
    }
    if requireQualifiedTarget && !targetToolName.contains(':') {
        return (
            None,
            Some(toolErrorResult(
                tool,
                "tool_name must use packageName:toolName format".to_string(),
            )),
        );
    }

    let paramsParams = tool
        .parameters
        .iter()
        .filter(|parameter| parameter.name == "params")
        .collect::<Vec<_>>();
    if paramsParams.len() != 1 {
        return (
            None,
            Some(toolErrorResult(
                tool,
                "Exactly one params parameter is required".to_string(),
            )),
        );
    }
    let paramsRaw = paramsParams[0].value.trim().to_string();
    if paramsRaw.is_empty() {
        return (
            None,
            Some(toolErrorResult(
                tool,
                "params must be a JSON object".to_string(),
            )),
        );
    }

    let Ok(paramsObject) = serde_json::from_str::<serde_json::Value>(&paramsRaw) else {
        return (
            None,
            Some(toolErrorResult(
                tool,
                "params must be a valid JSON object".to_string(),
            )),
        );
    };
    let Some(object) = paramsObject.as_object() else {
        return (
            None,
            Some(toolErrorResult(
                tool,
                "params must be a JSON object".to_string(),
            )),
        );
    };

    let mut forwardedParameters = object
        .iter()
        .map(|(key, value)| ToolParameter {
            name: key.clone(),
            value: match value {
                serde_json::Value::Null => "null".to_string(),
                serde_json::Value::String(text) => text.clone(),
                _ => value.to_string(),
            },
        })
        .collect::<Vec<_>>();

    for paramName in [
        "__operit_package_caller_name",
        "__operit_package_chat_id",
        "__operit_package_caller_card_id",
    ] {
        let value = tool
            .parameters
            .iter()
            .find(|parameter| parameter.name == paramName)
            .map(|parameter| parameter.value.trim().to_string())
            .filter(|value| !value.is_empty());
        if let Some(value) = value {
            if forwardedParameters.iter().all(|parameter| parameter.name != paramName) {
                forwardedParameters.push(ToolParameter {
                    name: paramName.to_string(),
                    value,
                });
            }
        }
    }

    (
        Some(ParsedProxyInvocation {
            targetToolName,
            forwardedParameters,
        }),
        None,
    )
}

struct ParsedProxyInvocation {
    targetToolName: String,
    forwardedParameters: Vec<ToolParameter>,
}
