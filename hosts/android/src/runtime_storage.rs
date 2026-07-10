use std::path::PathBuf;

use operit_host_api::{
    HostResult, HostSecretStore, RuntimeSqliteConnection, RuntimeSqliteHost, RuntimeStorageEntry,
    RuntimeStorageHost,
};

#[derive(Clone, Debug)]
pub struct AndroidRuntimeStorageHost {
    root: PathBuf,
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
    inner: operit_host_linux_native::LinuxRuntimeStorageHost,
}

impl AndroidRuntimeStorageHost {
    /// Creates an Android runtime storage host rooted at the supplied directory.
    pub fn new(root: PathBuf) -> Self {
        let runtimeRoot = root.join("runtime");
        let workspaceRoot = root.join("workspaces");
        Self::newWithRoots(root, runtimeRoot, workspaceRoot)
    }

    /// Creates an Android runtime storage host with explicit runtime and workspace roots.
    #[allow(non_snake_case)]
    pub fn newWithRoots(root: PathBuf, runtimeRoot: PathBuf, workspaceRoot: PathBuf) -> Self {
        Self {
            root: root.clone(),
            runtimeRoot: runtimeRoot.clone(),
            workspaceRoot: workspaceRoot.clone(),
            inner: operit_host_linux_native::LinuxRuntimeStorageHost::newWithRoots(
                root,
                runtimeRoot,
                workspaceRoot,
            ),
        }
    }
}

impl RuntimeStorageHost for AndroidRuntimeStorageHost {
    /// Returns the Android runtime storage root directory.
    fn rootDir(&self) -> Option<PathBuf> {
        Some(self.root.clone())
    }

    /// Returns the Android runtime files root directory.
    #[allow(non_snake_case)]
    fn runtimeRootDir(&self) -> Option<PathBuf> {
        Some(self.runtimeRoot.clone())
    }

    /// Returns the Android workspace collection root directory.
    #[allow(non_snake_case)]
    fn workspaceRootDir(&self) -> Option<PathBuf> {
        Some(self.workspaceRoot.clone())
    }

    /// Reads bytes from Android runtime storage.
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.inner.readBytes(path)
    }

    /// Writes bytes into Android runtime storage.
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.writeBytes(path, content)
    }

    /// Deletes an entry from Android runtime storage.
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.inner.delete(path, recursive)
    }

    /// Checks whether an Android runtime storage path exists.
    fn exists(&self, path: &str) -> HostResult<bool> {
        self.inner.exists(path)
    }

    /// Lists entries under an Android runtime storage prefix.
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        self.inner.list(prefix)
    }
}

impl HostSecretStore for AndroidRuntimeStorageHost {
    /// Reads secret bytes from the Android system-backed host secret store.
    fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
        #[cfg(target_os = "android")]
        {
            crate::secret_store::androidHostSecretStoreBridge()?.readSecret(key)
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = key;
            Err(operit_host_api::HostError::new(
                "Android host secret store is available on Android targets",
            ))
        }
    }

    /// Writes secret bytes into the Android system-backed host secret store.
    fn writeSecret(&self, key: &str, content: &[u8]) -> HostResult<()> {
        #[cfg(target_os = "android")]
        {
            crate::secret_store::androidHostSecretStoreBridge()?.writeSecret(key, content)
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = (key, content);
            Err(operit_host_api::HostError::new(
                "Android host secret store is available on Android targets",
            ))
        }
    }

    /// Deletes secret bytes from the Android system-backed host secret store.
    fn deleteSecret(&self, key: &str) -> HostResult<()> {
        #[cfg(target_os = "android")]
        {
            crate::secret_store::androidHostSecretStoreBridge()?.deleteSecret(key)
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = key;
            Err(operit_host_api::HostError::new(
                "Android host secret store is available on Android targets",
            ))
        }
    }
}

impl RuntimeSqliteHost for AndroidRuntimeStorageHost {
    /// Opens an SQLite database under Android runtime storage.
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        self.inner.openSqliteDatabase(path)
    }
}
