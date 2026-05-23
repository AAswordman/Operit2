use std::sync::Arc;

use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeResultData, DeviceInfoData, LocationData,
    NotificationData, SystemOperationHost, SystemSettingData,
};

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::{
    AITool, ToolExecutor, ToolValidationResult,
};

#[derive(Clone)]
pub struct StandardSystemOperationTools {
    pub systemOperationHost: Option<Arc<dyn SystemOperationHost>>,
}

#[derive(Clone, Copy)]
pub enum SystemOperationToolOperation {
    Toast,
    SendNotification,
    ModifySystemSetting,
    GetSystemSetting,
    InstallApp,
    UninstallApp,
    ListInstalledApps,
    StartApp,
    StopApp,
    GetNotifications,
    GetAppUsageTime,
    GetDeviceLocation,
    GetDeviceInfo,
}

#[derive(Clone)]
pub struct SystemOperationToolExecutor {
    pub tools: StandardSystemOperationTools,
    pub operation: SystemOperationToolOperation,
}

impl StandardSystemOperationTools {
    pub fn new(systemOperationHost: Option<Arc<dyn SystemOperationHost>>) -> Self {
        Self { systemOperationHost }
    }

    #[allow(non_snake_case)]
    pub fn toast(&self, tool: &AITool) -> ToolResult {
        let message = parameterValue(tool, "message");
        if message.is_empty() {
            return toolError(tool, "Must provide message parameter".to_string());
        }
        match self.host().and_then(|host| host.toast(&message)) {
            Ok(()) => toolSuccess(tool, "OK".to_string()),
            Err(error) => toolError(tool, format!("Toast failed: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    pub fn sendNotification(&self, tool: &AITool) -> ToolResult {
        let title = optionalParameterValue(tool, "title")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "Notification".to_string());
        let message = parameterValue(tool, "message");
        if message.is_empty() {
            return toolError(tool, "Must provide message parameter".to_string());
        }
        match self
            .host()
            .and_then(|host| host.sendNotification(&title, &message))
        {
            Ok(()) => toolSuccess(tool, "OK".to_string()),
            Err(error) => toolError(
                tool,
                format!("Failed to send notification: {}", error.message),
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn modifySystemSetting(&self, tool: &AITool) -> ToolResult {
        let setting = parameterValue(tool, "setting");
        let value = parameterValue(tool, "value");
        let namespace = optionalParameterValue(tool, "namespace").unwrap_or_else(|| "system".to_string());
        if setting.is_empty() || value.is_empty() {
            return toolError(tool, "Must provide setting and value parameters".to_string());
        }
        if !isValidNamespace(&namespace) {
            return toolError(tool, "Namespace must be one of: system, secure, global".to_string());
        }
        match self
            .host()
            .and_then(|host| host.modifySystemSetting(&namespace, &setting, &value))
        {
            Ok(data) => toolSuccess(tool, systemSettingDataToString(&data)),
            Err(error) => toolError(
                tool,
                format!("Error modifying system settings: {}", error.message),
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn getSystemSetting(&self, tool: &AITool) -> ToolResult {
        let setting = parameterValue(tool, "setting");
        let namespace = optionalParameterValue(tool, "namespace").unwrap_or_else(|| "system".to_string());
        if setting.is_empty() {
            return toolError(tool, "Must provide setting parameter".to_string());
        }
        if !isValidNamespace(&namespace) {
            return toolError(tool, "Namespace must be one of: system, secure, global".to_string());
        }
        match self
            .host()
            .and_then(|host| host.getSystemSetting(&namespace, &setting))
        {
            Ok(data) => toolSuccess(tool, systemSettingDataToString(&data)),
            Err(error) => toolError(
                tool,
                format!("Error getting system settings: {}", error.message),
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn installApp(&self, tool: &AITool) -> ToolResult {
        let apkPath = parameterValue(tool, "path");
        if apkPath.is_empty() {
            return toolError(tool, "Must provide installer file path".to_string());
        }
        match self.host().and_then(|host| host.installApp(&apkPath)) {
            Ok(data) => toolSuccess(tool, appOperationDataToString(&data)),
            Err(error) => toolError(
                tool,
                format!("Error requesting app installation: {}", error.message),
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn uninstallApp(&self, tool: &AITool) -> ToolResult {
        let packageName = parameterValue(tool, "package_name");
        if packageName.is_empty() {
            return toolError(tool, "Must provide package_name parameter".to_string());
        }
        match self.host().and_then(|host| host.uninstallApp(&packageName)) {
            Ok(data) => toolSuccess(tool, appOperationDataToString(&data)),
            Err(error) => toolError(
                tool,
                format!("Error requesting app uninstallation: {}", error.message),
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn listInstalledApps(&self, tool: &AITool) -> ToolResult {
        let includeSystemApps = parseBoolean(optionalParameterValue(tool, "include_system_apps").as_deref());
        match self
            .host()
            .and_then(|host| host.listInstalledApps(includeSystemApps))
        {
            Ok(data) => toolSuccess(tool, appListDataToString(&data)),
            Err(error) => toolError(tool, format!("Error listing installed apps: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    pub fn startApp(&self, tool: &AITool) -> ToolResult {
        let packageName = parameterValue(tool, "package_name");
        if packageName.is_empty() {
            return toolError(tool, "Must provide package_name parameter".to_string());
        }
        match self.host().and_then(|host| host.startApp(&packageName)) {
            Ok(data) => toolSuccess(tool, appOperationDataToString(&data)),
            Err(error) => toolError(tool, format!("Error starting app: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    pub fn stopApp(&self, tool: &AITool) -> ToolResult {
        let packageName = parameterValue(tool, "package_name");
        if packageName.is_empty() {
            return toolError(tool, "Must provide package_name parameter".to_string());
        }
        match self.host().and_then(|host| host.stopApp(&packageName)) {
            Ok(data) => toolSuccess(tool, appOperationDataToString(&data)),
            Err(error) => toolError(tool, format!("Error stopping app: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    pub fn getNotifications(&self, tool: &AITool) -> ToolResult {
        let limit = optionalParameterValue(tool, "limit")
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(10);
        let includeOngoing = parseBoolean(optionalParameterValue(tool, "include_ongoing").as_deref());
        match self
            .host()
            .and_then(|host| host.getNotifications(limit, includeOngoing))
        {
            Ok(data) => toolSuccess(tool, notificationDataToString(&data)),
            Err(error) => toolError(tool, format!("Error getting notifications: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    pub fn getAppUsageTime(&self, tool: &AITool) -> ToolResult {
        let packageName = parameterValue(tool, "package_name");
        let sinceHours = optionalParameterValue(tool, "since_hours")
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(24);
        let limit = optionalParameterValue(tool, "limit")
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(20);
        let includeSystemApps = parseBoolean(optionalParameterValue(tool, "include_system_apps").as_deref());
        if sinceHours <= 0 {
            return toolError(tool, "since_hours must be greater than 0".to_string());
        }
        if limit <= 0 {
            return toolError(tool, "limit must be greater than 0".to_string());
        }
        match self.host().and_then(|host| {
            host.getAppUsageTime(&packageName, sinceHours, limit, includeSystemApps)
        }) {
            Ok(data) => toolSuccess(tool, appUsageTimeResultDataToString(&data)),
            Err(error) => toolError(tool, format!("Error getting app usage time: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    pub fn getDeviceLocation(&self, tool: &AITool) -> ToolResult {
        let timeout = optionalParameterValue(tool, "timeout")
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(10);
        let highAccuracy = parseBoolean(optionalParameterValue(tool, "high_accuracy").as_deref());
        let includeAddress = optionalParameterValue(tool, "include_address")
            .map(|value| parseBoolean(Some(&value)))
            .unwrap_or(true);
        match self.host().and_then(|host| {
            host.getDeviceLocation(timeout, highAccuracy, includeAddress)
        }) {
            Ok(data) => toolSuccess(tool, locationDataToString(&data)),
            Err(error) => toolError(tool, format!("Error getting location information: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    pub fn getDeviceInfo(&self, tool: &AITool) -> ToolResult {
        match self.host().and_then(|host| host.getDeviceInfo()) {
            Ok(data) => toolSuccess(tool, deviceInfoDataToString(&data)),
            Err(error) => toolError(tool, format!("Error retrieving device info: {}", error.message)),
        }
    }

    fn host(&self) -> Result<&dyn SystemOperationHost, operit_host_api::HostError> {
        self.systemOperationHost
            .as_deref()
            .ok_or_else(|| operit_host_api::HostError::new("SystemOperationHost is not registered for this runtime."))
    }
}

impl ToolExecutor for SystemOperationToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        validateSystemOperationTool(self.operation, tool)
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        let result = match self.operation {
            SystemOperationToolOperation::Toast => self.tools.toast(tool),
            SystemOperationToolOperation::SendNotification => self.tools.sendNotification(tool),
            SystemOperationToolOperation::ModifySystemSetting => self.tools.modifySystemSetting(tool),
            SystemOperationToolOperation::GetSystemSetting => self.tools.getSystemSetting(tool),
            SystemOperationToolOperation::InstallApp => self.tools.installApp(tool),
            SystemOperationToolOperation::UninstallApp => self.tools.uninstallApp(tool),
            SystemOperationToolOperation::ListInstalledApps => self.tools.listInstalledApps(tool),
            SystemOperationToolOperation::StartApp => self.tools.startApp(tool),
            SystemOperationToolOperation::StopApp => self.tools.stopApp(tool),
            SystemOperationToolOperation::GetNotifications => self.tools.getNotifications(tool),
            SystemOperationToolOperation::GetAppUsageTime => self.tools.getAppUsageTime(tool),
            SystemOperationToolOperation::GetDeviceLocation => self.tools.getDeviceLocation(tool),
            SystemOperationToolOperation::GetDeviceInfo => self.tools.getDeviceInfo(tool),
        };
        vec![result]
    }
}

#[allow(non_snake_case)]
fn validateSystemOperationTool(
    operation: SystemOperationToolOperation,
    tool: &AITool,
) -> ToolValidationResult {
    let invalid = |message: &str| ToolValidationResult {
        valid: false,
        errorMessage: message.to_string(),
    };
    match operation {
        SystemOperationToolOperation::Toast => {
            if parameterValue(tool, "message").is_empty() {
                return invalid("message is required.");
            }
        }
        SystemOperationToolOperation::SendNotification => {
            if parameterValue(tool, "message").is_empty() {
                return invalid("message is required.");
            }
        }
        SystemOperationToolOperation::ModifySystemSetting => {
            if parameterValue(tool, "setting").is_empty() || parameterValue(tool, "value").is_empty() {
                return invalid("setting and value are required.");
            }
        }
        SystemOperationToolOperation::GetSystemSetting => {
            if parameterValue(tool, "setting").is_empty() {
                return invalid("setting is required.");
            }
        }
        SystemOperationToolOperation::InstallApp => {
            if parameterValue(tool, "path").is_empty() {
                return invalid("path is required.");
            }
        }
        SystemOperationToolOperation::UninstallApp
        | SystemOperationToolOperation::StartApp
        | SystemOperationToolOperation::StopApp => {
            if parameterValue(tool, "package_name").is_empty() {
                return invalid("package_name is required.");
            }
        }
        SystemOperationToolOperation::GetAppUsageTime => {
            if optionalParameterValue(tool, "since_hours")
                .as_deref()
                .is_some_and(|value| value.parse::<i32>().is_err())
            {
                return invalid("since_hours must be an integer.");
            }
            if optionalParameterValue(tool, "limit")
                .as_deref()
                .is_some_and(|value| value.parse::<i32>().is_err())
            {
                return invalid("limit must be an integer.");
            }
        }
        SystemOperationToolOperation::GetNotifications => {
            if optionalParameterValue(tool, "limit")
                .as_deref()
                .is_some_and(|value| value.parse::<i32>().is_err())
            {
                return invalid("limit must be an integer.");
            }
        }
        SystemOperationToolOperation::GetDeviceLocation => {
            if optionalParameterValue(tool, "timeout")
                .as_deref()
                .is_some_and(|value| value.parse::<i32>().is_err())
            {
                return invalid("timeout must be an integer.");
            }
        }
        SystemOperationToolOperation::ListInstalledApps | SystemOperationToolOperation::GetDeviceInfo => {}
    }
    ToolValidationResult {
        valid: true,
        errorMessage: String::new(),
    }
}

fn parameterValue(tool: &AITool, name: &str) -> String {
    optionalParameterValue(tool, name).unwrap_or_default()
}

fn optionalParameterValue(tool: &AITool, name: &str) -> Option<String> {
    tool.parameters
        .iter()
        .find(|parameter| parameter.name == name)
        .map(|parameter| parameter.value.trim().to_string())
}

#[allow(non_snake_case)]
fn isValidNamespace(namespace: &str) -> bool {
    matches!(namespace, "system" | "secure" | "global")
}

#[allow(non_snake_case)]
fn parseBoolean(raw: Option<&str>) -> bool {
    matches!(
        raw.map(|value| value.trim().to_lowercase()),
        Some(value) if value == "true" || value == "1" || value == "yes" || value == "y" || value == "on"
    )
}

#[allow(non_snake_case)]
fn toolSuccess(tool: &AITool, result: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result,
        error: None,
    }
}

#[allow(non_snake_case)]
fn toolError(tool: &AITool, error: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result: String::new(),
        error: Some(error),
    }
}

#[allow(non_snake_case)]
fn systemSettingDataToString(data: &SystemSettingData) -> String {
    format!("Current value of {}.{}: {}", data.namespace, data.setting, data.value)
}

#[allow(non_snake_case)]
fn appOperationDataToString(data: &AppOperationData) -> String {
    match data.operationType.as_str() {
        "install" => format!("Successfully installed app: {} {}", data.packageName, data.details),
        "uninstall" => format!("Successfully uninstalled app: {} {}", data.packageName, data.details),
        "start" => format!("Successfully started app: {} {}", data.packageName, data.details),
        "stop" => format!("Successfully stopped app: {} {}", data.packageName, data.details),
        _ => data.details.clone(),
    }
}

#[allow(non_snake_case)]
fn appListDataToString(data: &AppListData) -> String {
    let appType = if data.includesSystemApps {
        "All Apps"
    } else {
        "Third-Party Apps"
    };
    format!("Installed {appType} List:\n{}", data.packages.join("\n"))
}

#[allow(non_snake_case)]
fn notificationDataToString(data: &NotificationData) -> String {
    let mut text = format!("Device Notifications ({} total):\n", data.notifications.len());
    for (index, notification) in data.notifications.iter().enumerate() {
        text.push_str(&format!("{}. Package: {}\n", index + 1, notification.packageName));
        text.push_str(&format!("   Content: {}\n\n", notification.text));
    }
    if data.notifications.is_empty() {
        text.push_str("No notifications\n");
    }
    text
}

#[allow(non_snake_case)]
fn appUsageTimeResultDataToString(data: &AppUsageTimeResultData) -> String {
    let mut header = format!("App usage time (last {}h)", data.sinceHours);
    if let Some(packageName) = data.requestedPackageName.as_ref().filter(|value| !value.is_empty()) {
        header.push_str(&format!(" for {packageName}"));
    }
    if data.entries.is_empty() {
        return format!("{header}\nNo app usage found in the selected time window.");
    }
    let lines = data
        .entries
        .iter()
        .map(|entry| {
            format!(
                "- {} ({}): {}",
                entry.appName,
                entry.packageName,
                formatDuration(entry.totalForegroundTimeMs)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{header}\n{lines}")
}

#[allow(non_snake_case)]
fn formatDuration(durationMs: i64) -> String {
    if durationMs <= 0 {
        return "0s".to_string();
    }
    let totalSeconds = durationMs / 1000;
    let hours = totalSeconds / 3600;
    let minutes = (totalSeconds % 3600) / 60;
    let seconds = totalSeconds % 60;
    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}m"));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!("{seconds}s"));
    }
    parts.join(" ")
}

#[allow(non_snake_case)]
fn locationDataToString(data: &LocationData) -> String {
    let mut text = String::new();
    text.push_str("Device Location Information:\n");
    text.push_str(&format!("Longitude: {}\n", data.longitude));
    text.push_str(&format!("Latitude: {}\n", data.latitude));
    text.push_str(&format!("Accuracy: {} meters\n", data.accuracy));
    text.push_str(&format!("Provider: {}\n", data.provider));
    text.push_str(&format!("Timestamp: {}\n", data.timestamp));
    if !data.address.is_empty() {
        text.push_str(&format!("Address: {}\n", data.address));
    }
    if !data.city.is_empty() {
        text.push_str(&format!("City: {}\n", data.city));
    }
    if !data.province.is_empty() {
        text.push_str(&format!("Province/State: {}\n", data.province));
    }
    if !data.country.is_empty() {
        text.push_str(&format!("Country: {}\n", data.country));
    }
    text
}

#[allow(non_snake_case)]
fn deviceInfoDataToString(data: &DeviceInfoData) -> String {
    let mut text = String::new();
    text.push_str("Device Information:\n");
    text.push_str(&format!("Device ID: {}\n", data.deviceId));
    text.push_str(&format!("Model: {}\n", data.model));
    text.push_str(&format!("Manufacturer: {}\n", data.manufacturer));
    text.push_str(&format!("Android Version: {}\n", data.androidVersion));
    text.push_str(&format!("SDK Version: {}\n", data.sdkVersion));
    text.push_str(&format!("Screen Resolution: {}\n", data.screenResolution));
    text.push_str(&format!("Screen Density: {}\n", data.screenDensity));
    text.push_str(&format!("Total Memory: {}\n", data.totalMemory));
    text.push_str(&format!("Available Memory: {}\n", data.availableMemory));
    text.push_str(&format!("Total Storage: {}\n", data.totalStorage));
    text.push_str(&format!("Available Storage: {}\n", data.availableStorage));
    text.push_str(&format!("Battery Level: {}%\n", data.batteryLevel));
    text.push_str(&format!("Battery Charging: {}\n", data.batteryCharging));
    text.push_str(&format!("CPU Info: {}\n", data.cpuInfo));
    text.push_str(&format!("Network Type: {}\n", data.networkType));
    if !data.additionalInfo.is_empty() {
        text.push_str("Additional Info:\n");
        for (key, value) in &data.additionalInfo {
            text.push_str(&format!("- {}: {}\n", key, value));
        }
    }
    text
}
