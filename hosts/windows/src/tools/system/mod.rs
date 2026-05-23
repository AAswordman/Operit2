use std::collections::BTreeMap;
use std::process::Command;

use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeEntry, AppUsageTimeResultData, DeviceInfoData,
    HostError, HostResult, LocationData, NotificationData, NotificationEntry, SystemOperationHost,
    SystemSettingData,
};
use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub struct WindowsSystemOperationHost;

impl WindowsSystemOperationHost {
    pub fn new() -> Self {
        Self
    }
}

impl SystemOperationHost for WindowsSystemOperationHost {
    fn toast(&self, message: &str) -> HostResult<()> {
        if message.trim().is_empty() {
            return Err(HostError::new("Must provide message parameter"));
        }
        let status = Command::new("msg")
            .arg("*")
            .arg("/TIME:5")
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
        let text = if title.trim().is_empty() {
            message.to_string()
        } else {
            format!("{title}: {message}")
        };
        self.toast(&text)
    }

    fn modifySystemSetting(
        &self,
        namespace: &str,
        setting: &str,
        value: &str,
    ) -> HostResult<SystemSettingData> {
        modify_windows_system_setting(namespace, setting, value)
    }

    fn getSystemSetting(&self, namespace: &str, setting: &str) -> HostResult<SystemSettingData> {
        get_windows_system_setting(namespace, setting)
    }

    fn installApp(&self, path: &str) -> HostResult<AppOperationData> {
        request_windows_install_app(path)
    }

