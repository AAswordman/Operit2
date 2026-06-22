use std::sync::Arc;

use operit_core_proxy::LocalCoreProxy;
#[cfg(target_os = "linux")]
use operit_host_linux_native::{
    LinuxAudioPlaybackHost as NativeAudioPlaybackHost,
    LinuxBrowserAutomationHost as NativeBrowserAutomationHost,
    LinuxFileSystemHost as NativeFileSystemHost, LinuxHostRuntimeEventHost as NativeHostRuntimeEventHost,
    LinuxHttpHost as NativeHttpHost,
    LinuxManagedRuntimeHost as NativeManagedRuntimeHost,
    LinuxRuntimeStorageHost as NativeRuntimeStorageHost,
    LinuxSystemOperationHost as NativeSystemOperationHost, LinuxTerminalHost as NativeTerminalHost,
    LinuxWebVisitHost as NativeWebVisitHost,
};
#[cfg(windows)]
use operit_host_windows_native::{
    WindowsAudioPlaybackHost as NativeAudioPlaybackHost,
    WindowsBrowserAutomationHost as NativeBrowserAutomationHost,
    WindowsFileSystemHost as NativeFileSystemHost,
    WindowsHostRuntimeEventHost as NativeHostRuntimeEventHost, WindowsHttpHost as NativeHttpHost,
    WindowsManagedRuntimeHost as NativeManagedRuntimeHost,
    WindowsRuntimeStorageHost as NativeRuntimeStorageHost,
    WindowsSystemOperationHost as NativeSystemOperationHost,
    WindowsTerminalHost as NativeTerminalHost, WindowsWebVisitHost as NativeWebVisitHost,
};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;

#[cfg(not(any(windows, target_os = "linux")))]
compile_error!("operit2 CLI host is implemented for Windows and Linux.");

pub(crate) fn create_cli_application() -> OperitApplication {
    let runtimeStorageHost = Arc::new(NativeRuntimeStorageHost::new(
        NativeRuntimeStorageHost::defaultRoot(),
    ));
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let mut context =
        OperitApplicationContext::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
            Arc::new(NativeFileSystemHost::new()),
            Arc::new(NativeWebVisitHost::new()),
            Arc::new(NativeHttpHost::new()),
            Arc::new(NativeSystemOperationHost::new()),
            Arc::new(NativeManagedRuntimeHost::new()),
            runtimeStorageHost,
            runtimeSqliteHost,
        );
    #[cfg(any(target_os = "linux", windows))]
    {
        context = context.withTerminalHost(Arc::new(NativeTerminalHost::new()));
    }
    context = context.withAudioPlaybackHost(Arc::new(NativeAudioPlaybackHost::new()));
    context = context.withHostRuntimeEventHost(Arc::new(NativeHostRuntimeEventHost::new()));
    context = context.withBrowserAutomationHost(Arc::new(NativeBrowserAutomationHost::new()));
    let commandContext = context.clone();
    OperitApplication::newWithContext(context.withCoreCommandExecutor(Arc::new(move |args| {
        let output =
            operit_command_core::run_core_command_with_context(commandContext.clone(), &args)?;
        Ok(output.stdout)
    })))
}

pub(crate) fn create_local_core() -> LocalCoreProxy {
    LocalCoreProxy::new(create_cli_application())
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_runtime::api::chat::enhance::ToolExecutionManager::{AITool, ToolParameter};
    use operit_runtime::core::tools::AIToolHandler::AIToolHandler;
    use operit_runtime::core::tools::ToolResultDataClasses::ToolResultData;

    #[test]
    fn direct_terminal_tool_chain_executes_visible_terminal() {
        let application = create_cli_application();
        let mut handler = AIToolHandler::getInstance(application.applicationContext.clone());
        handler.registerDefaultTools();

        #[cfg(windows)]
        let (sessionName, command, expectedOutput) = (
            "direct-tool-visible-powershell",
            "Write-Output direct-tool-ok",
            "direct-tool-ok",
        );
        #[cfg(target_os = "linux")]
        let (sessionName, command, expectedOutput) = (
            "direct-tool-visible-linux",
            "printf 'direct-tool-ok\\n'; [ -t 0 ] && echo tty=yes || echo tty=no",
            "direct-tool-ok\ntty=yes",
        );

        let createResult = handler.executeTool(AITool {
            name: "create_terminal_session".to_string(),
            parameters: vec![ToolParameter {
                name: "session_name".to_string(),
                value: sessionName.to_string(),
            }],
        });
        assert!(createResult.success, "{:?}", createResult.error);
        let sessionId = match createResult.result {
            ToolResultData::TerminalSessionCreationResultData(data) => data.sessionId,
            data => panic!("create result data type mismatch: {}", data.toJson()),
        };

        let executeResult = handler.executeTool(AITool {
            name: "execute_in_terminal_session".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "session_id".to_string(),
                    value: sessionId.clone(),
                },
                ToolParameter {
                    name: "command".to_string(),
                    value: command.to_string(),
                },
                ToolParameter {
                    name: "timeout_ms".to_string(),
                    value: "3000".to_string(),
                },
            ],
        });
        assert!(executeResult.success, "{:?}", executeResult.error);
        match executeResult.result {
            ToolResultData::TerminalCommandResultData(data) => {
                assert_eq!(data.output, expectedOutput);
                assert_eq!(data.exitCode, 0);
                assert_eq!(data.timedOut, false);
            }
            data => panic!("execute result data type mismatch: {}", data.toJson()),
        }

        let screenResult = handler.executeTool(AITool {
            name: "get_terminal_session_screen".to_string(),
            parameters: vec![ToolParameter {
                name: "session_id".to_string(),
                value: sessionId,
            }],
        });
        assert!(screenResult.success, "{:?}", screenResult.error);
        match screenResult.result {
            ToolResultData::TerminalSessionScreenResultData(data) => {
                assert_eq!(data.commandRunning, false);
                assert!(!data.content.is_empty());
            }
            data => panic!("screen result data type mismatch: {}", data.toJson()),
        }
    }
}
