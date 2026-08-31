use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use operit_access_runtime::RemoteDeviceInfo;
use operit_core_application::{CoreApplication, CoreApplicationConfig};
use operit_host_api::HostManager::HostManager;
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
#[cfg(target_os = "macos")]
use operit_host_macos_native::{
    MacosBrowserAutomationHost as NativeBrowserAutomationHost,
    MacosFileSystemHost as NativeFileSystemHost,
    MacosHostRuntimeEventHost as NativeHostRuntimeEventHost,
    MacosHostRuntimeEventSchedulerHost as NativeHostRuntimeEventSchedulerHost,
    MacosHostRuntimeTaskSchedulerHost as NativeHostRuntimeTaskSchedulerHost,
    MacosHttpHost as NativeHttpHost, MacosManagedRuntimeHost as NativeManagedRuntimeHost,
    MacosRuntimeStorageHost as NativeRuntimeStorageHost,
    MacosSystemOperationHost as NativeSystemOperationHost, MacosTerminalHost as NativeTerminalHost,
    MacosWebVisitHost as NativeWebVisitHost,
};
use operit_host_native_common::NativeHostJavaScriptRuntimeHost;
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
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
compile_error!("operit2 CLI host is implemented for Windows, Linux, and macOS.");

/// Creates the CLI host manager with the configured runtime and workspace roots.
pub(crate) fn create_cli_host_manager() -> HostManager {
    let storageConfig = CliStorageConfig::read();
    let (runtimeRoot, workspaceRoot) = storageConfig.activeRoots();
    let archiveStagingHost = Arc::new(operit_host_native_common::NativeArchiveStagingHost::new(
        runtimeRoot.clone(),
    ));
    let runtimeStorageHost = Arc::new(NativeRuntimeStorageHost::new(
        runtimeRoot.clone(),
        workspaceRoot.clone(),
    ));
    let runtimeStorageWriteHost = Arc::new(
        operit_host_native_common::NativeRuntimeStorageHost::new(runtimeRoot, workspaceRoot),
    );
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
    .withHostSecretStore(hostSecretStore)
    .withWebSocketHost(Arc::new(NativeHttpHost::new()))
    .withArchiveStagingHost(archiveStagingHost)
    .withRuntimeStorageWriteHost(runtimeStorageWriteHost);
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
    context =
        context.withHostJavaScriptRuntimeHost(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    context = context.withBrowserAutomationHost(Arc::new(NativeBrowserAutomationHost::new()));
    let commandContext = context.clone();
    context.withCoreCommandExecutor(Arc::new(move |args: Vec<String>| {
        let output =
            operit_command_core::run_core_command_with_context(commandContext.clone(), &args)?;
        persist_cli_storage_config(&output.stdout)?;
        Ok(output.stdout)
    }))
}

/// Creates the CLI application with the configured runtime and workspace roots.
pub(crate) fn create_cli_application() -> OperitApplication {
    OperitApplication::newWithContext(create_cli_host_manager())
}

/// Starts the unified Core application tree for CLI commands.
pub(crate) async fn create_cli_core_application(
    deviceName: &str,
) -> Result<CoreApplication, String> {
    CoreApplication::start(CoreApplicationConfig::new(
        create_cli_host_manager(),
        RemoteDeviceInfo::nativeCli(deviceName)?,
    ))
    .await
}

/// Starts a short-lived CLI Core tree without claiming the persistent sync worker.
pub(crate) async fn create_cli_core_application_without_space_sync(
    deviceName: &str,
) -> Result<CoreApplication, String> {
    CoreApplication::start(
        CoreApplicationConfig::new(
            create_cli_host_manager(),
            RemoteDeviceInfo::nativeCli(deviceName)?,
        )
        .withSpaceSync(false),
    )
    .await
}

/// Starts the CLI Core tree after configuring its local client.
pub(crate) async fn create_cli_core_application_configured(
    deviceName: &str,
    configurator: impl FnOnce(&mut LocalCoreProxy) -> Result<(), String> + Send + 'static,
) -> Result<CoreApplication, String> {
    CoreApplication::start(
        CoreApplicationConfig::new(
            create_cli_host_manager(),
            RemoteDeviceInfo::nativeCli(deviceName)?,
        )
        .withLocalClientConfigurator(configurator),
    )
    .await
}

/// Describes one isolated CLI identity stored outside the active runtime root.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CliIdentity {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) createdAt: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CliStorageConfig {
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
    activeIdentityId: String,
    identities: Vec<CliIdentity>,
}

