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
        crate::PathMapper::PathMapper::storageLocationPath(path)
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
        use operit_host_contract::HostFileSystemRoots::HostFileSystemRoots;
        for path in [
            "/sdcard/Download/Operit",
            "/storage/emulated/0/Download/Operit",
        ] {
            let selected = VfsPath::storageLocationPath(path);
            let roots = HostFileSystemRoots::current();
            match roots {
                HostFileSystemRoots::Android => assert_eq!(selected.unwrap(), "/virtual/android/sdcard/Download/Operit"),
                HostFileSystemRoots::Windows | HostFileSystemRoots::Browser | HostFileSystemRoots::Unsupported => assert!(selected.is_err()),
                _ => assert_eq!(selected.unwrap(), format!("{}{path}", roots.posixVfsRoot().unwrap())),
            }
        }
        assert_eq!(
            VfsPath::storageLocationPath("/operitapp/data").unwrap(),
            "/operitapp/data"
        );
        assert!(VfsPath::storageLocationPath("relative-folder").is_err());
    }
}
