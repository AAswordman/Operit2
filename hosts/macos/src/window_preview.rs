use operit_host_api::{HostError, HostResult};
use std::{hash::{Hash, Hasher}, os::unix::fs::PermissionsExt, path::Path, process::Command};

/// Materializes the build's exact native helper and starts it without activating another application.
pub fn command(directory: &Path) -> HostResult<Command> {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/operit-window-preview"));
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hash);
    let path = directory.join(format!("{:016x}", hash.finish()));
    std::fs::create_dir_all(directory).map_err(|error| HostError::new(error.to_string()))?;
    if !path.exists() {
        std::fs::write(&path, bytes).map_err(|error| HostError::new(error.to_string()))?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| HostError::new(error.to_string()))?;
    }
    if std::fs::read(&path).map_err(|error| HostError::new(error.to_string()))? != bytes {
        return Err(HostError::new("The native window preview does not match this host build"));
    }
    Ok(Command::new(path))
}
