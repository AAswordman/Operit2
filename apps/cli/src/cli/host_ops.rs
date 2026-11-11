use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use flate2::read::GzDecoder;
use tar::Archive;
use zip::ZipArchive;
use operit_host_api::FileSource;
use operit_util::GithubReleaseUtil::FullUpdateTarget;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use super::{
    current_cli_install_state, install_cli_from_source, CliInstallState,
    DownloadedUpdateInstallStatus, InstallMode, InstallOutput,
};

/// Resolves a storage-management selection only at the CLI platform configuration boundary.
pub(crate) fn storage_target_native_path(path: &str) -> Result<PathBuf, String> {
    let context = crate::bootstrap::create_cli_host_manager();
    let filesystem = context.vfs()?;
    let file = filesystem.file(path)?;
    file.nativePath().map(PathBuf::from).map_err(|error| error.to_string())
}

/// Resolves the CLI release target at the existing native Host dispatch boundary.
pub(crate) fn cli_update_target() -> Result<FullUpdateTarget, String> {
    FullUpdateTarget::new(
        "cli",
        std::env::consts::OS,
        std::env::consts::ARCH,
    )
}

pub(super) const WINDOWS_CLI_RUNTIME_LIBRARIES: &[&str] = &["operit_local_llm.dll"];
pub(super) const WINDOWS_CLI_RUNTIME_DIRECTORIES: &[&str] = &["bash"];

/// Returns the runtime libraries installed beside this host's CLI executable.
pub(super) fn cli_runtime_libraries() -> &'static [&'static str] {
    platform_host().runtime_libraries()
}

/// Returns complete runtime trees installed beside this host's executable.
pub(super) fn cli_runtime_directories() -> &'static [&'static str] {
    platform_host().runtime_directories()
}

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
    /// Describes complete runtime directories required by this host's CLI.
    fn runtime_directories(&self) -> &'static [&'static str];
    /// Describes the native runtime files required by this host's CLI.
    fn runtime_libraries(&self) -> &'static [&'static str];

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
    /// Keeps the bundled Bash tree available after install and update.
    fn runtime_directories(&self) -> &'static [&'static str] {
        WINDOWS_CLI_RUNTIME_DIRECTORIES
    }
    /// Keeps the native inference DLL next to both installed Windows commands.
    fn runtime_libraries(&self) -> &'static [&'static str] {
        WINDOWS_CLI_RUNTIME_LIBRARIES
    }

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
    /// Unix CLI packages use their host terminal runtime.
    fn runtime_directories(&self) -> &'static [&'static str] {
        &[]
    }
    /// Unix CLI builds embed the native inference implementation statically.
    fn runtime_libraries(&self) -> &'static [&'static str] {
        &[]
    }

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