    fn uninstallApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        request_windows_uninstall_app(packageName)
    }

    fn listInstalledApps(&self, includeSystemApps: bool) -> HostResult<AppListData> {
        let output = Command::new("powershell.exe")
            .arg("-NoProfile")
            .arg("-Command")
            .arg("Get-ItemProperty 'HKLM:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*','HKLM:\\Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*','HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*' -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName } | Sort-Object DisplayName -Unique | ForEach-Object { $_.DisplayName }")
            .output()
            .map_err(|error| HostError::new(format!("Failed to list installed apps: {error}")))?;
        if !output.status.success() {
            return Err(HostError::new(format!(
                "Failed to list installed apps: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        let packages = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
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
        let status = Command::new("taskkill")
            .arg("/IM")
            .arg(packageName)
            .arg("/F")
            .status()
            .map_err(|error| HostError::new(format!("Error stopping app: {error}")))?;
        if !status.success() {
            return Err(HostError::new(format!(
                "Error stopping app: taskkill exited with {status}"
            )));
        }
        Ok(AppOperationData {
            operationType: "stop".to_string(),
            packageName: packageName.to_string(),
            success: true,
            details: "Stop request sent".to_string(),
        })
    }

    fn getNotifications(&self, limit: i32, _includeOngoing: bool) -> HostResult<NotificationData> {
        get_windows_notifications(limit)
    }

    fn getAppUsageTime(
        &self,
        packageName: &str,
        sinceHours: i32,
        limit: i32,
        includeSystemApps: bool,
    ) -> HostResult<AppUsageTimeResultData> {
        get_windows_app_usage_time(packageName, sinceHours, limit, includeSystemApps)
    }

    fn getDeviceLocation(
        &self,
        timeout: i32,
        highAccuracy: bool,
        includeAddress: bool,
    ) -> HostResult<LocationData> {
        get_windows_device_location(timeout, highAccuracy, includeAddress)
    }

    fn getDeviceInfo(&self) -> HostResult<DeviceInfoData> {
        get_windows_device_info()
    }
}

fn modify_windows_system_setting(
    namespace: &str,
    setting: &str,
    value: &str,
) -> HostResult<SystemSettingData> {
    let target = windows_setting_target(namespace, setting)?;
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$path = {path}
$name = {name}
$value = {value}
if (-not (Test-Path -LiteralPath $path)) {{
    New-Item -Path $path -Force | Out-Null
}}
New-ItemProperty -LiteralPath $path -Name $name -Value $value -PropertyType String -Force | Out-Null
[pscustomobject]@{{
    namespace = {namespace}
    setting = {setting}
    value = [string](Get-ItemPropertyValue -LiteralPath $path -Name $name)
}} | ConvertTo-Json -Compress
"#,
        path = ps_string_literal(&target.registryPath),
        name = ps_string_literal(&target.valueName),
        value = ps_string_literal(value),
        namespace = ps_string_literal(namespace),
        setting = ps_string_literal(setting),
    );
    parse_windows_system_setting(run_powershell_json(&script, "modify Windows system setting")?)
}

fn get_windows_system_setting(namespace: &str, setting: &str) -> HostResult<SystemSettingData> {
    let target = windows_setting_target(namespace, setting)?;
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$path = {path}
$name = {name}
$value = [string](Get-ItemPropertyValue -LiteralPath $path -Name $name)
[pscustomobject]@{{
    namespace = {namespace}
    setting = {setting}
    value = $value
}} | ConvertTo-Json -Compress
"#,
        path = ps_string_literal(&target.registryPath),
        name = ps_string_literal(&target.valueName),
        namespace = ps_string_literal(namespace),
        setting = ps_string_literal(setting),
    );
    parse_windows_system_setting(run_powershell_json(&script, "get Windows system setting")?)
}

#[derive(Debug)]
struct WindowsSettingTarget {
    registryPath: String,
    valueName: String,
}

#[allow(non_snake_case)]
fn windows_setting_target(namespace: &str, setting: &str) -> HostResult<WindowsSettingTarget> {
    let valueName = setting.trim();
    if valueName.is_empty() {
        return Err(HostError::new("setting is required"));
    }
    let base = match namespace {
        "system" => "HKCU:\\Software\\Operit2\\System",
        "secure" => "HKCU:\\Software\\Operit2\\Secure",
        "global" => "HKCU:\\Software\\Operit2\\Global",
        _ => {
            return Err(HostError::new(format!(
                "Unsupported Windows setting namespace: {namespace}"
            )))
        }
    };
    Ok(WindowsSettingTarget {
        registryPath: base.to_string(),
        valueName: valueName.to_string(),
    })
}

fn parse_windows_system_setting(value: Value) -> HostResult<SystemSettingData> {
    Ok(SystemSettingData {
        namespace: json_string(&value, "namespace")?,
        setting: json_string(&value, "setting")?,
        value: json_string(&value, "value")?,
    })
}

fn request_windows_install_app(path: &str) -> HostResult<AppOperationData> {
    if path.trim().is_empty() {
        return Err(HostError::new("path is required"));
    }
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$path = {path}
if (-not (Test-Path -LiteralPath $path)) {{
    throw "Installer path does not exist: $path"
}}
$extension = [System.IO.Path]::GetExtension($path).ToLowerInvariant()
if ($extension -eq ".msix" -or $extension -eq ".appx" -or $extension -eq ".msixbundle" -or $extension -eq ".appxbundle") {{
    Add-AppxPackage -Path $path
}} elseif ($extension -eq ".msi") {{
    $process = Start-Process -FilePath "msiexec.exe" -ArgumentList @("/i", $path) -Wait -PassThru
    if ($process.ExitCode -ne 0) {{ throw "msiexec exited with $($process.ExitCode)" }}
}} else {{
    $process = Start-Process -FilePath $path -Wait -PassThru
    if ($null -ne $process.ExitCode -and $process.ExitCode -ne 0) {{ throw "installer exited with $($process.ExitCode)" }}
}}
[pscustomobject]@{{
    operationType = "install"
    packageName = $path
    success = $true
    details = "Install request completed"
}} | ConvertTo-Json -Compress
"#,
        path = ps_string_literal(path),
    );
    parse_app_operation_data(run_powershell_json(&script, "install Windows app")?)
}

