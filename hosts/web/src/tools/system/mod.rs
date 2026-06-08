use js_sys::{Array, Reflect};
use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeEntry, AppUsageTimeResultData, DeviceInfoData,
    HostResult, LocationData, NotificationData, NotificationEntry, SystemOperationHost,
    SystemSettingData,
};
use wasm_bindgen::prelude::*;

use crate::common::{
    app_operation_data, call_system, js_error, js_string_array, js_string_map,
    read_bool_property, read_f32_property, read_f64_property, read_i32_property, read_i64_property,
    read_optional_string_property, read_string_property, system_setting_data,
};

#[derive(Clone, Debug, Default)]
pub struct WebSystemOperationHost;

unsafe impl Send for WebSystemOperationHost {}
unsafe impl Sync for WebSystemOperationHost {}

impl WebSystemOperationHost {
    pub fn new() -> Self {
        Self
    }
}

impl SystemOperationHost for WebSystemOperationHost {
    fn getSystemLanguageCode(&self) -> HostResult<String> {
        read_string_property(&call_system("getSystemLanguageCode", &[])?, "languageCode")
    }

    fn toast(&self, message: &str) -> HostResult<()> {
        call_system("toast", &[JsValue::from_str(message)])?;
        Ok(())
    }

    fn sendNotification(&self, title: &str, message: &str) -> HostResult<()> {
        call_system(
            "sendNotification",
            &[JsValue::from_str(title), JsValue::from_str(message)],
        )?;
        Ok(())
    }

    fn modifySystemSetting(
        &self,
        namespace: &str,
        setting: &str,
        value: &str,
    ) -> HostResult<SystemSettingData> {
        let value = call_system(
            "modifySystemSetting",
            &[
                JsValue::from_str(namespace),
                JsValue::from_str(setting),
                JsValue::from_str(value),
            ],
        )?;
        system_setting_data(value)
    }

    fn getSystemSetting(&self, namespace: &str, setting: &str) -> HostResult<SystemSettingData> {
        let value = call_system(
            "getSystemSetting",
            &[JsValue::from_str(namespace), JsValue::from_str(setting)],
        )?;
        system_setting_data(value)
    }

    fn installApp(&self, path: &str) -> HostResult<AppOperationData> {
        app_operation_data(call_system("installApp", &[JsValue::from_str(path)])?)
    }