impl CliStorageConfig {
    /// Reads the CLI storage configuration used for local runtime startup.
    fn read() -> Self {
        let path = cli_storage_config_path();
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let config = Self::current();
                write_cli_storage_config(&config).unwrap_or_else(|writeError| {
                    panic!(
                        "write initial CLI storage config failed at {}: {writeError}",
                        path.display()
                    )
                });
                return config;
            }
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
        let identity =
            newCliIdentity("Operit".to_string()).expect("initial CLI identity name must be valid");
        Self {
            runtimeRoot,
            workspaceRoot,
            activeIdentityId: identity.id.clone(),
            identities: vec![identity],
        }
    }

    /// Returns the active identity after validating the persisted selection.
    #[allow(non_snake_case)]
    fn activeIdentity(&self) -> &CliIdentity {
        self.identities
            .iter()
            .find(|identity| identity.id == self.activeIdentityId)
            .expect("active CLI identity must exist")
    }

    /// Resolves runtime and workspace roots owned by the active identity.
    #[allow(non_snake_case)]
    fn activeRoots(&self) -> (PathBuf, PathBuf) {
        let identityId = &self.activeIdentity().id;
        (
            identityRoot(&self.runtimeRoot, identityId),
            identityRoot(&self.workspaceRoot, identityId),
        )
    }
}

/// Returns every configured CLI identity and the active identity id.
pub(crate) fn cli_identities() -> (Vec<CliIdentity>, String) {
    let config = CliStorageConfig::read();
    (config.identities, config.activeIdentityId)
}

/// Creates one isolated CLI identity without changing the active process identity.
pub(crate) fn create_cli_identity(name: String) -> Result<CliIdentity, String> {
    let identity = newCliIdentity(name)?;
    let mut config = CliStorageConfig::read();
    config.identities.push(identity.clone());
    write_cli_storage_config(&config)?;
    Ok(identity)
}

/// Renames one CLI identity while preserving its storage directory id.
pub(crate) fn rename_cli_identity(identityId: &str, name: String) -> Result<(), String> {
    validateIdentityName(&name)?;
    let mut config = CliStorageConfig::read();
    let identity = config
        .identities
        .iter_mut()
        .find(|identity| identity.id == identityId)
        .ok_or_else(|| format!("CLI identity does not exist: {identityId}"))?;
    identity.name = name.trim().to_string();
    write_cli_storage_config(&config)
}

/// Selects the CLI identity used by subsequent commands.
pub(crate) fn select_cli_identity(identityId: &str) -> Result<(), String> {
    let mut config = CliStorageConfig::read();
    if !config
        .identities
        .iter()
        .any(|identity| identity.id == identityId)
    {
        return Err(format!("CLI identity does not exist: {identityId}"));
    }
    config.activeIdentityId = identityId.to_string();
    write_cli_storage_config(&config)
}

/// Returns the physical runtime root owned by the active CLI identity.
pub(crate) fn active_cli_runtime_root() -> PathBuf {
    CliStorageConfig::read().activeRoots().0
}

