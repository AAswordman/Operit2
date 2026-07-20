use std::sync::Arc;
#[cfg(test)]
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use operit_host_api::RuntimeStorageHost;
#[cfg(test)]
use operit_host_api::{HostError, HostResult, RuntimeStorageEntry};
use operit_util::RuntimeStorageLayout::RUNTIME_LOCAL_MODEL_REGISTRY_PATH;
use thiserror::Error;

use crate::LocalModelRegistry::LocalModelRegistrySnapshot;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum LocalModelRegistryStoreError {
    #[error("registry storage error: {0}")]
    Storage(String),
    #[error("registry JSON error: {0}")]
    Json(String),
    #[error("invalid registry storage path: {0}")]
    InvalidStoragePath(String),
}

#[derive(Clone)]
pub struct LocalModelRegistryStore {
    path: String,
    storageHost: Arc<dyn RuntimeStorageHost>,
}

impl std::fmt::Debug for LocalModelRegistryStore {
    /// Formats the registry store without exposing host internals.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalModelRegistryStore")
            .field("path", &self.path)
            .finish()
    }
}

impl LocalModelRegistryStore {
    /// Creates a registry store backed by the runtime storage host.
    pub fn forRuntimeStorage(storageHost: Arc<dyn RuntimeStorageHost>) -> Self {
        Self {
            path: RUNTIME_LOCAL_MODEL_REGISTRY_PATH.to_string(),
            storageHost,
        }
    }

    /// Creates a registry store backed by a native runtime root for tests.
    #[cfg(test)]
    pub fn forRuntimeRoot(runtimeRoot: PathBuf) -> Result<Self, LocalModelRegistryStoreError> {
        Ok(Self {
            path: RUNTIME_LOCAL_MODEL_REGISTRY_PATH.to_string(),
            storageHost: testRuntimeStorageHost(runtimeRoot),
        })
    }

    /// Creates a registry store backed by one explicit native registry path for tests.
    #[cfg(test)]
    pub fn new(registryPath: PathBuf) -> Self {
        Self {
            path: RUNTIME_LOCAL_MODEL_REGISTRY_PATH.to_string(),
            storageHost: Arc::new(TestRuntimeStorageHost::forRegistryPath(registryPath)),
        }
    }

    /// Reads the installed model and engine registry snapshot.
    pub fn read(&self) -> Result<LocalModelRegistrySnapshot, LocalModelRegistryStoreError> {
        if !self
            .storageHost
            .exists(&self.path)
            .map_err(|error| LocalModelRegistryStoreError::Storage(error.to_string()))?
        {
            return Ok(LocalModelRegistrySnapshot::empty());
        }
        let bytes = self
            .storageHost
            .readBytes(&self.path)
            .map_err(|error| LocalModelRegistryStoreError::Storage(error.to_string()))?;
        let content = String::from_utf8(bytes)
            .map_err(|error| LocalModelRegistryStoreError::Storage(error.to_string()))?;
        serde_json::from_str(&content)
            .map_err(|error| LocalModelRegistryStoreError::Json(error.to_string()))
    }

    /// Writes the installed model and engine registry snapshot.
    pub fn write(
        &self,
        snapshot: &LocalModelRegistrySnapshot,
    ) -> Result<(), LocalModelRegistryStoreError> {
        let content = serde_json::to_vec_pretty(snapshot)
            .map_err(|error| LocalModelRegistryStoreError::Json(error.to_string()))?;
        self.storageHost
            .writeBytes(&self.path, &content)
            .map_err(|error| LocalModelRegistryStoreError::Storage(error.to_string()))
    }
}

#[cfg(test)]
/// Creates a test runtime storage host rooted at one native runtime directory.
pub(crate) fn testRuntimeStorageHost(runtimeRoot: PathBuf) -> Arc<dyn RuntimeStorageHost> {
    Arc::new(TestRuntimeStorageHost::forRuntimeRoot(runtimeRoot))
}

