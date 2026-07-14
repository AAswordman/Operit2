use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
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
    backend: LocalModelRegistryStoreBackend,
}

#[derive(Clone)]
enum LocalModelRegistryStoreBackend {
    NativePath {
        path: PathBuf,
    },
    RuntimeStorage {
        path: String,
        storageHost: Arc<dyn RuntimeStorageHost>,
    },
}

impl std::fmt::Debug for LocalModelRegistryStore {
    /// Formats the registry store without exposing host internals.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.backend {
            LocalModelRegistryStoreBackend::NativePath { path } => formatter
                .debug_struct("LocalModelRegistryStore")
                .field("path", path)
                .finish(),
            LocalModelRegistryStoreBackend::RuntimeStorage { path, .. } => formatter
                .debug_struct("LocalModelRegistryStore")
                .field("path", path)
                .finish(),
        }
    }
}

impl LocalModelRegistryStore {
    /// Creates a registry store under one runtime root.
    pub fn forRuntimeRoot(runtimeRoot: PathBuf) -> Result<Self, LocalModelRegistryStoreError> {
        let path = runtimeLayoutPath(&runtimeRoot, RUNTIME_LOCAL_MODEL_REGISTRY_PATH)?;
        Ok(Self {
            backend: LocalModelRegistryStoreBackend::NativePath { path },
        })
    }

    /// Creates a registry store for one exact native path.
    pub fn new(path: PathBuf) -> Self {
        Self {
            backend: LocalModelRegistryStoreBackend::NativePath { path },
        }
    }

    /// Creates a registry store backed by the runtime storage host.
    pub fn forRuntimeStorage(storageHost: Arc<dyn RuntimeStorageHost>) -> Self {
        Self {
            backend: LocalModelRegistryStoreBackend::RuntimeStorage {
                path: RUNTIME_LOCAL_MODEL_REGISTRY_PATH.to_string(),
                storageHost,
            },
        }
    }

    /// Reads the installed model and engine registry snapshot.
    pub fn read(&self) -> Result<LocalModelRegistrySnapshot, LocalModelRegistryStoreError> {
        let content = match &self.backend {
            LocalModelRegistryStoreBackend::NativePath { path } => {
                if !path.exists() {
                    return Ok(LocalModelRegistrySnapshot::empty());
                }
                fs::read_to_string(path)
                    .map_err(|error| LocalModelRegistryStoreError::Storage(error.to_string()))?
            }
            LocalModelRegistryStoreBackend::RuntimeStorage { path, storageHost } => {
                if !storageHost
                    .exists(path)
                    .map_err(|error| LocalModelRegistryStoreError::Storage(error.to_string()))?
                {
                    return Ok(LocalModelRegistrySnapshot::empty());
                }
                let bytes = storageHost
                    .readBytes(path)
                    .map_err(|error| LocalModelRegistryStoreError::Storage(error.to_string()))?;
                String::from_utf8(bytes)
                    .map_err(|error| LocalModelRegistryStoreError::Storage(error.to_string()))?
            }
        };
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
        match &self.backend {
            LocalModelRegistryStoreBackend::NativePath { path } => {
                let parent = path.parent().ok_or_else(|| {
                    LocalModelRegistryStoreError::InvalidStoragePath(
                        path.to_string_lossy().to_string(),
                    )
                })?;
                fs::create_dir_all(parent)
                    .map_err(|error| LocalModelRegistryStoreError::Storage(error.to_string()))?;
                fs::write(path, content)
                    .map_err(|error| LocalModelRegistryStoreError::Storage(error.to_string()))
            }
            LocalModelRegistryStoreBackend::RuntimeStorage { path, storageHost } => storageHost
                .writeBytes(path, &content)
                .map_err(|error| LocalModelRegistryStoreError::Storage(error.to_string())),
        }
    }

    /// Returns the exact native registry path when the store is native-backed.
    pub fn path(&self) -> Option<&Path> {
        match &self.backend {
            LocalModelRegistryStoreBackend::NativePath { path } => Some(path.as_path()),
            LocalModelRegistryStoreBackend::RuntimeStorage { .. } => None,
        }
    }
}

/// Maps a runtime-layout path into a native path under the runtime root.
fn runtimeLayoutPath(
    runtimeRoot: &Path,
    storagePath: &str,
) -> Result<PathBuf, LocalModelRegistryStoreError> {
    let relative = storagePath
        .trim()
        .strip_prefix("runtime/")
        .ok_or_else(|| LocalModelRegistryStoreError::InvalidStoragePath(storagePath.to_string()))?;
    Ok(runtimeRoot.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR)))
}
