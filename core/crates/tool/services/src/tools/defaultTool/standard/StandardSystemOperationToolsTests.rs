use super::*;
use crate::files::RemoteFileGateway::RemoteFileGateway;
use crate::runtime_support::{RuntimeCoreNodeRouteState, RuntimeCoreNodeStatus};
use crate::ToolExecutionManager::{
    ToolExecutionManager, ToolExposureMode, ToolParameter, ToolRuntimeContext,
};
use operit_host_api::{AppInstallState, AppInstallTiming, AppOperationData, HostError, HostResult};
use operit_host_native_filesystem::PosixFileSystemHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_plugin_sdk::javascript::{
    JsExecutionCompletion, JsToolCallRequest, JsToolCallResult, JsToolCallResultData,
};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Default)]
struct InstallHost {
    calls: Mutex<Vec<(AppInstallTiming, String)>>,
}

/// Records device dispatch without ever running the machine's real package installer.
struct InstallGateway {
    state: RuntimeCoreNodeRouteState,
    calls: Mutex<Vec<JsToolCallRequest>>,
    error: Option<String>,
}

impl InstallGateway {
    /// Provides distinct local and remote device names for qualified package paths.
    fn new() -> Self {
        Self {
            state: RuntimeCoreNodeRouteState {
                currentNodeId: "local-core".into(),
                nodes: [node("local-core", "desktop"), node("remote-core", "phone")].into(),
            },
            calls: Mutex::new(Vec::new()),
            error: None,
        }
    }
}

impl RemoteFileGateway for InstallGateway {
    /// Returns the exact device snapshot configured by the test.
    fn routeState(&self) -> Result<RuntimeCoreNodeRouteState, String> {
        Ok(self.state.clone())
    }

    /// Returns a typed installation response from the selected remote device.
    fn dispatchRemoteTool(
        &self,
        request: JsToolCallRequest,
    ) -> JsExecutionCompletion<JsToolCallResult> {
        self.calls.lock().unwrap().push(request);
        let error = self.error.clone();
        Box::pin(async move {
            JsToolCallResult {
                success: error.is_none(),
                data: JsToolCallResultData::Value(
                    serde_json::from_str(
                        &ToolResultData::AppOperationData(
                            crate::tools::ToolResultDataClasses::AppOperationData {
                                operationType: "install_app".into(),
                                packageName: "app.remote.fixture".into(),
                                success: true,
                                details: "remote installer opened".into(),
                                installState: Some("installer_opened".into()),
                            },
                        )
                        .toJson(),
                    )
                    .unwrap(),
                ),
                error,
            }
        })
    }
}

/// Builds a reachable execution-capable space member.
fn node(id: &str, name: &str) -> RuntimeCoreNodeStatus {
    RuntimeCoreNodeStatus {
        nodeId: id.into(),
        displayName: name.into(),
        userName: String::new(),
        platform: "android".into(),
        model: "fixture".into(),
        reachable: true,
        canExecute: true,
        relayHops: None,
        relayPath: None,
    }
}

/// Supplies the real storage mapping and the shared file routing entry point.
fn installTools(
    vfs: VisualFileSystem,
    root: &std::path::Path,
    host: Arc<InstallHost>,
    gateway: Arc<InstallGateway>,
) -> StandardSystemOperationTools {
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.into(),
        root.join("workspaces"),
    ));
    StandardSystemOperationTools::new(
        Some(host),
        None,
        None,
        vfs.clone(),
        FileToolRouting::new(vfs, storage, gateway),
    )
}

/// Constructs the public installation request without pre-resolving its package path.
fn installCall(path: &str) -> AITool {
    AITool {
        name: "install_app".into(),
        parameters: vec![ToolParameter {
            name: "path".into(),
            value: path.into(),
        }],
    }
}

/// Captures the same current file directory used by ordinary file tools.
fn directoryContext(path: &str) -> ToolRuntimeContext {
    ToolRuntimeContext {
        callerChatId: None,
        callerParticipantId: None,
        workspacePath: Some(path.into()),
        workspaceFolders: vec![path.into()],
        toolExposureMode: ToolExposureMode::FULL,
        extensionOwner: "main".into(),
        executionData: Default::default(),
    }
}

