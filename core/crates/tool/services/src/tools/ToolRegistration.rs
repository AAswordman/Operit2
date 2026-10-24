use std::sync::Arc;

use operit_host_api::HostManager::{defaultHostRuntimeTaskSchedulerHost, HostManager};
use operit_plugin_sdk::js_sdk::tool_types::BuiltinToolName;
use operit_tools::tools::climode::CliToolModeSupport::{
    CliToolModeSupport, SEARCH_TOOL_NAME,
};
use operit_tools::tools::defaultTool::standard::StandardBluetoothTools::{
    BluetoothToolExecutor, BluetoothToolOperation, StandardBluetoothTools,
};
use operit_tools::tools::defaultTool::standard::StandardBrowserAutomationTools::{
    BrowserAutomationToolExecutor, StandardBrowserAutomationTools,
};
use operit_tools::tools::defaultTool::standard::StandardChatManagerTool::{
    ChatManagerToolExecutor, ChatManagerToolOperation, StandardChatManagerTool, PluginChatToolExecutor, PluginChatToolOperation,
};
use operit_tools::tools::defaultTool::standard::StandardFileSystemTools::{
    FileSystemToolExecutor, FileSystemToolOperation, StandardFileSystemTools,
};
use operit_tools::tools::defaultTool::standard::StandardHttpTools::{
    HttpToolExecutor, HttpToolOperation, StandardHttpTools,
};
use operit_tools::tools::defaultTool::standard::StandardMediaTools::{
    RegisterMediaExecutor, RegisterMediaFromPathExecutor,
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
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_tools::tools::AIToolHandler::{
    AIToolHandler, AsyncFnToolExecutor, FnToolExecutor, ToolRegistrationVisibility,
};
use operit_tools::tools::ToolResultDataClasses::{
    stringResultData, EnvironmentVariableReadResultData, EnvironmentVariableWriteResultData,
    JsOptional, SleepResultData, ToolResultData,
};
use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{
    AITool, ToolEffect, ToolExecutionManager, ToolParameter, ToolValidationResult, RegisteredToolExecutor,
};

use crate::tools::defaultTool::standard::{
    StandardModelTools::ModelToolExecutor,
    StandardTtsTools::{TtsToolExecutor, TtsToolOperation, SttToolExecutor},
    StandardThemeTools::{ThemeToolExecutor, ThemeToolOperation},
    StandardCatalogTools::CatalogToolExecutor,
    StandardStorageTools::StorageToolExecutor,
};

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
    let coreNodeRuntimeSupport = handler.runtimeSupport();
    handler.registerBuiltinTool(
        BuiltinToolName::ListCoreNodes,
        Box::new(FnToolExecutor {
            effect: ToolEffect::READ,
            validate: Arc::new(|_| ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            }),
            invoke: {
                let runtimeSupport = coreNodeRuntimeSupport.clone();
                Arc::new(move |tool| {
                    let state = match runtimeSupport.coreNodeRouteState() {
                        Ok(state) => state,
                        Err(error) => return toolErrorResult(tool, error),
                    };
                    let result = serde_json::json!({
                        "currentNodeId": state.currentNodeId,
                        "nodes": state.nodes.into_iter().map(|node| {
                            let mut encoded = serde_json::json!({
                                "nodeId": node.nodeId,
                                "displayName": node.displayName,
                                "userName": node.userName,
                                "platform": node.platform,
                                "model": node.model,
                                "reachable": node.reachable,
                                "canExecute": node.canExecute,
                            });
                            if let Some(relayHops) = node.relayHops {
                                encoded["relayHops"] = serde_json::json!(relayHops);
                            }
                            if let Some(relayPath) = node.relayPath {
                                encoded["relayPath"] = serde_json::json!(relayPath);
                            }
                            encoded
                        }).collect::<Vec<_>>(),
                    });
                    ToolResult {
                        toolName: tool.name.clone(),
                        success: true,
                        result: stringResultData(result.to_string()),
                        error: None,
                    }
                })
            },
        }),
        ToolRegistrationVisibility::PUBLIC,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::SwitchCore,
        Box::new(FnToolExecutor {
            effect: ToolEffect::WRITE,
            validate: Arc::new(|tool| {
                let targetNodeId = tool
                    .parameters
                    .iter()
                    .find(|parameter| parameter.name == "node_id")
                    .map(|parameter| parameter.value.trim());
                match targetNodeId {
                    Some(nodeId) if !nodeId.is_empty() => ToolValidationResult {
                        valid: true,
                        errorMessage: String::new(),
                    },
                    _ => ToolValidationResult {
                        valid: false,
                        errorMessage: "Missing required parameter: node_id".to_string(),
                    },
                }
            }),
            invoke: {
                let runtimeSupport = coreNodeRuntimeSupport.clone();
                Arc::new(move |tool| {
                    let targetReference = tool
                        .parameters
                        .iter()
                        .find(|parameter| parameter.name == "node_id")
                        .expect("validated switch_core invocation must contain node_id")
                        .value
                        .trim()
                        .to_string();
                    // The actual Binding move happens after this model turn ends;
                    // without a pre-flight check every invalid target would only
                    // fail asynchronously, leaving the tool result claiming success.
                    let state = match runtimeSupport.coreNodeRouteState() {
                        Ok(state) => state,
                        Err(error) => return toolErrorResult(tool, error),
                    };
                    // The marker must carry the canonical node id so an accepted
                    // device name or id prefix cannot move the Binding elsewhere.
                    let targetNodeId = match resolveSwitchCoreTarget(&state, &targetReference) {
                        Ok(node) => node.nodeId.clone(),
                        Err(error) => return toolErrorResult(tool, error),
                    };
                    ToolResult {
                        toolName: tool.name.clone(),
                        success: true,
                        result: stringResultData(targetNodeId),
                        error: None,
                    }
                })
            },
        }),
        ToolRegistrationVisibility::PUBLIC,
    );
    if let Some(fileSystemTools) = ToolGetter::getFileSystemTools(context, handler.runtimeSupport())
    {
        registerFileSystemTools(handler, fileSystemTools);
    }
    handler.registerBuiltinTool(
        BuiltinToolName::VisitWeb,
        Box::new(ToolGetter::getWebVisitTool(context)),
        ToolRegistrationVisibility::PUBLIC,
    );
    registerSystemOperationTools(handler, ToolGetter::getSystemOperationTools(context));
    registerMusicTools(handler, ToolGetter::getMusicTools(context));
    registerBluetoothTools(handler, ToolGetter::getBluetoothTools(context));
    registerChatTools(
        handler,
        StandardChatManagerTool::new(handler.runtimeSupport()),
    );

    let packageManager = handler.getOrCreatePackageManager();
    let usePackageManager = packageManager.clone();
    handler.registerBuiltinTool(
        BuiltinToolName::UsePackage,
        crate::ToolExecutionManager::RegisteredToolExecutor::Asynchronous(Box::new(AsyncFnToolExecutor {
            effect: ToolEffect::WRITE,
            validate: Arc::new(|_| ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            }),
            invoke: Arc::new(move |tool, invocationContext| {
                let usePackageManager = usePackageManager.clone();
                let usePackageHandler = invocationContext.host().clone();
                Box::pin(async move {
                let tool = &tool;
                let packageName = requiredParameterValue(tool, "package_name");
                if let Err(error) = RuntimePackageManager::prepareMcpPackage(&usePackageManager, &packageName).await {
                    return toolErrorResult(tool, error);
                }
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
                    let mut usePackageHandler = usePackageHandler;
                    usePackageHandler.registerPackageTools(usePackageManager.clone(), selectedPackage);
                }
                result
                })
            }),
        })),
        ToolRegistrationVisibility::PUBLIC,
    );
    let searchContext = context.clone();
    let searchPackageManager = packageManager.clone();
    let searchRuntimeSupport = handler.runtimeSupport();
    handler.registerAsyncTool(
        SEARCH_TOOL_NAME.to_string(),
        Box::new(AsyncFnToolExecutor {
            effect: ToolEffect::READ,
            validate: Arc::new(|_| ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            }),
            invoke: Arc::new(move |tool, invocationContext| {
                let searchHandler = invocationContext.host().clone();
                let searchContext = searchContext.clone();
                let searchPackageManager = searchPackageManager.clone();
                let searchRuntimeSupport = searchRuntimeSupport.clone();
                Box::pin(async move {
                    let useEnglish = false;
                    let context = match ToolExecutionManager::currentToolRuntimeContext() {
                        Some(context)
                            if context.toolExposureMode
                                == operit_tools::ToolExecutionManager::ToolExposureMode::CLI =>
                        {
                            context
                        }
                        _ => {
                            return toolErrorResult(
                                &tool,
                                CliToolModeSupport::buildCliModeUnavailableMessage(useEnglish),
                            )
                        }
                    };
                    let query = requiredParameterValue(&tool, "query");
                    if query.trim().is_empty() {
                        return toolErrorResult(
                            &tool,
                            "Missing required parameter: query".to_string(),
                        );
                    }
                    let limit = match tool
                        .parameters
                        .iter()
                        .find(|parameter| parameter.name == "limit")
                    {
                        Some(parameter) => match parameter.value.parse::<i32>() {
                            Ok(value) if value > 0 => value,
                            _ => {
                                return toolErrorResult(
                                    &tool,
                                    "limit must be a positive integer".to_string(),
                                )
                            }
                        },
                        None => CliToolModeSupport::defaultSearchLimit(),
                    };
                    let manager =
                        match RuntimePackageManager::readySnapshot(searchPackageManager).await {
                            Ok(manager) => manager,
                            Err(error) => return toolErrorResult(&tool, error),
                        };
                    let registeredToolNames = searchHandler.getAllToolNames().into_iter().collect();
                    let catalog = match CliToolModeSupport::buildHiddenToolCatalog(
                        &searchContext,
                        &manager,
                        useEnglish,
                        &searchContext.hostEnvironment,
                        searchRuntimeSupport.as_ref(),
                        &registeredToolNames,
                    ) {
                        Ok(catalog) => catalog,
                        Err(error) => return toolErrorResult(&tool, error),
                    };
                    let catalog = match searchRuntimeSupport
                        .filterToolCatalog(catalog, context)
                        .await
                    {
                        Ok(catalog) => catalog,
                        Err(error) => return toolErrorResult(&tool, error),
                    };
                    let results =
                        CliToolModeSupport::searchHiddenToolCatalog(&catalog, &query, limit);
                    ToolResult {
                        toolName: tool.name.clone(),
                        success: true,
                        result: stringResultData(CliToolModeSupport::formatSearchResults(
                            &query, &results, useEnglish,
                        )),
                        error: None,
                    }
                })
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
    registerChatTool(handler, &chatTools, BuiltinToolName::UpdateChatGroup, ChatManagerToolOperation::UpdateChatGroup);
    registerChatTool(
        handler, &chatTools, BuiltinToolName::UpdateChatPinned,
        ChatManagerToolOperation::UpdateChatPinned,
    );
    registerChatTool(
        handler, &chatTools, BuiltinToolName::UpdateChatLocked,
        ChatManagerToolOperation::UpdateChatLocked,
    );
    registerChatTool(
        handler, &chatTools, BuiltinToolName::ReorderChats,
        ChatManagerToolOperation::ReorderChats,
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
        BuiltinToolName::CallChatModel,
        ChatManagerToolOperation::CallChatModel,
    );
    registerChatTool(
        handler,
        &chatTools,
        BuiltinToolName::GetChatMessages,
        ChatManagerToolOperation::GetChatMessages,
    );
    registerChatTool(
        handler,
        &chatTools,
        BuiltinToolName::GetChatMessagesRange,
        ChatManagerToolOperation::GetChatMessagesRange,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::ChatSendMessage,
        RegisteredToolExecutor::Shared(Arc::new(PluginChatToolExecutor { operation: PluginChatToolOperation::SendMessage })),
        ToolRegistrationVisibility::INTERNAL,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::ChatObserve,
        RegisteredToolExecutor::Shared(Arc::new(PluginChatToolExecutor { operation: PluginChatToolOperation::Observe })),
        ToolRegistrationVisibility::INTERNAL,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::ChatCancel,
        RegisteredToolExecutor::Shared(Arc::new(PluginChatToolExecutor { operation: PluginChatToolOperation::Cancel })),
        ToolRegistrationVisibility::INTERNAL,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::ChatReadExtension,
        RegisteredToolExecutor::Shared(Arc::new(PluginChatToolExecutor { operation: PluginChatToolOperation::ReadExtension })),
        ToolRegistrationVisibility::INTERNAL,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::ChatWriteExtension,
        RegisteredToolExecutor::Shared(Arc::new(PluginChatToolExecutor { operation: PluginChatToolOperation::WriteExtension })),
        ToolRegistrationVisibility::INTERNAL,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::ChatDeleteExtension,
        RegisteredToolExecutor::Shared(Arc::new(PluginChatToolExecutor { operation: PluginChatToolOperation::DeleteExtension })),
        ToolRegistrationVisibility::INTERNAL,
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
        crate::ToolExecutionManager::RegisteredToolExecutor::Asynchronous(Box::new(
            ChatManagerToolExecutor {
                tools: chatTools.clone(),
                operation,
            },
        )),
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

/// Registers terminal tools with the same host execution and scheduling context.
#[allow(non_snake_case)]
fn registerTerminalTools(handler: &mut AIToolHandler, terminalTools: StandardTerminalTools) {
    handler.registerBuiltinTool(
        BuiltinToolName::TerminalObserve,
        operit_tools::ToolExecutionManager::RegisteredToolExecutor::Shared(Arc::new(
            operit_tools::tools::defaultTool::standard::StandardTerminalStreaming::TerminalStreamExecutor::new(
                terminalTools.terminalHost.clone(),
                handler.getContext().hostRuntimeTaskSchedulerHost.clone(),
            ),
        )),
        ToolRegistrationVisibility::PUBLIC,
    );
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
    handler.registerBuiltinTool(BuiltinToolName::PackageProxy, crate::ToolExecutionManager::RegisteredToolExecutor::Alias,
        ToolRegistrationVisibility::INTERNAL);
    handler.registerBuiltinTool(
        BuiltinToolName::EdgeExecute,
        crate::ToolExecutionManager::RegisteredToolExecutor::Asynchronous(Box::new(
            crate::tools::EdgePortTool::EdgePortToolExecutor { io: false, runtime: handler.runtimeSupport() })),
        ToolRegistrationVisibility::INTERNAL,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::IoExecute,
        crate::ToolExecutionManager::RegisteredToolExecutor::Asynchronous(Box::new(
            crate::tools::EdgePortTool::EdgePortToolExecutor { io: true, runtime: handler.runtimeSupport() })),
        ToolRegistrationVisibility::INTERNAL,
    );
    let readEnvironmentRuntimeSupport = handler.runtimeSupport();
    registerHttpTools(handler, ToolGetter::getHttpTools(context));
    if let Some(browserTools) = ToolGetter::getBrowserAutomationTools(context) {
        registerBrowserAutomationTools(handler, browserTools);
    }

    handler.registerBuiltinTool(
        BuiltinToolName::ExecuteCliCommand,
        crate::ToolExecutionManager::RegisteredToolExecutor::Asynchronous(Box::new(
            AsyncFnToolExecutor {
                effect: ToolEffect::WRITE,
                validate: Arc::new(|_| ToolValidationResult {
                    valid: true,
                    errorMessage: String::new(),
                }),
                invoke: Arc::new(move |tool, invocationContext| {
                    let cliCommandHandler = invocationContext.host().clone();
                    Box::pin(async move {
                        let argsRaw = requiredParameterValue(&tool, "args");
                        let args = match serde_json::from_str::<Vec<String>>(&argsRaw) {
                            Ok(args) => args,
                            Err(error) => {
                                return toolErrorResult(
                                    &tool,
                                    format!("args must be a JSON string array: {error}"),
                                );
                            }
                        };
                        let context = cliCommandHandler.getContext();
                        let Some(executor) = context.coreCommandExecutor else {
                            return toolErrorResult(
                                &tool,
                                "Core command executor is not configured.".to_string(),
                            );
                        };
                        match executor(args).await {
                            Ok(output) => ToolResult {
                                toolName: tool.name.clone(),
                                success: true,
                                result: stringResultData(output),
                                error: None,
                            },
                            Err(error) => toolErrorResult(&tool, error),
                        }
                    })
                }),
            },
        )),
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
    registerModelTools(handler);
    registerMediaTools(handler);
    registerTtsTools(handler);
    registerThemeTools(handler);
    registerCatalogTools(handler);
}

#[allow(non_snake_case)]
fn registerModelTools(handler: &mut AIToolHandler) {
    handler.registerBuiltinTool(
        BuiltinToolName::ModelsListSummaries,
        RegisteredToolExecutor::Shared(Arc::new(ModelToolExecutor)),
        ToolRegistrationVisibility::INTERNAL,
    );
}

#[allow(non_snake_case)]
fn registerMediaTools(handler: &mut AIToolHandler) {
    handler.registerBuiltinTool(
        BuiltinToolName::MediaRegister,
        RegisteredToolExecutor::Shared(Arc::new(RegisterMediaExecutor)),
        ToolRegistrationVisibility::INTERNAL,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::MediaRegisterFromPath,
        RegisteredToolExecutor::Shared(Arc::new(RegisterMediaFromPathExecutor)),
        ToolRegistrationVisibility::INTERNAL,
    );
}

#[allow(non_snake_case)]
fn registerTtsTools(handler: &mut AIToolHandler) {
    handler.registerBuiltinTool(
        BuiltinToolName::TtsSynthesize,
        RegisteredToolExecutor::Shared(Arc::new(TtsToolExecutor { operation: TtsToolOperation::Synthesize })),
        ToolRegistrationVisibility::INTERNAL,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::SttTranscribe,
        RegisteredToolExecutor::Shared(Arc::new(SttToolExecutor)),
        ToolRegistrationVisibility::INTERNAL,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::TtsListConfigs,
        RegisteredToolExecutor::Shared(Arc::new(TtsToolExecutor {
            operation: TtsToolOperation::ListConfigs,
        })),
        ToolRegistrationVisibility::INTERNAL,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::TtsGetCurrentConfigId,
        RegisteredToolExecutor::Shared(Arc::new(TtsToolExecutor {
            operation: TtsToolOperation::GetCurrentConfigId,
        })),
        ToolRegistrationVisibility::INTERNAL,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::TtsSetCurrentConfigId,
        RegisteredToolExecutor::Shared(Arc::new(TtsToolExecutor {
            operation: TtsToolOperation::SetCurrentConfigId,
        })),
        ToolRegistrationVisibility::INTERNAL,
    );
}

#[allow(non_snake_case)]
fn registerThemeTools(handler: &mut AIToolHandler) {
    handler.registerBuiltinTool(
        BuiltinToolName::ThemeListConfigs,
        RegisteredToolExecutor::Shared(Arc::new(ThemeToolExecutor {
            operation: ThemeToolOperation::ListConfigs,
        })),
        ToolRegistrationVisibility::INTERNAL,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::ThemeApplyConfig,
        RegisteredToolExecutor::Shared(Arc::new(ThemeToolExecutor {
            operation: ThemeToolOperation::ApplyConfig,
        })),
        ToolRegistrationVisibility::INTERNAL,
    );
}

#[allow(non_snake_case)]
fn registerCatalogTools(handler: &mut AIToolHandler) {
    handler.registerBuiltinTool(
        BuiltinToolName::CatalogReadToolSources,
        RegisteredToolExecutor::Shared(Arc::new(CatalogToolExecutor)),
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
        RegisteredToolExecutor::Asynchronous(Box::new(FileSystemToolExecutor {
            tools: fileSystemTools.clone(),
            operation: FileSystemToolOperation::ApplyFile,
        })),
        ToolRegistrationVisibility::INTERNAL,
    );
    handler.registerBuiltinTool(
        BuiltinToolName::StorageRequest,
        RegisteredToolExecutor::Shared(Arc::new(StorageToolExecutor)),
        ToolRegistrationVisibility::INTERNAL,
    );
    registerFileTransferTools(handler, &fileSystemTools);

}

/// Registers the internal cross-node transfer endpoints. They are transport
/// plumbing for `node:` copies and never appear in any model-facing catalog.
#[allow(non_snake_case)]
fn registerFileTransferTools(
    handler: &mut AIToolHandler,
    fileSystemTools: &StandardFileSystemTools,
) {
    use crate::tools::transfer::FileTransferTools::{
        FileTransferExecutor, FileTransferOperation, FS_PUSH_FILE_TOOL, FS_READ_CHUNK_TOOL,
        FS_TRANSFER_BEGIN_TOOL, FS_TRANSFER_CHUNK_TOOL, FS_TRANSFER_COMMIT_TOOL,
    };
    let endpoints = [
        (FS_TRANSFER_BEGIN_TOOL, FileTransferOperation::Begin),
        (FS_TRANSFER_CHUNK_TOOL, FileTransferOperation::Chunk),
        (FS_TRANSFER_COMMIT_TOOL, FileTransferOperation::Commit),
        (FS_READ_CHUNK_TOOL, FileTransferOperation::ReadChunk),
        (FS_PUSH_FILE_TOOL, FileTransferOperation::PushFile),
    ];
    for (name, operation) in endpoints {
        handler.registerInternalAsyncTool(
            name.to_string(),
            Box::new(FileTransferExecutor {
                tools: fileSystemTools.clone(),
                operation,
            }),
        );
    }
}

#[allow(non_snake_case)]
/// Registers file tools through the asynchronous Host execution contract.
fn registerFileSystemTool(
    handler: &mut AIToolHandler,
    fileSystemTools: &StandardFileSystemTools,
    name: BuiltinToolName,
    operation: FileSystemToolOperation,
) {
    // File tools own an asynchronous execution contract: qualified `node:` paths
    // dispatch to the target device through the device-tool transport.
    handler.registerBuiltinTool(
        name,
        RegisteredToolExecutor::Asynchronous(Box::new(FileSystemToolExecutor {
            tools: fileSystemTools.clone(),
            operation,
        })),
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

/// Builds an explicit failure result for tool validation or execution errors.
fn toolErrorResult(tool: &AITool, error: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result: stringResultData(""),
        error: Some(error),
    }
}

/// Validates one switch_core target against the live route state so an invalid
/// target fails the tool call instead of the asynchronous Binding move.
///
/// The caller receives the resolved member so the emitted marker always carries
/// the canonical node id, never the alias that was typed.
#[allow(non_snake_case)]
fn resolveSwitchCoreTarget<'a>(
    state: &'a crate::runtime_support::RuntimeCoreNodeRouteState,
    targetReference: &str,
) -> Result<&'a crate::runtime_support::RuntimeCoreNodeStatus, String> {
    use crate::runtime_support::{
        CoreNodeReferenceMatch, coreNodeReferenceLabel, formatCoreNodeReferenceCandidates,
    };
    match state.resolveNodeReference(targetReference) {
        CoreNodeReferenceMatch::Resolved(node) => {
            if !node.canExecute {
                return Err(format!(
                    "switch_core target cannot host Core execution: {}",
                    coreNodeReferenceLabel(node)
                ));
            }
            if !node.reachable {
                return Err(format!(
                    "switch_core target is not currently reachable: {}",
                    coreNodeReferenceLabel(node)
                ));
            }
            Ok(node)
        }
        CoreNodeReferenceMatch::Unknown => Err(format!(
            "switch_core target is not a member of the current device space: {targetReference}"
        )),
        CoreNodeReferenceMatch::Ambiguous(candidates) => Err(format!(
            "switch_core target is ambiguous: {targetReference}; candidates: {}",
            formatCoreNodeReferenceCandidates(&candidates)
        )),
    }
}

#[cfg(test)]
mod switch_core_tests {
    use super::*;

    fn routeState(nodes: Vec<(&str, &str, &str, bool)>) -> crate::runtime_support::RuntimeCoreNodeRouteState {
        operit_tools::runtime_support::RuntimeCoreNodeRouteState {
            currentNodeId: "local".to_string(),
            nodes: nodes
                .into_iter()
                .map(|(nodeId, displayName, userName, reachable)| crate::runtime_support::RuntimeCoreNodeStatus {
                    nodeId: nodeId.to_string(),
                    displayName: displayName.to_string(),
                    userName: userName.to_string(),
                    platform: "linux".to_string(),
                    model: String::new(),
                    reachable,
                    canExecute: true,
                    relayHops: None,
                    relayPath: None,
                })
                .collect(),
        }
    }

    #[test]
    fn switch_core_rejects_reachable_devices_without_execution_capacity() {
        let mut state = routeState(vec![("esp-relay", "desk-relay", "owner", true)]);
        state.nodes[0].canExecute = false;
        for reference in ["esp-relay", "desk-relay", "owner", "esp-"] {
            let error = resolveSwitchCoreTarget(&state, reference).unwrap_err();
            assert!(error.contains("cannot host Core execution"), "{error}");
            assert!(error.contains("esp-relay"), "{error}");
        }
    }

    #[test]
    fn switch_core_accepts_reachable_members_including_direct_peers() {
        let state = routeState(vec![
            ("local", "local", "tester", true),
            ("direct-peer", "direct-peer", "tester", true),
            ("relayed", "relayed", "tester", true),
        ]);
        assert_eq!(resolveSwitchCoreTarget(&state, "direct-peer").unwrap().nodeId, "direct-peer");
        assert_eq!(resolveSwitchCoreTarget(&state, "local").unwrap().nodeId, "local");
    }

    #[test]
    fn switch_core_rejects_unknown_and_unreachable_targets() {
        let state = routeState(vec![
            ("local", "local", "tester", true),
            ("core-1791589326795-13765763d4b74d4ab76648f26ee874be", "offline-cas", "tester", false),
        ]);
        assert!(resolveSwitchCoreTarget(&state, "stranger")
            .expect_err("unknown target must be rejected")
            .contains("not a member"));
        let unreachable = resolveSwitchCoreTarget(&state, "offline-cas")
            .expect_err("unreachable target must be rejected");
        assert!(unreachable.contains("not currently reachable"));
        assert!(unreachable.contains("core-1791589326795"), "canonical id must be reported: {unreachable}");
    }

    /// Device names, user names, and unique id prefixes resolve to the canonical id.
    #[test]
    fn switch_core_resolves_device_references_to_canonical_ids() {
        let cas = "core-1791589326795-13765763d4b74d4ab76648f26ee874be";
        let elish = "core-1791589400000-58f0d4ee0efc47f59a1f1b4d514b7af0";
        let state = routeState(vec![
            ("local", "local", "tester", true),
            (cas, "cas", "harmoon", true),
            (elish, "Elish", "lin", true),
        ]);
        for reference in ["cas", "harmoon", "core-1791589326795"] {
            assert_eq!(resolveSwitchCoreTarget(&state, reference).unwrap().nodeId, cas, "{reference}");
        }
        assert_eq!(resolveSwitchCoreTarget(&state, "elish").unwrap().nodeId, elish);
        assert_eq!(resolveSwitchCoreTarget(&state, "lin").unwrap().nodeId, elish);
    }

    /// Repeated names must fail with candidates instead of guessing a device.
    #[test]
    fn switch_core_reports_ambiguous_references() {
        let state = routeState(vec![
            ("core-1", "cas", "harmoon", true),
            ("core-2", "cas", "harmoon", true),
        ]);
        let error = resolveSwitchCoreTarget(&state, "cas").expect_err("ambiguous name must be rejected");
        assert!(error.contains("ambiguous"), "{error}");
        assert!(error.contains("cas (core-1)"), "{error}");
        assert!(error.contains("cas (core-2)"), "{error}");
    }
}
