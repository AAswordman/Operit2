#![allow(non_snake_case)]

use std::path::PathBuf;

use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeResultData, DeviceInfoData, FileEntry,
    FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest, GrepCodeResult,
    HostEnvironmentDescriptor, HostError, HostResult, LocationData, ManagedRuntimeHost,
    ManagedRuntimeProgram, ManagedRuntimeProcess, NotificationData, RuntimeCommandOutput,
    RuntimeProcessRequest, RuntimeSqliteConnection, RuntimeSqliteHost, RuntimeStorageEntry,
    RuntimeStorageHost, SystemOperationHost, SystemSettingData, WebVisitHost, WebVisitRequest,
    WebVisitResult,
};

#[derive(Clone, Debug, Default)]
pub struct AndroidFileSystemHost {
    inner: operit_host_linux_native::LinuxFileSystemHost,
}

impl AndroidFileSystemHost {
    pub fn new() -> Self {
        Self {
            inner: operit_host_linux_native::LinuxFileSystemHost::new(),
        }
    }
}

impl FileSystemHost for AndroidFileSystemHost {
    fn envLabel(&self) -> &str {
        "android"
    }

    fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
        HostEnvironmentDescriptor::android()
    }

    fn validatePath(&self, path: &str, paramName: &str) -> HostResult<()> {
        if path.trim().is_empty() {
            return Err(HostError::new(format!("{paramName} parameter is required")));
        }
        if !std::path::Path::new(path).is_absolute() {
            return Err(HostError::new(format!(
                "Invalid path: '{path}'. Path must be an absolute Android path."
            )));
        }
        Ok(())
    }

    fn listFiles(&self, path: &str) -> HostResult<Vec<FileEntry>> {
        self.inner.listFiles(path)
    }

    fn readFile(&self, path: &str) -> HostResult<String> {
        self.inner.readFile(path)
    }

    fn readFileWithLimit(&self, path: &str, maxBytes: usize) -> HostResult<String> {
        self.inner.readFileWithLimit(path, maxBytes)
    }

    fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.inner.readFileBytes(path)
    }

    fn writeFile(&self, path: &str, content: &str, append: bool) -> HostResult<()> {
        self.inner.writeFile(path, content, append)
    }

    fn writeFileBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.writeFileBytes(path, content)
    }

    fn deleteFile(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.inner.deleteFile(path, recursive)
    }

    fn fileExists(&self, path: &str) -> HostResult<FileExistence> {
        self.inner.fileExists(path)
    }

    fn moveFile(&self, source: &str, destination: &str) -> HostResult<()> {
        self.inner.moveFile(source, destination)
    }

    fn copyFile(&self, source: &str, destination: &str, recursive: bool) -> HostResult<()> {
        self.inner.copyFile(source, destination, recursive)
    }

    fn makeDirectory(&self, path: &str, createParents: bool) -> HostResult<()> {
        self.inner.makeDirectory(path, createParents)
    }

    fn findFiles(&self, request: FindFilesRequest) -> HostResult<Vec<String>> {
        self.inner.findFiles(request)
    }

    fn fileInfo(&self, path: &str) -> HostResult<FileInfo> {
        self.inner.fileInfo(path)
    }

    fn grepCode(&self, request: GrepCodeRequest) -> HostResult<GrepCodeResult> {
        self.inner.grepCode(request)
    }

    fn zipFiles(&self, source: &str, destination: &str) -> HostResult<()> {
        self.inner.zipFiles(source, destination)
    }

    fn unzipFiles(&self, source: &str, destination: &str) -> HostResult<()> {
        self.inner.unzipFiles(source, destination)
    }

    fn openFile(&self, path: &str) -> HostResult<()> {
        Err(HostError::new(format!(
            "Android open_file requires the Flutter Android host bridge: {path}"
        )))
    }

    fn shareFile(&self, path: &str, title: &str) -> HostResult<()> {
        Err(HostError::new(format!(
            "Android share_file requires the Flutter Android host bridge: {path} ({title})"
        )))
    }
}

#[derive(Clone, Debug)]
pub struct AndroidRuntimeStorageHost {
    inner: operit_host_linux_native::LinuxRuntimeStorageHost,
}

impl AndroidRuntimeStorageHost {
    pub fn new(root: PathBuf) -> Self {
        Self {
            inner: operit_host_linux_native::LinuxRuntimeStorageHost::new(root),
        }
    }
}

impl RuntimeStorageHost for AndroidRuntimeStorageHost {
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.inner.readBytes(path)
    }

    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.writeBytes(path, content)
    }

    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.inner.delete(path, recursive)
    }

    fn exists(&self, path: &str) -> HostResult<bool> {
        self.inner.exists(path)
    }

    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        self.inner.list(prefix)
    }
}

impl RuntimeSqliteHost for AndroidRuntimeStorageHost {
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        self.inner.openSqliteDatabase(path)
    }
}

#[derive(Clone, Default)]
pub struct AndroidManagedRuntimeHost;

