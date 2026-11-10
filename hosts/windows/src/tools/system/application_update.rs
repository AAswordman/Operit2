//! Self-update policy for this Windows app copy; ordinary install_app remains interactive.
use super::{ps_string_literal, run_powershell_json};
use operit_host_api::{
    AppInstallState, AppOperationData, ApplicationUpdatePackage, ApplicationUpdateTarget,
    HostError, HostResult,
};
use operit_host_native_common::AppInstaller::{
    installPortableApp, spawnApplicationUpdateHelper, waitForApplicationUpdateHelper,
};
use std::ffi::OsStr;
use std::path::Path;

const SCRIPT: &str = include_str!("application_update.ps1");

/// Queries the registered MSI ownership of the actual running executable.
fn installation(executable: &Path) -> HostResult<bool> {
    let executable = executable
        .to_str()
        .ok_or_else(|| HostError::new("Application executable path is not UTF-8"))?;
    let script = format!(
        "{SCRIPT}\nGet-OperitInstallation {} | ConvertTo-Json -Compress",
        ps_string_literal(executable),
    );
    run_powershell_json(&script, "identify Windows application installation")?
        .get("msi")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| HostError::new("Windows installation identity omitted its MSI state"))
}

/// Resolves this app copy's release format without changing other installed copies.
pub(super) fn target() -> HostResult<Option<ApplicationUpdateTarget>> {
    let executable = std::fs::canonicalize(std::env::current_exe()?)?;
    Ok(Some(ApplicationUpdateTarget {
        installTiming: operit_host_api::AppInstallTiming::AfterProcessExit,
        arch: std::env::consts::ARCH.into(),
        package: if installation(&executable)? {
            ApplicationUpdatePackage::WindowsMsi
        } else {
            ApplicationUpdatePackage::WindowsZip
        },
    }))
}

/// Starts a verified MSI helper and waits for its acknowledgement before returning.
pub(super) fn prepare(package: &str) -> HostResult<AppOperationData> {
    let path = Path::new(package);
    if !path.is_absolute() || !path.is_file() {
        return Err(HostError::new(
            "Update package must be a readable absolute file",
        ));
    }
    let executable = std::fs::canonicalize(std::env::current_exe()?)?;
    if !installation(&executable)? {
        return installPortableApp(package)?.ok_or_else(|| {
            HostError::new("This portable application requires its matching ZIP update package")
        });
    }
    if path.extension().and_then(OsStr::to_str) != Some("msi") {
        return Err(HostError::new(
            "This MSI installation requires its matching MSI update package",
        ));
    }
    let work = path
        .parent()
        .ok_or_else(|| HostError::new("Update directory is missing"))?;
    let token = uuid::Uuid::new_v4();
    let ready = work.join(format!("installer-{token}.ready"));
    let log = work.join(format!("installer-{token}.log"));
    let pid = std::process::id().to_string();
    let source = format!(
        "param([int]$ParentPid, [string]$Package, [string]$Executable, [string]$Ready, [string]$Log)\n{SCRIPT}\nInvoke-OperitMsiUpdate $ParentPid $Package $Executable $Ready $Log",
    );
    let mut child = spawnApplicationUpdateHelper(
        &source,
        &[
            OsStr::new(&pid),
            path.as_os_str(),
            executable.as_os_str(),
            ready.as_os_str(),
            log.as_os_str(),
        ],
        &ready,
        &log,
    )?;
    waitForApplicationUpdateHelper(&mut child, &ready, &log)?;
    Ok(AppOperationData {
        installState: Some(AppInstallState::RestartRequired),
        operationType: "install".into(),
        packageName: package.into(),
        success: true,
        details: format!(
            "更新已准备，退出后将静默升级并重新打开。安装日志：{}",
            log.display()
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exercises the real Windows launcher with Unicode paths and literal shell metacharacters.
    #[test]
    fn detached_helper_preserves_unicode_and_literal_arguments() {
        let root = std::env::temp_dir().join(format!("operit 更新 ' {}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let ready = root.join("helper.ready");
        let log = root.join("helper.log");
        let value = "原样传递 & '$value; (应用)";
        let mut child = spawnApplicationUpdateHelper(
            "param([string]$Value, [string]$Ready)\n[IO.File]::WriteAllText($Ready, $Value)\nStart-Sleep -Milliseconds 150",
            &[OsStr::new(value), ready.as_os_str()],
            &ready,
            &log,
        ).unwrap();
        waitForApplicationUpdateHelper(&mut child, &ready, &log).unwrap();
        assert!(child.wait().unwrap().success());
        assert_eq!(std::fs::read_to_string(&ready).unwrap(), value);
        assert!(std::fs::read(ready.with_extension("ps1"))
            .unwrap()
            .starts_with(&[0xef, 0xbb, 0xbf]));
        std::fs::remove_dir_all(root).unwrap();
    }
}
