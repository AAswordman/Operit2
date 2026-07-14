use std::path::PathBuf;

use operit_host_api::{
    HostResult, RuntimeSqliteConnection, RuntimeSqliteHost, RuntimeStorageEntry, RuntimeStorageHost,
};
use operit_host_native_common::NativeRuntimeStorageHost;

#[derive(Clone, Debug)]
pub struct OhosRuntimeStorageHost {
    inner: NativeRuntimeStorageHost,
}

impl OhosRuntimeStorageHost {
    /// Creates the OpenHarmony runtime storage host with app-owned roots.
    pub fn new(runtimeRoot: PathBuf, workspaceRoot: PathBuf) -> Self {
        Self {
            inner: NativeRuntimeStorageHost::new(runtimeRoot, workspaceRoot),
        }
    }
}

impl RuntimeStorageHost for OhosRuntimeStorageHost {
    /// Returns the OpenHarmony runtime storage physical root.
    fn runtimeRootDir(&self) -> Option<PathBuf> {
        self.inner.runtimeRootDir()
    }

    /// Returns the OpenHarmony workspace storage physical root.
    fn workspaceRootDir(&self) -> Option<PathBuf> {
        self.inner.workspaceRootDir()
    }

    /// Reads bytes from OpenHarmony runtime storage.
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.inner.readBytes(path)
    }

    /// Writes bytes into OpenHarmony runtime storage.
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.writeBytes(path, content)
    }

    /// Deletes an OpenHarmony runtime storage entry.
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.inner.delete(path, recursive)
    }

    /// Checks whether an OpenHarmony runtime storage entry exists.
    fn exists(&self, path: &str) -> HostResult<bool> {
        self.inner.exists(path)
    }

    /// Lists OpenHarmony runtime storage entries.
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        self.inner.list(prefix)
    }
}

impl RuntimeSqliteHost for OhosRuntimeStorageHost {
    /// Opens an OpenHarmony SQLite database under runtime storage.
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        self.inner.openSqliteDatabase(path)
    }
}
