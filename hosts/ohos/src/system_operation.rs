use std::collections::BTreeMap;
use std::sync::Arc;

use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeEntry, AppUsageTimeResultData, DeviceInfoData,
    HostError, HostResult, LocationData, NotificationData, NotificationEntry, OCRLanguage,
    OCRQuality, SystemOperationHost, SystemSettingData,
};
use serde_json::{json, Value};

pub type OhosLanguageReader = Arc<dyn Fn() -> HostResult<String> + Send + Sync>;
pub type OhosScreenshotCapturer = Arc<dyn Fn() -> HostResult<String> + Send + Sync>;
pub type OhosTextRecognizer =
    Arc<dyn Fn(&str, OCRLanguage, OCRQuality) -> HostResult<String> + Send + Sync>;
pub type OhosSystemController = Arc<dyn Fn(&str, Value) -> HostResult<Value> + Send + Sync>;

#[derive(Clone)]
pub struct OhosSystemOperationHost {
    languageReader: OhosLanguageReader,
    screenshotCapturer: OhosScreenshotCapturer,
    textRecognizer: OhosTextRecognizer,
    controller: OhosSystemController,
}

impl OhosSystemOperationHost {
    /// Creates an OpenHarmony system host from owner-provided platform callbacks.
    pub fn fromOwnerCallbacks(
        languageReader: OhosLanguageReader,
        screenshotCapturer: OhosScreenshotCapturer,
        textRecognizer: OhosTextRecognizer,
        controller: OhosSystemController,
    ) -> Self {
        Self {
            languageReader,
            screenshotCapturer,
            textRecognizer,
            controller,
        }
    }

    /// Executes one owner-host system operation.
    fn execute(&self, operation: &str, params: Value) -> HostResult<Value> {
        (self.controller)(operation, params)
    }

