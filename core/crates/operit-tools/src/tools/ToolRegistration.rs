use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use operit_host_api::HostManager::{defaultHostRuntimeTaskSchedulerHost, HostManager};
use operit_plugin_sdk::js_sdk::tool_types::BuiltinToolName;
use operit_plugin_sdk::package::ToolPackage;
use operit_tools::tools::climode::CliToolModeSupport::{
    CliToolModeSupport, PACKAGE_PROXY_TOOL_NAME, PROXY_TOOL_NAME, SEARCH_TOOL_NAME,
};
use operit_tools::tools::defaultTool::standard::StandardBluetoothTools::{
    BluetoothToolExecutor, BluetoothToolOperation, StandardBluetoothTools,
};
use operit_tools::tools::defaultTool::standard::StandardBrowserAutomationTools::{
    BrowserAutomationToolExecutor, StandardBrowserAutomationTools,
};
use operit_tools::tools::defaultTool::standard::StandardChatManagerTool::{
    ChatManagerToolExecutor, ChatManagerToolOperation, StandardChatManagerTool,
};
use operit_tools::tools::defaultTool::standard::StandardFileSystemTools::{
    FileSystemToolExecutor, FileSystemToolOperation, StandardFileSystemTools,
};
use operit_tools::tools::defaultTool::standard::StandardHttpTools::{
    HttpToolExecutor, HttpToolOperation, StandardHttpTools,
};
use operit_tools::tools::defaultTool::standard::StandardMemoryTools::{
    MemoryToolExecutor, MemoryToolOperation,
};
use operit_tools::tools::defaultTool::standard::StandardMusicTools::{
    MusicToolExecutor, MusicToolOperation, StandardMusicTools,
};
use operit_tools::tools::defaultTool::standard::StandardSystemOperationTools::{
    StandardSystemOperationTools, SystemOperationToolExecutor, SystemOperationToolOperation,
};
use operit_tools::tools::defaultTool::standard::StandardTerminalTools::{
    StandardTerminalTools, TerminalToolExecutor, TerminalToolOperation,
};
use operit_tools::tools::defaultTool::ToolGetter::ToolGetter;
use operit_tools::tools::mcp::MCPManager::MCPManager;
use operit_tools::tools::mcp::MCPToolExecutor::MCPToolExecutor;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_tools::tools::AIToolHandler::{
    AIToolHandler, FnToolExecutor, ToolRegistrationVisibility,
};
use operit_tools::tools::PackageToolExecutor::PackageToolExecutor;
use operit_tools::tools::ToolResultDataClasses::{
    stringResultData, EnvironmentVariableReadResultData, EnvironmentVariableWriteResultData,
    JsOptional, SleepResultData, ToolResultData,
};
use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{
    AITool, ToolEffect, ToolExecutionManager, ToolParameter, ToolValidationResult,
};

const FILE_SYSTEM_BUILTIN_TOOLS: &[BuiltinToolName] = &[
    BuiltinToolName::ListFiles,
    BuiltinToolName::ReadFile,
    BuiltinToolName::ReadFilePart,
    BuiltinToolName::ReadFileFull,
    BuiltinToolName::ReadFileBinary,
    BuiltinToolName::WriteFile,
    BuiltinToolName::WriteFileBinary,
    BuiltinToolName::DeleteFile,
    BuiltinToolName::FileExists,
    BuiltinToolName::MoveFile,
    BuiltinToolName::CopyFile,
    BuiltinToolName::MakeDirectory,
    BuiltinToolName::FindFiles,
    BuiltinToolName::GrepCode,
    BuiltinToolName::GrepContext,
    BuiltinToolName::FileInfo,
    BuiltinToolName::ZipFiles,
    BuiltinToolName::UnzipFiles,
    BuiltinToolName::OpenFile,
    BuiltinToolName::ShareFile,
    BuiltinToolName::DownloadFile,
    BuiltinToolName::ApplyFile,
    BuiltinToolName::CreateFile,
    BuiltinToolName::EditFile,
];

const BROWSER_AUTOMATION_BUILTIN_TOOLS: &[BuiltinToolName] = &[
    BuiltinToolName::BrowserClick,
    BuiltinToolName::BrowserClose,
    BuiltinToolName::BrowserCloseAll,
    BuiltinToolName::BrowserConsoleMessages,
    BuiltinToolName::BrowserDrag,
    BuiltinToolName::BrowserEvaluate,
    BuiltinToolName::BrowserFileUpload,
    BuiltinToolName::BrowserFillForm,
    BuiltinToolName::BrowserHandleDialog,
    BuiltinToolName::BrowserHover,
    BuiltinToolName::BrowserNavigate,
    BuiltinToolName::BrowserNavigateBack,
    BuiltinToolName::BrowserNetworkRequests,
    BuiltinToolName::BrowserPressKey,
    BuiltinToolName::BrowserResize,
    BuiltinToolName::BrowserRunCode,
    BuiltinToolName::BrowserSelectOption,
    BuiltinToolName::BrowserSnapshot,
    BuiltinToolName::BrowserTabs,
    BuiltinToolName::BrowserTakeScreenshot,
    BuiltinToolName::BrowserType,
    BuiltinToolName::BrowserWaitFor,
];

