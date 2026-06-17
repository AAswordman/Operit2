use std::path::PathBuf;

use operit_host_api::{
    HostResult, RuntimeSqliteConnection, RuntimeSqliteHost, RuntimeStorageEntry, RuntimeStorageHost,
};

#[derive(Clone, Debug)]
pub struct AndroidRuntimeStorageHost {
    root: PathBuf,
    inner: operit_host_linux_native::LinuxRuntimeStorageHost,
}

impl AndroidRuntimeStorageHost {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root: root.clone(),
            inner: operit_host_linux_native::LinuxRuntimeStorageHost::new(root),
        }
    }
}

impl RuntimeStorageHost for AndroidRuntimeStorageHost {
    fn rootDir(&self) -> Option<PathBuf> {
        Some(self.root.clone())
    }

    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.inner.readBytes(path)
    }

    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.writeBytes(path, content)
    }

    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.inner.delete(path, recursive)
    }

    fn exists(&self, path: &str) -> HostResult<bool> {
        self.inner.exists(path)
    }

    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        self.inner.list(prefix)
    }
}

impl RuntimeSqliteHost for AndroidRuntimeStorageHost {
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        self.inner.openSqliteDatabase(path)
    }
}
