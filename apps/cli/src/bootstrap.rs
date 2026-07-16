use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use operit_core_proxy::LocalCoreProxy;
use operit_host_api::HostManager::HostManager;
#[cfg(target_os = "macos")]
use operit_host_apple_native::{
    AppleBrowserAutomationHost as NativeBrowserAutomationHost,
    AppleFileSystemHost as NativeFileSystemHost,
    AppleHostRuntimeEventHost as NativeHostRuntimeEventHost,
    AppleHostRuntimeEventSchedulerHost as NativeHostRuntimeEventSchedulerHost,
    AppleHostRuntimeTaskSchedulerHost as NativeHostRuntimeTaskSchedulerHost,
    AppleHttpHost as NativeHttpHost, AppleManagedRuntimeHost as NativeManagedRuntimeHost,
    AppleRuntimeStorageHost as NativeRuntimeStorageHost,
    AppleSystemOperationHost as NativeSystemOperationHost, AppleTerminalHost as NativeTerminalHost,
    AppleWebVisitHost as NativeWebVisitHost,
};
#[cfg(target_os = "linux")]
use operit_host_linux_native::{
    LinuxAudioPlaybackHost as NativeAudioPlaybackHost, LinuxBluetoothHost as NativeBluetoothHost,
    LinuxBrowserAutomationHost as NativeBrowserAutomationHost,
    LinuxFileSystemHost as NativeFileSystemHost,
    LinuxHostRuntimeEventHost as NativeHostRuntimeEventHost,
    LinuxHostRuntimeEventSchedulerHost as NativeHostRuntimeEventSchedulerHost,
    LinuxHostRuntimeTaskSchedulerHost as NativeHostRuntimeTaskSchedulerHost,
    LinuxHttpHost as NativeHttpHost, LinuxManagedRuntimeHost as NativeManagedRuntimeHost,
    LinuxRuntimeStorageHost as NativeRuntimeStorageHost,
    LinuxSystemOperationHost as NativeSystemOperationHost, LinuxTerminalHost as NativeTerminalHost,
    LinuxWebVisitHost as NativeWebVisitHost,
};
#[cfg(windows)]
use operit_host_windows_native::{
    WindowsAudioPlaybackHost as NativeAudioPlaybackHost,
    WindowsBluetoothHost as NativeBluetoothHost,
    WindowsBrowserAutomationHost as NativeBrowserAutomationHost,
    WindowsFileSystemHost as NativeFileSystemHost,
    WindowsHostRuntimeEventHost as NativeHostRuntimeEventHost,
    WindowsHostRuntimeEventSchedulerHost as NativeHostRuntimeEventSchedulerHost,
    WindowsHostRuntimeTaskSchedulerHost as NativeHostRuntimeTaskSchedulerHost,
    WindowsHttpHost as NativeHttpHost, WindowsManagedRuntimeHost as NativeManagedRuntimeHost,
    WindowsRuntimeStorageHost as NativeRuntimeStorageHost,
    WindowsSystemOperationHost as NativeSystemOperationHost,
    WindowsTerminalHost as NativeTerminalHost, WindowsWebVisitHost as NativeWebVisitHost,
};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use serde::{Deserialize, Serialize};

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
compile_error!("operit2 CLI host is implemented for Windows, Linux, and macOS.");

/// Creates the CLI application with the configured runtime and workspace roots.
pub(crate) fn create_cli_application() -> OperitApplication {
    let storageConfig = CliStorageConfig::read();
    let runtimeStorageHost = Arc::new(NativeRuntimeStorageHost::new(
        storageConfig.runtimeRoot,
        storageConfig.workspaceRoot,
    ));
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let hostSecretStore = runtimeStorageHost.clone();
    let mut context = HostManager::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
        Arc::new(NativeFileSystemHost::new()),
        Arc::new(NativeWebVisitHost::new()),
        Arc::new(NativeHttpHost::new()),
        Arc::new(NativeSystemOperationHost::new()),
        Arc::new(NativeManagedRuntimeHost::new()),
        runtimeStorageHost,
        runtimeSqliteHost,
    )
    .withHostSecretStore(hostSecretStore);
    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    {
        context = context.withTerminalHost(Arc::new(NativeTerminalHost::new()));
    }
    #[cfg(any(target_os = "linux", windows))]
    {
        context = context.withAudioPlaybackHost(Arc::new(NativeAudioPlaybackHost::new()));
        context = context.withBluetoothHost(Arc::new(NativeBluetoothHost::new()));
    }
    context = context.withHostRuntimeEventHost(Arc::new(NativeHostRuntimeEventHost::new()));
    context = context
        .withHostRuntimeEventSchedulerHost(Arc::new(NativeHostRuntimeEventSchedulerHost::new()));
    context = context
        .withHostRuntimeTaskSchedulerHost(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    context = context.withBrowserAutomationHost(Arc::new(NativeBrowserAutomationHost::new()));
    let commandContext = context.clone();
    OperitApplication::newWithContext(context.withCoreCommandExecutor(Arc::new(
        move |args: Vec<String>| {
            let output =
                operit_command_core::run_core_command_with_context(commandContext.clone(), &args)?;
            persist_cli_storage_config(&output.stdout)?;
            Ok(output.stdout)
        },
    )))
}