/// Registers every built-in public and internal tool on the handler.
#[allow(non_snake_case)]
pub fn registerAllTools(handler: &mut AIToolHandler, context: &HostManager) {
    registerPublicTools(handler, context);
    registerInternalTools(handler, context);
}

#[allow(non_snake_case)]
fn registerPublicTools(handler: &mut AIToolHandler, context: &HostManager) {
    handler.registerBuiltinTool(
        BuiltinToolName::Sleep,
        Box::new(FnToolExecutor {
            effect: ToolEffect::READ,
            validate: Arc::new(|_| ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            }),
            invoke: Arc::new(|tool| {
                let durationMs = tool
                    .parameters
                    .iter()
                    .find(|parameter| parameter.name == "duration_ms")
                    .and_then(|parameter| parameter.value.parse::<i32>().ok())
                    .unwrap_or(1000);
                let sleptMs = durationMs.max(0);
                defaultHostRuntimeTaskSchedulerHost()
                    .scheduleDelayedHostRuntimeTask(
                        "builtin-tool-sleep",
                        sleptMs as u64,
                        Box::new(|| {}),
                    )
                    .expect("runtime task scheduler must schedule the sleep tool delay");
                ToolResult {
                    toolName: tool.name.clone(),
                    success: true,
                    result: ToolResultData::SleepResultData(SleepResultData {
                        requestedMs: durationMs,
                        sleptMs,
                    }),
                    error: None,
                }
            }),
        }),
        ToolRegistrationVisibility::PUBLIC,
    );
    if let Some(fileSystemTools) = ToolGetter::getFileSystemTools(context, handler.runtimeSupport())
    {
        registerFileSystemTools(handler, fileSystemTools);
    } else {
        handler.markBuiltinToolsUnavailable(
            FILE_SYSTEM_BUILTIN_TOOLS,
            "File-system host capability is unavailable",
        );
    }
    handler.registerBuiltinTool(
        BuiltinToolName::VisitWeb,
        Box::new(ToolGetter::getWebVisitTool(context)),
        ToolRegistrationVisibility::PUBLIC,
    );
    registerSystemOperationTools(handler, ToolGetter::getSystemOperationTools(context));
    registerMusicTools(handler, ToolGetter::getMusicTools(context));
    registerBluetoothTools(handler, ToolGetter::getBluetoothTools(context));
    registerMemoryPublicTools(handler);
    registerChatTools(
        handler,
        StandardChatManagerTool::new(handler.runtimeSupport()),
    );

    let packageManager = handler.getOrCreatePackageManager();
    let usePackageManager = packageManager.clone();
    let usePackageHandler = handler.clone();
    handler.registerBuiltinTool(
        BuiltinToolName::UsePackage,
        Box::new(FnToolExecutor {
            effect: ToolEffect::WRITE,
            validate: Arc::new(|_| ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            }),
            invoke: Arc::new(move |tool| {
                let packageName = requiredParameterValue(tool, "package_name");
                let (result, selectedPackage) = {
                    let mut guard = usePackageManager
                        .lock()
                        .expect("package manager mutex poisoned");
                    let result = guard.executeUsePackageTool(&tool.name, &packageName);
                    let selectedPackage = if result.success {
                        guard
                            .getEffectivePackageTools(&packageName)
                            .filter(|package| !guard.isToolPkgContainer(&package.name))
                    } else {
                        None
                    };
                    (result, selectedPackage)
                };
                if let Some(selectedPackage) = selectedPackage {
                    registerPackageTools(
                        &usePackageHandler,
                        usePackageManager.clone(),
                        selectedPackage,
                    );
                }
                result
            }),
        }),
        ToolRegistrationVisibility::PUBLIC,
    );
    let searchContext = context.clone();
    let searchPackageManager = packageManager.clone();
    let searchRuntimeSupport = handler.runtimeSupport();
    handler.registerTool(
        SEARCH_TOOL_NAME.to_string(),
        Box::new(FnToolExecutor {
            effect: ToolEffect::READ,
            validate: Arc::new(|_| ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            }),
            invoke: Arc::new(move |tool| {
                let useEnglish = false;
                let runtimeContext = ToolExecutionManager::currentToolRuntimeContext();
                if runtimeContext
                    .as_ref()
                    .map(|context| context.toolExposureMode.clone())
                    != Some(operit_tools::ToolExecutionManager::ToolExposureMode::CLI)
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

                let hostEnvironment = searchContext.hostEnvironment.clone();
                let packageManagerGuard = searchPackageManager
                    .lock()
                    .expect("package manager mutex poisoned");
                let roleCardToolAccess = searchRuntimeSupport.resolveCharacterCardToolAccess(
                    runtimeContext
                        .as_ref()
                        .and_then(|context| context.callerCardId.as_deref()),
                    &packageManagerGuard,
                    None,
                );
                let catalog = CliToolModeSupport::buildHiddenToolCatalog(
                    &searchContext,
                    &packageManagerGuard,
                    useEnglish,
                    &roleCardToolAccess,
                    &hostEnvironment,
                    searchRuntimeSupport.as_ref(),
                );
                let results = CliToolModeSupport::searchHiddenToolCatalog(&catalog, &query, limit);
                ToolResult {
                    toolName: tool.name.clone(),
                    success: true,
                    result: stringResultData(CliToolModeSupport::formatSearchResults(
                        &query, &results, useEnglish,
                    )),
                    error: None,
                }
            }),
        }),
    );
    let proxyHandler = handler.clone();
    handler.registerTool(
        PROXY_TOOL_NAME.to_string(),
        Box::new(FnToolExecutor {
            effect: ToolEffect::WRITE,
            validate: Arc::new(|_| ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            }),
            invoke: Arc::new(move |tool| {
                let useEnglish = false;
                let runtimeContext = ToolExecutionManager::currentToolRuntimeContext();
                if runtimeContext
                    .as_ref()
                    .map(|context| context.toolExposureMode.clone())
                    != Some(operit_tools::ToolExecutionManager::ToolExposureMode::CLI)
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
                    return toolErrorResult(
                        tool,
                        "Missing required parameter: tool_name".to_string(),
                    );
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

                let packageManager = proxyHandler.getOrCreatePackageManager();
                let packageManagerGuard = packageManager
                    .lock()
                    .expect("package manager mutex poisoned");
                let proxyRuntimeSupport = proxyHandler.runtimeSupport();
                let roleCardToolAccess = proxyRuntimeSupport.resolveCharacterCardToolAccess(
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
                        result: stringResultData(""),
                        error: Some(CliToolModeSupport::buildRoleAccessDeniedMessage(useEnglish)),
                    };
                }

                let proxiedTool = AITool {
                    name: resolvedInvocation.targetToolName,
                    parameters: resolvedInvocation.forwardedParameters,
                };
                let mut clonedHandler = proxyHandler.clone();
                let proxiedResult = clonedHandler.executeTool(proxiedTool);
                ToolResult {
                    toolName: proxiedResult.toolName,
                    success: proxiedResult.success,
                    result: proxiedResult.result,
                    error: proxiedResult.error,
                }
            }),
        }),
    );
    registerTerminalTools(handler, ToolGetter::getTerminalTools(context));
}

