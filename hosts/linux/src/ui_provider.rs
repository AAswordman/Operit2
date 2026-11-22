use operit_host_api::{HostError, HostResult};
use std::{hash::{Hash, Hasher}, path::Path, process::Command};

/// Runs the embedded provider and its capture module from one versioned host directory.
pub fn command(directory: &Path) -> HostResult<Command> {
    let files: [(&str, &[u8]); 2] = [
        ("ui_automation.py", include_bytes!("ui_automation.py")),
        ("window_surface.py", include_bytes!("window_surface.py")),
    ];
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    files.hash(&mut hash);
    let directory = directory.join(format!("{:016x}", hash.finish()));
    std::fs::create_dir_all(&directory).map_err(|error| HostError::new(error.to_string()))?;
    for (name, bytes) in files {
        let path = directory.join(name);
        if !path.exists() {
            std::fs::write(&path, bytes).map_err(|error| HostError::new(error.to_string()))?;
        }
        if std::fs::read(&path).map_err(|error| HostError::new(error.to_string()))? != bytes {
            return Err(HostError::new("The Linux UI provider does not match this host build"));
        }
    }
    let mut command = Command::new("python3");
    command.arg(directory.join("ui_automation.py"));
    Ok(command)
}
