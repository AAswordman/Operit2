use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::{
    AITool, ToolExecutionManager, ToolParameter, ToolValidationResult,
};
use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::climode::CliToolModeSupport::{
    CliToolModeSupport, PACKAGE_PROXY_TOOL_NAME, PROXY_TOOL_NAME, SEARCH_TOOL_NAME,
};
use crate::core::tools::defaultTool::standard::StandardBrowserAutomationTools::{
    BrowserAutomationToolExecutor, StandardBrowserAutomationTools,
};
use crate::core::tools::defaultTool::standard::StandardChatManagerTool::{
    ChatManagerToolExecutor, ChatManagerToolOperation, StandardChatManagerTool,
};
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
use crate::core::tools::defaultTool::standard::StandardTerminalTools::{
    StandardTerminalTools, TerminalToolExecutor, TerminalToolOperation,
};
use crate::core::tools::defaultTool::ToolGetter::ToolGetter;
use crate::core::tools::mcp::MCPManager::MCPManager;
use crate::core::tools::mcp::MCPToolExecutor::MCPToolExecutor;
use crate::core::tools::packTool::PackageManager::PackageManager;
use crate::core::tools::AIToolHandler::{AIToolHandler, FnToolExecutor};
use crate::core::tools::ToolPackage::{PackageToolExecutor, ToolPackage};
use crate::data::preferences::EnvPreferences::EnvPreferences;

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
                std::thread::sleep(std::time::Duration::from_millis(sleptMs as u64));
                ToolResult {
                    toolName: tool.name.clone(),
                    success: true,
                    result: serde_json::json!({
                        "requestedMs": durationMs,
                        "sleptMs": sleptMs
                    })
                    .to_string(),
                    error: None,
                }
            }),
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
    registerChatTools(handler, StandardChatManagerTool::new());

    let packageManager = handler.getOrCreatePackageManager();
    let usePackageManager = packageManager.clone();
    let usePackageHandler = handler.clone();
    handler.registerTool(
        "use_package".to_string(),
        Box::new(FnToolExecutor {
            name: "use_package".to_string(),
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
    );
    let searchContext = context.clone();
    let searchPackageManager = packageManager.clone();
    handler.registerTool(
        SEARCH_TOOL_NAME.to_string(),
        Box::new(FnToolExecutor {
            name: SEARCH_TOOL_NAME.to_string(),
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

                let hostEnvironment = searchContext.hostEnvironment.clone();
                let packageManagerGuard = searchPackageManager
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
                    &searchContext,
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
            }),
        }),
    );
    let proxyHandler = handler.clone();
    handler.registerTool(
        PROXY_TOOL_NAME.to_string(),
        Box::new(FnToolExecutor {
            name: PROXY_TOOL_NAME.to_string(),
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

                let packageManager = proxyHandler.getOrCreatePackageManager();
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
                let permissionSystem = proxyHandler.getToolPermissionSystem();
                let hasPermission = match permissionSystem.checkToolPermission(&proxiedTool) {
                    Ok(value) => value,
                    Err(_) => false,
                };
                if !hasPermission {
                    let errorMessage = "User cancelled the tool execution.".to_string();
                    proxyHandler.notifyToolPermissionChecked(&proxiedTool, false, Some(&errorMessage));
                    return ToolResult {
                        toolName: proxiedTool.name,
                        success: false,
                        result: String::new(),
                        error: Some(errorMessage),
                    };
                }

                proxyHandler.notifyToolPermissionChecked(&proxiedTool, true, None);
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
        "start_chat_service",
        ChatManagerToolOperation::StartChatService,
    );
    registerChatTool(
        handler,
        &chatTools,
        "stop_chat_service",
        ChatManagerToolOperation::StopChatService,
    );
    registerChatTool(
        handler,
        &chatTools,
        "create_new_chat",
        ChatManagerToolOperation::CreateNewChat,
    );
    registerChatTool(
        handler,
        &chatTools,
        "list_chats",
        ChatManagerToolOperation::ListChats,
    );
    registerChatTool(
        handler,
        &chatTools,
        "find_chat",
        ChatManagerToolOperation::FindChat,
    );
    registerChatTool(
        handler,
        &chatTools,
        "agent_status",
        ChatManagerToolOperation::AgentStatus,
    );
    registerChatTool(
        handler,
        &chatTools,
        "switch_chat",
        ChatManagerToolOperation::SwitchChat,
    );
    registerChatTool(
        handler,
        &chatTools,
        "update_chat_title",
        ChatManagerToolOperation::UpdateChatTitle,
    );
    registerChatTool(
        handler,
        &chatTools,
        "delete_chat",
        ChatManagerToolOperation::DeleteChat,
    );
    registerChatTool(
        handler,
        &chatTools,
        "send_message_to_ai",
        ChatManagerToolOperation::SendMessageToAi,
    );
    registerChatTool(
        handler,
        &chatTools,
        "send_message_to_ai_streaming",
        ChatManagerToolOperation::SendMessageToAiStreaming,
    );
    registerChatTool(
        handler,
        &chatTools,
        "list_character_cards",
        ChatManagerToolOperation::ListCharacterCards,
    );
    registerChatTool(
        handler,
        &chatTools,
        "get_chat_messages",
        ChatManagerToolOperation::GetChatMessages,
    );
}

#[allow(non_snake_case)]
fn registerChatTool(
    handler: &mut AIToolHandler,
    chatTools: &StandardChatManagerTool,
    name: &str,
    operation: ChatManagerToolOperation,
) {
    handler.registerTool(
        name.to_string(),
        Box::new(ChatManagerToolExecutor {
            tools: chatTools.clone(),
            operation,
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
fn registerTerminalTools(handler: &mut AIToolHandler, terminalTools: StandardTerminalTools) {
    registerTerminalTool(
        handler,
        &terminalTools,
        "get_terminal_info",
        TerminalToolOperation::GetTerminalInfo,
    );
    registerTerminalTool(
        handler,
        &terminalTools,
        "create_terminal_session",
        TerminalToolOperation::CreateSession,
    );
    registerTerminalTool(
        handler,
        &terminalTools,
        "execute_in_terminal_session",
        TerminalToolOperation::ExecuteInSession,
    );
    registerTerminalTool(
        handler,
        &terminalTools,
        "execute_in_terminal_session_streaming",
        TerminalToolOperation::ExecuteInSessionStreaming,
    );
    registerTerminalTool(
        handler,
        &terminalTools,
        "execute_hidden_terminal_command",
        TerminalToolOperation::ExecuteHiddenCommand,
    );
    registerTerminalTool(
        handler,
        &terminalTools,
        "close_terminal_session",
        TerminalToolOperation::CloseSession,
    );
    registerTerminalTool(
        handler,
        &terminalTools,
        "input_in_terminal_session",
        TerminalToolOperation::InputInSession,
    );
    registerTerminalTool(
        handler,
        &terminalTools,
        "get_terminal_session_screen",
        TerminalToolOperation::GetSessionScreen,
    );
}

#[allow(non_snake_case)]
fn registerTerminalTool(
    handler: &mut AIToolHandler,
    terminalTools: &StandardTerminalTools,
    name: &str,
    operation: TerminalToolOperation,
) {
    handler.registerTool(
        name.to_string(),
        Box::new(TerminalToolExecutor {
            tools: terminalTools.clone(),
            operation,
        }),
    );
}

#[allow(non_snake_case)]
fn registerInternalTools(handler: &mut AIToolHandler, context: &OperitApplicationContext) {
    registerHttpTools(handler, ToolGetter::getHttpTools(context));
    if let Some(browserTools) = ToolGetter::getBrowserAutomationTools(context) {
        registerBrowserAutomationTools(handler, browserTools);
    }
    registerMemoryInternalTools(handler);

    if let Some(fileSystemTools) = ToolGetter::getFileSystemTools(context) {
        handler.registerInternalTool(
            "apply_file".to_string(),
            Box::new(FileSystemToolExecutor {
                tools: fileSystemTools,
                operation: FileSystemToolOperation::ApplyFile,
            }),
        );
    }

    let packageProxyHandler = handler.clone();
    handler.registerInternalTool(
        "package_proxy".to_string(),
        Box::new(FnToolExecutor {
            name: "package_proxy".to_string(),
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
    );
    let cliCommandHandler = handler.clone();
    handler.registerInternalTool(
        "execute_cli_command".to_string(),
        Box::new(FnToolExecutor {
            name: "execute_cli_command".to_string(),
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
                        result: output,
                        error: None,
                    },
                    Err(error) => toolErrorResult(tool, error),
                }
            }),
        }),
    );
    handler.registerInternalTool(
        "read_environment_variable".to_string(),
        Box::new(FnToolExecutor {
            name: "read_environment_variable".to_string(),
            validate: Arc::new(|_| ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            }),
            invoke: Arc::new(|tool| {
                let key = requiredParameterValue(tool, "key");
                let envPreferences = EnvPreferences::getInstance();
                match envPreferences.getEnv(&key) {
                    Ok(value) => ToolResult {
                        toolName: tool.name.clone(),
                        success: true,
                        result: serde_json::json!({
                            "key": key,
                            "value": value,
                            "exists": value.is_some()
                        })
                        .to_string(),
                        error: None,
                    },
                    Err(error) => ToolResult {
                        toolName: tool.name.clone(),
                        success: false,
                        result: serde_json::json!({
                            "key": key,
                            "value": null,
                            "exists": false
                        })
                        .to_string(),
                        error: Some(error.to_string()),
                    },
                }
            }),
        }),
    );
    handler.registerInternalTool(
        "write_environment_variable".to_string(),
        Box::new(FnToolExecutor {
            name: "write_environment_variable".to_string(),
            validate: Arc::new(|_| ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            }),
            invoke: Arc::new(|tool| {
                let key = requiredParameterValue(tool, "key");
                let value = tool
                    .parameters
                    .iter()
                    .find(|parameter| parameter.name == "value")
                    .map(|parameter| parameter.value.clone())
                    .unwrap_or_default();
                let envPreferences = EnvPreferences::getInstance();
                let writeResult = if value.trim().is_empty() {
                    envPreferences.removeEnv(&key)
                } else {
                    envPreferences.setEnv(&key, value.trim())
                };
                if let Err(error) = writeResult {
                    return ToolResult {
                        toolName: tool.name.clone(),
                        success: false,
                        result: serde_json::json!({
                            "key": key,
                            "requestedValue": value,
                            "value": null,
                            "exists": false,
                            "cleared": value.trim().is_empty()
                        })
                        .to_string(),
                        error: Some(error.to_string()),
                    };
                }

                match envPreferences.getEnv(&key) {
                    Ok(current) => ToolResult {
                        toolName: tool.name.clone(),
                        success: true,
                        result: serde_json::json!({
                            "key": key,
                            "requestedValue": value,
                            "value": current,
                            "exists": current.is_some(),
                            "cleared": value.trim().is_empty()
                        })
                        .to_string(),
                        error: None,
                    },
                    Err(error) => ToolResult {
                        toolName: tool.name.clone(),
                        success: false,
                        result: serde_json::json!({
                            "key": key,
                            "requestedValue": value,
                            "value": null,
                            "exists": false,
                            "cleared": value.trim().is_empty()
                        })
                        .to_string(),
                        error: Some(error.to_string()),
                    },
                }
            }),
        }),
    );
}

#[allow(non_snake_case)]
fn registerBrowserAutomationTools(
    handler: &mut AIToolHandler,
    browserTools: StandardBrowserAutomationTools,
) {
    for name in [
        "browser_click",
        "browser_close",
        "browser_close_all",
        "browser_console_messages",
        "browser_drag",
        "browser_evaluate",
        "browser_file_upload",
        "browser_fill_form",
        "browser_handle_dialog",
        "browser_hover",
        "browser_navigate",
        "browser_navigate_back",
        "browser_network_requests",
        "browser_press_key",
        "browser_resize",
        "browser_run_code",
        "browser_select_option",
        "browser_snapshot",
        "browser_tabs",
        "browser_take_screenshot",
        "browser_type",
        "browser_wait_for",
    ] {
        handler.registerInternalTool(
            name.to_string(),
            Box::new(BrowserAutomationToolExecutor {
                tools: browserTools.clone(),
            }),
        );
    }
}

#[allow(non_snake_case)]
fn registerMemoryPublicTools(handler: &mut AIToolHandler) {
    registerMemoryTool(
        handler,
        "query_memory",
        MemoryToolOperation::QueryMemory,
        false,
    );
    registerMemoryTool(
        handler,
        "get_memory_by_title",
        MemoryToolOperation::GetMemoryByTitle,
        false,
    );
}

#[allow(non_snake_case)]
fn registerMemoryInternalTools(handler: &mut AIToolHandler) {
    registerMemoryTool(
        handler,
        "create_memory",
        MemoryToolOperation::CreateMemory,
        true,
    );
    registerMemoryTool(
        handler,
        "update_memory",
        MemoryToolOperation::UpdateMemory,
        true,
    );
    registerMemoryTool(
        handler,
        "delete_memory",
        MemoryToolOperation::DeleteMemory,
        true,
    );
    registerMemoryTool(
        handler,
        "move_memory",
        MemoryToolOperation::MoveMemory,
        true,
    );
    registerMemoryTool(
        handler,
        "update_user_preferences",
        MemoryToolOperation::UpdateUserPreferences,
        true,
    );
    registerMemoryTool(
        handler,
        "link_memories",
        MemoryToolOperation::LinkMemories,
        true,
    );
    registerMemoryTool(
        handler,
        "query_memory_links",
        MemoryToolOperation::QueryMemoryLinks,
        true,
    );
    registerMemoryTool(
        handler,
        "update_memory_link",
        MemoryToolOperation::UpdateMemoryLink,
        true,
    );
    registerMemoryTool(
        handler,
        "delete_memory_link",
        MemoryToolOperation::DeleteMemoryLink,
        true,
    );
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
        "grep_context",
        FileSystemToolOperation::GrepContext,
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
        result: String::new(),
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