#[allow(non_snake_case)]
fn registerChatTools(handler: &mut AIToolHandler, chatTools: StandardChatManagerTool) {
    registerChatTool(
        handler,
        &chatTools,
        BuiltinToolName::StartChatService,
        ChatManagerToolOperation::StartChatService,
    );
    registerChatTool(
        handler,
        &chatTools,
        BuiltinToolName::StopChatService,
        ChatManagerToolOperation::StopChatService,
    );
    registerChatTool(
        handler,
        &chatTools,
        BuiltinToolName::CreateNewChat,
        ChatManagerToolOperation::CreateNewChat,
    );
    registerChatTool(
        handler,
        &chatTools,
        BuiltinToolName::ListChats,
        ChatManagerToolOperation::ListChats,
    );
    registerChatTool(
        handler,
        &chatTools,
        BuiltinToolName::FindChat,
        ChatManagerToolOperation::FindChat,
    );
    registerChatTool(
        handler,
        &chatTools,
        BuiltinToolName::AgentStatus,
        ChatManagerToolOperation::AgentStatus,
    );
    registerChatTool(
        handler,
        &chatTools,
        BuiltinToolName::SwitchChat,
        ChatManagerToolOperation::SwitchChat,
    );
    registerChatTool(
        handler,
        &chatTools,
        BuiltinToolName::UpdateChatTitle,
        ChatManagerToolOperation::UpdateChatTitle,
    );
    registerChatTool(
        handler,
        &chatTools,
        BuiltinToolName::DeleteChat,
        ChatManagerToolOperation::DeleteChat,
    );
    registerChatTool(
        handler,
        &chatTools,
        BuiltinToolName::SendMessageToAi,
        ChatManagerToolOperation::SendMessageToAi,
    );
    registerChatTool(
        handler,
        &chatTools,
        BuiltinToolName::SendMessageToAiStreaming,
        ChatManagerToolOperation::SendMessageToAiStreaming,
    );
    registerChatTool(
        handler,
        &chatTools,
        BuiltinToolName::ListCharacterCards,
        ChatManagerToolOperation::ListCharacterCards,
    );
    registerChatTool(
        handler,
        &chatTools,
        BuiltinToolName::GetChatMessages,
        ChatManagerToolOperation::GetChatMessages,
    );
}