impl AndroidManagedRuntimeHost {
    pub fn new() -> Self {
        Self
    }
}

impl ManagedRuntimeHost for AndroidManagedRuntimeHost {
    fn runtimeWorkspaceDir(&self) -> HostResult<String> {
        Err(HostError::new(
            "Android managed runtime workspace requires the Android terminal host bridge",
        ))
    }

    fn resolveRuntimeExecutable(
        &self,
        program: ManagedRuntimeProgram,
        executablePath: Option<&str>,
    ) -> HostResult<String> {
        executablePath
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .ok_or_else(|| {
                HostError::new(format!(
                    "Android managed runtime executable is not configured for {:?}",
                    program
                ))
            })
    }

    fn startRuntimeProcess(
        &self,
        _request: RuntimeProcessRequest,
    ) -> HostResult<Box<dyn ManagedRuntimeProcess>> {
        Err(HostError::new(
            "Android managed runtime process requires the Android terminal host bridge",
        ))
    }

    fn runRuntimeCommand(&self, _request: RuntimeProcessRequest) -> HostResult<RuntimeCommandOutput> {
        Err(HostError::new(
            "Android managed runtime command requires the Android terminal host bridge",
        ))
    }
}

#[derive(Clone, Debug, Default)]
pub struct AndroidWebVisitHost;

impl AndroidWebVisitHost {
    pub fn new() -> Self {
        Self
    }
}

impl WebVisitHost for AndroidWebVisitHost {
    fn visitWeb(&self, _request: WebVisitRequest) -> HostResult<WebVisitResult> {
        Err(HostError::new(
            "Android visit_web requires the Android WebView host bridge",
        ))
    }
}

#[derive(Clone, Debug, Default)]
pub struct AndroidSystemOperationHost;

impl AndroidSystemOperationHost {
    pub fn new() -> Self {
        Self
    }
}

impl SystemOperationHost for AndroidSystemOperationHost {
    fn toast(&self, message: &str) -> HostResult<()> {
        Err(HostError::new(format!(
            "Android toast requires the Android UI host bridge: {message}"
        )))
    }

    fn sendNotification(&self, title: &str, message: &str) -> HostResult<()> {
        Err(HostError::new(format!(
            "Android notification requires the Android UI host bridge: {title}: {message}"
        )))
    }

    fn modifySystemSetting(
        &self,
        namespace: &str,
        setting: &str,
        value: &str,
    ) -> HostResult<SystemSettingData> {
        Err(HostError::new(format!(
            "Android modify_system_setting requires the Android system host bridge: {namespace}/{setting}={value}"
        )))
    }

    fn getSystemSetting(&self, namespace: &str, setting: &str) -> HostResult<SystemSettingData> {
        Err(HostError::new(format!(
            "Android get_system_setting requires the Android system host bridge: {namespace}/{setting}"
        )))
    }

    fn installApp(&self, path: &str) -> HostResult<AppOperationData> {
        Err(HostError::new(format!(
            "Android install_app requires the Android package host bridge: {path}"
        )))
    }

    fn uninstallApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        Err(HostError::new(format!(
            "Android uninstall_app requires the Android package host bridge: {packageName}"
        )))
    }

    fn listInstalledApps(&self, includeSystemApps: bool) -> HostResult<AppListData> {
        Err(HostError::new(format!(
            "Android list_installed_apps requires the Android package host bridge, include_system_apps={includeSystemApps}"
        )))
    }

    fn startApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        Err(HostError::new(format!(
            "Android start_app requires the Android package host bridge: {packageName}"
        )))
    }

    fn stopApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        Err(HostError::new(format!(
            "Android stop_app requires the Android package host bridge: {packageName}"
        )))
    }

    fn getNotifications(&self, limit: i32, includeOngoing: bool) -> HostResult<NotificationData> {
        Err(HostError::new(format!(
            "Android get_notifications requires the Android notification host bridge: limit={limit}, include_ongoing={includeOngoing}"
        )))
    }

    fn getAppUsageTime(
        &self,
        packageName: &str,
        sinceHours: i32,
        limit: i32,
        includeSystemApps: bool,
    ) -> HostResult<AppUsageTimeResultData> {
        Err(HostError::new(format!(
            "Android get_app_usage_time requires the Android usage stats host bridge: package={packageName}, since_hours={sinceHours}, limit={limit}, include_system_apps={includeSystemApps}"
        )))
    }

    fn getDeviceLocation(
        &self,
        timeout: i32,
        highAccuracy: bool,
        includeAddress: bool,
    ) -> HostResult<LocationData> {
        Err(HostError::new(format!(
            "Android get_device_location requires the Android location host bridge: timeout={timeout}, high_accuracy={highAccuracy}, include_address={includeAddress}"
        )))
    }

    fn getDeviceInfo(&self) -> HostResult<DeviceInfoData> {
        Err(HostError::new(
            "Android get_device_info requires the Android device info host bridge",
        ))
    }
}
