use std::sync::Arc;

use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeResultData, DeviceInfoData, HostError, HostResult,
    LocationData, NotificationData, OCRLanguage, OCRQuality, SystemNotificationRequest,
    SystemOperationHost, SystemSettingData,
};

pub type AndroidSystemSettingReader =
    Arc<dyn Fn(&str, &str) -> HostResult<SystemSettingData> + Send + Sync>;
pub type AndroidSystemSettingWriter =
    Arc<dyn Fn(&str, &str, &str) -> HostResult<SystemSettingData> + Send + Sync>;

#[derive(Clone)]
pub struct AndroidSystemOperationHost {
    settingReader: AndroidSystemSettingReader,
    settingWriter: AndroidSystemSettingWriter,
    language: Option<String>,
}

impl AndroidSystemOperationHost {
    /// Creates an Android system host with its required owner settings bindings.
    pub fn new(
        settingReader: AndroidSystemSettingReader,
        settingWriter: AndroidSystemSettingWriter,
    ) -> Self {
        Self {
            settingReader,
            settingWriter,
            language: None,
        }
    }

    /// Creates the standalone Android host with an explicit boot-time locale.
    pub fn forPrivilegedRuntime(
        language: String,
        settingReader: AndroidSystemSettingReader,
        settingWriter: AndroidSystemSettingWriter,
    ) -> Self {
        Self {
            settingReader,
            settingWriter,
            language: Some(language),
        }
    }
}

impl SystemOperationHost for AndroidSystemOperationHost {
    /// Reads the system language directly from the Android runtime host.
    fn getSystemLanguageCode(&self) -> HostResult<String> {
        if let Some(language) = &self.language {
            return Ok(language.clone());
        }
        #[cfg(target_os = "android")]
        {
            return crate::secret_store::androidHostSystemLanguageCode();
        }
        #[cfg(not(target_os = "android"))]
        Err(HostError::new(
            "Android get_system_language_code requires the Android system host bridge",
        ))
    }

    fn sendNotification(&self, request: &SystemNotificationRequest) -> HostResult<()> {
        Err(HostError::new(format!(
            "Android notification requires the Android UI host bridge: {}: {}",
            request.title, request.message,
        )))
    }

    /// Modifies a setting through the Android owner's settings API.
    fn modifySystemSetting(
        &self,
        namespace: &str,
        setting: &str,
        value: &str,
    ) -> HostResult<SystemSettingData> {
        (self.settingWriter)(namespace, setting, value)
    }

    /// Reads a setting through the Android owner's settings API.
    fn getSystemSetting(&self, namespace: &str, setting: &str) -> HostResult<SystemSettingData> {
        (self.settingReader)(namespace, setting)
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

    /// Requires the application owner to provide Android device information.
    fn getDeviceInfo(&self) -> HostResult<DeviceInfoData> {
        Err(HostError::new(
            "Android get_device_info requires the Android device info host bridge",
        ))
    }

    fn captureScreenshot(&self) -> HostResult<String> {
        Err(HostError::new(
            "Android capture_screenshot requires the Android screen capture host bridge",
        ))
    }

    fn recognizeText(
        &self,
        imagePath: &str,
        language: OCRLanguage,
        quality: OCRQuality,
    ) -> HostResult<String> {
        Err(HostError::new(format!(
            "Android OCR requires the Android OCR host bridge: path={imagePath}, language={}, quality={}",
            language.asHostValue(),
            quality.asHostValue()
        )))
    }
}