macro_rules! unrelated_system_operations {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $result:ty;)*) => {
        $(/// Rejects operations outside the installation boundary test.
        #[allow(non_snake_case, unused_variables)]
        fn $name(&self, $($arg: $ty),*) -> HostResult<$result> { unreachable!() })*
    };
}

impl SystemOperationHost for InstallHost {
    /// Consumes the generic file reference at the native provider boundary.
    fn installApp(
        &self,
        source: &dyn operit_host_api::FileSource,
        timing: AppInstallTiming,
    ) -> HostResult<AppOperationData> {
        let path = source.nativePath()?;
        self.calls.lock().unwrap().push((timing, path));
        match timing {
            AppInstallTiming::Immediate => Err(HostError::new("installer rejected package")),
            AppInstallTiming::AfterProcessExit => Ok(AppOperationData {
                operationType: "install".into(),
                packageName: "fixture".into(),
                success: true,
                details: "scheduled".into(),
                installState: Some(AppInstallState::RestartRequired),
            }),
        }
    }
    unrelated_system_operations! {
        getSystemLanguageCode() -> String;
        sendNotification(request: &operit_host_api::SystemNotificationRequest) -> ();
        modifySystemSetting(namespace: &str, setting: &str, value: &str) -> operit_host_api::SystemSettingData;
        getSystemSetting(namespace: &str, setting: &str) -> operit_host_api::SystemSettingData;
        uninstallApp(packageName: &str) -> AppOperationData;
        listInstalledApps(includeSystemApps: bool) -> operit_host_api::AppListData;
        startApp(packageName: &str) -> AppOperationData;
        stopApp(packageName: &str) -> AppOperationData;
        getNotifications(limit: i32, includeOngoing: bool) -> operit_host_api::NotificationData;
        getAppUsageTime(packageName: &str, sinceHours: i32, limit: i32, includeSystemApps: bool) -> operit_host_api::AppUsageTimeResultData;
        getDeviceLocation(timeout: i32, highAccuracy: bool, includeAddress: bool) -> operit_host_api::LocationData;
        getDeviceInfo() -> operit_host_api::DeviceInfoData;
        captureScreenshot() -> String;
        recognizeText(imagePath: &dyn operit_host_api::FileSource, language: operit_host_api::OCRLanguage, quality: operit_host_api::OCRQuality) -> String;
    }
}

/// Builds one isolated runtime filesystem for installation boundary tests.
fn vfs() -> (VisualFileSystem, PathBuf) {
    let root = std::env::temp_dir().join(format!("operit-install-source-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.clone(),
        root.join("workspaces"),
    ));
    let vfs = VisualFileSystem::fromStorage(
        Arc::new(PosixFileSystemHost::new()),
        storage,
    ).unwrap();
    (vfs, root)
}

