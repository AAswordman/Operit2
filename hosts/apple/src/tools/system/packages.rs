//! macOS package operations use bundle IDs and the system's installation/Trash UI.
use super::{apple_script_string, run_command_output};
use operit_host_api::{AppInstallState, AppListData, AppOperationData, HostError, HostResult};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
fn result(operation: &str, name: &str, details: &str) -> AppOperationData {
    AppOperationData {
        operationType: operation.into(),
        packageName: name.into(),
        success: true,
        details: details.into(),
        installState: None,
    }
}
fn scan(root: &Path, apps: &mut BTreeMap<String, PathBuf>, depth: usize) -> HostResult<()> {
    if !root.is_dir() || depth > 4 {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("app") {
            let info = path.join("Contents/Info.plist");
            let id = run_command_output(
                "/usr/bin/plutil",
                &[
                    "-extract",
                    "CFBundleIdentifier",
                    "raw",
                    "-o",
                    "-",
                    &info.to_string_lossy(),
                ],
            )
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
            apps.entry(id).or_insert(path);
        } else if path.is_dir() && !path.is_symlink() {
            scan(&path, apps, depth + 1)?;
        }
    }
    Ok(())
}
fn apps(include: bool) -> HostResult<BTreeMap<String, PathBuf>> {
    let mut apps = BTreeMap::new();
    if let Some(home) = env::var_os("HOME") {
        scan(&PathBuf::from(home).join("Applications"), &mut apps, 0)?;
    }
    scan(Path::new("/Applications"), &mut apps, 0)?;
    if include {
        scan(Path::new("/System/Applications"), &mut apps, 0)?;
        scan(
            Path::new("/System/Library/CoreServices/Applications"),
            &mut apps,
            0,
        )?;
    }
    Ok(apps)
}
pub(super) fn list(include: bool) -> HostResult<AppListData> {
    Ok(AppListData {
        includesSystemApps: include,
        packages: apps(include)?.into_keys().collect(),
    })
}
fn resolve(name: &str) -> HostResult<PathBuf> {
    if name.trim().is_empty() {
        return Err(HostError::new("package_name is required"));
    }
    let installed = apps(true)?;
    if let Some(path) = installed.get(name) {
        return Ok(path.clone());
    }
    let matches = installed
        .values()
        .filter(|p| {
            p.to_string_lossy() == name || p.file_stem().and_then(|s| s.to_str()) == Some(name)
        })
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        return Ok(matches[0].clone());
    }
    Err(HostError::new("Application missing or name is ambiguous. Pass a bundle ID returned by list_installed_apps."))
}
pub(super) fn start(name: &str) -> HostResult<AppOperationData> {
    let path = resolve(name)?;
    run_command_output("/usr/bin/open", &["-a", &path.to_string_lossy()])?;
    Ok(result("start", name, "Application launch requested"))
}
pub(super) fn stop(name: &str) -> HostResult<AppOperationData> {
    let path = resolve(name)?;
    // Ask System Events for running processes first so quit never launches a stopped app.
    let script=format!("tell application \"System Events\" to get unix id of every application process whose file is (POSIX file {} as alias)",apple_script_string(&path.to_string_lossy()));
    let pids = run_command_output("/usr/bin/osascript", &["-e", &script])?;
    if pids.trim().is_empty() {
        return Err(HostError::new("Application is not running"));
    }
    let script = format!(
        "tell application {} to quit",
        apple_script_string(&path.to_string_lossy())
    );
    run_command_output("/usr/bin/osascript", &["-e", &script])?;
    Ok(result(
        "stop",
        name,
        "Application accepted quit request; unsaved documents may require user confirmation",
    ))
}
pub(super) fn uninstall(name: &str) -> HostResult<AppOperationData> {
    let path = fs::canonicalize(resolve(name)?)?;
    if path.starts_with("/System") || path.starts_with("/Library") {
        return Err(HostError::new(
            "Protected system applications cannot be uninstalled",
        ));
    }
    if env::current_exe()?.starts_with(&path) {
        return Err(HostError::new(
            "Quit Operit and remove it from Finder to uninstall the running host",
        ));
    }
    let script = format!(
        "tell application \"Finder\" to delete (POSIX file {} as alias)",
        apple_script_string(&path.to_string_lossy())
    );
    run_command_output("/usr/bin/osascript", &["-e", &script])?;
    if path.exists() {
        return Err(HostError::new(
            "Application was not moved to Trash; complete Finder authentication and retry",
        ));
    }
    Ok(result("uninstall",name,"Application bundle moved to Trash. User data and package receipts are retained; applications with services/drivers may require their vendor uninstaller."))
}
pub(super) fn install(path: &str) -> HostResult<AppOperationData> {
    let source = Path::new(path);
    if !source.is_absolute() || !source.exists() {
        return Err(HostError::new(
            "Installer must be an existing absolute path",
        ));
    }
    let extension = source
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if extension == "app" && source.join("Contents/Info.plist").is_file() {
        let app_name = source
            .file_name()
            .ok_or_else(|| HostError::new("Invalid application path"))?;
        let destination = Path::new("/Applications").join(app_name);
        if env::current_exe()?.starts_with(&destination) {
            return Err(HostError::new(
                "Use the signed .pkg updater to replace the currently running application",
            ));
        }
        if fs::canonicalize(source)? == destination {
            return Err(HostError::new("Application is already in /Applications"));
        }
        let script=format!("tell application \"Finder\" to duplicate (POSIX file {} as alias) to (POSIX file \"/Applications\" as alias) with replacing",apple_script_string(path));
        run_command_output("/usr/bin/osascript", &["-e", &script])?;
        if !destination.join("Contents/Info.plist").is_file() {
            return Err(HostError::new("Application copy did not complete"));
        }
        let mut data=result("install",&destination.to_string_lossy(),"Application copied into /Applications; first launch may require Gatekeeper confirmation");
        data.installState = Some(AppInstallState::Completed);
        return Ok(data);
    }
    if !source.is_file() || !matches!(extension.as_str(), "pkg" | "mpkg" | "dmg") {
        return Err(HostError::new("Supported macOS installers: .pkg, .mpkg, .dmg or an extracted .app bundle. Extract ZIP archives first."));
    }
    let output = Command::new("/usr/bin/open").arg(source).output()?;
    if !output.status.success() {
        return Err(HostError::new(format!(
            "Cannot open installer: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let mut data=result("install",path,"System installer/disk image opened. Complete the system confirmation; installation is not yet verified.");
    data.installState = Some(AppInstallState::InstallerOpened);
    Ok(data)
}