#[allow(non_snake_case)]
fn registerChatTool(
    handler: &mut AIToolHandler,
    chatTools: &StandardChatManagerTool,
    name: BuiltinToolName,
    operation: ChatManagerToolOperation,
) {
    handler.registerBuiltinTool(
        name,
        Box::new(ChatManagerToolExecutor {
            tools: chatTools.clone(),
            operation,
        }),
        ToolRegistrationVisibility::PUBLIC,
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
        BuiltinToolName::Toast,
        SystemOperationToolOperation::Toast,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        BuiltinToolName::SendNotification,
        SystemOperationToolOperation::SendNotification,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        BuiltinToolName::ModifySystemSetting,
        SystemOperationToolOperation::ModifySystemSetting,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        BuiltinToolName::GetSystemSetting,
        SystemOperationToolOperation::GetSystemSetting,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        BuiltinToolName::InstallApp,
        SystemOperationToolOperation::InstallApp,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        BuiltinToolName::UninstallApp,
        SystemOperationToolOperation::UninstallApp,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        BuiltinToolName::ListInstalledApps,
        SystemOperationToolOperation::ListInstalledApps,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        BuiltinToolName::StartApp,
        SystemOperationToolOperation::StartApp,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        BuiltinToolName::StopApp,
        SystemOperationToolOperation::StopApp,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        BuiltinToolName::GetNotifications,
        SystemOperationToolOperation::GetNotifications,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        BuiltinToolName::GetAppUsageTime,
        SystemOperationToolOperation::GetAppUsageTime,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        BuiltinToolName::GetDeviceLocation,
        SystemOperationToolOperation::GetDeviceLocation,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        BuiltinToolName::DeviceInfo,
        SystemOperationToolOperation::GetDeviceInfo,
    );
    registerSystemOperationTool(
        handler,
        &systemOperationTools,
        BuiltinToolName::CaptureScreenshot,
        SystemOperationToolOperation::CaptureScreenshot,
    );
}

#[allow(non_snake_case)]
fn registerSystemOperationTool(
    handler: &mut AIToolHandler,
    systemOperationTools: &StandardSystemOperationTools,
    name: BuiltinToolName,
    operation: SystemOperationToolOperation,
) {
    handler.registerBuiltinTool(
        name,
        Box::new(SystemOperationToolExecutor {
            tools: systemOperationTools.clone(),
            operation,
        }),
        ToolRegistrationVisibility::INTERNAL,
    );
}

#[allow(non_snake_case)]
fn registerTerminalTools(handler: &mut AIToolHandler, terminalTools: StandardTerminalTools) {
    registerTerminalTool(
        handler,
        &terminalTools,
        BuiltinToolName::GetTerminalInfo,
        TerminalToolOperation::GetTerminalInfo,
    );
    registerTerminalTool(
        handler,
        &terminalTools,
        BuiltinToolName::CreateTerminalSession,
        TerminalToolOperation::CreateSession,
    );
    registerTerminalTool(
        handler,
        &terminalTools,
        BuiltinToolName::ExecuteInTerminalSession,
        TerminalToolOperation::ExecuteInSession,
    );
    registerTerminalTool(
        handler,
        &terminalTools,
        BuiltinToolName::ExecuteInTerminalSessionStreaming,
        TerminalToolOperation::ExecuteInSessionStreaming,
    );
    registerTerminalTool(
        handler,
        &terminalTools,
        BuiltinToolName::ExecuteHiddenTerminalCommand,
        TerminalToolOperation::ExecuteHiddenCommand,
    );
    registerTerminalTool(
        handler,
        &terminalTools,
        BuiltinToolName::CloseTerminalSession,
        TerminalToolOperation::CloseSession,
    );
    registerTerminalTool(
        handler,
        &terminalTools,
        BuiltinToolName::InputInTerminalSession,
        TerminalToolOperation::InputInSession,
    );
    registerTerminalTool(
        handler,
        &terminalTools,
        BuiltinToolName::GetTerminalSessionScreen,
        TerminalToolOperation::GetSessionScreen,
    );
}

#[allow(non_snake_case)]
fn registerTerminalTool(
    handler: &mut AIToolHandler,
    terminalTools: &StandardTerminalTools,
    name: BuiltinToolName,
    operation: TerminalToolOperation,
) {
    handler.registerBuiltinTool(
        name,
        Box::new(TerminalToolExecutor {
            tools: terminalTools.clone(),
            operation,
        }),
        ToolRegistrationVisibility::PUBLIC,
    );
}

#[allow(non_snake_case)]
fn registerMusicTools(handler: &mut AIToolHandler, musicTools: StandardMusicTools) {
    registerMusicTool(
        handler,
        &musicTools,
        BuiltinToolName::MusicPlay,
        MusicToolOperation::Play,
    );
    registerMusicTool(
        handler,
        &musicTools,
        BuiltinToolName::MusicPause,
        MusicToolOperation::Pause,
    );
    registerMusicTool(
        handler,
        &musicTools,
        BuiltinToolName::MusicResume,
        MusicToolOperation::Resume,
    );
    registerMusicTool(
        handler,
        &musicTools,
        BuiltinToolName::MusicStop,
        MusicToolOperation::Stop,
    );
    registerMusicTool(
        handler,
        &musicTools,
        BuiltinToolName::MusicSeek,
        MusicToolOperation::Seek,
    );
    registerMusicTool(
        handler,
        &musicTools,
        BuiltinToolName::MusicSetVolume,
        MusicToolOperation::SetVolume,
    );
    registerMusicTool(
        handler,
        &musicTools,
        BuiltinToolName::MusicStatus,
        MusicToolOperation::Status,
    );
}