    fn uninstallApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        app_operation_data(call_system("uninstallApp", &[JsValue::from_str(packageName)])?)
    }

    fn listInstalledApps(&self, includeSystemApps: bool) -> HostResult<AppListData> {
        let value = call_system("listInstalledApps", &[JsValue::from_bool(includeSystemApps)])?;
        Ok(AppListData {
            includesSystemApps: read_bool_property(&value, "includesSystemApps")?,
            packages: js_string_array(
                Reflect::get(&value, &JsValue::from_str("packages")).map_err(js_error)?,
                "packages",
            )?,
        })
    }

    fn startApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        app_operation_data(call_system("startApp", &[JsValue::from_str(packageName)])?)
    }

    fn stopApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        app_operation_data(call_system("stopApp", &[JsValue::from_str(packageName)])?)
    }

    fn getNotifications(&self, limit: i32, includeOngoing: bool) -> HostResult<NotificationData> {
        let value = call_system(
            "getNotifications",
            &[JsValue::from_f64(limit as f64), JsValue::from_bool(includeOngoing)],
        )?;
        let notifications = Array::from(
            &Reflect::get(&value, &JsValue::from_str("notifications")).map_err(js_error)?,
        );
        let mut parsed = Vec::new();
        for index in 0..notifications.length() {
            let entry = notifications.get(index);
            parsed.push(NotificationEntry {
                packageName: read_string_property(&entry, "packageName")?,
                text: read_string_property(&entry, "text")?,
                timestamp: read_i64_property(&entry, "timestamp")?,
            });
        }
        Ok(NotificationData {
            notifications: parsed,
            timestamp: read_i64_property(&value, "timestamp")?,
        })
    }

    fn getAppUsageTime(
        &self,
        packageName: &str,
        sinceHours: i32,
        limit: i32,
        includeSystemApps: bool,
    ) -> HostResult<AppUsageTimeResultData> {
        let value = call_system(
            "getAppUsageTime",
            &[
                JsValue::from_str(packageName),
                JsValue::from_f64(sinceHours as f64),
                JsValue::from_f64(limit as f64),
                JsValue::from_bool(includeSystemApps),
            ],
        )?;
        let entries = Array::from(&Reflect::get(&value, &JsValue::from_str("entries")).map_err(js_error)?);
        let mut parsed_entries = Vec::new();
        for index in 0..entries.length() {
            let entry = entries.get(index);
            parsed_entries.push(AppUsageTimeEntry {
                packageName: read_string_property(&entry, "packageName")?,
                appName: read_string_property(&entry, "appName")?,
                totalForegroundTimeMs: read_i64_property(&entry, "totalForegroundTimeMs")?,
                lastTimeUsed: read_i64_property(&entry, "lastTimeUsed")?,
                isSystemApp: read_bool_property(&entry, "isSystemApp")?,
            });
        }
        Ok(AppUsageTimeResultData {
            startTime: read_i64_property(&value, "startTime")?,
            endTime: read_i64_property(&value, "endTime")?,
            sinceHours: read_i32_property(&value, "sinceHours")?,
            requestedPackageName: read_optional_string_property(&value, "requestedPackageName")?,
            includesSystemApps: read_bool_property(&value, "includesSystemApps")?,
            totalEntries: read_i32_property(&value, "totalEntries")?,
            entries: parsed_entries,
        })
    }

    fn getDeviceLocation(
        &self,
        timeout: i32,
        highAccuracy: bool,
        includeAddress: bool,
    ) -> HostResult<LocationData> {
        let value = call_system(
            "getDeviceLocation",
            &[
                JsValue::from_f64(timeout as f64),
                JsValue::from_bool(highAccuracy),
                JsValue::from_bool(includeAddress),
            ],
        )?;
        Ok(LocationData {
            latitude: read_f64_property(&value, "latitude")?,
            longitude: read_f64_property(&value, "longitude")?,
            accuracy: read_f32_property(&value, "accuracy")?,
            provider: read_string_property(&value, "provider")?,
            timestamp: read_i64_property(&value, "timestamp")?,
            rawData: read_string_property(&value, "rawData")?,
            address: read_string_property(&value, "address")?,
            city: read_string_property(&value, "city")?,
            province: read_string_property(&value, "province")?,
            country: read_string_property(&value, "country")?,
        })
    }

    fn getDeviceInfo(&self) -> HostResult<DeviceInfoData> {
        let value = call_system("getDeviceInfo", &[])?;
        Ok(DeviceInfoData {
            deviceId: read_string_property(&value, "deviceId")?,
            model: read_string_property(&value, "model")?,
            manufacturer: read_string_property(&value, "manufacturer")?,
            androidVersion: read_string_property(&value, "androidVersion")?,
            sdkVersion: read_i32_property(&value, "sdkVersion")?,
            screenResolution: read_string_property(&value, "screenResolution")?,
            screenDensity: read_f32_property(&value, "screenDensity")?,
            totalMemory: read_string_property(&value, "totalMemory")?,
            availableMemory: read_string_property(&value, "availableMemory")?,
            totalStorage: read_string_property(&value, "totalStorage")?,
            availableStorage: read_string_property(&value, "availableStorage")?,
            batteryLevel: read_i32_property(&value, "batteryLevel")?,
            batteryCharging: read_bool_property(&value, "batteryCharging")?,
            cpuInfo: read_string_property(&value, "cpuInfo")?,
            networkType: read_string_property(&value, "networkType")?,
            additionalInfo: js_string_map(
                Reflect::get(&value, &JsValue::from_str("additionalInfo")).map_err(js_error)?,
                "additionalInfo",
            )?,
        })
    }
}
