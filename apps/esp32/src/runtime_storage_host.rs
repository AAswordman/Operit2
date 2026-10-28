//! Packed endpoint storage shared by firmware and the strict desktop simulator.
use crate::runtime_storage_codec::{self as codec, Files};
use crate::runtime_storage_pack as pack;
use operit_host_api::{HostError, HostResult, RuntimeStorageEntry, RuntimeStorageHost};
use std::sync::{Arc, Mutex};

/// Supplies only the durable snapshot carrier; record semantics remain shared.
pub trait RuntimeStorageBackend: codec::SnapshotBackend + Send {
    /// Reads the commit marker selecting the one authoritative snapshot bank.
    fn marker(&mut self) -> HostResult<Option<u32>>;
    /// Reads one compressed chunk into the caller's bounded buffer.
    fn read_chunk(&mut self, bank: u8, index: usize, destination: &mut [u8]) -> HostResult<usize>;
}

struct StorageState<B> {
    backend: B,
    files: Files,
    snapshot: Option<u8>,
}

/// Owns the same resident packed index, limits and transactions on every carrier.
pub struct PackedRuntimeStorageHost<B> {
    state: Mutex<StorageState<B>>,
}

impl<B: RuntimeStorageBackend> PackedRuntimeStorageHost<B> {
    /// Validates the committed snapshot before reclaiming its unpublished bank.
    pub fn open(mut backend: B) -> HostResult<Arc<Self>> {
        let marker = backend.marker()?;
        let snapshot = marker.map(|value| (value & 1) as u8);
        let mut files = match marker {
            Some(marker) => {
                let length = codec::snapshot_length(marker)?;
                let mut buffer = Vec::new();
                buffer
                    .try_reserve_exact(length)
                    .map_err(|_| codec::allocation_error("snapshot read", length))?;
                buffer.resize(length, 0);
                for (index, chunk) in buffer.chunks_mut(codec::SNAPSHOT_CHUNK_SIZE).enumerate() {
                    let mut stored = [0u8; codec::SNAPSHOT_CHUNK_SIZE + 3];
                    let length = backend.read_chunk((marker & 1) as u8, index, &mut stored)?;
                    codec::decode_snapshot_chunk(&stored[..length], chunk)?;
                }
                codec::decode_packed_files(&buffer)?
            }
            None => Files::new(),
        };
        files.compact_records()?;
        codec::recover_snapshot(&mut backend, snapshot)?;
        Ok(Arc::new(Self {
            state: Mutex::new(StorageState {
                backend,
                files,
                snapshot,
            }),
        }))
    }

    /// Commits a change while owning the only resident packed index.
    fn update(
        &self,
        change: impl FnOnce(&Files) -> HostResult<codec::FileChange>,
    ) -> HostResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| HostError::new("runtime snapshot lock poisoned"))?;
        let change = change(&state.files)?;
        let StorageState {
            backend,
            files,
            snapshot,
        } = &mut *state;
        codec::commit_file_change(files, snapshot, change, backend)
    }
}
impl<B: RuntimeStorageBackend> RuntimeStorageHost for PackedRuntimeStorageHost<B> {
    /// Releases parsed record caches after each access.
    fn retainPreferencesCache(&self) -> bool {
        false
    }
    /// Bounds completed endpoint application records.
    fn nodeLocalApplicationHistoryLimit(&self) -> Option<usize> {
        Some(2)
    }
    /// Keeps packed records behind the virtual storage interface.
    fn runtimeRootDir(&self) -> Option<std::path::PathBuf> {
        None
    }
    /// Exposes no physical workspace for a device endpoint.
    fn workspaceRootDir(&self) -> Option<std::path::PathBuf> {
        None
    }
    /// Decodes one packed runtime record.
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.state
            .lock()
            .map_err(|_| HostError::new("runtime NVS lock poisoned"))?
            .files
            .get(path)
            .ok_or_else(|| HostError::new("runtime storage entry not found"))
            .and_then(|bytes| pack::unpack(bytes))
    }
    /// Decodes one bounded range from a packed runtime record.
    fn readBytesRange(&self, path: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        self.state
            .lock()
            .map_err(|_| HostError::new("runtime NVS lock poisoned"))?
            .files
            .get(path)
            .ok_or_else(|| HostError::new("runtime storage entry not found"))
            .and_then(|bytes| pack::unpack_range(bytes, offset, length))
    }
    /// Publishes one record through the shared snapshot transaction.
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.update(|_| {
            Ok(codec::FileChange::Put {
                path: codec::copy_path(path)?,
                bytes: pack::pack(content)?,
            })
        })
        .map_err(|error| {
            HostError::new(format!(
                "{} (record {path}, {} expanded bytes)",
                error.message,
                content.len()
            ))
        })
    }
    /// Publishes complete record bytes through the inactive snapshot bank.
    fn writeBytesAtomically(&self, path: &str, content: &[u8]) -> HostResult<()> {
        // writeBytes publishes the complete NVS snapshot through the alternate bank.
        self.writeBytes(path, content)
    }
    /// Appends within the shared per-record storage bound.
    fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.update(|files| {
            let mut bytes = files
                .get(path)
                .map(|bytes| pack::unpack(bytes))
                .transpose()?
                .unwrap_or_default();
            if content.len() > pack::MAX_RECORD_SIZE.saturating_sub(bytes.len()) {
                return Err(HostError::new("ESP32 node state exceeds NVS budget"));
            }
            bytes
                .try_reserve_exact(content.len())
                .map_err(|_| codec::allocation_error("append", bytes.len() + content.len()))?;
            bytes.extend_from_slice(content);
            Ok(codec::FileChange::Put {
                path: codec::copy_path(path)?,
                bytes: pack::pack(&bytes)?,
            })
        })
    }
    /// Confirms the file already belongs to the synchronously committed NVS snapshot.
    fn flushFile(&self, path: &str) -> HostResult<()> {
        let state = self
            .state
            .lock()
            .map_err(|_| HostError::new("runtime NVS lock poisoned"))?;
        if !state.files.contains_key(path) {
            return Err(HostError::new("runtime storage entry not found"));
        }
        Ok(())
    }

    /// Commits one virtual record deletion.
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.update(|_| {
            Ok(codec::FileChange::Delete {
                path: codec::copy_path(path)?,
                recursive,
            })
        })
    }
    /// Checks the resident packed index without accessing the persistence carrier.
    fn exists(&self, path: &str) -> HostResult<bool> {
        Ok(self
            .state
            .lock()
            .map_err(|_| HostError::new("runtime NVS lock poisoned"))?
            .files
            .contains_key(path))
    }
    /// Lists logical records and validates their expanded lengths.
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        Ok(self
            .state
            .lock()
            .map_err(|_| HostError::new("runtime NVS lock poisoned"))?
            .files
            .iter()
            .filter(|(path, _)| path.starts_with(prefix))
            .map(|(path, bytes)| {
                Ok(RuntimeStorageEntry {
                    path: path.clone(),
                    isDirectory: false,
                    size: pack::expanded_size(bytes)? as i64,
                })
            })
            .collect::<HostResult<Vec<_>>>()?)
    }
}
