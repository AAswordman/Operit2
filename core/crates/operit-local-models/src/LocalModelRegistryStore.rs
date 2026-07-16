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
