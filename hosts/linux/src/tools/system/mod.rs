use std::fs;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, path::PathBuf};

use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeEntry, AppUsageTimeResultData, DeviceInfoData,
    HostError, HostResult, LocationData, NotificationData, NotificationEntry, SystemOperationHost,
    SystemSettingData,
};
use regex::Regex;
use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub struct LinuxSystemOperationHost;

impl LinuxSystemOperationHost {
    pub fn new() -> Self {
        Self
    }
}

impl SystemOperationHost for LinuxSystemOperationHost {
    fn toast(&self, message: &str) -> HostResult<()> {
        if message.trim().is_empty() {
            return Err(HostError::new("Must provide message parameter"));
        }
        let status = Command::new("notify-send")
            .arg("Operit")
            .arg(message)
            .status()
            .map_err(|error| HostError::new(format!("Toast failed: {error}")))?;
        if status.success() {
            Ok(())
        } else {
            Err(HostError::new(format!("Toast command exited with {status}")))
        }
    }

    fn sendNotification(&self, title: &str, message: &str) -> HostResult<()> {
        let title = if title.trim().is_empty() {
            "Notification"
        } else {
            title
        };
        let status = Command::new("notify-send")
            .arg(title)
            .arg(message)
            .status()
            .map_err(|error| HostError::new(format!("Failed to send notification: {error}")))?;
        if status.success() {
            Ok(())
        } else {
            Err(HostError::new(format!(
                "Notification command exited with {status}"
            )))
        }
    }

    fn modifySystemSetting(
        &self,
        namespace: &str,
        setting: &str,
        value: &str,
    ) -> HostResult<SystemSettingData> {
        modify_linux_system_setting(namespace, setting, value)
    }

    fn getSystemSetting(&self, namespace: &str, setting: &str) -> HostResult<SystemSettingData> {
        get_linux_system_setting(namespace, setting)
    }

    fn installApp(&self, path: &str) -> HostResult<AppOperationData> {
        request_linux_install_app(path)
    }

    fn uninstallApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        request_linux_uninstall_app(packageName)
    }

    fn listInstalledApps(&self, includeSystemApps: bool) -> HostResult<AppListData> {
        let mut packages = Vec::new();
        let applicationsDir = Path::new("/usr/share/applications");
        for entry in fs::read_dir(applicationsDir)
            .map_err(|error| HostError::new(format!("Failed to list desktop applications: {error}")))?
        {
            let entry = entry
                .map_err(|error| HostError::new(format!("Failed to read desktop entry: {error}")))?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("desktop") {
                continue;
            }
            let content = fs::read_to_string(&path)
                .map_err(|error| HostError::new(format!("Failed to read {}: {error}", path.display())))?;
            if let Some(name) = parseDesktopName(&content) {
                packages.push(name);
            }
        }
        packages.sort();
        packages.dedup();
        Ok(AppListData {
            includesSystemApps: includeSystemApps,
            packages,
        })
    }

    fn startApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        Command::new(packageName)
            .spawn()
            .map_err(|error| HostError::new(format!("Error starting app: {error}")))?;
        Ok(AppOperationData {
            operationType: "start".to_string(),
            packageName: packageName.to_string(),
            success: true,
            details: "Start request sent".to_string(),
        })
    }

    fn stopApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        let status = Command::new("pkill")
            .arg("-f")
            .arg(packageName)
            .status()
            .map_err(|error| HostError::new(format!("Error stopping app: {error}")))?;
        if !status.success() {
            return Err(HostError::new(format!(
                "Error stopping app: pkill exited with {status}"
            )));
        }
        Ok(AppOperationData {
            operationType: "stop".to_string(),
            packageName: packageName.to_string(),
            success: true,
            details: "Stop request sent".to_string(),
        })
    }

    fn getNotifications(&self, _limit: i32, _includeOngoing: bool) -> HostResult<NotificationData> {
        get_linux_notifications(_limit)
    }

    fn getAppUsageTime(
        &self,
        packageName: &str,
        sinceHours: i32,
        limit: i32,
        includeSystemApps: bool,
    ) -> HostResult<AppUsageTimeResultData> {
        get_linux_app_usage_time(packageName, sinceHours, limit, includeSystemApps)
    }

    fn getDeviceLocation(
        &self,
        timeout: i32,
        highAccuracy: bool,
        includeAddress: bool,
    ) -> HostResult<LocationData> {
        get_linux_device_location(timeout, highAccuracy, includeAddress)
    }

    fn getDeviceInfo(&self) -> HostResult<DeviceInfoData> {
        get_linux_device_info()
    }
}