#[allow(non_snake_case)]
fn registerMusicTool(
    handler: &mut AIToolHandler,
    musicTools: &StandardMusicTools,
    name: BuiltinToolName,
    operation: MusicToolOperation,
) {
    handler.registerBuiltinTool(
        name,
        Box::new(MusicToolExecutor {
            tools: musicTools.clone(),
            operation,
        }),
        ToolRegistrationVisibility::PUBLIC,
    );
}

#[allow(non_snake_case)]
fn registerBluetoothTools(handler: &mut AIToolHandler, bluetoothTools: StandardBluetoothTools) {
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::RequestBluetoothPermission,
        BluetoothToolOperation::RequestPermission,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::GetBluetoothState,
        BluetoothToolOperation::GetState,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::RequestEnableBluetooth,
        BluetoothToolOperation::RequestEnable,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::ListBluetoothBondedDevices,
        BluetoothToolOperation::ListBondedDevices,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::ScanBluetoothDevices,
        BluetoothToolOperation::ScanDevices,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::BluetoothConnect,
        BluetoothToolOperation::Connect,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::BluetoothListen,
        BluetoothToolOperation::Listen,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::BluetoothAccept,
        BluetoothToolOperation::Accept,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::BluetoothSend,
        BluetoothToolOperation::Send,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::BluetoothRead,
        BluetoothToolOperation::Read,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::BluetoothSendAndRead,
        BluetoothToolOperation::SendAndRead,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::BluetoothClose,
        BluetoothToolOperation::Close,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::BluetoothBleConnect,
        BluetoothToolOperation::BleConnect,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::BluetoothBleDiscoverServices,
        BluetoothToolOperation::BleDiscoverServices,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::BluetoothBleReadCharacteristic,
        BluetoothToolOperation::BleReadCharacteristic,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::BluetoothBleWriteCharacteristic,
        BluetoothToolOperation::BleWriteCharacteristic,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::BluetoothBleWriteAndReadCharacteristic,
        BluetoothToolOperation::BleWriteAndReadCharacteristic,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::BluetoothBleSubscribeCharacteristic,
        BluetoothToolOperation::BleSubscribeCharacteristic,
    );
    registerBluetoothTool(
        handler,
        &bluetoothTools,
        BuiltinToolName::BluetoothBleReadNotifications,
        BluetoothToolOperation::BleReadNotifications,
    );
}

#[allow(non_snake_case)]
fn registerBluetoothTool(
    handler: &mut AIToolHandler,
    bluetoothTools: &StandardBluetoothTools,
    name: BuiltinToolName,
    operation: BluetoothToolOperation,
) {
    handler.registerBuiltinTool(
        name,
        Box::new(BluetoothToolExecutor {
            tools: bluetoothTools.clone(),
            operation,
        }),
        ToolRegistrationVisibility::PUBLIC,
    );
}

