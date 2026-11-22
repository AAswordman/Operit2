use operit_host_api::{HostError, HostResult};
use std::{hash::{Hash, Hasher}, path::Path, process::Command};

/// Runs the exact embedded provider from a file, outside Windows' command-line length limit.
pub fn command(directory: &Path) -> HostResult<Command> {
    let bytes = include_bytes!("ui_automation.ps1");
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hash);
    let path = directory.join(format!("{:016x}.ps1", hash.finish()));
    std::fs::create_dir_all(directory).map_err(|error| HostError::new(error.to_string()))?;
    if !path.exists() {
        std::fs::write(&path, bytes).map_err(|error| HostError::new(error.to_string()))?;
    }
    if std::fs::read(&path).map_err(|error| HostError::new(error.to_string()))? != bytes {
        return Err(HostError::new("The UI automation provider does not match this host build"));
    }
    let mut command = Command::new("powershell.exe");
    command.args(["-NoProfile", "-NonInteractive", "-STA", "-ExecutionPolicy", "Bypass", "-File"]).arg(path);
    Ok(command)
}
