mod battery;
#[cfg(target_os = "macos")]
mod packages;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeResultData, DeviceInfoData,
    HostError, HostResult, LocationData, NotificationData, OCRLanguage, OCRQuality,
    SystemNotificationRequest, SystemOperationHost, SystemSettingData,
};
use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub struct AppleSystemOperationHost;

impl AppleSystemOperationHost {
    /// Retains current UIKit battery readings delivered by the iOS owner event channel.
    #[cfg(target_os = "ios")]
    pub fn observeBatteryEventJson(encoded: &str) {
        battery::observe_ios_event(encoded);
    }

    pub fn new() -> Self {
        Self
    }
}

impl SystemOperationHost for AppleSystemOperationHost {
    /// Reads the installed macOS distribution without exposing bundle paths to clients.
    fn applicationUpdateTarget(
        &self,
    ) -> HostResult<Option<operit_host_api::ApplicationUpdateTarget>> {
        #[cfg(target_os = "macos")]
        {
            let executable = std::env::current_exe()?;
            let contents = executable
                .parent()
                .and_then(Path::parent)
                .ok_or_else(|| HostError::new("Application bundle metadata is unavailable"))?;
            let metadata = Command::new("/usr/bin/plutil")
                .args(["-extract", "OperitDistributionType", "raw", "-o", "-"])
                .arg(contents.join("Info.plist"))
                .output()?;
            if !metadata.status.success() {
                return Err(HostError::new(
                    "Cannot read this macOS application's distribution metadata",
                ));
            }
            let buildType = String::from_utf8(metadata.stdout)
                .map_err(|error| HostError::new(error.to_string()))?
                .trim()
                .to_owned();
            return Ok(Some(operit_host_api::ApplicationUpdateTarget {
                installTiming: operit_host_api::AppInstallTiming::Immediate,
                arch: "universal".into(),
                package: operit_host_api::ApplicationUpdatePackage::MacosPkg { buildType },
            }));
        }
        #[cfg(not(target_os = "macos"))]
        Ok(None)
    }
    fn getSystemLanguageCode(&self) -> HostResult<String> {
        use core_foundation::base::TCFType;
        use core_foundation::string::CFString;
        extern "C" {
            fn CFLocaleCopyCurrent() -> core_foundation::base::CFTypeRef;
            fn CFLocaleGetIdentifier(locale: core_foundation::base::CFTypeRef)
                -> core_foundation::string::CFStringRef;
        }
        // Query the current locale on both Apple platforms without launching a
        // subprocess for every plugin entry. CopyCurrent retains the locale.
        unsafe {
            let locale = CFLocaleCopyCurrent();
            if locale.is_null() {
                return Err(HostError::new("Current Apple locale is unavailable"));
            }
            let value = CFString::wrap_under_get_rule(CFLocaleGetIdentifier(locale))
                .to_string().replace('_', "-");
            core_foundation::base::CFRelease(locale.cast());
            Ok(value)
        }
    }

    fn sendNotification(&self, request: &SystemNotificationRequest) -> HostResult<()> {
        #[cfg(target_os = "macos")]
        {
            let script = format!(
                "display notification {} with title {}",
                apple_script_string(&request.message),
                apple_script_string(if request.title.trim().is_empty() {
                    "Notification"
                } else {
                    &request.title
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
            let _ = request;
            Err(HostError::new(
                "iOS notifications must be requested through the app notification layer",
            ))
        }
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            let _ = request;
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

    /// Installs a VFS file reference using the requested platform execution timing.
    fn installApp(&self, source: &dyn operit_host_api::FileSource, timing: operit_host_api::AppInstallTiming) -> HostResult<AppOperationData> {
        timing.requireImmediate()?;
        let path = source.nativePath()?;
        let path = non_blank(&path, "path")?;
        #[cfg(target_os = "macos")]
        { packages::install(&path) }
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
        #[cfg(target_os = "macos")]
        { packages::uninstall(packageName) }
        #[cfg(not(target_os = "macos"))]
        { Err(HostError::new("iOS public APIs do not allow uninstalling arbitrary applications; use system application settings")) }
    }

    fn listInstalledApps(&self, includeSystemApps: bool) -> HostResult<AppListData> {
        #[cfg(target_os = "macos")]
        { packages::list(includeSystemApps) }
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
        { packages::start(&packageName) }
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
        { packages::stop(&packageName) }
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
            let _ = (packageName, includeSystemApps);
            Err(HostError::new("macOS does not expose historical foreground usage to this host. Screen Time requires a separately authorized integration; process uptime is not foreground usage."))
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

    /// Writes image bytes through the selected VFS capability and removes native staging.
    fn captureScreenshotTo(&self, target: &dyn operit_host_api::FileTarget) -> HostResult<()> {
        let path = self.captureScreenshot()?;
        let bytes = std::fs::read(&path).map_err(|error| operit_host_api::HostError::new(error.to_string()))?;
        let written = target.writeBytes(&bytes);
        std::fs::remove_file(&path).map_err(|error| operit_host_api::HostError::new(error.to_string()))?;
        written
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
        imagePath: &dyn operit_host_api::FileSource,
        language: OCRLanguage,
        _quality: OCRQuality,
    ) -> HostResult<String> {
        #[cfg(target_os = "macos")]
        {
            let imagePath = non_blank(&imagePath.nativePath()?, "image_path")?;
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
    let hostName = gethostname::gethostname()
        .into_string()
        .map_err(|error| HostError::new(format!("Apple host name is not valid UTF-8: {error:?}")))?;
    let mut additionalInfo = BTreeMap::new();
    additionalInfo.insert("Platform".to_string(), apple_platform_name().to_string());
    additionalInfo.insert("Host name".to_string(), hostName.clone());
    let (batteryLevel, batteryCharging) = battery::current_battery();
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
        batteryLevel,
        batteryCharging,
        cpuInfo: model,
        networkType: String::new(),
        additionalInfo,
    })
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