#[allow(non_snake_case)]
fn registerInternalTools(handler: &mut AIToolHandler, context: &HostManager) {
    let readEnvironmentRuntimeSupport = handler.runtimeSupport();
    registerHttpTools(handler, ToolGetter::getHttpTools(context));
    if let Some(browserTools) = ToolGetter::getBrowserAutomationTools(context) {
        registerBrowserAutomationTools(handler, browserTools);
    } else {
        handler.markBuiltinToolsUnavailable(
            BROWSER_AUTOMATION_BUILTIN_TOOLS,
            "Browser automation host capability is unavailable",
        );
    }
    registerMemoryInternalTools(handler);

    let packageProxyHandler = handler.clone();
    handler.registerBuiltinTool(
        BuiltinToolName::PackageProxy,
        Box::new(FnToolExecutor {
            effect: ToolEffect::WRITE,
            validate: Arc::new(|_| ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            }),
            invoke: Arc::new(move |tool| {
                let (parsedInvocation, parseError) = parseProxyInvocation(tool, true);
                if let Some(error) = parseError {
                    return error;
                }
                let Some(resolvedInvocation) = parsedInvocation else {
                    return toolErrorResult(
                        tool,
                        "Missing required parameter: tool_name".to_string(),
                    );
                };
                if resolvedInvocation.targetToolName == PACKAGE_PROXY_TOOL_NAME {
                    return toolErrorResult(tool, "tool_name cannot be package_proxy".to_string());
                }

                let proxiedTool = AITool {
                    name: resolvedInvocation.targetToolName,
                    parameters: resolvedInvocation.forwardedParameters,
                };
                let mut clonedHandler = packageProxyHandler.clone();
                let proxiedResult = clonedHandler.executeTool(proxiedTool);
                ToolResult {
                    toolName: proxiedResult.toolName,
                    success: proxiedResult.success,
                    result: proxiedResult.result,
                    error: proxiedResult.error,
                }
            }),
        }),
        ToolRegistrationVisibility::INTERNAL,
    );
    let cliCommandHandler = handler.clone();
    handler.registerBuiltinTool(
        BuiltinToolName::ExecuteCliCommand,
        Box::new(FnToolExecutor {
            effect: ToolEffect::WRITE,
            validate: Arc::new(|_| ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            }),
            invoke: Arc::new(move |tool| {
                let argsRaw = requiredParameterValue(tool, "args");
                let args = match serde_json::from_str::<Vec<String>>(&argsRaw) {
                    Ok(args) => args,
                    Err(error) => {
                        return toolErrorResult(
                            tool,
                            format!("args must be a JSON string array: {error}"),
                        );
                    }
                };
                let context = cliCommandHandler.getContext();
                let Some(executor) = context.coreCommandExecutor else {
                    return toolErrorResult(
                        tool,
                        "Core command executor is not configured.".to_string(),
                    );
                };
                match executor(args) {
                    Ok(output) => ToolResult {
                        toolName: tool.name.clone(),
                        success: true,
                        result: stringResultData(output),
                        error: None,
                    },
                    Err(error) => toolErrorResult(tool, error),
                }
            }),
        }),
        ToolRegistrationVisibility::INTERNAL,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::ReadEnvironmentVariable,
        Box::new(FnToolExecutor {
            effect: ToolEffect::READ,
            validate: Arc::new(|_| ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            }),
            invoke: Arc::new(move |tool| {
                let key = requiredParameterValue(tool, "key");
                if key.trim().is_empty() {
                    return ToolResult {
                        toolName: tool.name.clone(),
                        success: false,
                        result: ToolResultData::EnvironmentVariableReadResultData(
                            EnvironmentVariableReadResultData {
                                key: String::new(),
                                value: JsOptional::Null,
                                exists: false,
                            },
                        ),
                        error: Some("Missing required parameter: key".to_string()),
                    };
                }
                match readEnvironmentRuntimeSupport.readEnvironmentVariable(&key) {
                    Ok(value) => ToolResult {
                        toolName: tool.name.clone(),
                        success: true,
                        result: ToolResultData::EnvironmentVariableReadResultData(
                            EnvironmentVariableReadResultData {
                                key,
                                exists: value.is_some(),
                                value: JsOptional::from_nullable_option(value),
                            },
                        ),
                        error: None,
                    },
                    Err(error) => ToolResult {
                        toolName: tool.name.clone(),
                        success: false,
                        result: ToolResultData::EnvironmentVariableReadResultData(
                            EnvironmentVariableReadResultData {
                                key,
                                value: JsOptional::Null,
                                exists: false,
                            },
                        ),
                        error: Some(error.to_string()),
                    },
                }
            }),
        }),
        ToolRegistrationVisibility::INTERNAL,
    );
    let writeEnvironmentRuntimeSupport = handler.runtimeSupport();
    handler.registerBuiltinTool(
        BuiltinToolName::WriteEnvironmentVariable,
        Box::new(FnToolExecutor {
            effect: ToolEffect::WRITE,
            validate: Arc::new(|_| ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            }),
            invoke: Arc::new(move |tool| {
                let key = requiredParameterValue(tool, "key");
                let value = tool
                    .parameters
                    .iter()
                    .find(|parameter| parameter.name == "value")
                    .map(|parameter| parameter.value.clone())
                    .unwrap_or_default();
                let cleared = value.trim().is_empty();
                if key.trim().is_empty() {
                    return ToolResult {
                        toolName: tool.name.clone(),
                        success: false,
                        result: ToolResultData::EnvironmentVariableWriteResultData(
                            EnvironmentVariableWriteResultData {
                                key: String::new(),
                                requestedValue: String::new(),
                                value: JsOptional::Null,
                                exists: false,
                                cleared: false,
                            },
                        ),
                        error: Some("Missing required parameter: key".to_string()),
                    };
                }
                let writeResult = if cleared {
                    writeEnvironmentRuntimeSupport.removeEnvironmentVariable(&key)
                } else {
                    writeEnvironmentRuntimeSupport.writeEnvironmentVariable(&key, value.trim())
                };
                if let Err(error) = writeResult {
                    return ToolResult {
                        toolName: tool.name.clone(),
                        success: false,
                        result: ToolResultData::EnvironmentVariableWriteResultData(
                            EnvironmentVariableWriteResultData {
                                key,
                                requestedValue: value.clone(),
                                value: JsOptional::Null,
                                exists: false,
                                cleared,
                            },
                        ),
                        error: Some(error.to_string()),
                    };
                }

                match writeEnvironmentRuntimeSupport.readEnvironmentVariable(&key) {
                    Ok(current) => ToolResult {
                        toolName: tool.name.clone(),
                        success: true,
                        result: ToolResultData::EnvironmentVariableWriteResultData(
                            EnvironmentVariableWriteResultData {
                                key,
                                requestedValue: value,
                                exists: current.is_some(),
                                value: JsOptional::from_nullable_option(current),
                                cleared,
                            },
                        ),
                        error: None,
                    },
                    Err(error) => ToolResult {
                        toolName: tool.name.clone(),
                        success: false,
                        result: ToolResultData::EnvironmentVariableWriteResultData(
                            EnvironmentVariableWriteResultData {
                                key,
                                requestedValue: value.clone(),
                                value: JsOptional::Null,
                                exists: false,
                                cleared,
                            },
                        ),
                        error: Some(error.to_string()),
                    },
                }
            }),
        }),
        ToolRegistrationVisibility::INTERNAL,
    );
}