fn request_windows_uninstall_app(packageName: &str) -> HostResult<AppOperationData> {
    if packageName.trim().is_empty() {
        return Err(HostError::new("package_name is required"));
    }
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$packageName = {packageName}
if ($packageName -match "^[0-9a-fA-F]{{8}}-[0-9a-fA-F]{{4}}-[0-9a-fA-F]{{4}}-[0-9a-fA-F]{{4}}-[0-9a-fA-F]{{12}}$") {{
    $process = Start-Process -FilePath "msiexec.exe" -ArgumentList @("/x", "{{$packageName}}") -Wait -PassThru
    if ($process.ExitCode -ne 0) {{ throw "msiexec uninstall exited with $($process.ExitCode)" }}
}} else {{
    $appx = Get-AppxPackage -Name $packageName -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -eq $appx) {{
        $appx = Get-AppxPackage | Where-Object {{ $_.Name -eq $packageName -or $_.PackageFullName -eq $packageName }} | Select-Object -First 1
    }}
    if ($null -eq $appx) {{
        throw "No Appx/MSIX package matched package_name: $packageName. For MSI uninstall, pass the product code GUID."
    }}
    Remove-AppxPackage -Package $appx.PackageFullName
}}
[pscustomobject]@{{
    operationType = "uninstall"
    packageName = $packageName
    success = $true
    details = "Uninstall request completed"
}} | ConvertTo-Json -Compress
"#,
        packageName = ps_string_literal(packageName),
    );
    parse_app_operation_data(run_powershell_json(&script, "uninstall Windows app")?)
}

fn get_windows_notifications(limit: i32) -> HostResult<NotificationData> {
    if limit <= 0 {
        return Err(HostError::new("limit must be greater than 0"));
    }

    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Runtime.WindowsRuntime
[Windows.UI.Notifications.Management.UserNotificationListener, Windows.UI.Notifications, ContentType=WindowsRuntime] > $null
[Windows.UI.Notifications.Management.UserNotificationListenerAccessStatus, Windows.UI.Notifications, ContentType=WindowsRuntime] > $null
[Windows.UI.Notifications.NotificationKinds, Windows.UI.Notifications, ContentType=WindowsRuntime] > $null
[Windows.UI.Notifications.UserNotification, Windows.UI.Notifications, ContentType=WindowsRuntime] > $null
[Windows.UI.Notifications.KnownNotificationBindings, Windows.UI.Notifications, ContentType=WindowsRuntime] > $null

function Await-WinRtOperation {{
    param(
        [Parameter(Mandatory=$true)] $Operation,
        [Parameter(Mandatory=$true)] [Type] $ResultType,
        [Parameter(Mandatory=$true)] [int] $TimeoutMs,
        [Parameter(Mandatory=$true)] [string] $OperationName
    )
    $asTask = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{
        $_.Name -eq 'AsTask' -and $_.IsGenericMethod -and $_.GetParameters().Count -eq 1
    }} | Select-Object -First 1)
    if ($null -eq $asTask) {{
        throw "Windows Runtime AsTask bridge is unavailable."
    }}
    $task = $asTask.MakeGenericMethod($ResultType).Invoke($null, @($Operation))
    if (-not $task.Wait($TimeoutMs)) {{
        throw "$OperationName timed out."
    }}
    return $task.Result
}}

function Read-AppName {{
    param($Notification)
    try {{
        $name = [string]$Notification.AppInfo.DisplayInfo.DisplayName
        if ($name -and $name.Trim().Length -gt 0) {{ return $name }}
    }} catch {{}}
    try {{
        $id = [string]$Notification.AppInfo.Id
        if ($id -and $id.Trim().Length -gt 0) {{ return $id }}
    }} catch {{}}
    return "Unknown"
}}

function Read-NotificationText {{
    param($Notification)
    $parts = New-Object System.Collections.Generic.List[string]
    try {{
        $binding = $Notification.Notification.Visual.GetBinding([Windows.UI.Notifications.KnownNotificationBindings]::ToastGeneric)
        if ($null -ne $binding) {{
            foreach ($textElement in $binding.GetTextElements()) {{
                $text = [string]$textElement.Text
                if ($text -and $text.Trim().Length -gt 0) {{
                    $parts.Add($text.Trim()) > $null
                }}
            }}
        }}
    }} catch {{}}
    try {{
        foreach ($binding in $Notification.Notification.Visual.Bindings) {{
            foreach ($textElement in $binding.GetTextElements()) {{
                $text = [string]$textElement.Text
                if ($text -and $text.Trim().Length -gt 0 -and -not $parts.Contains($text.Trim())) {{
                    $parts.Add($text.Trim()) > $null
                }}
            }}
        }}
    }} catch {{}}
    return [string]::Join("`n", $parts)
}}