#[cfg(test)]
#[derive(Clone, Debug)]
struct TestRuntimeStorageHost {
    runtimeRoot: Option<PathBuf>,
    registryPath: Option<PathBuf>,
}

#[cfg(test)]
impl TestRuntimeStorageHost {
    /// Creates a test storage host rooted at one runtime directory.
    fn forRuntimeRoot(runtimeRoot: PathBuf) -> Self {
        Self {
            runtimeRoot: Some(runtimeRoot),
            registryPath: None,
        }
    }

    /// Creates a test storage host bound to one registry file.
    fn forRegistryPath(registryPath: PathBuf) -> Self {
        Self {
            runtimeRoot: None,
            registryPath: Some(registryPath),
        }
    }

    /// Resolves one virtual runtime storage path to its native test path.
    fn resolve(&self, path: &str) -> HostResult<PathBuf> {
        if path == RUNTIME_LOCAL_MODEL_REGISTRY_PATH {
            if let Some(registryPath) = &self.registryPath {
                return Ok(registryPath.clone());
            }
        }
        let runtimeRoot = self.runtimeRoot.as_ref().ok_or_else(|| {
            HostError::new(format!(
                "test runtime root is required for storage path: {path}"
            ))
        })?;
        let mut components = Path::new(path.trim()).components();
        match components.next() {
            Some(Component::Normal(segment)) if segment == "runtime" => {}
            _ => {
                return Err(HostError::new(format!(
                    "runtime storage path must start with runtime/: {path}"
                )));
            }
        }
        let mut resolved = runtimeRoot.clone();
        for component in components {
            match component {
                Component::Normal(segment) => resolved.push(segment),
                Component::CurDir => {}
                _ => {
                    return Err(HostError::new(format!(
                        "invalid runtime storage path: {path}"
                    )));
                }
            }
        }
        Ok(resolved)
    }

    /// Converts one native test path into a virtual runtime storage path.
    fn storagePathForNative(&self, path: &Path) -> HostResult<String> {
        let runtimeRoot = self.runtimeRoot.as_ref().ok_or_else(|| {
            HostError::new(format!(
                "test runtime root is required for native path: {}",
                path.display()
            ))
        })?;
        let relative = path.strip_prefix(runtimeRoot).map_err(|_| {
            HostError::new(format!(
                "native path is outside the test runtime root: {}",
                path.display()
            ))
        })?;
        let relative = relative.to_string_lossy().replace('\\', "/");
        if relative.is_empty() {
            Ok("runtime".to_string())
        } else {
            Ok(format!("runtime/{relative}"))
        }
    }
}

#[cfg(test)]
impl RuntimeStorageHost for TestRuntimeStorageHost {
    /// Returns the runtime root used by root-backed registry tests.
    fn runtimeRootDir(&self) -> Option<PathBuf> {
        self.runtimeRoot.clone()
    }

    /// Returns no workspace root because registry tests do not use workspace storage.
    fn workspaceRootDir(&self) -> Option<PathBuf> {
        None
    }

    /// Reads bytes from a virtual runtime storage path.
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        Ok(fs::read(self.resolve(path)?)?)
    }

    /// Writes bytes to a virtual runtime storage path.
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    /// Deletes a virtual runtime storage path.
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        let path = self.resolve(path)?;
        if !path.exists() {
            return Ok(());
        }
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_dir() {
            if recursive {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_dir(path)?;
            }
        } else {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Checks whether a virtual runtime storage path exists.
    fn exists(&self, path: &str) -> HostResult<bool> {
        Ok(self.resolve(path)?.exists())
    }

    /// Lists direct children under one virtual runtime storage directory.
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        let directory = self.resolve(prefix)?;
        if !directory.exists() {
            return Ok(Vec::new());
        }
        let mut entries = Vec::new();
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            entries.push(RuntimeStorageEntry {
                path: self.storagePathForNative(&entry.path())?,
                isDirectory: metadata.is_dir(),
                size: metadata.len() as i64,
            });
        }
        Ok(entries)
    }
}
