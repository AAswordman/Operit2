//! Stable registry/Appx IDs and exact executable paths for Windows package tools.
use super::{parse_app_operation_data, ps_string_literal, run_powershell_json};
use operit_host_api::{AppListData, AppOperationData, HostError, HostResult};
const CATALOG: &str = r#"
$ErrorActionPreference = 'Stop'
function Read-Apps {
    $roots = @('HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*','HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*')
    @(Get-ItemProperty $roots -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName } | ForEach-Object {
        [pscustomobject]@{ id = 'win32:' + $_.PSPath; registry = $_; appx = $null; system = ($_.SystemComponent -eq 1 -or [bool]$_.ParentKeyName) }
    }) + @(Get-AppxPackage | ForEach-Object {
        [pscustomobject]@{ id = 'appx:' + $_.Name; registry = $null; appx = $_; system = ($_.IsFramework -or $_.NonRemovable -or $_.IsResourcePackage) }
    })
}
function Resolve-App($id) {
    $matches = @(Read-Apps | Where-Object { $_.id -eq $id -or ($null -ne $_.registry -and ($_.registry.DisplayName -eq $id -or $_.registry.PSChildName -eq $id)) -or ($null -ne $_.appx -and ($_.appx.Name -eq $id -or $_.appx.PackageFullName -eq $id)) })
    if ($matches.Count -ne 1) { throw 'Application missing or ambiguous. Use an ID from list_installed_apps.' }
    return $matches[0]
}
function Resolve-Executable($app) {
    $icon = [Environment]::ExpandEnvironmentVariables([string]$app.registry.DisplayIcon)
    if ($icon -match '^"([^"]+\.exe)"(?:,.*)?$') { $candidate=$Matches[1] }
    elseif ($icon -match '^(.+\.exe)(?:,.*)?$') { $candidate=$Matches[1] }
    else { $candidate='' }
    if ($candidate -and (Test-Path -LiteralPath $candidate -PathType Leaf)) { return (Get-Item -LiteralPath $candidate).FullName }
    $location = [Environment]::ExpandEnvironmentVariables([string]$app.registry.InstallLocation)
    if ($location -and (Test-Path -LiteralPath $location -PathType Container)) {
        $files = @(Get-ChildItem -LiteralPath $location -Filter '*.exe' -File | Where-Object { $_.Name -notmatch '(?i)unins|setup|update|crash' })
        if ($files.Count -eq 1) { return $files[0].FullName }
    }
    throw 'No unambiguous executable registered for this application. Pass an absolute executable path for start_app/stop_app.'
}
"#;
pub(super) fn list(include: bool) -> HostResult<AppListData> {
    let script=format!("{CATALOG}\n$items = @(Read-Apps | Where-Object {{ {} -or -not $_.system }} | ForEach-Object {{ $_.id }} | Sort-Object -Unique)\nConvertTo-Json -InputObject $items -Compress",if include {"$true"}else{"$false"});
    let value = run_powershell_json(&script, "list Windows applications")?;
    let packages = value
        .as_array()
        .ok_or_else(|| HostError::new("Invalid application catalog"))?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    Ok(AppListData {
        includesSystemApps: include,
        packages,
    })
}
pub(super) fn operation(name: &str, operation: &str) -> HostResult<AppOperationData> {
    if name.trim().is_empty() {
        return Err(HostError::new("package_name is required"));
    }
    let action = match operation {
        "start" => {
            r#"
if ([System.IO.Path]::IsPathRooted($name) -and (Test-Path -LiteralPath $name -PathType Leaf) -and [System.IO.Path]::GetExtension($name) -eq '.exe') {
    Start-Process -FilePath $name | Out-Null
} else {
    $app=Resolve-App $name
    if ($null -ne $app.appx) {
        $family=$app.appx.PackageFamilyName + '!'
        $launchers=@(Get-StartApps | Where-Object { $_.AppID.StartsWith($family, [StringComparison]::OrdinalIgnoreCase) })
        if ($launchers.Count -ne 1) { throw 'Package has no single launchable entry; use its application-specific launcher.' }
        Start-Process explorer.exe -ArgumentList ('shell:AppsFolder\' + $launchers[0].AppID) | Out-Null
    } else { Start-Process -FilePath (Resolve-Executable $app) | Out-Null }
}
$details='Application launch requested'
"#
        }
        "stop" => {
            r#"
$app=$null
if ([System.IO.Path]::IsPathRooted($name) -and (Test-Path -LiteralPath $name -PathType Leaf) -and [System.IO.Path]::GetExtension($name) -eq '.exe') { $exe=(Get-Item -LiteralPath $name).FullName }
else { $app=Resolve-App $name; if ($null -eq $app.appx) { $exe=Resolve-Executable $app } }
$matches=@(Get-Process | Where-Object {
    try {
        if ($null -ne $app -and $null -ne $app.appx) { $_.Path.StartsWith($app.appx.InstallLocation.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase) }
        else { $_.Path -eq $exe }
    } catch { $false }
})
if ($matches.Count -eq 0) { throw 'No running process matched the exact application executable/package directory.' }
if (@($matches | Where-Object { $_.Id -eq $PID }).Count -gt 0) { throw 'Cannot stop the active operation host.' }
$matches | Stop-Process -ErrorAction Stop
$details='Matching application processes stopped'
"#
        }
        "uninstall" => {
            r#"
$app=Resolve-App $name
if ($app.system) { throw 'Protected system components cannot be uninstalled through this tool.' }
if ($null -ne $app.appx) { Remove-AppxPackage -Package $app.appx.PackageFullName }
else {
    if ($app.registry.WindowsInstaller -eq 1 -and $app.registry.PSChildName -match '^\{[0-9a-fA-F-]{36}\}$') {
        $process=Start-Process msiexec.exe -ArgumentList ('/x ' + $app.registry.PSChildName) -Wait -PassThru
    } else {
        $command=[Environment]::ExpandEnvironmentVariables([string]$app.registry.UninstallString)
        if ($command -match '^"([^"]+\.exe)"\s*(.*)$') { $exe=$Matches[1]; $arguments=$Matches[2] }
        elseif ($command -match '^(.+?\.exe)(?:\s+(.*))?$') { $exe=$Matches[1]; $arguments=$Matches[2] }
        else { throw 'No supported executable uninstaller is registered for this application.' }
        if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
            $resolved=Get-Command -Name $exe -CommandType Application -ErrorAction Stop
            $exe=$resolved.Source
        }
        # Pass the registered command line directly, without cmd.exe or Invoke-Expression.
        if ([string]::IsNullOrWhiteSpace($arguments)) { $process=Start-Process -FilePath $exe -Wait -PassThru }
        else { $process=Start-Process -FilePath $exe -ArgumentList $arguments -Wait -PassThru }
    }
    if ($process.ExitCode -notin @(0,1641,3010)) { throw "Uninstaller exited with $($process.ExitCode); canceled or failed." }
    if (Get-ItemProperty -LiteralPath $app.registry.PSPath -ErrorAction SilentlyContinue) {
        $details='Uninstaller exited successfully; registration remains. Complete any system prompts or restart. Removal is not yet verified.'
    }
}
if (-not $details) { $details='Application uninstalled' }
"#
        }
        _ => return Err(HostError::new("Unknown application operation")),
    };
    let script=format!("{CATALOG}\n$name={}\n$details=''\n{action}\n[pscustomobject]@{{ operationType={}; packageName=$name; success=$true; details=$details }} | ConvertTo-Json -Compress",ps_string_literal(name),ps_string_literal(operation));
    parse_app_operation_data(run_powershell_json(
        &script,
        &format!("{operation} Windows application"),
    )?)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_catalog_and_no_shell_uninstall() {
        assert!(CATALOG.contains("'win32:' + $_.PSPath"));
        assert!(CATALOG.contains("'appx:' + $_.Name"));
        assert_eq!(
            ps_string_literal("x'; Stop-Process *; '"),
            "'x''; Stop-Process *; '''"
        );
    }
}
