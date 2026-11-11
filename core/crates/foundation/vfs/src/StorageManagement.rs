//! Storage location reporting for settings and migration, separate from business file access.
use serde::{Deserialize, Serialize};

use crate::VfsPath;
use crate::VisualFileSystem::VisualFileSystem;

/// Describes a stable application alias and the storage location backing it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct StorageLocation {
    pub vfsPath: String,
    pub locationVfsPath: String,
    /// A settings display value; file consumers must use one of the virtual addresses.
    pub displayPath: String,
}

impl VfsPath {
    /// Converts a user-selected storage directory without applying plugin compatibility aliases.
    pub fn storageLocationPath(path: &str) -> Result<String, String> {
        let path = path.trim().replace('\\', "/");
        match path.split('/').nth(1) {
            Some("operitapp" | "app" | "virtual" | "mnt") if path.starts_with('/') => {
                Self::canonicalizeVfsPath(&path)
            }
            _ => Self::normalizeWorkspaceBindingPath(&path),
        }
    }
}

impl VisualFileSystem {
    /// Reports actual storage locations only through the explicit storage management interface.
    pub fn storageLocations(&self) -> Result<Vec<StorageLocation>, String> {
        ["/operitapp/data", "/operitapp/workspaces"]
            .into_iter()
            .map(|path| {
                let resolved = self.resolvePath(path)?;
                let native = resolved.nativePath()?;
                Ok(StorageLocation {
                    vfsPath: path.into(),
                    locationVfsPath: VfsPath::storageLocationPath(&native)?,
                    displayPath: native,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::VfsPath;

    /// Keeps selected shared-storage folders at their actual location instead of plugin storage.
    #[test]
    fn selected_shared_storage_preserves_its_physical_location() {
        for path in [
            "/sdcard/Download/Operit",
            "/storage/emulated/0/Download/Operit",
        ] {
            assert_eq!(
                VfsPath::storageLocationPath(path).unwrap(),
                "/virtual/android/sdcard/Download/Operit"
            );
        }
        assert_eq!(
            VfsPath::storageLocationPath("/operitapp/data").unwrap(),
            "/operitapp/data"
        );
        assert!(VfsPath::storageLocationPath("relative-folder").is_err());
    }
}
