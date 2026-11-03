use crate::{HostError, HostResult, TerminalHost};

/// Dispatches Linux guest VFS locators independently of the runtime host filesystem.
pub fn resolveLinuxGuestDirectory(
    workingDir: &str,
    resolveHostDirectory: &dyn Fn(&str) -> HostResult<String>,
) -> HostResult<String> {
    let path = workingDir.trim();
    match path.strip_prefix("/virtual/linux").or_else(|| path.strip_prefix("/mnt/linux")) {
        Some("") => Ok("/".to_string()),
        Some(relative) if relative.starts_with('/') => Ok(relative.to_string()),
        _ => resolveHostDirectory(path),
    }
}

/// Resolves a tool-created session's directory in its selected implementation.
pub fn resolveSessionDirectory(
    host: &dyn TerminalHost,
    requestedType: Option<&str>,
    workspace: &str,
    resolveHostDirectory: &dyn Fn(&str) -> HostResult<String>,
) -> HostResult<String> {
    let info = host.terminalInfo()?;
    let (terminal, terminalType) = if let Some(kind) = requestedType {
        let selected = info.types.iter().find(|entry| entry.terminalType == kind && entry.available)
            .ok_or_else(|| HostError::new(format!("Terminal type is unavailable: {kind}")))?;
        (selected.terminal.as_str(), selected.terminalType.as_str())
    } else {
        (info.terminal.as_str(), info.terminalType.as_str())
    };
    host.resolveWorkingDirectory(terminal, terminalType, workspace, resolveHostDirectory)
}

/// Builds a literal directory change for the selected interpreter.
pub fn directoryChangeCommand(terminalType: &str, workingDir: &str) -> HostResult<String> {
    if workingDir.is_empty() || workingDir.contains('\0') {
        return Err(HostError::new("Terminal working directory is empty or contains NUL"));
    }
    match terminalType {
        "powershell" => Ok(format!(
            "Set-Location -LiteralPath '{}' -ErrorAction Stop",
            workingDir.replace('\'', "''"),
        )),
        "bash" | "zsh" | "shell" => Ok(format!(
            "cd '{}'",
            workingDir.replace('\'', "'\\''"),
        )),
        value => Err(HostError::new(format!("Unsupported terminal type: {value}"))),
    }
}

/// Enters the resolved workspace after shell startup and rejects failed changes.
pub fn enterTerminalDirectory(
    host: &dyn TerminalHost,
    sessionId: &str,
    terminalType: &str,
    workingDir: &str,
) -> HostResult<()> {
    let command = directoryChangeCommand(terminalType, workingDir)?;
    let result = host.executeInSession(sessionId, &command, 120000)?;
    if result.timedOut || result.exitCode != 0 {
        return Err(HostError::new(format!(
            "Cannot enter terminal working directory {workingDir}: timedOut={}, exitCode={}\n{}",
            result.timedOut, result.exitCode, result.output.trim(),
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HostError;
    use std::cell::Cell;

    /// Resolves guest roots without consulting an unavailable native Linux mount.
    #[test]
    fn guest_roots_do_not_require_host_mounts() {
        let resolveHost = |_: &str| Err(HostError::new("/virtual/linux is not mounted"));
        for path in ["/virtual/linux", "/virtual/linux/", " /virtual/linux "] {
            assert_eq!(resolveLinuxGuestDirectory(path, &resolveHost).unwrap(), "/");
        }
    }

    #[test]
    fn both_guest_mounts_use_the_same_terminal_namespace() {
        let rejectHost = |_: &str| Err(HostError::new("host must not resolve guest aliases"));
        assert_eq!(resolveLinuxGuestDirectory("/mnt/linux", &rejectHost).unwrap(), "/");
        assert_eq!(resolveLinuxGuestDirectory("/mnt/linux/home/project", &rejectHost).unwrap(), "/home/project");
        let host = |path: &str| Ok(path.to_string());
        assert_eq!(resolveLinuxGuestDirectory("/app/workspaces/demo", &host).unwrap(), "/app/workspaces/demo");
    }


    /// Keeps guest subdirectories in the selected Linux terminal namespace.
    #[test]
    fn guest_subdirectories_are_not_host_paths() {
        let calls = Cell::new(0);
        let resolveHost = |_: &str| {
            calls.set(calls.get() + 1);
            Err(HostError::new("host resolver must not receive guest paths"))
        };
        assert_eq!(
            resolveLinuxGuestDirectory("/virtual/linux/home/user/work space", &resolveHost).unwrap(),
            "/home/user/work space",
        );
        assert_eq!(calls.get(), 0);
    }

    /// Preserves runtime and host-mounted workspace mappings without replacing errors.
    #[test]
    fn host_namespace_errors_are_preserved() {
        let calls = Cell::new(0);
        let resolveHost = |path: &str| {
            calls.set(calls.get() + 1);
            Err(HostError::new(format!("not mounted: {path}")))
        };
        for path in [
            "/operitapp/workspaces/project",
            "/virtual/android/sdcard/project",
            "/virtual/linuxish/project",
        ] {
            assert_eq!(
                resolveLinuxGuestDirectory(path, &resolveHost)
                    .unwrap_err()
                    .message,
                format!("not mounted: {path}"),
            );
        }
        assert_eq!(calls.get(), 3);
    }

    /// Preserves the physical directory chosen by the host VFS for shared storage.
    #[test]
    fn host_workspace_uses_its_resolved_physical_directory() {
        let resolveHost = |path: &str| {
            assert_eq!(path, "/operitapp/workspaces/project");
            Ok("/data/user/0/app.operit/files/workspaces/project".to_string())
        };
        assert_eq!(
            resolveLinuxGuestDirectory("/operitapp/workspaces/project", &resolveHost).unwrap(),
            "/data/user/0/app.operit/files/workspaces/project",
        );
    }
}
