use std::path::Path;
use std::process::{Command, Stdio};

pub(super) fn schedule_cli_update(
    source: &Path,
    operit: &Path,
    operit2: &Path,
    install_dir: &Path,
) -> Result<(), String> {
    platform_host().schedule_cli_update(source, operit, operit2, install_dir)
}

pub(super) fn schedule_cli_uninstall(
    operit: &Path,
    operit2: &Path,
    install_dir: &Path,
) -> Result<(), String> {
    platform_host().schedule_cli_uninstall(operit, operit2, install_dir)
}

trait CliHostOperations {
    fn schedule_cli_update(
        &self,
        source: &Path,
        operit: &Path,
        operit2: &Path,
        install_dir: &Path,
    ) -> Result<(), String>;

    fn schedule_cli_uninstall(
        &self,
        operit: &Path,
        operit2: &Path,
        install_dir: &Path,
    ) -> Result<(), String>;
}

#[cfg(windows)]
struct WindowsCliHostOperations;

#[cfg(not(windows))]
struct UnixCliHostOperations;

#[cfg(windows)]
fn platform_host() -> WindowsCliHostOperations {
    WindowsCliHostOperations
}

#[cfg(not(windows))]
fn platform_host() -> UnixCliHostOperations {
    UnixCliHostOperations
}

#[cfg(windows)]
impl CliHostOperations for WindowsCliHostOperations {
    fn schedule_cli_update(
        &self,
        source: &Path,
        operit: &Path,
        operit2: &Path,
        install_dir: &Path,
    ) -> Result<(), String> {
        schedule_windows_cli_update(source, operit, operit2, install_dir)
    }

    fn schedule_cli_uninstall(
        &self,
        operit: &Path,
        operit2: &Path,
        install_dir: &Path,
    ) -> Result<(), String> {
        schedule_windows_cli_uninstall(operit, operit2, install_dir)
    }
}

#[cfg(not(windows))]
impl CliHostOperations for UnixCliHostOperations {
    fn schedule_cli_update(
        &self,
        source: &Path,
        operit: &Path,
        operit2: &Path,
        install_dir: &Path,
    ) -> Result<(), String> {
        schedule_unix_cli_update(source, operit, operit2, install_dir)
    }

    fn schedule_cli_uninstall(
        &self,
        operit: &Path,
        operit2: &Path,
        install_dir: &Path,
    ) -> Result<(), String> {
        schedule_unix_cli_uninstall(operit, operit2, install_dir)
    }
}

#[cfg(windows)]
fn schedule_windows_cli_update(
    source: &Path,
    operit: &Path,
    operit2: &Path,
    install_dir: &Path,
) -> Result<(), String> {
    let script = format!(
        r#"$ErrorActionPreference = "Stop"
Start-Sleep -Milliseconds 800
Stop-OperitProcesses
Copy-OperitFile '{}' '{}'
Copy-OperitFile '{}' '{}'
[Environment]::SetEnvironmentVariable('Path', (Update-OperitPath [Environment]::GetEnvironmentVariable('Path', 'User') '{}'), 'User')
"#,
        ps_single_quote(source),
        ps_single_quote(operit),
        ps_single_quote(source),
        ps_single_quote(operit2),
        ps_single_quote(install_dir),
    );
    let helper = windows_path_update_function() + &script;
    spawn_detached_powershell_script(&helper)
}

#[cfg(windows)]
fn schedule_windows_cli_uninstall(
    operit: &Path,
    operit2: &Path,
    install_dir: &Path,
) -> Result<(), String> {
    let script = format!(
        r#"$ErrorActionPreference = "Stop"
Start-Sleep -Milliseconds 800
Stop-OperitProcesses
Remove-OperitFile '{}'
Remove-OperitFile '{}'
[Environment]::SetEnvironmentVariable('Path', (Remove-OperitPath [Environment]::GetEnvironmentVariable('Path', 'User') '{}'), 'User')
"#,
        ps_single_quote(operit),
        ps_single_quote(operit2),
        ps_single_quote(install_dir),
    );
    let helper = windows_path_update_function() + &script;
    spawn_detached_powershell_script(&helper)
}