$listener = [Windows.UI.Notifications.Management.UserNotificationListener]::Current
$access = Await-WinRtOperation `
    -Operation $listener.RequestAccessAsync() `
    -ResultType ([Windows.UI.Notifications.Management.UserNotificationListenerAccessStatus]) `
    -TimeoutMs 30000 `
    -OperationName "Windows notification listener access request"
if ([string]$access -ne "Allowed") {{
    throw "Windows notification listener access is $access."
}}

$listType = [System.Collections.Generic.IReadOnlyList[Windows.UI.Notifications.UserNotification]]
$notifications = Await-WinRtOperation `
    -Operation $listener.GetNotificationsAsync([Windows.UI.Notifications.NotificationKinds]::Toast) `
    -ResultType $listType `
    -TimeoutMs 30000 `
    -OperationName "Windows notification list request"

$items = @()
foreach ($notification in ($notifications | Sort-Object -Property CreationTime -Descending | Select-Object -First {limit})) {{
    $items += [pscustomobject]@{{
        packageName = Read-AppName $notification
        text = Read-NotificationText $notification
        timestamp = [long]$notification.CreationTime.ToUnixTimeMilliseconds()
    }}
}}

[pscustomobject]@{{
    timestamp = [long]([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds())
    notifications = @($items)
}} | ConvertTo-Json -Depth 8 -Compress
"#,
        limit = limit,
    );

    let output = Command::new("powershell.exe")
        .arg("-NoProfile")
        .arg("-Command")
        .arg(script)
        .output()
        .map_err(|error| HostError::new(format!("Failed to query Windows notifications: {error}")))?;

    if !output.status.success() {
        let errorText = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(HostError::new(format!(
            "Failed to query Windows notifications: {errorText}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let value: Value = serde_json::from_str(&stdout).map_err(|error| {
        HostError::new(format!("Failed to parse Windows notification data: {error}"))
    })?;
    let timestamp = json_i64(&value, "timestamp")?;
    let notificationsValue = value
        .get("notifications")
        .and_then(Value::as_array)
        .ok_or_else(|| HostError::new("Windows notification data is missing notifications array"))?;
    let mut notifications = Vec::new();
    for item in notificationsValue {
        notifications.push(NotificationEntry {
            packageName: json_string(item, "packageName")?,
            text: json_string(item, "text")?,
            timestamp: json_i64(item, "timestamp")?,
        });
    }
    Ok(NotificationData {
        notifications,
        timestamp,
    })
}

fn get_windows_app_usage_time(
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
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$requestedName = {packageName}
$sinceHours = {sinceHours}
$limit = {limit}
$includeSystemApps = {includeSystemApps}
$now = [DateTimeOffset]::UtcNow
$startWindow = $now.AddHours(-1 * $sinceHours)
$items = @()
foreach ($process in Get-Process) {{
    try {{
        $name = [string]$process.ProcessName
        if ($requestedName -and $requestedName.Trim().Length -gt 0 -and $name -notlike "*$requestedName*") {{
            continue
        }}
        $path = ""
        try {{ $path = [string]$process.Path }} catch {{}}
        $isSystem = $false
        if ($path -match "^[A-Za-z]:\\Windows\\" -or $path -match "^[A-Za-z]:\\Program Files\\WindowsApps\\") {{
            $isSystem = $true
        }}
        if (-not $includeSystemApps -and $isSystem) {{
            continue
        }}
        $startTime = [DateTimeOffset]$process.StartTime
        $effectiveStart = if ($startTime -gt $startWindow) {{ $startTime }} else {{ $startWindow }}
        $durationMs = [Math]::Max(0, [int64]($now - $effectiveStart).TotalMilliseconds)
        $displayName = $name
        if ($process.MainWindowTitle -and $process.MainWindowTitle.Trim().Length -gt 0) {{
            $displayName = "$name - $($process.MainWindowTitle)"
        }}
        $items += [pscustomobject]@{{
            packageName = $name
            appName = $displayName
            totalForegroundTimeMs = $durationMs
            lastTimeUsed = [long]$now.ToUnixTimeMilliseconds()
            isSystemApp = $isSystem
        }}
    }} catch {{}}
}}
$items = $items | Sort-Object -Property totalForegroundTimeMs -Descending | Select-Object -First $limit
[pscustomobject]@{{
    startTime = [long]$startWindow.ToUnixTimeMilliseconds()
    endTime = [long]$now.ToUnixTimeMilliseconds()
    sinceHours = $sinceHours
    requestedPackageName = $requestedName
    includesSystemApps = $includeSystemApps
    totalEntries = @($items).Count
    entries = @($items)
}} | ConvertTo-Json -Depth 8 -Compress
"#,
        packageName = ps_string_literal(packageName),
        sinceHours = sinceHours,
        limit = limit,
        includeSystemApps = if includeSystemApps { "$true" } else { "$false" },
    );
    parse_app_usage_time(run_powershell_json(&script, "get Windows app usage time")?)
}