    /// Reads a required string field from a JSON object.
    fn stringField(value: &Value, key: &str) -> HostResult<String> {
        value
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| HostError::new(format!("OpenHarmony system response requires {key}")))
    }

    /// Reads a required boolean field from a JSON object.
    fn boolField(value: &Value, key: &str) -> HostResult<bool> {
        value
            .get(key)
            .and_then(Value::as_bool)
            .ok_or_else(|| HostError::new(format!("OpenHarmony system response requires {key}")))
    }

    /// Reads a required i64 field from a JSON object.
    fn i64Field(value: &Value, key: &str) -> HostResult<i64> {
        value
            .get(key)
            .and_then(Value::as_i64)
            .ok_or_else(|| HostError::new(format!("OpenHarmony system response requires {key}")))
    }

    /// Reads a required i32 field from a JSON object.
    fn i32Field(value: &Value, key: &str) -> HostResult<i32> {
        let raw = Self::i64Field(value, key)?;
        i32::try_from(raw).map_err(|_| {
            HostError::new(format!("OpenHarmony system response {key} is out of range"))
        })
    }

    /// Reads a required f64 field from a JSON object.
    fn f64Field(value: &Value, key: &str) -> HostResult<f64> {
        value
            .get(key)
            .and_then(Value::as_f64)
            .ok_or_else(|| HostError::new(format!("OpenHarmony system response requires {key}")))
    }

    /// Reads a required f32 field from a JSON object.
    fn f32Field(value: &Value, key: &str) -> HostResult<f32> {
        Ok(Self::f64Field(value, key)? as f32)
    }

    /// Reads a required array field from a JSON object.
    fn arrayField<'a>(value: &'a Value, key: &str) -> HostResult<&'a Vec<Value>> {
        value
            .get(key)
            .and_then(Value::as_array)
            .ok_or_else(|| HostError::new(format!("OpenHarmony system response requires {key}")))
    }

    /// Parses a system setting response from owner JSON.
    fn systemSetting(value: Value) -> HostResult<SystemSettingData> {
        Ok(SystemSettingData {
            namespace: Self::stringField(&value, "namespace")?,
            setting: Self::stringField(&value, "setting")?,
            value: Self::stringField(&value, "value")?,
        })
    }

    /// Parses an app operation response from owner JSON.
    fn appOperation(value: Value) -> HostResult<AppOperationData> {
        Ok(AppOperationData {
            operationType: Self::stringField(&value, "operationType")?,
            packageName: Self::stringField(&value, "packageName")?,
            success: Self::boolField(&value, "success")?,
            details: Self::stringField(&value, "details")?,
        })
    }

    /// Parses an app list response from owner JSON.
    fn appList(value: Value) -> HostResult<AppListData> {
        let packages = Self::arrayField(&value, "packages")?
            .iter()
            .map(|entry| {
                entry
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| HostError::new("OpenHarmony app list package must be a string"))
            })
            .collect::<HostResult<Vec<_>>>()?;
        Ok(AppListData {
            includesSystemApps: Self::boolField(&value, "includesSystemApps")?,
            packages,
        })
    }

    /// Parses notification entries from owner JSON.
    fn notifications(value: Value) -> HostResult<NotificationData> {
        let notifications = Self::arrayField(&value, "notifications")?
            .iter()
            .map(|entry| {
                Ok(NotificationEntry {
                    packageName: Self::stringField(entry, "packageName")?,
                    text: Self::stringField(entry, "text")?,
                    timestamp: Self::i64Field(entry, "timestamp")?,
                })
            })
            .collect::<HostResult<Vec<_>>>()?;
        Ok(NotificationData {
            notifications,
            timestamp: Self::i64Field(&value, "timestamp")?,
        })
    }

    /// Parses app usage entries from owner JSON.
    fn appUsage(value: Value) -> HostResult<AppUsageTimeResultData> {
        let requestedPackageName = match value.get("requestedPackageName") {
            Some(Value::String(package)) => Some(package.clone()),
            Some(Value::Null) => None,
            _ => {
                return Err(HostError::new(
                    "OpenHarmony app usage response requires requestedPackageName",
                ))
            }
        };
        let entries = Self::arrayField(&value, "entries")?
            .iter()
            .map(|entry| {
                Ok(AppUsageTimeEntry {
                    packageName: Self::stringField(entry, "packageName")?,
                    appName: Self::stringField(entry, "appName")?,
                    totalForegroundTimeMs: Self::i64Field(entry, "totalForegroundTimeMs")?,
                    lastTimeUsed: Self::i64Field(entry, "lastTimeUsed")?,
                    isSystemApp: Self::boolField(entry, "isSystemApp")?,
                })
            })
            .collect::<HostResult<Vec<_>>>()?;
        Ok(AppUsageTimeResultData {
            startTime: Self::i64Field(&value, "startTime")?,
            endTime: Self::i64Field(&value, "endTime")?,
            sinceHours: Self::i32Field(&value, "sinceHours")?,
            requestedPackageName,
            includesSystemApps: Self::boolField(&value, "includesSystemApps")?,
            totalEntries: Self::i32Field(&value, "totalEntries")?,
            entries,
        })
    }

    /// Parses a location response from owner JSON.
    fn location(value: Value) -> HostResult<LocationData> {
        Ok(LocationData {
            latitude: Self::f64Field(&value, "latitude")?,
            longitude: Self::f64Field(&value, "longitude")?,
            accuracy: Self::f32Field(&value, "accuracy")?,
            provider: Self::stringField(&value, "provider")?,
            timestamp: Self::i64Field(&value, "timestamp")?,
            rawData: Self::stringField(&value, "rawData")?,
            address: Self::stringField(&value, "address")?,
            city: Self::stringField(&value, "city")?,
            province: Self::stringField(&value, "province")?,
            country: Self::stringField(&value, "country")?,
        })
    }

    /// Parses device information from owner JSON.
    fn deviceInfo(value: Value) -> HostResult<DeviceInfoData> {
        let additionalInfoValue = value
            .get("additionalInfo")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                HostError::new("OpenHarmony device info response requires additionalInfo")
            })?;
        let mut additionalInfo = BTreeMap::new();
        for (key, entry) in additionalInfoValue {
            let Some(text) = entry.as_str() else {
                return Err(HostError::new(format!(
                    "OpenHarmony device info additionalInfo {key} must be a string"
                )));
            };
            additionalInfo.insert(key.clone(), text.to_string());
        }
        Ok(DeviceInfoData {
            deviceId: Self::stringField(&value, "deviceId")?,
            model: Self::stringField(&value, "model")?,
            manufacturer: Self::stringField(&value, "manufacturer")?,
            androidVersion: Self::stringField(&value, "androidVersion")?,
            sdkVersion: Self::i32Field(&value, "sdkVersion")?,
            screenResolution: Self::stringField(&value, "screenResolution")?,
            screenDensity: Self::f32Field(&value, "screenDensity")?,
            totalMemory: Self::stringField(&value, "totalMemory")?,
            availableMemory: Self::stringField(&value, "availableMemory")?,
            totalStorage: Self::stringField(&value, "totalStorage")?,
            availableStorage: Self::stringField(&value, "availableStorage")?,
            batteryLevel: Self::i32Field(&value, "batteryLevel")?,
            batteryCharging: Self::boolField(&value, "batteryCharging")?,
            cpuInfo: Self::stringField(&value, "cpuInfo")?,
            networkType: Self::stringField(&value, "networkType")?,
            additionalInfo,
        })
    }
}