#[allow(non_snake_case)]
fn registerBrowserAutomationTools(
    handler: &mut AIToolHandler,
    browserTools: StandardBrowserAutomationTools,
) {
    for name in [
        BuiltinToolName::BrowserClick,
        BuiltinToolName::BrowserClose,
        BuiltinToolName::BrowserCloseAll,
        BuiltinToolName::BrowserConsoleMessages,
        BuiltinToolName::BrowserDrag,
        BuiltinToolName::BrowserEvaluate,
        BuiltinToolName::BrowserFileUpload,
        BuiltinToolName::BrowserFillForm,
        BuiltinToolName::BrowserHandleDialog,
        BuiltinToolName::BrowserHover,
        BuiltinToolName::BrowserNavigate,
        BuiltinToolName::BrowserNavigateBack,
        BuiltinToolName::BrowserNetworkRequests,
        BuiltinToolName::BrowserPressKey,
        BuiltinToolName::BrowserResize,
        BuiltinToolName::BrowserRunCode,
        BuiltinToolName::BrowserSelectOption,
        BuiltinToolName::BrowserSnapshot,
        BuiltinToolName::BrowserTabs,
        BuiltinToolName::BrowserTakeScreenshot,
        BuiltinToolName::BrowserType,
        BuiltinToolName::BrowserWaitFor,
    ] {
        handler.registerBuiltinTool(
            name,
            Box::new(BrowserAutomationToolExecutor {
                tools: browserTools.clone(),
            }),
            ToolRegistrationVisibility::INTERNAL,
        );
    }
}

#[allow(non_snake_case)]
fn registerMemoryPublicTools(handler: &mut AIToolHandler) {
    registerMemoryTool(
        handler,
        BuiltinToolName::QueryMemory,
        MemoryToolOperation::QueryMemory,
        false,
    );
    registerMemoryTool(
        handler,
        BuiltinToolName::GetMemoryByTitle,
        MemoryToolOperation::GetMemoryByTitle,
        false,
    );
}

#[allow(non_snake_case)]
fn registerMemoryInternalTools(handler: &mut AIToolHandler) {
    registerMemoryTool(
        handler,
        BuiltinToolName::CreateMemory,
        MemoryToolOperation::CreateMemory,
        true,
    );
    registerMemoryTool(
        handler,
        BuiltinToolName::UpdateMemory,
        MemoryToolOperation::UpdateMemory,
        true,
    );
    registerMemoryTool(
        handler,
        BuiltinToolName::DeleteMemory,
        MemoryToolOperation::DeleteMemory,
        true,
    );
    registerMemoryTool(
        handler,
        BuiltinToolName::MoveMemory,
        MemoryToolOperation::MoveMemory,
        true,
    );
    registerMemoryTool(
        handler,
        BuiltinToolName::UpdateUserPreferences,
        MemoryToolOperation::UpdateUserPreferences,
        true,
    );
    registerMemoryTool(
        handler,
        BuiltinToolName::LinkMemories,
        MemoryToolOperation::LinkMemories,
        true,
    );
    registerMemoryTool(
        handler,
        BuiltinToolName::QueryMemoryLinks,
        MemoryToolOperation::QueryMemoryLinks,
        true,
    );
    registerMemoryTool(
        handler,
        BuiltinToolName::UpdateMemoryLink,
        MemoryToolOperation::UpdateMemoryLink,
        true,
    );
    registerMemoryTool(
        handler,
        BuiltinToolName::DeleteMemoryLink,
        MemoryToolOperation::DeleteMemoryLink,
        true,
    );
}

#[allow(non_snake_case)]
fn registerMemoryTool(
    handler: &mut AIToolHandler,
    name: BuiltinToolName,
    operation: MemoryToolOperation,
    internal: bool,
) {
    let executor = Box::new(MemoryToolExecutor {
        operation,
        runtimeSupport: handler.runtimeSupport(),
    });
    let visibility = if internal {
        ToolRegistrationVisibility::INTERNAL
    } else {
        ToolRegistrationVisibility::PUBLIC
    };
    handler.registerBuiltinTool(name, executor, visibility);
}

