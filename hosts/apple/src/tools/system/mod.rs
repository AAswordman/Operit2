use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeEntry, AppUsageTimeResultData, DeviceInfoData,
    HostError, HostResult, LocationData, NotificationData, OCRLanguage, OCRQuality,
    SystemOperationHost, SystemSettingData,
};
use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub struct AppleSystemOperationHost;

impl AppleSystemOperationHost {
    pub fn new() -> Self {
        Self
    }
}

impl SystemOperationHost for AppleSystemOperationHost {
    fn getSystemLanguageCode(&self) -> HostResult<String> {
        let output = run_command_output("defaults", &["read", "-g", "AppleLocale"])?;
        let value = output.trim().replace('_', "-");
        if value.is_empty() {
            return Err(HostError::new("AppleLocale is empty"));
        }
        Ok(value)
    }

    fn toast(&self, message: &str) -> HostResult<()> {
        if message.trim().is_empty() {
            return Err(HostError::new("message parameter is required"));
        }
        self.sendNotification("Operit", message)
    }

    fn sendNotification(&self, title: &str, message: &str) -> HostResult<()> {
        #[cfg(target_os = "macos")]
        {
            let script = format!(
                "display notification {} with title {}",
                apple_script_string(message),
                apple_script_string(if title.trim().is_empty() {
                    "Notification"
                } else {
                    title
                })
            );
            let status = Command::new("osascript")
                .arg("-e")
                .arg(script)
                .status()
                .map_err(|error| {
                    HostError::new(format!("Failed to send macOS notification: {error}"))
                })?;
            if !status.success() {
                return Err(HostError::new(format!(
                    "macOS notification command exited with {status}"
                )));
            }
            Ok(())
        }
        #[cfg(target_os = "ios")]
        {
            let _ = title;
            let _ = message;
            Err(HostError::new(
                "iOS notifications must be requested through the app notification layer",
            ))
        }
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            let _ = title;
            let _ = message;
            Err(HostError::new(
                "Apple notification host is available only on iOS or macOS",
            ))
        }
    }

    fn modifySystemSetting(
        &self,
        namespace: &str,
        setting: &str,
        value: &str,
    ) -> HostResult<SystemSettingData> {
        #[cfg(target_os = "macos")]
        {
            let domain = non_blank(namespace, "namespace")?;
            let key = non_blank(setting, "setting")?;
            let status = Command::new("defaults")
                .args(["write", &domain, &key, value])
                .status()
                .map_err(|error| {
                    HostError::new(format!("Failed to modify macOS setting: {error}"))
                })?;
            if !status.success() {
                return Err(HostError::new(format!(
                    "macOS defaults write exited with {status}"
                )));
            }
            self.getSystemSetting(&domain, &key)
        }
        #[cfg(target_os = "ios")]
        {
            let _ = namespace;
            let _ = setting;
            let _ = value;
            Err(HostError::new(
                "iOS system settings are not writable by this host",
            ))
        }
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            let _ = namespace;
            let _ = setting;
            let _ = value;
            Err(HostError::new(
                "Apple system setting host is available only on iOS or macOS",
            ))
        }
    }

    fn getSystemSetting(&self, namespace: &str, setting: &str) -> HostResult<SystemSettingData> {
        #[cfg(target_os = "macos")]
        {
            let domain = non_blank(namespace, "namespace")?;
            let key = non_blank(setting, "setting")?;
            let value = run_command_output("defaults", &["read", &domain, &key])?
                .trim()
                .to_string();
            Ok(SystemSettingData {
                namespace: domain,
                setting: key,
                value,
            })
        }
        #[cfg(target_os = "ios")]
        {
            let _ = namespace;
            let _ = setting;
            Err(HostError::new(
                "iOS system settings are not readable by this host",
            ))
        }
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            let _ = namespace;
            let _ = setting;
            Err(HostError::new(
                "Apple system setting host is available only on iOS or macOS",
            ))
        }
    }

    fn installApp(&self, path: &str) -> HostResult<AppOperationData> {
        let path = non_blank(path, "path")?;
        #[cfg(target_os = "macos")]
        {
            let status = Command::new("open").arg(&path).status().map_err(|error| {
                HostError::new(format!("Failed to open macOS installer: {error}"))
            })?;
            if !status.success() {
                return Err(HostError::new(format!(
                    "macOS installer open exited with {status}"
                )));
            }
            Ok(AppOperationData {
                operationType: "install".to_string(),
                packageName: path,
                success: true,
                details: "Installer opened with macOS open".to_string(),
            })
        }
        #[cfg(target_os = "ios")]
        {
            Err(HostError::new(format!(
                "iOS app installation is not available for this host: {path}"
            )))
        }
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            Err(HostError::new(format!(
                "Apple app installation host is available only on iOS or macOS: {path}"
            )))
        }
    }

    fn uninstallApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        Err(HostError::new(format!(
            "Apple app uninstall requires user-controlled system UI: {}",
            non_blank(packageName, "package_name")?
        )))
    }

    fn listInstalledApps(&self, includeSystemApps: bool) -> HostResult<AppListData> {
        #[cfg(target_os = "macos")]
        {
            let mut packages = Vec::new();
            collect_app_names(Path::new("/Applications"), &mut packages)?;
            if includeSystemApps {
                collect_app_names(Path::new("/System/Applications"), &mut packages)?;
            }
            packages.sort();
            packages.dedup();
            Ok(AppListData {
                includesSystemApps: includeSystemApps,
                packages,
            })
        }
        #[cfg(target_os = "ios")]
        {
            let _ = includeSystemApps;
            Err(HostError::new(
                "iOS installed application listing is not exposed to this host",
            ))
        }
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            let _ = includeSystemApps;
            Err(HostError::new(
                "Apple installed application host is available only on iOS or macOS",
            ))
        }
    }

    fn startApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        let packageName = non_blank(packageName, "package_name")?;
        #[cfg(target_os = "macos")]
        {
            let status = Command::new("open")
                .arg("-a")
                .arg(&packageName)
                .status()
                .map_err(|error| HostError::new(format!("Failed to start macOS app: {error}")))?;
            if !status.success() {
                return Err(HostError::new(format!(
                    "macOS open -a exited with {status}"
                )));
            }
            Ok(AppOperationData {
                operationType: "start".to_string(),
                packageName,
                success: true,
                details: "Start request sent with macOS open".to_string(),
            })
        }
        #[cfg(target_os = "ios")]
        {
            Err(HostError::new(format!(
                "iOS app start is not available for this host: {packageName}"
            )))
        }
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            Err(HostError::new(format!(
                "Apple app start host is available only on iOS or macOS: {packageName}"
            )))
        }
    }

    fn stopApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        let packageName = non_blank(packageName, "package_name")?;
        #[cfg(target_os = "macos")]
        {
            let status = Command::new("osascript")
                .arg("-e")
                .arg(format!("quit app {}", apple_script_string(&packageName)))
                .status()
                .map_err(|error| HostError::new(format!("Failed to stop macOS app: {error}")))?;
            if !status.success() {
                return Err(HostError::new(format!(
                    "macOS quit app command exited with {status}"
                )));
            }
            Ok(AppOperationData {
                operationType: "stop".to_string(),
                packageName,
                success: true,
                details: "Stop request sent with AppleScript".to_string(),
            })
        }
        #[cfg(target_os = "ios")]
        {
            Err(HostError::new(format!(
                "iOS app stop is not available for this host: {packageName}"
            )))
        }
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            Err(HostError::new(format!(
                "Apple app stop host is available only on iOS or macOS: {packageName}"
            )))
        }
    }

    fn getNotifications(&self, _limit: i32, _includeOngoing: bool) -> HostResult<NotificationData> {
        Err(HostError::new(
            "Apple notification history is not exposed to third-party hosts",
        ))
    }

    fn getAppUsageTime(
        &self,
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
        #[cfg(target_os = "macos")]
        {
            let now = unix_time_millis()?;
            let startTime = now - i64::from(sinceHours) * 60 * 60 * 1000;
            let output = run_command_output("ps", &["-axo", "comm=,etimes="])?;
            let mut entries = Vec::new();
            for line in output.lines() {
                let mut parts = line.split_whitespace().collect::<Vec<_>>();
                if parts.len() < 2 {
                    continue;
                }
                let elapsedText = match parts.pop() {
                    Some(value) => value,
                    None => continue,
                };
                let elapsed = match elapsedText.parse::<i64>() {
                    Ok(value) => value,
                    Err(_) => continue,
                };
                let command = parts.join(" ");
                let name = match Path::new(&command)
                    .file_name()
                    .and_then(|value| value.to_str())
                {
                    Some(value) => value.to_string(),
                    None => command.clone(),
                };
                if !packageName.trim().is_empty() && name != packageName {
                    continue;
                }
                let isSystemApp = command.starts_with("/System/");
                if !includeSystemApps && isSystemApp {
                    continue;
                }
                let totalForegroundTimeMs = elapsed.saturating_mul(1000);
                entries.push(AppUsageTimeEntry {
                    packageName: name.clone(),
                    appName: name,
                    totalForegroundTimeMs,
                    lastTimeUsed: now,
                    isSystemApp,
                });
            }
            entries.sort_by(|left, right| {
                right.totalForegroundTimeMs.cmp(&left.totalForegroundTimeMs)
            });
            entries.truncate(limit as usize);
            Ok(AppUsageTimeResultData {
                startTime,
                endTime: now,
                sinceHours,
                requestedPackageName: if packageName.trim().is_empty() {
                    None
                } else {
                    Some(packageName.to_string())
                },
                includesSystemApps: includeSystemApps,
                totalEntries: entries.len() as i32,
                entries,
            })
        }
        #[cfg(target_os = "ios")]
        {
            let _ = packageName;
            let _ = includeSystemApps;
            Err(HostError::new(
                "iOS app usage time is not exposed to this host",
            ))
        }
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            let _ = packageName;
            let _ = includeSystemApps;
            Err(HostError::new(
                "Apple app usage time host is available only on iOS or macOS",
            ))
        }
    }

    fn getDeviceLocation(
        &self,
        _timeout: i32,
        _highAccuracy: bool,
        _includeAddress: bool,
    ) -> HostResult<LocationData> {
        Err(HostError::new(
            "Apple location must be requested by the Flutter owner UI",
        ))
    }

    fn getDeviceInfo(&self) -> HostResult<DeviceInfoData> {
        get_apple_device_info()
    }

    fn captureScreenshot(&self) -> HostResult<String> {
        #[cfg(target_os = "macos")]
        {
            let outputPath = temp_capture_path("macos_screen")?;
            let status = Command::new("screencapture")
                .arg("-x")
                .arg(&outputPath)
                .status()
                .map_err(|error| {
                    HostError::new(format!("Failed to capture macOS screenshot: {error}"))
                })?;
            if !status.success() {
                return Err(HostError::new(format!(
                    "macOS screencapture exited with {status}"
                )));
            }
            validate_file_path(&outputPath, "macOS screenshot")
        }
        #[cfg(target_os = "ios")]
        {
            Err(HostError::new(
                "iOS screenshot capture must be requested by the Flutter owner UI",
            ))
        }
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            Err(HostError::new(
                "Apple screenshot host is available only on iOS or macOS",
            ))
        }
    }

    fn recognizeText(
        &self,
        imagePath: &str,
        language: OCRLanguage,
        _quality: OCRQuality,
    ) -> HostResult<String> {
        #[cfg(target_os = "macos")]
        {
            let imagePath = non_blank(imagePath, "image_path")?;
            let languageHint = match language {
                OCRLanguage::Latin => "en-US",
                OCRLanguage::Chinese => "zh-Hans",
                OCRLanguage::Japanese => "ja-JP",
                OCRLanguage::Korean => "ko-KR",
            };
            let script = format!(
                r#"
import Vision
import Foundation

let imageURL = URL(fileURLWithPath: {image_path})
let request = VNRecognizeTextRequest()
request.recognitionLanguages = [{language}]
request.recognitionLevel = .accurate
let handler = VNImageRequestHandler(url: imageURL, options: [:])
try handler.perform([request])
let text = request.results?.compactMap {{ $0.topCandidates(1).first?.string }}.joined(separator: "\n") ?? ""
print(text)
"#,
                image_path = swift_string_literal(&imagePath),
                language = swift_string_literal(languageHint)
            );
            let scriptPath = temp_text_path("macos_ocr", "swift")?;
            fs::write(&scriptPath, script)?;
            let output = Command::new("swift")
                .arg(&scriptPath)
                .output()
                .map_err(|error| {
                    HostError::new(format!("Failed to run macOS OCR script: {error}"))
                })?;
            let _ = fs::remove_file(&scriptPath);
            if !output.status.success() {
                return Err(HostError::new(format!(
                    "macOS OCR script failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                )));
            }
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        #[cfg(target_os = "ios")]
        {
            let _ = imagePath;
            let _ = language;
            Err(HostError::new(
                "iOS OCR must be requested by the Flutter owner UI",
            ))
        }
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            let _ = imagePath;
            let _ = language;
            Err(HostError::new(
                "Apple OCR host is available only on iOS or macOS",
            ))
        }
    }
}

fn get_apple_device_info() -> HostResult<DeviceInfoData> {
    #[cfg(target_os = "macos")]
    let model = run_command_output("uname", &["-m"])?.trim().to_string();
    #[cfg(target_os = "macos")]
    let osVersion = run_command_output("sw_vers", &["-productVersion"])?
        .trim()
        .to_string();
    #[cfg(target_os = "ios")]
    let model = "ios".to_string();
    #[cfg(target_os = "ios")]
    let osVersion = String::new();
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    let model = "apple".to_string();
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    let osVersion = String::new();
    let hostName = env::var("HOSTNAME").map_err(|error| {
        HostError::new(format!(
            "HOSTNAME is required for Apple device info: {error}"
        ))
    })?;
    let mut additionalInfo = BTreeMap::new();
    additionalInfo.insert("Platform".to_string(), apple_platform_name().to_string());
    additionalInfo.insert("Host name".to_string(), hostName.clone());
    Ok(DeviceInfoData {
        deviceId: hostName,
        model: model.clone(),
        manufacturer: "Apple".to_string(),
        androidVersion: osVersion,
        sdkVersion: 0,
        screenResolution: String::new(),
        screenDensity: 1.0,
        totalMemory: String::new(),
        availableMemory: String::new(),
        totalStorage: String::new(),
        availableStorage: String::new(),
        batteryLevel: 0,
        batteryCharging: false,
        cpuInfo: model,
        networkType: String::new(),
        additionalInfo,
    })
}

fn collect_app_names(dir: &Path, packages: &mut Vec<String>) -> HostResult<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("app") {
            if let Some(name) = path.file_stem().and_then(|value| value.to_str()) {
                packages.push(name.to_string());
            }
        }
    }
    Ok(())
}