fn get_windows_device_location(
    timeout: i32,
    highAccuracy: bool,
    includeAddress: bool,
) -> HostResult<LocationData> {
    if timeout <= 0 {
        return Err(HostError::new("timeout must be greater than 0"));
    }

    let timeoutMs = timeout.saturating_mul(1000);
    let desiredAccuracy = if highAccuracy { "High" } else { "Default" };
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Runtime.WindowsRuntime
[Windows.Devices.Geolocation.Geolocator, Windows.Devices.Geolocation, ContentType=WindowsRuntime] > $null
[Windows.Devices.Geolocation.Geoposition, Windows.Devices.Geolocation, ContentType=WindowsRuntime] > $null
$locator = [Windows.Devices.Geolocation.Geolocator]::new()
$locator.DesiredAccuracy = [Windows.Devices.Geolocation.PositionAccuracy]::{desiredAccuracy}
$operation = $locator.GetGeopositionAsync()
$asTask = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{
    $_.Name -eq 'AsTask' -and $_.IsGenericMethod -and $_.GetParameters().Count -eq 1
}} | Select-Object -First 1)
$task = $asTask.MakeGenericMethod([Windows.Devices.Geolocation.Geoposition]).Invoke($null, @($operation))
if (-not $task.Wait({timeoutMs})) {{
    throw 'Windows location request timed out.'
}}
$position = $task.Result
$coordinate = $position.Coordinate
$point = $coordinate.Point.Position
$civic = $position.CivicAddress
$address = ''
$city = ''
$province = ''
$country = ''
if ({includeAddress}) {{
    if ($null -ne $civic) {{
        $city = [string]$civic.City
        $province = [string]$civic.State
        $country = [string]$civic.Country
        $addressParts = @($city, $province, $country, [string]$civic.PostalCode) | Where-Object {{ $_ -and $_.Trim().Length -gt 0 }}
        $address = [string]::Join(', ', $addressParts)
    }}
}}
[pscustomobject]@{{
    latitude = [double]$point.Latitude
    longitude = [double]$point.Longitude
    accuracy = [double]$coordinate.Accuracy
    provider = 'windows-geolocator'
    timestamp = [long]$coordinate.Timestamp.ToUnixTimeMilliseconds()
    rawData = ''
    address = $address
    city = $city
    province = $province
    country = $country
}} | ConvertTo-Json -Compress
"#,
        desiredAccuracy = desiredAccuracy,
        timeoutMs = timeoutMs,
        includeAddress = if includeAddress { "$true" } else { "$false" },
    );

    let output = Command::new("powershell.exe")
        .arg("-NoProfile")
        .arg("-Command")
        .arg(script)
        .output()
        .map_err(|error| HostError::new(format!("Failed to query Windows location: {error}")))?;

    if !output.status.success() {
        let errorText = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(HostError::new(format!(
            "Failed to query Windows location: {errorText}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let value: Value = serde_json::from_str(&stdout)
        .map_err(|error| HostError::new(format!("Failed to parse Windows location data: {error}")))?;
    Ok(LocationData {
        latitude: json_f64(&value, "latitude")?,
        longitude: json_f64(&value, "longitude")?,
        accuracy: json_f64(&value, "accuracy")? as f32,
        provider: json_string(&value, "provider")?,
        timestamp: json_i64(&value, "timestamp")?,
        rawData: json_string(&value, "rawData")?,
        address: json_string(&value, "address")?,
        city: json_string(&value, "city")?,
        province: json_string(&value, "province")?,
        country: json_string(&value, "country")?,
    })
}

fn get_windows_device_info() -> HostResult<DeviceInfoData> {
    let script = r#"
$ErrorActionPreference = 'Stop'
function Format-ByteSize {
    param([int64]$Size)
    $kb = 1024.0
    $mb = $kb * 1024.0
    $gb = $mb * 1024.0
    $tb = $gb * 1024.0
    if ($Size -lt $kb) { return "$Size B" }
    if ($Size -lt $mb) { return ('{0:N2} KB' -f ($Size / $kb)) }
    if ($Size -lt $gb) { return ('{0:N2} MB' -f ($Size / $mb)) }
    if ($Size -lt $tb) { return ('{0:N2} GB' -f ($Size / $gb)) }
    return ('{0:N2} TB' -f ($Size / $tb))
}
$machineGuid = [string](Get-ItemPropertyValue -Path 'HKLM:\SOFTWARE\Microsoft\Cryptography' -Name MachineGuid)
$computer = Get-CimInstance Win32_ComputerSystem
$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$screen = Get-CimInstance Win32_VideoController | Select-Object -First 1
$drive = Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='C:'" | Select-Object -First 1
$battery = Get-CimInstance Win32_Battery | Select-Object -First 1
$network = Get-CimInstance Win32_NetworkAdapter -Filter "NetEnabled=True" | Select-Object -First 1
$buildNumber = 0
[int]::TryParse([string]$os.BuildNumber, [ref]$buildNumber) > $null
$batteryLevel = 0
$batteryCharging = $false
if ($null -ne $battery) {
    $batteryLevel = [int]$battery.EstimatedChargeRemaining
    $batteryCharging = ([int]$battery.BatteryStatus -eq 2)
}
$screenResolution = ''
if ($null -ne $screen -and $screen.CurrentHorizontalResolution -and $screen.CurrentVerticalResolution) {
    $screenResolution = "$($screen.CurrentHorizontalResolution)x$($screen.CurrentVerticalResolution)"
}
$networkType = ''
if ($null -ne $network) {
    $networkType = [string]$network.NetConnectionID
    if (-not $networkType -or $networkType.Trim().Length -eq 0) {
        $networkType = [string]$network.Name
    }
}
$additionalInfo = [ordered]@{
    'Device name' = [string]$computer.Name
    'Product name' = [string]$computer.Model
    'Hardware name' = [string]$computer.SystemType
    'Build fingerprint' = [string]$os.BuildNumber
    'Build time' = [string]$os.InstallDate
}
[pscustomobject]@{
    deviceId = $machineGuid
    model = [string]$computer.Model
    manufacturer = [string]$computer.Manufacturer
    androidVersion = [string]$os.Caption + ' ' + [string]$os.Version
    sdkVersion = $buildNumber
    screenResolution = $screenResolution
    screenDensity = 1.0
    totalMemory = Format-ByteSize ([int64]$computer.TotalPhysicalMemory)
    availableMemory = Format-ByteSize ([int64]($os.FreePhysicalMemory * 1024))
    totalStorage = Format-ByteSize ([int64]$drive.Size)
    availableStorage = Format-ByteSize ([int64]$drive.FreeSpace)
    batteryLevel = $batteryLevel
    batteryCharging = $batteryCharging
    cpuInfo = [string]$cpu.Name
    networkType = $networkType
    additionalInfo = $additionalInfo
} | ConvertTo-Json -Depth 8 -Compress
"#;
    parse_device_info_data(run_powershell_json(script, "get Windows device info")?)
}

fn json_f64(value: &Value, key: &str) -> HostResult<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| HostError::new(format!("Windows location data is missing numeric field: {key}")))
}

fn json_i64(value: &Value, key: &str) -> HostResult<i64> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| HostError::new(format!("Windows location data is missing integer field: {key}")))
}