#[cfg(windows)]
fn windows_path_update_function() -> String {
    r#"
function Split-OperitPath([string]$Value) {
    if ([string]::IsNullOrWhiteSpace($Value)) { return @() }
    return $Value -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
}
function Test-OperitPathEqual([string]$Left, [string]$Right) {
    return [string]::Equals($Left.TrimEnd('\', '/'), $Right.TrimEnd('\', '/'), [System.StringComparison]::OrdinalIgnoreCase)
}
function Update-OperitPath([string]$Value, [string]$InstallDir) {
    $Parts = @(Split-OperitPath $Value)
    foreach ($Part in $Parts) {
        if (Test-OperitPathEqual $Part $InstallDir) { return ($Parts -join ';') }
    }
    return (($Parts + $InstallDir) -join ';')
}
function Remove-OperitPath([string]$Value, [string]$InstallDir) {
    $Parts = @(Split-OperitPath $Value | Where-Object { -not (Test-OperitPathEqual $_ $InstallDir) })
    return ($Parts -join ';')
}
function Stop-OperitProcesses {
    $CurrentPid = $PID
    $Processes = Get-CimInstance Win32_Process | Where-Object {
        $_.ProcessId -ne $CurrentPid -and
        ($_.Name -ieq 'operit.exe' -or $_.Name -ieq 'operit2.exe')
    }
    foreach ($Process in $Processes) {
        try {
            Stop-Process -Id $Process.ProcessId -Force -ErrorAction Stop
        } catch {}
    }
}
function Copy-OperitFile([string]$Source, [string]$Destination) {
    for ($Index = 0; $Index -lt 80; $Index++) {
        try {
            Copy-Item -LiteralPath $Source -Destination $Destination -Force
            return
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }
    Copy-Item -LiteralPath $Source -Destination $Destination -Force
}
function Remove-OperitFile([string]$Path) {
    for ($Index = 0; $Index -lt 80; $Index++) {
        try {
            if (Test-Path -LiteralPath $Path) { Remove-Item -LiteralPath $Path -Force }
            return
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }
    if (Test-Path -LiteralPath $Path) { Remove-Item -LiteralPath $Path -Force }
}
"#
    .to_string()
}

#[cfg(not(windows))]
fn schedule_unix_cli_update(
    source: &Path,
    operit: &Path,
    operit2: &Path,
    install_dir: &Path,
) -> Result<(), String> {
    let script = format!(
        r#"set -eu
sleep 0.8
stop_operit_processes
copy_operit_file '{}' '{}'
copy_operit_file '{}' '{}'
update_operit_path '{}'
"#,
        shell_single_quote(source),
        shell_single_quote(operit),
        shell_single_quote(source),
        shell_single_quote(operit2),
        shell_single_quote(install_dir),
    );
    spawn_detached_shell_script(&(unix_cli_update_functions() + &script))
}

#[cfg(not(windows))]
fn schedule_unix_cli_uninstall(
    operit: &Path,
    operit2: &Path,
    install_dir: &Path,
) -> Result<(), String> {
    let script = format!(
        r#"set -eu
sleep 0.8
stop_operit_processes
remove_operit_file '{}'
remove_operit_file '{}'
remove_operit_path '{}'
"#,
        shell_single_quote(operit),
        shell_single_quote(operit2),
        shell_single_quote(install_dir),
    );
    spawn_detached_shell_script(&(unix_cli_update_functions() + &script))
}

#[cfg(not(windows))]
fn unix_cli_update_functions() -> String {
    r#"
stop_operit_processes() {
  pkill -x operit 2>/dev/null || true
  pkill -x operit2 2>/dev/null || true
}

copy_operit_file() {
  source_path="$1"
  destination_path="$2"
  destination_dir="$(dirname "$destination_path")"
  destination_name="$(basename "$destination_path")"
  temp_path="$destination_dir/.$destination_name.tmp.$$"
  cp "$source_path" "$temp_path"
  chmod 755 "$temp_path"
  mv -f "$temp_path" "$destination_path"
}

remove_operit_file() {
  rm -f "$1"
}

update_operit_path() {
  install_dir="$1"
  profile_file="$HOME/.profile"
  line='export PATH="$HOME/.local/bin:$PATH"'
  touch "$profile_file"
  grep -Fxq "$line" "$profile_file" || printf '\n%s\n' "$line" >> "$profile_file"
}

remove_operit_path() {
  profile_file="$HOME/.profile"
  line='export PATH="$HOME/.local/bin:$PATH"'
  if [ -f "$profile_file" ]; then
    temp_file="$profile_file.tmp.$$"
    grep -Fxv "$line" "$profile_file" > "$temp_file" || true
    mv -f "$temp_file" "$profile_file"
  fi
}
"#
    .to_string()
}

#[cfg(not(windows))]
fn shell_single_quote(path: &Path) -> String {
    path.display().to_string().replace('\'', "'\"'\"'")
}

#[cfg(not(windows))]
fn spawn_detached_shell_script(script: &str) -> Result<(), String> {
    Command::new("sh")
        .args(["-c", script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(windows)]
fn ps_single_quote(path: &Path) -> String {
    path.display().to_string().replace('\'', "''")
}

#[cfg(windows)]
fn spawn_detached_powershell_script(script: &str) -> Result<(), String> {
    let encoded = encode_powershell_command(script);
    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-EncodedCommand",
            &encoded,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(windows)]
fn encode_powershell_command(script: &str) -> String {
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;

    let bytes = script
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    BASE64.encode(bytes)
}