fn run_command_output(program: &str, args: &[&str]) -> HostResult<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| HostError::new(format!("Failed to run {program}: {error}")))?;
    if !output.status.success() {
        return Err(HostError::new(format!(
            "{program} exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn non_blank(value: &str, name: &str) -> HostResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(HostError::new(format!("{name} parameter is required")));
    }
    Ok(trimmed.to_string())
}

fn apple_platform_name() -> &'static str {
    #[cfg(target_os = "ios")]
    {
        "ios"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        "apple"
    }
}

fn apple_script_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn swift_string_literal(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn temp_capture_path(prefix: &str) -> HostResult<PathBuf> {
    let tempDir = env::temp_dir().join("operit-runtime").join("temp");
    fs::create_dir_all(&tempDir).map_err(|error| {
        HostError::new(format!(
            "Failed to create temporary directory {}: {error}",
            tempDir.display()
        ))
    })?;
    Ok(tempDir.join(format!("{prefix}_{}.png", Uuid::new_v4())))
}

fn temp_text_path(prefix: &str, extension: &str) -> HostResult<PathBuf> {
    let tempDir = env::temp_dir().join("operit-runtime").join("temp");
    fs::create_dir_all(&tempDir).map_err(|error| {
        HostError::new(format!(
            "Failed to create temporary directory {}: {error}",
            tempDir.display()
        ))
    })?;
    Ok(tempDir.join(format!("{prefix}_{}.{}", Uuid::new_v4(), extension)))
}

fn validate_file_path(path: &Path, operation: &str) -> HostResult<String> {
    let metadata = fs::metadata(path).map_err(|error| {
        HostError::new(format!(
            "Failed to verify {operation} output {}: {error}",
            path.display()
        ))
    })?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(HostError::new(format!(
            "{operation} did not create a valid file: {}",
            path.display()
        )));
    }
    Ok(path.to_string_lossy().into_owned())
}

fn unix_time_millis() -> HostResult<i64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| HostError::new(error.to_string()))?
        .as_millis() as i64)
}
