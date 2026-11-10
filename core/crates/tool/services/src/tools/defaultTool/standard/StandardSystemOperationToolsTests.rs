use super::*;
use operit_host_api::{AppInstallState, AppInstallTiming, AppOperationData, HostError, HostResult};
use operit_host_native_filesystem::PosixFileSystemHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Default)]
struct InstallHost {
    calls: Mutex<Vec<(AppInstallTiming, String)>>,
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
        recognizeText(imagePath: &str, language: operit_host_api::OCRLanguage, quality: operit_host_api::OCRQuality) -> String;
    }
}

/// Builds one isolated runtime filesystem for installation boundary tests.
fn vfs() -> (VisualFileSystem, PathBuf) {
    let root = std::env::temp_dir().join(format!("operit-install-source-{}", uuid::Uuid::new_v4()));
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.clone(),
        root.join("workspaces"),
    ));
    let vfs = VisualFileSystem::new(
        Arc::new(PosixFileSystemHost::new()),
        PathMapper::new(root.clone(), root.join("workspaces"), storage),
    );
    (vfs, root)
}

/// Keeps native compatibility in path mapping and leaves install timing to the system provider.
#[test]
fn install_tool_passes_vfs_references_to_the_system_provider() {
    let (vfs, root) = vfs();
    let host = Arc::new(InstallHost::default());
    let path = "/operitapp/data/temp/application_updates/install/package.msi";
    vfs.writeFileBytes(path, b"package").unwrap();
    let native = root.join("temp/application_updates/install/package.msi");
    let native = native.to_string_lossy().into_owned();
    let tools = StandardSystemOperationTools::new(Some(host.clone()), None, None, vfs.clone());
    for input in [path, native.as_str()] {
        let result = tools.installApp(&AITool {
            name: "install_app".into(),
            parameters: vec![crate::ToolExecutionManager::ToolParameter {
                name: "path".into(),
                value: input.into(),
            }],
        });
        assert!(!result.success);
        assert!(result.error.unwrap().contains("installer rejected package"));
    }
    let source = vfs.resolvePath(path).unwrap();
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

/// Rejects document references without handing opaque locators to native installer code.
#[test]
fn document_mounts_cannot_be_used_as_native_install_sources() {
    let (vfs, root) = vfs();
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.clone(),
        root.join("workspaces"),
    ));
    let registry = crate::files::MountRegistry::MountRegistry::withStorage(storage);
    let mount = registry
        .register(
            "/virtual/test/documents",
            "test",
            "opaque-root",
            "Documents",
        )
        .unwrap();
    let source = vfs
        .resolvePath(&format!("{}/package.apk", mount.vfsPath()))
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