#[allow(non_snake_case)]
fn registerHttpTools(handler: &mut AIToolHandler, httpTools: StandardHttpTools) {
    registerHttpTool(
        handler,
        &httpTools,
        BuiltinToolName::HttpRequest,
        HttpToolOperation::HttpRequest,
    );
    registerHttpTool(
        handler,
        &httpTools,
        BuiltinToolName::MultipartRequest,
        HttpToolOperation::MultipartRequest,
    );
    registerHttpTool(
        handler,
        &httpTools,
        BuiltinToolName::ManageCookies,
        HttpToolOperation::ManageCookies,
    );
}

#[allow(non_snake_case)]
fn registerHttpTool(
    handler: &mut AIToolHandler,
    httpTools: &StandardHttpTools,
    name: BuiltinToolName,
    operation: HttpToolOperation,
) {
    handler.registerBuiltinTool(
        name,
        Box::new(HttpToolExecutor {
            tools: httpTools.clone(),
            operation,
        }),
        ToolRegistrationVisibility::INTERNAL,
    );
}

#[allow(non_snake_case)]
fn registerFileSystemTools(handler: &mut AIToolHandler, fileSystemTools: StandardFileSystemTools) {
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::ListFiles,
        FileSystemToolOperation::ListFiles,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::ReadFile,
        FileSystemToolOperation::ReadFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::ReadFilePart,
        FileSystemToolOperation::ReadFilePart,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::ReadFileFull,
        FileSystemToolOperation::ReadFileFull,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::ReadFileBinary,
        FileSystemToolOperation::ReadFileBinary,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::WriteFile,
        FileSystemToolOperation::WriteFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::WriteFileBinary,
        FileSystemToolOperation::WriteFileBinary,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::DeleteFile,
        FileSystemToolOperation::DeleteFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::FileExists,
        FileSystemToolOperation::FileExists,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::MoveFile,
        FileSystemToolOperation::MoveFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::CopyFile,
        FileSystemToolOperation::CopyFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::MakeDirectory,
        FileSystemToolOperation::MakeDirectory,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::FindFiles,
        FileSystemToolOperation::FindFiles,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::FileInfo,
        FileSystemToolOperation::FileInfo,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::CreateFile,
        FileSystemToolOperation::CreateFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::EditFile,
        FileSystemToolOperation::EditFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::ZipFiles,
        FileSystemToolOperation::ZipFiles,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::UnzipFiles,
        FileSystemToolOperation::UnzipFiles,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::OpenFile,
        FileSystemToolOperation::OpenFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::ShareFile,
        FileSystemToolOperation::ShareFile,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::GrepCode,
        FileSystemToolOperation::GrepCode,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::GrepContext,
        FileSystemToolOperation::GrepContext,
    );
    registerFileSystemTool(
        handler,
        &fileSystemTools,
        BuiltinToolName::DownloadFile,
        FileSystemToolOperation::DownloadFile,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::ApplyFile,
        Box::new(FileSystemToolExecutor {
            tools: fileSystemTools,
            operation: FileSystemToolOperation::ApplyFile,
        }),
        ToolRegistrationVisibility::INTERNAL,
    );
}

#[allow(non_snake_case)]
fn registerFileSystemTool(
    handler: &mut AIToolHandler,
    fileSystemTools: &StandardFileSystemTools,
    name: BuiltinToolName,
    operation: FileSystemToolOperation,
) {
    handler.registerBuiltinTool(
        name,
        Box::new(FileSystemToolExecutor {
            tools: fileSystemTools.clone(),
            operation,
        }),
        ToolRegistrationVisibility::PUBLIC,
    );
}

fn requiredParameterValue(tool: &AITool, name: &str) -> String {
    tool.parameters
        .iter()
        .find(|parameter| parameter.name == name)
        .map(|parameter| parameter.value.trim().to_string())
        .unwrap_or_default()
}

#[allow(non_snake_case)]
fn registerPackageTools(
    handler: &AIToolHandler,
    packageManager: Arc<Mutex<RuntimePackageManager>>,
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
                Box::new(MCPToolExecutor::new(MCPManager::getInstance(
                    context.clone(),
                ))),
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

fn toolErrorResult(tool: &AITool, error: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result: stringResultData(""),
        error: Some(error),
    }
}

#[allow(non_snake_case)]
fn parseProxyInvocation(
    tool: &AITool,
    requireQualifiedTarget: bool,
) -> (Option<ParsedProxyInvocation>, Option<ToolResult>) {
    let allowedParamNames = BTreeSet::from_iter(
        [
            "tool_name",
            "params",
            "__operit_package_caller_name",
            "__operit_package_chat_id",
            "__operit_package_caller_card_id",
        ]
        .into_iter()
        .map(String::from),
    );
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
            if forwardedParameters
                .iter()
                .all(|parameter| parameter.name != paramName)
            {
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