/// Keeps native compatibility in path mapping and leaves install timing to the system provider.
#[tokio::test]
async fn install_tool_passes_vfs_references_to_the_system_provider() {
    let (vfs, root) = vfs();
    let host = Arc::new(InstallHost::default());
    let path = "/operitapp/data/temp/application_updates/install/package.msi";
    vfs.writeFileBytes(path, b"package").unwrap();
    let native = root.join("temp/application_updates/install/package.msi");
    let native = native.to_string_lossy().into_owned();
    let tools = installTools(
        vfs.clone(),
        &root,
        host.clone(),
        Arc::new(InstallGateway::new()),
    );
    for input in [path, native.as_str()] {
        let result = tools
            .installApp(&AITool {
                name: "install_app".into(),
                parameters: vec![crate::ToolExecutionManager::ToolParameter {
                    name: "path".into(),
                    value: input.into(),
                }],
            })
            .await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("installer rejected package"));
    }
    let source = vfs.file(path).unwrap();
    let deferred = host
        .installApp(&source, AppInstallTiming::AfterProcessExit)
        .unwrap();
    assert_eq!(
        deferred.installState,
        Some(AppInstallState::RestartRequired)
    );
    let calls = host.calls.lock().unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].0, AppInstallTiming::Immediate);
    assert_eq!(calls[1].0, AppInstallTiming::Immediate);
    assert_eq!(calls[2].0, AppInstallTiming::AfterProcessExit);
    for (_, received) in calls.iter() {
        assert_eq!(
            std::path::Path::new(received),
            std::path::Path::new(&native)
        );
        assert_eq!(
            std::fs::read(received).unwrap(),
            vfs.readFileBytes(path).unwrap()
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// Resolves relative packages and self-qualified paths against the caller's selected directory.
#[tokio::test]
async fn install_tool_uses_the_current_file_directory() {
    let (vfs, root) = vfs();
    let host = Arc::new(InstallHost::default());
    let gateway = Arc::new(InstallGateway::new());
    let path = "/operitapp/workspaces/demo/build/package.apk";
    vfs.writeFileBytes(path, b"package").unwrap();
    let tools = installTools(vfs, &root, host.clone(), gateway.clone());
    for input in [
        "build/package.apk",
        "./build/../build/package.apk",
        "desktop:build/package.apk",
        "local-core:/operitapp/workspaces/demo/build/package.apk",
    ] {
        let result = ToolExecutionManager::scopeToolRuntimeContext(
            directoryContext("/operitapp/workspaces/demo"),
            tools.installApp(&installCall(input)),
        )
        .await;
        assert_eq!(
            result.error.as_deref(),
            Some("Error requesting app installation: installer rejected package")
        );
    }
    let calls = host.calls.lock().unwrap();
    assert_eq!(calls.len(), 4);
    for (_, path) in calls.iter() {
        assert_eq!(
            std::path::Path::new(path),
            root.join("workspaces/demo/build/package.apk")
        );
    }
    assert!(gateway.calls.lock().unwrap().is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

/// Uses the conversation's bound output directory when the caller has no workspace.
#[tokio::test]
async fn install_relative_package_uses_the_chat_output_directory() {
    let (vfs, root) = vfs();
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.clone(),
        root.join("workspaces"),
    ));
    let directory = operit_store::ChatOutputStore::ChatOutputStore::new(storage)
        .bind("chat-a", 1_700_000_000_000, None)
        .unwrap();
    let path = format!("{directory}/package.apk");
    vfs.writeFileBytes(&path, b"package").unwrap();
    let native = operit_host_api::FileSource::nativePath(&vfs.file(&path).unwrap()).unwrap();
    let host = Arc::new(InstallHost::default());
    let tools = installTools(vfs, &root, host.clone(), Arc::new(InstallGateway::new()));
    let mut context = directoryContext("/operitapp/workspaces/unused");
    context.workspacePath = None;
    context.workspaceFolders.clear();
    context.callerChatId = Some("chat-a".into());
    let result = ToolExecutionManager::scopeToolRuntimeContext(
        context,
        tools.installApp(&installCall("package.apk")),
    )
    .await;
    assert_eq!(
        result.error.as_deref(),
        Some("Error requesting app installation: installer rejected package")
    );
    assert_eq!(
        host.calls.lock().unwrap().as_slice(),
        &[(AppInstallTiming::Immediate, native)]
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Rejects unbound relative inputs rather than guessing a directory or dispatching installation.
#[tokio::test]
async fn install_relative_package_requires_a_current_directory() {
    let (vfs, root) = vfs();
    let host = Arc::new(InstallHost::default());
    let gateway = Arc::new(InstallGateway::new());
    let tools = installTools(vfs, &root, host.clone(), gateway.clone());
    let result = tools.installApp(&installCall("package.apk")).await;
    assert!(!result.success);
    assert_eq!(
        result.error.as_deref(),
        Some("Relative file paths require a current file directory")
    );
    assert!(host.calls.lock().unwrap().is_empty());
    assert!(gateway.calls.lock().unwrap().is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

/// Dispatches target-local VFS paths while preserving the remote installation result.
#[tokio::test]
async fn install_qualified_package_runs_on_the_selected_node() {
    let (vfs, root) = vfs();
    let host = Arc::new(InstallHost::default());
    let gateway = Arc::new(InstallGateway::new());
    let tools = installTools(vfs, &root, host.clone(), gateway.clone());
    let result = ToolExecutionManager::scopeToolRuntimeContext(
        directoryContext("/operitapp/workspaces/local-only"),
        tools.installApp(&installCall("phone:/operitapp/data/downloads/package.apk")),
    )
    .await;
    assert!(result.success);
    let ToolResultData::AppOperationData(data) = result.result else {
        panic!("installation result lost its type")
    };
    assert_eq!(data.packageName, "app.remote.fixture");
    assert_eq!(data.installState.as_deref(), Some("installer_opened"));
    assert!(host.calls.lock().unwrap().is_empty());
    let calls = gateway.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].qualified_tool_name(), "install_app");
    assert_eq!(calls[0].device.as_ref().unwrap().node_id, "remote-core");
    assert_eq!(
        calls[0].parameters["path"],
        "/operitapp/data/downloads/package.apk"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Leaves failed remote installation on the target node without invoking the local installer.
#[tokio::test]
async fn install_remote_failure_is_reported_without_local_execution() {
    let (vfs, root) = vfs();
    let host = Arc::new(InstallHost::default());
    let mut gateway = InstallGateway::new();
    gateway.error = Some("Remote installation denied".into());
    let gateway = Arc::new(gateway);
    let tools = installTools(vfs, &root, host.clone(), gateway.clone());
    let result = tools
        .installApp(&installCall("remote-core:/operitapp/data/package.apk"))
        .await;
    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("Remote installation denied"));
    assert!(host.calls.lock().unwrap().is_empty());
    assert_eq!(gateway.calls.lock().unwrap().len(), 1);
    std::fs::remove_dir_all(root).unwrap();
}

/// Rejects unknown, ambiguous, offline, and non-executing destinations before transport dispatch.
#[tokio::test]
async fn install_rejects_invalid_device_targets() {
    let (vfs, root) = vfs();
    let host = Arc::new(InstallHost::default());
    for (mode, reference, message) in [
        ("unknown", "missing", "not a member"),
        ("ambiguous", "phone", "matches multiple devices"),
        ("offline", "phone", "currently unreachable"),
        ("unsupported", "phone", "cannot run file operations"),
    ] {
        let mut gateway = InstallGateway::new();
        match mode {
            "ambiguous" => gateway.state.nodes.push(node("other-core", "phone")),
            "offline" => gateway.state.nodes[1].reachable = false,
            "unsupported" => gateway.state.nodes[1].canExecute = false,
            _ => {}
        }
        let gateway = Arc::new(gateway);
        let tools = installTools(vfs.clone(), &root, host.clone(), gateway.clone());
        let result = tools
            .installApp(&installCall(&format!(
                "{reference}:/operitapp/data/package.apk"
            )))
            .await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains(message));
        assert!(gateway.calls.lock().unwrap().is_empty());
    }
    assert!(host.calls.lock().unwrap().is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

/// Retains installation's external write policy after switching to asynchronous routing.
#[test]
fn install_executor_keeps_the_host_write_approval_boundary() {
    let (vfs, root) = vfs();
    let executor = InstallAppToolExecutor {
        tools: installTools(
            vfs,
            &root,
            Arc::new(InstallHost::default()),
            Arc::new(InstallGateway::new()),
        ),
    };
    let call = installCall("/operitapp/data/package.apk");
    let access = executor.accessSpec(&call).unwrap();
    assert_eq!(access.effect, ToolEffect::WRITE);
    assert!(matches!(access.boundary, ToolBoundary::None));
    assert!(executor.validateParameters(&call).valid);
    assert!(!executor.validateParameters(&installCall(" ")).valid);
    std::fs::remove_dir_all(root).unwrap();
}

/// Rejects document references without handing opaque locators to native installer code.
#[test]
fn document_mounts_cannot_be_used_as_native_install_sources() {
    let (vfs, root) = vfs();
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.clone(),
        root.join("workspaces"),
    ));
    let registry = operit_vfs::MountRegistry::MountRegistry::withStorage(storage);
    let mount = registry
        .register(
            "/virtual/test/documents",
            "test",
            "opaque-root",
            "Documents",
        )
        .unwrap();
    let source = vfs
        .file(&format!("{}/package.apk", mount.vfsPath()))
        .unwrap();
    let host = InstallHost::default();
    for timing in [
        AppInstallTiming::Immediate,
        AppInstallTiming::AfterProcessExit,
    ] {
        assert!(host.installApp(&source, timing).is_err());
    }
    assert!(host.calls.lock().unwrap().is_empty());
    std::fs::remove_dir_all(root).unwrap();
}