fn get_linux_notifications(limit: i32) -> HostResult<NotificationData> {
    if limit <= 0 {
        return Err(HostError::new("limit must be greater than 0"));
    }
    let output = Command::new("dunstctl")
        .arg("history")
        .output()
        .map_err(|error| HostError::new(format!("Failed to query Linux notifications with dunstctl: {error}")))?;
    if !output.status.success() {
        return Err(HostError::new(format!(
            "Failed to query Linux notifications with dunstctl: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let value: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| HostError::new(format!("Failed to parse dunst notification history: {error}")))?;
    let mut notifications = Vec::new();
    collect_dunst_notifications(&value, &mut notifications);
    notifications.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
    notifications.truncate(limit as usize);
    Ok(NotificationData {
        notifications,
        timestamp: unix_time_millis()?,
    })
}

fn collect_dunst_notifications(value: &Value, notifications: &mut Vec<NotificationEntry>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_dunst_notifications(item, notifications);
            }
        }
        Value::Object(map) => {
            let hasNotificationFields = map.contains_key("appname")
                || map.contains_key("summary")
                || map.contains_key("body");
            if hasNotificationFields {
                let packageName = dunst_data_string(map.get("appname")).unwrap_or_else(|| "dunst".to_string());
                let summary = dunst_data_string(map.get("summary")).unwrap_or_default();
                let body = dunst_data_string(map.get("body")).unwrap_or_default();
                let text = [summary, body]
                    .into_iter()
                    .filter(|part| !part.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                let timestamp = dunst_data_i64(map.get("timestamp")).unwrap_or(0);
                notifications.push(NotificationEntry {
                    packageName,
                    text,
                    timestamp,
                });
            } else {
                for item in map.values() {
                    collect_dunst_notifications(item, notifications);
                }
            }
        }
        _ => {}
    }
}

fn dunst_data_string(value: Option<&Value>) -> Option<String> {
    let value = value?;
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    value
        .get("data")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn dunst_data_i64(value: Option<&Value>) -> Option<i64> {
    let value = value?;
    if let Some(number) = value.as_i64() {
        return Some(number);
    }
    value.get("data").and_then(Value::as_i64)
}

fn request_linux_install_app(path: &str) -> HostResult<AppOperationData> {
    if path.trim().is_empty() {
        return Err(HostError::new("path is required"));
    }
    let path = Path::new(path);
    if !path.exists() {
        return Err(HostError::new(format!(
            "Installer path does not exist: {}",
            path.display()
        )));
    }
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map_err(|error| HostError::new(format!("Error requesting Linux app installation: {error}")))?;
    Ok(AppOperationData {
        operationType: "install".to_string(),
        packageName: path.display().to_string(),
        success: true,
        details: "Install request opened with xdg-open".to_string(),
    })
}

fn request_linux_uninstall_app(packageName: &str) -> HostResult<AppOperationData> {
    let packageName = packageName.trim();
    if packageName.is_empty() {
        return Err(HostError::new("package_name is required"));
    }
    if packageName.ends_with(".desktop") {
        let desktopPath = local_applications_dir()?.join(packageName);
        if !desktopPath.exists() {
            return Err(HostError::new(format!(
                "Local desktop entry does not exist: {}",
                desktopPath.display()
            )));
        }
        fs::remove_file(&desktopPath).map_err(|error| {
            HostError::new(format!(
                "Failed to remove local desktop entry {}: {error}",
                desktopPath.display()
            ))
        })?;
        return Ok(AppOperationData {
            operationType: "uninstall".to_string(),
            packageName: packageName.to_string(),
            success: true,
            details: format!("Removed local desktop entry {}", desktopPath.display()),
        });
    }

    let status = Command::new("flatpak")
        .args(["uninstall", "-y", packageName])
        .status()
        .map_err(|error| HostError::new(format!("Failed to request Flatpak uninstall: {error}")))?;
    if !status.success() {
        return Err(HostError::new(format!(
            "Flatpak uninstall exited with {status}. Linux host supports uninstall_app for Flatpak app IDs or local .desktop entries."
        )));
    }
    Ok(AppOperationData {
        operationType: "uninstall".to_string(),
        packageName: packageName.to_string(),
        success: true,
        details: "Flatpak uninstall request completed".to_string(),
    })
}

fn modify_linux_system_setting(
    namespace: &str,
    setting: &str,
    value: &str,
) -> HostResult<SystemSettingData> {
    let target = linux_gsettings_target(setting)?;
    let status = Command::new("gsettings")
        .args(["set", &target.schema, &target.key, value])
        .status()
        .map_err(|error| HostError::new(format!("Failed to modify Linux setting with gsettings: {error}")))?;
    if !status.success() {
        return Err(HostError::new(format!(
            "gsettings set exited with {status}; setting must use schema:key form."
        )));
    }
    get_linux_system_setting(namespace, setting)
}

fn get_linux_system_setting(namespace: &str, setting: &str) -> HostResult<SystemSettingData> {
    let target = linux_gsettings_target(setting)?;
    let output = Command::new("gsettings")
        .args(["get", &target.schema, &target.key])
        .output()
        .map_err(|error| HostError::new(format!("Failed to get Linux setting with gsettings: {error}")))?;
    if !output.status.success() {
        return Err(HostError::new(format!(
            "gsettings get exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(SystemSettingData {
        namespace: namespace.to_string(),
        setting: setting.to_string(),
        value: String::from_utf8_lossy(&output.stdout).trim().to_string(),
    })
}

struct LinuxGSettingsTarget {
    schema: String,
    key: String,
}

fn linux_gsettings_target(setting: &str) -> HostResult<LinuxGSettingsTarget> {
    let setting = setting.trim();
    let Some((schema, key)) = setting.split_once(':') else {
        return Err(HostError::new(
            "Linux get_system_setting/modify_system_setting requires setting in gsettings schema:key form.",
        ));
    };
    let schema = schema.trim();
    let key = key.trim();
    if schema.is_empty() || key.is_empty() {
        return Err(HostError::new(
            "Linux gsettings schema and key must both be non-empty.",
        ));
    }
    Ok(LinuxGSettingsTarget {
        schema: schema.to_string(),
        key: key.to_string(),
    })
}

fn get_linux_app_usage_time(
    packageName: &str,
    sinceHours: i32,
    limit: i32,
    includeSystemApps: bool,
) -> HostResult<AppUsageTimeResultData> {
    if sinceHours <= 0 {
        return Err(HostError::new("since_hours must be greater than 0"));
    }
    if limit <= 0 {
        return Err(HostError::new("limit must be greater than 0"));
    }
    let now = unix_time_millis()?;
    let startTime = now - i64::from(sinceHours) * 60 * 60 * 1000;
    let output = Command::new("ps")
        .args(["-eo", "comm=,etimes=,args="])
        .output()
        .map_err(|error| HostError::new(format!("Failed to query Linux processes: {error}")))?;
    if !output.status.success() {
        return Err(HostError::new(format!(
            "Failed to query Linux processes: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let mut entries = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 2 {
            continue;
        }
        let commandName = parts[0].to_string();
        if !packageName.trim().is_empty() && !commandName.contains(packageName.trim()) {
            continue;
        }
        let elapsedSeconds = match parts[1].parse::<i64>() {
            Ok(value) => value,
            Err(_) => continue,
        };
        let argsText = if parts.len() > 2 {
            parts[2..].join(" ")
        } else {
            commandName.clone()
        };
        let isSystemApp = argsText.starts_with("/usr/sbin/")
            || argsText.starts_with("/sbin/")
            || argsText.starts_with('[');
        if !includeSystemApps && isSystemApp {
            continue;
        }
        let elapsedMs = elapsedSeconds.saturating_mul(1000);
        let windowMs = i64::from(sinceHours) * 60 * 60 * 1000;
        entries.push(AppUsageTimeEntry {
            packageName: commandName.clone(),
            appName: commandName,
            totalForegroundTimeMs: elapsedMs.min(windowMs),
            lastTimeUsed: now,
            isSystemApp,
        });
    }
    entries.sort_by(|left, right| right.totalForegroundTimeMs.cmp(&left.totalForegroundTimeMs));
    entries.truncate(limit as usize);
    Ok(AppUsageTimeResultData {
        startTime,
        endTime: now,
        sinceHours,
        requestedPackageName: if packageName.trim().is_empty() {
            None
        } else {
            Some(packageName.trim().to_string())
        },
        includesSystemApps: includeSystemApps,
        totalEntries: entries.len() as i32,
        entries,
    })
}

fn get_linux_device_location(
    timeout: i32,
    highAccuracy: bool,
    includeAddress: bool,
) -> HostResult<LocationData> {
    if timeout <= 0 {
        return Err(HostError::new("timeout must be greater than 0"));
    }
    let accuracyLevel = if highAccuracy { "8" } else { "6" };
    let script = format!(
        r#"
set -eu
client_output=$(gdbus call --system --dest org.freedesktop.GeoClue2 --object-path /org/freedesktop/GeoClue2/Manager --method org.freedesktop.GeoClue2.Manager.GetClient)
client_path=$(printf '%s\n' "$client_output" | sed -n "s/.*objectpath '\([^']*\)'.*/\1/p")
if [ -z "$client_path" ]; then
  echo "GeoClue client path was not returned." >&2
  exit 2
fi
gdbus call --system --dest org.freedesktop.GeoClue2 --object-path "$client_path" --method org.freedesktop.DBus.Properties.Set org.freedesktop.GeoClue2.Client DesktopId "<'operit2'>" >/dev/null
gdbus call --system --dest org.freedesktop.GeoClue2 --object-path "$client_path" --method org.freedesktop.DBus.Properties.Set org.freedesktop.GeoClue2.Client RequestedAccuracyLevel "<uint32 {accuracyLevel}>" >/dev/null
gdbus call --system --dest org.freedesktop.GeoClue2 --object-path "$client_path" --method org.freedesktop.GeoClue2.Client.Start >/dev/null
deadline=$(( $(date +%s) + {timeout} ))
location_path=''
while [ "$(date +%s)" -le "$deadline" ]; do
  location_output=$(gdbus call --system --dest org.freedesktop.GeoClue2 --object-path "$client_path" --method org.freedesktop.DBus.Properties.Get org.freedesktop.GeoClue2.Client Location)
  location_path=$(printf '%s\n' "$location_output" | sed -n "s/.*objectpath '\([^']*\)'.*/\1/p")
  if [ -n "$location_path" ] && [ "$location_path" != "/" ]; then
    break
  fi
  sleep 1
done
if [ -z "$location_path" ] || [ "$location_path" = "/" ]; then
  echo "GeoClue did not return a location before timeout." >&2
  exit 3
fi
lat=$(gdbus call --system --dest org.freedesktop.GeoClue2 --object-path "$location_path" --method org.freedesktop.DBus.Properties.Get org.freedesktop.GeoClue2.Location Latitude)
lon=$(gdbus call --system --dest org.freedesktop.GeoClue2 --object-path "$location_path" --method org.freedesktop.DBus.Properties.Get org.freedesktop.GeoClue2.Location Longitude)
acc=$(gdbus call --system --dest org.freedesktop.GeoClue2 --object-path "$location_path" --method org.freedesktop.DBus.Properties.Get org.freedesktop.GeoClue2.Location Accuracy)
ts=$(date +%s%3N)
printf 'LAT=%s\nLON=%s\nACC=%s\nTIMESTAMP=%s\nPATH=%s\n' "$lat" "$lon" "$acc" "$ts" "$location_path"
"#,
        accuracyLevel = accuracyLevel,
        timeout = timeout,
    );
    let output = Command::new("sh")
        .arg("-lc")
        .arg(script)
        .output()
        .map_err(|error| HostError::new(format!("Failed to query Linux location: {error}")))?;
    if !output.status.success() {
        return Err(HostError::new(format!(
            "Failed to query Linux location with GeoClue2: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let latitude = parse_gdbus_double(&stdout, "LAT")?;
    let longitude = parse_gdbus_double(&stdout, "LON")?;
    let accuracy = parse_gdbus_double(&stdout, "ACC")? as f32;
    let timestamp = parse_key_i64(&stdout, "TIMESTAMP")?;
    Ok(LocationData {
        latitude,
        longitude,
        accuracy,
        provider: "geoclue2".to_string(),
        timestamp,
        rawData: stdout.to_string(),
        address: if includeAddress {
            "GeoClue2 did not provide a civic address through this host implementation.".to_string()
        } else {
            String::new()
        },
        city: String::new(),
        province: String::new(),
        country: String::new(),
    })
}

fn get_linux_device_info() -> HostResult<DeviceInfoData> {
    let machineId = read_trimmed_file("/etc/machine-id")?;
    let osRelease = fs::read_to_string("/etc/os-release")
        .map_err(|error| HostError::new(format!("Failed to read /etc/os-release: {error}")))?;
    let prettyName = os_release_value(&osRelease, "PRETTY_NAME")?;
    let kernel = command_stdout("uname", &["-r"], "read Linux kernel version")?;
    let model = command_stdout("uname", &["-m"], "read Linux machine architecture")?;
    let manufacturer = read_trimmed_file("/sys/devices/virtual/dmi/id/sys_vendor")?;
    let productName = read_trimmed_file("/sys/devices/virtual/dmi/id/product_name")?;
    let memInfo = fs::read_to_string("/proc/meminfo")
        .map_err(|error| HostError::new(format!("Failed to read /proc/meminfo: {error}")))?;
    let totalMemoryKb = meminfo_kb(&memInfo, "MemTotal")?;
    let availableMemoryKb = meminfo_kb(&memInfo, "MemAvailable")?;
    let storage = linux_storage_info("/")?;
    let cpuInfo = linux_cpu_info()?;
    let (batteryLevel, batteryCharging) = linux_battery_info()?;
    let networkType = linux_network_type()?;
    let mut additionalInfo = BTreeMap::new();
    additionalInfo.insert("Device name".to_string(), command_stdout("hostname", &[], "read Linux hostname")?);
    additionalInfo.insert("Product name".to_string(), productName.clone());
    additionalInfo.insert("Hardware name".to_string(), model.clone());
    additionalInfo.insert("Build fingerprint".to_string(), kernel.clone());
    additionalInfo.insert("Build time".to_string(), read_trimmed_file("/proc/version")?);
    Ok(DeviceInfoData {
        deviceId: machineId,
        model: productName,
        manufacturer,
        androidVersion: format!("{prettyName} {kernel}"),
        sdkVersion: 0,
        screenResolution: linux_screen_resolution()?,
        screenDensity: 1.0,
        totalMemory: format_size(totalMemoryKb.saturating_mul(1024)),
        availableMemory: format_size(availableMemoryKb.saturating_mul(1024)),
        totalStorage: format_size(storage.totalBytes),
        availableStorage: format_size(storage.availableBytes),
        batteryLevel,
        batteryCharging,
        cpuInfo,
        networkType,
        additionalInfo,
    })
}

fn read_trimmed_file(path: &str) -> HostResult<String> {
    let text = fs::read_to_string(path)
        .map_err(|error| HostError::new(format!("Failed to read {path}: {error}")))?;
    Ok(text.trim().to_string())
}

fn command_stdout(command: &str, args: &[&str], operation: &str) -> HostResult<String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .map_err(|error| HostError::new(format!("Failed to {operation}: {error}")))?;
    if !output.status.success() {
        return Err(HostError::new(format!(
            "Failed to {operation}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn os_release_value(content: &str, key: &str) -> HostResult<String> {
    let prefix = format!("{key}=");
    let raw = content
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .ok_or_else(|| HostError::new(format!("/etc/os-release is missing field: {key}")))?;
    Ok(raw.trim().trim_matches('"').to_string())
}

fn meminfo_kb(content: &str, key: &str) -> HostResult<u64> {
    let prefix = format!("{key}:");
    let line = content
        .lines()
        .find(|line| line.starts_with(&prefix))
        .ok_or_else(|| HostError::new(format!("/proc/meminfo is missing field: {key}")))?;
    let raw = line[prefix.len()..]
        .split_whitespace()
        .next()
        .ok_or_else(|| HostError::new(format!("/proc/meminfo field {key} has no value")))?;
    raw.parse::<u64>()
        .map_err(|error| HostError::new(format!("/proc/meminfo field {key} is not numeric: {error}")))
}

#[derive(Clone, Copy)]
struct LinuxStorageInfo {
    totalBytes: u64,
    availableBytes: u64,
}

fn linux_storage_info(path: &str) -> HostResult<LinuxStorageInfo> {
    let output = command_stdout("df", &["-B1", path], "read Linux storage information")?;
    let line = output
        .lines()
        .nth(1)
        .ok_or_else(|| HostError::new("df output is missing storage data row"))?;
    let parts = line.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 4 {
        return Err(HostError::new("df output storage data row is incomplete"));
    }
    let totalBytes = parts[1]
        .parse::<u64>()
        .map_err(|error| HostError::new(format!("df total bytes is not numeric: {error}")))?;
    let availableBytes = parts[3]
        .parse::<u64>()
        .map_err(|error| HostError::new(format!("df available bytes is not numeric: {error}")))?;
    Ok(LinuxStorageInfo {
        totalBytes,
        availableBytes,
    })
}

fn linux_cpu_info() -> HostResult<String> {
    let cpuInfo = fs::read_to_string("/proc/cpuinfo")
        .map_err(|error| HostError::new(format!("Failed to read /proc/cpuinfo: {error}")))?;
    let modelName = cpuInfo
        .lines()
        .find_map(|line| line.strip_prefix("model name"))
        .and_then(|value| value.split_once(':').map(|(_, name)| name.trim().to_string()))
        .ok_or_else(|| HostError::new("/proc/cpuinfo is missing model name"))?;
    Ok(modelName)
}

fn linux_battery_info() -> HostResult<(i32, bool)> {
    let powerSupplyPath = Path::new("/sys/class/power_supply");
    let entries = fs::read_dir(powerSupplyPath)
        .map_err(|error| HostError::new(format!("Failed to read {}: {error}", powerSupplyPath.display())))?;
    for entry in entries {
        let entry = entry
            .map_err(|error| HostError::new(format!("Failed to read power supply entry: {error}")))?;
        let path = entry.path();
        let typeText = fs::read_to_string(path.join("type"))
            .map_err(|error| HostError::new(format!("Failed to read power supply type: {error}")))?;
        if typeText.trim() == "Battery" {
            let capacityText = fs::read_to_string(path.join("capacity"))
                .map_err(|error| HostError::new(format!("Failed to read battery capacity: {error}")))?;
            let statusText = fs::read_to_string(path.join("status"))
                .map_err(|error| HostError::new(format!("Failed to read battery status: {error}")))?;
            let level = capacityText
                .trim()
                .parse::<i32>()
                .map_err(|error| HostError::new(format!("Battery capacity is not numeric: {error}")))?;
            let charging = matches!(statusText.trim(), "Charging" | "Full");
            return Ok((level, charging));
        }
    }
    Err(HostError::new("Linux battery information was not found in /sys/class/power_supply"))
}

fn linux_network_type() -> HostResult<String> {
    let route = command_stdout("ip", &["route", "show", "default"], "read Linux default route")?;
    let parts = route.split_whitespace().collect::<Vec<_>>();
    let devIndex = parts
        .iter()
        .position(|part| *part == "dev")
        .ok_or_else(|| HostError::new("Linux default route output is missing dev field"))?;
    let device = parts
        .get(devIndex + 1)
        .ok_or_else(|| HostError::new("Linux default route output is missing interface name"))?;
    Ok((*device).to_string())
}

fn linux_screen_resolution() -> HostResult<String> {
    let output = command_stdout("sh", &["-lc", "xrandr --current | sed -n 's/.* current \\([0-9][0-9]*\\) x \\([0-9][0-9]*\\).*/\\1x\\2/p' | head -n 1"], "read Linux screen resolution")?;
    if output.trim().is_empty() {
        return Err(HostError::new("xrandr did not return a current screen resolution"));
    }
    Ok(output)
}

fn format_size(size: u64) -> String {
    let kb = 1024.0;
    let mb = kb * 1024.0;
    let gb = mb * 1024.0;
    let tb = gb * 1024.0;
    let size = size as f64;
    if size < kb {
        return format!("{} B", size as u64);
    }
    if size < mb {
        return format!("{:.2} KB", size / kb);
    }
    if size < gb {
        return format!("{:.2} MB", size / mb);
    }
    if size < tb {
        return format!("{:.2} GB", size / gb);
    }
    format!("{:.2} TB", size / tb)
}

#[allow(non_snake_case)]
fn parseDesktopName(content: &str) -> Option<String> {
    content
        .lines()
        .find_map(|line| line.strip_prefix("Name="))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn parse_gdbus_double(text: &str, key: &str) -> HostResult<f64> {
    let pattern = format!(r"(?m)^{}=\(<([-+0-9.eE]+)>,\)", regex::escape(key));
    let regex = Regex::new(&pattern)
        .map_err(|error| HostError::new(format!("Failed to build GeoClue parser: {error}")))?;
    let raw = regex
        .captures(text)
        .and_then(|captures| captures.get(1))
        .map(|match_| match_.as_str())
        .ok_or_else(|| HostError::new(format!("GeoClue output is missing field: {key}")))?;
    raw.parse::<f64>()
        .map_err(|error| HostError::new(format!("GeoClue field {key} is not numeric: {error}")))
}

fn parse_key_i64(text: &str, key: &str) -> HostResult<i64> {
    let prefix = format!("{key}=");
    let raw = text
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .ok_or_else(|| HostError::new(format!("Output is missing field: {key}")))?;
    raw.trim()
        .parse::<i64>()
        .map_err(|error| HostError::new(format!("Field {key} is not an integer: {error}")))
}

fn unix_time_millis() -> HostResult<i64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| HostError::new(format!("System clock is before UNIX_EPOCH: {error}")))?;
    i64::try_from(duration.as_millis())
        .map_err(|_| HostError::new("Current time is out of range"))
}

fn local_applications_dir() -> HostResult<PathBuf> {
    let home = env::var_os("HOME").ok_or_else(|| {
        HostError::new("HOME is required to resolve the local applications directory")
    })?;
    Ok(PathBuf::from(home).join(".local/share/applications"))
}
