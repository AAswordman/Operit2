use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_util::RuntimeStorageLayout::{runtimeStorageOwnership, RuntimeStorageOwnership};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::SyncOperationStore::{
    NewSyncOperation, SyncOperation, SyncOperationSemantics, SyncOperationStore,
};

pub const RUNTIME_FILE_SYNC_DOMAIN: &str = "runtime_file";
const RUNTIME_FILE_SYNC_ENTITY_TYPE: &str = "file";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeFileSyncReference {
    pub contentHash: String,
    pub size: i64,
}

/// Records and applies synchronized snapshots for explicitly registered runtime files.
#[derive(Clone)]
pub struct RuntimeFileSyncStore {
    storageHost: Arc<dyn RuntimeStorageHost>,
    syncRootPath: String,
    syncOperationStore: SyncOperationStore,
}

impl RuntimeFileSyncStore {
    /// Creates a runtime file synchronization store over one host and operation root.
    pub fn new(storageHost: Arc<dyn RuntimeStorageHost>, syncRootPath: impl Into<String>) -> Self {
        let syncRootPath = syncRootPath.into();
        Self {
            syncOperationStore: SyncOperationStore::new(storageHost.clone(), syncRootPath.clone()),
            storageHost,
            syncRootPath,
        }
    }

    /// Writes a complete Space file snapshot and records its synchronization operation.
    #[allow(non_snake_case)]
    pub fn writeBytes(&self, storagePath: &str, content: &[u8]) -> Result<(), String> {
        requireSpaceFile(storagePath)?;
        self.storageHost
            .writeBytes(storagePath, content)
            .map_err(|error| error.to_string())?;
        let reference = self.storeBlob(content)?;
        let deviceId = self
            .syncOperationStore
            .localDeviceId()
            .map_err(|error| error.to_string())?;
        self.syncOperationStore
            .appendLocalOperation(
                &deviceId,
                NewSyncOperation {
                    domain: RUNTIME_FILE_SYNC_DOMAIN.to_string(),
                    entityType: RUNTIME_FILE_SYNC_ENTITY_TYPE.to_string(),
                    entityId: storagePath.to_string(),
                    operation: "upsert".to_string(),
                    semantics: SyncOperationSemantics::EntityState,
                    payload: serde_json::to_value(reference).map_err(|error| error.to_string())?,
                },
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    /// Appends bytes to a Space file and records the resulting complete snapshot.
    #[allow(non_snake_case)]
    pub fn appendBytes(&self, storagePath: &str, content: &[u8]) -> Result<(), String> {
        requireSpaceFile(storagePath)?;
        let mut next = if self
            .storageHost
            .exists(storagePath)
            .map_err(|error| error.to_string())?
        {
            self.storageHost
                .readBytes(storagePath)
                .map_err(|error| error.to_string())?
        } else {
            Vec::new()
        };
        next.extend_from_slice(content);
        self.writeBytes(storagePath, &next)
    }

    /// Deletes a Space file and records a synchronized tombstone.
    pub fn delete(&self, storagePath: &str) -> Result<(), String> {
        requireSpaceFile(storagePath)?;
        if self
            .storageHost
            .exists(storagePath)
            .map_err(|error| error.to_string())?
        {
            self.storageHost
                .delete(storagePath, false)
                .map_err(|error| error.to_string())?;
        }
        let deviceId = self
            .syncOperationStore
            .localDeviceId()
            .map_err(|error| error.to_string())?;
        self.syncOperationStore
            .appendLocalOperation(
                &deviceId,
                NewSyncOperation {
                    domain: RUNTIME_FILE_SYNC_DOMAIN.to_string(),
                    entityType: RUNTIME_FILE_SYNC_ENTITY_TYPE.to_string(),
                    entityId: storagePath.to_string(),
                    operation: "delete".to_string(),
                    semantics: SyncOperationSemantics::EntityState,
                    payload: serde_json::Value::Null,
                },
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    /// Applies one validated runtime file operation without recording another local mutation.
    #[allow(non_snake_case)]
    pub fn applySyncedOperation(
        storageHost: Arc<dyn RuntimeStorageHost>,
        syncRootPath: impl Into<String>,
        storagePath: &str,
        operation: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        requireSpaceFile(storagePath)?;
        let store = Self::new(storageHost.clone(), syncRootPath);
        match operation {
            "upsert" => {
                let reference: RuntimeFileSyncReference =
                    serde_json::from_value(payload).map_err(|error| error.to_string())?;
                let content = store.readBlob(&reference)?;
                storageHost
                    .writeBytes(storagePath, &content)
                    .map_err(|error| error.to_string())
            }
            "delete" => {
                if storageHost
                    .exists(storagePath)
                    .map_err(|error| error.to_string())?
                {
                    storageHost
                        .delete(storagePath, false)
                        .map_err(|error| error.to_string())?;
                }
                Ok(())
            }
            other => Err(format!("unsupported runtime file sync operation: {other}")),
        }
    }

    /// Returns the blob required by one runtime-file upsert operation.
    #[allow(non_snake_case)]
    pub fn requiredBlob(
        operation: &SyncOperation,
    ) -> Result<Option<RuntimeFileSyncReference>, String> {
        if operation.domain != RUNTIME_FILE_SYNC_DOMAIN {
            return Ok(None);
        }
        if operation.entityType != RUNTIME_FILE_SYNC_ENTITY_TYPE {
            return Err(format!(
                "unsupported runtime file entity type: {}",
                operation.entityType
            ));
        }
        match operation.operation.as_str() {
            "upsert" => serde_json::from_value(operation.payload.clone())
                .map(Some)
                .map_err(|error| error.to_string()),
            "delete" => Ok(None),
            other => Err(format!("unsupported runtime file sync operation: {other}")),
        }
    }

    /// Reports whether a verified content-addressed blob is available locally.
    #[allow(non_snake_case)]
    pub fn hasBlob(&self, reference: &RuntimeFileSyncReference) -> Result<bool, String> {
        validateBlobReference(reference)?;
        let path = self.blobPath(&reference.contentHash)?;
        if !self
            .storageHost
            .exists(&path)
            .map_err(|error| error.to_string())?
        {
            return Ok(false);
        }
        let content = self
            .storageHost
            .readBytes(&path)
            .map_err(|error| error.to_string())?;
        verifyBlob(reference, &content)?;
        Ok(true)
    }

    /// Reads one bounded range from a verified content-addressed blob.
    #[allow(non_snake_case)]
    pub fn readBlobChunk(
        &self,
        contentHash: &str,
        offset: i64,
        length: i64,
    ) -> Result<Vec<u8>, String> {
        validateContentHash(contentHash)?;
        let offset = usize::try_from(offset)
            .map_err(|_| "blob chunk offset must not be negative".to_string())?;
        let length = usize::try_from(length)
            .map_err(|_| "blob chunk length must not be negative".to_string())?;
        self.storageHost
            .readBytesRange(
                &self.blobPath(contentHash)?,
                u64::try_from(offset)
                    .map_err(|_| "blob chunk offset does not fit u64".to_string())?,
                length,
            )
            .map_err(|error| error.to_string())
    }

    /// Verifies and persists one complete content-addressed blob.
    #[allow(non_snake_case)]
    pub fn writeBlob(
        &self,
        reference: &RuntimeFileSyncReference,
        content: &[u8],
    ) -> Result<(), String> {
        verifyBlob(reference, content)?;
        self.storageHost
            .writeBytes(&self.blobPath(&reference.contentHash)?, content)
            .map_err(|error| error.to_string())
    }

    /// Resolves the verified host storage path for one content-addressed blob.
    #[allow(non_snake_case)]
    pub fn blobStoragePath(&self, reference: &RuntimeFileSyncReference) -> Result<String, String> {
        validateBlobReference(reference)?;
        self.blobPath(&reference.contentHash)
    }

    /// Persists local bytes and returns their verified content identity.
    #[allow(non_snake_case)]
    fn storeBlob(&self, content: &[u8]) -> Result<RuntimeFileSyncReference, String> {
        let size = i64::try_from(content.len())
            .map_err(|_| "runtime file size does not fit i64".to_string())?;
        let reference = RuntimeFileSyncReference {
            contentHash: contentHash(content),
            size,
        };
        self.writeBlob(&reference, content)?;
        Ok(reference)
    }

    /// Reads and verifies one complete content-addressed blob.
    #[allow(non_snake_case)]
    fn readBlob(&self, reference: &RuntimeFileSyncReference) -> Result<Vec<u8>, String> {
        validateBlobReference(reference)?;
        let content = self
            .storageHost
            .readBytes(&self.blobPath(&reference.contentHash)?)
            .map_err(|error| error.to_string())?;
        verifyBlob(reference, &content)?;
        Ok(content)
    }

    /// Resolves one validated blob hash below the private synchronization root.
    #[allow(non_snake_case)]
    fn blobPath(&self, contentHash: &str) -> Result<String, String> {
        validateContentHash(contentHash)?;
        Ok(format!("{}/blobs/{contentHash}", self.syncRootPath))
    }
}

/// Computes the lowercase SHA-256 identity of one byte sequence.
#[allow(non_snake_case)]
fn contentHash(content: &[u8]) -> String {
    let digest = Sha256::digest(content);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Validates one content-addressed blob reference.
#[allow(non_snake_case)]
fn validateBlobReference(reference: &RuntimeFileSyncReference) -> Result<(), String> {
    validateContentHash(&reference.contentHash)?;
    if reference.size < 0 {
        return Err("runtime file blob size must not be negative".to_string());
    }
    Ok(())
}

/// Validates a lowercase SHA-256 storage key.
#[allow(non_snake_case)]
fn validateContentHash(contentHash: &str) -> Result<(), String> {
    if contentHash.len() != 64
        || !contentHash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("runtime file blob hash must be lowercase SHA-256 hex".to_string());
    }
    Ok(())
}

/// Verifies declared blob size and content identity.
#[allow(non_snake_case)]
fn verifyBlob(reference: &RuntimeFileSyncReference, content: &[u8]) -> Result<(), String> {
    validateBlobReference(reference)?;
    let size = i64::try_from(content.len())
        .map_err(|_| "runtime file size does not fit i64".to_string())?;
    if size != reference.size {
        return Err(format!(
            "runtime file blob size mismatch: expected {}, got {size}",
            reference.size
        ));
    }
    let actualHash = contentHash(content);
    if actualHash != reference.contentHash {
        return Err(format!(
            "runtime file blob hash mismatch: expected {}, got {actualHash}",
            reference.contentHash
        ));
    }
    Ok(())
}

/// Rejects an operation whose entity is outside the explicit Space file registry.
#[allow(non_snake_case)]
fn requireSpaceFile(storagePath: &str) -> Result<(), String> {
    if runtimeStorageOwnership(storagePath)? != RuntimeStorageOwnership::Space {
        return Err(format!(
            "runtime file is not registered for Space synchronization: {storagePath}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use operit_host_api::{HostError, RuntimeStorageEntry};
    use operit_util::RuntimeStorageLayout::{
        RUNTIME_CLIENT_LOG_PATH, RUNTIME_SYNC_DIR_PATH, RUNTIME_WEBSESSION_BROWSER_HISTORY_PATH,
    };

    use super::*;
    use crate::SyncOperationStore::SyncClock;

    #[derive(Clone, Default)]
    struct MemoryStorageHost {
        files: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    }

    impl RuntimeStorageHost for MemoryStorageHost {
        /// Reports no physical runtime root for the in-memory host.
        fn runtimeRootDir(&self) -> Option<std::path::PathBuf> {
            None
        }

        /// Reports no physical workspace root for the in-memory host.
        fn workspaceRootDir(&self) -> Option<std::path::PathBuf> {
            None
        }

        /// Reads one in-memory runtime storage object.
        fn readBytes(&self, path: &str) -> operit_host_api::HostResult<Vec<u8>> {
            self.files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .get(path)
                .cloned()
                .ok_or_else(|| HostError::new(format!("missing test file: {path}")))
        }

        /// Reads one bounded range from an in-memory runtime storage object.
        fn readBytesRange(
            &self,
            path: &str,
            offset: u64,
            length: usize,
        ) -> operit_host_api::HostResult<Vec<u8>> {
            let content = self.readBytes(path)?;
            let offset = usize::try_from(offset)
                .map_err(|_| HostError::new("test range offset does not fit usize"))?;
            if offset >= content.len() {
                return Ok(Vec::new());
            }
            let end = offset
                .checked_add(length)
                .ok_or_else(|| HostError::new("test range length overflow"))?
                .min(content.len());
            Ok(content[offset..end].to_vec())
        }

        /// Writes one in-memory runtime storage object.
        fn writeBytes(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
            self.files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .insert(path.to_string(), content.to_vec());
            Ok(())
        }

        /// Appends bytes to one in-memory runtime storage object.
        fn appendBytes(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
            self.files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .entry(path.to_string())
                .or_default()
                .extend_from_slice(content);
            Ok(())
        }

        /// Deletes one in-memory runtime storage object.
        fn delete(&self, path: &str, _recursive: bool) -> operit_host_api::HostResult<()> {
            self.files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .remove(path);
            Ok(())
        }

        /// Reports whether one in-memory runtime storage object exists.
        fn exists(&self, path: &str) -> operit_host_api::HostResult<bool> {
            Ok(self
                .files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .contains_key(path))
        }

        /// Lists in-memory runtime storage objects below one prefix.
        fn list(&self, prefix: &str) -> operit_host_api::HostResult<Vec<RuntimeStorageEntry>> {
            Ok(self
                .files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .iter()
                .filter(|(path, _)| path.starts_with(prefix))
                .map(|(path, content)| RuntimeStorageEntry {
                    path: path.clone(),
                    isDirectory: false,
                    size: content.len() as i64,
                })
                .collect())
        }
    }

    /// Verifies upsert and delete operations reproduce the same registered file on another host.
    #[test]
    fn synchronizes_registered_file_snapshots_and_tombstones() {
        let source = Arc::new(MemoryStorageHost::default());
        let target = Arc::new(MemoryStorageHost::default());
        let path = RUNTIME_WEBSESSION_BROWSER_HISTORY_PATH;
        let sourceStore = RuntimeFileSyncStore::new(source.clone(), RUNTIME_SYNC_DIR_PATH);
        sourceStore
            .writeBytes(path, b"[{\"url\":\"https://operit.app\"}]")
            .unwrap();

        let operationStore = SyncOperationStore::new(source, RUNTIME_SYNC_DIR_PATH);
        let operations = operationStore
            .operationsSince(
                &SyncClock::empty(),
                &[RUNTIME_FILE_SYNC_DOMAIN.to_string()],
                16,
            )
            .unwrap();
        assert_eq!(operations.len(), 1);
        let reference = RuntimeFileSyncStore::requiredBlob(&operations[0])
            .unwrap()
            .expect("runtime file upsert must reference a blob");
        let content = sourceStore.readBlob(&reference).unwrap();
        RuntimeFileSyncStore::new(target.clone(), RUNTIME_SYNC_DIR_PATH)
            .writeBlob(&reference, &content)
            .unwrap();
        RuntimeFileSyncStore::applySyncedOperation(
            target.clone(),
            RUNTIME_SYNC_DIR_PATH,
            &operations[0].entityId,
            &operations[0].operation,
            operations[0].payload.clone(),
        )
        .unwrap();
        assert_eq!(
            target.readBytes(path).unwrap(),
            b"[{\"url\":\"https://operit.app\"}]"
        );

        sourceStore.delete(path).unwrap();
        let deleteOperation = operationStore
            .operationsSince(
                &SyncClock::empty(),
                &[RUNTIME_FILE_SYNC_DOMAIN.to_string()],
                16,
            )
            .unwrap()
            .into_iter()
            .find(|operation| operation.operation == "delete")
            .expect("runtime file tombstone must be exported");
        RuntimeFileSyncStore::applySyncedOperation(
            target.clone(),
            RUNTIME_SYNC_DIR_PATH,
            &deleteOperation.entityId,
            &deleteOperation.operation,
            deleteOperation.payload,
        )
        .unwrap();
        assert!(!target.exists(path).unwrap());
    }

    /// Verifies a node-local file cannot enter the runtime file synchronization log.
    #[test]
    fn rejects_node_local_files() {
        let host = Arc::new(MemoryStorageHost::default());
        let store = RuntimeFileSyncStore::new(host, RUNTIME_SYNC_DIR_PATH);
        assert!(store.writeBytes(RUNTIME_CLIENT_LOG_PATH, b"log").is_err());
    }
}