/// Schedules replacement of the Windows commands and their native runtime.
#[cfg(windows)]
fn schedule_windows_cli_update(
    source: &Path,
    operit: &Path,
    operit2: &Path,
    install_dir: &Path,
) -> Result<(), String> {
    let runtime_copies = WINDOWS_CLI_RUNTIME_LIBRARIES
        .iter()
        .map(|name| {
            format!(
                "Copy-OperitFile '{}' '{}'\n",
                ps_single_quote(&source.with_file_name(name)),
                ps_single_quote(&install_dir.join(name))
            )
        })
        .collect::<String>();
    let runtime_directories = WINDOWS_CLI_RUNTIME_DIRECTORIES
        .iter()
        .map(|name| {
            format!(
                "Copy-OperitDirectory '{}' '{}' '{}'\n",
                ps_single_quote(&source.with_file_name(name)),
                ps_single_quote(&install_dir.join(name)),
                ps_single_quote(install_dir),
            )
        })
        .collect::<String>();
    let script = format!(
        r#"$ErrorActionPreference = "Stop"
Start-Sleep -Milliseconds 800
Stop-OperitProcesses
Copy-OperitFile '{}' '{}'
Copy-OperitFile '{}' '{}'
{runtime_copies}{runtime_directories}[Environment]::SetEnvironmentVariable('Path', (Update-OperitPath [Environment]::GetEnvironmentVariable('Path', 'User') '{}'), 'User')
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

/// Schedules removal of Windows commands and their native runtime after exit.
#[cfg(windows)]
fn schedule_windows_cli_uninstall(
    operit: &Path,
    operit2: &Path,
    install_dir: &Path,
) -> Result<(), String> {
    let runtime_removals = WINDOWS_CLI_RUNTIME_LIBRARIES
        .iter()
        .map(|name| {
            format!(
                "Remove-OperitFile '{}'\n",
                ps_single_quote(&install_dir.join(name))
            )
        })
        .collect::<String>();
    let runtime_directories = WINDOWS_CLI_RUNTIME_DIRECTORIES
        .iter()
        .map(|name| {
            format!(
                "Remove-OperitDirectory '{}' '{}'\n",
                ps_single_quote(&install_dir.join(name)),
                ps_single_quote(install_dir),
            )
        })
        .collect::<String>();
    let script = format!(
        r#"$ErrorActionPreference = "Stop"
Start-Sleep -Milliseconds 800
Stop-OperitProcesses
Remove-OperitFile '{}'
Remove-OperitFile '{}'
{runtime_removals}{runtime_directories}[Environment]::SetEnvironmentVariable('Path', (Remove-OperitPath [Environment]::GetEnvironmentVariable('Path', 'User') '{}'), 'User')
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
# Removes only a concrete runtime child inside the selected installation.
function Remove-OperitDirectory([string]$Path, [string]$InstallDir) {
    $Target = [IO.Path]::GetFullPath($Path)
    $Root = [IO.Path]::GetFullPath($InstallDir)
    if (-not (Test-OperitPathEqual ([IO.Path]::GetDirectoryName($Target)) $Root)) {
        throw "Runtime directory escapes CLI installation: $Target"
    }
    if (Test-Path -LiteralPath $Target) {
        $Item = Get-Item -LiteralPath $Target -Force
        if ($Item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Runtime directory is a link: $Target" }
        Remove-Item -LiteralPath $Target -Recurse -Force
    }
}
# Replaces the complete bundled tree after the running CLI processes have stopped.
function Copy-OperitDirectory([string]$Source, [string]$Destination, [string]$InstallDir) {
    if (-not (Test-Path -LiteralPath $Source -PathType Container)) { throw "Missing runtime directory: $Source" }
    Remove-OperitDirectory $Destination $InstallDir
    Copy-Item -LiteralPath $Source -Destination $Destination -Recurse -Force
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

/// Consumes a VFS file reference and installs the CLI package inside the native host boundary.
pub(crate) fn install_downloaded_cli_update(
    target: &FullUpdateTarget,
    source: &dyn FileSource,
    output: InstallOutput,
) -> Result<DownloadedUpdateInstallStatus, String> {
    if target.product != "cli" {
        return Ok(DownloadedUpdateInstallStatus::TargetMismatch);
    }
    let current_target = crate::cli::host_ops::cli_update_target()?;
    if target != &current_target {
        return Ok(DownloadedUpdateInstallStatus::TargetMismatch);
    }
    let native_path = source.nativePath().map_err(|error| error.to_string())?;
    let source = extract_cli_binary_from_package(target, Path::new(&native_path))?;
    let install_state = current_cli_install_state()?;
    match install_state {
        CliInstallState::Installed => {
            install_cli_from_source(&source, InstallMode::Update, output, |_| {})?;
            Ok(DownloadedUpdateInstallStatus::Scheduled)
        }
        CliInstallState::NotInstalled => Ok(DownloadedUpdateInstallStatus::NotInstalled),
    }
}

/// Extracts a validated native CLI package beside its VFS download task.
fn extract_cli_binary_from_package(
    target: &FullUpdateTarget,
    package_path: &Path,
) -> Result<PathBuf, String> {
    let extract_dir = package_path.parent().ok_or("CLI package has no parent directory")?
        .join(format!("cli-{}", super::unique_suffix()));
    fs::create_dir_all(&extract_dir).map_err(|error| error.to_string())?;
    match target.platform.as_str() {
        "windows" => extract_cli_binary_from_zip(package_path, &extract_dir),
        "linux" | "macos" => extract_cli_binary_from_tar_gz(package_path, &extract_dir),
        other => Err(format!("Unsupported CLI update package platform: {other}")),
    }
}

/// Extracts the Windows executable together with its required runtime libraries.
pub(super) fn extract_cli_binary_from_zip(package_path: &Path, extract_dir: &Path) -> Result<PathBuf, String> {
    let file = fs::File::open(package_path).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|error| error.to_string())?;
    for name in std::iter::once("operit2.exe")
        .chain(WINDOWS_CLI_RUNTIME_LIBRARIES.iter().copied())
    {
        let mut entry = archive
            .by_name(name)
            .map_err(|error| format!("{name} not found in {}: {error}", package_path.display()))?;
        let destination = extract_dir.join(name);
        let mut output = fs::File::create(&destination).map_err(|error| error.to_string())?;
        io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
    }
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| format!("Invalid CLI package path: {}", entry.name()))?;
        if relative
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err(format!("Invalid CLI package path: {}", entry.name()));
        }
        if !WINDOWS_CLI_RUNTIME_DIRECTORIES
            .iter()
            .any(|name| relative.starts_with(name))
        {
            continue;
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(format!("Runtime package entry is a link: {}", entry.name()));
        }
        let destination = extract_dir.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
        } else {
            fs::create_dir_all(destination.parent().ok_or("Runtime file has no parent")?)
                .map_err(|error| error.to_string())?;
            let mut output = fs::File::create(&destination).map_err(|error| error.to_string())?;
            io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
        }
    }
    for name in [
        "bash/bin/bash.exe",
        "bash/usr/bin/msys-2.0.dll",
        "bash/LICENSE.txt",
    ] {
        if !extract_dir.join(name).is_file() {
            return Err(format!("{name} not found in {}", package_path.display()));
        }
    }
    Ok(extract_dir.join("operit2.exe"))
}

/// Extracts the executable from a native Unix CLI release archive.
fn extract_cli_binary_from_tar_gz(
    package_path: &Path,
    extract_dir: &Path,
) -> Result<PathBuf, String> {
    let file = fs::File::open(package_path).map_err(|error| error.to_string())?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    for entry in archive.entries().map_err(|error| error.to_string())? {
        let mut entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path().map_err(|error| error.to_string())?;
        if archive_path_is_exact_file(&path, "operit2") {
            let destination = extract_dir.join("operit2");
            entry
                .unpack(&destination)
                .map_err(|error| error.to_string())?;
            #[cfg(unix)]
            {
                let mut permissions = fs::metadata(&destination)
                    .map_err(|error| error.to_string())?
                    .permissions();
                permissions.set_mode(0o755);
                fs::set_permissions(&destination, permissions)
                    .map_err(|error| error.to_string())?;
            }
            return Ok(destination);
        }
    }
    Err(format!("operit2 not found in {}", package_path.display()))
}

/// Accepts only the declared executable at the archive root.
fn archive_path_is_exact_file(path: &Path, file_name: &str) -> bool {
    let mut components = path.components().filter_map(|component| match component {
        Component::CurDir => None,
        Component::Normal(value) => Some(value),
        _ => Some(OsStr::new("")),
    });
    let first = components.next();
    let second = components.next();
    first == Some(OsStr::new(file_name)) && second.is_none()
}
