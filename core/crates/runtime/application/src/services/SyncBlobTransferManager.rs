use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_host_api::{RuntimeStorageHost, RuntimeStorageWriteHost};
use operit_store::RuntimeFileSyncStore::{RuntimeFileSyncReference, RuntimeFileSyncStore};
use operit_util::stream::ReverseStream::ReverseStream;
use operit_util::stream::Stream::Stream;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use sha2::{Digest, Sha256};

const SYNC_BLOB_CHUNK_BYTES: usize = 64 * 1024;

/// Owns streamed synchronization blob uploads for one CoreNode runtime.
#[derive(Clone)]
pub struct SyncBlobTransferManager {
    storageHost: Arc<dyn RuntimeStorageHost>,
    storageWriteHost: Arc<dyn RuntimeStorageWriteHost>,
}

impl SyncBlobTransferManager {
    /// Creates a synchronization blob transfer manager over the runtime storage host.
    #[allow(non_snake_case)]
    pub fn getInstance(hostManager: &HostManager) -> Result<Self, String> {
        let storageHost = hostManager.runtimeStorageHost.clone().ok_or_else(|| {
            "RuntimeStorageHost is not registered for blob synchronization".to_string()
        })?;
        let storageWriteHost = hostManager.runtimeStorageWriteHost.clone().ok_or_else(|| {
            "RuntimeStorageWriteHost is not registered for blob synchronization".to_string()
        })?;
        Ok(Self {
            storageHost,
            storageWriteHost,
        })
    }

    /// Receives and verifies one complete content-addressed blob through a push stream.
    #[allow(non_snake_case)]
    pub async fn syncReceiveBlob(
        &self,
        contentHash: String,
        size: i64,
        mut chunks: ReverseStream<Vec<u8>>,
    ) -> Result<(), String> {
        let expectedSize = usize::try_from(size)
            .map_err(|_| "synchronization blob size must not be negative".to_string())?;
        let reference = RuntimeFileSyncReference { contentHash, size };
        let blobStore = RuntimeFileSyncStore::new(self.storageHost.clone(), RUNTIME_SYNC_DIR_PATH);
        let blobPath = blobStore.blobStoragePath(&reference)?;
        let mut writer = self
            .storageWriteHost
            .createWriteSession(&blobPath)
            .map_err(|error| error.to_string())?;

        let mut received = 0usize;
        let mut hasher = Sha256::new();
        while let Some(chunk) = chunks.recv().await {
            if chunk.len() > SYNC_BLOB_CHUNK_BYTES {
                writer.discard().map_err(|error| error.to_string())?;
                return Err(format!(
                    "synchronization blob chunk exceeds {SYNC_BLOB_CHUNK_BYTES} bytes"
                ));
            }
            if chunk.len() > expectedSize - received {
                writer.discard().map_err(|error| error.to_string())?;
                return Err(format!(
                    "synchronization blob exceeds declared size {expectedSize}"
                ));
            }
            writer
                .writeChunk(&chunk)
                .map_err(|error| error.to_string())?;
            hasher.update(&chunk);
            received += chunk.len();
        }
        if received != expectedSize {
            writer.discard().map_err(|error| error.to_string())?;
            return Err(format!(
                "synchronization blob size mismatch: expected {expectedSize}, got {received}"
            ));
        }
        let actualHash = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if actualHash != reference.contentHash {
            writer.discard().map_err(|error| error.to_string())?;
            return Err(format!(
                "synchronization blob hash mismatch: expected {}, got {actualHash}",
                reference.contentHash
            ));
        }
        writer.commitFast().map_err(|error| error.to_string())
    }
}
