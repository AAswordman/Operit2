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
pub type AndroidPackageOperation =
    Arc<dyn Fn(&str, serde_json::Value) -> HostResult<serde_json::Value> + Send + Sync>;

#[derive(Clone)]
pub struct AndroidSystemOperationHost {
    settingReader: AndroidSystemSettingReader,
    settingWriter: AndroidSystemSettingWriter,
    packageOperation: AndroidPackageOperation,
    language: Option<String>,
}

impl AndroidSystemOperationHost {
    /// Creates an Android system host with its required owner settings bindings.
    pub fn new(
        settingReader: AndroidSystemSettingReader,
        settingWriter: AndroidSystemSettingWriter,
        packageOperation: AndroidPackageOperation,
    ) -> Self {
        Self {
            settingReader,
            settingWriter,
            packageOperation,
            language: None,
        }
    }

    /// Creates the standalone Android host with an explicit boot-time locale.
    pub fn forPrivilegedRuntime(
        language: String,
        settingReader: AndroidSystemSettingReader,
        settingWriter: AndroidSystemSettingWriter,
        packageOperation: AndroidPackageOperation,
    ) -> Self {
        Self {
            settingReader,
            settingWriter,
            packageOperation,
            language: Some(language),
        }
    }

    fn packageRequest<T: serde::de::DeserializeOwned>(
        &self,
        operation: &str,
        params: serde_json::Value,
    ) -> HostResult<T> {
        let response = (self.packageOperation)(operation, params)?;
        serde_json::from_value(response).map_err(|error| {
            HostError::new(format!("Android {operation} response invalid: {error}"))
        })
    }
}

impl SystemOperationHost for AndroidSystemOperationHost {
    /// Selects the APK matching the runtime's Android ABI.
    fn applicationUpdateTarget(
        &self,
    ) -> HostResult<Option<operit_host_api::ApplicationUpdateTarget>> {
        let arch = match std::env::consts::ARCH {
            "aarch64" => "arm64-v8a",
            "arm" => "armeabi-v7a",
            arch => arch,
        };
        Ok(Some(operit_host_api::ApplicationUpdateTarget {
            installTiming: operit_host_api::AppInstallTiming::Immediate,
            arch: arch.into(),
            package: operit_host_api::ApplicationUpdatePackage::AndroidApk,
        }))
    }
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

    /// Installs a VFS file reference using the requested platform execution timing.
    fn installApp(&self, source: &dyn operit_host_api::FileSource, timing: operit_host_api::AppInstallTiming) -> HostResult<AppOperationData> {
        timing.requireImmediate()?;
        let path = source.nativePath()?;
        self.packageRequest("install_app", serde_json::json!({"path": path}))
    }

    fn uninstallApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        self.packageRequest(
            "uninstall_app",
            serde_json::json!({"packageName": packageName}),
        )
    }

    fn listInstalledApps(&self, includeSystemApps: bool) -> HostResult<AppListData> {
        self.packageRequest(
            "list_installed_apps",
            serde_json::json!({"includeSystemApps": includeSystemApps}),
        )
    }

    fn startApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        self.startAppWithActivity(packageName, None)
    }

    fn startAppWithActivity(
        &self,
        packageName: &str,
        activity: Option<&str>,
    ) -> HostResult<AppOperationData> {
        self.packageRequest(
            "start_app",
            serde_json::json!({"packageName": packageName, "activity": activity}),
        )
    }

    fn stopApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        self.packageRequest("stop_app", serde_json::json!({"packageName": packageName}))
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
        self.packageRequest("get_app_usage_time", serde_json::json!({"packageName": packageName, "sinceHours": sinceHours, "limit": limit, "includeSystemApps": includeSystemApps}))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn package_operations_forward_arguments_and_decode_responses() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let recorded = calls.clone();
        let host = AndroidSystemOperationHost::new(
            Arc::new(|_, _| Err(HostError::new("unused"))),
            Arc::new(|_, _, _| Err(HostError::new("unused"))),
            Arc::new(move |operation, params| {
                recorded
                    .lock()
                    .unwrap()
                    .push((operation.to_string(), params));
                Ok(match operation {
                    "list_installed_apps" => {
                        serde_json::json!({"includesSystemApps": true, "packages": ["app.example"]})
                    }
                    "get_app_usage_time" => {
                        serde_json::json!({"startTime": 1, "endTime": 2, "sinceHours": 3, "requestedPackageName": null, "includesSystemApps": false, "totalEntries": 0, "entries": []})
                    }
                    _ => {
                        serde_json::json!({"operationType": operation, "packageName": "app.example", "success": true, "details": "requested"})
                    }
                })
            }),
        );
        struct PackageSource;
        impl operit_host_api::FileSource for PackageSource {
            /// Supplies the native file only when the Android provider consumes the reference.
            fn nativePath(&self) -> HostResult<String> { Ok("/tmp/example.apk".into()) }
        }
        host.installApp(&PackageSource, operit_host_api::AppInstallTiming::Immediate).unwrap();
        assert!(host.installApp(&PackageSource, operit_host_api::AppInstallTiming::AfterProcessExit).is_err());
        host.uninstallApp("app.example").unwrap();
        assert_eq!(
            host.listInstalledApps(true).unwrap().packages,
            ["app.example"]
        );
        host.startAppWithActivity("app.example", Some(".MainActivity"))
            .unwrap();
        host.stopApp("app.example").unwrap();
        assert_eq!(
            host.getAppUsageTime("", 3, 10, false).unwrap().totalEntries,
            0
        );
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 6);
        assert_eq!(calls[3].1["activity"], ".MainActivity");
        assert_eq!(calls[5].1["sinceHours"], 3);
    }

    #[test]
    fn malformed_package_response_is_an_error() {
        let host = AndroidSystemOperationHost::new(
            Arc::new(|_, _| Err(HostError::new("unused"))),
            Arc::new(|_, _, _| Err(HostError::new("unused"))),
            Arc::new(|_, _| Ok(serde_json::json!({}))),
        );
        assert!(host.startApp("app.example").is_err());
    }
}