/// Creates the local core proxy used by CLI commands and services.
pub(crate) fn create_local_core() -> LocalCoreProxy {
    LocalCoreProxy::new(create_cli_application())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CliStorageConfig {
    dataRoot: PathBuf,
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
}

impl CliStorageConfig {
    /// Reads the CLI storage configuration used for local runtime startup.
    fn read() -> Self {
        let path = cli_storage_config_path();
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Self::current(),
            Err(error) => {
                panic!(
                    "read CLI storage config failed at {}: {error}",
                    path.display()
                )
            }
        };
        serde_json::from_str(&content).unwrap_or_else(|error| {
            panic!(
                "parse CLI storage config failed at {}: {error}",
                path.display()
            )
        })
    }

    /// Builds the current platform storage root configuration.
    fn current() -> Self {
        let runtimeRoot = NativeRuntimeStorageHost::defaultRuntimeRoot();
        let workspaceRoot = NativeRuntimeStorageHost::defaultWorkspaceRoot();
        let dataRoot = runtimeRoot
            .parent()
            .expect("default runtime root must have a parent")
            .to_path_buf();
        assert_eq!(
            workspaceRoot.parent(),
            Some(dataRoot.as_path()),
            "default runtime and workspace roots must share one data root"
        );
        Self {
            runtimeRoot,
            workspaceRoot,
            dataRoot,
        }
    }
}

/// Persists storage roots emitted by the core storage migrate command.
pub(crate) fn persist_cli_storage_config(stdout: &str) -> Result<(), String> {
    let Some(config) = parse_storage_migration_output(stdout)? else {
        return Ok(());
    };
    write_cli_storage_config(&config)
}

/// Parses storage command output into a startup storage configuration.
fn parse_storage_migration_output(stdout: &str) -> Result<Option<CliStorageConfig>, String> {
    let mut dataRoot = None;
    let mut runtimeRoot = None;
    let mut workspaceRoot = None;
    let mut changed = false;
    for line in stdout.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "dataRoot" => dataRoot = Some(PathBuf::from(value)),
            "runtimeRoot" => runtimeRoot = Some(PathBuf::from(value)),
            "workspaceRoot" => workspaceRoot = Some(PathBuf::from(value)),
            "storageConfig" if value == "updated" => changed = true,
            _ => {}
        }
    }
    if !changed {
        return Ok(None);
    }
    Ok(Some(CliStorageConfig {
        dataRoot: dataRoot.ok_or_else(|| "storage migrate output missed dataRoot".to_string())?,
        runtimeRoot: runtimeRoot
            .ok_or_else(|| "storage migrate output missed runtimeRoot".to_string())?,
        workspaceRoot: workspaceRoot
            .ok_or_else(|| "storage migrate output missed workspaceRoot".to_string())?,
    }))
}

/// Writes the CLI storage configuration file.
fn write_cli_storage_config(config: &CliStorageConfig) -> Result<(), String> {
    let path = cli_storage_config_path();
    let parent = path
        .parent()
        .expect("CLI storage config path must include parent directory");
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let content = serde_json::to_string_pretty(config).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

/// Returns the CLI storage configuration file path.
fn cli_storage_config_path() -> PathBuf {
    cli_config_dir().join("storage.json")
}

/// Returns the CLI configuration directory.
fn cli_config_dir() -> PathBuf {
    #[cfg(windows)]
    {
        let appdata = env::var_os("APPDATA").expect("APPDATA is required for Operit2 CLI config");
        return PathBuf::from(appdata).join("Operit2").join("cli");
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(xdg_config_home) = env::var_os("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg_config_home).join("operit2");
        }
        let home = env::var_os("HOME").expect("HOME is required for Operit2 CLI config");
        return PathBuf::from(home).join(".config").join("operit2");
    }
    #[cfg(target_os = "macos")]
    {
        let home = env::var_os("HOME").expect("HOME is required for Operit2 CLI config");
        return PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("Operit2")
            .join("cli");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_tools::tools::AIToolHandler::AIToolHandler;
    use operit_tools::tools::ToolResultDataClasses::ToolResultData;
    use operit_tools::ToolExecutionManager::{AITool, ToolParameter};

    #[test]
    fn direct_terminal_tool_chain_executes_visible_terminal() {
        let application = create_cli_application();
        let mut handler = application.toolHandler.clone();
        handler.registerDefaultTools();

        #[cfg(windows)]
        let (sessionName, command, expectedOutput) = (
            "direct-tool-visible-powershell",
            "Write-Output direct-tool-ok",
            "direct-tool-ok",
        );
        #[cfg(any(target_os = "linux", target_os = "macos"))]
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