fn json_string(value: &Value, key: &str) -> HostResult<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| HostError::new(format!("Windows location data is missing string field: {key}")))
}

fn parse_app_operation_data(value: Value) -> HostResult<AppOperationData> {
    Ok(AppOperationData {
        operationType: json_string(&value, "operationType")?,
        packageName: json_string(&value, "packageName")?,
        success: json_bool(&value, "success")?,
        details: json_string(&value, "details")?,
    })
}

fn parse_app_usage_time(value: Value) -> HostResult<AppUsageTimeResultData> {
    let entriesValue = value
        .get("entries")
        .and_then(Value::as_array)
        .ok_or_else(|| HostError::new("Windows app usage data is missing entries array"))?;
    let mut entries = Vec::new();
    for entry in entriesValue {
        entries.push(AppUsageTimeEntry {
            packageName: json_string(entry, "packageName")?,
            appName: json_string(entry, "appName")?,
            totalForegroundTimeMs: json_i64(entry, "totalForegroundTimeMs")?,
            lastTimeUsed: json_i64(entry, "lastTimeUsed")?,
            isSystemApp: json_bool(entry, "isSystemApp")?,
        });
    }
    Ok(AppUsageTimeResultData {
        startTime: json_i64(&value, "startTime")?,
        endTime: json_i64(&value, "endTime")?,
        sinceHours: json_i32(&value, "sinceHours")?,
        requestedPackageName: json_optional_string(&value, "requestedPackageName"),
        includesSystemApps: json_bool(&value, "includesSystemApps")?,
        totalEntries: json_i32(&value, "totalEntries")?,
        entries,
    })
}

