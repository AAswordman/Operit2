//! Portable desktop packages are installed through SystemOperationHost::installApp.
//! All extraction/replacement lives in the host; the shared core owns downloads; clients only present status.
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Component, Path};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use operit_host_api::{AppOperationData, HostError, HostResult};

/// Returns None for ordinary installers so each host retains its normal installation behavior.
pub fn installPortableApp(package: &str) -> HostResult<Option<AppOperationData>> {
    let path = Path::new(package);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let platform = std::env::consts::OS;
    let portable = match platform {
        "windows" => name.starts_with("operit2-app-windows-") && name.ends_with(".zip"),
        "linux" => name.starts_with("operit2-app-linux-") && name.ends_with(".tar.gz"),
        _ => false,
    };
    if !portable {
        return Ok(None);
    }
    if !path.is_absolute() || !path.is_file() {
        return Err(HostError::new("Installer must be a readable absolute file"));
    }
    let executable = fs::canonicalize(std::env::current_exe()?)?;
    let target = executable
        .parent()
        .ok_or_else(|| HostError::new("Application directory is missing"))?;
    let executableName = executable
        .file_name()
        .ok_or_else(|| HostError::new("Application executable is missing"))?;
    if !target.join("data/flutter_assets").is_dir() {
        return Err(HostError::new(
            "当前运行的不是完整发行版，请从发布页下载安装包",
        ));
    }
    let parent = target
        .parent()
        .ok_or_else(|| HostError::new("Application parent directory is missing"))?;
    let token = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let stage = parent.join(format!(".operit-install-{token}"));
    let backup = parent.join(format!(".operit-backup-{token}"));
    let lock = parent.join(format!(".operit-install-{}.lock", std::process::id()));
    let _lockFile = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .map_err(|error| {
            HostError::new(format!(
                "无法准备更新（可能已有安装任务，或安装目录不可写）：{error}"
            ))
        })?;
    let result = (|| -> HostResult<AppOperationData> {
        fs::create_dir(&stage)?;
        extractPackage(path, &stage)?;
        if !stage.join(executableName).is_file() || !stage.join("data/flutter_assets").is_dir() {
            return Err(HostError::new("更新包缺少应用程序或资源文件"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let app = stage.join(executableName);
            let mode = fs::metadata(&app)?.permissions().mode();
            fs::set_permissions(app, fs::Permissions::from_mode(mode | 0o100))?;
        }
        let work = path
            .parent()
            .ok_or_else(|| HostError::new("Installer directory is missing"))?;
        let ready = work.join(format!("installer-{token}.ready"));
        let log = work.join(format!("installer-{token}.log"));
        let mut child =
            spawnInstaller(target, &stage, &backup, executableName, &lock, &ready, &log)?;
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() {
            if child.try_wait()?.is_some() || Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(HostError::new(format!(
                    "更新安装助手未能启动，应用保持打开。日志：{}",
                    log.display()
                )));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(AppOperationData {
            installState: Some(operit_host_api::AppInstallState::RestartRequired),
            operationType: "install".to_string(),
            packageName: package.to_string(),
            success: true,
            details: format!(
                "更新已准备，将在应用退出后替换并重新打开；旧版备份：{}；安装日志：{}",
                backup.display(),
                log.display()
            ),
        })
    })();
    if result.is_err() {
        drop(_lockFile);
        let _ = fs::remove_file(&lock);
        let _ = fs::remove_dir_all(&stage);
    }
    result.map(Some)
}

/// Both decoders reject paths and links that would escape the private staging directory.
fn extractPackage(package: &Path, stage: &Path) -> HostResult<()> {
    if package
        .extension()
        .is_some_and(|extension| extension == "zip")
    {
        let mut archive = zip::ZipArchive::new(File::open(package)?)
            .map_err(|error| HostError::new(format!("安装包损坏：{error}")))?;
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|error| HostError::new(error.to_string()))?;
            let enclosed = entry
                .enclosed_name()
                .ok_or_else(|| HostError::new("安装包包含无效路径"))?;
            if entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            {
                return Err(HostError::new("Windows 安装包不支持符号链接"));
            }
            let output = stage.join(enclosed);
            if entry.is_dir() {
                fs::create_dir_all(output)?;
            } else {
                if let Some(parent) = output.parent() {
                    fs::create_dir_all(parent)?;
                }
                io::copy(&mut entry, &mut File::create(output)?)?;
            }
        }
    } else {
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(File::open(package)?));
        for entry in archive.entries()? {
            let mut entry = entry?;
            if entry.path()?.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            }) {
                return Err(HostError::new("安装包包含无效路径"));
            }
            if entry.header().entry_type().is_symlink()
                || entry.header().entry_type().is_hard_link()
            {
                let link = entry
                    .link_name()?
                    .ok_or_else(|| HostError::new("安装包包含无效链接"))?;
                if link.components().any(|component| {
                    matches!(
                        component,
                        Component::ParentDir | Component::RootDir | Component::Prefix(_)
                    )
                }) {
                    return Err(HostError::new("安装包链接超出安装目录"));
                }
            } else if !entry.header().entry_type().is_file()
                && !entry.header().entry_type().is_dir()
            {
                return Err(HostError::new("安装包包含不支持的文件类型"));
            }
            if !entry.unpack_in(stage)? {
                return Err(HostError::new("安装包路径超出安装目录"));
            }
        }
    }
    Ok(())
}

