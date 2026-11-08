//! Desktop IDs round-trip through list/start/stop/uninstall. Commands never interpolate a shell.
use operit_host_api::{AppInstallState, AppListData, AppOperationData, HostError, HostResult};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn checked(command: &mut Command) -> HostResult<String> {
    let output = command
        .output()
        .map_err(|e| HostError::new(format!("Cannot run {command:?}: {e}")))?;
    if !output.status.success() {
        return Err(HostError::new(format!(
            "{command:?}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
fn result(operation: &str, name: &str, details: &str) -> AppOperationData {
    AppOperationData {
        operationType: operation.into(),
        packageName: name.into(),
        success: true,
        details: details.into(),
        installState: None,
    }
}
fn data_home() -> HostResult<PathBuf> {
    if let Some(path) = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        return Ok(path);
    }
    Ok(
        PathBuf::from(env::var_os("HOME").ok_or_else(|| HostError::new("HOME is unavailable"))?)
            .join(".local/share"),
    )
}
fn roots() -> HostResult<Vec<PathBuf>> {
    let mut roots = vec![data_home()?.join("applications")];
    let dirs = env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    roots.extend(
        env::split_paths(&dirs)
            .filter(|p| p.is_absolute())
            .map(|p| p.join("applications")),
    );
    roots.push(data_home()?.join("flatpak/exports/share/applications"));
    roots.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    roots.push(PathBuf::from("/var/lib/snapd/desktop/applications"));
    Ok(roots)
}
fn fields(content: &str) -> BTreeMap<String, String> {
    let mut active = false;
    let mut fields = BTreeMap::new();
    for line in content.lines().map(str::trim) {
        if line.starts_with('[') {
            active = line == "[Desktop Entry]";
            continue;
        }
        if active && !line.starts_with('#') {
            if let Some((k, v)) = line.split_once('=') {
                fields.insert(k.into(), v.into());
            }
        }
    }
    fields
}
struct Desktop {
    path: PathBuf,
    fields: BTreeMap<String, String>,
}
fn scan(dir: &Path, root: &Path, entries: &mut BTreeMap<String, Desktop>) -> HostResult<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() && !path.is_symlink() {
            scan(&path, root, entries)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
            let id = path
                .strip_prefix(root)
                .map_err(|e| HostError::new(e.to_string()))?
                .to_string_lossy()
                .replace('/', "-");
            if !entries.contains_key(&id) {
                if let Ok(content) = fs::read_to_string(&path) {
                    entries.insert(
                        id,
                        Desktop {
                            path,
                            fields: fields(&content),
                        },
                    );
                }
            }
        }
    }
    Ok(())
}
fn desktops() -> HostResult<BTreeMap<String, Desktop>> {
    let mut apps = BTreeMap::new();
    for root in roots()? {
        scan(&root, &root, &mut apps)?;
    }
    Ok(apps)
}
fn desktop(name: &str) -> HostResult<Desktop> {
    if name.trim().is_empty() || name.starts_with('-') {
        return Err(HostError::new(
            "A desktop ID from list_installed_apps is required",
        ));
    }
    let app = desktops()?
        .remove(name)
        .ok_or_else(|| HostError::new(format!("Desktop ID not installed: {name}")))?;
    if app.fields.get("Hidden").is_some_and(|s| s == "true") {
        return Err(HostError::new("Application is hidden/uninstalled"));
    }
    Ok(app)
}
pub(super) fn list(include: bool) -> HostResult<AppListData> {
    let packages = desktops()?
        .into_iter()
        .filter(|(_, app)| {
            app.fields.get("Type").is_some_and(|s| s == "Application")
                && !app.fields.get("Hidden").is_some_and(|s| s == "true")
                && (include || !app.fields.get("NoDisplay").is_some_and(|s| s == "true"))
        })
        .map(|(id, _)| id)
        .collect();
    Ok(AppListData {
        includesSystemApps: include,
        packages,
    })
}
pub(super) fn start(name: &str) -> HostResult<AppOperationData> {
    let app = desktop(name)?;
    checked(Command::new("gio").arg("launch").arg(app.path))?;
    Ok(result(
        "start",
        name,
        "Desktop application launch requested",
    ))
}
// Extract the executable only; do not execute Desktop Entry Exec as shell text.
fn executable(exec: &str) -> HostResult<String> {
    let exec = exec.trim();
    let value = if let Some(rest) = exec.strip_prefix('"') {
        rest.split_once('"').map(|(v, _)| v)
    } else {
        exec.split_whitespace().next()
    }
    .filter(|s| !s.is_empty())
    .ok_or_else(|| HostError::new("Desktop entry has no executable"))?;
    if matches!(
        Path::new(value).file_name().and_then(|s| s.to_str()),
        Some("env" | "sh" | "bash" | "flatpak" | "snap")
    ) {
        return Err(HostError::new(
            "Cannot safely identify this launcher process; use the application's own quit action",
        ));
    }
    if value.contains('\\') || value.contains('%') {
        return Err(HostError::new(
            "Unsupported escaped executable in desktop entry",
        ));
    }
    Ok(value.into())
}
pub(super) fn stop(name: &str) -> HostResult<AppOperationData> {
    let app = desktop(name)?;
    if let Some(id) = app.fields.get("X-Flatpak") {
        checked(Command::new("flatpak").args(["kill", id]))?;
    } else {
        let executable = executable(app.fields.get("Exec").map(String::as_str).unwrap_or(""))?;
        let target = if Path::new(&executable).is_absolute() {
            fs::canonicalize(&executable)?
        } else {
            let paths = env::var_os("PATH").unwrap_or_default();
            env::split_paths(&paths)
                .map(|p| p.join(&executable))
                .find(|p| p.is_file())
                .ok_or_else(|| HostError::new("Application executable was not found"))?
                .canonicalize()?
        };
        let uid = checked(Command::new("id").arg("-u"))?;
        let pids = checked(Command::new("ps").args(["-u", &uid, "-o", "pid="]))?;
        let mut count = 0;
        for pid in pids
            .split_whitespace()
            .filter(|p| p.chars().all(|c| c.is_ascii_digit()))
        {
            if pid.parse::<u32>().ok() == Some(std::process::id()) {
                continue;
            }
            if fs::read_link(format!("/proc/{pid}/exe")).ok().as_ref() == Some(&target) {
                checked(Command::new("kill").args(["-TERM", "--", pid]))?;
                count += 1;
            }
        }
        if count == 0 {
            return Err(HostError::new(
                "No running process matched the application's exact executable",
            ));
        }
    }
    Ok(result(
        "stop",
        name,
        "Termination requested for matching application processes",
    ))
}
pub(super) fn install(path: &str) -> HostResult<AppOperationData> {
    if let Some(result) = operit_host_native_common::AppInstaller::installPortableApp(path)? {
        return Ok(result);
    }
    let package = Path::new(path);
    if !package.is_absolute() || !package.is_file() {
        return Err(HostError::new(
            "Installer must be an existing absolute file",
        ));
    }
    let extension = package
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(
        extension.as_str(),
        "deb" | "rpm" | "flatpakref" | "flatpak" | "snap"
    ) {
        return Err(HostError::new("Supported installers: .deb, .rpm, .flatpakref, .flatpak, .snap and Operit portable archives. AppImage files are portable executables, not installed packages."));
    }
    checked(Command::new("xdg-open").arg(package))?;
    let mut data = result(
        "install",
        path,
        "System package installer opened; complete authentication and installation there",
    );
    data.installState = Some(AppInstallState::InstallerOpened);
    Ok(data)
}
pub(super) fn uninstall(name: &str) -> HostResult<AppOperationData> {
    let app = desktop(name)?;
    if let Some(id) = app.fields.get("X-Flatpak") {
        let user = app.path.starts_with(data_home()?);
        checked(Command::new("flatpak").args([
            "uninstall",
            if user { "--user" } else { "--system" },
            "--noninteractive",
            "-y",
            id,
        ]))?;
    } else if let Some(id) = app.fields.get("X-SnapInstanceName") {
        checked(Command::new("pkexec").args(["snap", "remove", id]))?;
    } else {
        // Resolve the owning package. Removing a .desktop file is not uninstalling an application.
        let file = fs::canonicalize(&app.path)?;
        let file = file.to_string_lossy();
        if let Ok(owner) = checked(Command::new("dpkg-query").args(["-S", &file])) {
            let package = owner
                .lines()
                .next()
                .and_then(|l| l.split_once(": "))
                .map(|(p, _)| p)
                .filter(|p| valid_package(p))
                .ok_or_else(|| HostError::new("Ambiguous Debian package ownership"))?;
            checked(Command::new("pkexec").args(["apt-get", "remove", "-y", "--", package]))?;
        } else if let Ok(owner) =
            checked(Command::new("rpm").args(["-qf", "--qf", "%{NAME}", &file]))
        {
            if !valid_package(&owner) {
                return Err(HostError::new("Invalid RPM package owner"));
            }
            // rpm checks dependencies instead of silently removing dependent packages.
            checked(Command::new("pkexec").args(["rpm", "-e", "--", &owner]))?;
        } else {
            return Err(HostError::new("No package manager owns this desktop entry. Use the application's uninstaller; its shortcut has been preserved."));
        }
    }
    Ok(result(
        "uninstall",
        name,
        "Package manager completed uninstall",
    ))
}
fn valid_package(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('-')
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".+-_:".contains(&c))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn desktop_groups_and_hidden_overrides() {
        let f = fields("[Desktop Entry]\nType=Application\nName=Hello\nExec=\"/opt/My App/app\" %U\nHidden=true\n[Desktop Action X]\nExec=wrong");
        assert_eq!(executable(&f["Exec"]).unwrap(), "/opt/My App/app");
        assert_eq!(f["Hidden"], "true");
        assert!(executable("sh -c anything").is_err());
        assert!(executable("env X=1 app").is_err());
        assert!(!valid_package("--all"));
        assert!(valid_package("app:amd64"));
    }
    #[test]
    fn desktop_ids_keep_user_override() {
        let dir = env::temp_dir().join(format!(
            "operit-desktop-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let user = dir.join("user");
        let system = dir.join("system/vendor");
        fs::create_dir_all(&user).unwrap();
        fs::create_dir_all(&system).unwrap();
        fs::write(
            user.join("vendor-app.desktop"),
            "[Desktop Entry]\nHidden=true",
        )
        .unwrap();
        fs::write(
            system.join("app.desktop"),
            "[Desktop Entry]\nType=Application",
        )
        .unwrap();
        let mut apps = BTreeMap::new();
        scan(&user, &user, &mut apps).unwrap();
        scan(&dir.join("system"), &dir.join("system"), &mut apps).unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps["vendor-app.desktop"].fields["Hidden"], "true");
        fs::remove_dir_all(dir).unwrap();
    }
}