/// Rewrites local storage migration targets into the active identity directories.
pub(crate) fn scope_cli_storage_command_args(args: &[String]) -> Result<Vec<String>, String> {
    if args.first().map(String::as_str) != Some("storage")
        || args.get(1).map(String::as_str) != Some("migrate")
    {
        return Ok(args.to_vec());
    }
    let identityId = CliStorageConfig::read().activeIdentity().id.clone();
    let mut scoped = args.to_vec();
    let mut index = 2usize;
    while index < scoped.len() {
        match scoped[index].as_str() {
            "--runtime" | "--workspace" => {
                index += 1;
                let root = scoped
                    .get_mut(index)
                    .ok_or_else(|| "storage migrate root is missing".to_string())?;
                *root = identityRoot(&PathBuf::from(root.as_str()), &identityId)
                    .to_string_lossy()
                    .into_owned();
            }
            argument => {
                return Err(format!("unknown storage migrate argument: {argument}"));
            }
        }
        index += 1;
    }
    Ok(scoped)
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
    let mut runtimeRoot = None;
    let mut workspaceRoot = None;
    let mut changed = false;
    for line in stdout.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "runtimeRoot" => runtimeRoot = Some(PathBuf::from(value)),
            "workspaceRoot" => workspaceRoot = Some(PathBuf::from(value)),
            "storageConfig" if value == "updated" => changed = true,
            _ => {}
        }
    }
    if !changed {
        return Ok(None);
    }
    let current = CliStorageConfig::read();
    let identityId = current.activeIdentity().id.clone();
    let runtimeRoot = identityBaseRoot(
        runtimeRoot.ok_or_else(|| "storage migrate output missed runtimeRoot".to_string())?,
        &identityId,
    )?;
    let workspaceRoot = identityBaseRoot(
        workspaceRoot.ok_or_else(|| "storage migrate output missed workspaceRoot".to_string())?,
        &identityId,
    )?;
    Ok(Some(CliStorageConfig {
        runtimeRoot,
        workspaceRoot,
        activeIdentityId: current.activeIdentityId,
        identities: current.identities,
    }))
}

/// Creates one validated CLI identity record.
#[allow(non_snake_case)]
fn newCliIdentity(name: String) -> Result<CliIdentity, String> {
    validateIdentityName(&name)?;
    Ok(CliIdentity {
        id: format!("identity-{}", Uuid::new_v4().simple()),
        name: name.trim().to_string(),
        createdAt: operit_host_api::TimeUtils::currentTimeMillis(),
    })
}

/// Validates one user-visible CLI identity name.
#[allow(non_snake_case)]
fn validateIdentityName(name: &str) -> Result<(), String> {
    let normalized = name.trim();
    if normalized.is_empty() {
        return Err("CLI identity name must not be empty".to_string());
    }
    if normalized.chars().count() > 80 {
        return Err("CLI identity name must not exceed 80 characters".to_string());
    }
    if normalized.chars().any(char::is_control) {
        return Err("CLI identity name must not contain control characters".to_string());
    }
    Ok(())
}

/// Appends the canonical identity directory segments to one base root.
#[allow(non_snake_case)]
fn identityRoot(baseRoot: &std::path::Path, identityId: &str) -> PathBuf {
    baseRoot.join("identities").join(identityId)
}

/// Extracts the configured base root from one active identity root.
#[allow(non_snake_case)]
fn identityBaseRoot(identityPath: PathBuf, identityId: &str) -> Result<PathBuf, String> {
    let identityDirectory = identityPath
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or_else(|| "identity storage path has no final directory".to_string())?;
    if identityDirectory != identityId {
        return Err(format!(
            "identity storage path must end with the active identity id: {identityId}"
        ));
    }
    let identitiesDirectory = identityPath
        .parent()
        .ok_or_else(|| "identity storage path has no identities directory".to_string())?;
    if identitiesDirectory.file_name() != Some(std::ffi::OsStr::new("identities")) {
        return Err("identity storage path must be inside an identities directory".to_string());
    }
    identitiesDirectory
        .parent()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| "identity storage path has no base root".to_string())
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
