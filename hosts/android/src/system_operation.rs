use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeResultData, DeviceInfoData, HostError, HostResult,
    LocationData, NotificationData, SystemOperationHost, SystemSettingData,
};

#[derive(Clone, Debug, Default)]
pub struct AndroidSystemOperationHost;

impl AndroidSystemOperationHost {
    pub fn new() -> Self {
        Self
    }
}

impl SystemOperationHost for AndroidSystemOperationHost {
    fn getSystemLanguageCode(&self) -> HostResult<String> {
        Err(HostError::new(
            "Android get_system_language_code requires the Android system host bridge",
        ))
    }

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
