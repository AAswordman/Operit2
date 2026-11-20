//! Preserves platform system capabilities while adding Flutter owner callbacks.
use std::sync::Arc;
use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeResultData, DeviceInfoData, HostError,
    HostResult, LocationData, NotificationData, OCRLanguage, OCRQuality,
    SystemNotificationRequest, SystemOperationHost, SystemSettingData,
};

/// Selects concrete owner callbacks once, during host assembly.
pub(crate) struct FlutterSystemBindings {
    pub notificationSender: Arc<dyn Fn(&SystemNotificationRequest) -> HostResult<()> + Send + Sync>,
    pub notifications: Arc<dyn Fn(i32, bool) -> HostResult<NotificationData> + Send + Sync>,
    pub location: Arc<dyn Fn(i32, bool, bool) -> HostResult<LocationData> + Send + Sync>,
    pub deviceInfo: Arc<dyn Fn() -> HostResult<DeviceInfoData> + Send + Sync>,
    pub screenshot: Arc<dyn Fn() -> HostResult<String> + Send + Sync>,
    pub recognition: Arc<dyn Fn(&dyn operit_host_api::FileSource, OCRLanguage, OCRQuality) -> HostResult<String> + Send + Sync>,
}

/// Adds owner authorization and UI operations to an already selected system host.
pub(crate) struct FlutterSystemOperationBridge {
    host: Arc<dyn SystemOperationHost>,
    bindings: FlutterSystemBindings,
}

impl FlutterSystemOperationBridge {
    /// Creates a fixed binding set without choosing another backend after failures.
    pub(crate) fn new(host: Arc<dyn SystemOperationHost>, bindings: FlutterSystemBindings) -> Self {
        Self { host, bindings }
    }
}

impl SystemOperationHost for FlutterSystemOperationBridge {
    /// Preserves the selected host's lossless installer transformation capability.
    fn transformApplicationUpdate(
        &self,
        source: &dyn operit_host_api::FileSource,
        target: &dyn operit_host_api::FileTarget,
        transform: operit_host_api::ApplicationUpdateTransform,
        control: &operit_host_api::HttpDownloadControl,
        onProgress: &(dyn Fn(u64, u64) + Send + Sync),
    ) -> HostResult<()> {
        self.host
            .transformApplicationUpdate(source, target, transform, control, onProgress)
    }
    /// Preserves installed-package access instead of replacing it with the desktop default.
    fn applicationUpdateBase(&self) -> HostResult<operit_host_api::ApplicationUpdateBase> {
        self.host.applicationUpdateBase()
    }
    /// Preserves the selected host's application installation identity.
    fn applicationUpdateTarget(
        &self,
    ) -> HostResult<Option<operit_host_api::ApplicationUpdateTarget>> {
        self.host.applicationUpdateTarget()
    }
    /// Reads the selected host's language.
    fn getSystemLanguageCode(&self) -> HostResult<String> {
        self.host.getSystemLanguageCode()
    }
    /// Sends a system notification using the owner's authorization UI.
    fn sendNotification(&self, request: &SystemNotificationRequest) -> HostResult<()> {
        (self.bindings.notificationSender)(request)
    }
    /// Modifies settings through the selected system host.
    fn modifySystemSetting(
        &self,
        namespace: &str,
        setting: &str,
        value: &str,
    ) -> HostResult<SystemSettingData> {
        self.host.modifySystemSetting(namespace, setting, value)
    }
    /// Reads settings through the selected system host.
    fn getSystemSetting(&self, namespace: &str, setting: &str) -> HostResult<SystemSettingData> {
        self.host.getSystemSetting(namespace, setting)
    }
    /// Forwards the file reference and execution timing without resolving native paths.
    fn installApp(&self, source: &dyn operit_host_api::FileSource, timing: operit_host_api::AppInstallTiming) -> HostResult<AppOperationData> {
        self.host.installApp(source, timing)
    }
    /// Uninstalls an app through the selected system host.
    fn uninstallApp(&self, name: &str) -> HostResult<AppOperationData> {
        self.host.uninstallApp(name)
    }
    /// Lists apps through the selected system host.
    fn listInstalledApps(&self, includeSystem: bool) -> HostResult<AppListData> {
        self.host.listInstalledApps(includeSystem)
    }
    /// Starts an app through the selected system host.
    fn startApp(&self, name: &str) -> HostResult<AppOperationData> {
        self.host.startApp(name)
    }
    fn startAppWithActivity(
        &self,
        name: &str,
        activity: Option<&str>,
    ) -> HostResult<AppOperationData> {
        self.host.startAppWithActivity(name, activity)
    }
    /// Stops an app through the selected system host.
    fn stopApp(&self, name: &str) -> HostResult<AppOperationData> {
        self.host.stopApp(name)
    }
    /// Reads notifications through the callback selected at host assembly.
    fn getNotifications(&self, limit: i32, includeOngoing: bool) -> HostResult<NotificationData> {
        (self.bindings.notifications)(limit, includeOngoing)
    }
    /// Reads app usage through the selected system host.
    fn getAppUsageTime(
        &self,
        name: &str,
        hours: i32,
        limit: i32,
        includeSystem: bool,
    ) -> HostResult<AppUsageTimeResultData> {
        self.host.getAppUsageTime(name, hours, limit, includeSystem)
    }
    /// Reads current location through the owner's authorization UI.
    fn getDeviceLocation(
        &self,
        timeout: i32,
        highAccuracy: bool,
        includeAddress: bool,
    ) -> HostResult<LocationData> {
        (self.bindings.location)(timeout, highAccuracy, includeAddress)
    }
    /// Reads information through the callback selected at host assembly.
    fn getDeviceInfo(&self) -> HostResult<DeviceInfoData> {
        (self.bindings.deviceInfo)()
    }
    /// Captures a screen through the callback selected at host assembly.
    /// Writes image bytes through the selected VFS capability and removes native staging.
    fn captureScreenshotTo(&self, target: &dyn operit_host_api::FileTarget) -> HostResult<()> {
        let path = self.captureScreenshot()?;
        let bytes = std::fs::read(&path).map_err(|error| operit_host_api::HostError::new(error.to_string()))?;
        let written = target.writeBytes(&bytes);
        std::fs::remove_file(&path).map_err(|error| operit_host_api::HostError::new(error.to_string()))?;
        written
    }

    fn captureScreenshot(&self) -> HostResult<String> {
        (self.bindings.screenshot)()
    }
    /// Recognizes text through the owner's OCR implementation.
    fn recognizeText(
        &self,
        path: &dyn operit_host_api::FileSource,
        language: OCRLanguage,
        quality: OCRQuality,
    ) -> HostResult<String> {
        (self.bindings.recognition)(path, language, quality)
    }
}
