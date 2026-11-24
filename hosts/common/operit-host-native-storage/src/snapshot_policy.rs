//! Keeps native execution environments outside portable application snapshots.
use std::fs;
use std::path::{Path, PathBuf};

use operit_host_contract::{HostError, HostResult};

/// Native execution payloads and launch profiles contain device-specific files and paths.
const PRIVATE_PATHS: &[&str] = &[
    "terminal/rootfs",
    "config/preferences/terminal_profiles.json",
    "ohos-vroot",
];

/// Classifies runtime-relative paths by components, including isolated identity owners.
pub fn is_host_private_runtime_path(mut path: &Path) -> bool {
    if let Ok(identity) = path.strip_prefix("identities") {
        let mut components = identity.components();
        components.next();
        path = components.as_path();
    }
    PRIVATE_PATHS.iter().any(|root| path.starts_with(root))
}

/// Lists existing private roots without descending into rootfs, links, or mounted guest trees.
pub fn host_private_runtime_roots(runtime: &Path) -> HostResult<Vec<PathBuf>> {
    let mut owners = vec![runtime.to_path_buf()];
    let identities = runtime.join("identities");
    match fs::symlink_metadata(&identities) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            for entry in fs::read_dir(identities)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    owners.push(entry.path());
                }
            }
        }
        Ok(_) => return Err(HostError::new("Identity storage is not a regular directory")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let mut roots = Vec::new();
    for owner in owners {
        for relative in PRIVATE_PATHS {
            let path = owner.join(relative);
            match fs::symlink_metadata(&path) {
                Ok(_) => roots.push(path),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checks component boundaries and identity ownership without reading guest contents.
    #[test]
    fn private_paths_are_anchored_to_runtime_owners() {
        for (path, excluded) in [
            ("", false),
            ("identities", false),
            ("identities/a", false),
            ("terminal/rootfs", true),
            ("identities/a/terminal/rootfs/image/sdcard", true),
            ("config/preferences/terminal_profiles.json", true),
            ("identities/a/ohos-vroot/rootfs/bin/sh", true),
            ("identities/a/terminal", false),
            ("terminal/rootfs-notes/file", false),
            ("data/terminal/rootfs/file", false),
            ("identities/a/data/terminal/rootfs/file", false),
            ("config/preferences/terminal_profiles.json.backup", false),
        ] {
            assert_eq!(is_host_private_runtime_path(Path::new(path)), excluded, "{path}");
        }
    }
}