impl SystemOperationHost for OhosSystemOperationHost {
    /// Reads the OpenHarmony system language code through the owner app.
    fn getSystemLanguageCode(&self) -> HostResult<String> {
        (self.languageReader)()
    }

    /// Shows a toast through the OpenHarmony owner app.
    fn toast(&self, message: &str) -> HostResult<()> {
        self.execute("toast", json!({ "message": message }))?;
        Ok(())
    }

    /// Sends a notification through the OpenHarmony owner app.
    fn sendNotification(&self, title: &str, message: &str) -> HostResult<()> {
        self.execute(
            "send_notification",
            json!({ "title": title, "message": message }),
        )?;
        Ok(())
    }

    /// Modifies an OpenHarmony system setting through the owner app.
    fn modifySystemSetting(
        &self,
        namespace: &str,
        setting: &str,
        value: &str,
    ) -> HostResult<SystemSettingData> {
        Self::systemSetting(self.execute(
            "modify_system_setting",
            json!({ "namespace": namespace, "setting": setting, "value": value }),
        )?)
    }

    /// Reads an OpenHarmony system setting through the owner app.
    fn getSystemSetting(&self, namespace: &str, setting: &str) -> HostResult<SystemSettingData> {
        Self::systemSetting(self.execute(
            "get_system_setting",
            json!({ "namespace": namespace, "setting": setting }),
        )?)
    }

    /// Installs an app through the OpenHarmony owner app.
    fn installApp(&self, path: &str) -> HostResult<AppOperationData> {
        Self::appOperation(self.execute("install_app", json!({ "path": path }))?)
    }

    /// Uninstalls an app through the OpenHarmony owner app.
    fn uninstallApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        Self::appOperation(self.execute("uninstall_app", json!({ "packageName": packageName }))?)
    }

    /// Lists installed apps through the OpenHarmony owner app.
    fn listInstalledApps(&self, includeSystemApps: bool) -> HostResult<AppListData> {
        Self::appList(self.execute(
            "list_installed_apps",
            json!({ "includeSystemApps": includeSystemApps }),
        )?)
    }

    /// Starts an app through the OpenHarmony owner app.
    fn startApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        Self::appOperation(self.execute("start_app", json!({ "packageName": packageName }))?)
    }

    /// Stops an app through the OpenHarmony owner app.
    fn stopApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        Self::appOperation(self.execute("stop_app", json!({ "packageName": packageName }))?)
    }

    /// Reads notifications through the OpenHarmony owner app.
    fn getNotifications(&self, limit: i32, includeOngoing: bool) -> HostResult<NotificationData> {
        Self::notifications(self.execute(
            "get_notifications",
            json!({ "limit": limit, "includeOngoing": includeOngoing }),
        )?)
    }

    /// Reads app usage time through the OpenHarmony owner app.
    fn getAppUsageTime(
        &self,
        packageName: &str,
        sinceHours: i32,
        limit: i32,
        includeSystemApps: bool,
    ) -> HostResult<AppUsageTimeResultData> {
        Self::appUsage(self.execute(
            "get_app_usage_time",
            json!({
                "packageName": packageName,
                "sinceHours": sinceHours,
                "limit": limit,
                "includeSystemApps": includeSystemApps
            }),
        )?)
    }

    /// Reads device location through the OpenHarmony owner app.
    fn getDeviceLocation(
        &self,
        timeout: i32,
        highAccuracy: bool,
        includeAddress: bool,
    ) -> HostResult<LocationData> {
        Self::location(self.execute(
            "get_device_location",
            json!({
                "timeout": timeout,
                "highAccuracy": highAccuracy,
                "includeAddress": includeAddress
            }),
        )?)
    }

    /// Reads OpenHarmony device information through the owner app.
    fn getDeviceInfo(&self) -> HostResult<DeviceInfoData> {
        Self::deviceInfo(self.execute("device_info", json!({}))?)
    }

    /// Captures an OpenHarmony screenshot through the owner app.
    fn captureScreenshot(&self) -> HostResult<String> {
        (self.screenshotCapturer)()
    }

    /// Recognizes text from an image through the owner app.
    fn recognizeText(
        &self,
        imagePath: &str,
        language: OCRLanguage,
        quality: OCRQuality,
    ) -> HostResult<String> {
        (self.textRecognizer)(imagePath, language, quality)
    }
}