fn spawnInstaller(
    target: &Path,
    stage: &Path,
    backup: &Path,
    executable: &std::ffi::OsStr,
    lock: &Path,
    ready: &Path,
    log: &Path,
) -> HostResult<Child> {
    let script = ready.with_extension(if cfg!(windows) { "ps1" } else { "sh" });
    fs::write(
        &script,
        if cfg!(windows) {
            WINDOWS_INSTALLER
        } else {
            UNIX_INSTALLER
        },
    )?;
    let mut command;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command = Command::new("powershell.exe");
        // Detach from the UI process while keeping the helper invisible.
        command.creation_flags(0x08000000 | 0x00000200);
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ]);
    }
    #[cfg(not(windows))]
    {
        command = Command::new("/bin/sh");
    }
    let logFile = File::create(log)?;
    command
        .arg(script)
        .arg(std::process::id().to_string())
        .arg(target)
        .arg(stage)
        .arg(backup)
        .arg(executable)
        .arg(lock)
        .arg(ready)
        .stdin(Stdio::null())
        .stdout(logFile.try_clone()?)
        .stderr(logFile)
        .spawn()
        .map_err(|error| HostError::new(format!("无法启动安装助手：{error}")))
}

const UNIX_INSTALLER: &str = r#"#!/bin/sh
set -eu
parent_pid=$1
target=$2
staged=$3
backup=$4
executable=$5
lock=$6
ready=$7
moved_old=0
cleanup() {
  code=$?
  trap - EXIT
  rm -f "$lock"
  if [ "$code" -ne 0 ]; then
    if [ "$moved_old" -eq 1 ]; then
      if [ -d "$target" ]; then mv "$target" "$staged.failed" || true; fi
      mv "$backup" "$target" || true
    fi
    printf '%s\n' 'Installation failed; previous version restored.'
  fi
  if [ "$moved_old" -eq 1 ] && [ -f "$target/$executable" ]; then
    nohup "$target/$executable" </dev/null >/dev/null 2>&1 &
  fi
  exit "$code"
}
trap cleanup EXIT
: > "$ready"
while kill -0 "$parent_pid" 2>/dev/null; do sleep 1; done
mv "$target" "$backup"
moved_old=1
mv "$staged" "$target"
printf '%s\n' 'installed'
"#;