fn parse_device_info_data(value: Value) -> HostResult<DeviceInfoData> {
    let mut additionalInfo = BTreeMap::new();
    let additionalInfoValue = value
        .get("additionalInfo")
        .and_then(Value::as_object)
        .ok_or_else(|| HostError::new("Windows device info is missing additionalInfo object"))?;
    for (key, value) in additionalInfoValue {
        additionalInfo.insert(key.clone(), value.as_str().unwrap_or_default().to_string());
    }
    Ok(DeviceInfoData {
        deviceId: json_string(&value, "deviceId")?,
        model: json_string(&value, "model")?,
        manufacturer: json_string(&value, "manufacturer")?,
        androidVersion: json_string(&value, "androidVersion")?,
        sdkVersion: json_i32(&value, "sdkVersion")?,
        screenResolution: json_string(&value, "screenResolution")?,
        screenDensity: json_f64(&value, "screenDensity")? as f32,
        totalMemory: json_string(&value, "totalMemory")?,
        availableMemory: json_string(&value, "availableMemory")?,
        totalStorage: json_string(&value, "totalStorage")?,
        availableStorage: json_string(&value, "availableStorage")?,
        batteryLevel: json_i32(&value, "batteryLevel")?,
        batteryCharging: json_bool(&value, "batteryCharging")?,
        cpuInfo: json_string(&value, "cpuInfo")?,
        networkType: json_string(&value, "networkType")?,
        additionalInfo,
    })
}

fn run_powershell_json(script: &str, operation: &str) -> HostResult<Value> {
    let output = Command::new("powershell.exe")
        .arg("-NoProfile")
        .arg("-Command")
        .arg(script)
        .output()
        .map_err(|error| HostError::new(format!("Failed to {operation}: {error}")))?;
    if !output.status.success() {
        let errorText = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(HostError::new(format!("Failed to {operation}: {errorText}")));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    serde_json::from_str(&stdout)
        .map_err(|error| HostError::new(format!("Failed to parse {operation} result: {error}")))
}

fn ps_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn json_bool(value: &Value, key: &str) -> HostResult<bool> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| HostError::new(format!("Windows data is missing boolean field: {key}")))
}

fn json_i32(value: &Value, key: &str) -> HostResult<i32> {
    let raw = json_i64(value, key)?;
    i32::try_from(raw)
        .map_err(|_| HostError::new(format!("Windows data integer field is out of range: {key}")))
}

fn json_optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|text| !text.is_empty())
}