const WINDOWS_INSTALLER: &str = r#"param(
  [int]$ParentPid, [string]$Target, [string]$Staged, [string]$Backup,
  [string]$Executable, [string]$Lock, [string]$Ready
)
$ErrorActionPreference = 'Stop'
$movedOld = $false
try {
  Set-Content -LiteralPath $Ready -Value 'ready'
  while (Get-Process -Id $ParentPid -ErrorAction SilentlyContinue) { Start-Sleep -Seconds 1 }
  for ($attempt = 0; ; $attempt++) {
    try { Move-Item -LiteralPath $Target -Destination $Backup; break }
    catch { if ($attempt -ge 9) { throw }; Start-Sleep -Seconds 1 }
  }
  $movedOld = $true
  Move-Item -LiteralPath $Staged -Destination $Target
  Start-Process -FilePath (Join-Path $Target $Executable) -WorkingDirectory $Target
  Write-Output 'installed'
} catch {
  Write-Output ($_ | Out-String)
  if ($movedOld) {
    if (Test-Path -LiteralPath $Target) { Move-Item -LiteralPath $Target -Destination ($Staged + '.failed') }
    Move-Item -LiteralPath $Backup -Destination $Target
    Start-Process -FilePath (Join-Path $Target $Executable) -WorkingDirectory $Target
  }
} finally {
  Remove-Item -LiteralPath $Lock -ErrorAction SilentlyContinue
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn fixture() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "operit-installer-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }
    #[test]
    fn zip_extracts_bundle_and_rejects_parent_traversal() {
        let dir = fixture();
        let package = dir.join("app.zip");
        for name in ["data/flutter_assets/asset", "../escape"] {
            let mut archive = zip::ZipWriter::new(File::create(&package).unwrap());
            archive
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            archive.write_all(b"fixture").unwrap();
            archive.finish().unwrap();
            let stage = dir.join("stage");
            fs::create_dir_all(&stage).unwrap();
            let result = extractPackage(&package, &stage);
            if name.starts_with("..") {
                assert!(result.is_err());
                assert!(!dir.join("escape").exists());
            } else {
                result.unwrap();
                assert_eq!(fs::read(stage.join(name)).unwrap(), b"fixture");
            }
        }
        fs::remove_dir_all(dir).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn helper_swaps_and_rolls_back_without_touching_running_application() {
        use std::os::unix::fs::PermissionsExt;
        for fail in [false, true] {
            let dir = fixture();
            let target = dir.join("installed app");
            let stage = dir.join("staged app");
            let backup = dir.join("backup");
            fs::create_dir_all(&target).unwrap();
            fs::write(target.join("version"), b"old").unwrap();
            let executable = target.join("operit");
            fs::write(&executable, b"#!/bin/sh\nexit 0\n").unwrap();
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
            if !fail {
                fs::create_dir_all(&stage).unwrap();
                fs::write(stage.join("version"), b"new").unwrap();
                fs::write(stage.join("operit"), b"#!/bin/sh\nexit 0\n").unwrap();
                fs::set_permissions(stage.join("operit"), fs::Permissions::from_mode(0o700))
                    .unwrap();
            }
            let script = dir.join("install.sh");
            fs::write(&script, UNIX_INSTALLER).unwrap();
            // A reaped child PID ensures no running app/process is terminated.
            let mut parent = Command::new("/bin/sh")
                .args(["-c", "exit 0"])
                .spawn()
                .unwrap();
            let pid = parent.id();
            parent.wait().unwrap();
            let status = Command::new("/bin/sh")
                .arg(&script)
                .arg(pid.to_string())
                .arg(&target)
                .arg(&stage)
                .arg(&backup)
                .arg("operit")
                .arg(dir.join("lock"))
                .arg(dir.join("ready"))
                .status()
                .unwrap();
            assert_eq!(status.success(), !fail);
            assert_eq!(
                fs::read(target.join("version")).unwrap(),
                if fail { b"old" } else { b"new" }
            );
            if !fail {
                assert_eq!(fs::read(backup.join("version")).unwrap(), b"old");
            }
            fs::remove_dir_all(dir).unwrap();
        }
    }
}
